use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

mod support;

use adoc_local::{PathPolicy, ProjectRootPathPolicy};
use adoc_mcp::{
    AdocDiffParams, AdocPatchCheckParams, AdocReviewParams, AgentDocMcpServer, BuildParams,
    InitParams, PatchInput, ProjectStatusParams, SearchParams, WhyParams,
};
use rmcp::ServerHandler;
use rmcp::model::ResourceContents;

use support::build_v3_review_fixture;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory can be created");
    }
    fs::write(path, contents).expect("file can be written");
}

fn source() -> &'static str {
    "# Billing @doc(team.billing)\n\n::claim billing.credits\nstatus: draft\n--\nCredits apply after payment.\n::\n"
}

fn copy_billing_pilot_fixture(root: &Path) {
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/billing-pilot")
        .canonicalize()
        .expect("billing pilot fixture path");
    for file in [
        "agentdoc.config.yaml",
        "01-glossary.adoc",
        "02-claims.adoc",
        "03-decisions.adoc",
        "04-warnings.adoc",
    ] {
        fs::copy(fixture_root.join(file), root.join(file)).expect("fixture file copies");
    }
}

#[test]
fn tool_selected_project_policy_cannot_widen_the_gateway_audience() {
    let gateway = tempfile::tempdir().expect("gateway root");
    let selected = tempfile::tempdir().expect("selected project");
    write(
        &gateway.path().join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\nretrieval_policy:\n  audience: public\n  allowed_visibilities: [public]\n",
    );
    write(
        &selected.path().join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\nretrieval_policy:\n  audience: internal\n  allowed_visibilities: [public, internal]\n",
    );
    write(
        &selected.path().join("docs/index.adoc"),
        &source().replace("status: draft", "status: draft\nvisibility: internal"),
    );
    let server = AgentDocMcpServer::new(gateway.path().to_path_buf());
    let built = server
        .run_build(BuildParams {
            project_root: Some(selected.path().to_path_buf()),
            path: Some("docs".into()),
            out: Some("dist".into()),
            no_embeddings: true,
        })
        .expect("fixture builds");
    assert_eq!(built["ok"], true);
    let html = fs::read_to_string(selected.path().join("dist/docs.html")).expect("built HTML");
    assert!(
        html.contains("billing.credits"),
        "restricted marker preserves existence"
    );
    assert!(
        !html.contains("Credits apply after payment."),
        "gateway build policy wins over selected project policy"
    );
    let local = adoc_local::LocalContext::new(
        selected.path().to_path_buf(),
        ProjectRootPathPolicy::new(selected.path()).expect("local path policy"),
    )
    .why(adoc_local::WhyInput {
        object_id: "billing.credits".into(),
        artifact: Some("dist/docs.graph.json".into()),
    })
    .expect("explicit local project policy permits its internal object");
    assert_eq!(local.records.len(), 1);
    let response = server
        .run_why(WhyParams {
            project_root: Some(selected.path().to_path_buf()),
            object_id: "billing.credits".into(),
            artifact: Some("dist/docs.graph.json".into()),
        })
        .expect("gateway returns a retrieval envelope")
        .structured_content
        .unwrap();
    assert_eq!(response["records"], serde_json::json!([]));
    assert_eq!(
        response["diagnostics"][0]["code"],
        "retrieval.object_not_found"
    );
}

#[test]
fn path_policy_rejects_parent_escape() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    let policy = ProjectRootPathPolicy::new(root).expect("policy");

    let error = policy
        .resolve_read_path(Path::new("../outside.adoc"))
        .expect_err("escape rejected");

    assert!(error.to_string().contains("path_outside_project"));
}

#[test]
fn init_tool_writes_inside_project_root() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    let value = server
        .run_init(InitParams { project_root: None })
        .expect("init succeeds");

    assert_eq!(value["schema_version"], "adoc.mcp.command.v0");
    assert_eq!(value["ok"], true);
    assert!(workspace.path().join("agentdoc.config.yaml").exists());
    assert!(workspace.path().join("docs/index.adoc").exists());
}

#[test]
fn build_tool_rejects_configured_outputs_outside_project_root() {
    let workspace = tempfile::tempdir().expect("workspace");
    let outside = tempfile::tempdir().expect("outside");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        &format!(
            "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  html: {}\n  graph: dist/docs.graph.json\nembeddings:\n  provider: none\n",
            outside.path().join("docs.html").display()
        ),
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());

    let error = server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: false,
        })
        .expect_err("outside configured output is rejected");

    assert!(error.to_string().contains("path_outside_project"));
    assert!(!outside.path().join("docs.html").exists());
}

#[test]
fn patch_check_accepts_inline_patch_json() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: Some("docs".into()),
            out: Some("dist".into()),
            no_embeddings: true,
        })
        .expect("build succeeds");
    let graph: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("dist/docs.graph.json")).unwrap())
            .unwrap();
    let hash = graph["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["id"] == "billing.credits")
        .unwrap()["content_hash"]
        .as_str()
        .unwrap();

    let value = server
        .run_patch_check(AdocPatchCheckParams {
            project_root: None,
            artifact: Some("dist/docs.graph.json".into()),
            input: PatchInput::Inline {
                patch: serde_json::json!({
                    "schema_version": "adoc.patch.v0",
                    "op": "replace_body",
                    "target": "billing.credits",
                    "base_hash": hash,
                    "changes": { "body": "Credits apply after ledger commit." },
                    "reason": "Update billing behavior."
                }),
            },
        })
        .expect("inline patch check succeeds");

    assert_eq!(value["schema_version"], "adoc.patch.check.v0");
    assert_eq!(value["valid"], true);
}

#[test]
fn server_implements_rmcp_server_handler_with_tools_capability() {
    fn assert_handler<T: ServerHandler>(_server: &T) {}

    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    assert_handler(&server);
    assert!(server.get_info().capabilities.tools.is_some());
    assert!(server.get_info().capabilities.resources.is_some());
    assert!(server.get_info().capabilities.prompts.is_some());
}

#[test]
fn project_status_tool_is_read_only_by_default() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: none\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());

    let value = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: None,
            no_embeddings: false,
        })
        .expect("status succeeds");

    assert_eq!(value["schema_version"], "adoc.project.status.v0");
    assert_eq!(value["refresh"]["requested"], "none");
    assert_eq!(value["artifacts"]["graph"]["exists"], false);
    assert_eq!(value["readiness"]["retrieval"], false);
    assert!(
        !root.join("dist/docs.graph.json").exists(),
        "default status must not build artifacts"
    );
}

#[test]
fn project_status_tool_can_refresh_with_check_or_build() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: none\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());

    let check = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: Some("check".to_string()),
            no_embeddings: true,
        })
        .expect("status check succeeds");
    assert_eq!(check["refresh"]["requested"], "check");
    assert_eq!(check["refresh"]["exit_code"], 0);
    assert_eq!(check["artifacts"]["graph"]["exists"], false);
    assert!(!root.join("dist/docs.graph.json").exists());

    let build = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: Some("build".to_string()),
            no_embeddings: false,
        })
        .expect("status build succeeds");
    assert_eq!(build["refresh"]["requested"], "build");
    assert_eq!(build["refresh"]["exit_code"], 0);
    assert_eq!(
        build["artifacts"]["graph"]["schema_version"],
        "adoc.graph.v6"
    );
    assert_eq!(build["artifacts"]["graph"]["object_count"], 1);
    assert_eq!(build["readiness"]["patch_validation"], true);
}

#[test]
fn project_status_rejects_configured_outputs_outside_project_root() {
    let workspace = tempfile::tempdir().expect("workspace");
    let outside = tempfile::tempdir().expect("outside");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        &format!(
            "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  html: {}\n  graph: dist/docs.graph.json\nembeddings:\n  provider: none\n",
            outside.path().join("docs.html").display()
        ),
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());

    let error = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: Some("build".to_string()),
            no_embeddings: false,
        })
        .expect_err("outside configured output is rejected");

    assert!(error.to_string().contains("path_outside_project"));
    assert!(!outside.path().join("docs.html").exists());
}

#[test]
fn lists_and_reads_all_stable_agent_resources() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());
    let expected = [
        "adoc://agent/v0/usage-contract",
        "adoc://agent/v0/tool-guide",
        "adoc://agent/v0/answer-contract",
        "adoc://agent/v0/agent-instruction-guide",
        "adoc://agent/v0/contradiction-guide",
        "adoc://agent/v0/source-guide",
        "adoc://agent/v0/api-guide",
        "adoc://agent/v0/observation-guide",
        "adoc://agent/v0/question-guide",
        "adoc://agent/v0/task-guide",
        "adoc://agent/v0/patch-contract",
        "adoc://agent/v0/patch-apply-guide",
        "adoc://agent/v0/project-status-guide",
        "adoc://agent/v0/dogfood-billing-pilot",
        "adoc://agent/v0/review-workflow",
        "adoc://agent/v0/change-assessment-workflow",
        "adoc://agent/v0/compat-guide",
        "adoc://agent/v0/writeback-record",
        "adoc://agent/v0/schema/retrieval",
        "adoc://agent/v0/schema/graph-traversal",
        "adoc://agent/v0/schema/patch",
        "adoc://agent/v0/schema/project-status",
        "adoc://agent/v0/schema/mcp-command",
        "adoc://agent/v0/schema/diff",
        "adoc://agent/v0/schema/review",
        "adoc://agent/v0/schema/stale",
        "adoc://agent/v0/schema/contradictions",
        "adoc://agent/v0/schema/impacted",
        "adoc://agent/v0/schema/change-assessment",
        "adoc://agent/v0/schema/migrate-report",
        "adoc://agent/v0/schema/retrieval-envelope.json",
        "adoc://agent/v0/schema/adoc.managed_field_declassification.v0.schema.json",
        "adoc://agent/v0/schema/adoc.managed_field_provenance.v0.schema.json",
        "adoc://agent/v0/schema/adoc.sensitive_access.v1.schema.json",
        "adoc://agent/v0/schema/adoc.sensitive_access.v0.schema.json",
        "adoc://agent/v0/schema/adoc.portable_projection_input.v0.schema.json",
        "adoc://agent/v0/schema/adoc.portable_projection.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.portable_export_preparation.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.portable_export_manifest.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.portable_export_finalization.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.export_native_fact.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.portable_export_receipt.v0.schema.json",
        "adoc://agent/v0/schema/adoc.managed_retrieval_input.v0.schema.json",
        "adoc://agent/v0/schema/retrieval-envelope.v0.json",
        "adoc://agent/v0/schema/graph-traversal-envelope.json",
        "adoc://agent/v0/schema/patch-input.json",
        "adoc://agent/v0/schema/patch-check.json",
        "adoc://agent/v0/schema/project-status.json",
        "adoc://agent/v0/schema/mcp-command.json",
        "adoc://agent/v0/schema/adoc.diff.v0.schema.json",
        "adoc://agent/v0/schema/adoc.review.v0.schema.json",
        "adoc://agent/v0/schema/adoc.stale.v0.schema.json",
        "adoc://agent/v0/schema/adoc.contradictions.v0.schema.json",
        "adoc://agent/v0/schema/adoc.impacted.v0.schema.json",
        "adoc://agent/v0/schema/adoc.change_assessment.v0.schema.json",
        "adoc://agent/v0/schema/adoc.patch.apply.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migrate.report.v0.schema.json",
        "adoc://agent/v0/schema/adoc.lifecycle_mapping.v0.schema.json",
        "adoc://agent/v0/schema/adoc.repository_baseline.v0.schema.json",
        "adoc://agent/v0/schema/adoc.semantic_context.v0.schema.json",
        "adoc://agent/v0/schema/adoc.semantic_context_input.v0.schema.json",
        "adoc://agent/v0/schema/adoc.semantic_assessment.v0.schema.json",
        "adoc://agent/v0/schema/adoc.executor_qualification.v0.schema.json",
        "adoc://agent/v0/schema/adoc.semantic_executor_request.v0.schema.json",
        "adoc://agent/v0/schema/adoc.semantic_executor_receipt.v0.schema.json",
        "adoc://agent/v0/schema/adoc.source_assertion.v0.schema.json",
        "adoc://agent/v0/schema/adoc.source_binding.v0.schema.json",
        "adoc://agent/v0/schema/adoc.authorization_decision.v0.schema.json",
        "adoc://agent/v0/schema/adoc.source_record.v0.schema.json",
        "adoc://agent/v0/schema/adoc.source_record.v1.schema.json",
        "adoc://agent/v0/schema/adoc.work_request.v0.schema.json",
        "adoc://agent/v0/schema/adoc.work_result.v0.schema.json",
        "adoc://agent/v0/schema/adoc.proposal.v0.schema.json",
        "adoc://agent/v0/schema/adoc.gate_result.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.connector_capabilities.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.approval_command.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.assessment_submission.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.egress_policy.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.gate_decision.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.ingestion_result.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migration_qualification.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migration_qualification_receipt.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_qualification_result.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_import_job.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_validation_invocation.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migration_import.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_import_result.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migration_request.v0.schema.json",
        "adoc://agent/v0/schema/adoc.migration_receipt.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_receipt.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.migration_request.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.proposal_command.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.repository_config.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.work_request.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.work_result.v0.schema.json",
        "adoc://agent/v0/schema/agentdoc.cloud.writeback_record.v0.schema.json",
        "adoc://agent/v0/schema/search-artifact.json",
        "adoc://agent/v0/schema/graph-artifact.v6.json",
    ];

    let resources = server.list_agent_resources();
    let listed = resources
        .iter()
        .map(|resource| resource.uri.as_str())
        .collect::<Vec<_>>();
    assert_eq!(listed, expected);

    for uri in expected {
        let resource = resources
            .iter()
            .find(|resource| resource.uri == uri)
            .expect("resource listed");
        let result = server.read_agent_resource(uri).expect("resource reads");
        assert_eq!(result.contents.len(), 1);
        let ResourceContents::TextResourceContents {
            mime_type, text, ..
        } = &result.contents[0]
        else {
            panic!("agent resources should be text");
        };
        if uri.ends_with(".json") {
            assert_eq!(
                resource.mime_type.as_deref(),
                Some("application/schema+json")
            );
            assert_eq!(mime_type.as_deref(), Some("application/schema+json"));
            let schema: serde_json::Value =
                serde_json::from_str(text).expect("json schema resource is valid JSON");
            assert_eq!(
                schema["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
        } else {
            assert_eq!(resource.mime_type.as_deref(), Some("text/markdown"));
            assert_eq!(mime_type.as_deref(), Some("text/markdown"));
            assert!(text.starts_with("# "));
            assert!(
                text.contains("V2.2") || text.contains("adoc.") || text.contains("agentdoc.cloud.")
            );
        }
    }
}

#[test]
fn lists_versioned_prompts_and_pinned_aliases() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    let prompts = server.list_agent_prompts();
    let names = prompts
        .iter()
        .map(|prompt| prompt.name.as_str())
        .collect::<Vec<_>>();
    assert_eq!(
        names,
        [
            "adoc_answer_with_citations_v0",
            "adoc_answer_with_citations",
            "adoc_propose_patch_v0",
            "adoc_propose_patch",
            "adoc_propose_patch_v1",
            "adoc_inspect_project_status_v0",
            "adoc_inspect_project_status",
            "adoc_dogfood_billing_pilot_v0",
            "adoc_dogfood_billing_pilot",
            "adoc_review_pull_request_v0",
            "adoc_review_pull_request",
            "adoc_explain_what_changed_v0",
            "adoc_explain_what_changed",
        ]
    );

    let answer = prompts
        .iter()
        .find(|prompt| prompt.name == "adoc_answer_with_citations_v0")
        .expect("answer prompt listed");
    let args = answer.arguments.as_ref().expect("rich arguments");
    assert!(args.iter().any(|arg| {
        arg.name == "query"
            && arg.required == Some(true)
            && arg
                .description
                .as_deref()
                .is_some_and(|desc| desc.contains("question"))
    }));
    assert!(args.iter().any(|arg| {
        arg.name == "retrieval_mode"
            && arg.description.as_deref().is_some_and(|desc| {
                desc.contains("hybrid") && desc.contains("semantic") && desc.contains("lexical")
            })
    }));

    let versioned = server
        .get_agent_prompt("adoc_answer_with_citations_v0", None)
        .expect("versioned prompt");
    let alias = server
        .get_agent_prompt("adoc_answer_with_citations", None)
        .expect("alias prompt");
    assert_eq!(alias.messages, versioned.messages);

    let text = serde_json::to_string(&versioned).expect("prompt serializes");
    assert!(text.contains("adoc_search"));
    assert!(text.contains("Object ID"));
    assert!(text.contains("status"));
    assert!(text.contains("caveats"));
}

#[test]
fn dogfood_billing_pilot_flow_uses_status_search_why_and_patch_check() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    copy_billing_pilot_fixture(root);
    let server = AgentDocMcpServer::new(root.to_path_buf());

    let initial_status = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: None,
            no_embeddings: false,
        })
        .expect("initial status succeeds");
    assert_eq!(initial_status["schema_version"], "adoc.project.status.v0");

    let build_status = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: Some("build".to_string()),
            no_embeddings: true,
        })
        .expect("status build succeeds");
    assert_eq!(build_status["readiness"]["retrieval"], true);
    assert_eq!(build_status["readiness"]["patch_validation"], true);

    let search = server
        .run_search(SearchParams {
            project_root: None,
            query: "billing.credits".to_string(),
            artifact: None,
            search_artifact: None,
            semantic: false,
            lexical: true,
            objects_only: false,
            prose_only: false,
            kind: None,
            status: None,
            owner: None,
            source_path: None,
            related_to: None,
            relation: None,
            direction: None,
            top: Some(5),
        })
        .expect("lexical search succeeds")
        .structured_content
        .unwrap();
    assert_eq!(search["schema_version"], "adoc.retrieval.v1");
    assert!(
        search["records"]
            .as_array()
            .expect("records array")
            .iter()
            .any(|record| record["id"] == "billing.credits")
    );

    let why = server
        .run_why(WhyParams {
            project_root: None,
            object_id: "billing.credits".to_string(),
            artifact: None,
        })
        .expect("why succeeds")
        .structured_content
        .unwrap();
    let record = &why["records"][0];
    assert_eq!(record["id"], "billing.credits");
    assert_eq!(record["kind"], "glossary");
    assert_eq!(record["owner"], "team-billing");
    let base_hash = record["content_hash"]
        .as_str()
        .expect("record has content hash");

    let patch = server
        .run_patch_check(AdocPatchCheckParams {
            project_root: None,
            artifact: None,
            input: PatchInput::Inline {
                patch: serde_json::json!({
                    "schema_version": "adoc.patch.v0",
                    "op": "replace_body",
                    "target": "billing.credits",
                    "base_hash": base_hash,
                    "changes": {
                        "body": "Credits are account balance adjustments that reduce future invoices after reviewed ledger, refund, or support correction events."
                    },
                    "reason": "Dogfood a validated billing glossary patch."
                }),
            },
        })
        .expect("inline patch validates");

    assert_eq!(patch["schema_version"], "adoc.patch.check.v0");
    assert_eq!(patch["valid"], true);
}

#[test]
fn stdio_server_smoke_lists_agent_resources() {
    let workspace = tempfile::tempdir().expect("workspace");
    let mut child = Command::new(env!("CARGO_BIN_EXE_adoc-mcp"))
        .current_dir(workspace.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("adoc-mcp binary spawns");
    let mut stdin = child.stdin.take().expect("stdin is piped");
    let stdout = child.stdout.take().expect("stdout is piped");
    let mut stdout = BufReader::new(stdout);

    write_json_line(
        &mut stdin,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "adoc-mcp-test", "version": "0" }
            }
        }),
    );
    let init = read_json_line(&mut stdout);
    assert_eq!(init["id"], 1);
    assert!(init["result"]["capabilities"]["tools"].is_object());
    assert!(init["result"]["capabilities"]["resources"].is_object());
    assert!(init["result"]["capabilities"]["prompts"].is_object());

    write_json_line(
        &mut stdin,
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }),
    );
    write_json_line(
        &mut stdin,
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "resources/list"
        }),
    );
    let resources = read_json_line(&mut stdout);
    assert_eq!(resources["id"], 2);
    assert!(
        resources["result"]["resources"]
            .as_array()
            .expect("resources array")
            .iter()
            .any(|resource| resource["uri"] == "adoc://agent/v0/usage-contract")
    );

    drop(stdin);
    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn adoc_diff_returns_diff_envelope_for_two_commit_fixture() {
    let fixture = build_v3_review_fixture("diff-tool");
    let server = AgentDocMcpServer::new(fixture.root.clone());

    let value = server
        .run_diff(AdocDiffParams {
            project_root: None,
            base_ref: "main".to_string(),
            head_ref: None,
        })
        .expect("adoc_diff runs");

    assert_eq!(value["schema_version"], "adoc.diff.v0");
    let changed = value["changed"].as_array().expect("changed array");
    assert!(
        changed.iter().any(|entry| entry["id"] == "billing.refunds"),
        "expected billing.refunds in changed[]; got {value}"
    );
    let refunds = changed
        .iter()
        .find(|entry| entry["id"] == "billing.refunds")
        .expect("billing.refunds entry");
    assert!(refunds["base"]["content_hash"].is_string());
    assert!(refunds["head"]["content_hash"].is_string());
    assert_ne!(
        refunds["base"]["content_hash"],
        refunds["head"]["content_hash"]
    );
}

#[test]
fn adoc_diff_rejects_unresolvable_base_ref() {
    let fixture = build_v3_review_fixture("diff-bad-ref");
    let server = AgentDocMcpServer::new(fixture.root.clone());

    let error = server
        .run_diff(AdocDiffParams {
            project_root: None,
            base_ref: "definitely-not-a-real-ref".to_string(),
            head_ref: None,
        })
        .expect_err("unresolvable ref must error");

    assert!(
        error.to_string().contains("definitely-not-a-real-ref"),
        "error must surface the bad ref: {error}"
    );
}

#[test]
fn adoc_review_returns_review_envelope_with_obligations_and_impact() {
    let fixture = build_v3_review_fixture("review-tool");
    let server = AgentDocMcpServer::new(fixture.root.clone());

    let value = server
        .run_review(AdocReviewParams {
            project_root: None,
            base_ref: "main".to_string(),
            head_ref: None,
            patch: None,
        })
        .expect("adoc_review runs");

    assert_eq!(value["schema_version"], "adoc.review.v0");
    assert_eq!(value["diff"]["schema_version"], "adoc.diff.v0");

    let impact = value["impact"].as_array().expect("impact array");
    assert!(
        impact.iter().any(|entry| entry["id"] == "billing.refunds"),
        "billing.refunds should be impacted because crates/billing/src/refund.rs changed; got {value}"
    );

    let reviewers = value["required_reviewers"]
        .as_array()
        .expect("required_reviewers array");
    assert!(
        reviewers
            .iter()
            .any(|entry| entry["owner"] == "team-billing"),
        "team-billing must be a required reviewer: {value}"
    );

    let obligations = value["proof_obligations"]
        .as_array()
        .expect("proof_obligations array");
    assert!(
        !obligations.is_empty(),
        "verified-claim body change + impacted source path should produce proof obligations: {value}"
    );
}

#[test]
fn adoc_review_accepts_explicit_head_ref() {
    let fixture = build_v3_review_fixture("review-head-ref");
    let server = AgentDocMcpServer::new(fixture.root.clone());

    let value = server
        .run_review(AdocReviewParams {
            project_root: None,
            base_ref: "main".to_string(),
            head_ref: Some("feature".to_string()),
            patch: None,
        })
        .expect("adoc_review with explicit head_ref runs");

    assert_eq!(value["schema_version"], "adoc.review.v0");
    assert!(
        value["diff"]["changed"]
            .as_array()
            .expect("changed array")
            .iter()
            .any(|entry| entry["id"] == "billing.refunds")
    );
}

#[test]
fn project_status_readiness_review_is_true_in_git_repo_with_head() {
    let fixture = build_v3_review_fixture("readiness-review-ok");
    let server = AgentDocMcpServer::new(fixture.root.clone());

    let value = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: None,
            no_embeddings: false,
        })
        .expect("status succeeds");

    assert_eq!(value["readiness"]["review"], true);
}

#[test]
fn project_status_readiness_review_is_false_in_non_git_dir() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    let value = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: None,
            no_embeddings: false,
        })
        .expect("status succeeds");

    assert_eq!(value["readiness"]["review"], false);
}

fn write_json_line(stdin: &mut impl Write, value: serde_json::Value) {
    writeln!(stdin, "{value}").expect("json request can be written");
    stdin.flush().expect("json request can be flushed");
}

fn read_json_line(stdout: &mut impl BufRead) -> serde_json::Value {
    let mut line = String::new();
    stdout
        .read_line(&mut line)
        .expect("json response can be read");
    assert!(!line.is_empty(), "expected json response line");
    serde_json::from_str(&line).expect("response is valid JSON")
}

// ---------------------------------------------------------------------------
// V6.4 TB4 — gated adoc_patch_apply
// ---------------------------------------------------------------------------

fn patch_apply_project(
    name: &str,
    mcp_block: &str,
) -> (tempfile::TempDir, AgentDocMcpServer, String) {
    let _ = name;
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/billing.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        &format!(
            "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: none\n{mcp_block}"
        ),
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: true,
        })
        .expect("build succeeds");
    let graph: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("dist/docs.graph.json")).unwrap())
            .expect("graph json parses");
    let base_hash = graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["id"] == "billing.credits")
        .expect("target node")["content_hash"]
        .as_str()
        .expect("content hash")
        .to_string();
    (workspace, server, base_hash)
}

fn inline_replace_body(base_hash: &str) -> PatchInput {
    PatchInput::Inline {
        patch: serde_json::json!({
            "schema_version": "adoc.patch.v0",
            "op": "replace_body",
            "target": "billing.credits",
            "base_hash": base_hash,
            "changes": { "body": "Credits apply after ledger commit." },
            "reason": "TB4 gate test."
        }),
    }
}

#[test]
fn patch_apply_tool_is_registered_even_when_the_gate_is_disabled() {
    let (_workspace, server, _base_hash) = patch_apply_project("registered", "");
    // The tool router is static: the tool exists regardless of project
    // config; only the call-time gate differs (ADR-0037).
    assert!(server.get_info().capabilities.tools.is_some());
    let refusal = server
        .run_patch_apply(adoc_mcp::AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_replace_body("sha256:any"),
        })
        .expect("gate refusal is a normal envelope, not a protocol error");
    assert_eq!(refusal["schema_version"], "adoc.patch.apply.v0");
}

#[test]
fn patch_apply_refuses_with_one_fix_oriented_diagnostic_when_disabled() {
    let (workspace, server, base_hash) = patch_apply_project("disabled", "");
    let original = fs::read_to_string(workspace.path().join("docs/billing.adoc")).expect("source");

    let refusal = server
        .run_patch_apply(adoc_mcp::AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_replace_body(&base_hash),
        })
        .expect("normal envelope");

    assert_eq!(refusal["applied"], false);
    assert_eq!(refusal["written_files"].as_array().map(Vec::len), Some(0));
    assert_eq!(refusal["trace"]["interface"], "mcp");
    let diagnostics = refusal["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(diagnostics.len(), 1, "exactly one diagnostic");
    assert_eq!(diagnostics[0]["code"], "mcp.patch_apply_disabled");
    let message = diagnostics[0]["message"].as_str().expect("message");
    assert!(
        message.contains("mcp: { patch_apply: enabled }"),
        "names the config key: {message}"
    );
    assert!(
        message.contains("adoc_patch_check"),
        "names the fallback: {message}"
    );

    assert_eq!(
        fs::read_to_string(workspace.path().join("docs/billing.adoc")).expect("source"),
        original,
        "disabled gate writes nothing"
    );
}

#[test]
fn patch_apply_applies_through_the_sandboxed_use_case_when_enabled() {
    let (workspace, server, base_hash) =
        patch_apply_project("enabled", "mcp:\n  patch_apply: enabled\n");

    let envelope = server
        .run_patch_apply(adoc_mcp::AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_replace_body(&base_hash),
        })
        .expect("apply runs");

    // Same use case as the CLI by construction; the envelope differs only in
    // trace.interface (slice acceptance).
    assert_eq!(envelope["schema_version"], "adoc.patch.apply.v0");
    assert_eq!(envelope["applied"], true);
    assert_eq!(envelope["check"]["valid"], true);
    assert_eq!(envelope["post_check"]["error_count"], 0);
    assert_eq!(envelope["artifacts_stale"], true);
    assert_eq!(envelope["trace"]["interface"], "mcp");

    let rewritten = fs::read_to_string(workspace.path().join("docs/billing.adoc")).expect("source");
    assert!(rewritten.contains("Credits apply after ledger commit."));
    assert_eq!(
        rewritten,
        source().replace(
            "Credits apply after payment.",
            "Credits apply after ledger commit."
        ),
        "formatting preserved outside the body span"
    );
}

#[test]
fn patch_apply_rejects_patch_paths_escaping_the_project_root() {
    let (_workspace, server, _base_hash) =
        patch_apply_project("sandbox", "mcp:\n  patch_apply: enabled\n");

    let error = server
        .run_patch_apply(adoc_mcp::AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: PatchInput::Path {
                patch_path: PathBuf::from("../outside-patch.json"),
            },
        })
        .expect_err("escape rejected");

    assert!(error.to_string().contains("path_outside_project"));
}

#[test]
fn propose_patch_v0_prompt_stays_byte_stable_and_v1_is_apply_aware() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    // ADR-0014: the v0 prompt and its unversioned alias are pinned. This
    // literal is a byte-for-byte copy of the published prompt text — any
    // edit to the const must fail here.
    const PINNED_V0_BODY: &str = "Use AgentDoc V2.2 patch validation before proposing source changes.\n\nWorkflow:\n1. Inspect readiness with adoc_project_status.\n2. Retrieve the target Object ID with adoc_why or adoc_search.\n3. Build a single-operation adoc.patch.v0 JSON proposal using replace_body, update_fields, create_object, supersede, or revoke; include reason and current base_hash when updating existing knowledge.\n4. Validate the inline patch with adoc_patch_check.\n5. Report validity, diagnostics, affected relations, diffs, and proof obligations.\n\nDo not apply patches, rewrite AgentDoc Source, approve knowledge, or create hosted review state.";

    let v0 = server
        .get_agent_prompt("adoc_propose_patch_v0", None)
        .expect("v0 prompt");
    let v0_text = serde_json::to_value(&v0).expect("serializes")["messages"][0]["content"]["text"]
        .as_str()
        .expect("text")
        .to_string();
    assert_eq!(
        v0_text, PINNED_V0_BODY,
        "adoc_propose_patch_v0 must stay byte-stable (ADR-0014)"
    );

    let alias = server
        .get_agent_prompt("adoc_propose_patch", None)
        .expect("alias prompt");
    assert_eq!(alias.messages, v0.messages, "unversioned alias stays on v0");

    let v1 = server
        .get_agent_prompt("adoc_propose_patch_v1", None)
        .expect("v1 prompt");
    let v1_text = serde_json::to_value(&v1).expect("serializes").to_string();
    assert!(v1_text.contains("adoc_patch_apply"));
    assert!(v1_text.contains("patch_apply_enabled"));
    assert!(v1_text.contains("post_check"));
    assert!(v1_text.contains("artifacts_stale"));
}

// ---------------------------------------------------------------------------
// V1.7.1 (ADR-0040): blended prose search through the MCP gateway.
// ---------------------------------------------------------------------------

fn mixed_mode_search_params(query: &str) -> SearchParams {
    SearchParams {
        project_root: None,
        query: query.to_string(),
        artifact: None,
        search_artifact: None,
        semantic: false,
        lexical: true,
        objects_only: false,
        prose_only: false,
        kind: None,
        status: None,
        owner: None,
        source_path: None,
        related_to: None,
        relation: None,
        direction: None,
        top: Some(10),
    }
}

/// Builds a mixed `.adoc` + `.md` project and returns its server.
fn mixed_mode_server() -> (tempfile::TempDir, AgentDocMcpServer) {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("docs/guides/onboarding.md"),
        "# Onboarding\n\nFollow the onboarding checklist before requesting credits.\n",
    );
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: none\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    let build = server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: true,
        })
        .expect("build succeeds");
    assert_eq!(build["exit_code"], 0, "build must be clean, got {build:#}");
    (workspace, server)
}

#[test]
fn adoc_search_blends_prose_records_into_v1_envelope() {
    let (_workspace, server) = mixed_mode_server();

    let envelope = server
        .run_search(mixed_mode_search_params("credits"))
        .expect("blended search succeeds")
        .structured_content
        .unwrap();

    assert_eq!(envelope["schema_version"], "adoc.retrieval.v1");
    let records = envelope["records"].as_array().expect("records array");
    assert!(
        records
            .iter()
            .any(|record| record["record_type"] == "knowledge_object"),
        "blended search returns the claim, got {envelope:#}"
    );
    let prose = records
        .iter()
        .find(|record| record["record_type"] == "prose")
        .expect("blended search returns the .md paragraph");
    assert_eq!(prose["page_id"], "guides.onboarding");
    assert_eq!(prose["heading_context"], "Onboarding");

    let mut objects_only = mixed_mode_search_params("credits");
    objects_only.objects_only = true;
    let envelope = server
        .run_search(objects_only)
        .expect("objects-only search succeeds")
        .structured_content
        .unwrap();
    assert!(
        envelope["records"]
            .as_array()
            .expect("records array")
            .iter()
            .all(|record| record["record_type"] == "knowledge_object"),
        "objects_only restricts to Knowledge Objects"
    );

    let mut prose_only = mixed_mode_search_params("credits");
    prose_only.prose_only = true;
    let envelope = server
        .run_search(prose_only)
        .expect("prose-only search succeeds")
        .structured_content
        .unwrap();
    assert!(
        envelope["records"]
            .as_array()
            .expect("records array")
            .iter()
            .all(|record| record["record_type"] == "prose"),
        "prose_only restricts to prose records"
    );
}

#[test]
fn adoc_search_rejects_conflicting_scope_arguments() {
    let (_workspace, server) = mixed_mode_server();

    let mut both = mixed_mode_search_params("credits");
    both.objects_only = true;
    both.prose_only = true;
    let error = server.run_search(both).expect_err("both scopes conflict");
    assert!(error.to_string().contains("mutually exclusive"));

    // V1.7.2: prose_only + semantic is no longer an argument conflict —
    // prose vectors ship in adoc.search.v2. On this fixture (built with
    // provider: none) it fails later, at the missing search artifact.
    let mut prose_semantic = mixed_mode_search_params("credits");
    prose_semantic.prose_only = true;
    prose_semantic.semantic = true;
    prose_semantic.lexical = false;
    let envelope = server
        .run_search(prose_semantic)
        .expect("prose_only + semantic passes argument validation")
        .structured_content
        .unwrap();
    assert!(
        envelope["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .any(|diagnostic| diagnostic["code"] == "search.artifact_missing"),
        "expected the missing-artifact diagnostic, not an argument conflict: {envelope:#}"
    );
    assert_eq!(envelope["records"], serde_json::json!([]));

    let mut prose_filtered = mixed_mode_search_params("credits");
    prose_filtered.prose_only = true;
    prose_filtered.kind = Some("claim".to_string());
    let error = server
        .run_search(prose_filtered)
        .expect_err("prose_only + kind filter conflicts");
    assert!(error.to_string().contains("metadata filters"));

    // Graph traversal arguments are Knowledge-Object-only too; the CLI blocks
    // them transitively (`relation`/`direction` require `related_to`), and the
    // MCP adapter must not be looser.
    let mut prose_relation = mixed_mode_search_params("credits");
    prose_relation.prose_only = true;
    prose_relation.relation = Some("depends_on".to_string());
    let error = server
        .run_search(prose_relation)
        .expect_err("prose_only + relation conflicts");
    assert!(error.to_string().contains("metadata filters"));

    let mut prose_direction = mixed_mode_search_params("credits");
    prose_direction.prose_only = true;
    prose_direction.direction = Some("down".to_string());
    let error = server
        .run_search(prose_direction)
        .expect_err("prose_only + direction conflicts");
    assert!(error.to_string().contains("metadata filters"));
}

#[test]
fn sensitive_retrieval_requires_recording_context() {
    for class in ["internal", "restricted"] {
        let workspace = tempfile::tempdir().unwrap();
        write(
            &workspace.path().join("agentdoc.config.yaml"),
            "version: 1\nmode: strict\ndocs_path: docs\nretrieval_policy:\n  audience: restricted\n  allowed_visibilities: [public, internal, restricted]\n",
        );
        write(
            &workspace.path().join("docs/index.adoc"),
            &source().replace(
                "status: draft",
                &format!("status: draft\nvisibility: {class}"),
            ),
        );
        let server = AgentDocMcpServer::new(workspace.path().to_path_buf()).with_retrieval_policy(
            serde_json::from_value(serde_json::json!({"audience":"restricted", "allowed_visibilities":["public", "internal", "restricted"], "excluded_object_ids":[]})).unwrap()
        );
        assert_eq!(
            server
                .run_build(BuildParams {
                    project_root: None,
                    path: Some("docs".into()),
                    out: Some("dist".into()),
                    no_embeddings: true
                })
                .unwrap()["ok"],
            true
        );
        let local = adoc_local::LocalContext::new(
            workspace.path().to_path_buf(),
            ProjectRootPathPolicy::new(workspace.path()).unwrap(),
        )
        .why(adoc_local::WhyInput {
            object_id: "billing.credits".into(),
            artifact: Some("dist/docs.graph.json".into()),
        })
        .unwrap();
        assert_eq!(
            serde_json::to_value(&local.records[0].record).unwrap()["classification"],
            class
        );
        let error = server
            .run_why(WhyParams {
                project_root: None,
                object_id: "billing.credits".into(),
                artifact: Some("dist/docs.graph.json".into()),
            })
            .expect_err("sensitive MCP output needs trusted recording context");
        assert!(
            error
                .to_string()
                .contains("retrieval.audit_sink_unavailable")
        );
        assert!(!error.to_string().contains("Credits apply"));
    }
}
