use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::domain::graph::GraphRepositoryIdentity;
use crate::domain::ports::source_provider::{SourceLoadError, SourceProvider};
use crate::domain::source::{LogicalPath, SOURCE_EXTENSIONS, SourceFile};

/// Reads `.adoc` files from a directory tree (or a single file) on disk.
///
/// Iteration order is the lexicographic ordering of paths so that compilation
/// is deterministic across platforms.
///
/// Project-bound callers supply the discovered project and documentation
/// roots explicitly. Standalone callers use [`Self::new`] and publish paths
/// relative to the selected file or directory.
#[derive(Debug, Clone)]
pub(crate) struct FsSourceProvider {
    root: PathBuf,
    project_roots: Option<ProjectRoots>,
}

#[derive(Debug, Clone)]
struct ProjectRoots {
    project: PathBuf,
    docs: PathBuf,
}

impl FsSourceProvider {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            project_roots: None,
        }
    }

    pub(crate) fn for_project(root: PathBuf, project_root: PathBuf, docs_root: PathBuf) -> Self {
        Self {
            root,
            project_roots: Some(ProjectRoots {
                project: project_root,
                docs: docs_root,
            }),
        }
    }
}

impl SourceProvider for FsSourceProvider {
    fn repository_identity(&self) -> GraphRepositoryIdentity {
        match self.project_roots {
            Some(_) => GraphRepositoryIdentity::local_project("agentdoc.config.yaml".to_string()),
            None => GraphRepositoryIdentity::standalone(),
        }
    }

    fn contains(&self, path: &Path) -> bool {
        // Yielded paths share the coordinate space of `root` as given
        // (absolute or cwd-relative), so the resolved link path probes
        // the filesystem directly.
        path.exists()
    }

    fn load_sources(&self) -> Vec<Result<SourceFile, SourceLoadError>> {
        let project_roots = match self.resolved_project_roots() {
            Ok(roots) => roots,
            Err(error) => return vec![Err(error)],
        };
        if let Some(roots) = &project_roots {
            let selected_root = match self.root.canonicalize() {
                Ok(root) => root,
                Err(error) => {
                    return vec![Err(SourceLoadError::unreadable(
                        self.root.clone(),
                        error.to_string(),
                    ))];
                }
            };
            if !selected_root.starts_with(&roots.docs) {
                return vec![Err(SourceLoadError::unsafe_source_path(
                    self.root.clone(),
                    "selected source root resolves outside the configured documentation root",
                ))];
            }
        }

        if self.root.is_file() && !is_source_path(&self.root) {
            return vec![Err(SourceLoadError::unsupported_source_extension(
                self.root.clone(),
            ))];
        }

        let mut results = Vec::new();
        for source_path in source_paths(
            &self.root,
            project_roots.as_ref().map(|roots| roots.docs.as_path()),
        ) {
            match source_path {
                Ok(source_path) => {
                    results.push(self.load_source(source_path, project_roots.as_ref()))
                }
                Err(error) => results.push(Err(error)),
            }
        }
        results
    }
}

impl FsSourceProvider {
    fn resolved_project_roots(&self) -> Result<Option<ProjectRoots>, SourceLoadError> {
        let Some(roots) = &self.project_roots else {
            return Ok(None);
        };
        let project = canonical_root(&roots.project)?;
        let docs = canonical_root(&roots.docs)?;
        if !docs.starts_with(&project) {
            return Err(SourceLoadError::unsafe_source_path(
                roots.docs.clone(),
                "configured documentation root resolves outside the project root",
            ));
        }
        Ok(Some(ProjectRoots { project, docs }))
    }

    fn load_source(
        &self,
        source_path: SourcePath,
        project_roots: Option<&ProjectRoots>,
    ) -> Result<SourceFile, SourceLoadError> {
        let physical_path = source_path.path.canonicalize().map_err(|error| {
            SourceLoadError::unreadable(source_path.path.clone(), error.to_string())
        })?;
        let (identity_path, logical_path) = match project_roots {
            Some(roots) => {
                if !physical_path.starts_with(&roots.docs) {
                    return Err(SourceLoadError::unsafe_source_path(
                        source_path.path,
                        "source resolves outside the configured project documentation root",
                    ));
                }
                let identity = physical_path.strip_prefix(&roots.docs).map_err(|_| {
                    SourceLoadError::unsafe_source_path(
                        source_path.path.clone(),
                        "source is outside the configured documentation root",
                    )
                })?;
                let logical = physical_path.strip_prefix(&roots.project).map_err(|_| {
                    SourceLoadError::unsafe_source_path(
                        source_path.path.clone(),
                        "source is outside the configured project root",
                    )
                })?;
                (identity.to_path_buf(), logical.to_path_buf())
            }
            None => (source_path.identity_path.clone(), source_path.identity_path),
        };
        let logical_path = LogicalPath::from_relative_path(&logical_path).map_err(|_| {
            SourceLoadError::unsafe_source_path(logical_path, "logical source path is not portable")
        })?;
        let text = fs::read_to_string(&physical_path).map_err(|error| {
            let diagnostic_path = match project_roots {
                Some(_) => PathBuf::from(logical_path.as_str()),
                None => physical_path.clone(),
            };
            SourceLoadError::unreadable(diagnostic_path, error.to_string())
        })?;
        Ok(SourceFile::new_with_coordinates(
            physical_path,
            text,
            identity_path,
            PathBuf::from(logical_path.as_str()),
        ))
    }
}

fn canonical_root(root: &Path) -> Result<PathBuf, SourceLoadError> {
    root.canonicalize()
        .map_err(|error| SourceLoadError::unreadable(root.to_path_buf(), error.to_string()))
}

#[derive(Debug, Clone)]
struct SourcePath {
    path: PathBuf,
    identity_path: PathBuf,
}

fn source_paths(
    root: &Path,
    containment_root: Option<&Path>,
) -> Vec<Result<SourcePath, SourceLoadError>> {
    if root.is_file() {
        return vec![Ok(SourcePath {
            path: root.to_path_buf(),
            identity_path: root
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| root.into()),
        })];
    }

    if !root.exists() {
        return vec![Ok(SourcePath {
            path: root.to_path_buf(),
            identity_path: root
                .file_name()
                .map(PathBuf::from)
                .unwrap_or_else(|| root.into()),
        })];
    }

    let mut paths = Vec::new();
    let mut visited = BTreeSet::new();
    collect_source_files(root, root, containment_root, &mut visited, &mut paths);
    paths.sort_by(|left, right| source_path_result_path(left).cmp(source_path_result_path(right)));
    paths
}

fn collect_source_files(
    root: &Path,
    directory: &Path,
    containment_root: Option<&Path>,
    visited: &mut BTreeSet<PathBuf>,
    paths: &mut Vec<Result<SourcePath, SourceLoadError>>,
) {
    let canonical_directory = match directory.canonicalize() {
        Ok(path) => path,
        Err(error) => {
            paths.push(source_path_for_unreadable_directory(root, directory, error));
            return;
        }
    };
    if containment_root.is_some_and(|root| !canonical_directory.starts_with(root)) {
        paths.push(Err(SourceLoadError::unsafe_source_path(
            directory.to_path_buf(),
            "source directory resolves outside the configured documentation root",
        )));
        return;
    }
    if !visited.insert(canonical_directory) {
        return;
    }

    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) => {
            paths.push(source_path_for_unreadable_directory(root, directory, error));
            return;
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_source_files(root, &path, containment_root, visited, paths);
        } else if is_source_path(&path) {
            let identity_path = path
                .strip_prefix(root)
                .map(PathBuf::from)
                .unwrap_or_else(|_| path.clone());
            paths.push(Ok(SourcePath {
                path,
                identity_path,
            }));
        }
    }
}

fn source_path_for_unreadable_directory(
    _root: &Path,
    directory: &Path,
    error: io::Error,
) -> Result<SourcePath, SourceLoadError> {
    Err(SourceLoadError::unreadable_directory(
        directory.to_path_buf(),
        error.to_string(),
    ))
}

fn source_path_result_path(result: &Result<SourcePath, SourceLoadError>) -> &Path {
    match result {
        Ok(source_path) => &source_path.path,
        Err(load_error) => &load_error.path,
    }
}

fn is_source_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| SOURCE_EXTENSIONS.contains(&extension))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    #[cfg(unix)]
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;
    use crate::domain::ports::source_provider::SourceLoadErrorKind;

    #[test]
    fn source_paths_preserves_unreadable_directory_as_load_error() {
        let root = Path::new("/workspace");
        let blocked = root.join("blocked");
        let error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");

        let result = source_path_for_unreadable_directory(root, &blocked, error);

        let load_error = result.expect_err("unreadable directory should be a load error");
        assert_eq!(load_error.path, blocked);
        assert_eq!(load_error.kind, SourceLoadErrorKind::UnreadableDirectory);
        assert!(load_error.message.contains("permission denied"));
    }

    #[test]
    fn unreadable_source_uses_project_logical_path_and_preserves_standalone_path() {
        let project = tempfile::tempdir().expect("project");
        let docs = project.path().join("docs");
        fs::create_dir(&docs).expect("docs");
        let source = docs.join("invalid.adoc");
        fs::write(&source, [0xff, 0xfe]).expect("invalid UTF-8 source");
        for (provider, expected_path) in [
            (
                FsSourceProvider::for_project(docs.clone(), project.path().to_path_buf(), docs),
                PathBuf::from("docs/invalid.adoc"),
            ),
            (
                FsSourceProvider::new(source.clone()),
                source.canonicalize().expect("physical path"),
            ),
        ] {
            let error = provider
                .load_sources()
                .pop()
                .expect("source result")
                .expect_err("unreadable source");
            assert_eq!(error.kind, SourceLoadErrorKind::Unreadable);
            assert_eq!(error.path, expected_path);
        }
    }

    #[test]
    fn project_provider_assigns_docs_relative_identity_and_project_relative_logical_path() {
        let temp_root = tempfile::tempdir().expect("temp project");
        let docs_root = temp_root.path().join("knowledge");
        fs::create_dir_all(docs_root.join("billing")).expect("create docs");
        let physical_path = docs_root.join("billing/guide.adoc");
        fs::write(&physical_path, "# Billing\n").expect("write source");

        let sources = FsSourceProvider::for_project(
            docs_root.clone(),
            temp_root.path().to_path_buf(),
            docs_root,
        )
        .load_sources();
        let source = sources
            .into_iter()
            .next()
            .expect("one source")
            .expect("load");

        assert_eq!(
            source.physical_path,
            physical_path.canonicalize().expect("canonical")
        );
        assert_eq!(source.identity_path, PathBuf::from("billing/guide.adoc"));
        assert_eq!(
            source.logical_path,
            PathBuf::from("knowledge/billing/guide.adoc")
        );
    }

    #[cfg(unix)]
    #[test]
    fn project_provider_rejects_source_symlink_that_escapes_docs_root() {
        use std::os::unix::fs::symlink;

        let temp_root = tempfile::tempdir().expect("temp project");
        let docs_root = temp_root.path().join("docs");
        fs::create_dir(&docs_root).expect("create docs");
        let outside = temp_root.path().join("outside.adoc");
        fs::write(&outside, "# Secret\n").expect("write outside source");
        symlink(&outside, docs_root.join("inside.adoc")).expect("create symlink");

        let results = FsSourceProvider::for_project(
            docs_root.clone(),
            temp_root.path().to_path_buf(),
            docs_root,
        )
        .load_sources();

        assert!(matches!(
            results.as_slice(),
            [Err(SourceLoadError {
                kind: SourceLoadErrorKind::UnsafeSourcePath,
                ..
            })]
        ));
    }

    #[test]
    fn project_provider_rejects_selected_root_outside_docs_even_when_empty() {
        let temp_root = tempfile::tempdir().expect("temp project");
        let docs_root = temp_root.path().join("docs");
        let outside_root = temp_root.path().join("outside");
        fs::create_dir(&docs_root).expect("create docs");
        fs::create_dir(&outside_root).expect("create outside root");

        let results = FsSourceProvider::for_project(
            outside_root.clone(),
            temp_root.path().to_path_buf(),
            docs_root,
        )
        .load_sources();

        assert!(matches!(
            results.as_slice(),
            [Err(SourceLoadError {
                kind: SourceLoadErrorKind::UnsafeSourcePath,
                path,
                ..
            })] if path == &outside_root
        ));
    }

    #[cfg(unix)]
    #[test]
    fn project_provider_rejects_source_directory_symlink_that_escapes_docs_root() {
        use std::os::unix::fs::symlink;

        let temp_root = tempfile::tempdir().expect("temp project");
        let docs_root = temp_root.path().join("docs");
        let outside_root = temp_root.path().join("outside");
        fs::create_dir(&docs_root).expect("create docs");
        fs::create_dir(&outside_root).expect("create outside root");
        fs::write(outside_root.join("secret.adoc"), "# Secret\n").expect("write secret");
        symlink(&outside_root, docs_root.join("linked")).expect("create directory symlink");

        let results = FsSourceProvider::for_project(
            docs_root.clone(),
            temp_root.path().to_path_buf(),
            docs_root,
        )
        .load_sources();

        assert!(matches!(
            results.as_slice(),
            [Err(SourceLoadError {
                kind: SourceLoadErrorKind::UnsafeSourcePath,
                ..
            })]
        ));
    }

    #[cfg(unix)]
    #[test]
    fn fs_source_provider_reports_read_dir_failure_as_unreadable_directory() {
        let temp_root = TempRoot::new("adoc-unreadable-directory");
        let root = temp_root.path();
        fs::write(root.join("readable.adoc"), "# Readable\n").expect("write readable source");
        let blocked = root.join("blocked");
        fs::create_dir(&blocked).expect("create blocked directory");
        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o000))
            .expect("make blocked directory unreadable");

        let results = FsSourceProvider::new(root.to_path_buf()).load_sources();

        fs::set_permissions(&blocked, fs::Permissions::from_mode(0o700))
            .expect("restore blocked directory permissions before assertions");

        let load_error = results
            .iter()
            .find_map(|result| match result {
                Err(error) if error.kind == SourceLoadErrorKind::UnreadableDirectory => Some(error),
                _ => None,
            })
            .expect("provider should preserve read_dir failure as unreadable directory");

        assert_eq!(load_error.path, blocked);
        assert!(
            load_error
                .message
                .to_ascii_lowercase()
                .contains("permission denied"),
            "expected real permission error, got: {}",
            load_error.message
        );
        assert!(
            !load_error
                .message
                .to_ascii_lowercase()
                .contains("is a directory"),
            "directory traversal failure must not be reported through read_to_string: {}",
            load_error.message
        );
    }

    #[cfg(unix)]
    struct TempRoot {
        path: PathBuf,
    }

    #[cfg(unix)]
    impl TempRoot {
        fn new(label: &str) -> Self {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system time is after unix epoch")
                .as_nanos();
            let path =
                std::env::temp_dir().join(format!("{}-{}-{}", label, std::process::id(), unique));
            fs::create_dir(&path).expect("create temp root");
            Self { path }
        }

        fn path(&self) -> &Path {
            &self.path
        }
    }

    #[cfg(unix)]
    impl Drop for TempRoot {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
