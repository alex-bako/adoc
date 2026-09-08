//! AgentDoc Validation Runtime receipts (E1.7; SEMANTICS §S6/§S10;
//! MILESTONES §E1.7; PRD §6.7).
//!
//! All AgentDoc-domain validation runs through this pinned runtime; it
//! returns a registered `adoc.validation_receipt.v0` or `.v1` envelope binding the
//! exact runtime identity, the digest of every compiled source input and
//! named validation-context input, the consumed contract versions, the
//! closed `pass | fail` result, and a digest of the typed diagnostics.
//! Receipts are deterministic: stable ordering, no wall-clock timestamps
//! anywhere — lifecycle evaluation pins to an explicit `evaluation_date`
//! input, never "today".
//!
//! Scope of the digest binding: Evidence Anchor reads (ADR-0048) sit
//! OUTSIDE it. Anchor checking is advisory — every anchor diagnostic is a
//! warning (`check_evidence_anchors` constructs warnings only), so anchor
//! bytes can never flip `result`; they can shape warning diagnostics and
//! therefore `diagnostics_digest`, and they are deliberately not digested
//! into `inputs`. Digest-bound evidence is E4.1's Source Record surface.
//!
//! Validator-only construction (stop-ship, MILESTONES §E1.7): the fields of
//! [`ValidationReceipt`] are private. The public runtime and crate-private
//! migration entry share one validator body; unvalidated JSON has no core
//! representation downstream code can consume. The guarantee rests on field privacy and
//! the deliberate ABSENCE of a `Deserialize` derive — never derive
//! `Deserialize` on [`ValidationReceipt`]: a consumer of receipt bytes must
//! parse into its own shape and verify digests, or forged receipts become
//! constructible from bytes while every existing test stays green (E1.3
//! precedent, `domain/reconciliation.rs`). Compile-time visibility proofs
//! live as `compile_fail` doctests on the type.
//!
//! The runtime binary digest is supplied by the invoking harness as an
//! explicit attested input — a binary cannot hash itself deterministically.
//! The T2 harness (`scripts/validation-runtime/run.sh`) verifies the
//! binary's sha256 against its recorded pin before invoking and passes the
//! verified digest through; the receipt records what the harness attested.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use serde::Serialize;
use thiserror::Error;

use crate::application::compile::{LocalProjectContext, compile_with_provider_anchored_for_date};
use crate::domain::diagnostic::{Diagnostic, DiagnosticCode, Severity};
use crate::domain::graph::{GraphArtifactDocument, GraphNode, GraphRepositoryIdentity};
use crate::domain::hashing::sha256_prefixed;
use crate::domain::ports::source_provider::{SourceLoadError, SourceProvider};
use crate::domain::source::SourceFile;
use crate::infrastructure::artifact::graph_json::parse_graph_artifact_document;
use crate::infrastructure::source::evidence_fs::FsEvidenceFileReader;
use crate::infrastructure::source::fs::FsSourceProvider;

const VALIDATION_RECEIPT_V0_SCHEMA_VERSION: &str = "adoc.validation_receipt.v0";
const VALIDATION_RECEIPT_V1_SCHEMA_VERSION: &str = "adoc.validation_receipt.v1";

/// The graph contract whose semantics this runtime validates; recorded under
/// `contract_versions` in every receipt.
const GRAPH_CONTRACT_KEY: &str = "graph";

/// A validation request against a resolved source root. Resolution (config
/// discovery, docs-path selection, path policy) stays in the Local Workflow
/// Layer; the runtime receives exactly what `adoc check` would compile.
#[derive(Debug, Clone)]
pub struct ValidationRuntimeInput {
    /// Source file or directory to validate (the resolved check root).
    pub root: PathBuf,
    /// Project context when the root belongs to a configured project.
    pub project: Option<LocalProjectContext>,
    /// Evidence Anchor resolution root (ADR-0048), as check threads it.
    /// Anchor reads are outside the receipt's digest binding — advisory
    /// warnings only, never result-affecting (see the module docs).
    pub anchor_root: PathBuf,
    /// Explicit lifecycle evaluation date. Mandatory: receipts never read
    /// the wall clock.
    pub evaluation_date: NaiveDate,
    /// Release version of the invoking runtime surface (the `adoc` binary).
    pub runtime_version: String,
    /// Harness-attested sha256 digest of the invoking binary
    /// (`sha256:<64 hex>`); see the module docs.
    pub runtime_binary_digest: String,
    /// Discovered project config file, digested as validation context.
    pub config_path: Option<PathBuf>,
    /// Cloud-supplied immutable source invocation manifest. Its digest binds
    /// namespace, revision, Source Binding, ACL snapshot, and config evidence
    /// without teaching the validation runtime Cloud semantics.
    pub source_invocation: Option<PathBuf>,
    /// Graph artifact to validate against the recompiled source
    /// (E1.7.T4). Consumed exact-match: any `schema_version` other than
    /// `adoc.graph.v6` is rejected with `schema.unsupported_version`, and
    /// a schema-valid artifact whose governed-meaning content hashes do
    /// not correspond to the source fails with
    /// `validation.context_artifact_drift` — JSON Schema stays preflight
    /// only (ADR-0015).
    pub context_artifact: Option<PathBuf>,
    /// Digest-bound semantic context validated against the explicit
    /// evaluation date and, for graph-backed context, the same once-read
    /// graph artifact supplied above (E3.1).
    pub semantic_context: Option<PathBuf>,
    /// Trusted exact revisions and assessment digest the semantic context
    /// must match. A semantic context without these expectations fails
    /// closed rather than treating its self-declared bindings as authority.
    pub semantic_context_expectations: Option<crate::SemanticContextExpectedBindings>,
}

/// A validation run: the digest-bound receipt plus the ordinary typed
/// diagnostics for human/agent presentation. `diagnostics_digest` inside the
/// receipt is computed over exactly this array's canonical serialization.
#[derive(Debug, Clone)]
pub struct ValidationRuntimeOutcome {
    pub receipt: ValidationReceipt,
    pub diagnostics: Vec<Diagnostic>,
    pub(crate) graph_artifact: Option<String>,
    pub(crate) source_files: Vec<SourceFile>,
}

/// Digest-bound AgentDoc validation receipt (SEMANTICS §S6).
///
/// Constructible only by this module's shared runtime validator; serialized
/// with [`ValidationReceipt::to_canonical_json`]. Unvalidated JSON has no core
/// representation:
///
/// ```compile_fail
/// // Private fields (E0451): no literal construction outside the validator
/// // module. All eight fields are named so only field privacy can fail this
/// // — a missing-fields E0063 could otherwise mask fields regressing to pub.
/// let receipt = adoc_core::ValidationReceipt {
///     schema_version: todo!(),
///     runtime: todo!(),
///     contract_versions: todo!(),
///     evaluation_date: todo!(),
///     inputs: todo!(),
///     context: todo!(),
///     result: todo!(),
///     diagnostics_digest: todo!(),
/// };
/// ```
///
/// ```compile_fail
/// // No Deserialize impl: receipt bytes cannot become the typed envelope.
/// fn requires_deserialize<T: serde::de::DeserializeOwned>() {}
/// requires_deserialize::<adoc_core::ValidationReceipt>();
/// ```
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReceipt {
    schema_version: String,
    runtime: RuntimeIdentity,
    contract_versions: BTreeMap<String, String>,
    evaluation_date: String,
    inputs: Vec<DigestEntry>,
    context: Vec<NamedDigestEntry>,
    result: ValidationResult,
    diagnostics_digest: String,
}

#[derive(Debug, Clone, Serialize)]
struct RuntimeIdentity {
    version: String,
    binary_digest: String,
}

/// One consumed source input: Logical Source Path plus content digest.
#[derive(Debug, Clone, Serialize)]
struct DigestEntry {
    path: String,
    digest: String,
}

/// One named validation-context input (`config`, `source_invocation`,
/// `context_artifact`, or `semantic_context`).
#[derive(Debug, Clone, Serialize)]
struct NamedDigestEntry {
    name: String,
    digest: String,
}

/// Closed result vocabulary (CONTRACT-REGISTRY.md): `pass | fail`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidationResult {
    Pass,
    Fail,
}

impl ValidationReceipt {
    /// The canonical serialized form: pretty JSON in declared field order
    /// with a trailing newline. Byte-identical across invocations, machines,
    /// and invocation surfaces for the same inputs (E1.7 exit gate).
    pub fn to_canonical_json(&self) -> String {
        let mut serialized = serde_json::to_string_pretty(self)
            .unwrap_or_else(|_| unreachable!("receipt fields serialize infallibly"));
        serialized.push('\n');
        serialized
    }

    pub fn result(&self) -> ValidationResult {
        self.result
    }
}

#[derive(Debug, Error)]
pub enum ValidationRuntimeError {
    #[error(
        "runtime binary digest '{digest}' is not 'sha256:' plus 64 lowercase hex characters; \
         the invoking harness must pass the digest it verified against its recorded pin"
    )]
    InvalidRuntimeBinaryDigest { digest: String },
    #[error("validation context '{path}' is unreadable: {message}")]
    ContextUnreadable { path: PathBuf, message: String },
}

/// Run AgentDoc-domain validation and construct the digest-bound receipt through
/// the same validator body used by exact-snapshot migration imports.
pub fn run_validation_runtime(
    input: ValidationRuntimeInput,
) -> Result<ValidationRuntimeOutcome, ValidationRuntimeError> {
    let provider = match &input.project {
        Some(project) => FsSourceProvider::for_project(
            input.root.clone(),
            project.project_root.clone(),
            project.docs_root.clone(),
        ),
        None => FsSourceProvider::new(input.root.clone()),
    };
    run_with_provider(&provider, input)
}

/// The provider-generic body of [`run_validation_runtime`] — the test seam
/// pinning that every input is read exactly once: the receipt's digests and
/// the validated content come from the same read, so a file changing
/// between "digest" and "validate" cannot yield a receipt attesting digests
/// that were never validated.
fn run_with_provider<P: SourceProvider>(
    provider: &P,
    input: ValidationRuntimeInput,
) -> Result<ValidationRuntimeOutcome, ValidationRuntimeError> {
    run_with_context_bytes(provider, input, None)
}

/// Validate a migration's once-read raw snapshot, including honest UTF-8 failures.
pub(crate) fn run_migration_snapshot_validation(
    input: ValidationRuntimeInput,
    sources: &[Result<SourceFile, SourceLoadError>],
) -> Result<ValidationRuntimeOutcome, ValidationRuntimeError> {
    let provider = match &input.project {
        Some(project) => FsSourceProvider::for_project(
            input.root.clone(),
            project.project_root.clone(),
            project.docs_root.clone(),
        ),
        None => FsSourceProvider::new(input.root.clone()),
    };
    run_with_provider(
        &SnapshotSourceProvider {
            sources: sources.to_vec(),
            inner: &provider,
        },
        input,
    )
}

/// Revalidate the whole once-read snapshot with exact migration context bytes.
/// The same validator constructs the receipt; no externally supplied receipt is consumed.
pub(crate) fn run_migration_validation(
    mut input: ValidationRuntimeInput,
    source_files: &[SourceFile],
    invocation: &[u8],
    graph: &[u8],
) -> Result<ValidationRuntimeOutcome, ValidationRuntimeError> {
    let provider = match &input.project {
        Some(project) => FsSourceProvider::for_project(
            input.root.clone(),
            project.project_root.clone(),
            project.docs_root.clone(),
        ),
        None => FsSourceProvider::new(input.root.clone()),
    };
    let snapshot = SnapshotSourceProvider {
        sources: source_files.iter().cloned().map(Ok).collect(),
        inner: &provider,
    };
    input.source_invocation = Some(PathBuf::from("source_invocation"));
    input.context_artifact = Some(PathBuf::from("context_artifact"));
    run_with_context_bytes(&snapshot, input, Some((invocation, graph)))
}

fn run_with_context_bytes<P: SourceProvider>(
    provider: &P,
    input: ValidationRuntimeInput,
    migration_context: Option<(&[u8], &[u8])>,
) -> Result<ValidationRuntimeOutcome, ValidationRuntimeError> {
    require_sha256_digest(&input.runtime_binary_digest)?;
    let receipt_schema_version = if input.source_invocation.is_some() {
        VALIDATION_RECEIPT_V1_SCHEMA_VERSION
    } else {
        VALIDATION_RECEIPT_V0_SCHEMA_VERSION
    };

    // ONE source read serves both the digests and the compile below.
    let sources = provider.load_sources();
    let inputs = source_input_digests(&sources);
    let mut context = Vec::new();
    if let Some(config_path) = &input.config_path {
        context.push(NamedDigestEntry {
            name: "config".to_string(),
            digest: file_digest(config_path)?,
        });
    }
    if let Some(source_invocation) = &input.source_invocation {
        context.push(NamedDigestEntry {
            name: "source_invocation".to_string(),
            digest: match migration_context {
                Some((bytes, _)) => sha256_prefixed(bytes),
                None => file_digest(source_invocation)?,
            },
        });
    }

    // ONE artifact read: the same bytes are digested and validated.
    let context_artifact_bytes = match &input.context_artifact {
        Some(artifact_path) => {
            let bytes = match migration_context {
                Some((_, bytes)) => bytes.to_vec(),
                None => fs::read(artifact_path).map_err(|error| {
                    ValidationRuntimeError::ContextUnreadable {
                        path: artifact_path.clone(),
                        message: error.to_string(),
                    }
                })?,
            };
            context.push(NamedDigestEntry {
                name: "context_artifact".to_string(),
                digest: sha256_prefixed(&bytes),
            });
            Some(bytes)
        }
        None => None,
    };

    // ONE semantic-context read: the receipt digest and domain validation
    // consume the same byte snapshot.
    let semantic_context_bytes = match &input.semantic_context {
        Some(semantic_path) => {
            let bytes = fs::read(semantic_path).map_err(|error| {
                ValidationRuntimeError::ContextUnreadable {
                    path: semantic_path.clone(),
                    message: error.to_string(),
                }
            })?;
            context.push(NamedDigestEntry {
                name: "semantic_context".to_string(),
                digest: sha256_prefixed(&bytes),
            });
            Some(bytes)
        }
        None => None,
    };

    let snapshot = SnapshotSourceProvider {
        sources,
        inner: provider,
    };
    let reader = FsEvidenceFileReader::new(input.anchor_root.clone());
    let compiled =
        compile_with_provider_anchored_for_date(&snapshot, &reader, input.evaluation_date);
    let mut diagnostics = compiled.diagnostics;
    if let (Some(artifact_path), Some(bytes)) = (&input.context_artifact, &context_artifact_bytes) {
        diagnostics.extend(validate_context_artifact(
            artifact_path,
            bytes,
            compiled
                .artifacts
                .as_ref()
                .map(|artifacts| artifacts.graph_json.as_str()),
        ));
    }
    if let Some(bytes) = &semantic_context_bytes {
        match &input.semantic_context_expectations {
            Some(expectations) => {
                match semantic_validation_basis(
                    input.evaluation_date,
                    context_artifact_bytes.as_deref(),
                    expectations,
                )
                .and_then(|basis| crate::validate_semantic_context(bytes, &basis))
                {
                    Ok(semantic_context) => {
                        if let Some(code) = semantic_context.outcome().diagnostic_code() {
                            diagnostics.push(Diagnostic::error(
                                code,
                                format!(
                                    "semantic context outcome is {}",
                                    semantic_context.outcome().as_str()
                                ),
                            ));
                        }
                    }
                    Err(error) => diagnostics.push(Diagnostic::error(
                        error.diagnostic_code(),
                        error.to_string(),
                    )),
                }
            }
            None => diagnostics.push(Diagnostic::error(
                DiagnosticCode::SemanticContextBasisMismatch,
                "semantic context exact revision and assessment expectations were not supplied",
            )),
        }
    }

    let result = if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == Severity::Error)
    {
        ValidationResult::Fail
    } else {
        ValidationResult::Pass
    };

    let diagnostics_digest = sha256_prefixed(
        &serde_json::to_vec(&diagnostics)
            .unwrap_or_else(|_| unreachable!("diagnostics serialize infallibly")),
    );

    let mut contract_versions = BTreeMap::from([(
        GRAPH_CONTRACT_KEY.to_string(),
        crate::infrastructure::artifact::graph_json::SUPPORTED_GRAPH_SCHEMA_VERSION.to_string(),
    )]);
    if semantic_context_bytes.is_some() {
        contract_versions.insert(
            "semantic_context".to_string(),
            crate::SEMANTIC_CONTEXT_SCHEMA_VERSION.to_string(),
        );
    }
    let receipt = ValidationReceipt {
        schema_version: receipt_schema_version.to_string(),
        runtime: RuntimeIdentity {
            version: input.runtime_version,
            binary_digest: input.runtime_binary_digest,
        },
        contract_versions,
        evaluation_date: input.evaluation_date.format("%Y-%m-%d").to_string(),
        inputs,
        context,
        result,
        diagnostics_digest,
    };

    Ok(ValidationRuntimeOutcome {
        receipt,
        diagnostics,
        graph_artifact: compiled.artifacts.map(|artifacts| artifacts.graph_json),
        source_files: snapshot
            .sources
            .into_iter()
            .filter_map(Result::ok)
            .collect(),
    })
}

fn semantic_validation_basis(
    evaluation_date: NaiveDate,
    graph_artifact_bytes: Option<&[u8]>,
    expectations: &crate::SemanticContextExpectedBindings,
) -> Result<crate::SemanticContextValidationBasis, crate::SemanticContextError> {
    let graph_artifact_digest = graph_artifact_bytes.map(sha256_prefixed);
    let mut graph_object_contexts = BTreeMap::new();
    for context in &expectations.graph_object_contexts {
        if !crate::is_semantic_context_text(&context.object_id)
            || !crate::is_semantic_context_text(&context.class_id)
            || !crate::is_semantic_context_text(&context.scope_ref)
        {
            return Err(crate::SemanticContextError::BasisMismatch {
                message: "graph object context basis is invalid".to_string(),
            });
        }
        if graph_object_contexts
            .insert(context.object_id.as_str(), context)
            .is_some()
        {
            return Err(crate::SemanticContextError::BasisMismatch {
                message: "graph object context basis contains duplicate Object IDs".to_string(),
            });
        }
    }
    let graph_document = graph_artifact_bytes
        // Parsing failures were already emitted by `validate_context_artifact`.
        // An empty projection keeps citation resolution fail-closed without
        // duplicating those artifact diagnostics.
        .and_then(|bytes| parse_graph_artifact_document(Path::new("context_artifact"), bytes).ok());
    let graph_was_parsed = graph_document.is_some();
    let (graph_objects, citation_contents) = graph_document
        .map(|document| {
            let mut objects = Vec::new();
            let mut contents = Vec::new();
            for node in document.nodes {
                let GraphNode::KnowledgeObject(object) = node else {
                    continue;
                };
                let object_id = object.id;
                let semantic_hash = object.content_hash;
                if let Some(context) = graph_object_contexts.remove(object_id.as_str()) {
                    contents.push(crate::CitationContentProjection {
                        handle: crate::CitationHandle::KnowledgeObject {
                            object_id: object_id.clone(),
                            semantic_hash: semantic_hash.clone(),
                        },
                        class_id: context.class_id.clone(),
                        scope_ref: context.scope_ref.clone(),
                        content_digest: crate::semantic_context_content_digest(
                            &serde_json::json!({ "body": object.body }),
                        ),
                        truncated_content_digests: Vec::new(),
                    });
                    if let Some(binding) = &object.source_binding {
                        contents.push(crate::CitationContentProjection {
                            handle: crate::CitationHandle::SourceBinding {
                                object_id: object_id.clone(),
                            },
                            class_id: context.class_id.clone(),
                            scope_ref: context.scope_ref.clone(),
                            content_digest: crate::semantic_context_content_digest(
                                &serde_json::to_value(binding).unwrap_or_else(|_| {
                                    unreachable!("GraphSourceBinding serialization is infallible")
                                }),
                            ),
                            truncated_content_digests: Vec::new(),
                        });
                    }
                    for (index, evidence) in object.evidence.iter().enumerate() {
                        contents.push(crate::CitationContentProjection {
                            handle: crate::CitationHandle::Evidence {
                                object_id: object_id.clone(),
                                evidence_index: u32::try_from(index).unwrap_or_else(|_| {
                                    unreachable!("graph evidence count is bounded by address space")
                                }),
                            },
                            class_id: context.class_id.clone(),
                            scope_ref: context.scope_ref.clone(),
                            content_digest: crate::semantic_context_content_digest(
                                &serde_json::to_value(evidence).unwrap_or_else(|_| {
                                    unreachable!("GraphEvidence serialization is infallible")
                                }),
                            ),
                            truncated_content_digests: Vec::new(),
                        });
                    }
                }
                objects.push(crate::GraphCitationObject {
                    object_id,
                    semantic_hash,
                    has_source_binding: object.source_binding.is_some(),
                    evidence_count: object.evidence.len(),
                });
            }
            (objects, contents)
        })
        .unwrap_or_default();
    if !graph_object_contexts.is_empty() && graph_was_parsed {
        return Err(crate::SemanticContextError::BasisMismatch {
            message: "graph object context basis contains unknown Object IDs".to_string(),
        });
    }
    Ok(crate::SemanticContextValidationBasis {
        evaluation_date,
        subject_revision: expectations.subject_revision.clone(),
        source_revision: expectations.source_revision.clone(),
        base_revision: expectations.base_revision.clone(),
        head_revision: expectations.head_revision.clone(),
        assessment_digest: expectations.assessment_digest.clone(),
        selection_algorithm: expectations.selection_algorithm.clone(),
        selection_version: expectations.selection_version.clone(),
        context_classes: expectations.context_classes.clone(),
        authorized_scope: expectations.authorized_scope.clone(),
        capability_policy: expectations.capability_policy.clone(),
        graph_artifact_digest,
        managed_revision_digest: None,
        graph_objects,
        // Diff hunks and Source Assertions resolve against Source Records
        // (E4.1); local receipt mode has neither projection, so citations
        // to either handle kind fail closed until that wave supplies them.
        diff_hunks: Vec::new(),
        source_assertions: Vec::new(),
        citation_contents,
    })
}

/// The compile input pinned to the exact bytes the receipt digests: yields
/// the one snapshot [`run_with_provider`] already read, so digesting and
/// validating can never observe different filesystem states. Repository
/// identity and `contains` stay with the real provider — only the source
/// read is pinned.
struct SnapshotSourceProvider<'a, P: SourceProvider> {
    sources: Vec<Result<SourceFile, SourceLoadError>>,
    inner: &'a P,
}

impl<P: SourceProvider> SourceProvider for SnapshotSourceProvider<'_, P> {
    fn load_sources(&self) -> Vec<Result<SourceFile, SourceLoadError>> {
        self.sources.clone()
    }

    fn repository_identity(&self) -> GraphRepositoryIdentity {
        self.inner.repository_identity()
    }

    fn contains(&self, path: &Path) -> bool {
        self.inner.contains(path)
    }
}

/// Digest every loaded source, keyed by Logical Source Path (portable,
/// project-relative) in lexicographic order. Load failures carry no digest —
/// the compile run over the same snapshot reports them as typed diagnostics
/// and the result fails closed.
fn source_input_digests(sources: &[Result<SourceFile, SourceLoadError>]) -> Vec<DigestEntry> {
    let mut digests = BTreeMap::new();
    for source in sources.iter().flatten() {
        digests.insert(
            source.logical_path.to_string_lossy().into_owned(),
            sha256_prefixed(source.text.as_bytes()),
        );
    }
    digests
        .into_iter()
        .map(|(path, digest)| DigestEntry { path, digest })
        .collect()
}

fn file_digest(path: &PathBuf) -> Result<String, ValidationRuntimeError> {
    let bytes = fs::read(path).map_err(|error| ValidationRuntimeError::ContextUnreadable {
        path: path.clone(),
        message: error.to_string(),
    })?;
    Ok(sha256_prefixed(&bytes))
}

/// E1.7.T4: domain validation of a context artifact against the recompiled
/// source. `contents` is the exact byte snapshot the receipt digested.
/// `parse_graph_artifact_document` enforces the exact-match version gate
/// (`schema.unsupported_version` for v5 and v99 alike) and JSON shape; the
/// governed-meaning comparison below is what JSON Schema can never see:
/// every Knowledge Object must equal the recompiled source's, and the node
/// and edge sets must coincide. When the source itself has errors the
/// recompiled artifact does not exist and the run already fails, so only
/// the version/shape gate applies.
fn validate_context_artifact(
    artifact_path: &Path,
    contents: &[u8],
    recompiled_graph_json: Option<&str>,
) -> Vec<Diagnostic> {
    let document = match parse_graph_artifact_document(artifact_path, contents) {
        Ok(document) => document,
        Err(diagnostics) => return diagnostics,
    };
    let Some(recompiled_graph_json) = recompiled_graph_json else {
        return Vec::new();
    };
    let recompiled: GraphArtifactDocument = match serde_json::from_str(recompiled_graph_json) {
        Ok(recompiled) => recompiled,
        // Cross-module invariant (compile.rs serialization ↔ graph/mod.rs
        // deserialization): its own graph_json always parses today, but a
        // future serde asymmetry must degrade to a typed fail-closed
        // diagnostic — never a panic in the receipt path.
        Err(error) => {
            return vec![Diagnostic::error(
                DiagnosticCode::IoArtifactMalformed,
                format!("recompiled graph artifact is malformed: {error}"),
            )];
        }
    };

    let artifact_objects = knowledge_objects(&document);
    let recompiled_objects = knowledge_objects(&recompiled);
    let mut diagnostics = Vec::new();
    for (id, artifact_object) in &artifact_objects {
        match recompiled_objects.get(id) {
            None => diagnostics.push(drift_diagnostic(
                id,
                "records a Knowledge Object the compiled source does not contain",
            )),
            // Full-node comparison, never hash strings alone: the
            // recorded content_hash is the producer's own claim, so a
            // forgery that tampers a governed field but replays the
            // honest hash would compare the claim against itself.
            Some(recompiled_object) if recompiled_object != artifact_object => {
                diagnostics.push(drift_diagnostic(
                    id,
                    "records a Knowledge Object that does not match the recompiled source",
                ));
            }
            Some(_) => {}
        }
    }
    for id in recompiled_objects.keys() {
        if !artifact_objects.contains_key(id) {
            diagnostics.push(drift_diagnostic(
                id,
                "does not record a Knowledge Object the compiled source contains",
            ));
        }
    }
    // Prose nodes and edges are governed carriage too. Wholesale
    // equality catches everything outside the per-object comparison;
    // `diagnostics` and `repository_identity` stay outside the
    // comparison — both can legitimately differ between the build and
    // check contexts (e.g. build-only embedding notices).
    if diagnostics.is_empty()
        && (document.nodes != recompiled.nodes || document.edges != recompiled.edges)
    {
        diagnostics.push(Diagnostic::error(
            DiagnosticCode::ValidationContextArtifactDrift,
            "the context artifact carries nodes or edges that do not match the recompiled source",
        ));
    }
    diagnostics
}

fn knowledge_objects(
    document: &GraphArtifactDocument,
) -> BTreeMap<&str, &crate::domain::graph::GraphKnowledgeObjectNode> {
    document
        .nodes
        .iter()
        .filter_map(|node| match node {
            GraphNode::KnowledgeObject(object) => Some((object.id.as_str(), object)),
            _ => None,
        })
        .collect()
}

/// Drift diagnostics deliberately carry no filesystem path: the message
/// feeds `diagnostics_digest`, and the receipt must stay byte-identical
/// across machines (the artifact is already identified by its digest in the
/// receipt's `context`; a caller-facing path would be absolute under
/// `ProjectRootPathPolicy`).
fn drift_diagnostic(object_id: &str, detail: &str) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::ValidationContextArtifactDrift,
        format!("the context artifact {detail} for `{object_id}`"),
    )
    .with_object_id(object_id)
}

fn require_sha256_digest(digest: &str) -> Result<(), ValidationRuntimeError> {
    if crate::is_sha256_digest(digest) {
        return Ok(());
    }
    Err(ValidationRuntimeError::InvalidRuntimeBinaryDigest {
        digest: digest.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    const TEST_DIGEST: &str =
        "sha256:ca1bf018dc0b72ee1197d9d521d96d227cd3e54cc81528ea5f45776c99d95f4d";

    fn write(path: &Path, contents: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("parent dir");
        }
        fs::write(path, contents).expect("fixture write");
    }

    #[test]
    fn migration_qualification_raw_snapshot_preserves_receipt_and_failed_input_honesty() {
        let project = tempfile::tempdir().unwrap();
        let docs = project.path().join("docs");
        write(
            &docs.join("valid.adoc"),
            "# Valid @doc(test.page)\n\n::claim test.claim\nstatus: draft\n--\nBody.\n::\n",
        );
        fs::write(docs.join("invalid.adoc"), [0xff, 0xfe]).unwrap();
        let config = project.path().join("agentdoc.config.yaml");
        write(&config, "version: 1\nmode: strict\ndocs_path: docs\n");
        let mut input = standalone_input(&docs);
        input.anchor_root = project.path().to_path_buf();
        input.project = Some(super::super::compile::LocalProjectContext {
            project_root: project.path().to_path_buf(),
            docs_root: docs.clone(),
        });
        input.config_path = Some(config);
        let ordinary = run_validation_runtime(input.clone()).unwrap();
        let provider =
            FsSourceProvider::for_project(docs.clone(), project.path().to_path_buf(), docs.clone());
        let raw = provider.load_raw_migration_sources(4096).unwrap();
        assert_eq!(raw[0].bytes, [0xff, 0xfe]);
        assert!(provider.load_raw_migration_sources(1).is_err());
        // Once-read source bytes still determine both validation and receipt after disk changes.
        fs::write(docs.join("invalid.adoc"), "# Now valid\n").unwrap();
        fs::write(docs.join("valid.adoc"), "dirty invalid input").unwrap();
        let result = run_migration_snapshot_validation(
            input,
            &raw.iter()
                .map(|source| source.loaded.clone())
                .collect::<Vec<_>>(),
        )
        .unwrap();
        assert_eq!(
            ordinary.receipt.to_canonical_json(),
            result.receipt.to_canonical_json()
        );
        assert_eq!(result.receipt.result(), ValidationResult::Fail);
        assert!(result.graph_artifact.is_none());
        let receipt: serde_json::Value =
            serde_json::from_str(&result.receipt.to_canonical_json()).unwrap();
        assert_eq!(receipt["inputs"].as_array().unwrap().len(), 1);
        assert_eq!(receipt["inputs"][0]["path"], "docs/valid.adoc");
        assert_eq!(
            receipt["diagnostics_digest"],
            sha256_prefixed(&serde_json::to_vec(&result.diagnostics).unwrap())
        );
        assert_eq!(receipt["context"].as_array().unwrap().len(), 1);
        assert_eq!(receipt["context"][0]["name"], "config");
    }

    fn standalone_input(root: &Path) -> ValidationRuntimeInput {
        ValidationRuntimeInput {
            root: root.to_path_buf(),
            project: None,
            anchor_root: root.to_path_buf(),
            evaluation_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            runtime_version: "0.4.0".to_string(),
            runtime_binary_digest: TEST_DIGEST.to_string(),
            config_path: None,
            source_invocation: None,
            context_artifact: None,
            semantic_context: None,
            semantic_context_expectations: None,
        }
    }

    #[test]
    fn migration_validation_reuses_all_source_bytes_and_checks_bound_graph() {
        let workspace = tempfile::tempdir().expect("workspace");
        let root = workspace.path();
        write(&root.join("one.adoc"), valid_source());
        write(
            &root.join("two.adoc"),
            "# Other @doc(test.other)\n\nIndependent prose.\n",
        );
        let input = standalone_input(root);
        let original = run_validation_runtime(input.clone()).expect("original validation");
        assert_eq!(original.receipt.result(), ValidationResult::Pass);
        assert_eq!(original.source_files.len(), 2);
        let graph = original.graph_artifact.as_ref().expect("compiled graph");
        write(&root.join("one.adoc"), "::claim broken\n::\n");
        let invocation = br#"{"stable":"source-metadata"}"#;
        let validated = run_migration_validation(
            input.clone(),
            &original.source_files,
            invocation,
            graph.as_bytes(),
        )
        .expect("snapshot validation");
        assert_eq!(validated.receipt.result(), ValidationResult::Pass);
        let receipt = serde_json::to_value(&validated.receipt).expect("receipt");
        assert_eq!(
            receipt["inputs"],
            serde_json::to_value(&original.receipt).expect("original receipt")["inputs"]
        );
        assert_eq!(receipt["context"][0]["digest"], sha256_prefixed(invocation));
        assert_eq!(
            receipt["context"][1]["digest"],
            sha256_prefixed(graph.as_bytes())
        );
        let mut forged: serde_json::Value = serde_json::from_str(graph).expect("graph");
        let node = forged["nodes"]
            .as_array_mut()
            .expect("nodes")
            .iter_mut()
            .find(|node| node["type"] == "knowledge_object")
            .expect("object");
        node["body"] = "Substituted source meaning.".into();
        let failed = run_migration_validation(
            input,
            &original.source_files,
            invocation,
            &serde_json::to_vec(&forged).expect("forged graph"),
        )
        .expect("failed evidence");
        assert_eq!(failed.receipt.result(), ValidationResult::Fail);
        assert!(failed.diagnostics.iter().any(|diagnostic| diagnostic.code == DiagnosticCode::ValidationContextArtifactDrift));
    }

    /// Compile the workspace and return its graph artifact JSON — the
    /// honest artifact a build of the same source would publish.
    fn compiled_graph_json(root: &Path) -> String {
        let compiled = crate::compile_workspace_for_date(
            crate::CompileInput {
                root: root.to_path_buf(),
            },
            NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
        );
        compiled
            .artifacts
            .expect("clean fixture compiles artifacts")
            .graph_json
    }

    fn graph_v6_schema_accepts(instance_json: &str) -> bool {
        let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/agent/v0/schema/graph-artifact.v6.json");
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(schema_path).expect("schema is readable"))
                .expect("schema is json");
        let instance: serde_json::Value =
            serde_json::from_str(instance_json).expect("artifact is json");
        jsonschema::validator_for(&schema)
            .expect("schema compiles")
            .is_valid(&instance)
    }

    fn valid_source() -> &'static str {
        "# Billing @doc(team.billing)\n\n::claim billing.ready\nstatus: draft\n--\nBilling docs are ready.\n::\n"
    }

    fn semantic_expectations() -> crate::SemanticContextExpectedBindings {
        crate::SemanticContextExpectedBindings {
            subject_revision: crate::ExactRevision {
                system: "git".to_string(),
                value: "head-sha".to_string(),
            },
            source_revision: crate::ExactRevision {
                system: "git".to_string(),
                value: "head-sha".to_string(),
            },
            base_revision: crate::ExactRevision {
                system: "git".to_string(),
                value: "base-sha".to_string(),
            },
            head_revision: crate::ExactRevision {
                system: "git".to_string(),
                value: "head-sha".to_string(),
            },
            assessment_digest: TEST_DIGEST.to_string(),
            selection_algorithm: "changed-only".to_string(),
            selection_version: "1".to_string(),
            context_classes: vec![crate::ContextClass {
                class_id: "changed_knowledge".to_string(),
                requirement: crate::ContextRequirement::Required,
                byte_budget: 4096,
            }],
            authorized_scope: vec!["repo:billing".to_string()],
            capability_policy: crate::CapabilityPolicy {
                version: "semantic-context-policy-v1".to_string(),
                rules: [
                    crate::UnavailabilityReason::Permission,
                    crate::UnavailabilityReason::Retention,
                    crate::UnavailabilityReason::SourceOutage,
                    crate::UnavailabilityReason::Truncation,
                    crate::UnavailabilityReason::ResourceLimit,
                ]
                .into_iter()
                .map(|reason| crate::CapabilityPolicyRule {
                    reason,
                    outcome: crate::UnavailabilityOutcome::Insufficient,
                })
                .collect(),
            },
            graph_object_contexts: vec![crate::GraphObjectContextExpectation {
                object_id: "billing.ready".to_string(),
                class_id: "changed_knowledge".to_string(),
                scope_ref: "repo:billing".to_string(),
            }],
        }
    }

    fn semantic_context_json(
        graph_json: &str,
        knowledge_basis: crate::KnowledgeBasis,
        unavailable_reason: Option<crate::UnavailabilityReason>,
    ) -> String {
        let expectations = semantic_expectations();
        let graph: GraphArtifactDocument = serde_json::from_str(graph_json).expect("graph parses");
        let object = knowledge_objects(&graph)
            .get("billing.ready")
            .copied()
            .expect("fixture object");
        crate::build_semantic_context(crate::SemanticContextInput {
            evaluation_date: NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
            subject_revision: expectations.subject_revision,
            source_revision: expectations.source_revision,
            base_revision: expectations.base_revision,
            head_revision: expectations.head_revision,
            basis: crate::SemanticContextBasis {
                assessment_digest: expectations.assessment_digest,
                knowledge_basis,
            },
            selection: crate::SemanticContextSelection {
                algorithm: "changed-only".to_string(),
                version: "1".to_string(),
                authorized_scope: vec!["repo:billing".to_string()],
            },
            capability_policy: crate::CapabilityPolicy {
                version: "semantic-context-policy-v1".to_string(),
                rules: [
                    crate::UnavailabilityReason::Permission,
                    crate::UnavailabilityReason::Retention,
                    crate::UnavailabilityReason::SourceOutage,
                    crate::UnavailabilityReason::Truncation,
                    crate::UnavailabilityReason::ResourceLimit,
                ]
                .into_iter()
                .map(|reason| crate::CapabilityPolicyRule {
                    reason,
                    outcome: crate::UnavailabilityOutcome::Insufficient,
                })
                .collect(),
            },
            context_classes: vec![crate::ContextClass {
                class_id: "changed_knowledge".to_string(),
                requirement: crate::ContextRequirement::Required,
                byte_budget: 4096,
            }],
            items: unavailable_reason
                .is_none()
                .then(|| crate::SemanticContextItem {
                    handle_id: "billing-ready".to_string(),
                    class_id: "changed_knowledge".to_string(),
                    scope_ref: "repo:billing".to_string(),
                    handle: crate::CitationHandle::KnowledgeObject {
                        object_id: object.id.clone(),
                        semantic_hash: object.content_hash.clone(),
                    },
                    content: serde_json::json!({ "body": object.body }),
                    truncated: false,
                })
                .into_iter()
                .collect(),
            unavailability: unavailable_reason
                .map(|reason| crate::ContextUnavailability {
                    record_id: "unavailable-1".to_string(),
                    class_id: "changed_knowledge".to_string(),
                    kind: crate::ContextUnavailabilityKind::Omission,
                    reason,
                })
                .into_iter()
                .collect(),
        })
        .expect("semantic context builds")
        .to_canonical_json()
        .expect("semantic context serializes")
    }

    fn write_semantic_context(workspace: &Path, contents: &str) -> PathBuf {
        let path = workspace.join("semantic-context.json");
        write(&path, contents);
        path
    }

    /// E1.7.T1 parity discipline (ADR-0015): the serialized receipt
    /// validates against the published JSON Schema — the schema documents
    /// the envelope, it never constructs it.
    #[test]
    fn serialized_receipt_validates_against_published_schema() {
        let workspace = tempfile::tempdir().expect("workspace");
        write(&workspace.path().join("docs/index.adoc"), valid_source());

        let outcome = run_validation_runtime(standalone_input(&workspace.path().join("docs")))
            .expect("validation runs");
        assert_receipt_matches_published_schema(&outcome.receipt.to_canonical_json());
    }

    /// Parity discipline (ADR-0015) helper shared by receipt tests.
    fn assert_receipt_matches_published_schema(receipt_json: &str) {
        let instance: serde_json::Value =
            serde_json::from_str(receipt_json).expect("receipt is json");
        let schema_file = match instance["schema_version"].as_str() {
            Some(VALIDATION_RECEIPT_V0_SCHEMA_VERSION) => "adoc.validation_receipt.v0.schema.json",
            Some(VALIDATION_RECEIPT_V1_SCHEMA_VERSION) => "adoc.validation_receipt.v1.schema.json",
            version => panic!("unexpected validation receipt version: {version:?}"),
        };
        let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/agent/v0/schema")
            .join(schema_file);
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(schema_path).expect("schema is readable"))
                .expect("schema is json");
        let validator = jsonschema::validator_for(&schema).expect("schema compiles");
        let errors = validator
            .iter_errors(&instance)
            .map(|error| error.to_string())
            .collect::<Vec<_>>();
        assert!(
            errors.is_empty(),
            "{schema_file} validation failed:\n{}\ninstance:\n{instance:#}",
            errors.join("\n")
        );
    }

    /// Receipt determinism: same inputs, byte-identical canonical JSON, and
    /// the digest fields bind runtime identity, inputs, and diagnostics.
    #[test]
    fn receipt_is_deterministic_and_digest_bound() {
        let workspace = tempfile::tempdir().expect("workspace");
        write(&workspace.path().join("docs/index.adoc"), valid_source());
        let input = standalone_input(&workspace.path().join("docs"));

        let first = run_validation_runtime(input.clone()).expect("first run");
        let second = run_validation_runtime(input).expect("second run");
        assert_eq!(
            first.receipt.to_canonical_json(),
            second.receipt.to_canonical_json(),
            "receipts must be byte-identical across invocations"
        );

        let value: serde_json::Value =
            serde_json::from_str(&first.receipt.to_canonical_json()).expect("receipt is json");
        assert_eq!(
            value["schema_version"],
            VALIDATION_RECEIPT_V0_SCHEMA_VERSION
        );
        assert_eq!(value["runtime"]["binary_digest"], TEST_DIGEST);
        assert_eq!(value["contract_versions"]["graph"], "adoc.graph.v6");
        assert_eq!(value["result"], "pass");
        assert_eq!(value["inputs"][0]["path"], "index.adoc");
        assert_eq!(
            value["diagnostics_digest"],
            sha256_prefixed(&serde_json::to_vec(&first.diagnostics).expect("serializable")),
            "diagnostics_digest is the sha256 of the canonically serialized diagnostics array"
        );
    }

    /// Domain-invalid source (a dangling reference parses cleanly but
    /// violates domain rules): the runtime fails closed and the receipt
    /// records the failure with a digest over the typed diagnostics.
    #[test]
    fn domain_invalid_source_yields_fail_receipt() {
        let workspace = tempfile::tempdir().expect("workspace");
        write(
            &workspace.path().join("docs/index.adoc"),
            "# Billing @doc(team.billing)\n\n::claim billing.ready\nstatus: draft\ndepends_on: [billing.missing]\n--\nBilling docs are ready.\n::\n",
        );

        let outcome = run_validation_runtime(standalone_input(&workspace.path().join("docs")))
            .expect("validation runs");
        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.severity == Severity::Error),
            "expected typed error diagnostics, got: {:?}",
            outcome.diagnostics
        );
    }

    /// E1.7.T4 baseline: an honest artifact of the same source passes; the
    /// receipt records its digest as context and the consumed graph
    /// contract version.
    #[test]
    fn matching_context_artifact_passes_and_is_digest_bound() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let artifact_path = workspace.path().join("dist/docs.graph.json");
        write(&artifact_path, &compiled_graph_json(&docs));

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(artifact_path);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Pass);
        let value: serde_json::Value =
            serde_json::from_str(&outcome.receipt.to_canonical_json()).expect("receipt is json");
        assert_eq!(value["context"][0]["name"], "context_artifact");
        assert_eq!(value["contract_versions"]["graph"], "adoc.graph.v6");
    }

    #[test]
    fn source_invocation_manifest_is_digest_bound() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let invocation = workspace.path().join("source-invocation.json");
        write(
            &invocation,
            r#"{"source":"agentdoc-dev/adoc","revision":"head-sha"}"#,
        );

        let mut input = standalone_input(&docs);
        input.source_invocation = Some(invocation);
        let outcome = run_validation_runtime(input).expect("validation runs");
        let receipt: serde_json::Value =
            serde_json::from_str(&outcome.receipt.to_canonical_json()).expect("receipt json");

        assert_eq!(receipt["schema_version"], "adoc.validation_receipt.v1");
        assert_eq!(receipt["context"][0]["name"], "source_invocation");
        assert_eq!(
            receipt["context"][0]["digest"],
            sha256_prefixed(r#"{"source":"agentdoc-dev/adoc","revision":"head-sha"}"#.as_bytes())
        );
        assert_receipt_matches_published_schema(&outcome.receipt.to_canonical_json());
    }

    #[test]
    fn published_v0_receipt_schema_rejects_source_invocation_context() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let outcome = run_validation_runtime(standalone_input(&docs)).expect("validation runs");
        let mut receipt: serde_json::Value =
            serde_json::from_str(&outcome.receipt.to_canonical_json()).expect("receipt json");
        receipt["context"] = serde_json::json!([{
            "name": "source_invocation",
            "digest": TEST_DIGEST
        }]);

        let schema_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/agent/v0/schema/adoc.validation_receipt.v0.schema.json");
        let schema: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(schema_path).expect("v0 schema is readable"))
                .expect("v0 schema is json");
        assert!(
            !jsonschema::validator_for(&schema)
                .expect("v0 schema compiles")
                .is_valid(&receipt)
        );
    }

    #[test]
    fn matching_semantic_context_is_validated_and_digest_bound_in_the_receipt() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let artifact_path = workspace.path().join("dist/docs.graph.json");
        write(&artifact_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                None,
            ),
        );

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(artifact_path);
        input.semantic_context = Some(semantic_path);
        input.semantic_context_expectations = Some(semantic_expectations());
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Pass);
        let receipt: serde_json::Value =
            serde_json::from_str(&outcome.receipt.to_canonical_json()).expect("receipt json");
        assert_eq!(
            receipt["contract_versions"]["semantic_context"],
            crate::SEMANTIC_CONTEXT_SCHEMA_VERSION
        );
        assert_eq!(receipt["context"][1]["name"], "semantic_context");
        assert_receipt_matches_published_schema(&outcome.receipt.to_canonical_json());
    }

    #[test]
    fn managed_revision_semantic_context_fails_closed_in_receipt_mode() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let graph_path = workspace.path().join("dist/docs.graph.json");
        write(&graph_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::ManagedRevision {
                    digest: TEST_DIGEST.to_string(),
                },
                None,
            ),
        );

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(graph_path);
        input.semantic_context = Some(semantic_path);
        input.semantic_context_expectations = Some(semantic_expectations());
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::SemanticContextBasisMismatch
            })
        );
    }

    #[test]
    fn graph_semantic_context_without_context_artifact_fails_closed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                None,
            ),
        );

        let mut input = standalone_input(&docs);
        input.semantic_context = Some(semantic_path);
        input.semantic_context_expectations = Some(semantic_expectations());
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::SemanticContextBasisMismatch
            })
        );
    }

    #[test]
    fn semantic_context_without_trusted_expectations_fails_closed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let graph_path = workspace.path().join("dist/docs.graph.json");
        write(&graph_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                None,
            ),
        );

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(graph_path);
        input.semantic_context = Some(semantic_path);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(outcome.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SemanticContextBasisMismatch
                && diagnostic
                    .message
                    .contains("expectations were not supplied")
        }));
    }

    #[test]
    fn semantic_context_outcome_failure_is_receipted_with_its_contract_version() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let graph_path = workspace.path().join("dist/docs.graph.json");
        write(&graph_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                Some(crate::UnavailabilityReason::Permission),
            ),
        );

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(graph_path);
        input.semantic_context = Some(semantic_path);
        let mut expectations = semantic_expectations();
        expectations.graph_object_contexts.clear();
        input.semantic_context_expectations = Some(expectations);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(outcome.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SemanticContextInsufficientContext
        }));
        let canonical = outcome.receipt.to_canonical_json();
        let receipt: serde_json::Value = serde_json::from_str(&canonical).expect("receipt json");
        assert_eq!(
            receipt["contract_versions"]["semantic_context"],
            crate::SEMANTIC_CONTEXT_SCHEMA_VERSION
        );
        assert_receipt_matches_published_schema(&canonical);
    }

    #[test]
    fn mismatched_semantic_context_revision_fails_the_receipt() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let graph_path = workspace.path().join("dist/docs.graph.json");
        write(&graph_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                None,
            ),
        );
        let mut expectations = semantic_expectations();
        expectations.head_revision.value = "other-head".to_string();

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(graph_path);
        input.semantic_context = Some(semantic_path);
        input.semantic_context_expectations = Some(expectations);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(outcome.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SemanticContextBasisMismatch
                && diagnostic.message.contains("head revision differs")
        }));
    }

    #[test]
    fn mismatched_semantic_context_class_fails_the_receipt() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let graph_json = compiled_graph_json(&docs);
        let graph_path = workspace.path().join("dist/docs.graph.json");
        write(&graph_path, &graph_json);
        let semantic_path = write_semantic_context(
            workspace.path(),
            &semantic_context_json(
                &graph_json,
                crate::KnowledgeBasis::GraphArtifact {
                    digest: sha256_prefixed(graph_json.as_bytes()),
                },
                None,
            ),
        );
        let mut expectations = semantic_expectations();
        expectations.graph_object_contexts[0].class_id = "related_knowledge".to_string();

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(graph_path);
        input.semantic_context = Some(semantic_path);
        input.semantic_context_expectations = Some(expectations);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(outcome.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == DiagnosticCode::SemanticContextBasisMismatch
                && diagnostic
                    .message
                    .contains("class for handle 'billing-ready'")
        }));
    }

    /// E1.7.T4 stop-ship cut: a context artifact the published JSON Schema
    /// ACCEPTS but whose governed-meaning content_hash does not correspond
    /// to the source is rejected by the runtime with typed diagnostics,
    /// and the receipt records the failure. JSON Schema stays
    /// preflight/documentation only (ADR-0015): schema validity alone can
    /// never make a payload domain-valid, and a preflight that accepted
    /// this fixture would be overruled by this runtime.
    #[test]
    fn schema_valid_domain_invalid_context_artifact_fails_closed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let honest = compiled_graph_json(&docs);
        let honest_value: serde_json::Value = serde_json::from_str(&honest).expect("json");
        let real_hash = honest_value["nodes"]
            .as_array()
            .expect("nodes array")
            .iter()
            .find_map(|node| {
                (node["type"] == "knowledge_object").then(|| {
                    node["content_hash"]
                        .as_str()
                        .expect("content_hash string")
                        .to_string()
                })
            })
            .expect("fixture has a knowledge object");
        let forged_hash = format!("sha256:{}", "0".repeat(64));
        let forged = honest.replace(&real_hash, &forged_hash);
        assert!(
            graph_v6_schema_accepts(&forged),
            "the forged artifact must stay JSON-Schema-valid — the cut proves schema \
             validity is not domain validity"
        );
        let artifact_path = workspace.path().join("dist/docs.graph.json");
        write(&artifact_path, &forged);

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(artifact_path);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ValidationContextArtifactDrift
                    && diagnostic.severity == Severity::Error
            }),
            "expected validation.context_artifact_drift, got: {:?}",
            outcome.diagnostics
        );
        // The fail receipt is still a valid envelope of the published
        // receipt schema (parity discipline covers both results).
        let canonical = outcome.receipt.to_canonical_json();
        assert_receipt_matches_published_schema(&canonical);
        let receipt_value: serde_json::Value =
            serde_json::from_str(&canonical).expect("receipt is json");
        assert_eq!(receipt_value["result"], "fail");
    }

    /// E1.7.T4 adversarial hardening: a forgery that tampers a governed
    /// field but REPLAYS the honest content_hash string must still fail
    /// closed — the drift check may never reduce to comparing the
    /// producer's own hash claims against themselves.
    #[test]
    fn hash_preserving_governed_field_forgery_fails_closed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let honest = compiled_graph_json(&docs);
        let mut value: serde_json::Value = serde_json::from_str(&honest).expect("json");
        let node = value["nodes"]
            .as_array_mut()
            .expect("nodes array")
            .iter_mut()
            .find(|node| node["type"] == "knowledge_object")
            .expect("fixture has a knowledge object");
        assert_eq!(node["status"], "draft", "fixture authors status: draft");
        node["status"] = serde_json::Value::from("approved");
        let forged = serde_json::to_string(&value).expect("serializes");
        assert!(
            graph_v6_schema_accepts(&forged),
            "the forged artifact must stay JSON-Schema-valid"
        );
        let artifact_path = workspace.path().join("dist/docs.graph.json");
        write(&artifact_path, &forged);

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(artifact_path);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ValidationContextArtifactDrift
                    && diagnostic.severity == Severity::Error
            }),
            "a status forgery with an untouched content_hash must drift-fail, got: {:?}",
            outcome.diagnostics
        );
    }

    /// E1.7.T4 adversarial hardening: prose nodes and edges are governed
    /// carriage too — a forgery outside the Knowledge Object set must not
    /// pass just because every Knowledge Object matches.
    #[test]
    fn prose_node_forgery_fails_closed() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let honest = compiled_graph_json(&docs);
        let mut value: serde_json::Value = serde_json::from_str(&honest).expect("json");
        let node = value["nodes"]
            .as_array_mut()
            .expect("nodes array")
            .iter_mut()
            .find(|node| node["type"] == "page")
            .expect("fixture has a page node");
        node["title"] = serde_json::Value::from("Refunds");
        let forged = serde_json::to_string(&value).expect("serializes");
        assert!(
            graph_v6_schema_accepts(&forged),
            "the forged artifact must stay JSON-Schema-valid"
        );
        let artifact_path = workspace.path().join("dist/docs.graph.json");
        write(&artifact_path, &forged);

        let mut input = standalone_input(&docs);
        input.context_artifact = Some(artifact_path);
        let outcome = run_validation_runtime(input).expect("validation runs");

        assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
        assert!(
            outcome.diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::ValidationContextArtifactDrift
                    && diagnostic.severity == Severity::Error
            }),
            "a prose-node forgery must drift-fail, got: {:?}",
            outcome.diagnostics
        );
    }

    /// E1.7.T4: unknown envelope versions reject exact-match — an older
    /// v5 and an unknown v99 alike — with `schema.unsupported_version`;
    /// the receipt records the failure and no drift comparison runs.
    #[test]
    fn unknown_context_artifact_version_rejects_exact_match() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let honest = compiled_graph_json(&docs);

        for version in ["adoc.graph.v5", "adoc.graph.v99"] {
            let artifact_path = workspace.path().join("dist/docs.graph.json");
            write(&artifact_path, &honest.replace("adoc.graph.v6", version));

            let mut input = standalone_input(&docs);
            input.context_artifact = Some(artifact_path);
            let outcome = run_validation_runtime(input).expect("validation runs");

            assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
            assert!(
                outcome
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code == DiagnosticCode::SchemaUnsupportedVersion),
                "{version}: expected schema.unsupported_version, got: {:?}",
                outcome.diagnostics
            );
            assert!(
                !outcome
                    .diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.code
                        == DiagnosticCode::ValidationContextArtifactDrift),
                "{version}: version gate must reject before any drift comparison"
            );
        }
    }

    /// The recompiled-graph parse invariant spans two modules (compile.rs
    /// serialization, graph/mod.rs deserialization); if a future serde
    /// asymmetry breaks it, the receipt path must fail CLOSED with a typed
    /// diagnostic — never panic in the one path whose contract is typed
    /// failure.
    #[test]
    fn malformed_recompiled_graph_json_degrades_to_a_typed_diagnostic() {
        let workspace = tempfile::tempdir().expect("workspace");
        let docs = workspace.path().join("docs");
        write(&docs.join("index.adoc"), valid_source());
        let honest = compiled_graph_json(&docs);

        let diagnostics = validate_context_artifact(
            Path::new("dist/docs.graph.json"),
            honest.as_bytes(),
            Some("not json"),
        );

        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == DiagnosticCode::IoArtifactMalformed
                    && diagnostic.severity == Severity::Error
            }),
            "a malformed recompiled graph must fail closed with io.artifact_malformed, got: {diagnostics:?}"
        );
    }

    /// Fail-receipt portability: the unqualified determinism claim (module
    /// docs, registry row) covers FAIL receipts too, so the same forgery
    /// validated at two different absolute locations must produce
    /// byte-identical receipts — no machine-specific path may leak into
    /// `diagnostics_digest`.
    #[test]
    fn fail_receipts_are_byte_identical_across_artifact_locations() {
        let receipt_at_fresh_location = || {
            let workspace = tempfile::tempdir().expect("workspace");
            let docs = workspace.path().join("docs");
            write(&docs.join("index.adoc"), valid_source());
            let mut value: serde_json::Value =
                serde_json::from_str(&compiled_graph_json(&docs)).expect("json");
            value["nodes"]
                .as_array_mut()
                .expect("nodes array")
                .iter_mut()
                .find(|node| node["type"] == "knowledge_object")
                .expect("fixture has a knowledge object")["status"] =
                serde_json::Value::from("approved");
            let artifact_path = workspace.path().join("dist/docs.graph.json");
            write(
                &artifact_path,
                &serde_json::to_string(&value).expect("serializes"),
            );

            let mut input = standalone_input(&docs);
            input.context_artifact = Some(artifact_path);
            let outcome = run_validation_runtime(input).expect("validation runs");
            assert_eq!(outcome.receipt.result(), ValidationResult::Fail);
            outcome.receipt.to_canonical_json()
        };

        assert_eq!(
            receipt_at_fresh_location(),
            receipt_at_fresh_location(),
            "the same forgery at two absolute locations must yield byte-identical fail receipts"
        );
    }

    /// A provider whose second read observes different content — the
    /// editor-save / concurrent-checkout race between "digest" and
    /// "validate". The runtime must read exactly once, so the flip is
    /// never observed.
    struct FlipFlopProvider {
        first: &'static str,
        second: &'static str,
        calls: std::cell::Cell<usize>,
    }

    impl SourceProvider for FlipFlopProvider {
        fn load_sources(&self) -> Vec<Result<SourceFile, SourceLoadError>> {
            let call = self.calls.get();
            self.calls.set(call + 1);
            let text = if call == 0 { self.first } else { self.second };
            vec![Ok(SourceFile::new_with_identity_path(
                PathBuf::from("index.adoc"),
                text.to_string(),
                PathBuf::from("index.adoc"),
            ))]
        }

        fn contains(&self, _path: &Path) -> bool {
            true
        }
    }

    /// Exact-input binding (E1.7 exit gate): the digests in `inputs` and
    /// the validated content come from the SAME read. A source that
    /// changes between two hypothetical reads must not produce a receipt
    /// attesting digests that were never validated — the runtime reads
    /// once, so the flipped content is never observed.
    #[test]
    fn receipt_digests_bind_the_exact_content_that_was_validated() {
        let workspace = tempfile::tempdir().expect("workspace");
        let provider = FlipFlopProvider {
            first: valid_source(),
            second: "# Billing @doc(team.billing)\n\n::claim billing.ready\nstatus: draft\ndepends_on: [billing.missing]\n--\nBilling docs are ready.\n::\n",
            calls: std::cell::Cell::new(0),
        };
        let outcome = run_with_provider(&provider, standalone_input(workspace.path()))
            .expect("validation runs");

        assert_eq!(
            outcome.receipt.result(),
            ValidationResult::Pass,
            "the validated content must be the digested content, got: {:?}",
            outcome.diagnostics
        );
        let value: serde_json::Value =
            serde_json::from_str(&outcome.receipt.to_canonical_json()).expect("receipt is json");
        assert_eq!(
            value["inputs"][0]["digest"],
            sha256_prefixed(valid_source().as_bytes()),
            "inputs must digest the exact bytes the run validated"
        );
    }

    /// The harness-attested digest is validated at the boundary: anything
    /// but `sha256:` + 64 lowercase hex refuses before any validation runs.
    #[test]
    fn malformed_runtime_binary_digest_is_refused() {
        let workspace = tempfile::tempdir().expect("workspace");
        write(&workspace.path().join("docs/index.adoc"), valid_source());
        for bad in [
            "",
            "sha256:",
            "sha256:short",
            "sha256:CA1BF018DC0B72EE1197D9D521D96D227CD3E54CC81528EA5F45776C99D95F4D",
            "md5:ca1bf018dc0b72ee1197d9d521d96d227cd3e54cc81528ea5f45776c99d95f4d",
        ] {
            let mut input = standalone_input(&workspace.path().join("docs"));
            input.runtime_binary_digest = bad.to_string();
            assert!(
                matches!(
                    run_validation_runtime(input),
                    Err(ValidationRuntimeError::InvalidRuntimeBinaryDigest { .. })
                ),
                "digest {bad:?} must refuse"
            );
        }
    }
}
