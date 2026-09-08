//! Exact-snapshot preparation composes the authoritative Validation Runtime.
use super::validation_runtime::{
    ValidationReceipt, ValidationResult, ValidationRuntimeInput, run_validation_runtime,
};
use crate::domain::{
    diagnostic::Diagnostic,
    hashing::sha256_prefixed,
    migration::{MIGRATION_RECEIPT_SCHEMA_VERSION, MigrationError, MigrationRequest},
    ports::snapshot_workspace::{GitRef, SnapshotSelector, SnapshotWorkspaceProvider},
};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// Validator-only output; deliberately no Deserialize or public field construction.
#[derive(Debug, Serialize)]
pub struct MigrationReceipt {
    schema_version: &'static str,
    phase: &'static str,
    request: MigrationRequest,
    request_digest: String,
    validation_receipt: ValidationReceipt,
    diagnostics: Vec<Diagnostic>,
}
impl MigrationReceipt {
    pub fn result(&self) -> ValidationResult {
        self.validation_receipt.result()
    }
    pub fn to_canonical_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self).map(|json| json + "\n")
    }
}
/// The composition root privately resolves committed config; validation runs here.
pub(crate) fn prepare_with_provider(
    bytes: &[u8],
    provider: &impl SnapshotWorkspaceProvider,
    resolve: impl FnOnce(&Path) -> Result<MigrationValidationTarget, MigrationError>,
    runtime_version: String,
    runtime_binary_digest: String,
) -> Result<MigrationReceipt, MigrationError> {
    let request = MigrationRequest::parse(bytes)?;
    let snapshot = provider
        .checkout(&SnapshotSelector::GitRef(GitRef::new(
            &request.revision.value,
        )))
        .map_err(|_| MigrationError::SnapshotUnavailable)?;
    let target = resolve(snapshot.path())?;
    let outcome = run_validation_runtime(ValidationRuntimeInput {
        root: target.root,
        project: target.project,
        anchor_root: snapshot.path().to_path_buf(),
        evaluation_date: request.date()?,
        runtime_version,
        runtime_binary_digest,
        config_path: target.config_path,
        source_invocation: None,
        context_artifact: None,
        semantic_context: None,
        semantic_context_expectations: None,
    })
    .map_err(|_| MigrationError::ValidationUnavailable)?;
    Ok(MigrationReceipt {
        schema_version: MIGRATION_RECEIPT_SCHEMA_VERSION,
        phase: "prepare",
        request,
        request_digest: sha256_prefixed(bytes),
        validation_receipt: outcome.receipt,
        diagnostics: outcome.diagnostics,
    })
}
#[derive(Debug)]
pub(crate) struct MigrationValidationTarget {
    pub root: PathBuf,
    pub project: Option<super::compile::LocalProjectContext>,
    pub config_path: Option<PathBuf>,
}
