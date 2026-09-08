mod artifact_paths;
mod assess_changes;
mod build;
mod check;
mod contradictions;
mod diff;
mod graph;
mod impacted_by;
mod init;
mod managed_retrieval;
mod migrate;
mod patch;
mod portable_project;
mod proposal_record;
pub(crate) use portable_project::portable_project;
mod review;
mod search;
mod semantic_executor;
mod stale;
mod why;

use std::path::PathBuf;

use adoc_core::{Diagnostic, RetrievalEnvelope, RetrievalRecord, Severity};
use adoc_local::ResolvedRetrievalRecord;

use crate::error::CliError;
use crate::presentation::{
    ExpiresInfo, PresentationRecord, ResolvedFormat, json as json_presentation,
};

pub(crate) use assess_changes::{
    AssessChangesCommandInput, BaselineCommandInput, assess_changes, baseline,
};
pub(crate) use build::build;
pub(crate) use check::{check, check_receipt};
pub(crate) use contradictions::{ContradictionsCommandInput, contradictions};
pub(crate) use diff::{DiffCommandInput, diff};
pub(crate) use graph::{GraphCommandInput, graph};
pub(crate) use impacted_by::{ImpactedByCommandInput, impacted_by};
pub(crate) use init::init;
pub(crate) use managed_retrieval::managed_retrieve;
pub(crate) use migrate::{MigrateCommandInput, migrate};
pub(crate) use patch::{PatchCommandInput, patch};
pub(crate) use proposal_record::proposal_record;
pub(crate) use review::{ReviewCommandInput, review};
pub(crate) use search::{SearchCommandInput, search_command};
pub(crate) use semantic_executor::{semantic_context, semantic_executor};
pub(crate) use stale::{StaleCommandInput, stale};
pub(crate) use why::why;

/// The shared JSON-emission tail of every read command: serialize the
/// envelope to stdout, keep the command's exit code, and route an I/O
/// failure through the standard report path.
fn write_json_or_report<T: serde::Serialize>(envelope: &T, exit_code: i32) -> i32 {
    json_presentation::write_json(envelope, &mut std::io::stdout()).map_or_else(
        |source| report(CliError::StdoutIo { source }),
        |()| exit_code,
    )
}

/// The signal read commands' shared error emission: JSON output still ships
/// the full envelope; human formats print the diagnostics to stderr. The
/// impacted-by Markdown error branch is deliberately NOT unified here
/// (ADR-0038 records the per-command differences as intent).
fn emit_envelope_error<T: serde::Serialize>(
    envelope: &T,
    diagnostics: &[Diagnostic],
    resolved: ResolvedFormat,
    exit_code: i32,
) -> i32 {
    if resolved == ResolvedFormat::Json {
        return write_json_or_report(envelope, exit_code);
    }
    eprint_diagnostics(diagnostics);
    exit_code
}

fn emit_retrieval_error(
    diagnostics: Vec<Diagnostic>,
    resolved: ResolvedFormat,
    exit_code: i32,
) -> i32 {
    if resolved == ResolvedFormat::Json {
        return json_presentation::write_envelope_json(
            &RetrievalEnvelope::new(Vec::new(), diagnostics),
            &mut std::io::stdout(),
        )
        .map_or_else(
            |source| report(CliError::StdoutIo { source }),
            |()| exit_code,
        );
    }
    eprint_diagnostics(&diagnostics);
    exit_code
}

fn presentation_record_from_resolved(
    resolved: ResolvedRetrievalRecord,
    include_expires: bool,
) -> PresentationRecord {
    let record = resolved.record;
    let expires = include_expires.then(|| expires_info(&record)).flatten();
    PresentationRecord {
        record,
        related_statuses: resolved.related_statuses,
        expires,
    }
}

fn expires_info(record: &RetrievalRecord) -> Option<ExpiresInfo> {
    record
        .fields
        .get("expires_at")
        .and_then(|value| chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d").ok())
        .map(|date| {
            let today = chrono::Local::now().date_naive();
            ExpiresInfo {
                date,
                days_until: (date - today).num_days(),
            }
        })
}

fn report(error: CliError) -> i32 {
    eprintln!("{error}");
    error.exit_code()
}

fn current_dir() -> Result<PathBuf, CliError> {
    std::env::current_dir().map_err(|source| adoc_local::LocalError::CurrentDir { source }.into())
}

/// The single implementation of the stable human diagnostic format
/// (`file:line:col: severity[code] message` plus `object_id:`/`help:`
/// continuation lines). Both the stdout and stderr printers render through
/// this, so the format is written once and unit-testable without spawning
/// the binary.
fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    use std::fmt::Write as _;

    // Repo-relative paths: GitHub problem matchers (and humans reading CI
    // logs) resolve paths against the checkout root = the working directory.
    let base = std::env::current_dir().ok();
    let mut out = String::new();
    for diagnostic in diagnostics {
        if let Some(span) = &diagnostic.span {
            writeln!(
                out,
                "{}:{}:{}: {}[{}] {}",
                crate::presentation::relativize_path(&span.file, base.as_deref()).display(),
                span.start.line,
                span.start.column,
                diagnostic.severity,
                diagnostic.code,
                diagnostic.message
            )
            .expect("writing to String cannot fail");
        } else {
            writeln!(
                out,
                "{}[{}] {}",
                diagnostic.severity, diagnostic.code, diagnostic.message
            )
            .expect("writing to String cannot fail");
        }
        if let Some(object_id) = &diagnostic.object_id {
            writeln!(out, "  object_id: {object_id}").expect("writing to String cannot fail");
        }
        if let Some(help) = &diagnostic.help {
            writeln!(out, "  help: {help}").expect("writing to String cannot fail");
        }
    }
    out
}

fn format_summary(diagnostics: &[Diagnostic]) -> String {
    let errors = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let warnings = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Warning)
        .count();

    format!("{errors} errors, {warnings} warnings")
}

fn print_diagnostics(diagnostics: &[Diagnostic]) {
    print!("{}", format_diagnostics(diagnostics));
}

fn eprint_diagnostics(diagnostics: &[Diagnostic]) {
    eprint!("{}", format_diagnostics(diagnostics));
}

fn print_summary(diagnostics: &[Diagnostic]) {
    println!("{}", format_summary(diagnostics));
}

mod migration_prepare;
pub(crate) use migration_prepare::migration_prepare;

#[cfg(test)]
mod format_tests {
    use adoc_core::{Diagnostic, DiagnosticCode, Severity};

    use super::{format_diagnostics, format_summary};

    // The spanned `file:line:col:` prefix stays covered by the check_cli
    // integration suite — `SourceSpan` is not on the Public Core Surface, so
    // a spanned Diagnostic cannot be constructed here.
    fn diagnostic(severity: Severity, message: &str) -> Diagnostic {
        Diagnostic {
            code: DiagnosticCode::SchemaMissingField,
            severity,
            message: message.to_string(),
            span: None,
            object_id: None,
            help: None,
        }
    }

    #[test]
    fn spanless_diagnostic_renders_severity_code_and_message() {
        assert_eq!(
            format_diagnostics(&[diagnostic(Severity::Error, "missing `owner`")]),
            "error[schema.missing_field] missing `owner`\n"
        );
    }

    #[test]
    fn object_id_and_help_render_as_continuation_lines() {
        let mut with_details = diagnostic(Severity::Error, "bad field");
        with_details.object_id = Some("billing.credits".to_string());
        with_details.help = Some("Add the missing field.".to_string());
        assert_eq!(
            format_diagnostics(&[with_details]),
            "error[schema.missing_field] bad field\n  object_id: billing.credits\n  help: Add the missing field.\n"
        );
    }

    #[test]
    fn no_diagnostics_renders_nothing() {
        assert_eq!(format_diagnostics(&[]), "");
    }

    #[test]
    fn summary_counts_errors_and_warnings() {
        assert_eq!(format_summary(&[]), "0 errors, 0 warnings");
        assert_eq!(
            format_summary(&[
                diagnostic(Severity::Error, "x"),
                diagnostic(Severity::Error, "y"),
                diagnostic(Severity::Warning, "expired"),
            ]),
            "2 errors, 1 warnings"
        );
    }
}
