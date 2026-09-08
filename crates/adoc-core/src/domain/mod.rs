// see ADR-0009
pub(crate) mod artifact;
pub(crate) mod ast;
pub(crate) mod diagnostic;
pub(crate) mod executor_qualification;
pub(crate) mod external_work;
pub(crate) mod gate_result;
pub(crate) mod gateway_sensitive_access;
pub(crate) mod graph;
pub(crate) mod hashing;
pub(crate) mod identity;
pub(crate) mod inline;
pub(crate) mod knowledge_object;
// E1.2–E1.6: the managed identity, reconciliation, managed state,
// lifecycle mapping, and stage-bound obligation contracts are consumed
// by their domain tests and the stacked E1 slices; no local adapter
// surface exists yet (Cloud is the adapter, in its own repository), so
// the lib build allows dead_code here.
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod lifecycle_mapping;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod managed;
pub(crate) mod managed_field_declassification;
pub(crate) mod managed_field_provenance;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod managed_state;
pub(crate) mod obligation;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod obligation_record;
pub(crate) mod patch;
pub(crate) mod ports;
pub(crate) mod project_config;
pub(crate) mod proposal;
#[cfg_attr(not(test), allow(dead_code))]
pub(crate) mod reconciliation;
pub(crate) mod retrieval;
pub(crate) mod review;
pub(crate) mod rules;
pub(crate) mod scan;
pub(crate) mod semantic_assessment;
pub(crate) mod semantic_context;
pub(crate) mod semantic_executor;
pub(crate) mod sensitive_access;
pub(crate) mod services;
pub(crate) mod source;
pub(crate) mod source_edit;
pub(crate) mod source_provenance;
pub(crate) mod source_record;
pub(crate) mod url_safety;
pub(crate) mod value_objects;
pub(crate) mod values;

pub(crate) mod migration;
