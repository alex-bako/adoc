// see ADR-0009
pub(crate) mod apply;
pub(crate) mod artifact_inspection;
pub(crate) mod change_assessment;
pub(crate) mod compile;
pub(crate) mod evidence_anchor;
pub(crate) mod field_projection;
pub(crate) mod graph;
pub(crate) mod managed_retrieval;
pub(crate) mod migrate;
pub(crate) mod patch;
pub(crate) mod portable_projection;
pub(crate) mod proposal;
pub(crate) mod resolve_knowledge_objects;
pub(crate) mod resolve_object_references;
pub(crate) mod retrieval;
pub(crate) mod review;
pub(crate) mod review_envelope;
pub(crate) mod search_artifact;
pub(crate) mod signals;
pub(crate) mod validation_runtime;

/// The UTC calendar date used as `today` by clock-dependent derivations
/// (compile-time lifecycle validation and the V6.1 read-time signal queries).
pub(crate) fn local_today() -> chrono::NaiveDate {
    chrono::Utc::now().date_naive()
}

pub mod read_access;

pub(crate) mod migration;
