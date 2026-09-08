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
    let outcome = run_validation_runtime(target.runtime_input(
        snapshot.path(),
        &request,
        runtime_version,
        runtime_binary_digest,
    )?)
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
    pub config_bytes: String,
}
impl MigrationValidationTarget {
    fn runtime_input(
        &self,
        snapshot: &Path,
        request: &MigrationRequest,
        runtime_version: String,
        runtime_binary_digest: String,
    ) -> Result<ValidationRuntimeInput, MigrationError> {
        Ok(ValidationRuntimeInput {
            root: self.root.clone(),
            project: self.project.clone(),
            anchor_root: snapshot.to_path_buf(),
            evaluation_date: request.date()?,
            runtime_version,
            runtime_binary_digest,
            config_path: self.config_path.clone(),
            source_invocation: None,
            context_artifact: None,
            semantic_context: None,
            semantic_context_expectations: None,
        })
    }
}

use crate::domain::migration::{
    MIGRATION_IMPORT_MAX_BYTES, MIGRATION_IMPORT_SCHEMA_VERSION,
    MIGRATION_VALIDATION_INVOCATION_SCHEMA_VERSION, MigrationImportJob,
};

/// Candidate input evidence only; no field grants activation or promotion authority.
#[derive(Debug, Serialize)]
pub struct MigrationImportBundle {
    schema_version: &'static str,
    request: MigrationRequest,
    request_digest: String,
    job_digest: String,
    config_bytes: String,
    graph_artifact_bytes: String,
    sources: Vec<MigrationImportedSource>,
}
#[derive(Debug, Serialize)]
struct MigrationImportedSource {
    path: String,
    source_bytes: String,
    source_record_bytes: String,
    source_binding_bytes: String,
    source_invocation_bytes: String,
    validation_receipt_bytes: String,
}
#[derive(Serialize)]
struct MigrationValidationInvocation<'a> {
    schema_version: &'static str,
    workspace_id: &'a str,
    source_record_id: &'a str,
    source_record_digest: String,
    source_binding_id: &'a str,
    source_binding_digest: String,
    source_acl_snapshot_id: &'a str,
    config_digest: String,
    evaluation_date: &'a str,
    source_path: &'a str,
    request_digest: &'a str,
}
impl MigrationImportBundle {
    pub fn to_canonical_json(&self) -> Result<String, MigrationError> {
        use std::io::Write;
        let mut output = MigrationOutput(Vec::new());
        serde_json::to_writer(&mut output, self).map_err(|_| MigrationError::OutputLimit)?;
        output
            .write_all(b"\n")
            .map_err(|_| MigrationError::OutputLimit)?;
        String::from_utf8(output.0).map_err(|_| MigrationError::ValidationUnavailable)
    }
}
struct MigrationOutput(Vec<u8>);
impl std::io::Write for MigrationOutput {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > MIGRATION_IMPORT_MAX_BYTES.saturating_sub(self.0.len()) {
            return Err(std::io::Error::other("migration output limit"));
        }
        self.0.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

pub(crate) fn import_with_provider(
    request_bytes: &[u8],
    job_bytes: &[u8],
    provider: &impl SnapshotWorkspaceProvider,
    resolve: impl FnOnce(&Path) -> Result<MigrationValidationTarget, MigrationError>,
    runtime_version: String,
    runtime_binary_digest: String,
) -> Result<MigrationImportBundle, MigrationError> {
    use crate::domain::source_provenance::{
        SourceBindingCoordinates, SourceBindingInput, build_source_binding,
    };
    use crate::domain::source_record::{
        RetentionClass, SourceArtifact, SourceRecordInput, build_source_record,
    };
    use std::collections::BTreeMap;
    let request = MigrationRequest::parse(request_bytes)?;
    let job = MigrationImportJob::parse(job_bytes, &request)?;
    let snapshot = provider
        .checkout(&SnapshotSelector::GitRef(GitRef::new(
            &request.revision.value,
        )))
        .map_err(|_| MigrationError::SnapshotUnavailable)?;
    let target = resolve(snapshot.path())?;
    let input = target.runtime_input(
        snapshot.path(),
        &request,
        runtime_version,
        runtime_binary_digest,
    )?;
    let validated =
        run_validation_runtime(input.clone()).map_err(|_| MigrationError::ValidationUnavailable)?;
    if validated.receipt.result() != ValidationResult::Pass {
        return Err(MigrationError::ValidationFailed);
    }
    let graph = validated
        .graph_artifact
        .ok_or(MigrationError::ValidationFailed)?;
    let metadata: BTreeMap<_, _> = job
        .sources
        .iter()
        .map(|source| (source.path.as_str(), source))
        .collect();
    if metadata.len() != validated.source_files.len() {
        return Err(MigrationError::InvalidJob);
    }
    let request_digest = sha256_prefixed(request_bytes);
    let observed_at = chrono::DateTime::parse_from_rfc3339(&job.observed_at)
        .map_err(|_| MigrationError::InvalidJob)?
        .with_timezone(&chrono::Utc);
    let mut bundle_bytes = graph.len().saturating_add(target.config_bytes.len());
    if bundle_bytes > MIGRATION_IMPORT_MAX_BYTES {
        return Err(MigrationError::OutputLimit);
    }
    let mut sources = Vec::new();
    for source in &validated.source_files {
        let path = source
            .logical_path
            .to_str()
            .ok_or(MigrationError::UnsafeSource)?;
        let metadata = metadata.get(path).ok_or(MigrationError::InvalidJob)?;
        let record = build_source_record(SourceRecordInput {
            source_record_id: metadata.source_record_id.clone(),
            workspace_id: request.workspace_id.clone(),
            connector_id: job.connector_id.clone(),
            source: SourceArtifact {
                provider: "git".into(),
                kind: "file".into(),
                external_id: path.into(),
                external_version: request.revision.value.clone(),
            },
            source_acl_scope: job.source_acl_scope.clone(),
            observed_at,
            media_type: "text/plain".into(),
            retention_class: RetentionClass::ExactCandidateInput,
            exact_bytes: source.text.as_bytes(),
        })
        .map_err(|_| MigrationError::InvalidJob)?;
        let binding = build_source_binding(SourceBindingInput {
            source_binding_id: metadata.source_binding_id.clone(),
            workspace_id: request.workspace_id.clone(),
            source_record_id: metadata.source_record_id.clone(),
            coordinates: SourceBindingCoordinates {
                connector: "git".into(),
                source: path.into(),
                revision: Some(request.revision.value.clone()),
                path: path.into(),
                anchor: "document".into(),
                source_revision_digest: record.content_digest().into(),
            },
        })
        .map_err(|_| MigrationError::InvalidJob)?;
        let source_record_bytes = record
            .to_canonical_json()
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        let source_binding_bytes = binding
            .to_canonical_json()
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        let invocation = MigrationValidationInvocation {
            schema_version: MIGRATION_VALIDATION_INVOCATION_SCHEMA_VERSION,
            workspace_id: &request.workspace_id,
            source_record_id: &metadata.source_record_id,
            source_record_digest: sha256_prefixed(source_record_bytes.as_bytes()),
            source_binding_id: &metadata.source_binding_id,
            source_binding_digest: sha256_prefixed(source_binding_bytes.as_bytes()),
            source_acl_snapshot_id: &job.source_acl_scope.snapshot_id,
            config_digest: sha256_prefixed(target.config_bytes.as_bytes()),
            evaluation_date: &request.evaluation_date,
            source_path: path,
            request_digest: &request_digest,
        };
        let source_invocation_bytes = serde_json::to_string(&invocation)
            .map_err(|_| MigrationError::ValidationUnavailable)?;
        // ponytail: one full validation per source (maximum 512); reuse a validated
        // compiler snapshot if larger imports require measured throughput improvements.
        let validation = super::validation_runtime::run_migration_validation(
            input.clone(),
            &validated.source_files,
            source_invocation_bytes.as_bytes(),
            graph.as_bytes(),
        )
        .map_err(|_| MigrationError::ValidationUnavailable)?;
        if validation.receipt.result() != ValidationResult::Pass {
            return Err(MigrationError::ValidationFailed);
        }
        let validation_receipt_bytes = validation.receipt.to_canonical_json();
        for bytes in [
            &source.text,
            &source_record_bytes,
            &source_binding_bytes,
            &source_invocation_bytes,
            &validation_receipt_bytes,
        ] {
            bundle_bytes = bundle_bytes.saturating_add(bytes.len());
        }
        if bundle_bytes > MIGRATION_IMPORT_MAX_BYTES {
            return Err(MigrationError::OutputLimit);
        }
        sources.push(MigrationImportedSource {
            path: path.into(),
            source_bytes: source.text.clone(),
            source_record_bytes,
            source_binding_bytes,
            source_invocation_bytes,
            validation_receipt_bytes,
        });
    }
    Ok(MigrationImportBundle {
        schema_version: MIGRATION_IMPORT_SCHEMA_VERSION,
        request,
        request_digest,
        job_digest: sha256_prefixed(job_bytes),
        config_bytes: target.config_bytes,
        graph_artifact_bytes: graph,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    #[test]
    fn migration_output_limits_serialized_bytes_including_json_escaping() {
        let mut output = MigrationOutput(Vec::new());
        let text = "\"".repeat(MIGRATION_IMPORT_MAX_BYTES / 2);
        assert!(serde_json::to_writer(&mut output, &text).is_err());
        assert!(output.0.len() <= MIGRATION_IMPORT_MAX_BYTES);
        let mut output = MigrationOutput(vec![b' '; MIGRATION_IMPORT_MAX_BYTES - 1]);
        assert!(output.write_all(b"x").is_ok());
        assert!(output.write_all(b"x").is_err());
        assert_eq!(output.0.len(), MIGRATION_IMPORT_MAX_BYTES);
    }
}
