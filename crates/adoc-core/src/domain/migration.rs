//! Portable preparation inputs. Preparation grants no import or promotion authority.
use super::diagnostic::DiagnosticCode;
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const MIGRATION_REQUEST_SCHEMA_VERSION: &str = "adoc.migration_request.v0";
pub const MIGRATION_RECEIPT_SCHEMA_VERSION: &str = "adoc.migration_receipt.v0";
pub const MIGRATION_REQUEST_MAX_BYTES: usize = 16_384;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MigrationRequest {
    pub(crate) schema_version: String,
    pub(crate) request_id: String,
    pub(crate) workspace_id: String,
    pub(crate) source_id: String,
    pub(crate) repository_identity: String,
    pub(crate) revision: MigrationRevision,
    pub(crate) evaluation_date: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationRevision {
    pub(crate) system: String,
    pub(crate) value: String,
}
#[derive(Debug, Error)]
pub enum MigrationError {
    #[error("migration import job is invalid")]
    InvalidJob,
    #[error("migration source validation failed")]
    ValidationFailed,
    #[error("migration import exceeds its output limit")]
    OutputLimit,
    #[error("migration request is invalid")]
    InvalidRequest,
    #[error("migration requires an exact nonzero lowercase Git commit SHA-1")]
    ExactRevisionRequired,
    #[error("migration snapshot is unavailable")]
    SnapshotUnavailable,
    #[error("migration source cannot be safely materialized")]
    UnsafeSource,
    #[error("migration validation is unavailable")]
    ValidationUnavailable,
}
impl MigrationError {
    pub fn diagnostic_code(&self) -> DiagnosticCode {
        match self {
            Self::InvalidJob => DiagnosticCode::MigrationInvalidJob,
            Self::ValidationFailed => DiagnosticCode::MigrationValidationFailed,
            Self::OutputLimit => DiagnosticCode::MigrationOutputLimit,
            Self::InvalidRequest => DiagnosticCode::MigrationInvalidRequest,
            Self::ExactRevisionRequired => DiagnosticCode::MigrationExactRevisionRequired,
            Self::SnapshotUnavailable => DiagnosticCode::MigrationSnapshotUnavailable,
            Self::UnsafeSource => DiagnosticCode::MigrationUnsafeSource,
            Self::ValidationUnavailable => DiagnosticCode::MigrationValidationUnavailable,
        }
    }
}
impl MigrationRequest {
    pub fn parse(bytes: &[u8]) -> Result<Self, MigrationError> {
        if bytes.len() > MIGRATION_REQUEST_MAX_BYTES {
            return Err(MigrationError::InvalidRequest);
        }
        let value: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| MigrationError::InvalidRequest)?;
        if value
            .get("schema_version")
            .and_then(serde_json::Value::as_str)
            != Some(MIGRATION_REQUEST_SCHEMA_VERSION)
        {
            return Err(MigrationError::InvalidRequest);
        }
        let revision = &value["revision"];
        if revision["system"].as_str() != Some("git")
            || !revision["value"].as_str().is_some_and(is_exact_revision)
        {
            return Err(MigrationError::ExactRevisionRequired);
        }
        // Deserialize from original bytes to reject duplicate struct fields too.
        let request: Self =
            serde_json::from_slice(bytes).map_err(|_| MigrationError::InvalidRequest)?;
        for text in [
            &request.request_id,
            &request.workspace_id,
            &request.source_id,
            &request.repository_identity,
        ] {
            if text.is_empty()
                || text.len() > 1024
                || text.trim() != text
                || text.chars().any(char::is_control)
            {
                return Err(MigrationError::InvalidRequest);
            }
        }
        request.date()?;
        Ok(request)
    }
    pub(crate) fn date(&self) -> Result<NaiveDate, MigrationError> {
        if self.evaluation_date.len() != 10 {
            return Err(MigrationError::InvalidRequest);
        }
        let date = NaiveDate::parse_from_str(&self.evaluation_date, "%Y-%m-%d")
            .map_err(|_| MigrationError::InvalidRequest)?;
        if date.format("%Y-%m-%d").to_string() != self.evaluation_date {
            return Err(MigrationError::InvalidRequest);
        }
        Ok(date)
    }
}
fn is_exact_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && value.bytes().any(|b| b != b'0')
}
pub const MIGRATION_IMPORT_JOB_SCHEMA_VERSION: &str = "agentdoc.cloud.migration_import_job.v0";
pub const MIGRATION_IMPORT_SCHEMA_VERSION: &str = "adoc.migration_import.v0";
pub const MIGRATION_VALIDATION_INVOCATION_SCHEMA_VERSION: &str =
    "agentdoc.cloud.migration_validation_invocation.v0";
pub const MIGRATION_IMPORT_JOB_MAX_BYTES: usize = 512 * 1024;
pub const MIGRATION_IMPORT_MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationImportJob {
    pub schema_version: String,
    pub connector_id: String,
    pub observed_at: String,
    pub source_acl_scope: super::source_record::SourceAclScope,
    pub sources: Vec<MigrationImportSource>,
}
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MigrationImportSource {
    pub path: String,
    pub source_record_id: String,
    pub source_binding_id: String,
}
impl MigrationImportJob {
    pub fn parse(bytes: &[u8], request: &MigrationRequest) -> Result<Self, MigrationError> {
        use super::{source::LogicalPath, source_record::SourceAclResourceKind};
        use chrono::{DateTime, SecondsFormat, Utc};
        use std::collections::BTreeSet;
        if bytes.len() > MIGRATION_IMPORT_JOB_MAX_BYTES {
            return Err(MigrationError::InvalidJob);
        }
        let shape: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| MigrationError::InvalidJob)?;
        if !shape.is_object()
            || !shape["source_acl_scope"].is_object()
            || !shape["source_acl_scope"]["source"].is_object()
            || !shape["sources"]
                .as_array()
                .is_some_and(|sources| sources.iter().all(serde_json::Value::is_object))
        {
            return Err(MigrationError::InvalidJob);
        }
        let job: Self = serde_json::from_slice(bytes).map_err(|_| MigrationError::InvalidJob)?;
        if job.schema_version != MIGRATION_IMPORT_JOB_SCHEMA_VERSION
            || job.sources.is_empty()
            || job.sources.len() > 512
            || job.source_acl_scope.source.kind != SourceAclResourceKind::Repository
            || job.source_acl_scope.source.id != request.repository_identity
            || job.source_acl_scope.source_container_id != request.source_id
        {
            return Err(MigrationError::InvalidJob);
        }
        let observed_at = DateTime::parse_from_rfc3339(&job.observed_at)
            .map_err(|_| MigrationError::InvalidJob)?
            .with_timezone(&Utc);
        if job.observed_at.len() != 20
            || observed_at.to_rfc3339_opts(SecondsFormat::Secs, true) != job.observed_at
        {
            return Err(MigrationError::InvalidJob);
        }
        let valid_text = |value: &str| {
            !value.is_empty()
                && value.len() <= 1024
                && value.trim() == value
                && !value.chars().any(char::is_control)
        };
        if !valid_text(&job.connector_id) || !valid_text(&job.source_acl_scope.snapshot_id) {
            return Err(MigrationError::InvalidJob);
        }
        let (mut paths, mut records, mut bindings) =
            (BTreeSet::new(), BTreeSet::new(), BTreeSet::new());
        for source in &job.sources {
            if !valid_text(&source.path)
                || LogicalPath::parse(&source.path).is_err()
                || !valid_text(&source.source_record_id)
                || !valid_text(&source.source_binding_id)
                || !paths.insert(&source.path)
                || !records.insert(&source.source_record_id)
                || !bindings.insert(&source.source_binding_id)
            {
                return Err(MigrationError::InvalidJob);
            }
        }
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn migration_import_job_is_closed_bounded_and_source_bound() {
        use serde_json::json;
        let request = MigrationRequest::parse(&serde_json::to_vec(&json!({"schema_version":MIGRATION_REQUEST_SCHEMA_VERSION,"request_id":"r","workspace_id":"w","source_id":"s","repository_identity":"repo","revision":{"system":"git","value":"a".repeat(40)},"evaluation_date":"2026-09-08"})).unwrap()).unwrap();
        let job = json!({"schema_version":MIGRATION_IMPORT_JOB_SCHEMA_VERSION,"connector_id":"connector","observed_at":"2026-09-08T12:00:00Z","source_acl_scope":{"snapshot_id":"acl","source_container_id":"s","source":{"kind":"repository","id":"repo"}},"sources":[{"path":"docs/file.adoc","source_record_id":"r","source_binding_id":"b"}]});
        assert!(MigrationImportJob::parse(&serde_json::to_vec(&job).unwrap(), &request).is_ok());
        for (pointer, value) in [
            ("/schema_version", json!("unknown")),
            ("/connector_id", json!(" ")),
            ("/observed_at", json!("2026-09-08T12:00:00.1Z")),
            ("/observed_at", json!("2026-09-08T12:00:00+00:00")),
            ("/source_acl_scope/source_container_id", json!("other")),
            ("/source_acl_scope/source/id", json!("other")),
            ("/source_acl_scope/source/kind", json!("project")),
            ("/sources", json!([])),
            ("/sources/0/path", json!("../escape.adoc")),
            ("/sources/0/path", json!("/absolute.adoc")),
            ("/sources/0/path", json!("C:\\escape.adoc")),
            ("/sources/0", json!(["docs/file.adoc", "r", "b"])),
            ("/sources/0/source_record_id", json!("x".repeat(1025))),
        ] {
            let mut invalid = job.clone();
            *invalid.pointer_mut(pointer).unwrap() = value;
            assert!(
                MigrationImportJob::parse(&serde_json::to_vec(&invalid).unwrap(), &request)
                    .is_err(),
                "{pointer}"
            );
        }
        let mut extra = job.clone();
        extra["foreign"] = json!(true);
        assert!(MigrationImportJob::parse(&serde_json::to_vec(&extra).unwrap(), &request).is_err());
        let mut duplicate = job.clone();
        duplicate["sources"]
            .as_array_mut()
            .unwrap()
            .push(job["sources"][0].clone());
        assert!(
            MigrationImportJob::parse(&serde_json::to_vec(&duplicate).unwrap(), &request).is_err()
        );
        assert!(
            MigrationImportJob::parse(&vec![b' '; MIGRATION_IMPORT_JOB_MAX_BYTES + 1], &request)
                .is_err()
        );
    }

    #[test]
    fn migration_request_requires_exact_revision_and_closed_contract() {
        let value = serde_json::json!({"schema_version": MIGRATION_REQUEST_SCHEMA_VERSION, "request_id":"r", "workspace_id":"w", "source_id":"s", "repository_identity":"repo", "revision":{"system":"git", "value":"a".repeat(40)}, "evaluation_date":"2026-09-08"});
        assert!(MigrationRequest::parse(&serde_json::to_vec(&value).unwrap()).is_ok());
        for revision in ["", "HEAD", "abcdef1", &"0".repeat(40), &"A".repeat(40)] {
            let mut invalid = value.clone();
            invalid["revision"]["value"] = revision.into();
            assert!(matches!(
                MigrationRequest::parse(&serde_json::to_vec(&invalid).unwrap()),
                Err(MigrationError::ExactRevisionRequired)
            ));
        }
        for (field, bad) in [
            ("evaluation_date", serde_json::json!("2026-02-29")),
            ("workspace_id", serde_json::json!(" ")),
            ("foreign", serde_json::json!(true)),
        ] {
            let mut invalid = value.clone();
            invalid[field] = bad;
            assert!(MigrationRequest::parse(&serde_json::to_vec(&invalid).unwrap()).is_err());
        }
    }
}
