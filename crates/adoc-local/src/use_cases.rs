use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::num::NonZeroUsize;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use adoc_core::{
    ArtifactInspection, ArtifactLoadStatus, BuildArtifacts, BuildEmbeddingMode,
    BuildInput as CoreBuildInput, ChangeAssessmentEnvelope,
    ChangeAssessmentInput as CoreChangeAssessmentInput, CompileInput, CompileResult,
    ContradictionsEnvelope, Diagnostic, DiagnosticCode, EmbeddingProviderSelection, GitRef,
    GraphArtifactInspectionInput, GraphDirection, GraphInput as CoreGraphInput, GraphRelationKind,
    GraphSession, GraphTraversalEnvelope, GraphTraversalQuery, GraphTraversalResult,
    ImpactedEnvelope, LocalProjectContext, ObjectDiffEnvelope,
    PatchApplyInput as CorePatchApplyInput, PatchApplyResult, PatchCheckResult, PatchInput,
    ProseRecord, RelPath, RepositoryBaselineEnvelope, RetrievalEntry, RetrievalEnvelope,
    RetrievalInput, RetrievalLoadResult, RetrievalRecord, ReviewEnvelope, ReviewError,
    ReviewInput as CoreReviewInput, SearchArtifactInspectionInput, SearchFilters, SearchMode,
    SearchQuery, SearchRecordScope, Severity, SnapshotSelector, StaleEnvelope,
    apply_patch_for_date as core_apply_patch_for_date, assess_changes_from_git,
    build_project_workspace_with_embedding_provider_for_date,
    build_workspace_with_embedding_provider_for_date, changed_files_from_git,
    changed_paths_strings, check_patch as core_check_patch,
    compile_project_workspace_with_anchor_root_for_date,
    compile_workspace_with_anchor_root_for_date, diff_objects, embed_query_with_embedding_provider,
    empty_contradictions_envelope, empty_impacted_envelope, empty_stale_envelope,
    evaluate_contradictions, evaluate_impacted, evaluate_stale, export_workspace,
    git_review_available, inspect_graph_artifact, inspect_search_artifact, load_graph_session,
    load_retrieval_session_with_embedding_provider, load_review_from_git,
    load_review_with_changed_files_from_git, migrate_workspace, parse_patch_from_path,
    parse_patch_from_value, patch_apply_refusal, repository_baseline_from_git, review_with_patch,
    search as core_search, traverse_graph, validate_changed_paths, why_object,
};
use adoc_core::{MigrateMode, MigrateReportEnvelope};
use serde::Serialize;

use crate::config::{CONFIG_FILE_NAME, config_path_present};
use crate::{EmbeddingsProvider, LocalContext, LocalError, PathPolicy, ProjectConfig};

const DEFAULT_GRAPH_ARTIFACT_PATH: &str = "dist/docs.graph.json";
const DEFAULT_SEARCH_ARTIFACT_PATH: &str = "dist/docs.search.json";
const DEFAULT_HTML_ARTIFACT_PATH: &str = "dist/docs.html";
const PROJECT_STATUS_SCHEMA_VERSION: &str = "adoc.project.status.v0";
const INIT_INDEX_PATH: &str = "docs/index.adoc";
const INIT_CONFIG_TEMPLATE: &str = "\
version: 1
mode: strict
docs_path: docs
outputs:
  dir: dist
embeddings:
  provider: local
";

#[derive(Debug, Clone, Serialize)]
pub struct InitOutcome {
    pub created: Vec<PathBuf>,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct CheckInput {
    pub path: Option<PathBuf>,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckOutcome {
    pub diagnostics: Vec<Diagnostic>,
    pub exit_code: i32,
}

/// E1.7: receipt-mode check. `as_of` is mandatory (receipts never read the
/// wall clock) and the runtime binary digest is the invoking harness's
/// attested input — see `adoc_core::run_validation_runtime`.
#[derive(Debug, Clone)]
pub struct CheckReceiptInput {
    pub path: Option<PathBuf>,
    pub as_of: chrono::NaiveDate,
    pub runtime_version: String,
    pub runtime_binary_digest: String,
    /// Exact source namespace/revision/Binding/ACL/config invocation manifest.
    pub source_invocation: Option<PathBuf>,
    /// Graph artifact validated against the recompiled source (exact-match
    /// version gate + governed-meaning drift check, E1.7.T4).
    pub context_artifact: Option<PathBuf>,
    /// E3.1 semantic context validated and digest-bound by receipt mode.
    pub semantic_context: Option<PathBuf>,
    /// Trusted exact bindings expected in the semantic context.
    pub semantic_context_expectations: Option<adoc_core::SemanticContextExpectedBindings>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckReceiptOutcome {
    pub receipt: adoc_core::ValidationReceipt,
    pub diagnostics: Vec<Diagnostic>,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct MigrateInput {
    pub path: Option<PathBuf>,
    pub write: bool,
    pub force: bool,
    /// V8.1.4: export strict prose-mode `.adoc` back to `.md` instead of
    /// importing (ADR-0043 §5).
    pub export: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MigrateOutcome {
    pub report: MigrateReportEnvelope,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct BuildInput {
    /// Explicit local HTML audience; trusted gateway policy takes precedence.
    pub audience: Option<String>,
    pub path: Option<PathBuf>,
    pub out: Option<PathBuf>,
    pub no_embeddings: bool,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildOutputs {
    pub html: PathBuf,
    pub graph: PathBuf,
    pub search: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BuildOutcome {
    pub diagnostics: Vec<Diagnostic>,
    pub outputs: Option<BuildOutputs>,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct WhyInput {
    pub object_id: String,
    pub artifact: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResolvedRetrievalRecord {
    pub record: RetrievalRecord,
    pub related_statuses: BTreeMap<String, Option<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WhyOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub artifact: PathBuf,
    pub records: Vec<ResolvedRetrievalRecord>,
    pub diagnostics: Vec<Diagnostic>,
    #[serde(skip)]
    pub duration: Duration,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct GraphInput {
    pub object_id: String,
    pub artifact: Option<PathBuf>,
    pub relation: Option<GraphRelationKind>,
    pub direction: Option<GraphDirection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GraphOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub envelope: GraphTraversalEnvelope,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct StaleInput {
    pub artifact: Option<PathBuf>,
    /// `--within <N>d` horizon in days; `None` disables `expiring_soon`.
    pub within_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub envelope: StaleEnvelope,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct ContradictionsInput {
    pub artifact: Option<PathBuf>,
    /// `--all`: include `resolved` and `dismissed` contradictions in the
    /// listing (never affects `contradicted_claims`).
    pub all: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContradictionsOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub envelope: ContradictionsEnvelope,
    pub exit_code: i32,
}

/// V6.3 — the two mutually exclusive `adoc impacted-by` input shapes. The
/// XOR is enforced at the interface layer (clap / MCP argument validation);
/// this enum makes the exclusivity structural here.
#[derive(Debug, Clone)]
pub enum ImpactedChangedSet {
    /// Explicit repo-relative changed paths (`adoc impacted-by <path>...`).
    Paths(Vec<String>),
    /// Derive the changed set from git: `<git-ref>` vs the working tree
    /// (`adoc impacted-by --ref <git-ref>`).
    GitRef(String),
}

#[derive(Debug, Clone)]
pub struct ImpactedInput {
    pub artifact: Option<PathBuf>,
    pub changed: ImpactedChangedSet,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImpactedOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub envelope: ImpactedEnvelope,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct SearchInput {
    pub query: String,
    pub artifact: Option<PathBuf>,
    pub search_artifact: Option<PathBuf>,
    pub semantic: bool,
    pub lexical: bool,
    pub kind: Option<String>,
    pub status: Option<String>,
    pub owner: Option<String>,
    pub source_path: Option<String>,
    pub related_to: Option<String>,
    pub relation: Option<GraphRelationKind>,
    pub direction: Option<GraphDirection>,
    pub top: NonZeroUsize,
    /// V1.7.1 (ADR-0040): which record types the search returns. Structural —
    /// the interface layers (clap conflicts, MCP argument validation) map
    /// their flag pairs onto this enum, so an invalid combination cannot
    /// reach the use case.
    pub scope: SearchRecordScope,
}

/// V1.7.1: one search result entry — a Knowledge Object record enriched with
/// its relation-target statuses, or a prose record (which has no relations to
/// enrich). Serializes with the same `record_type` tag as the wire envelope.
// Size asymmetry follows the `RetrievalEntry` precedent: the record carries
// all fields inline; boxing adds indirection for no wire benefit.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "record_type", rename_all = "snake_case")]
pub enum ResolvedSearchEntry {
    KnowledgeObject(ResolvedRetrievalRecord),
    Prose(ProseRecord),
}

#[derive(Debug, Clone, Serialize)]
pub struct SearchOutcome {
    #[serde(skip)]
    pub read_access: adoc_core::ReadAccess,
    pub envelope: RetrievalEnvelope,
    pub records: Vec<ResolvedSearchEntry>,
    pub diagnostics: Vec<Diagnostic>,
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct PatchCheckInput {
    pub patch_path: PathBuf,
    pub artifact: Option<PathBuf>,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchCheckOutcome {
    #[serde(flatten)]
    pub result: PatchCheckResult,
    pub exit_code: i32,
}

/// V6.4 — patch source for `adoc patch --apply` and the MCP
/// `adoc_patch_apply` tool. Mirrors [`ReviewPatchSource`] (path vs inline
/// JSON) so the same driving adapters can populate either variant.
#[derive(Debug, Clone)]
pub enum PatchApplySource {
    Path(PathBuf),
    Inline(serde_json::Value),
}

#[derive(Debug, Clone)]
pub struct PatchApplyInput {
    pub patch: PatchApplySource,
    pub artifact: Option<PathBuf>,
    /// Recorded in the envelope's `trace.interface` (`"cli"` or `"mcp"`).
    pub interface: String,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PatchApplyOutcome {
    #[serde(flatten)]
    pub result: PatchApplyResult,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct DiffInput {
    pub base_ref: String,
    pub head_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DiffOutcome {
    #[serde(flatten)]
    pub envelope: ObjectDiffEnvelope,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct ReviewInput {
    pub base_ref: String,
    pub head_ref: Option<String>,
    /// V3.7 — optional patch source threaded through into
    /// [`adoc_core::review_with_patch`]. `None` produces the V3.3/V3.4/V3.6
    /// envelope unchanged.
    pub patch: Option<ReviewPatchSource>,
}

/// V3.7 — orchestration-layer patch source for `adoc review --patch` and the
/// equivalent MCP parameter. Mirrors V2.1's [`PatchInput`] shape (path vs
/// inline JSON) so the same driving adapters can populate either variant.
#[derive(Debug, Clone)]
pub enum ReviewPatchSource {
    Path(PathBuf),
    Inline(serde_json::Value),
}

#[derive(Debug, Clone, Serialize)]
pub struct ReviewOutcome {
    #[serde(flatten)]
    pub envelope: ReviewEnvelope,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct AssessmentInput {
    pub base_ref: String,
    pub head_ref: Option<String>,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssessmentOutcome {
    #[serde(flatten)]
    pub envelope: ChangeAssessmentEnvelope,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Debug, Clone)]
pub struct RepositoryBaselineInput {
    pub git_ref: String,
    pub as_of: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepositoryBaselineOutcome {
    #[serde(flatten)]
    pub envelope: RepositoryBaselineEnvelope,
    #[serde(skip)]
    pub exit_code: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatusRefresh {
    None,
    Check,
    Build,
}

#[derive(Debug, Clone)]
pub struct ProjectStatusInput {
    pub refresh: ProjectStatusRefresh,
    pub no_embeddings: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusConfig {
    pub discovered: bool,
    pub path: Option<PathBuf>,
    pub embeddings_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusPaths {
    pub docs: PathBuf,
    pub html: Option<PathBuf>,
    pub graph: PathBuf,
    pub search: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusRefreshReport {
    pub requested: ProjectStatusRefresh,
    pub exit_code: Option<i32>,
    pub diagnostics: Vec<Diagnostic>,
    pub outputs: Option<BuildOutputs>,
}

pub type ProjectArtifactLoadStatus = ArtifactLoadStatus;
pub type ProjectArtifactStatus = ArtifactInspection;

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusArtifacts {
    pub graph: ProjectArtifactStatus,
    pub search: ProjectArtifactStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusReadiness {
    pub retrieval: bool,
    pub semantic_search: bool,
    pub patch_validation: bool,
    pub review: bool,
    /// V6.4 TB4 (ADR-0037): `true` only when the project opted into MCP
    /// patch apply via `mcp: { patch_apply: enabled }`. Agents check this
    /// before constructing a patch for apply.
    pub patch_apply_enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProjectStatusOutcome {
    pub schema_version: &'static str,
    pub project_root: PathBuf,
    pub config: ProjectStatusConfig,
    pub paths: ProjectStatusPaths,
    pub refresh: ProjectStatusRefreshReport,
    pub artifacts: ProjectStatusArtifacts,
    pub readiness: ProjectStatusReadiness,
    pub exit_code: i32,
}

/// The Local Workflow Layer's command surface: one method per local
/// operation, on the one type every adapter already holds. Bodies live in
/// the private `*_with_context` functions below; the `PathPolicy` generic
/// remains the test seam.
impl<P> LocalContext<P>
where
    P: PathPolicy,
{
    #[tracing::instrument(name = "adoc.init", level = "info", skip_all)]
    pub fn init(&self) -> Result<InitOutcome, LocalError> {
        init_with_context(self)
    }

    #[tracing::instrument(name = "adoc.check", level = "info", skip_all)]
    pub fn check(&self, input: CheckInput) -> Result<CheckOutcome, LocalError> {
        check_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.check_receipt", level = "info", skip_all)]
    pub fn check_receipt(
        &self,
        input: CheckReceiptInput,
    ) -> Result<CheckReceiptOutcome, LocalError> {
        check_receipt_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.migrate", level = "info", skip_all)]
    pub fn migrate(&self, input: MigrateInput) -> Result<MigrateOutcome, LocalError> {
        migrate_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.build", level = "info", skip_all)]
    pub fn build(&self, input: BuildInput) -> Result<BuildOutcome, LocalError> {
        build_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.why", level = "info", skip_all)]
    pub fn why(&self, input: WhyInput) -> Result<WhyOutcome, LocalError> {
        why_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.graph", level = "info", skip_all)]
    pub fn graph(&self, input: GraphInput) -> Result<GraphOutcome, LocalError> {
        graph_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.stale", level = "info", skip_all)]
    pub fn stale(&self, input: StaleInput) -> Result<StaleOutcome, LocalError> {
        stale_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.contradictions", level = "info", skip_all)]
    pub fn contradictions(
        &self,
        input: ContradictionsInput,
    ) -> Result<ContradictionsOutcome, LocalError> {
        contradictions_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.impacted", level = "info", skip_all)]
    pub fn impacted(&self, input: ImpactedInput) -> Result<ImpactedOutcome, LocalError> {
        impacted_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.search", level = "info", skip_all)]
    pub fn search(&self, input: SearchInput) -> Result<SearchOutcome, LocalError> {
        search_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.patch_check", level = "info", skip_all)]
    pub fn patch_check(&self, input: PatchCheckInput) -> Result<PatchCheckOutcome, LocalError> {
        patch_check_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.patch_apply", level = "info", skip_all)]
    pub fn patch_apply(&self, input: PatchApplyInput) -> Result<PatchApplyOutcome, LocalError> {
        patch_apply_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.diff", level = "info", skip_all)]
    pub fn diff(&self, input: DiffInput) -> Result<DiffOutcome, LocalError> {
        diff_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.review", level = "info", skip_all)]
    pub fn review(&self, input: ReviewInput) -> Result<ReviewOutcome, LocalError> {
        review_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.assess_changes", level = "info", skip_all)]
    pub fn assess_changes(&self, input: AssessmentInput) -> Result<AssessmentOutcome, LocalError> {
        assess_changes_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.baseline", level = "info", skip_all)]
    pub fn repository_baseline(
        &self,
        input: RepositoryBaselineInput,
    ) -> Result<RepositoryBaselineOutcome, LocalError> {
        repository_baseline_with_context(self, input)
    }

    #[tracing::instrument(name = "adoc.project_status", level = "info", skip_all)]
    pub fn project_status(
        &self,
        input: ProjectStatusInput,
    ) -> Result<ProjectStatusOutcome, LocalError> {
        project_status_with_context(self, input)
    }
}

fn init_with_context<P>(context: &LocalContext<P>) -> Result<InitOutcome, LocalError>
where
    P: PathPolicy,
{
    let project_root = context
        .path_policy()
        .resolve_write_path(context.config_start())?;
    write_init_files(&project_root)?;
    Ok(InitOutcome {
        created: vec![
            project_root.join(CONFIG_FILE_NAME),
            project_root.join(INIT_INDEX_PATH),
        ],
        exit_code: 0,
    })
}

/// The check pipeline's resolved compile target, shared by `check` and
/// `check_receipt` so both validate exactly the same sources with the same
/// Evidence Anchor root (ADR-0048).
struct ResolvedCheckTarget {
    path: PathBuf,
    anchor_root: PathBuf,
    project: Option<adoc_core::LocalProjectContext>,
    config_path: Option<PathBuf>,
}

fn resolve_check_target<P>(
    context: &LocalContext<P>,
    path: Option<&Path>,
) -> Result<ResolvedCheckTarget, LocalError>
where
    P: PathPolicy,
{
    let path = path
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    // Check always discovers the config — even with an explicit path — so
    // Evidence Anchors (ADR-0048) resolve against the project root: the
    // discovered config's directory, else the context start. The explicit
    // path still wins for docs resolution. Only check threads the anchor
    // root; build, review, diff, and patch recompiles stay anchor-free.
    let config = discover_project_config_if(true, context.config_start())?;
    let anchor_root = config
        .as_ref()
        .and_then(|config| config.path.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| context.config_start().to_path_buf());
    let path = resolve_docs_path_with_config(path, config.as_ref())?;
    let path = context.path_policy().resolve_read_path(&path)?;
    let project = config
        .as_ref()
        .and_then(|config| project_context_for_selected_path(config, &path));
    Ok(ResolvedCheckTarget {
        path,
        anchor_root,
        project,
        config_path: config.map(|config| config.path),
    })
}

fn check_with_context<P>(
    context: &LocalContext<P>,
    input: CheckInput,
) -> Result<CheckOutcome, LocalError>
where
    P: PathPolicy,
{
    let evaluation_date = input
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let target = resolve_check_target(context, input.path.as_deref())?;
    let result = match target.project {
        Some(project) => compile_project_workspace_with_anchor_root_for_date(
            CompileInput { root: target.path },
            project,
            target.anchor_root,
            evaluation_date,
        ),
        None => compile_workspace_with_anchor_root_for_date(
            CompileInput { root: target.path },
            Some(target.anchor_root),
            evaluation_date,
        ),
    };
    let exit_code = if result.has_errors() { 1 } else { 0 };

    Ok(CheckOutcome {
        diagnostics: result.diagnostics,
        exit_code,
    })
}

fn check_receipt_with_context<P>(
    context: &LocalContext<P>,
    input: CheckReceiptInput,
) -> Result<CheckReceiptOutcome, LocalError>
where
    P: PathPolicy,
{
    let target = resolve_check_target(context, input.path.as_deref())?;
    let source_invocation = input
        .source_invocation
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let context_artifact = input
        .context_artifact
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let semantic_context = input
        .semantic_context
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let outcome = adoc_core::run_validation_runtime(adoc_core::ValidationRuntimeInput {
        root: target.path,
        project: target.project,
        anchor_root: target.anchor_root,
        evaluation_date: input.as_of,
        runtime_version: input.runtime_version,
        runtime_binary_digest: input.runtime_binary_digest,
        config_path: target.config_path,
        source_invocation,
        context_artifact,
        semantic_context,
        semantic_context_expectations: input.semantic_context_expectations,
    })
    .map_err(|source| LocalError::ValidationRuntime { source })?;
    let exit_code = match outcome.receipt.result() {
        adoc_core::ValidationResult::Pass => 0,
        adoc_core::ValidationResult::Fail => 1,
    };

    Ok(CheckReceiptOutcome {
        receipt: outcome.receipt,
        diagnostics: outcome.diagnostics,
        exit_code,
    })
}

fn migrate_with_context<P>(
    context: &LocalContext<P>,
    input: MigrateInput,
) -> Result<MigrateOutcome, LocalError>
where
    P: PathPolicy,
{
    let path = input
        .path
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let config = discover_project_config_if(path.is_none(), context.config_start())?;
    let path = resolve_docs_path_with_config(path, config.as_ref())?;
    let path = context.path_policy().resolve_read_path(&path)?;

    let mode = if input.write {
        MigrateMode::Write { force: input.force }
    } else {
        MigrateMode::DryRun
    };
    let result = if input.export {
        export_workspace(path, mode)
    } else {
        migrate_workspace(path, mode)
    };
    let written = input.write && !result.has_errors();
    if written {
        execute_migration_writes(&result.files)?;
    }

    let exit_code = if result.has_errors() { 1 } else { 0 };
    Ok(MigrateOutcome {
        report: MigrateReportEnvelope::new(result, written),
        exit_code,
    })
}

/// Two-phase `--write` execution (ADR-0043 §3): create every target first
/// (create-new; a failure removes the targets already created and aborts),
/// and only after all targets exist remove the sources. There is no
/// half-written success state; the committed sources make phase 2 failures
/// recoverable from git.
fn execute_migration_writes(files: &[adoc_core::MigratedFile]) -> Result<(), LocalError> {
    execute_migration_writes_with(files, write_new_target, |path| fs::remove_file(path))
}

fn execute_migration_writes_with(
    files: &[adoc_core::MigratedFile],
    mut create: impl FnMut(&Path, &[u8]) -> io::Result<()>,
    mut remove: impl FnMut(&Path) -> io::Result<()>,
) -> Result<(), LocalError> {
    let mut created: Vec<&Path> = Vec::with_capacity(files.len());
    for file in files {
        if let Err(source) = create(&file.target_path, file.target_text.as_bytes()) {
            for path in created {
                let _ = remove(path);
            }
            return Err(LocalError::WriteFailed {
                path: file.target_path.clone(),
                source,
            });
        }
        created.push(&file.target_path);
    }
    let mut removed: Vec<PathBuf> = Vec::with_capacity(files.len());
    for file in files {
        if let Err(source) = remove(&file.source_path) {
            return Err(LocalError::RemoveFailed {
                path: file.source_path.clone(),
                removed,
                source,
            });
        }
        removed.push(file.source_path.clone());
    }
    Ok(())
}

fn build_with_context<P>(
    context: &LocalContext<P>,
    input: BuildInput,
) -> Result<BuildOutcome, LocalError>
where
    P: PathPolicy,
{
    let evaluation_date = input
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let path = input
        .path
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let out = input
        .out
        .as_deref()
        .map(|path| context.path_policy().resolve_write_path(path))
        .transpose()?;
    let config = discover_project_config_if(true, context.config_start())?;
    let policy = context.retrieval_policy_override().cloned().or_else(|| {
        let configured = config
            .as_ref()
            .and_then(|config| config.retrieval_policy.clone());
        match input.audience {
            Some(audience) => {
                let mut policy = configured.unwrap_or_else(|| adoc_core::RetrievalPolicy {
                    audience: audience.clone(),
                    allowed_visibilities: ["public", "internal", "restricted"]
                        .map(str::to_string)
                        .into_iter()
                        .collect(),
                    excluded_object_ids: Default::default(),
                });
                policy.audience = audience;
                Some(policy)
            }
            None => configured,
        }
    });
    let path = resolve_docs_path_with_config(path, config.as_ref())?;
    let path = context.path_policy().resolve_read_path(&path)?;
    let embedding_mode = resolve_embedding_mode(config.as_ref(), input.no_embeddings);
    let embedding_provider = resolve_embedding_provider_selection(config.as_ref());
    let project = config
        .as_ref()
        .and_then(|config| project_context_for_selected_path(config, &path));

    match out {
        Some(out) => build_to_dir(
            path,
            out,
            embedding_mode,
            embedding_provider,
            project,
            evaluation_date,
            policy,
        ),
        None => {
            let output_paths = resolve_build_output_paths(config.as_ref(), embedding_mode)?;
            let output_paths = resolve_build_output_paths_with_policy(&output_paths, context)?;
            build_to_paths(
                path,
                output_paths,
                embedding_mode,
                embedding_provider,
                project,
                evaluation_date,
                policy,
            )
        }
    }
}

/// The shared read-command prefix (ADR-0038: every artifact reader locates
/// the Graph Artifact the same way): resolve an explicit `--artifact` through
/// the path policy, discover config when artifact defaults or retrieval policy
/// require it, and gate the final artifact path through the path policy again.
fn resolve_graph_artifact_for_read<P>(
    context: &LocalContext<P>,
    artifact_arg: Option<&Path>,
    needs_retrieval_policy: bool,
) -> Result<(PathBuf, Option<ProjectConfig>), LocalError>
where
    P: PathPolicy,
{
    let artifact = artifact_arg
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let config = discover_project_config_if(
        artifact.is_none() || needs_retrieval_policy,
        context.config_start(),
    )?;
    let artifact = resolve_graph_artifact_path_with_config(artifact, config.as_ref());
    Ok((context.path_policy().resolve_read_path(&artifact)?, config))
}

/// Load the graph session for a read command. The session is usable only
/// when it loaded AND its diagnostics carry no errors; otherwise the caller
/// ships its command-specific empty envelope with these load diagnostics.
fn load_graph_session_for_query<P: PathPolicy>(
    context: &LocalContext<P>,
    artifact: Option<&Path>,
) -> Result<(Option<GraphSession>, Vec<Diagnostic>), LocalError> {
    let (graph_artifact, config) = match resolve_graph_artifact_for_read(context, artifact, true) {
        Ok(resolved) => resolved,
        Err(error) => return Ok((None, vec![retrieval_config_diagnostic(error)?])),
    };
    let load_result = load_graph_session(CoreGraphInput {
        policy: context
            .retrieval_policy_override()
            .cloned()
            .or_else(|| config.and_then(|config| config.retrieval_policy)),
        graph_artifact_path: graph_artifact,
    });
    let diagnostics = load_result.diagnostics;
    let session = load_result
        .session
        .filter(|_| !diagnostics_have_errors(&diagnostics));
    Ok((session, diagnostics))
}

fn why_with_context<P>(context: &LocalContext<P>, input: WhyInput) -> Result<WhyOutcome, LocalError>
where
    P: PathPolicy,
{
    let (artifact, config) =
        match resolve_graph_artifact_for_read(context, input.artifact.as_deref(), true) {
            Ok(resolved) => resolved,
            Err(error) => {
                let diagnostic = retrieval_config_diagnostic(error)?;
                return Ok(WhyOutcome {
                    read_access: Default::default(),
                    artifact: resolve_graph_artifact_path_with_config(input.artifact, None),
                    records: Vec::new(),
                    diagnostics: vec![diagnostic],
                    duration: Duration::ZERO,
                    exit_code: 2,
                });
            }
        };
    let load_result = load_retrieval_session_with_embedding_provider(
        RetrievalInput {
            artifact_path: artifact.clone(),
            search_artifact_path: None,
            policy: context
                .retrieval_policy_override()
                .cloned()
                .or_else(|| config.and_then(|config| config.retrieval_policy)),
        },
        EmbeddingProviderSelection::Local,
    );
    let RetrievalLoadResult {
        session,
        diagnostics: load_diagnostics,
    } = load_result;
    let load_exit_code = why_exit_code_for_diagnostics(&load_diagnostics);
    let Some(session) = session.filter(|_| !diagnostics_have_errors(&load_diagnostics)) else {
        return Ok(WhyOutcome {
            read_access: Default::default(),
            artifact,
            records: Vec::new(),
            diagnostics: load_diagnostics,
            duration: Duration::ZERO,
            exit_code: load_exit_code,
        });
    };

    let started = Instant::now();
    let mut why_result = why_object(&session, &input.object_id);
    let mut entries: Vec<_> = why_result
        .records
        .into_iter()
        .map(RetrievalEntry::KnowledgeObject)
        .collect();
    let read_access = match adoc_core::retrieval_read_access(&session, &mut entries) {
        Ok(access) => access,
        Err(diagnostic) => {
            entries.clear();
            why_result.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    why_result.records = entries
        .into_iter()
        .filter_map(|entry| match entry {
            RetrievalEntry::KnowledgeObject(record) => Some(record),
            _ => None,
        })
        .collect();
    let duration = started.elapsed();
    let diagnostics = merge_diagnostics(load_diagnostics, why_result.diagnostics);
    let exit_code = why_exit_code_for_diagnostics(&diagnostics);
    let records = why_result
        .records
        .into_iter()
        .map(|record| resolved_record(&session, record))
        .collect();

    Ok(WhyOutcome {
        read_access,
        artifact,
        records,
        diagnostics,
        duration,
        exit_code,
    })
}

fn graph_with_context<P>(
    context: &LocalContext<P>,
    input: GraphInput,
) -> Result<GraphOutcome, LocalError>
where
    P: PathPolicy,
{
    let (session, mut diagnostics) =
        load_graph_session_for_query(context, input.artifact.as_deref())?;
    let Some(session) = session else {
        let exit_code = graph_exit_code_for_diagnostics(&diagnostics);
        return Ok(GraphOutcome {
            read_access: Default::default(),
            envelope: GraphTraversalEnvelope::new(
                input.object_id,
                Vec::new(),
                Vec::new(),
                diagnostics,
            ),
            exit_code,
        });
    };

    let traversal = traverse_graph(
        &session,
        GraphTraversalQuery {
            root_id: input.object_id.clone(),
            direction: input.direction.unwrap_or_default(),
            relations: input.relation.into_iter().collect(),
        },
    );
    diagnostics = merge_diagnostics(diagnostics, traversal.diagnostics);
    let exit_code = graph_exit_code_for_diagnostics(&diagnostics);
    let result = GraphTraversalResult {
        root: traversal.root,
        nodes: traversal.nodes,
        edges: traversal.edges,
        diagnostics,
    };

    let mut envelope = GraphTraversalEnvelope::from(result);
    let read_access = match adoc_core::graph_read_access(&session, &envelope) {
        Ok(access) => access,
        Err(diagnostic) => {
            envelope.nodes.clear();
            envelope.edges.clear();
            envelope.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    let exit_code = exit_code.max(graph_exit_code_for_diagnostics(&envelope.diagnostics));
    Ok(GraphOutcome {
        read_access,
        envelope,
        exit_code,
    })
}

fn stale_with_context<P>(
    context: &LocalContext<P>,
    input: StaleInput,
) -> Result<StaleOutcome, LocalError>
where
    P: PathPolicy,
{
    let (session, diagnostics) = load_graph_session_for_query(context, input.artifact.as_deref())?;
    let Some(session) = session else {
        let exit_code = signal_query_exit_code(&diagnostics);
        return Ok(StaleOutcome {
            read_access: Default::default(),
            envelope: empty_stale_envelope(diagnostics),
            exit_code,
        });
    };

    let mut envelope = evaluate_stale(&session, input.within_days, diagnostics);
    let read_access = match adoc_core::stale_read_access(&session, &envelope) {
        Ok(access) => access,
        Err(diagnostic) => {
            envelope.records.clear();
            envelope.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    let exit_code = signal_query_exit_code(&envelope.diagnostics);
    Ok(StaleOutcome {
        read_access,
        envelope,
        exit_code,
    })
}

fn contradictions_with_context<P>(
    context: &LocalContext<P>,
    input: ContradictionsInput,
) -> Result<ContradictionsOutcome, LocalError>
where
    P: PathPolicy,
{
    let (session, diagnostics) = load_graph_session_for_query(context, input.artifact.as_deref())?;
    let Some(session) = session else {
        let exit_code = signal_query_exit_code(&diagnostics);
        return Ok(ContradictionsOutcome {
            read_access: Default::default(),
            envelope: empty_contradictions_envelope(diagnostics),
            exit_code,
        });
    };

    let mut envelope = evaluate_contradictions(&session, input.all, diagnostics);
    let read_access = match adoc_core::contradictions_read_access(&session, &envelope) {
        Ok(access) => access,
        Err(diagnostic) => {
            envelope.contradictions.clear();
            envelope.contradicted_claims.clear();
            envelope.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    let exit_code = signal_query_exit_code(&envelope.diagnostics);
    Ok(ContradictionsOutcome {
        read_access,
        envelope,
        exit_code,
    })
}

fn impacted_with_context<P>(
    context: &LocalContext<P>,
    input: ImpactedInput,
) -> Result<ImpactedOutcome, LocalError>
where
    P: PathPolicy,
{
    // Resolve the changed set before touching the artifact so input errors
    // short-circuit deterministically (the envelope still ships, ADR-0038).
    //
    // The git derivation deliberately skips `PathPolicy::resolve_read_path`
    // (unlike every artifact read below): git discovers the repository by
    // walking up from `config_start` to `.git` itself, and git history is
    // not a filesystem read in the policy sense. If a future policy needs
    // to gate "read git state outside the policy root", this is the seam.
    let changed = match &input.changed {
        ImpactedChangedSet::Paths(paths) => validate_changed_paths(paths),
        ImpactedChangedSet::GitRef(base_ref) => {
            changed_files_from_git(context.config_start().to_path_buf(), base_ref)
        }
    };
    let changed: Vec<RelPath> = match changed {
        Ok(changed) => changed,
        Err(diagnostics) => {
            let exit_code = impacted_exit_code(&diagnostics);
            return Ok(ImpactedOutcome {
                read_access: Default::default(),
                envelope: empty_impacted_envelope(Vec::new(), diagnostics),
                exit_code,
            });
        }
    };

    let (session, diagnostics) = load_graph_session_for_query(context, input.artifact.as_deref())?;
    let Some(session) = session else {
        let exit_code = impacted_exit_code(&diagnostics);
        return Ok(ImpactedOutcome {
            read_access: Default::default(),
            envelope: empty_impacted_envelope(changed_paths_strings(&changed), diagnostics),
            exit_code,
        });
    };

    let mut envelope = evaluate_impacted(&session, &changed, diagnostics);
    let read_access = match adoc_core::impacted_read_access(&session, &envelope) {
        Ok(access) => access,
        Err(diagnostic) => {
            envelope.impacted.clear();
            envelope.proof_obligations.clear();
            envelope.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    let exit_code = impacted_exit_code(&envelope.diagnostics);
    Ok(ImpactedOutcome {
        read_access,
        envelope,
        exit_code,
    })
}

fn search_with_context<P>(
    context: &LocalContext<P>,
    input: SearchInput,
) -> Result<SearchOutcome, LocalError>
where
    P: PathPolicy,
{
    let requested_mode = if input.semantic {
        SearchMode::Semantic
    } else if input.lexical {
        SearchMode::Lexical
    } else {
        SearchMode::Hybrid
    };

    let artifact = input
        .artifact
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let search_artifact = input
        .search_artifact
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    // An explicit artifact changes the data source, never the policy source.
    let config = match ProjectConfig::discover_from(context.config_start()) {
        Ok(config) => config,
        Err(error) => {
            return Ok(search_outcome(
                Vec::new(),
                vec![retrieval_config_diagnostic(error)?],
                2,
            ));
        }
    };
    // Policy discovery must not change the historical provider/path defaults
    // for a search whose relevant artifact paths are all explicit.
    let defaults_config = config.as_ref().filter(|_| {
        artifact.is_none() || (requested_mode != SearchMode::Lexical && search_artifact.is_none())
    });
    let embedding_provider = resolve_embedding_provider_selection(defaults_config);
    let artifact = resolve_graph_artifact_path_with_config(artifact, defaults_config);
    let artifact = context.path_policy().resolve_read_path(&artifact)?;
    let search_artifact_path = match requested_mode {
        SearchMode::Lexical => None,
        SearchMode::Hybrid | SearchMode::Semantic => {
            let path = resolve_search_artifact_path_with_config(search_artifact, defaults_config);
            Some(context.path_policy().resolve_read_path(&path)?)
        }
    };
    let load_result = load_retrieval_session_with_embedding_provider(
        RetrievalInput {
            artifact_path: artifact,
            search_artifact_path,
            policy: context.retrieval_policy_override().cloned().or_else(|| {
                config
                    .as_ref()
                    .and_then(|config| config.retrieval_policy.clone())
            }),
        },
        embedding_provider,
    );
    let RetrievalLoadResult {
        session,
        diagnostics: load_diagnostics,
    } = load_result;
    let Some(session) = session.filter(|_| !diagnostics_have_errors(&load_diagnostics)) else {
        return Ok(search_outcome(Vec::new(), load_diagnostics, 2));
    };

    if requested_mode == SearchMode::Semantic && !session.has_semantic_index() {
        let mut diagnostics = load_diagnostics;
        diagnostics.push(Diagnostic {
            code: DiagnosticCode::SearchArtifactMissing,
            severity: Severity::Error,
            message: "Semantic search requested but no usable search index is loaded.".to_string(),
            span: None,
            object_id: None,
            help: Some(
                DiagnosticCode::SearchArtifactMissing
                    .default_help()
                    .to_string(),
            ),
        });
        return Ok(search_outcome(Vec::new(), diagnostics, 2));
    }

    let mode = match requested_mode {
        SearchMode::Hybrid if session.has_semantic_index() => SearchMode::Hybrid,
        SearchMode::Hybrid => SearchMode::Lexical,
        mode => mode,
    };
    let needs_query_vector = matches!(mode, SearchMode::Hybrid | SearchMode::Semantic);
    let query_vector = if needs_query_vector {
        match embed_query_with_embedding_provider(&input.query, embedding_provider) {
            Ok(vector) => Some(vector),
            Err(embed_error) => {
                let mode_label = match mode {
                    SearchMode::Hybrid => "hybrid search",
                    SearchMode::Semantic => "semantic search",
                    SearchMode::Lexical => "lexical search",
                };
                let (code, message) = match &embed_error {
                    adoc_core::EmbedQueryError::ModelLoad(msg) => (
                        DiagnosticCode::EmbedModelLoadFailed,
                        format!("{mode_label} requested but embedding model failed to load: {msg}"),
                    ),
                    adoc_core::EmbedQueryError::Compute(msg) => (
                        DiagnosticCode::EmbedComputeFailed,
                        format!("{mode_label} requested but query embedding failed: {msg}"),
                    ),
                };
                let diagnostic = Diagnostic {
                    code,
                    severity: Severity::Error,
                    message,
                    span: None,
                    object_id: None,
                    help: Some(code.default_help().to_string()),
                };
                return Ok(search_outcome(Vec::new(), vec![diagnostic], 2));
            }
        }
    } else {
        None
    };

    let mut search_result = core_search(
        &session,
        SearchQuery {
            text: input.query,
            mode,
            filters: SearchFilters {
                kind: input.kind,
                status: input.status,
                owner: input.owner,
                source_path: input.source_path,
                related_to: input.related_to,
                relation: input.relation,
                direction: input.direction,
            },
            top: input.top,
            query_vector,
            scope: input.scope,
        },
    );
    let read_access = match adoc_core::retrieval_read_access(&session, &mut search_result.records) {
        Ok(access) => access,
        Err(diagnostic) => {
            search_result.records.clear();
            search_result.diagnostics.push(*diagnostic);
            Default::default()
        }
    };
    let diagnostics = merge_diagnostics(load_diagnostics, search_result.diagnostics);
    let exit_code = search_exit_code(&diagnostics);
    let records = search_result
        .records
        .into_iter()
        .map(|entry| match entry {
            RetrievalEntry::KnowledgeObject(record) => {
                ResolvedSearchEntry::KnowledgeObject(resolved_record(&session, record))
            }
            RetrievalEntry::Prose(record) => ResolvedSearchEntry::Prose(record),
        })
        .collect::<Vec<_>>();

    let mut outcome = search_outcome(records, diagnostics, exit_code);
    outcome.read_access = read_access;
    Ok(outcome)
}

fn diff_with_context<P>(
    context: &LocalContext<P>,
    input: DiffInput,
) -> Result<DiffOutcome, LocalError>
where
    P: PathPolicy,
{
    let (project_root, docs_path) = review_project_context(context.config_start())?;
    let review_input = CoreReviewInput {
        project_root,
        docs_path,
        base: SnapshotSelector::GitRef(GitRef::new(input.base_ref)),
        head: snapshot_selector_from_head_ref(input.head_ref),
    };
    let load =
        load_review_from_git(review_input).map_err(|source| LocalError::Review { source })?;
    let diff = diff_objects(&load.session);
    let envelope = ObjectDiffEnvelope::from_diff(diff, load.diagnostics);
    Ok(DiffOutcome {
        envelope,
        exit_code: 0,
    })
}

fn review_with_context<P>(
    context: &LocalContext<P>,
    input: ReviewInput,
) -> Result<ReviewOutcome, LocalError>
where
    P: PathPolicy,
{
    let (project_root, docs_path) = review_project_context(context.config_start())?;
    // V3.7: parse the patch source before snapshotting so a malformed patch
    // doesn't pay the cost of a worktree checkout. Path-policy resolution
    // happens here at the orchestration boundary; adoc-core never sees a
    // raw filesystem path.
    let patch_document = match input.patch {
        Some(ReviewPatchSource::Path(path)) => {
            let resolved = context.path_policy().resolve_read_path(&path)?;
            Some(
                parse_patch_from_path(&resolved).map_err(|source| LocalError::Review {
                    source: ReviewError::PatchParse { source },
                })?,
            )
        }
        Some(ReviewPatchSource::Inline(value)) => Some(parse_patch_from_value(value).map_err(
            |source| LocalError::Review {
                source: ReviewError::PatchParse { source },
            },
        )?),
        None => None,
    };

    let review_input = CoreReviewInput {
        project_root,
        docs_path,
        base: SnapshotSelector::GitRef(GitRef::new(input.base_ref)),
        head: snapshot_selector_from_head_ref(input.head_ref),
    };
    let load = load_review_with_changed_files_from_git(review_input)
        .map_err(|source| LocalError::Review { source })?;
    let envelope = review_with_patch(&load.session, load.diagnostics, patch_document.as_ref());
    Ok(ReviewOutcome {
        envelope,
        exit_code: 0,
    })
}

fn assess_changes_with_context<P>(
    context: &LocalContext<P>,
    input: AssessmentInput,
) -> Result<AssessmentOutcome, LocalError>
where
    P: PathPolicy,
{
    let envelope = assess_changes_from_git(CoreChangeAssessmentInput {
        project_root: assessment_project_root(context.config_start()),
        base_ref: input.base_ref,
        head_ref: input.head_ref,
        evaluation_date: input
            .as_of
            .unwrap_or_else(|| chrono::Utc::now().date_naive()),
    });
    let exit_code = if envelope.is_complete() { 0 } else { 2 };
    Ok(AssessmentOutcome {
        envelope,
        exit_code,
    })
}

fn repository_baseline_with_context<P>(
    context: &LocalContext<P>,
    input: RepositoryBaselineInput,
) -> Result<RepositoryBaselineOutcome, LocalError>
where
    P: PathPolicy,
{
    let project_root = assessment_project_root(context.config_start())
        .unwrap_or_else(|| context.config_start().to_path_buf());
    let envelope = repository_baseline_from_git(
        project_root,
        input.git_ref,
        input
            .as_of
            .unwrap_or_else(|| chrono::Utc::now().date_naive()),
    );
    let exit_code = if envelope.paths.status == "available" {
        0
    } else {
        2
    };
    Ok(RepositoryBaselineOutcome {
        envelope,
        exit_code,
    })
}

fn snapshot_selector_from_head_ref(head_ref: Option<String>) -> SnapshotSelector {
    match head_ref {
        Some(spec) => SnapshotSelector::GitRef(GitRef::new(spec)),
        None => SnapshotSelector::Workdir,
    }
}

fn patch_check_with_context<P>(
    context: &LocalContext<P>,
    input: PatchCheckInput,
) -> Result<PatchCheckOutcome, LocalError>
where
    P: PathPolicy,
{
    let _evaluation_date = input
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let patch_path = context.path_policy().resolve_read_path(&input.patch_path)?;
    let (graph_artifact, _) =
        resolve_graph_artifact_for_read(context, input.artifact.as_deref(), false)?;
    let result = core_check_patch(PatchInput {
        graph_artifact_path: graph_artifact,
        patch_path,
    });
    let exit_code = patch_exit_code(&result);

    Ok(PatchCheckOutcome { result, exit_code })
}

/// V6.4 — patch apply orchestration. The docs root resolves through the
/// identical chain `check_with_context` uses: `content_hash` payloads embed
/// source paths, so the apply-time recompile reproduces artifact hashes only
/// when the docs root is spelled byte-identically to the one `adoc build`
/// used. A parse failure becomes a refusal envelope (exit 1), not a process
/// error.
fn patch_apply_with_context<P>(
    context: &LocalContext<P>,
    input: PatchApplyInput,
) -> Result<PatchApplyOutcome, LocalError>
where
    P: PathPolicy,
{
    let evaluation_date = input
        .as_of
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let patch = match input.patch {
        PatchApplySource::Path(path) => {
            let resolved = context.path_policy().resolve_read_path(&path)?;
            parse_patch_from_path(&resolved)
        }
        PatchApplySource::Inline(value) => parse_patch_from_value(value),
    };
    let patch = match patch {
        Ok(patch) => patch,
        Err(error) => {
            let result = patch_apply_refusal(error.diagnostics().to_vec(), &input.interface);
            return Ok(PatchApplyOutcome {
                result,
                exit_code: 1,
            });
        }
    };

    let artifact = input
        .artifact
        .as_deref()
        .map(|path| context.path_policy().resolve_read_path(path))
        .transpose()?;
    let config = discover_project_config_if(true, context.config_start())?;
    let graph_artifact = resolve_graph_artifact_path_with_config(artifact, config.as_ref());
    let graph_artifact = context.path_policy().resolve_read_path(&graph_artifact)?;
    let docs_root = resolve_docs_path_with_config(None, config.as_ref())?;
    let docs_root = context.path_policy().resolve_read_path(&docs_root)?;
    let project_root = context
        .path_policy()
        .resolve_write_path(context.config_start())?;

    let result = core_apply_patch_for_date(
        CorePatchApplyInput {
            graph_artifact_path: graph_artifact,
            docs_root,
            project_root,
            interface: input.interface,
        },
        patch,
        evaluation_date,
    );
    let exit_code = patch_apply_exit_code(&result);

    Ok(PatchApplyOutcome { result, exit_code })
}

fn project_status_with_context<P>(
    context: &LocalContext<P>,
    input: ProjectStatusInput,
) -> Result<ProjectStatusOutcome, LocalError>
where
    P: PathPolicy,
{
    let config = ProjectConfig::discover_from(context.config_start())?;
    let paths = project_status_paths(context, config.as_ref())?;
    let refresh = run_project_status_refresh(context, input)?;
    let exit_code = refresh.exit_code.unwrap_or(0);
    let graph_inspection = inspect_graph_artifact(GraphArtifactInspectionInput {
        graph_artifact_path: paths.graph.clone(),
    });
    let search_inspection = inspect_search_artifact(SearchArtifactInspectionInput {
        graph_artifact_path: paths.graph.clone(),
        search_artifact_path: paths.search.clone(),
        embedding_provider: project_status_embedding_provider(config.as_ref()),
    });
    let artifacts = ProjectStatusArtifacts {
        graph: graph_inspection,
        search: search_inspection,
    };
    let graph_ready = artifacts.graph.load_status == ArtifactLoadStatus::Readable;
    let search_ready = artifacts.search.load_status == ArtifactLoadStatus::Readable;
    let semantic_enabled = config
        .as_ref()
        .map(|config| config.embeddings_provider != EmbeddingsProvider::None)
        .unwrap_or(true);
    let readiness = ProjectStatusReadiness {
        retrieval: graph_ready,
        semantic_search: graph_ready && search_ready && semantic_enabled,
        patch_validation: graph_ready,
        review: git_review_available(context.config_start()),
        patch_apply_enabled: config
            .as_ref()
            .map(|config| config.mcp_patch_apply_enabled)
            .unwrap_or(false),
    };

    Ok(ProjectStatusOutcome {
        schema_version: PROJECT_STATUS_SCHEMA_VERSION,
        project_root: context.config_start().to_path_buf(),
        config: ProjectStatusConfig {
            discovered: config.is_some(),
            path: config.as_ref().map(|config| config.path.clone()),
            embeddings_provider: config
                .as_ref()
                .map(|config| embedding_provider_label(config.embeddings_provider).to_string()),
        },
        paths,
        refresh,
        artifacts,
        readiness,
        exit_code,
    })
}

fn run_project_status_refresh<P>(
    context: &LocalContext<P>,
    input: ProjectStatusInput,
) -> Result<ProjectStatusRefreshReport, LocalError>
where
    P: PathPolicy,
{
    match input.refresh {
        ProjectStatusRefresh::None => Ok(ProjectStatusRefreshReport {
            requested: ProjectStatusRefresh::None,
            exit_code: None,
            diagnostics: Vec::new(),
            outputs: None,
        }),
        ProjectStatusRefresh::Check => {
            let outcome = check_with_context(
                context,
                CheckInput {
                    path: None,
                    as_of: None,
                },
            )?;
            Ok(ProjectStatusRefreshReport {
                requested: ProjectStatusRefresh::Check,
                exit_code: Some(outcome.exit_code),
                diagnostics: outcome.diagnostics,
                outputs: None,
            })
        }
        ProjectStatusRefresh::Build => {
            let outcome = build_with_context(
                context,
                BuildInput {
                    audience: None,
                    path: None,
                    out: None,
                    no_embeddings: input.no_embeddings,
                    as_of: None,
                },
            )?;
            Ok(ProjectStatusRefreshReport {
                requested: ProjectStatusRefresh::Build,
                exit_code: Some(outcome.exit_code),
                diagnostics: outcome.diagnostics,
                outputs: outcome.outputs,
            })
        }
    }
}

fn project_status_paths<P>(
    context: &LocalContext<P>,
    config: Option<&ProjectConfig>,
) -> Result<ProjectStatusPaths, LocalError>
where
    P: PathPolicy,
{
    let docs = config
        .map(|config| config.docs_path.clone())
        .unwrap_or_else(|| context.config_start().to_path_buf());
    let html = config
        .and_then(|config| config.outputs.html.clone())
        .or_else(|| Some(PathBuf::from(DEFAULT_HTML_ARTIFACT_PATH)));
    let graph = resolve_graph_artifact_path_with_config(None, config);
    let search = config
        .and_then(|config| config.outputs.search.clone())
        .or_else(|| Some(PathBuf::from(DEFAULT_SEARCH_ARTIFACT_PATH)));

    Ok(ProjectStatusPaths {
        docs: context.path_policy().resolve_read_path(&docs)?,
        html: html
            .as_deref()
            .map(|path| context.path_policy().resolve_write_path(path))
            .transpose()?,
        graph: context.path_policy().resolve_write_path(&graph)?,
        search: search
            .as_deref()
            .map(|path| context.path_policy().resolve_write_path(path))
            .transpose()?,
    })
}

fn embedding_provider_label(provider: EmbeddingsProvider) -> &'static str {
    match provider {
        EmbeddingsProvider::Local => "local",
        EmbeddingsProvider::Deterministic => "deterministic",
        EmbeddingsProvider::None => "none",
    }
}

fn resolve_embedding_provider_selection(
    config: Option<&ProjectConfig>,
) -> EmbeddingProviderSelection {
    match config.map(|config| config.embeddings_provider) {
        Some(EmbeddingsProvider::Deterministic) => EmbeddingProviderSelection::Deterministic,
        Some(EmbeddingsProvider::Local | EmbeddingsProvider::None) | None => {
            EmbeddingProviderSelection::Local
        }
    }
}

fn project_status_embedding_provider(
    config: Option<&ProjectConfig>,
) -> Option<EmbeddingProviderSelection> {
    match config.map(|config| config.embeddings_provider) {
        Some(EmbeddingsProvider::None) => None,
        Some(EmbeddingsProvider::Deterministic) => Some(EmbeddingProviderSelection::Deterministic),
        Some(EmbeddingsProvider::Local) | None => Some(EmbeddingProviderSelection::Local),
    }
}

fn write_init_files(project_root: &Path) -> Result<(), LocalError> {
    let config_path = project_root.join(CONFIG_FILE_NAME);
    let index_path = project_root.join(INIT_INDEX_PATH);

    for target in [&config_path, &index_path] {
        if target.exists() {
            return Err(LocalError::InitTargetExists {
                path: target.to_path_buf(),
            });
        }
    }

    if let Some(parent) = index_path.parent() {
        fs::create_dir_all(parent).map_err(|source| LocalError::CreateOutputDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    write_new_file(&config_path, INIT_CONFIG_TEMPLATE.as_bytes())?;
    if let Err(error) = write_new_file(&index_path, init_index_template().as_bytes()) {
        cleanup_init_paths([&config_path]);
        return Err(error);
    }

    Ok(())
}

fn init_index_template() -> &'static str {
    "\
# AgentDoc Project @doc(project.index)

This project was initialized with AgentDoc.

::claim project.initialized
status: draft
--
The project has an initialized AgentDoc source tree.
::
"
}

fn write_new_file(path: &Path, contents: &[u8]) -> Result<(), LocalError> {
    write_new_target(path, contents).map_err(|source| {
        if source.kind() == io::ErrorKind::AlreadyExists {
            LocalError::InitTargetExists {
                path: path.to_path_buf(),
            }
        } else {
            LocalError::WriteFailed {
                path: path.to_path_buf(),
                source,
            }
        }
    })
}

/// Create-new + write with self-cleanup: a file is only removed when this
/// call created it and the write then failed. A pre-existing file surfaces
/// as `AlreadyExists` and is never touched — the caller cannot tell a
/// partial write from a user's file, so the distinction must live here.
fn write_new_target(path: &Path, contents: &[u8]) -> io::Result<()> {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    file.write_all(contents).inspect_err(|_| {
        let _ = fs::remove_file(path);
    })
}

fn cleanup_init_paths<P: AsRef<Path>>(paths: impl IntoIterator<Item = P>) {
    for path in paths {
        let _ = fs::remove_file(path.as_ref());
    }
}

fn build_to_dir(
    path: PathBuf,
    out: PathBuf,
    embedding_mode: BuildEmbeddingMode,
    embedding_provider: EmbeddingProviderSelection,
    project: Option<LocalProjectContext>,
    evaluation_date: chrono::NaiveDate,
    policy: Option<adoc_core::RetrievalPolicy>,
) -> Result<BuildOutcome, LocalError> {
    let input = CoreBuildInput {
        policy,
        root: path,
        embeddings: embedding_mode,
        prior_search_artifact_path: Some(out.join("docs.search.json")),
    };
    let result = match project {
        Some(project) => build_project_workspace_with_embedding_provider_for_date(
            input,
            project,
            embedding_provider,
            evaluation_date,
        ),
        None => build_workspace_with_embedding_provider_for_date(
            input,
            embedding_provider,
            evaluation_date,
        ),
    };
    finish_build_result(result, &out)
}

fn build_to_paths(
    path: PathBuf,
    output_paths: BuildOutputs,
    embedding_mode: BuildEmbeddingMode,
    embedding_provider: EmbeddingProviderSelection,
    project: Option<LocalProjectContext>,
    evaluation_date: chrono::NaiveDate,
    policy: Option<adoc_core::RetrievalPolicy>,
) -> Result<BuildOutcome, LocalError> {
    let input = CoreBuildInput {
        policy,
        root: path,
        embeddings: embedding_mode,
        prior_search_artifact_path: output_paths.search.clone(),
    };
    let result = match project {
        Some(project) => build_project_workspace_with_embedding_provider_for_date(
            input,
            project,
            embedding_provider,
            evaluation_date,
        ),
        None => build_workspace_with_embedding_provider_for_date(
            input,
            embedding_provider,
            evaluation_date,
        ),
    };
    finish_build_result_at_paths(result, &output_paths)
}

fn finish_build_result(result: CompileResult, out: &Path) -> Result<BuildOutcome, LocalError> {
    let has_errors = result.has_errors();
    let diagnostics = result.diagnostics;

    let artifacts = match result.artifacts {
        Some(artifacts) => artifacts,
        None if has_errors => {
            return Ok(BuildOutcome {
                diagnostics,
                outputs: None,
                exit_code: 1,
            });
        }
        None => return Err(LocalError::BuildMissingArtifacts),
    };

    let paths = output_paths_for_dir(out, artifacts.search_json.is_some())?;
    write_artifacts_to_paths(&paths, &artifacts)?;
    Ok(BuildOutcome {
        diagnostics,
        outputs: Some(paths),
        exit_code: i32::from(has_errors),
    })
}

fn finish_build_result_at_paths(
    result: CompileResult,
    paths: &BuildOutputs,
) -> Result<BuildOutcome, LocalError> {
    let has_errors = result.has_errors();
    let diagnostics = result.diagnostics;

    let artifacts = match result.artifacts {
        Some(artifacts) => artifacts,
        None if has_errors => {
            return Ok(BuildOutcome {
                diagnostics,
                outputs: None,
                exit_code: 1,
            });
        }
        None => return Err(LocalError::BuildMissingArtifacts),
    };

    write_artifacts_to_paths(paths, &artifacts)?;
    Ok(BuildOutcome {
        diagnostics,
        outputs: Some(paths.clone()),
        exit_code: i32::from(has_errors),
    })
}

fn output_paths_for_dir(out: &Path, include_search: bool) -> Result<BuildOutputs, LocalError> {
    if out.exists() && !out.is_dir() {
        return Err(LocalError::OutputPathIsFile {
            path: out.to_path_buf(),
        });
    }

    fs::create_dir_all(out).map_err(|source| LocalError::CreateOutputDirectory {
        path: out.to_path_buf(),
        source,
    })?;

    Ok(BuildOutputs {
        html: out.join("docs.html"),
        graph: out.join("docs.graph.json"),
        search: include_search.then(|| out.join("docs.search.json")),
    })
}

struct ArtifactWriteEntry {
    path: PathBuf,
    contents: Vec<u8>,
}

fn write_artifacts_to_paths(
    paths: &BuildOutputs,
    artifacts: &BuildArtifacts,
) -> Result<(), LocalError> {
    for entry in serialize_artifacts(paths, artifacts) {
        write_file_with_parents(&entry.path, &entry.contents)?;
    }

    Ok(())
}

fn serialize_artifacts(
    paths: &BuildOutputs,
    artifacts: &BuildArtifacts,
) -> Vec<ArtifactWriteEntry> {
    let mut entries = vec![ArtifactWriteEntry {
        path: paths.html.clone(),
        contents: artifacts.html.as_bytes().to_vec(),
    }];

    entries.push(ArtifactWriteEntry {
        path: paths.graph.clone(),
        contents: artifacts.graph_json.as_bytes().to_vec(),
    });

    if let (Some(search_json), Some(search_path)) =
        (artifacts.search_json.as_ref(), paths.search.as_ref())
    {
        entries.push(ArtifactWriteEntry {
            path: search_path.clone(),
            contents: search_json.as_bytes().to_vec(),
        });
    }

    entries
}

fn write_file_with_parents(path: &Path, contents: &[u8]) -> Result<(), LocalError> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|source| LocalError::CreateOutputDirectory {
            path: parent.to_path_buf(),
            source,
        })?;
    }

    fs::write(path, contents).map_err(|source| LocalError::WriteFailed {
        path: path.to_path_buf(),
        source,
    })
}

fn resolve_embedding_mode(
    config: Option<&ProjectConfig>,
    no_embeddings: bool,
) -> BuildEmbeddingMode {
    if no_embeddings
        || config
            .map(|config| config.embeddings_provider == EmbeddingsProvider::None)
            .unwrap_or(false)
    {
        BuildEmbeddingMode::Skipped
    } else {
        BuildEmbeddingMode::Enabled
    }
}

fn resolve_build_output_paths(
    config: Option<&ProjectConfig>,
    embedding_mode: BuildEmbeddingMode,
) -> Result<BuildOutputs, LocalError> {
    let Some(config) = config else {
        return Err(LocalError::ConfigMissing {
            message: "adoc build requires --out or agentdoc.config.yaml outputs".to_string(),
            config_path: None,
        });
    };

    let search_required = embedding_mode == BuildEmbeddingMode::Enabled;
    let html = config.outputs.html.clone();
    let graph = config.outputs.graph.clone();
    let search = config.outputs.search.clone();

    match (html, graph, search_required, search) {
        (Some(html), Some(graph), true, Some(search)) => Ok(BuildOutputs {
            html,
            graph,
            search: Some(search),
        }),
        (Some(html), Some(graph), false, search) => Ok(BuildOutputs {
            html,
            graph,
            search,
        }),
        _ => Err(LocalError::ConfigMissing {
            message: if search_required {
                "adoc build requires outputs.dir or exact html, graph, and search outputs"
            } else {
                "adoc build requires outputs.dir or exact html and graph outputs"
            }
            .to_string(),
            config_path: Some(config.path.clone()),
        }),
    }
}

fn resolve_build_output_paths_with_policy<P>(
    paths: &BuildOutputs,
    context: &LocalContext<P>,
) -> Result<BuildOutputs, LocalError>
where
    P: PathPolicy,
{
    Ok(BuildOutputs {
        html: context.path_policy().resolve_write_path(&paths.html)?,
        graph: context.path_policy().resolve_write_path(&paths.graph)?,
        search: paths
            .search
            .as_deref()
            .map(|path| context.path_policy().resolve_write_path(path))
            .transpose()?,
    })
}

fn discover_project_config_if(
    needed: bool,
    start: &Path,
) -> Result<Option<ProjectConfig>, LocalError> {
    if needed {
        ProjectConfig::discover_from(start)
    } else {
        Ok(None)
    }
}

fn local_project_context(config: &ProjectConfig) -> LocalProjectContext {
    LocalProjectContext {
        project_root: config
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
        docs_root: config.docs_path.clone(),
    }
}

fn project_context_for_selected_path(
    config: &ProjectConfig,
    selected_path: &Path,
) -> Option<LocalProjectContext> {
    let selected = selected_path.canonicalize().ok()?;
    let docs = config.docs_path.canonicalize().ok()?;
    selected
        .starts_with(docs)
        .then(|| local_project_context(config))
}

fn review_project_context(start: &Path) -> Result<(PathBuf, PathBuf), LocalError> {
    let project_root = git_project_root(start).ok_or_else(|| LocalError::ConfigMissing {
        message: "adoc diff/review requires a Git repository".to_string(),
        config_path: None,
    })?;
    Ok((project_root, PathBuf::new()))
}

fn git_project_root(start: &Path) -> Option<PathBuf> {
    let mut current = start.canonicalize().ok()?;
    loop {
        if current.join(".git").exists() {
            return Some(current);
        }
        if !current.pop() {
            return None;
        }
    }
}

fn assessment_project_root(start: &Path) -> Option<PathBuf> {
    let repository_root = git_project_root(start)?;
    let mut current = start.canonicalize().ok()?;
    loop {
        // Stop on present or inaccessible config paths; only the selected
        // snapshot may supply config contents or report its read failure.
        if config_path_present(&current.join(CONFIG_FILE_NAME)).unwrap_or(true) {
            return Some(current);
        }
        if current == repository_root || !current.pop() {
            return Some(repository_root);
        }
    }
}

fn resolve_docs_path_with_config(
    path: Option<PathBuf>,
    config: Option<&ProjectConfig>,
) -> Result<PathBuf, LocalError> {
    path.or_else(|| config.map(|config| config.docs_path.clone()))
        .ok_or_else(|| LocalError::ConfigMissing {
            message: "adoc check/build requires a path or agentdoc.config.yaml with docs_path"
                .to_string(),
            config_path: config.map(|config| config.path.clone()),
        })
}

fn resolve_graph_artifact_path_with_config(
    path: Option<PathBuf>,
    config: Option<&ProjectConfig>,
) -> PathBuf {
    if let Some(path) = path {
        return path;
    }

    config
        .as_ref()
        .and_then(|config| config.outputs.graph.clone())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_GRAPH_ARTIFACT_PATH))
}

fn resolve_search_artifact_path_with_config(
    path: Option<PathBuf>,
    config: Option<&ProjectConfig>,
) -> PathBuf {
    if let Some(path) = path {
        return path;
    }

    config
        .as_ref()
        .and_then(|config| config.outputs.search.clone())
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SEARCH_ARTIFACT_PATH))
}

fn resolved_record(
    session: &adoc_core::RetrievalSession,
    record: RetrievalRecord,
) -> ResolvedRetrievalRecord {
    let related_statuses = session.related_statuses(&record);
    ResolvedRetrievalRecord {
        record,
        related_statuses,
    }
}

fn search_outcome(
    records: Vec<ResolvedSearchEntry>,
    diagnostics: Vec<Diagnostic>,
    exit_code: i32,
) -> SearchOutcome {
    let envelope = RetrievalEnvelope::new(
        records
            .iter()
            .map(|resolved| match resolved {
                ResolvedSearchEntry::KnowledgeObject(resolved) => {
                    RetrievalEntry::KnowledgeObject(resolved.record.clone())
                }
                ResolvedSearchEntry::Prose(record) => RetrievalEntry::Prose(record.clone()),
            })
            .collect(),
        diagnostics.clone(),
    );
    SearchOutcome {
        read_access: Default::default(),
        envelope,
        records,
        diagnostics,
        exit_code,
    }
}

fn diagnostics_have_errors(diagnostics: &[Diagnostic]) -> bool {
    diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
}

/// An unreadable or malformed config cannot establish retrieval authority.
/// Preserve typed policy errors without exposing parser or filesystem details.
fn retrieval_config_diagnostic(error: LocalError) -> Result<Diagnostic, LocalError> {
    match error {
        LocalError::RetrievalPolicy { diagnostic, .. } => return Ok(*diagnostic),
        LocalError::ConfigParse { .. } | LocalError::ConfigRead { .. } => {}
        error => return Err(error),
    }
    Ok(Diagnostic {
        code: DiagnosticCode::RetrievalPolicyInvalid,
        severity: Severity::Error,
        message: format!(
            "Config `{}` could not be read or parsed; retrieval policy cannot be established.",
            CONFIG_FILE_NAME
        ),
        span: None,
        object_id: None,
        help: Some("Run `adoc check` to diagnose the config, then make it readable and valid before retrying retrieval.".to_string()),
    })
}

fn merge_diagnostics(
    mut load_diagnostics: Vec<Diagnostic>,
    mut command_diagnostics: Vec<Diagnostic>,
) -> Vec<Diagnostic> {
    load_diagnostics.append(&mut command_diagnostics);
    load_diagnostics
}

fn exit_code_for_diagnostics(
    diagnostics: &[Diagnostic],
    mapper: impl Fn(&Diagnostic) -> Option<i32>,
) -> i32 {
    diagnostics.iter().filter_map(mapper).min().unwrap_or(0)
}

fn why_exit_code_for_diagnostics(diagnostics: &[Diagnostic]) -> i32 {
    exit_code_for_diagnostics(diagnostics, why_diagnostic_exit_code)
}

fn why_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match (diagnostic.code, diagnostic.severity) {
        (DiagnosticCode::IdInvalid, _) => Some(1),
        (DiagnosticCode::RetrievalObjectNotFound, _) => Some(3),
        (_, Severity::Error) => Some(2),
        _ => None,
    }
}

fn graph_exit_code_for_diagnostics(diagnostics: &[Diagnostic]) -> i32 {
    exit_code_for_diagnostics(diagnostics, graph_diagnostic_exit_code)
}

fn graph_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match (diagnostic.code, diagnostic.severity) {
        (DiagnosticCode::IdInvalid, _) => Some(1),
        (DiagnosticCode::GraphObjectNotFound, _) => Some(3),
        (_, Severity::Error) => Some(2),
        _ => None,
    }
}

/// Lifecycle-signal queries (`adoc stale`, `adoc contradictions`) are queries,
/// not gates: records never affect the exit code; only artifact-load errors do.
fn signal_query_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    exit_code_for_diagnostics(diagnostics, signal_query_diagnostic_exit_code)
}

fn signal_query_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match diagnostic.severity {
        Severity::Error => Some(2),
        _ => None,
    }
}

/// V6.3 exit-code split: user-input errors (invalid path argument,
/// unresolvable `--ref`) exit 1; environment errors (git unavailable,
/// artifact load failure) exit 2; findings never affect the exit code.
fn impacted_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    exit_code_for_diagnostics(diagnostics, impacted_diagnostic_exit_code)
}

fn impacted_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match (diagnostic.code, diagnostic.severity) {
        (DiagnosticCode::ImpactedInvalidPath | DiagnosticCode::ImpactedRefUnresolvable, _) => {
            Some(1)
        }
        (_, Severity::Error) => Some(2),
        _ => None,
    }
}

fn search_exit_code(diagnostics: &[Diagnostic]) -> i32 {
    exit_code_for_diagnostics(diagnostics, search_diagnostic_exit_code)
}

fn search_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match (diagnostic.code, diagnostic.severity) {
        (DiagnosticCode::SearchInvalidFilter, _) => Some(1),
        (_, Severity::Error) => Some(2),
        _ => None,
    }
}

fn patch_exit_code(result: &PatchCheckResult) -> i32 {
    if result.valid {
        0
    } else {
        exit_code_for_diagnostics(&result.diagnostics, patch_diagnostic_exit_code).max(1)
    }
}

/// V6.4 apply exit codes (ADR-0036), deliberately distinct from the check's
/// 0–4 map: `0` applied and post-check clean; `1` refused, nothing written
/// (including a stale `base_hash`); `2` applied but the post-check reports
/// new errors — agents must treat `2` as "stop and surface to a human".
fn patch_apply_exit_code(result: &PatchApplyResult) -> i32 {
    if !result.applied {
        1
    } else if result.post_check.error_count > 0 {
        2
    } else {
        0
    }
}

fn patch_diagnostic_exit_code(diagnostic: &Diagnostic) -> Option<i32> {
    match (diagnostic.code, diagnostic.severity) {
        (DiagnosticCode::PatchBaseHashMismatch, _) => Some(4),
        (DiagnosticCode::GraphObjectNotFound, _) => Some(3),
        (
            DiagnosticCode::IoArtifactMissing
            | DiagnosticCode::IoArtifactUnreadable
            | DiagnosticCode::IoArtifactMalformed
            | DiagnosticCode::SchemaUnsupportedVersion
            | DiagnosticCode::IdDuplicateInArtifact
            | DiagnosticCode::IdInvalid,
            _,
        ) => Some(2),
        (
            DiagnosticCode::PatchInvalidDocument
            | DiagnosticCode::PatchValidationFailed
            | DiagnosticCode::PatchTargetAlreadyExists
            | DiagnosticCode::PatchPlacementInvalid,
            _,
        ) => Some(1),
        (_, Severity::Error) => Some(1),
        _ => None,
    }
}

/// Local adapter for exact committed-snapshot migration preparation.
pub fn prepare_migration(
    repository: &Path,
    request: &[u8],
    runtime_version: String,
    runtime_binary_digest: String,
) -> Result<adoc_core::MigrationReceipt, adoc_core::MigrationError> {
    adoc_core::prepare_migration_from_git(
        repository,
        request,
        runtime_version,
        runtime_binary_digest,
    )
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::io;

    use super::*;

    fn migrated_file(name: &str) -> adoc_core::MigratedFile {
        adoc_core::MigratedFile {
            source_path: PathBuf::from(format!("/docs/{name}.md")),
            target_path: PathBuf::from(format!("/docs/{name}.adoc")),
            target_text: format!("# {name}\n"),
            prose_blocks: 1,
        }
    }

    #[test]
    fn failed_target_write_removes_already_created_targets_and_keeps_sources() {
        let files = [
            migrated_file("one"),
            migrated_file("two"),
            migrated_file("three"),
        ];
        let removed = RefCell::new(Vec::new());
        let mut creations = 0;

        let result = execute_migration_writes_with(
            &files,
            |_, _| {
                creations += 1;
                if creations == 2 {
                    Err(io::Error::other("disk full"))
                } else {
                    Ok(())
                }
            },
            |path| {
                removed.borrow_mut().push(path.to_path_buf());
                Ok(())
            },
        );

        match result {
            Err(LocalError::WriteFailed { path, .. }) => {
                assert_eq!(path, files[1].target_path);
            }
            other => panic!("expected WriteFailed, got {other:?}"),
        }
        assert_eq!(
            *removed.borrow(),
            vec![files[0].target_path.clone()],
            "only the already-created target is cleaned up; no source is removed"
        );
    }

    #[test]
    fn failed_source_removal_reports_already_removed_sources() {
        let files = [
            migrated_file("one"),
            migrated_file("two"),
            migrated_file("three"),
        ];

        let result = execute_migration_writes_with(
            &files,
            |_, _| Ok(()),
            |path| {
                if path == files[1].source_path {
                    Err(io::Error::other("locked"))
                } else {
                    Ok(())
                }
            },
        );

        match result {
            Err(error @ LocalError::RemoveFailed { .. }) => {
                let LocalError::RemoveFailed { path, removed, .. } = &error else {
                    unreachable!();
                };
                assert_eq!(*path, files[1].source_path);
                assert_eq!(*removed, vec![files[0].source_path.clone()]);
                let message = error.to_string();
                assert!(
                    message.contains("every .adoc target was written"),
                    "message must state on-disk state: {message}"
                );
                assert!(
                    message.contains("/docs/one.md"),
                    "message must list already-removed sources: {message}"
                );
            }
            other => panic!("expected RemoveFailed, got {other:?}"),
        }
    }

    #[test]
    fn write_new_target_refuses_existing_file_and_leaves_it_intact() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("page.adoc");
        fs::write(&path, "user content").expect("seed existing target");

        let error = write_new_target(&path, b"migrated").expect_err("create_new must refuse");

        assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(
            fs::read_to_string(&path).expect("pre-existing target must survive"),
            "user content"
        );
    }
}
