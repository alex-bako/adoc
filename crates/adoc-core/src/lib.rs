mod application;
mod domain;
mod infrastructure;

pub use application::portable_projection::{
    MAX_PORTABLE_PROJECTION_BYTES, run_portable_projection,
};

use std::path::PathBuf;

pub use domain::gateway_sensitive_access::{
    AccessedKnowledgeObject, GATEWAY_SENSITIVE_ACCESS_SCHEMA_VERSION,
    GatewaySensitiveAccessCommand, GatewaySensitiveAccessError, GatewaySensitiveAccessEvent,
    GatewaySensitiveAccessInput, GatewaySensitiveAccessObject, MAX_GATEWAY_SENSITIVE_ACCESS_BYTES,
    ReadAccess, build_gateway_sensitive_access, gateway_policy_digest,
    validate_gateway_sensitive_access,
};

pub const CONNECTOR_CAPABILITIES_SCHEMA_VERSION: &str = "agentdoc.connector_capabilities.v0";

pub use application::apply::{
    ApplyProposer, ApplyTrace, ObjectHashes, PATCH_APPLY_SCHEMA_VERSION, PatchApplyResult,
    PostCheckReport, WrittenFile, mcp_patch_apply_disabled_refusal,
};
pub use application::artifact_inspection::{
    ArtifactInspection, ArtifactLoadStatus, GraphArtifactInspectionInput,
    SearchArtifactInspectionInput,
};
pub use application::change_assessment::{
    AssessedPath, AssessmentCompleteness, AssessmentConfig, AssessmentDiagnostic, AssessmentMatch,
    AssessmentObject, AssessmentObjectReason, AssessmentObligation, AssessmentOutcome,
    AssessmentPolicy, AssessmentReviewer, AssessmentSignal, AssessmentSnapshot,
    AssessmentSnapshots, AssessmentSource, AssessmentSummary, AssessmentValidation,
    AuthorityPromotion, Availability, CHANGE_ASSESSMENT_SCHEMA_VERSION, ChangeAssessmentEnvelope,
    ChangeAssessmentInput, KnowledgeChange, KnowledgeChanges, KnowledgeSnapshot,
    PathClassification, PolicyChanges, REPOSITORY_BASELINE_SCHEMA_VERSION,
    RepositoryBaselineEnvelope, RepositoryBaselineReadiness, SnapshotConfig,
};
pub use application::compile::{
    BuildArtifacts, BuildEmbeddingMode, BuildInput, CompileInput, CompileResult,
    LocalProjectContext,
};
pub use application::graph::{
    GRAPH_TRAVERSAL_SCHEMA_VERSION, GraphInput, GraphLoadResult, GraphSession,
    GraphTraversalEnvelope, traverse_graph,
};
pub use application::managed_retrieval::{
    MANAGED_RETRIEVAL_INPUT_SCHEMA_VERSION, ManagedAccessedObject, ManagedFieldProjection,
    ManagedRetrievalBinding, ManagedRetrievalCanonicalIdentity, ManagedRetrievalOutcome,
    ManagedRetrievalQuery, run_managed_retrieval,
};
pub use application::migrate::{
    MIGRATE_REPORT_SCHEMA_VERSION, MigrateCounts, MigrateDirection, MigrateMode,
    MigrateReportEnvelope, MigrateReportFile, MigrateResult, MigratedFile,
};
pub use application::patch::{
    PATCH_CHECK_SCHEMA_VERSION, PatchCheckResult, PatchInput, PatchJsonInput, PatchParseError,
};
pub use application::proposal::{
    build_proposal_record, build_proposal_record_with_dispositions, validate_proposal_record,
};
pub use application::read_access::{
    contradictions_read_access, graph_read_access, impacted_read_access, retrieval_read_access,
    stale_read_access,
};
pub use application::retrieval::{
    RETRIEVAL_SCHEMA_VERSION, RetrievalEnvelope, RetrievalInput, RetrievalLoadResult,
    RetrievalSession, SearchFilters, SearchQuery, SearchRecordScope, SearchResult, WhyResult,
    search, why_object,
};
pub use application::review::{
    DIFF_SCHEMA_VERSION, REVIEW_SCHEMA_VERSION, ReviewConfigError, ReviewError, ReviewInput,
    ReviewLoadResult, ReviewSession, diff_objects, proof_obligations, review_with_patch,
};
pub use application::review_envelope::{ObjectDiffEnvelope, ReviewEnvelope};
pub use application::signals::{
    CONTRADICTIONS_SCHEMA_VERSION, ContradictedClaimRecord, ContradictionRecord,
    ContradictionsEnvelope, IMPACTED_SCHEMA_VERSION, ImpactReason, ImpactedEnvelope,
    ImpactedRecord, STALE_SCHEMA_VERSION, StaleCategory, StaleEnvelope, StaleRecord,
};
pub use application::validation_runtime::{
    ValidationReceipt, ValidationResult, ValidationRuntimeError, ValidationRuntimeInput,
    ValidationRuntimeOutcome, run_validation_runtime,
};
pub use domain::diagnostic::{Diagnostic, DiagnosticCode, Severity};
pub use domain::executor_qualification::{
    EXECUTOR_QUALIFICATION_SCHEMA_VERSION, ExecutorAuthority, ExecutorConfiguration,
    ExecutorEligibility, ExecutorQualification, ExecutorQualificationError,
    ExecutorQualificationExpectedBindings, ModelConfiguration, QualificationLayer,
    RequalificationTrigger, validate_executor_qualification,
};
pub use domain::external_work::{
    CapabilityRequirement, ContractRequirement, ExternalWorkError, WORK_REQUEST_SCHEMA_VERSION,
    WORK_RESULT_SCHEMA_VERSION, WorkChangeRequest, WorkRequest, WorkRequestInput, WorkResult,
    WorkResultInput, WorkRuntime, WorkSource, WorkloadAuthorization, build_work_request,
    build_work_result, validate_work_request, validate_work_result,
};
pub use domain::gate_result::{
    GATE_RESULT_SCHEMA_VERSION, GateMode, GateOutcome, GateReason, GateResult, GateResultError,
    validate_gate_result,
};
pub use domain::graph::{
    GraphDirection, GraphRelationKind, GraphTraversalEdge, GraphTraversalNode, GraphTraversalQuery,
    GraphTraversalResult, ProseBlockKind,
};
pub use domain::knowledge_object::block_kind_names;
pub use domain::obligation::ProofObligation;
pub use domain::patch::{AffectedRelation, PatchDiff, PatchDocument, PatchOperation};
pub use domain::ports::snapshot_workspace::{GitRef, SnapshotError, SnapshotSelector};
pub use domain::project_config::{
    EmbeddingsProvider, ParsedConfigOutputs, ParsedProjectConfig, ProjectConfigDocumentError,
    parse_project_config,
};
pub use domain::proposal::{
    ContentBinding, PROPOSAL_SCHEMA_VERSION, ProposalBindings, ProposalChangeRequest,
    ProposalDispositionInput, ProposalDispositionKind, ProposalPatch, ProposalPatchInput,
    ProposalRecord, ProposalRecordError,
};
pub use domain::retrieval::RetrievalPolicy;
pub use domain::retrieval::{
    ProseRecord, RetrievalEntry, RetrievalMatch, RetrievalRecord, RetrievalRelations,
    RetrievalSource, SearchMode,
};
pub use domain::review::field_change::{FieldChange, RelationKind};
pub use domain::review::impact::{ImpactReasonKind, ImpactedObject, compute_impact};
pub use domain::review::object_change::ChangedObject;
pub use domain::review::object_diff::ObjectDiff;
pub use domain::review::reviewer::{RequiredReviewer, required_reviewers};
pub use domain::semantic_assessment::{
    AffectedObject, CandidateUpdate, HumanReview, HumanReviewAuthority,
    HumanReviewExpectedBindings, HumanReviewIndependence, MATERIALITY_POLICY_VERSION,
    SEMANTIC_ASSESSMENT_SCHEMA_VERSION, SemanticAssessment, SemanticAssessmentError,
    SemanticAssessmentScope, SemanticClassification, SemanticDisposition, SemanticExecutorIdentity,
    SemanticFinding, SemanticMateriality, validate_human_semantic_assessment,
    validate_semantic_assessment,
};
pub use domain::semantic_context::{
    CapabilityPolicy, CapabilityPolicyRule, CitationContentProjection, CitationHandle,
    ContextClass, ContextRequirement, ContextUnavailability, ContextUnavailabilityKind,
    DiffHunkCitation, ExactRevision, GraphCitationObject, GraphObjectContextExpectation,
    KnowledgeBasis, SEMANTIC_CONTEXT_INPUT_SCHEMA_VERSION, SEMANTIC_CONTEXT_SCHEMA_VERSION,
    SemanticContext, SemanticContextBasis, SemanticContextError, SemanticContextExpectedBindings,
    SemanticContextInput, SemanticContextItem, SemanticContextOutcome, SemanticContextSelection,
    SemanticContextValidationBasis, SourceAssertionCitation, UnavailabilityOutcome,
    UnavailabilityReason, build_semantic_context, build_semantic_context_from_document,
    is_semantic_context_text, is_sha256_digest, is_valid_capability_policy,
    semantic_context_content_digest, validate_semantic_context,
    validate_semantic_context_integrity,
};
pub use domain::semantic_executor::{
    SEMANTIC_EXECUTOR_RECEIPT_SCHEMA_VERSION, SEMANTIC_EXECUTOR_REQUEST_SCHEMA_VERSION,
    SemanticAdapterDescriptor, SemanticAdapterKind, SemanticEndpointClass, SemanticExecutorError,
    SemanticExecutorOutcome, SemanticExecutorReceipt, SemanticExecutorRequest,
    SemanticPromptContract, complete_semantic_execution, fail_semantic_execution,
    semantic_prompt_digest, validate_semantic_executor_request,
};
pub use domain::services::suggest_typed_blocks::SuggestedTypedBlock;
pub use domain::source_provenance::{
    SOURCE_ASSERTION_SCHEMA_VERSION, SOURCE_BINDING_SCHEMA_VERSION, SourceAssertion,
    SourceAssertionInput, SourceBinding, SourceBindingCoordinates, SourceBindingInput,
    SourceProvenanceError, build_source_assertion, build_source_binding, validate_source_assertion,
    validate_source_binding,
};
pub use domain::source_record::{
    RetentionClass, SOURCE_RECORD_SCHEMA_VERSION, SOURCE_RECORD_SCHEMA_VERSION_V0,
    SourceAclResource, SourceAclResourceKind, SourceAclScope, SourceArtifact, SourceRecord,
    SourceRecordError, SourceRecordInput, build_source_record, validate_source_record,
};
pub use domain::value_objects::rel_path::{RelPath, RelPathError};
pub use infrastructure::git::error::GitError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProviderSelection {
    Local,
    Deterministic,
}

impl EmbeddingProviderSelection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Deterministic => "deterministic",
        }
    }
}

/// Error returned by [`embed_query`].
#[derive(Debug)]
pub enum EmbedQueryError {
    /// The embedding model could not be loaded.
    ModelLoad(String),
    /// The query vector could not be computed.
    Compute(String),
}

impl std::fmt::Display for EmbedQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmbedQueryError::ModelLoad(msg) => write!(f, "embedding provider unavailable: {msg}"),
            EmbedQueryError::Compute(msg) => write!(f, "query embedding failed: {msg}"),
        }
    }
}

impl std::error::Error for EmbedQueryError {}

/// Embed a query string using the default embedding provider.
///
/// Returns the query vector as a `Vec<f32>`.
pub fn embed_query(query: &str) -> Result<Vec<f32>, EmbedQueryError> {
    embed_query_with_embedding_provider(query, EmbeddingProviderSelection::Local)
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn embed_query_with_embedding_provider(
    query: &str,
    provider: EmbeddingProviderSelection,
) -> Result<Vec<f32>, EmbedQueryError> {
    let provider = embedding_provider(provider).map_err(map_provider_error)?;
    provider.embed_query(query).map_err(map_provider_error)
}

pub(crate) fn map_provider_error(
    err: domain::ports::embedding_provider::EmbeddingError,
) -> EmbedQueryError {
    use domain::ports::embedding_provider::EmbeddingError;
    match err {
        EmbeddingError::ModelLoad(msg) => EmbedQueryError::ModelLoad(msg),
        EmbeddingError::Compute(msg) => EmbedQueryError::Compute(msg),
        EmbeddingError::DimensionMismatch { expected, actual } => EmbedQueryError::Compute(
            format!("query vector dim {actual} does not match provider dim {expected}"),
        ),
    }
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn compile_workspace(input: CompileInput) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(input.root);
    application::compile::compile_with_provider(&provider)
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn compile_workspace_for_date(
    input: CompileInput,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(input.root);
    application::compile::compile_with_provider_for_date(&provider, evaluation_date)
}

pub fn compile_project_workspace(
    input: CompileInput,
    project: LocalProjectContext,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::for_project(
        input.root,
        project.project_root,
        project.docs_root,
    );
    application::compile::compile_with_provider(&provider)
}

pub fn compile_project_workspace_with_anchor_root(
    input: CompileInput,
    project: LocalProjectContext,
    anchor_root: std::path::PathBuf,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::for_project(
        input.root,
        project.project_root,
        project.docs_root,
    );
    let reader = infrastructure::source::evidence_fs::FsEvidenceFileReader::new(anchor_root);
    application::compile::compile_with_provider_anchored(&provider, &reader)
}

pub fn compile_project_workspace_with_anchor_root_for_date(
    input: CompileInput,
    project: LocalProjectContext,
    anchor_root: std::path::PathBuf,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::for_project(
        input.root,
        project.project_root,
        project.docs_root,
    );
    let reader = infrastructure::source::evidence_fs::FsEvidenceFileReader::new(anchor_root);
    application::compile::compile_with_provider_anchored_for_date(
        &provider,
        &reader,
        evaluation_date,
    )
}

/// V8.5.1 (ADR-0048): [`compile_workspace`] plus Evidence Anchor
/// verification — anchored `source` paths are re-hashed against
/// `anchor_root`. `None` behaves exactly like [`compile_workspace`]; only
/// the check entry passes `Some`.
#[tracing::instrument(level = "debug", skip_all)]
pub fn compile_workspace_with_anchor_root(
    input: CompileInput,
    anchor_root: Option<std::path::PathBuf>,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(input.root);
    match anchor_root {
        Some(root) => {
            let reader = infrastructure::source::evidence_fs::FsEvidenceFileReader::new(root);
            application::compile::compile_with_provider_anchored(&provider, &reader)
        }
        None => application::compile::compile_with_provider(&provider),
    }
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn compile_workspace_with_anchor_root_for_date(
    input: CompileInput,
    anchor_root: Option<std::path::PathBuf>,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(input.root);
    match anchor_root {
        Some(root) => {
            let reader = infrastructure::source::evidence_fs::FsEvidenceFileReader::new(root);
            application::compile::compile_with_provider_anchored_for_date(
                &provider,
                &reader,
                evaluation_date,
            )
        }
        None => application::compile::compile_with_provider_for_date(&provider, evaluation_date),
    }
}

/// V8.1.1: convert every Compatibility Mode (`.md`) source under `root` to
/// canonical prose-mode `.adoc` text (ADR-0043). Performs no writes — the
/// result carries the rendered text per file; the local adapter executes
/// `--write` all-or-nothing.
#[tracing::instrument(level = "debug", skip_all)]
pub fn migrate_workspace(root: std::path::PathBuf, mode: MigrateMode) -> MigrateResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(root);
    application::migrate::migrate_with_provider(&provider, mode)
}

/// V8.1.4: convert every strict prose-mode `.adoc` source under `root` back
/// to Markdown (`adoc migrate --export`, ADR-0043 §5). Performs no writes;
/// a page containing typed blocks refuses the run with
/// `migrate.export_typed_blocks_present`.
#[tracing::instrument(level = "debug", skip_all)]
pub fn export_workspace(root: std::path::PathBuf, mode: MigrateMode) -> MigrateResult {
    let provider = infrastructure::source::fs::FsSourceProvider::new(root);
    application::migrate::export_with_provider(&provider, mode)
}

pub fn build_workspace(input: BuildInput) -> CompileResult {
    build_workspace_with_embedding_provider(input, EmbeddingProviderSelection::Local)
}

pub fn build_project_workspace(input: BuildInput, project: LocalProjectContext) -> CompileResult {
    build_project_workspace_with_embedding_provider(
        input,
        project,
        EmbeddingProviderSelection::Local,
    )
}

pub fn build_project_workspace_with_embedding_provider(
    input: BuildInput,
    project: LocalProjectContext,
    provider: EmbeddingProviderSelection,
) -> CompileResult {
    build_workspace_with_embedding_provider_factory_and_context(input, Some(project), || {
        embedding_provider(provider)
    })
}

pub fn build_project_workspace_with_embedding_provider_for_date(
    input: BuildInput,
    project: LocalProjectContext,
    provider: EmbeddingProviderSelection,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult {
    build_workspace_with_embedding_provider_factory_and_context_for_date(
        input,
        Some(project),
        || embedding_provider(provider),
        evaluation_date,
    )
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn build_workspace_with_embedding_provider(
    input: BuildInput,
    provider: EmbeddingProviderSelection,
) -> CompileResult {
    build_workspace_with_embedding_provider_factory(input, || embedding_provider(provider))
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn build_workspace_with_embedding_provider_for_date(
    input: BuildInput,
    provider: EmbeddingProviderSelection,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult {
    build_workspace_with_embedding_provider_factory_and_context_for_date(
        input,
        None,
        || embedding_provider(provider),
        evaluation_date,
    )
}

#[tracing::instrument(level = "debug", skip_all, fields(artifact = %input.graph_artifact_path.display()))]
pub fn load_graph_session(input: GraphInput) -> GraphLoadResult {
    application::graph::load_graph_session_with_readers(
        input,
        &infrastructure::artifact::GraphJsonArtifact,
    )
}

/// Evaluate the V6.1 `adoc stale` query against today's date, re-deriving
/// staleness and review-overdue-ness from authored fields at read time.
#[tracing::instrument(level = "debug", skip_all)]
pub fn evaluate_stale(
    session: &GraphSession,
    within_days: Option<u32>,
    diagnostics: Vec<Diagnostic>,
) -> StaleEnvelope {
    application::signals::evaluate_stale_today(session, within_days, diagnostics)
}

/// Evaluate the V6.2 `adoc contradictions` query: unresolved contradictions
/// (all statuses with `include_all`) plus contradicted claims with their
/// implicating contradiction ids, re-derived at read time with no clock
/// dependence — a pure function of the artifact.
#[tracing::instrument(level = "debug", skip_all)]
pub fn evaluate_contradictions(
    session: &GraphSession,
    include_all: bool,
    diagnostics: Vec<Diagnostic>,
) -> ContradictionsEnvelope {
    application::signals::evaluate_contradictions(session, include_all, diagnostics)
}

/// Empty `adoc.contradictions.v0` envelope for artifact-load-failure paths.
pub fn empty_contradictions_envelope(diagnostics: Vec<Diagnostic>) -> ContradictionsEnvelope {
    application::signals::empty_contradictions_envelope(diagnostics)
}

/// Empty `adoc.stale.v0` envelope for artifact-load-failure paths;
/// `evaluated_at` is still populated.
pub fn empty_stale_envelope(diagnostics: Vec<Diagnostic>) -> StaleEnvelope {
    application::signals::empty_stale_envelope_today(diagnostics)
}

/// Evaluate the V6.3 `adoc impacted-by` query: verified claims and accepted
/// decisions implicated by the changed-path set, each carrying its
/// impact-review proof obligation. No clock dependence — a pure function of
/// the artifact and the changed set.
#[tracing::instrument(level = "debug", skip_all)]
pub fn evaluate_impacted(
    session: &GraphSession,
    changed_files: &[RelPath],
    diagnostics: Vec<Diagnostic>,
) -> ImpactedEnvelope {
    application::signals::evaluate_impacted(session, changed_files, diagnostics)
}

/// Empty `adoc.impacted.v0` envelope for failure paths (invalid input,
/// artifact load failure). `changed_paths` echoes whatever was resolved
/// before the failure.
pub fn empty_impacted_envelope(
    changed_paths: Vec<String>,
    diagnostics: Vec<Diagnostic>,
) -> ImpactedEnvelope {
    application::signals::empty_impacted_envelope(changed_paths, diagnostics)
}

/// Sorted, deduplicated wire strings for an `adoc.impacted.v0` envelope's
/// `changed_paths` — shared by the success and failure paths.
pub fn changed_paths_strings(changed_files: &[RelPath]) -> Vec<String> {
    application::signals::changed_paths_strings(changed_files)
}

/// V6.3 — changed files for `adoc impacted-by --ref <base>`: base = git ref,
/// head = working tree (the `adoc review <ref>` selector shape). No compile,
/// no snapshot worktree. Failure is returned as ready-to-embed envelope
/// diagnostics (`impacted.ref_unresolvable` or `impacted.git_unavailable`),
/// symmetric with [`validate_changed_paths`].
#[tracing::instrument(level = "debug", skip_all)]
pub fn changed_files_from_git(
    repo_root: std::path::PathBuf,
    base_ref: &str,
) -> Result<Vec<RelPath>, Vec<Diagnostic>> {
    use domain::ports::changed_files::ChangedFilesProvider;
    let provider = infrastructure::git::changed_files::GitChangedFilesProvider::new(repo_root);
    provider
        .changed_files(
            &SnapshotSelector::GitRef(GitRef::new(base_ref)),
            &SnapshotSelector::Workdir,
        )
        .map_err(|error| {
            vec![application::signals::changed_files_failure_diagnostic(
                &error, base_ref,
            )]
        })
}

/// V6.3 — validate explicit positional changed paths. Every invalid value
/// yields one `impacted.invalid_path` diagnostic; all are collected, not
/// first-error.
pub fn validate_changed_paths(paths: &[String]) -> Result<Vec<RelPath>, Vec<Diagnostic>> {
    application::signals::validate_changed_paths(paths)
}

pub fn check_patch(input: PatchInput) -> PatchCheckResult {
    application::patch::check_patch_with_readers(
        input,
        &infrastructure::artifact::GraphJsonArtifact,
        &infrastructure::artifact::PatchJsonArtifact,
    )
}

/// Public entry point for V3.1 review loading. Constructs the
/// `GitWorktreeProvider` adapter against `input.project_root` and delegates
/// to the application layer. Mirrors the existing `compile_workspace` /
/// `check_patch` three-line wrapper pattern.
pub fn load_review_from_git(input: ReviewInput) -> Result<ReviewLoadResult, ReviewError> {
    let resolved = infrastructure::git::revision::resolve_review(
        &input.project_root,
        &input.base,
        &input.head,
    )
    .map_err(|source| ReviewError::Comparison { source })?;
    let provider =
        infrastructure::git::worktree::GitWorktreeProvider::new(input.project_root.clone())
            .with_expected_workdir_head(resolved.head_sha);
    application::review::load_project_review_with_providers(
        ReviewInput {
            base: resolved.base,
            head: resolved.head,
            ..input
        },
        &provider,
    )
}

/// Public entry point for V3.3 review loading. Constructs both the
/// `GitWorktreeProvider` and the `GitChangedFilesProvider` against
/// `input.project_root` and delegates to the application layer. Produces a
/// session populated with the V3.3 impact and required-reviewer projections.
pub fn load_review_with_changed_files_from_git(
    input: ReviewInput,
) -> Result<ReviewLoadResult, ReviewError> {
    let resolved = infrastructure::git::revision::resolve_review(
        &input.project_root,
        &input.base,
        &input.head,
    )
    .map_err(|source| ReviewError::Comparison { source })?;
    let expected_workdir_head = resolved.head_sha.clone();
    let snapshot =
        infrastructure::git::worktree::GitWorktreeProvider::new(input.project_root.clone())
            .with_expected_workdir_head(resolved.head_sha);
    let changed_files = infrastructure::git::changed_files::GitChangedFilesProvider::new(
        input.project_root.clone(),
    )
    .with_expected_workdir_head(expected_workdir_head);
    application::review::load_project_review_with_changed_files(
        ReviewInput {
            base: resolved.base,
            head: resolved.head,
            ..input
        },
        &snapshot,
        &changed_files,
    )
}

/// Produce the canonical V9.2.1 assessment for one Git comparison.
pub fn assess_changes_from_git(input: ChangeAssessmentInput) -> ChangeAssessmentEnvelope {
    assess_changes_from_git_with_worktree_status(
        input,
        infrastructure::git::revision::worktree_is_dirty,
    )
}

/// Inventory every tracked path at one immutable Git ref against AgentDoc knowledge.
pub fn repository_baseline_from_git(
    project_root: PathBuf,
    git_ref: String,
    evaluation_date: chrono::NaiveDate,
) -> RepositoryBaselineEnvelope {
    let input = ChangeAssessmentInput {
        project_root: Some(project_root.clone()),
        base_ref: git_ref.clone(),
        head_ref: Some(git_ref),
        evaluation_date,
    };
    let requested = SnapshotSelector::GitRef(GitRef::new(input.base_ref.clone()));
    let resolved = match infrastructure::git::revision::resolve_review(
        &project_root,
        &requested,
        &requested,
    ) {
        Ok(resolved) => resolved,
        Err(error) => {
            return application::change_assessment::repository_baseline(
                application::change_assessment::unresolved_envelope(&input, error),
            );
        }
    };
    let resolved_input = application::change_assessment::ResolvedAssessmentInput {
        requested_base_ref: input.base_ref,
        requested_base_commit: resolved.requested_base_sha,
        comparison_base_commit: resolved.head_sha.clone(),
        head_ref: input.head_ref,
        head_commit: resolved.head_sha.clone(),
        base: resolved.base,
        head: resolved.head,
        worktree_dirty: None,
        evaluation_date,
    };
    let snapshots = infrastructure::git::worktree::GitWorktreeProvider::new(project_root.clone());
    let tracked = infrastructure::git::changed_files::GitTrackedFilesProvider::new(project_root);
    application::change_assessment::repository_baseline(
        application::change_assessment::assess_with_providers(resolved_input, &snapshots, &tracked),
    )
}

fn assess_changes_from_git_with_worktree_status(
    input: ChangeAssessmentInput,
    worktree_status: impl FnOnce(
        &std::path::Path,
    ) -> Result<bool, domain::ports::snapshot_workspace::SnapshotError>,
) -> ChangeAssessmentEnvelope {
    let Some(project_root) = input.project_root.as_ref() else {
        return application::change_assessment::snapshot_failed_envelope(
            &input,
            "adoc assess-changes requires a Git repository".to_string(),
        );
    };
    let requested_base = SnapshotSelector::GitRef(GitRef::new(input.base_ref.clone()));
    let requested_head = input
        .head_ref
        .as_ref()
        .map(|head| SnapshotSelector::GitRef(GitRef::new(head.clone())))
        .unwrap_or(SnapshotSelector::Workdir);
    let resolved = match infrastructure::git::revision::resolve_review(
        project_root,
        &requested_base,
        &requested_head,
    ) {
        Ok(resolved) => resolved,
        Err(error) => {
            return application::change_assessment::unresolved_envelope(&input, error);
        }
    };
    let comparison_base_commit = match &resolved.base {
        SnapshotSelector::GitRef(value) => value.as_str().to_string(),
        SnapshotSelector::Workdir => {
            return application::change_assessment::unresolved_envelope(
                &input,
                domain::ports::snapshot_workspace::SnapshotError::ComparisonBaseUnavailable {
                    reason: "comparison base resolved to a mutable worktree".to_string(),
                },
            );
        }
    };
    let mut resolved_input = application::change_assessment::ResolvedAssessmentInput {
        requested_base_ref: input.base_ref,
        requested_base_commit: resolved.requested_base_sha,
        comparison_base_commit,
        head_ref: input.head_ref,
        head_commit: resolved.head_sha.clone(),
        base: resolved.base,
        head: resolved.head,
        worktree_dirty: None,
        evaluation_date: input.evaluation_date,
    };
    if resolved_input.head_ref.is_none() {
        match worktree_status(project_root) {
            Ok(dirty) => resolved_input.worktree_dirty = Some(dirty),
            Err(error) => {
                return application::change_assessment::resolved_snapshot_failed_envelope(
                    &resolved_input,
                    format!("could not determine mutable worktree status: {error}"),
                );
            }
        }
    }
    let snapshot = infrastructure::git::worktree::GitWorktreeProvider::new(project_root.clone())
        .with_expected_workdir_head(resolved_input.head_commit.clone());
    let changed_files =
        infrastructure::git::changed_files::GitChangedFilesProvider::new(project_root.clone())
            .with_expected_workdir_head(resolved_input.head_commit.clone());
    application::change_assessment::assess_with_providers(resolved_input, &snapshot, &changed_files)
}

/// V3.6 readiness probe for the review pipeline. Returns `true` when the local
/// `git` binary is available and `repo_root` has a resolvable `HEAD` ref —
/// i.e. when `load_review_from_git` has at least a usable default base ref to
/// compare against. Backs the `readiness.review` field on
/// `adoc.project.status.v0`. Never panics; any failure becomes `false`.
pub fn git_review_available(repo_root: &std::path::Path) -> bool {
    infrastructure::git::is_review_available(repo_root)
}

/// V3.7 — parse an `adoc.patch.v0` JSON document from the file at `path`.
/// Mirrors V2's `check_patch` file-reading discipline but exposes the parsed
/// [`PatchDocument`] for callers that want to compose patch validation with
/// other application surfaces (e.g. `adoc review --patch`). Patch validation
/// against a graph is still done through [`check_patch`] / [`check_patch_json`]
/// or, for review composition, [`review_with_patch`].
pub fn parse_patch_from_path(path: &std::path::Path) -> Result<PatchDocument, PatchParseError> {
    infrastructure::artifact::read_patch_document(path).map_err(|diagnostics| {
        PatchParseError::Read {
            path: path.to_path_buf(),
            diagnostics,
        }
    })
}

/// V3.7 — parse an `adoc.patch.v0` JSON document from an in-memory
/// `serde_json::Value`. Used by the MCP inline-patch path on
/// `adoc_review { patch: { source: "inline", patch: {...} } }`.
pub fn parse_patch_from_value(value: serde_json::Value) -> Result<PatchDocument, PatchParseError> {
    infrastructure::artifact::read_patch_document_value(value, "Inline patch document")
        .map_err(|diagnostics| PatchParseError::Inline { diagnostics })
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn check_patch_json(input: PatchJsonInput) -> PatchCheckResult {
    use domain::ports::artifact_reader::ArtifactReader;

    let graph_document =
        match infrastructure::artifact::GraphJsonArtifact.read(&input.graph_artifact_path) {
            Ok(document) => document,
            Err(diagnostics) => {
                return application::patch::PatchCheckResult::failure(
                    application::patch::artifact_failure_diagnostics(diagnostics),
                );
            }
        };
    let patch_document = match infrastructure::artifact::read_patch_document_value(
        input.patch,
        "Inline patch document",
    ) {
        Ok(document) => document,
        Err(diagnostics) => return application::patch::PatchCheckResult::failure(diagnostics),
    };

    application::patch::check_patch_documents(graph_document, patch_document)
}

/// V6.4 — input for [`apply_patch`]. `docs_root` must be resolved through the
/// same chain `adoc check`/`adoc build` use: `content_hash` payloads embed
/// source paths, so the apply-time recompile reproduces artifact hashes only
/// when the docs root is spelled byte-identically. `project_root` bounds the
/// write sandbox.
#[derive(Debug, Clone)]
pub struct PatchApplyInput {
    pub graph_artifact_path: std::path::PathBuf,
    pub docs_root: std::path::PathBuf,
    pub project_root: std::path::PathBuf,
    /// Recorded in the envelope's `trace.interface` (`"cli"` or `"mcp"`).
    pub interface: String,
}

/// V6.4 — apply a parsed patch document to AgentDoc source (ADR-0036).
/// Refusals come back as the same `adoc.patch.apply.v0` envelope with
/// `applied: false`; this function never panics on user input.
#[tracing::instrument(level = "debug", skip_all)]
pub fn apply_patch(input: PatchApplyInput, patch: PatchDocument) -> PatchApplyResult {
    let provider = infrastructure::source::fs::FsSourceProvider::for_project(
        input.docs_root.clone(),
        input.project_root.clone(),
        input.docs_root,
    );
    let writer = infrastructure::source::fs_writer::FsWorkspaceWriter::new(input.project_root);
    application::apply::apply_patch_with_ports(
        &input.graph_artifact_path,
        patch,
        &infrastructure::artifact::GraphJsonArtifact,
        &provider,
        &writer,
        &input.interface,
    )
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn apply_patch_for_date(
    input: PatchApplyInput,
    patch: PatchDocument,
    evaluation_date: chrono::NaiveDate,
) -> PatchApplyResult {
    let provider = infrastructure::source::fs::FsSourceProvider::for_project(
        input.docs_root.clone(),
        input.project_root.clone(),
        input.docs_root,
    );
    let writer = infrastructure::source::fs_writer::FsWorkspaceWriter::new(input.project_root);
    application::apply::apply_patch_with_ports_for_date(
        &input.graph_artifact_path,
        patch,
        &infrastructure::artifact::GraphJsonArtifact,
        &provider,
        &writer,
        &input.interface,
        evaluation_date,
    )
}

/// V6.4 — build a refusal envelope from parse-failure diagnostics, so the
/// orchestration layers (CLI, MCP) can report unparseable patch input inside
/// the normal `adoc.patch.apply.v0` shape instead of a process error.
pub fn patch_apply_refusal(diagnostics: Vec<Diagnostic>, interface: &str) -> PatchApplyResult {
    PatchApplyResult::refused(
        diagnostics,
        ApplyTrace {
            interface: interface.to_string(),
            proposer: None,
        },
    )
}

fn embedding_provider(
    provider: EmbeddingProviderSelection,
) -> Result<
    Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
    domain::ports::embedding_provider::EmbeddingError,
> {
    match provider {
        EmbeddingProviderSelection::Deterministic => Ok(Box::new(
            infrastructure::embedding::deterministic::DeterministicProvider::default(),
        )),
        EmbeddingProviderSelection::Local => local_embedding_provider(),
    }
}

fn local_embedding_provider() -> Result<
    Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
    domain::ports::embedding_provider::EmbeddingError,
> {
    #[cfg(feature = "test-embedding-provider")]
    if use_deterministic_test_embedding_provider() {
        return embedding_provider(EmbeddingProviderSelection::Deterministic);
    }

    #[cfg(feature = "test-embedding-provider")]
    if use_force_load_fail_test_embedding_provider() {
        return Err(
            domain::ports::embedding_provider::EmbeddingError::ModelLoad(
                "forced load failure for tests".into(),
            ),
        );
    }

    local_fast_embedding_provider()
}

#[cfg(feature = "embeddings")]
fn local_fast_embedding_provider() -> Result<
    Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
    domain::ports::embedding_provider::EmbeddingError,
> {
    infrastructure::embedding::fastembed::FastEmbedProvider::try_new().map(|provider| {
        Box::new(provider) as Box<dyn domain::ports::embedding_provider::EmbeddingProvider>
    })
}

#[cfg(not(feature = "embeddings"))]
fn local_fast_embedding_provider() -> Result<
    Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
    domain::ports::embedding_provider::EmbeddingError,
> {
    Err(domain::ports::embedding_provider::EmbeddingError::ModelLoad(
        "embedding support is disabled; rebuild with the `embeddings` feature or run with --no-embeddings".to_string(),
    ))
}

#[cfg(feature = "test-embedding-provider")]
fn use_deterministic_test_embedding_provider() -> bool {
    matches!(
        std::env::var("ADOC_TEST_EMBEDDING_PROVIDER").as_deref(),
        Ok("deterministic" | "in-memory")
    )
}

#[cfg(feature = "test-embedding-provider")]
fn use_force_load_fail_test_embedding_provider() -> bool {
    std::env::var("ADOC_TEST_EMBEDDING_PROVIDER").as_deref() == Ok("force-load-fail")
}

fn build_workspace_with_embedding_provider_factory<F>(
    input: BuildInput,
    provider_factory: F,
) -> CompileResult
where
    F: FnMut() -> Result<
        Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
        domain::ports::embedding_provider::EmbeddingError,
    >,
{
    build_workspace_with_embedding_provider_factory_and_context(input, None, provider_factory)
}

fn build_workspace_with_embedding_provider_factory_and_context<F>(
    input: BuildInput,
    project: Option<LocalProjectContext>,
    mut provider_factory: F,
) -> CompileResult
where
    F: FnMut() -> Result<
        Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
        domain::ports::embedding_provider::EmbeddingError,
    >,
{
    let provider = match project {
        Some(project) => infrastructure::source::fs::FsSourceProvider::for_project(
            input.root,
            project.project_root,
            project.docs_root,
        ),
        None => infrastructure::source::fs::FsSourceProvider::new(input.root),
    };
    match input.embeddings {
        BuildEmbeddingMode::Enabled => application::compile::build_with_provider(
            &provider,
            application::compile::BuildOptions {
                policy: input.policy,
                embeddings: application::compile::BuildEmbeddingBehavior::EnabledFactory {
                    provider_factory: &mut provider_factory,
                },
                prior_search_artifact_path: input.prior_search_artifact_path,
            },
        ),
        BuildEmbeddingMode::Skipped => application::compile::build_with_provider(
            &provider,
            application::compile::BuildOptions {
                policy: input.policy,
                embeddings: application::compile::BuildEmbeddingBehavior::Skipped,
                prior_search_artifact_path: input.prior_search_artifact_path,
            },
        ),
    }
}

fn build_workspace_with_embedding_provider_factory_and_context_for_date<F>(
    input: BuildInput,
    project: Option<LocalProjectContext>,
    mut provider_factory: F,
    evaluation_date: chrono::NaiveDate,
) -> CompileResult
where
    F: FnMut() -> Result<
        Box<dyn domain::ports::embedding_provider::EmbeddingProvider>,
        domain::ports::embedding_provider::EmbeddingError,
    >,
{
    let provider = match project {
        Some(project) => infrastructure::source::fs::FsSourceProvider::for_project(
            input.root,
            project.project_root,
            project.docs_root,
        ),
        None => infrastructure::source::fs::FsSourceProvider::new(input.root),
    };
    let options = application::compile::BuildOptions {
        policy: input.policy,
        embeddings: match input.embeddings {
            BuildEmbeddingMode::Enabled => {
                application::compile::BuildEmbeddingBehavior::EnabledFactory {
                    provider_factory: &mut provider_factory,
                }
            }
            BuildEmbeddingMode::Skipped => application::compile::BuildEmbeddingBehavior::Skipped,
        },
        prior_search_artifact_path: input.prior_search_artifact_path,
    };
    application::compile::build_with_provider_for_date(&provider, options, evaluation_date)
}

pub fn load_retrieval_session(input: RetrievalInput) -> RetrievalLoadResult {
    load_retrieval_session_with_embedding_provider(input, EmbeddingProviderSelection::Local)
}

#[tracing::instrument(level = "debug", skip_all)]
pub fn load_retrieval_session_with_embedding_provider(
    input: RetrievalInput,
    provider: EmbeddingProviderSelection,
) -> RetrievalLoadResult {
    // Only resolve the active provider's model header when a search-artifact
    // path is provided; lexical-only callers must not pay the embedding-model
    // metadata lookup. Resolution itself is metadata-only — no model download.
    let active_model = if input.search_artifact_path.is_some() {
        active_search_model_header_for(provider)
    } else {
        None
    };
    application::retrieval::load_retrieval_session_with_readers(
        input,
        &infrastructure::artifact::SearchJsonArtifact,
        &infrastructure::artifact::GraphJsonArtifact,
        active_model,
    )
}

pub fn inspect_graph_artifact(input: GraphArtifactInspectionInput) -> ArtifactInspection {
    application::artifact_inspection::inspect_graph_artifact(input)
}

pub fn inspect_search_artifact(input: SearchArtifactInspectionInput) -> ArtifactInspection {
    application::artifact_inspection::inspect_search_artifact(input)
}

/// Resolves the active embedding provider's `SearchModelHeader` without
/// loading the underlying model. Returns `None` when the binary was built
/// without the `embeddings` feature.
fn active_search_model_header_for(
    provider: EmbeddingProviderSelection,
) -> Option<domain::artifact::SearchModelHeader> {
    match provider {
        EmbeddingProviderSelection::Deterministic => {
            Some(infrastructure::embedding::deterministic::DeterministicProvider::metadata_header())
        }
        EmbeddingProviderSelection::Local => local_search_model_header(),
    }
}

fn local_search_model_header() -> Option<domain::artifact::SearchModelHeader> {
    #[cfg(feature = "test-embedding-provider")]
    if use_deterministic_test_embedding_provider() {
        return active_search_model_header_for(EmbeddingProviderSelection::Deterministic);
    }

    // When the test provider is set to force a load failure, return None so
    // that the model-mismatch gate is bypassed.  The failure is then surfaced
    // by `embed_query` itself, which is the code path the caller intends to
    // exercise.
    #[cfg(feature = "test-embedding-provider")]
    if use_force_load_fail_test_embedding_provider() {
        return None;
    }

    #[cfg(feature = "embeddings")]
    {
        Some(infrastructure::embedding::fastembed::FastEmbedProvider::metadata_header())
    }

    #[cfg(not(feature = "embeddings"))]
    {
        None
    }
}

pub use domain::sensitive_access::{
    SENSITIVE_ACCESS_SCHEMA_VERSION, SensitiveAccessCaller, SensitiveAccessCommand,
    SensitiveAccessContext, SensitiveAccessError, SensitiveAccessEvent, SensitiveAccessObject,
    SensitiveClassification, validate_sensitive_access,
};

pub use domain::managed_field_declassification::{
    MANAGED_FIELD_DECLASSIFICATION_SCHEMA_VERSION, ManagedFieldDeclassification,
    ManagedFieldDeclassificationError, ManagedFieldDeclassificationFieldInput,
    ManagedFieldDeclassificationInput, build_managed_field_declassification,
    validate_managed_field_declassification,
};

pub use domain::managed_field_provenance::{
    MANAGED_FIELD_PROVENANCE_SCHEMA_VERSION, ManagedFieldContribution,
    ManagedFieldContributionInput, ManagedFieldProvenance, ManagedFieldProvenanceError,
    ManagedFieldProvenanceInput, build_managed_field_provenance, strictest_contributing_visibility,
    validate_managed_field_provenance,
};

pub use application::migration::MigrationReceipt;
pub use domain::migration::{
    MIGRATION_RECEIPT_SCHEMA_VERSION, MIGRATION_REQUEST_MAX_BYTES,
    MIGRATION_REQUEST_SCHEMA_VERSION, MigrationError, MigrationRequest,
};

/// Prepare an exact commit from a worker-owned repository. The caller must provide
/// the isolated, network-denied execution boundary; this is not a sandbox.
pub fn prepare_migration_from_git(
    repository: &std::path::Path,
    request_bytes: &[u8],
    runtime_version: String,
    runtime_binary_digest: String,
) -> Result<MigrationReceipt, MigrationError> {
    let request = MigrationRequest::parse(request_bytes)?;
    let provider = infrastructure::git::worktree::GitWorktreeProvider::for_migration(
        repository,
        &request.revision.value,
    )?;
    application::migration::prepare_with_provider(
        request_bytes,
        &provider,
        resolve_migration_target,
        runtime_version,
        runtime_binary_digest,
    )
}

pub use application::migration::MigrationImportBundle;
pub use domain::migration::{
    MIGRATION_IMPORT_JOB_MAX_BYTES, MIGRATION_IMPORT_JOB_SCHEMA_VERSION,
    MIGRATION_IMPORT_MAX_BYTES, MIGRATION_IMPORT_SCHEMA_VERSION,
    MIGRATION_VALIDATION_INVOCATION_SCHEMA_VERSION,
};

/// Produce bounded inactive candidate inputs from an exact worker-owned snapshot.
pub fn import_migration_from_git(
    repository: &std::path::Path,
    request_bytes: &[u8],
    job_bytes: &[u8],
    runtime_version: String,
    runtime_binary_digest: String,
) -> Result<MigrationImportBundle, MigrationError> {
    let request = MigrationRequest::parse(request_bytes)?;
    domain::migration::MigrationImportJob::parse(job_bytes, &request)?;
    let provider = infrastructure::git::worktree::GitWorktreeProvider::for_migration(
        repository,
        &request.revision.value,
    )?;
    application::migration::import_with_provider(
        request_bytes,
        job_bytes,
        &provider,
        resolve_migration_target,
        runtime_version,
        runtime_binary_digest,
    )
}

fn resolve_migration_target(
    snapshot: &std::path::Path,
) -> Result<application::migration::MigrationValidationTarget, MigrationError> {
    let config_path = snapshot.join("agentdoc.config.yaml");
    let text =
        std::fs::read_to_string(&config_path).map_err(|_| MigrationError::ValidationUnavailable)?;
    let config = parse_project_config(&text).map_err(|_| MigrationError::ValidationUnavailable)?;
    let root = snapshot.join(config.docs_path);
    if !root.is_dir() {
        return Err(MigrationError::ValidationUnavailable);
    }
    Ok(application::migration::MigrationValidationTarget {
        project: Some(LocalProjectContext {
            project_root: snapshot.to_path_buf(),
            docs_root: root.clone(),
        }),
        root,
        config_path: Some(config_path),
        config_bytes: text,
    })
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;
    use crate::domain::ports::embedding_provider::{EmbeddingError, EmbeddingProvider};

    #[test]
    fn worktree_status_failure_emits_resolved_snapshot_failure() {
        let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .canonicalize()
            .expect("workspace root");
        let envelope = assess_changes_from_git_with_worktree_status(
            ChangeAssessmentInput {
                project_root: Some(project_root),
                base_ref: "HEAD".to_string(),
                head_ref: None,
                evaluation_date: chrono::NaiveDate::from_ymd_opt(2026, 7, 22).expect("date"),
            },
            |_| {
                Err(
                    domain::ports::snapshot_workspace::SnapshotError::ProviderUnavailable {
                        reason: "status unavailable".to_string(),
                    },
                )
            },
        );

        assert_eq!(envelope.completeness, AssessmentCompleteness::Error);
        assert_eq!(envelope.outcome, AssessmentOutcome::NotEvaluated);
        assert!(envelope.snapshots.head.resolved_commit.is_some());
        assert_eq!(envelope.snapshots.head.worktree_state, None);
        assert_eq!(
            envelope.diagnostics[0].code,
            DiagnosticCode::AssessmentSnapshotFailed.as_str()
        );
    }

    #[test]
    fn parse_patch_from_path_missing_returns_read_error_with_diagnostics() {
        let path = std::path::PathBuf::from("/nonexistent/v3.7/patch.json");
        let err = parse_patch_from_path(&path).expect_err("missing file must error");
        match err {
            PatchParseError::Read {
                path: reported,
                diagnostics,
            } => {
                assert_eq!(reported, path);
                assert!(
                    !diagnostics.is_empty(),
                    "diagnostics must explain the failure"
                );
            }
            other => panic!("expected Read variant, got: {other:?}"),
        }
    }

    #[test]
    fn parse_patch_from_value_with_wrong_shape_returns_inline_error() {
        let value = serde_json::json!({ "not": "a patch" });
        let err = parse_patch_from_value(value).expect_err("malformed JSON must error");
        match err {
            PatchParseError::Inline { diagnostics } => {
                assert!(!diagnostics.is_empty());
                assert_eq!(diagnostics[0].code, DiagnosticCode::PatchInvalidDocument);
            }
            other => panic!("expected Inline variant, got: {other:?}"),
        }
    }

    #[test]
    fn parse_patch_from_value_with_valid_v0_returns_patch_document() {
        let value = serde_json::json!({
            "schema_version": "adoc.patch.v0",
            "op": "replace_body",
            "target": "billing.credits",
            "base_hash": "sha256:billing.credits",
            "changes": { "body": "New body." },
            "reason": "demo"
        });
        let _parsed = parse_patch_from_value(value).expect("valid patch parses");
    }

    #[test]
    fn build_workspace_enabled_maps_default_embedding_provider_load_failure() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let result = build_workspace_with_embedding_provider_factory(
            BuildInput {
                policy: None,
                root: workspace.path().to_path_buf(),
                embeddings: BuildEmbeddingMode::Enabled,
                prior_search_artifact_path: None,
            },
            || Err(EmbeddingError::ModelLoad("model unavailable".to_string())),
        );

        assert!(result.has_errors());
        let artifacts = result
            .artifacts
            .expect("clean source still returns V0 artifacts on embedding failure");
        assert!(
            artifacts.search_json.is_none(),
            "failed embedding build should not return a search artifact"
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::EmbedModelLoadFailed)
        );
    }

    #[test]
    fn build_workspace_skipped_does_not_construct_default_embedding_provider() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        let constructed = Cell::new(false);
        let result = build_workspace_with_embedding_provider_factory(
            BuildInput {
                policy: None,
                root: workspace.path().to_path_buf(),
                embeddings: BuildEmbeddingMode::Skipped,
                prior_search_artifact_path: None,
            },
            || -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
                constructed.set(true);
                panic!("skipped build must not construct embedding provider")
            },
        );

        assert!(!constructed.get());
        assert!(!result.has_errors(), "skipped build should pass");
    }

    #[test]
    fn build_workspace_enabled_does_not_construct_embedding_provider_when_source_has_errors() {
        let workspace = tempfile::tempdir().expect("temp workspace");
        std::fs::write(
            workspace.path().join("guide.adoc"),
            "# Guide @doc(team.guide)\n\n<div>raw</div>\n",
        )
        .expect("source can be written");
        let constructed = Cell::new(false);

        let result = build_workspace_with_embedding_provider_factory(
            BuildInput {
                policy: None,
                root: workspace.path().to_path_buf(),
                embeddings: BuildEmbeddingMode::Enabled,
                prior_search_artifact_path: None,
            },
            || -> Result<Box<dyn EmbeddingProvider>, EmbeddingError> {
                constructed.set(true);
                panic!("source errors must not construct embedding provider")
            },
        );

        assert!(!constructed.get());
        assert!(result.has_errors(), "raw HTML should produce an error");
        assert!(result.artifacts.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == DiagnosticCode::ParseRawHtml)
        );
    }

    #[cfg(not(feature = "embeddings"))]
    #[test]
    fn build_workspace_enabled_without_embeddings_feature_reports_model_load_failure() {
        let workspace = tempfile::tempdir().expect("temp workspace");

        let result = build_workspace(BuildInput {
            policy: None,
            root: workspace.path().to_path_buf(),
            embeddings: BuildEmbeddingMode::Enabled,
            prior_search_artifact_path: None,
        });

        assert!(result.has_errors());
        let diagnostic = result
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == DiagnosticCode::EmbedModelLoadFailed)
            .expect("model load diagnostic");
        assert!(
            diagnostic.message.contains("`embeddings` feature"),
            "diagnostic should explain the missing feature: {diagnostic:?}"
        );
        assert!(
            diagnostic
                .help
                .as_deref()
                .expect("help")
                .contains("--no-embeddings")
        );
    }

    #[test]
    fn embed_query_compute_error_does_not_leak_debug_format() {
        let err = map_provider_error(EmbeddingError::Compute("encoder failed".to_string()));
        let msg = err.to_string();
        assert!(
            !msg.contains("Compute("),
            "Debug variant name must not appear in user message: {msg}"
        );
        assert!(
            !msg.contains('"'),
            "Debug-style quotes must not appear in user message: {msg}"
        );
        assert!(
            !msg.contains('{'),
            "Debug-style braces must not appear in user message: {msg}"
        );
        assert!(
            msg.contains("encoder failed"),
            "Inner message must be preserved verbatim: {msg}"
        );
    }

    #[test]
    fn embed_query_model_load_error_does_not_leak_debug_format() {
        let err = map_provider_error(EmbeddingError::ModelLoad("model unavailable".to_string()));
        let msg = err.to_string();
        assert!(
            !msg.contains("ModelLoad("),
            "Debug variant name must not appear in user message: {msg}"
        );
        assert!(
            !msg.contains('"'),
            "Debug-style quotes must not appear in user message: {msg}"
        );
        assert!(
            !msg.contains('{'),
            "Debug-style braces must not appear in user message: {msg}"
        );
        assert!(
            msg.contains("model unavailable"),
            "Inner message must be preserved verbatim: {msg}"
        );
    }

    #[test]
    fn embed_query_dimension_mismatch_maps_to_compute_with_dims() {
        let err = map_provider_error(EmbeddingError::DimensionMismatch {
            expected: 384,
            actual: 512,
        });
        let msg = err.to_string();
        assert!(
            matches!(err, EmbedQueryError::Compute(_)),
            "DimensionMismatch must map to Compute variant"
        );
        assert!(
            msg.contains("384"),
            "Expected dim must appear in message: {msg}"
        );
        assert!(
            msg.contains("512"),
            "Actual dim must appear in message: {msg}"
        );
        assert!(
            !msg.contains("DimensionMismatch"),
            "Debug variant name must not appear in user message: {msg}"
        );
    }

    #[cfg(feature = "test-embedding-provider")]
    #[test]
    fn test_embedding_provider_env_uses_deterministic_only_when_explicitly_requested() {
        temp_env_remove("ADOC_TEST_EMBEDDING_PROVIDER", || {
            assert!(!use_deterministic_test_embedding_provider());
        });
        temp_env_set("ADOC_TEST_EMBEDDING_PROVIDER", "fastembed", || {
            assert!(!use_deterministic_test_embedding_provider());
        });
        temp_env_set("ADOC_TEST_EMBEDDING_PROVIDER", "deterministic", || {
            assert!(use_deterministic_test_embedding_provider());
        });
        temp_env_set("ADOC_TEST_EMBEDDING_PROVIDER", "in-memory", || {
            assert!(use_deterministic_test_embedding_provider());
        });
    }

    #[cfg(feature = "test-embedding-provider")]
    fn temp_env_remove(name: &str, test: impl FnOnce()) {
        let previous = std::env::var_os(name);
        unsafe {
            std::env::remove_var(name);
        }
        test();
        restore_env(name, previous);
    }

    #[cfg(feature = "test-embedding-provider")]
    fn temp_env_set(name: &str, value: &str, test: impl FnOnce()) {
        let previous = std::env::var_os(name);
        unsafe {
            std::env::set_var(name, value);
        }
        test();
        restore_env(name, previous);
    }

    #[cfg(feature = "test-embedding-provider")]
    fn restore_env(name: &str, value: Option<std::ffi::OsString>) {
        unsafe {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}
