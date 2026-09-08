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
#[cfg(test)]
mod tests {
    use super::*;
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
