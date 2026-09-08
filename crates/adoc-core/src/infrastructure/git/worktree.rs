//! `SnapshotWorkspaceProvider` adapter backed by the system `git` binary.
//!
//! Uses `git worktree add --detach` to materialize the target ref into a
//! temporary directory under `std::env::temp_dir()`, and `git worktree
//! remove --force` on drop. The application layer never sees the
//! intermediate worktree — it only receives a `SnapshotWorkspace` handle.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::domain::ports::snapshot_workspace::{
    SnapshotError, SnapshotSelector, SnapshotWorkspace, SnapshotWorkspaceProvider,
};

use super::error::GitError;
use super::util::clear_git_env;

/// V3.1 git-CLI adapter for [`SnapshotWorkspaceProvider`].
pub(crate) struct GitWorktreeProvider {
    repo_root: PathBuf,
    expected_workdir_head: Option<String>,
    hardened: bool,
}

impl GitWorktreeProvider {
    pub(crate) fn new(repo_root: impl Into<PathBuf>) -> Self {
        Self {
            repo_root: repo_root.into(),
            expected_workdir_head: None,
            hardened: false,
        }
    }

    pub(crate) fn with_expected_workdir_head(mut self, sha: String) -> Self {
        self.expected_workdir_head = Some(sha);
        self
    }
}

impl SnapshotWorkspaceProvider for GitWorktreeProvider {
    fn checkout(&self, selector: &SnapshotSelector) -> Result<SnapshotWorkspace, SnapshotError> {
        match selector {
            SnapshotSelector::Workdir => {
                if let Some(expected) = &self.expected_workdir_head {
                    verify_head(&self.repo_root, expected, self.hardened)?;
                }
                Ok(SnapshotWorkspace::workdir(self.repo_root.clone()))
            }
            SnapshotSelector::GitRef(spec) => self.checkout_ref(spec.as_str()),
        }
    }
}

impl GitWorktreeProvider {
    fn checkout_ref(&self, spec: &str) -> Result<SnapshotWorkspace, SnapshotError> {
        let sha = resolve_ref(&self.repo_root, spec, self.hardened)?;
        if self.hardened && sha != spec {
            return Err(SnapshotError::UnresolvableRef {
                spec: spec.into(),
                reason: "exact commit mismatch".into(),
            });
        }
        let project_prefix = project_prefix(&self.repo_root, self.hardened)?;

        let tmp = generate_worktree_path();

        let repo_root = self.repo_root.clone();
        let tmp_for_cleanup = tmp.clone();
        let hardened = self.hardened;
        let cleanup = Box::new(move || {
            run_git_worktree_remove(&repo_root, &tmp_for_cleanup, hardened);
            // Best-effort fallback in case `git worktree remove` left
            // residue (e.g. partial worktree-add).
            let _ = fs::remove_dir_all(&tmp_for_cleanup);
        });

        let workspace = SnapshotWorkspace::with_cleanup(tmp.join(project_prefix), cleanup);
        add_worktree(&self.repo_root, &tmp, &sha, self.hardened)?;
        verify_head(&tmp, &sha, self.hardened)?;
        Ok(workspace)
    }
}

impl GitWorktreeProvider {
    pub(crate) fn for_migration(
        repo: &Path,
        revision: &str,
    ) -> Result<Self, crate::MigrationError> {
        use crate::MigrationError;
        let resolved =
            resolve_ref(repo, revision, true).map_err(|_| MigrationError::SnapshotUnavailable)?;
        if resolved != revision {
            return Err(MigrationError::SnapshotUnavailable);
        }
        if !project_prefix(repo, true)
            .map_err(|_| MigrationError::SnapshotUnavailable)?
            .as_os_str()
            .is_empty()
        {
            return Err(MigrationError::UnsafeSource);
        }
        let tree = run_git(repo, &["ls-tree", "-rz", "--full-tree", revision], true)
            .map_err(|_| MigrationError::SnapshotUnavailable)?;
        if !tree.status.success() {
            return Err(MigrationError::SnapshotUnavailable);
        }
        for entry in tree.stdout.split(|b| *b == 0).filter(|e| !e.is_empty()) {
            let Some(tab) = entry.iter().position(|b| *b == b'\t') else {
                return Err(MigrationError::UnsafeSource);
            };
            let path = &entry[tab + 1..];
            // ponytail: refuse all attributes, symlinks and gitlinks; add a verified
            // raw-blob materializer when migrations need those repository features.
            if !(entry.starts_with(b"100644 ") || entry.starts_with(b"100755 "))
                || path
                    .rsplit(|b| *b == b'/')
                    .next()
                    .is_some_and(|name| name.eq_ignore_ascii_case(b".gitattributes"))
            {
                return Err(MigrationError::UnsafeSource);
            }
        }
        let attributes = run_git(repo, &["rev-parse", "--git-path", "info/attributes"], true)
            .map_err(|_| MigrationError::UnsafeSource)?;
        if !attributes.status.success() {
            return Err(MigrationError::UnsafeSource);
        }
        let path = std::str::from_utf8(&attributes.stdout)
            .map_err(|_| MigrationError::UnsafeSource)?
            .trim();
        match fs::read(repo.join(path)) {
            Ok(bytes) if !bytes.is_empty() => return Err(MigrationError::UnsafeSource),
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => {
                return Err(MigrationError::UnsafeSource);
            }
            _ => {}
        }
        Ok(Self {
            repo_root: repo.to_path_buf(),
            expected_workdir_head: None,
            hardened: true,
        })
    }
}

fn git_command(hardened: bool) -> Command {
    let mut command = Command::new("git");
    if hardened {
        command
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_NO_REPLACE_OBJECTS", "1")
            .env("GIT_ATTR_NOSYSTEM", "1")
            .env("GIT_TERMINAL_PROMPT", "0")
            .args([
                "-c",
                "core.hooksPath=/dev/null",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "core.sparseCheckout=false",
                "-c",
                "core.sparseCheckoutCone=false",
                "-c",
                "core.attributesFile=/dev/null",
                "-c",
                "core.autocrlf=false",
                "-c",
                "core.symlinks=true",
                "-c",
                "submodule.recurse=false",
                "-c",
                "protocol.allow=never",
            ]);
    }
    command
}

fn project_prefix(project_root: &Path, hardened: bool) -> Result<PathBuf, SnapshotError> {
    let output = run_git(project_root, &["rev-parse", "--show-prefix"], hardened)?;
    if !output.status.success() {
        return Err(SnapshotError::ProviderUnavailable {
            reason: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }
    let value =
        std::str::from_utf8(&output.stdout).map_err(|_| SnapshotError::ProviderUnavailable {
            reason: "git returned a non-UTF-8 project prefix".to_string(),
        })?;
    // Git's empty prefix is the repository-root project; joining it preserves `tmp`.
    Ok(PathBuf::from(value.trim_end_matches(['\r', '\n'])))
}

fn verify_head(repo_root: &Path, expected: &str, hardened: bool) -> Result<(), SnapshotError> {
    let output = run_git(
        repo_root,
        &["rev-parse", "--verify", "HEAD^{commit}"],
        hardened,
    )?;
    if !output.status.success() || !head_output_matches(&output.stdout, expected) {
        return Err(SnapshotError::ProviderUnavailable {
            reason: format!("materialized HEAD did not match intended revision {expected}"),
        });
    }
    Ok(())
}

fn head_output_matches(stdout: &[u8], expected: &str) -> bool {
    stdout
        .strip_suffix(b"\n")
        .map(|line| line.strip_suffix(b"\r").unwrap_or(line))
        == Some(expected.as_bytes())
}

fn resolve_ref(repo_root: &Path, spec: &str, hardened: bool) -> Result<String, SnapshotError> {
    let output = run_git(
        repo_root,
        &["rev-parse", "--verify", &format!("{spec}^{{commit}}")],
        hardened,
    )?;
    if !output.status.success() {
        return Err(GitError::RefNotResolvable {
            spec: spec.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
        .into());
    }
    let sha = std::str::from_utf8(&output.stdout)
        .map_err(|_| SnapshotError::ProviderUnavailable {
            reason: "git returned non-UTF-8 revision output".to_string(),
        })?
        .trim();
    if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SnapshotError::UnresolvableRef {
            spec: spec.to_string(),
            reason: "git returned an invalid commit id".to_string(),
        });
    }
    Ok(sha.to_ascii_lowercase())
}

fn add_worktree(
    repo_root: &Path,
    tmp: &Path,
    spec: &str,
    hardened: bool,
) -> Result<(), SnapshotError> {
    let output = run_git(
        repo_root,
        &[
            "worktree",
            "add",
            "--detach",
            tmp.to_str().ok_or_else(|| {
                SnapshotError::from(GitError::WorktreeCreate {
                    tmp: tmp.to_path_buf(),
                    stderr: "worktree path is not valid UTF-8".to_string(),
                })
            })?,
            spec,
        ],
        hardened,
    )?;
    if !output.status.success() {
        return Err(GitError::WorktreeCreate {
            tmp: tmp.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
        .into());
    }
    Ok(())
}

fn run_git_worktree_remove(repo_root: &Path, tmp: &Path, hardened: bool) {
    // Drop-time cleanup: errors are absorbed (cannot propagate from Drop)
    // but recorded as best-effort. The fallback `fs::remove_dir_all` in
    // the caller covers cases where `git worktree remove` itself fails.
    let Some(tmp_str) = tmp.to_str() else {
        return;
    };
    let mut command = git_command(hardened);
    command
        .arg("-C")
        .arg(repo_root)
        .args(["worktree", "remove", "--force", tmp_str]);
    clear_git_env(&mut command);
    let _ = command.output();
}

fn run_git(repo_root: &Path, args: &[&str], hardened: bool) -> Result<Output, SnapshotError> {
    let mut command = git_command(hardened);
    command.arg("-C").arg(repo_root);
    for arg in args {
        command.arg(arg);
    }
    clear_git_env(&mut command);
    command
        .output()
        .map_err(|source| match source.kind() {
            std::io::ErrorKind::NotFound => GitError::GitNotFound,
            _ => GitError::CommandSpawn {
                program: "git".to_string(),
                source,
            },
        })
        .map_err(SnapshotError::from)
}

fn generate_worktree_path() -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    // The nonce only disambiguates paths across pid reuse; pid + counter
    // already guarantee uniqueness within a process, so a clock set before
    // the epoch degrades to a fixed nonce instead of crashing the command.
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos());
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "adoc-worktree-{}-{counter}-{nonce}",
        std::process::id()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(cwd: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(cwd).args(args);
        clear_git_env(&mut command);
        let output = command
            .output()
            .unwrap_or_else(|error| panic!("spawn `git {args:?}`: {error}"));
        assert!(
            output.status.success(),
            "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    struct Repo {
        _tempdir: tempfile::TempDir,
        root: PathBuf,
    }

    impl Repo {
        fn new() -> Self {
            let temp = tempfile::Builder::new()
                .prefix("adoc-git-test-")
                .tempdir()
                .expect("tempdir");
            let root = temp.path().to_path_buf();
            run(&root, &["init", "--initial-branch=main"]);
            run(&root, &["config", "user.email", "test@adoc.dev"]);
            run(&root, &["config", "user.name", "adoc tests"]);
            run(&root, &["config", "commit.gpgsign", "false"]);
            fs::write(root.join("hello.txt"), "hello world\n").expect("write hello.txt");
            run(&root, &["add", "-A"]);
            run(&root, &["commit", "-m", "initial"]);
            Self {
                _tempdir: temp,
                root,
            }
        }
    }

    #[test]
    fn workdir_selector_returns_repo_root_without_cleanup() {
        let repo = Repo::new();
        let provider = GitWorktreeProvider::new(repo.root.clone());

        let workspace = provider
            .checkout(&SnapshotSelector::Workdir)
            .expect("workdir checkout succeeds");

        assert_eq!(workspace.path(), repo.root.as_path());
        // Drop is a no-op for workdir; nothing should be removed.
        drop(workspace);
        assert!(repo.root.join("hello.txt").exists());
    }

    #[test]
    fn workdir_selector_rejects_a_head_mismatch() {
        let repo = Repo::new();
        let provider =
            GitWorktreeProvider::new(repo.root.clone()).with_expected_workdir_head("0".repeat(40));

        let error = provider
            .checkout(&SnapshotSelector::Workdir)
            .expect_err("mismatched workdir HEAD must fail");

        assert!(format!("{error}").contains("did not match intended revision"));
    }

    #[test]
    fn head_verification_accepts_only_lf_or_crlf_terminated_sha() {
        let expected = "0123456789abcdef0123456789abcdef01234567";

        assert!(head_output_matches(
            format!("{expected}\n").as_bytes(),
            expected
        ));
        assert!(head_output_matches(
            format!("{expected}\r\n").as_bytes(),
            expected
        ));
        assert!(!head_output_matches(
            format!("{expected} \n").as_bytes(),
            expected
        ));
    }

    #[test]
    fn gitref_head_returns_separate_path_with_committed_file_present() {
        use crate::domain::ports::snapshot_workspace::GitRef;
        let repo = Repo::new();
        let provider = GitWorktreeProvider::new(repo.root.clone());

        let workspace = provider
            .checkout(&SnapshotSelector::GitRef(GitRef::new("HEAD")))
            .expect("HEAD checkout succeeds");

        assert_ne!(workspace.path(), repo.root.as_path());
        assert!(workspace.path().exists());
        let hello = fs::read_to_string(workspace.path().join("hello.txt"))
            .expect("hello.txt exists in worktree");
        assert_eq!(hello, "hello world\n");
    }

    #[test]
    fn dropping_gitref_workspace_removes_the_temporary_worktree() {
        use crate::domain::ports::snapshot_workspace::GitRef;
        let repo = Repo::new();
        let provider = GitWorktreeProvider::new(repo.root.clone());

        let tmp_path = {
            let workspace = provider
                .checkout(&SnapshotSelector::GitRef(GitRef::new("HEAD")))
                .expect("HEAD checkout succeeds");
            let path = workspace.path().to_path_buf();
            assert!(path.exists());
            path
        };

        assert!(
            !tmp_path.exists(),
            "worktree directory must be cleaned up on drop"
        );

        // `git worktree list` should no longer reference the temp path.
        let mut listing_command = Command::new("git");
        listing_command
            .arg("-C")
            .arg(&repo.root)
            .args(["worktree", "list"]);
        clear_git_env(&mut listing_command);
        let listing = listing_command.output().expect("git worktree list runs");
        let listing = String::from_utf8_lossy(&listing.stdout);
        assert!(
            !listing.contains(tmp_path.to_str().expect("path is utf-8")),
            "git worktree list still mentions cleaned-up worktree: {listing}"
        );
    }

    #[test]
    fn unresolvable_ref_returns_ref_not_resolvable_error() {
        use crate::domain::ports::snapshot_workspace::GitRef;
        let repo = Repo::new();
        let provider = GitWorktreeProvider::new(repo.root.clone());

        let error = provider
            .checkout(&SnapshotSelector::GitRef(GitRef::new(
                "definitely-not-a-real-ref",
            )))
            .expect_err("bad ref must error");

        match error {
            SnapshotError::UnresolvableRef { spec, .. } => {
                assert_eq!(spec, "definitely-not-a-real-ref");
            }
            other => panic!("expected UnresolvableRef, got: {other:?}"),
        }
    }

    #[test]
    fn missing_repo_dir_propagates_command_failure() {
        use crate::domain::ports::snapshot_workspace::GitRef;
        let provider =
            GitWorktreeProvider::new(PathBuf::from("/this/path/should/not/exist/anywhere"));

        let error = provider
            .checkout(&SnapshotSelector::GitRef(GitRef::new("HEAD")))
            .expect_err("non-repo must error");

        // The exact variant depends on git's stderr; we just need the
        // error to render a non-empty message via the adapter mapping.
        assert!(!format!("{error}").is_empty());
    }
}
