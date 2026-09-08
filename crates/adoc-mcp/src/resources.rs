use rmcp::model::{ReadResourceResult, Resource, ResourceContents};

#[derive(Debug, Clone, Copy)]
struct AgentResource {
    uri: &'static str,
    name: &'static str,
    title: &'static str,
    description: &'static str,
    mime_type: &'static str,
    contents: &'static str,
}

const MARKDOWN: &str = "text/markdown";
const JSON_SCHEMA: &str = "application/schema+json";

const RESOURCES: &[AgentResource] = &[
    AgentResource {
        uri: "adoc://agent/v0/usage-contract",
        name: "agent-usage-contract",
        title: "Agent Usage Contract",
        description: "V2.2 stable AgentDoc agent usage rules.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/usage-contract.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/tool-guide",
        name: "agent-tool-guide",
        title: "Agent Tool Guide",
        description: "Recommended V2.2 MCP tool order for AgentDoc.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/tool-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/answer-contract",
        name: "agent-answer-contract",
        title: "Agent Answer Contract",
        description: "Citation requirements for AgentDoc answers.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/answer-contract.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/agent-instruction-guide",
        name: "agent-instruction-guide",
        title: "Agent Instruction Guide",
        description: "V5 agent_instruction objects are authored knowledge, never runtime ACLs.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/agent-instruction-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/contradiction-guide",
        name: "agent-contradiction-guide",
        title: "Contradiction Guide",
        description: "V5.6 contradiction objects are manually authored cross-references linking conflicting claims.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/contradiction-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/source-guide",
        name: "agent-source-guide",
        title: "Source Guide",
        description: "V5.7 source objects are reusable evidence pointers referencing external artefacts by path or URL.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/source-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/api-guide",
        name: "agent-api-guide",
        title: "API Guide",
        description: "V6.5.1 api objects are typed API contracts; verified apis require schema evidence.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/api-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/observation-guide",
        name: "agent-observation-guide",
        title: "Observation Guide",
        description: "V6.5.2 observation objects record findings from support, analytics, research, and ops.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/observation-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/question-guide",
        name: "agent-question-guide",
        title: "Question Guide",
        description: "V6.5.3 question objects are tracked open questions; answered questions name their resolving claim/decision.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/question-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/task-guide",
        name: "agent-task-guide",
        title: "Task Guide",
        description: "V6.5.4 task objects are documentation action items; open tasks with a past due date warn task.overdue.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/task-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/patch-contract",
        name: "agent-patch-contract",
        title: "Agent Patch Contract",
        description: "Read-only AgentDoc patch proposal rules.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/patch-contract.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/patch-apply-guide",
        name: "agent-patch-apply-guide",
        title: "Patch Apply Guide",
        description: "V6.4 gated apply loop: propose, check, apply, re-check, cite the post-check.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/patch-apply-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/project-status-guide",
        name: "agent-project-status-guide",
        title: "Project Status Guide",
        description: "How to interpret adoc.project.status.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/project-status-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/dogfood-billing-pilot",
        name: "agent-dogfood-billing-pilot",
        title: "Billing Pilot Dogfood",
        description: "V2.2 dogfood flow for examples/billing-pilot.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/dogfood-billing-pilot.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/review-workflow",
        name: "agent-review-workflow",
        title: "Review Workflow",
        description: "V3.6 PR-review workflow over adoc_diff and adoc_review.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/review-workflow.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/change-assessment-workflow",
        name: "agent-change-assessment-workflow",
        title: "Local Change Assessment Workflow",
        description: "V9.2.1 report-only local Git change assessment workflow.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/change-assessment-workflow.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/compat-guide",
        name: "agent-compat-guide",
        title: "Markdown Compatibility Guide",
        description: "V4 Markdown compatibility mode: how .md sources appear in the graph and what is citable.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/compat-guide.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/writeback-record",
        name: "agent-writeback-record",
        title: "Cloud Writeback Record",
        description: "Cloud writeback identity, event lineage, and admission bindings.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/writeback-record.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/retrieval",
        name: "schema-retrieval",
        title: "Retrieval Schema Reference",
        description: "Markdown reference for adoc.retrieval.v1.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/retrieval.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/graph-traversal",
        name: "schema-graph-traversal",
        title: "Graph Traversal Schema Reference",
        description: "Markdown reference for adoc.graph.traversal.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/graph-traversal.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/patch",
        name: "schema-patch",
        title: "Patch Schema Reference",
        description: "Markdown reference for adoc.patch.v0 and adoc.patch.check.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/patch.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/project-status",
        name: "schema-project-status",
        title: "Project Status Schema Reference",
        description: "Markdown reference for adoc.project.status.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/project-status.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/mcp-command",
        name: "schema-mcp-command",
        title: "MCP Command Schema Reference",
        description: "Markdown reference for adoc.mcp.command.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/mcp-command.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/diff",
        name: "schema-diff",
        title: "Diff Schema Reference",
        description: "Markdown reference for adoc.diff.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/diff.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/review",
        name: "schema-review",
        title: "Review Schema Reference",
        description: "Markdown reference for adoc.review.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/review.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/stale",
        name: "schema-stale",
        title: "Stale Query Schema Reference",
        description: "Markdown reference for adoc.stale.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/stale.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/contradictions",
        name: "schema-contradictions",
        title: "Contradictions Query Schema Reference",
        description: "Markdown reference for adoc.contradictions.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/contradictions.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/impacted",
        name: "schema-impacted",
        title: "Impacted-By Query Schema Reference",
        description: "Markdown reference for adoc.impacted.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/impacted.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/change-assessment",
        name: "schema-change-assessment",
        title: "Change Assessment Schema Reference",
        description: "Markdown reference for adoc.change_assessment.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/change-assessment.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/migrate-report",
        name: "schema-migrate-report",
        title: "Migration Report Schema Reference",
        description: "Markdown reference for adoc.migrate.report.v0.",
        mime_type: MARKDOWN,
        contents: include_str!("../../../docs/agent/v0/schema/migrate-report.md"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/retrieval-envelope.json",
        name: "schema-retrieval-envelope-json",
        title: "Retrieval Envelope JSON Schema",
        description: "JSON Schema for adoc.retrieval.v1.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/retrieval-envelope.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.managed_field_declassification.v0.schema.json",
        name: "schema-managed-field-declassification-v0-json",
        title: "Managed Field Declassification JSON Schema",
        description: "Immutable governed selected-field detail; shape validity grants no authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.managed_field_declassification.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.managed_field_provenance.v0.schema.json",
        name: "schema-managed-field-provenance-v0-json",
        title: "Managed Field Provenance JSON Schema",
        description: "Immutable exact-version field links to native workspace assertion rows; validity grants no authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.managed_field_provenance.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.sensitive_access.v1.schema.json",
        name: "schema-sensitive-access-v1-json",
        title: "Gateway-Reported Sensitive Access Event JSON Schema",
        description: "Immutable gateway-reported repository access metadata; validity grants no authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.sensitive_access.v1.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.sensitive_access.v0.schema.json",
        name: "schema-sensitive-access-v0-json",
        title: "Sensitive Access Event JSON Schema",
        description: "Clock-free managed workspace sensitive access metadata; validity grants no authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.sensitive_access.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.portable_projection_input.v0.schema.json",
        name: "schema-adoc-portable_projection_input-v0-json",
        title: "Portable Export Contract JSON Schema",
        description: "Closed portable export transport; schema validity does not establish release authority or historical authenticity.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.portable_projection_input.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.portable_projection.v0.schema.json",
        name: "schema-adoc-portable_projection-v0-json",
        title: "Portable Export Contract JSON Schema",
        description: "Closed portable export transport; schema validity does not establish release authority or historical authenticity.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.portable_projection.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.portable_export_preparation.v0.schema.json",
        name: "schema-agentdoc-cloud-portable_export_preparation-v0-json",
        title: "Portable Export Contract JSON Schema",
        description: "Closed portable export transport; schema validity does not establish release authority or historical authenticity.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.portable_export_preparation.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.portable_export_manifest.v0.schema.json",
        name: "schema-agentdoc-cloud-portable_export_manifest-v0-json",
        title: "Portable Export Contract JSON Schema",
        description: "Closed portable export transport; schema validity does not establish release authority or historical authenticity.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.portable_export_manifest.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.portable_export_finalization.v0.schema.json",
        name: "schema-agentdoc-cloud-portable_export_finalization-v0-json",
        title: "Portable Export Contract JSON Schema",
        description: "Closed portable export transport; schema validity does not establish release authority or historical authenticity.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.portable_export_finalization.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.export_native_fact.v0.schema.json",
        name: "schema-agentdoc-cloud-export_native_fact-v0-json",
        title: "Portable Export Native Contract JSON Schema",
        description: "Closed native evidence and authorization receipt; schema validity does not authorize release or attest a client download.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.export_native_fact.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.portable_export_receipt.v0.schema.json",
        name: "schema-agentdoc-cloud-portable_export_receipt-v0-json",
        title: "Portable Export Native Contract JSON Schema",
        description: "Closed native evidence and authorization receipt; schema validity does not authorize release or attest a client download.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.portable_export_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.managed_retrieval_input.v0.schema.json",
        name: "schema-managed-retrieval-input-v0-json",
        title: "Managed Retrieval Input JSON Schema",
        description: "Private trusted-runtime input; schema validity does not grant managed access.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.managed_retrieval_input.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/retrieval-envelope.v0.json",
        name: "schema-retrieval-envelope-v0-json",
        title: "Retrieval Envelope JSON Schema (legacy v0)",
        description: "JSON Schema for the legacy adoc.retrieval.v0 envelope; superseded by adoc.retrieval.v1 (ADR-0040) but kept published.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/retrieval-envelope.v0.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/graph-traversal-envelope.json",
        name: "schema-graph-traversal-envelope-json",
        title: "Graph Traversal Envelope JSON Schema",
        description: "JSON Schema for adoc.graph.traversal.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/graph-traversal-envelope.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/patch-input.json",
        name: "schema-patch-input-json",
        title: "Patch Input JSON Schema",
        description: "JSON Schema for adoc.patch.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/patch-input.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/patch-check.json",
        name: "schema-patch-check-json",
        title: "Patch Check JSON Schema",
        description: "JSON Schema for adoc.patch.check.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/patch-check.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/project-status.json",
        name: "schema-project-status-json",
        title: "Project Status JSON Schema",
        description: "JSON Schema for adoc.project.status.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/project-status.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/mcp-command.json",
        name: "schema-mcp-command-json",
        title: "MCP Command JSON Schema",
        description: "JSON Schema for adoc.mcp.command.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/mcp-command.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.diff.v0.schema.json",
        name: "schema-adoc-diff-v0-json",
        title: "Object Diff JSON Schema",
        description: "JSON Schema for adoc.diff.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.diff.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.review.v0.schema.json",
        name: "schema-adoc-review-v0-json",
        title: "Review Report JSON Schema",
        description: "JSON Schema for adoc.review.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.review.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.stale.v0.schema.json",
        name: "schema-adoc-stale-v0-json",
        title: "Stale Query JSON Schema",
        description: "JSON Schema for adoc.stale.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.stale.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.contradictions.v0.schema.json",
        name: "schema-adoc-contradictions-v0-json",
        title: "Contradictions Query JSON Schema",
        description: "JSON Schema for adoc.contradictions.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.contradictions.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.impacted.v0.schema.json",
        name: "schema-adoc-impacted-v0-json",
        title: "Impacted-By Query JSON Schema",
        description: "JSON Schema for adoc.impacted.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.impacted.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.change_assessment.v0.schema.json",
        name: "schema-adoc-change-assessment-v0-json",
        title: "Local Change Assessment JSON Schema",
        description: "JSON Schema for adoc.change_assessment.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.change_assessment.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.patch.apply.v0.schema.json",
        name: "schema-adoc-patch-apply-v0-json",
        title: "Patch Apply JSON Schema",
        description: "JSON Schema for adoc.patch.apply.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.patch.apply.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migrate.report.v0.schema.json",
        name: "schema-adoc-migrate-report-v0-json",
        title: "Migration Report JSON Schema",
        description: "JSON Schema for adoc.migrate.report.v0.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.migrate.report.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.lifecycle_mapping.v0.schema.json",
        name: "schema-adoc-lifecycle-mapping-v0-json",
        title: "Lifecycle Mapping JSON Schema",
        description: "JSON Schema for adoc.lifecycle_mapping.v0, the versioned flat-status to managed-state mapping/projection contract Cloud consumes as data.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.lifecycle_mapping.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.repository_baseline.v0.schema.json",
        name: "schema-adoc-repository-baseline-v0-json",
        title: "Repository Baseline JSON Schema",
        description: "JSON Schema for adoc.repository_baseline.v0, the deterministic repository-wide coverage inventory at one immutable Git revision.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.repository_baseline.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.semantic_context.v0.schema.json",
        name: "schema-adoc-semantic-context-v0-json",
        title: "Semantic Context JSON Schema",
        description: "JSON Schema for adoc.semantic_context.v0, the digest-bound exact-revision context supplied to semantic executors.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.semantic_context.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.semantic_context_input.v0.schema.json",
        name: "schema-adoc-semantic-context-input-v0-json",
        title: "Semantic Context Producer Input JSON Schema",
        description: "JSON Schema for adoc.semantic_context_input.v0; adoc-core derives coverage, outcome, and the canonical context digest.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.semantic_context_input.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.semantic_assessment.v0.schema.json",
        name: "schema-adoc-semantic-assessment-v0-json",
        title: "Semantic Assessment JSON Schema",
        description: "JSON Schema for adoc.semantic_assessment.v0, the provider-neutral findings contract validated against one exact semantic context.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.semantic_assessment.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.executor_qualification.v0.schema.json",
        name: "schema-adoc-executor-qualification-v0-json",
        title: "Executor Qualification JSON Schema",
        description: "JSON Schema for adoc.executor_qualification.v0, the four-layer capability qualification record bound to exact executor configuration digests.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.executor_qualification.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.semantic_executor_request.v0.schema.json",
        name: "schema-adoc-semantic-executor-request-v0-json",
        title: "Semantic Executor Request JSON Schema",
        description: "JSON Schema for the shared Claude, Codex, generic, local, and human semantic executor request.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.semantic_executor_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.semantic_executor_receipt.v0.schema.json",
        name: "schema-adoc-semantic-executor-receipt-v0-json",
        title: "Semantic Executor Receipt JSON Schema",
        description: "JSON Schema for deterministic semantic adapter receipts and recorded failures.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.semantic_executor_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.source_assertion.v0.schema.json",
        name: "schema-adoc-source-assertion-v0-json",
        title: "Source Assertion JSON Schema",
        description: "JSON Schema for digest-bound immutable atomic source assertions.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.source_assertion.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.source_binding.v0.schema.json",
        name: "schema-adoc-source-binding-v0-json",
        title: "Source Binding JSON Schema",
        description: "JSON Schema for standalone graph-v6 source placement bindings.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.source_binding.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.authorization_decision.v0.schema.json",
        name: "schema-adoc-authorization-decision-v0-json",
        title: "Authorization Decision JSON Schema",
        description: "JSON Schema for scoped authorization decisions and shared source-resource identities.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.source_record.v0.schema.json",
        name: "schema-adoc-source-record-v0-json",
        title: "Source Record JSON Schema",
        description: "JSON Schema for digest-bound immutable source observations.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.source_record.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.source_record.v1.schema.json",
        name: "schema-adoc-source-record-v1-json",
        title: "Source Record v1 JSON Schema",
        description: "JSON Schema for source observations bound to one exact ACL Snapshot.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.source_record.v1.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.work_request.v0.schema.json",
        name: "schema-adoc-work-request-v0-json",
        title: "External Work Request JSON Schema",
        description: "JSON Schema for the digest-bound external work request.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.work_request.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.work_result.v0.schema.json",
        name: "schema-adoc-work-result-v0-json",
        title: "External Work Result JSON Schema",
        description: "JSON Schema for replay-safe external work results.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.work_result.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.proposal.v0.schema.json",
        name: "schema-adoc-proposal-v0-json",
        title: "AgentDoc Canonical Proposal Record JSON Schema",
        description: "JSON Schema for the canonical proposal record bound to exact revisions, assessment, semantic context, and content digests.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.proposal.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.gate_result.v0.schema.json",
        name: "schema-adoc-gate-result-v0-json",
        title: "AgentDoc Gate Result JSON Schema",
        description: "JSON Schema for deterministic four-mode gate conclusions.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/adoc.gate_result.v0.schema.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.connector_capabilities.v0.schema.json",
        name: "schema-agentdoc-connector-capabilities-v0-json",
        title: "Connector Capability Manifest JSON Schema",
        description: "JSON Schema for version-exact, per-capability connector maturity manifests.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.connector_capabilities.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.approval_command.v0.schema.json",
        name: "schema-agentdoc-cloud-approval-command-v0-json",
        title: "AgentDoc Cloud Approval Command JSON Schema",
        description: "JSON Schema for the versioned Cloud approval command.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.approval_command.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.assessment_submission.v0.schema.json",
        name: "schema-agentdoc-cloud-assessment-submission-v0-json",
        title: "AgentDoc Cloud Assessment Submission JSON Schema",
        description: "JSON Schema for the versioned Cloud assessment submission.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.assessment_submission.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.egress_policy.v0.schema.json",
        name: "schema-agentdoc-cloud-egress-policy-v0-json",
        title: "AgentDoc Cloud Egress Policy JSON Schema",
        description: "JSON Schema for the versioned Cloud data-egress policy.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.egress_policy.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.gate_decision.v0.schema.json",
        name: "schema-agentdoc-cloud-gate-decision-v0-json",
        title: "AgentDoc Cloud Gate Decision JSON Schema",
        description: "JSON Schema for the versioned Cloud gate decision.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.gate_decision.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.ingestion_result.v0.schema.json",
        name: "schema-agentdoc-cloud-ingestion-result-v0-json",
        title: "AgentDoc Cloud Ingestion Result JSON Schema",
        description: "JSON Schema for the versioned Cloud ingestion outcome.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.ingestion_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_initialization_request.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-initialization-request-v0-json",
        title: "Migration Initialization Request JSON Schema",
        description: "Closed Cloud-owned exact-revision human initialization request contract.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_initialization_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_initialization_attestation.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-initialization-attestation-v0-json",
        title: "Migration Initialization Attestation JSON Schema",
        description: "Closed Cloud-owned exact-revision human initialization attestation contract.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_initialization_attestation.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_initialization_result.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-initialization-result-v0-json",
        title: "Migration Initialization Result JSON Schema",
        description: "Closed Cloud-owned exact-revision human initialization result contract.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_initialization_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_completion_receipt.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-completion-receipt-v0-json",
        title: "Migration Completion Receipt JSON Schema",
        description: "Closed Cloud-owned portable migration correspondence; no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_completion_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_transition_request.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-transition-request-v0-json",
        title: "Migration Transition Request JSON Schema",
        description: "Closed Cloud migration lifecycle contract; native evidence and authority remain required.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_transition_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_transition_receipt.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-transition-receipt-v0-json",
        title: "Migration Transition Receipt JSON Schema",
        description: "Closed Cloud migration lifecycle contract; native evidence and authority remain required.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_transition_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_transition_result.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-transition-result-v0-json",
        title: "Migration Transition Result JSON Schema",
        description: "Closed Cloud migration lifecycle contract; native evidence and authority remain required.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_transition_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migration_qualification.v0.schema.json",
        name: "schema-adoc-migration_qualification-v0-json",
        title: "Migration Qualification Contract JSON Schema",
        description: "Versioned qualification and flagged source evidence; no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.migration_qualification.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migration_qualification_receipt.v0.schema.json",
        name: "schema-adoc-migration_qualification_receipt-v0-json",
        title: "Migration Qualification Contract JSON Schema",
        description: "Versioned qualification and flagged source evidence; no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.migration_qualification_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_qualification_result.v0.schema.json",
        name: "schema-agentdoc-cloud-migration_qualification_result-v0-json",
        title: "Migration Qualification Contract JSON Schema",
        description: "Versioned qualification and flagged source evidence; no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_qualification_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_import_job.v0.schema.json",
        name: "schema-agentdoc-cloud-migration_import_job-v0-json",
        title: "Migration Import Contract JSON Schema",
        description: "Exact-snapshot migration import contract; candidate preparation grants no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_import_job.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_validation_invocation.v0.schema.json",
        name: "schema-agentdoc-cloud-migration_validation_invocation-v0-json",
        title: "Migration Import Contract JSON Schema",
        description: "Exact-snapshot migration import contract; candidate preparation grants no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_validation_invocation.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migration_import.v0.schema.json",
        name: "schema-adoc-migration_import-v0-json",
        title: "Migration Import Contract JSON Schema",
        description: "Exact-snapshot migration import contract; candidate preparation grants no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.migration_import.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_import_result.v0.schema.json",
        name: "schema-agentdoc-cloud-migration_import_result-v0-json",
        title: "Migration Import Contract JSON Schema",
        description: "Exact-snapshot migration import contract; candidate preparation grants no activation authority.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_import_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migration_request.v0.schema.json",
        name: "schema-adoc-migration-request-v0-json",
        title: "AgentDoc Migration Request JSON Schema",
        description: "Portable exact-revision migration preparation request contract.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.migration_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/adoc.migration_receipt.v0.schema.json",
        name: "schema-adoc-migration-receipt-v0-json",
        title: "AgentDoc Migration Receipt JSON Schema",
        description: "Portable exact-revision migration preparation receipt contract.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/adoc.migration_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_receipt.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-receipt-v0-json",
        title: "AgentDoc Cloud Migration Receipt JSON Schema",
        description: "JSON Schema for the versioned Cloud migration receipt.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_receipt.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.migration_request.v0.schema.json",
        name: "schema-agentdoc-cloud-migration-request-v0-json",
        title: "AgentDoc Cloud Migration Request JSON Schema",
        description: "JSON Schema for the versioned Cloud migration request.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.migration_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.proposal_command.v0.schema.json",
        name: "schema-agentdoc-cloud-proposal-command-v0-json",
        title: "AgentDoc Cloud Proposal Command JSON Schema",
        description: "JSON Schema for the versioned Cloud proposal command.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.proposal_command.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.repository_config.v0.schema.json",
        name: "schema-agentdoc-cloud-repository-config-v0-json",
        title: "AgentDoc Cloud Repository Configuration JSON Schema",
        description: "JSON Schema for the versioned Cloud repository configuration.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.repository_config.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.work_request.v0.schema.json",
        name: "schema-agentdoc-cloud-work-request-v0-json",
        title: "AgentDoc Cloud Work Request JSON Schema",
        description: "JSON Schema for the versioned Cloud work request.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.work_request.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.work_result.v0.schema.json",
        name: "schema-agentdoc-cloud-work-result-v0-json",
        title: "AgentDoc Cloud Work Result JSON Schema",
        description: "JSON Schema for the versioned Cloud work result.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.work_result.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/agentdoc.cloud.writeback_record.v0.schema.json",
        name: "schema-agentdoc-cloud-writeback-record-v0-json",
        title: "AgentDoc Cloud Writeback Record JSON Schema",
        description: "Closed schema for exact projection lineage and mandatory target revision preconditions; not dispatch authorization.",
        mime_type: JSON_SCHEMA,
        contents: include_str!(
            "../../../docs/agent/v0/schema/agentdoc.cloud.writeback_record.v0.schema.json"
        ),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/search-artifact.json",
        name: "schema-search-artifact-json",
        title: "Search Artifact JSON Schema",
        description: "JSON Schema for adoc.search.v2, the dist/docs.search.json wire shape. The artifact itself is a build output, not an MCP resource.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/search-artifact.json"),
    },
    AgentResource {
        uri: "adoc://agent/v0/schema/graph-artifact.v6.json",
        name: "schema-graph-artifact-v6-json",
        title: "Graph Artifact v6 JSON Schema",
        description: "JSON Schema for adoc.graph.v6, separating the governed-meaning content hash from portable source coordinates.",
        mime_type: JSON_SCHEMA,
        contents: include_str!("../../../docs/agent/v0/schema/graph-artifact.v6.json"),
    },
];

pub fn list() -> Vec<Resource> {
    RESOURCES
        .iter()
        .map(|resource| {
            Resource::new(resource.uri, resource.name)
                .with_title(resource.title)
                .with_description(resource.description)
                .with_mime_type(resource.mime_type)
                .with_size(resource.contents.len() as u64)
        })
        .collect()
}

pub fn read(uri: &str) -> Option<ReadResourceResult> {
    RESOURCES
        .iter()
        .find(|resource| resource.uri == uri)
        .map(|resource| {
            ReadResourceResult::new(vec![
                ResourceContents::text(resource.contents, resource.uri)
                    .with_mime_type(resource.mime_type),
            ])
        })
}
