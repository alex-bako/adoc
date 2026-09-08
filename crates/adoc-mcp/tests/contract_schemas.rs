use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use adoc_core::{
    GitRef, ObjectDiffEnvelope, ReviewEnvelope, ReviewInput, SnapshotSelector, diff_objects,
    load_review_from_git, load_review_with_changed_files_from_git, parse_patch_from_value,
    review_with_patch,
};
use adoc_local::{AssessmentInput, LocalContext, RepositoryBaselineInput, UnrestrictedPathPolicy};
use adoc_mcp::{
    AdocPatchCheckParams, AdocReviewParams, AgentDocMcpServer, BuildParams, ContradictionsParams,
    GraphParams, ImpactedByParams, InitParams, PatchInput, ProjectStatusParams, SearchParams,
    StaleParams,
};
use serde_json::json;

const CANONICAL_SOURCE_ACL_OBSERVED_AT: &str = "2026-08-23T11:59:00Z";

#[test]
fn withheld_retrieval_source_remains_schema_compatible() {
    let envelope = json!({"schema_version":"adoc.retrieval.v1","records":[{
        "record_type":"knowledge_object","id":"billing.selected","kind":"claim",
        "content_hash":format!("sha256:{}","a".repeat(64)),"body":"Approved body.","source":{},
        "relations":{"depends_on":[],"supersedes":[],"related_to":[]}}],"diagnostics":[]});
    assert_valid("retrieval-envelope.json", &envelope);
}

#[test]
fn managed_field_declassification_schema_matches_domain() {
    let id = "00000000-0000-4000-8000-000000000001";
    let digest = format!("sha256:{}", "a".repeat(64));
    let value = json!({"schema_version":"adoc.managed_field_declassification.v0",
        "request_id":id,"workspace_id":id,"canonical_id":id,"version_id":id,
        "content_digest":digest,"provenance_record_digest":digest,"state_event_ordinal":0,
        "state_event_digest":digest,"fields":[{"selector":"/body","prior_classification":"restricted",
        "new_classification":"public","assertion_ids":[id]}],"principal_id":id,"auth_session_id":id,
        "authorization_decision_id":id,"policy_version":"native-policy-v1","rationale":"Approved field release.",
        "effective_date":"2028-02-29","restricted_evidence_remains_hidden":true});
    let name = "adoc.managed_field_declassification.v0.schema.json";
    assert_valid(name, &value);
    assert!(
        adoc_core::validate_managed_field_declassification(&serde_json::to_vec(&value).unwrap())
            .is_ok()
    );
    for (key, bad) in [
        ("state_event_ordinal", json!(-1)),
        ("state_event_ordinal", json!(9007199254740992u64)),
        ("restricted_evidence_remains_hidden", json!(false)),
        ("effective_date", json!("2026-02-29")),
        ("rationale", json!(" ")),
        ("rationale", json!("é".repeat(4097))),
        ("policy_version", json!(" p ")),
        ("foreign", json!(true)),
        ("fields", json!([])),
    ] {
        let mut invalid = value.clone();
        invalid[key] = bad;
        assert!(!schema_accepts(name, &invalid));
        assert!(
            adoc_core::validate_managed_field_declassification(
                &serde_json::to_vec(&invalid).unwrap()
            )
            .is_err(),
            "{key}"
        );
    }
    for bad in [
        json!(["/body", "restricted", "public", [id]]),
        json!({"selector":"/body","prior_classification":"public","new_classification":"public","assertion_ids":[id]}),
        json!({"selector":"/body","prior_classification":"internal","new_classification":"restricted","assertion_ids":[id]}),
    ] {
        let mut invalid = value.clone();
        invalid["fields"][0] = bad;
        assert!(!schema_accepts(name, &invalid));
    }
}

#[test]
fn managed_retrieval_input_is_closed_and_requires_exact_byte_bindings() {
    let name = "adoc.managed_retrieval_input.v0.schema.json";
    let valid = json!({
        "schema_version": "adoc.managed_retrieval_input.v0",
        "workspace_id": "workspace-1",
        "policy": {"audience":"public", "allowed_visibilities":["public"], "excluded_object_ids":[]},
        "receipts": [{"id":"receipt-1", "workspace_id":"workspace-1", "graph_bytes":"{}", "graph_digest":format!("sha256:{}", "a".repeat(64))}],
        "objects": [{"canonical":{"workspace_id":"workspace-1", "canonical_id":"object-1"}, "version_id":"version-1", "object_id":"billing.credit", "receipt_id":"receipt-1", "content_bytes":"{}", "content_digest":format!("sha256:{}", "b".repeat(64))}]
    });
    let mut projected = valid.clone();
    projected["objects"][0]["field_projection"] = json!({
        "workspace_id":"00000000-0000-4000-8000-000000000001",
        "canonical_id":"00000000-0000-4000-8000-000000000002",
        "version_id":"00000000-0000-4000-8000-000000000003",
        "content_digest":format!("sha256:{}", "b".repeat(64)),
        "fields":[{"selector":"/fields/owner", "classification":null}]
    });
    assert_valid(name, &projected);
    let mut approved = projected.clone();
    approved["objects"][0]["field_projection"]["fields"][0]["classification"] = json!("public");
    approved["objects"][0]["field_projection"]["fields"][0]["declassification"] = json!({
        "state_event_ordinal":0,"state_event_digest":format!("sha256:{}","a".repeat(64)),
        "detail_digest":format!("sha256:{}","b".repeat(64)),"prior_classification":"internal"});
    assert_valid(name, &approved);
    for (pointer, bad) in [
        ("/declassification", json!(null)),
        ("/declassification", json!([])),
        ("/declassification/state_event_ordinal", json!(-1)),
        ("/declassification/detail_digest", json!("bad")),
        ("/declassification/prior_classification", json!("public")),
        ("/classification", json!(null)),
        ("/classification", json!("internal")),
    ] {
        let mut invalid = approved.clone();
        *invalid["objects"][0]["field_projection"]["fields"][0]
            .pointer_mut(pointer)
            .unwrap() = bad;
        assert!(!schema_accepts(name, &invalid), "{pointer}");
    }

    let mut accepted_unknown_keys = Vec::new();
    for (key, value) in [
        ("uuid", json!("00000000-0000-4000-8000-000000000001")),
        (
            "fieldProjection",
            projected["objects"][0]["field_projection"].clone(),
        ),
    ] {
        let mut invalid = valid.clone();
        invalid["objects"][0][key] = value;
        if schema_accepts(name, &invalid) {
            accepted_unknown_keys.push(key);
        }
    }
    assert!(
        accepted_unknown_keys.is_empty(),
        "accepted unsupported object keys: {accepted_unknown_keys:?}"
    );
    for (pointer, value) in [
        ("", json!(null)),
        ("", json!([])),
        ("/fields", json!([])),
        ("/fields", json!([["/body", null]])),
        ("/fields/0/selector", json!("/status")),
        ("/fields/0/classification", json!("secret")),
    ] {
        let mut invalid = projected.clone();
        *invalid["objects"][0]["field_projection"]
            .pointer_mut(pointer)
            .unwrap() = value;
        assert!(!schema_accepts(name, &invalid), "projection {pointer}");
    }
    let mut missing = projected.clone();
    missing["objects"][0]["field_projection"]["fields"][0]
        .as_object_mut()
        .unwrap()
        .remove("classification");
    assert!(!schema_accepts(name, &missing));
    // Structural admission only; core separately verifies JSON, digests and joins.
    assert_valid(name, &valid);
    for pointer in [
        "/objects/0/content_digest",
        "/receipts/0/graph_digest",
        "/objects/0/canonical/workspace_id",
        "/policy/audience",
    ] {
        let mut invalid = valid.clone();
        *invalid.pointer_mut(pointer).unwrap() = json!(null);
        assert!(!schema_accepts(name, &invalid), "accepted null {pointer}");
    }
    for pointer in [
        "",
        "/policy",
        "/objects/0",
        "/objects/0/canonical",
        "/receipts/0",
    ] {
        let mut invalid = valid.clone();
        invalid
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("authority".into(), json!("caller-asserted"));
        assert!(
            !schema_accepts(name, &invalid),
            "accepted extra key {pointer}"
        );
    }
    let mut invalid = valid.clone();
    invalid["schema_version"] = json!("unregistered version");
    assert!(!schema_accepts(name, &invalid));
    invalid = valid;
    invalid["objects"][0]["object_id"] = json!("INVALID");
    assert!(!schema_accepts(name, &invalid));
}
const CANONICAL_SOURCE_ACL_EXPIRED_AT: &str = "2026-08-23T11:59:30Z";
const CANONICAL_EVALUATION_TIME: &str = "2026-08-23T12:00:00Z";
const CANONICAL_SOURCE_ACL_EXPIRES_AT: &str = "2026-08-23T12:04:00Z";

fn canonical_source_acl_join() -> serde_json::Value {
    json!({
        "snapshot_id": "acl-1",
        "workspace_id": "workspace-1",
        "connector_id": "github",
        "source_container_id": "agentdoc-dev",
        "source": { "kind": "repository", "id": "cloud" },
        "acl_policy_version": "github-acl-v1",
        "observed_at": CANONICAL_SOURCE_ACL_OBSERVED_AT
    })
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent directory can be created");
    }
    fs::write(path, contents).expect("file can be written");
}

fn source() -> &'static str {
    "# Billing @doc(team.billing)\n\n::claim billing.ready\nstatus: draft\n--\nBilling docs are ready.\n::\n"
}

fn schema(name: &str) -> serde_json::Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/agent/v0/schema")
        .join(name);
    serde_json::from_str(&fs::read_to_string(path).expect("schema is readable"))
        .expect("schema is json")
}

struct AgentSchemaRetriever;

impl jsonschema::Retrieve for AgentSchemaRetriever {
    fn retrieve(
        &self,
        uri: &jsonschema::Uri<String>,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
        let name = uri
            .as_str()
            .strip_prefix("adoc://agent/v0/schema/")
            .filter(|name| !name.contains('/'))
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, uri.as_str().to_string()))?;
        Ok(schema(name))
    }
}

fn validator_for(schema: &serde_json::Value) -> jsonschema::Validator {
    jsonschema::options()
        .with_retriever(AgentSchemaRetriever)
        .build(schema)
        .expect("schema compiles")
}

fn assert_valid(schema_name: &str, instance: &serde_json::Value) {
    let schema = schema(schema_name);
    let validator = validator_for(&schema);
    let errors = validator
        .iter_errors(instance)
        .map(|error| error.to_string())
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "{schema_name} validation failed:\n{}\ninstance:\n{}",
        errors.join("\n"),
        serde_json::to_string_pretty(instance).expect("instance pretty prints")
    );
}

fn schema_accepts(schema_name: &str, instance: &serde_json::Value) -> bool {
    let schema = schema(schema_name);
    validator_for(&schema).is_valid(instance)
}

fn cloud_egress_policy_fixture() -> serde_json::Value {
    json!({
        "schema_version": "agentdoc.cloud.egress_policy.v0",
        "payload": {
            "scope": {
                "workspace_id": "10000000-0000-0000-0000-000000000001",
                "resource": {"kind": "repository", "id": "30000000-0000-0000-0000-000000000001"}
            },
            "categories": {
                "raw_source": false, "source_excerpts": false, "pr_diffs": false,
                "compiled_objects": false, "embeddings": false,
                "semantic_assessments": false, "audit_metadata": false
            }
        }
    })
}

#[test]
fn cloud_egress_policy_requires_exactly_seven_explicit_boolean_categories() {
    let name = "agentdoc.cloud.egress_policy.v0.schema.json";
    let policy = cloud_egress_policy_fixture();
    assert_valid(name, &policy);
    let expected: BTreeSet<_> = policy["payload"]["categories"]
        .as_object()
        .expect("categories")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        expected.len(),
        7,
        "the category vocabulary is closed at seven"
    );
    let schema = schema(name);
    let categories = &schema["properties"]["payload"]["properties"]["categories"];
    let required: BTreeSet<_> = categories["required"]
        .as_array()
        .expect("required categories")
        .iter()
        .map(|key| key.as_str().expect("category name"))
        .collect();
    let properties: BTreeSet<_> = categories["properties"]
        .as_object()
        .expect("category properties")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(required, expected, "required category vocabulary drifted");
    assert_eq!(properties, expected, "optional category vocabulary widened");
    let mut unknown = policy.clone();
    unknown["payload"]["categories"]["future_category"] = json!(false);
    assert!(
        !schema_accepts(name, &unknown),
        "unknown egress category accepted"
    );

    for key in policy["payload"]["categories"]
        .as_object()
        .expect("categories")
        .keys()
    {
        let mut enabled = policy.clone();
        enabled["payload"]["categories"][key] = json!(true);
        assert_valid(name, &enabled);
        let mut missing = policy.clone();
        missing["payload"]["categories"]
            .as_object_mut()
            .expect("categories")
            .remove(key);
        assert!(!schema_accepts(name, &missing), "missing {key} accepted");
        for value in [json!(null), json!("false"), json!(0), json!([]), json!({})] {
            let mut invalid = policy.clone();
            invalid["payload"]["categories"][key] = value;
            assert!(!schema_accepts(name, &invalid), "nonboolean {key} accepted");
        }
    }
}

#[test]
fn cloud_egress_policy_closes_the_wrapper_and_scope_binding() {
    let name = "agentdoc.cloud.egress_policy.v0.schema.json";
    let policy = cloud_egress_policy_fixture();
    for pointer in ["", "/payload", "/payload/scope", "/payload/scope/resource"] {
        let mut extra = policy.clone();
        extra.pointer_mut(pointer).expect("object")["extra"] = json!(true);
        assert!(
            !schema_accepts(name, &extra),
            "extra field at {pointer:?} accepted"
        );
    }
    for pointer in [
        "/schema_version",
        "/payload",
        "/payload/scope",
        "/payload/scope/workspace_id",
        "/payload/scope/resource",
        "/payload/categories",
    ] {
        let mut invalid = policy.clone();
        *invalid.pointer_mut(pointer).expect("field") = json!(null);
        assert!(!schema_accepts(name, &invalid), "null {pointer} accepted");
        let (parent, key) = pointer.rsplit_once('/').expect("field pointer");
        let mut missing = policy.clone();
        missing
            .pointer_mut(parent)
            .expect("parent object")
            .as_object_mut()
            .expect("object")
            .remove(key);
        assert!(
            !schema_accepts(name, &missing),
            "missing {pointer} accepted"
        );
    }
    for repository in [
        "",
        "repository-1",
        "ABCDEFAB-0000-0000-0000-000000000001",
        "30000000-0000-0000-0000-000000000001\n",
    ] {
        let mut invalid = policy.clone();
        invalid["payload"]["scope"]["resource"]["id"] = json!(repository);
        assert!(
            !schema_accepts(name, &invalid),
            "malformed repository id {repository:?} accepted"
        );
    }
    let mut version = policy;
    version["schema_version"] = json!("agentdoc.cloud.egress_policy.v99");
    assert!(
        !schema_accepts(name, &version),
        "unsupported policy version accepted"
    );
}

#[test]
fn cloud_egress_policy_requires_canonical_workspace_ids_in_both_scopes() {
    let name = "agentdoc.cloud.egress_policy.v0.schema.json";
    let mut policy = cloud_egress_policy_fixture();
    for scope in [
        policy["payload"]["scope"].clone(),
        json!({
            "workspace_id": "10000000-0000-0000-0000-000000000001",
            "connector_id": "40000000-0000-0000-0000-000000000001"
        }),
    ] {
        policy["payload"]["scope"] = scope.clone();
        assert_valid(name, &policy);
        for workspace in [
            "not-a-uuid",
            "ABCDEFAB-0000-0000-0000-000000000001",
            "10000000-0000-0000-0000-000000000001\n",
        ] {
            policy["payload"]["scope"]["workspace_id"] = json!(workspace);
            assert!(
                !schema_accepts(name, &policy),
                "malformed workspace id {workspace:?} accepted for scope {scope}"
            );
        }
    }
}

#[test]
fn cloud_egress_policy_preserves_connector_and_native_source_scopes() {
    let name = "agentdoc.cloud.egress_policy.v0.schema.json";
    let kinds = ["repository", "project", "space", "channel"];
    let authorization = schema("adoc.authorization_decision.v0.schema.json");
    let inherited_kinds: BTreeSet<_> =
        authorization["$defs"]["sourceResource"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("source resource kinds")
            .iter()
            .map(|kind| kind.as_str().expect("source resource kind"))
            .collect();
    assert_eq!(
        inherited_kinds,
        BTreeSet::from(kinds),
        "inherited egress source-resource vocabulary drifted"
    );
    let connector = json!({
        "workspace_id": "10000000-0000-0000-0000-000000000001",
        "connector_id": "40000000-0000-0000-0000-000000000001"
    });
    let mut policy = cloud_egress_policy_fixture();
    policy["payload"]["scope"] = connector.clone();
    assert_valid(name, &policy);
    let mut container = connector.clone();
    container["source_container_id"] = json!("provider-account");
    policy["payload"]["scope"] = container.clone();
    assert_valid(name, &policy);
    for kind in kinds {
        let mut source = container.clone();
        source["resource"] = json!({"kind": kind, "id": "native-source-id"});
        policy["payload"]["scope"] = source;
        assert_valid(name, &policy);
    }
    let no_container = json!({
        "workspace_id": connector["workspace_id"],
        "connector_id": connector["connector_id"],
        "resource": {"kind": "channel", "id": "native-source-id"}
    });
    let invalid_scopes = [
        json!({}),
        json!({"workspace_id": connector["workspace_id"]}),
        json!({"workspace_id": connector["workspace_id"], "source_container_id": "unbound"}),
        json!({"workspace_id": connector["workspace_id"], "connector_id": "github"}),
        json!({"workspace_id": connector["workspace_id"], "connector_id": connector["connector_id"], "source_container_id": " padded "}),
        json!({"workspace_id": connector["workspace_id"], "connector_id": connector["connector_id"], "knowledge_kind": "claim"}),
        json!({"workspace_id": connector["workspace_id"], "connector_id": connector["connector_id"], "source_container_id": "account", "resource": {"kind":"unknown", "id":"native"}}),
        no_container,
    ];
    for scope in invalid_scopes {
        policy["payload"]["scope"] = scope.clone();
        assert!(
            !schema_accepts(name, &policy),
            "invalid scope accepted: {scope}"
        );
    }
}

#[test]
fn cloud_operation_contracts_round_trip_and_reject_the_registered_unknown_version() {
    for id in [
        "agentdoc.cloud.assessment_submission.v0",
        "agentdoc.cloud.ingestion_result.v0",
        "agentdoc.cloud.repository_config.v0",
        "agentdoc.cloud.work_request.v0",
        "agentdoc.cloud.work_result.v0",
        "agentdoc.cloud.gate_decision.v0",
        "agentdoc.cloud.proposal_command.v0",
        "agentdoc.cloud.approval_command.v0",
        "agentdoc.cloud.migration_request.v0",
        "agentdoc.cloud.migration_receipt.v0",
        "agentdoc.cloud.egress_policy.v0",
    ] {
        let file = format!("{id}.schema.json");
        let payload = if id == "agentdoc.cloud.gate_decision.v0" {
            json!({
                "schema_version": "adoc.gate_result.v0",
                "head_sha": "0123456789abcdef0123456789abcdef01234567",
                "policy_version": "gate-policy-v1",
                "input_digests": [],
                "effective_mode": "advisory",
                "result": "pass",
                "reasons": []
            })
        } else if id == "agentdoc.cloud.egress_policy.v0" {
            cloud_egress_policy_fixture()["payload"].clone()
        } else {
            json!({ "fixture": "round-trip" })
        };
        let fixture = json!({ "schema_version": id, "payload": payload });
        let bytes = serde_json::to_vec(&fixture).expect("fixture serializes");
        let round_trip: serde_json::Value =
            serde_json::from_slice(&bytes).expect("fixture deserializes");

        assert_eq!(round_trip, fixture, "{id} changed across JSON round-trip");
        assert_valid(&file, &round_trip);
    }

    let unsupported = json!({
        "schema_version": "agentdoc.cloud.assessment_submission.v99",
        "payload": { "fixture": "rejected-version" }
    });
    assert!(!schema_accepts(
        "agentdoc.cloud.assessment_submission.v0.schema.json",
        &unsupported
    ));
}

#[test]
fn cloud_gate_decision_wraps_the_shared_gate_result_contract() {
    let gate = schema("agentdoc.cloud.gate_decision.v0.schema.json");
    assert_eq!(
        gate.pointer("/properties/payload/$ref"),
        Some(&json!(
            "adoc://agent/v0/schema/adoc.gate_result.v0.schema.json"
        ))
    );

    let contradictory = json!({
        "schema_version": "agentdoc.cloud.gate_decision.v0",
        "payload": {
            "schema_version": "adoc.gate_result.v0",
            "head_sha": "0123456789abcdef0123456789abcdef01234567",
            "policy_version": "gate-policy-v1",
            "input_digests": [],
            "effective_mode": "advisory",
            "result": "pass",
            "reasons": ["gate.assessment_missing"]
        }
    });
    assert!(
        !validator_for(&gate).is_valid(&contradictory),
        "the transport must enforce its registered gate-result dependency"
    );
}

#[test]
fn connector_capability_manifest_schema_is_closed_and_version_exact() {
    let manifest = json!({
        "schema_version": "agentdoc.connector_capabilities.v0",
        "adapter": { "name": "github-action", "version": "2.0.0-alpha.19" },
        "publisher": { "id": "agentdoc-dev/action", "kind": "agentdoc" },
        "overall_stage": "Beta",
        "capabilities": {
          "change_request.trusted_assessment": {
            "version": "1",
            "maturity": "ga",
            "dependencies": [{
                "name": "source.read_exact_revision",
                "version_range": ">=1 <2"
            }],
            "known_limitations": ["GitHub repositories only"],
            "supported_contract_ranges": [{
                "schema": "agentdoc.cloud.assessment_submission",
                "version_range": ">=0 <1"
            }],
            "processing_modes": ["source_ci"],
            "deployment_modes": ["github_action"],
            "qualification_evidence_ref": "agentdoc:evidence/github-action/trusted-assessment-v1"
          }
        }
    });
    let name = "agentdoc.connector_capabilities.v0.schema.json";

    assert_eq!(
        manifest["schema_version"],
        adoc_core::CONNECTOR_CAPABILITIES_SCHEMA_VERSION
    );
    assert_valid(name, &manifest);

    let mut unknown_version = manifest.clone();
    unknown_version["schema_version"] = json!("agentdoc.connector_capabilities.v1");
    assert!(!schema_accepts(name, &unknown_version));

    let mut unknown_maturity = manifest.clone();
    unknown_maturity["capabilities"]["change_request.trusted_assessment"]["maturity"] =
        json!("stable");
    assert!(!schema_accepts(name, &unknown_maturity));

    let mut unevidenced_ga = manifest.clone();
    unevidenced_ga["capabilities"]["change_request.trusted_assessment"]
        .as_object_mut()
        .expect("capability is an object")
        .remove("qualification_evidence_ref");
    assert!(!schema_accepts(name, &unevidenced_ga));

    let mut duplicate_identity = manifest.clone();
    let mut conflicting =
        duplicate_identity["capabilities"]["change_request.trusted_assessment"].clone();
    conflicting["maturity"] = json!("deprecated");
    duplicate_identity["capabilities"] = json!([
        {
            "name": "change_request.trusted_assessment",
            "version": "1",
            "maturity": "ga"
        },
        conflicting
    ]);
    assert!(!schema_accepts(name, &duplicate_identity));

    let mut unknown_field = manifest;
    unknown_field["capabilities"]["change_request.trusted_assessment"]["silently_weaken_gate"] =
        json!(true);
    assert!(!schema_accepts(name, &unknown_field));
}

fn project_with_built_graph() -> (tempfile::TempDir, AgentDocMcpServer, String) {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    // V1.7.1: a .md page contributes prose blocks so retrieval-envelope
    // validation covers both record types.
    write(
        &root.join("docs/guides/onboarding.md"),
        "# Onboarding\n\nBilling onboarding starts with a sandbox workspace.\n",
    );
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: false,
        })
        .expect("build succeeds");
    let graph: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("dist/docs.graph.json")).unwrap())
            .expect("graph json parses");
    let base_hash = graph["nodes"]
        .as_array()
        .expect("nodes")
        .iter()
        .find(|node| node["id"] == "billing.ready")
        .expect("target node")["content_hash"]
        .as_str()
        .expect("content hash")
        .to_string();
    (workspace, server, base_hash)
}

#[test]
fn validates_representative_serialized_agent_envelopes_against_contract_schemas() {
    let (workspace, server, base_hash) = project_with_built_graph();

    let graph_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workspace.path().join("dist/docs.graph.json"))
            .expect("graph artifact reads"),
    )
    .expect("graph artifact parses");
    assert_valid("graph-artifact.v6.json", &graph_artifact);

    let retrieval = server
        .run_search(SearchParams {
            project_root: None,
            query: "billing".to_string(),
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
        .expect("search succeeds")
        .structured_content
        .unwrap();
    assert_valid("retrieval-envelope.json", &retrieval);

    let graph = server
        .run_graph(GraphParams {
            project_root: None,
            object_id: "billing.ready".to_string(),
            artifact: None,
            relation: None,
            direction: None,
        })
        .expect("graph succeeds")
        .structured_content
        .unwrap();
    assert_valid("graph-traversal-envelope.json", &graph);

    for patch in [
        json!({
            "schema_version": "adoc.patch.v0",
            "op": "replace_body",
            "target": "billing.ready",
            "base_hash": base_hash.clone(),
            "changes": { "body": "Billing docs are ready after review." },
            "reason": "Update body."
        }),
        json!({
            "schema_version": "adoc.patch.v0",
            "op": "update_fields",
            "target": "billing.ready",
            "base_hash": base_hash.clone(),
            "changes": { "fields": { "owner": "team-billing" } },
            "reason": "Set owner."
        }),
        json!({
            "schema_version": "adoc.patch.v0",
            "op": "create_object",
            "target": "billing.created",
            "changes": {
                "kind": "claim",
                "status": "draft",
                "body": "Created claim.",
                "fields": {},
                "placement": { "page_id": "team.billing", "after": "billing.ready" }
            },
            "reason": "Create follow-up claim."
        }),
        json!({
            "schema_version": "adoc.patch.v0",
            "op": "supersede",
            "target": "billing.ready",
            "base_hash": base_hash.clone(),
            "changes": { "supersedes": ["billing.created"] },
            "reason": "Record supersession."
        }),
        json!({
            "schema_version": "adoc.patch.v0",
            "op": "revoke",
            "target": "billing.ready",
            "base_hash": base_hash.clone(),
            "changes": {},
            "reason": "Revoke stale claim."
        }),
    ] {
        assert_valid("patch-input.json", &patch);
    }

    let patch_check = server
        .run_patch_check(AdocPatchCheckParams {
            project_root: None,
            artifact: None,
            input: PatchInput::Inline {
                patch: json!({
                    "schema_version": "adoc.patch.v0",
                    "op": "replace_body",
                    "target": "billing.ready",
                    "base_hash": base_hash,
                    "changes": { "body": "Billing docs are ready after review." },
                    "reason": "Update body."
                }),
            },
        })
        .expect("patch check succeeds");
    assert_valid("patch-check.json", &patch_check);

    let project_status = server
        .run_project_status(ProjectStatusParams {
            project_root: None,
            refresh: Some("none".to_string()),
            no_embeddings: false,
        })
        .expect("project status succeeds");
    assert_valid("project-status.json", &project_status);

    // An empty-records stale envelope is also a contract case: the fixture
    // project has no expiry or review fields at all.
    let stale = server
        .run_stale(StaleParams {
            project_root: None,
            artifact: None,
            within_days: None,
        })
        .expect("stale succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.stale.v0.schema.json", &stale);
    assert_eq!(stale["records"], serde_json::json!([]));

    // Likewise the empty-lists contradictions envelope: the fixture project
    // has no contradiction objects at all.
    let contradictions = server
        .run_contradictions(ContradictionsParams {
            project_root: None,
            artifact: None,
            all: false,
        })
        .expect("contradictions succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.contradictions.v0.schema.json", &contradictions);
    assert_eq!(contradictions["contradictions"], serde_json::json!([]));
    assert_eq!(contradictions["contradicted_claims"], serde_json::json!([]));
}

#[test]
fn patch_input_schema_matches_the_public_parser_structural_contract() {
    let replace = json!({
        "schema_version": "adoc.patch.v0",
        "op": "replace_body",
        "target": "billing.ready",
        "base_hash": "sha256:content",
        "changes": { "body": "Updated body." },
        "reason": "Update body."
    });
    let create = json!({
        "schema_version": "adoc.patch.v0",
        "op": "create_object",
        "target": "billing.created",
        "changes": {
            "kind": "claim",
            "status": "draft",
            "body": "Created claim.",
            "fields": { "owner": "team-billing" },
            "placement": { "page_id": "team.billing", "after": "billing.ready" }
        },
        "reason": "Create claim.",
        "proposer": { "type": "agent", "id": "agentdoc-action" }
    });
    let update = json!({
        "schema_version": "adoc.patch.v0",
        "op": "update_fields",
        "target": "billing.ready",
        "base_hash": "sha256:content",
        "changes": { "fields": { "owner": "team-billing" } },
        "reason": "Set owner."
    });
    let supersede = json!({
        "schema_version": "adoc.patch.v0",
        "op": "supersede",
        "target": "billing.ready",
        "base_hash": "sha256:content",
        "changes": { "supersedes": ["billing.old"] },
        "reason": "Record supersession."
    });
    let revoke = json!({
        "schema_version": "adoc.patch.v0",
        "op": "revoke",
        "target": "billing.ready",
        "base_hash": "sha256:content",
        "changes": {},
        "reason": "Revoke stale knowledge."
    });

    let mut cases = vec![
        ("replace_body", replace.clone()),
        ("create_object", create.clone()),
        ("update_fields", update.clone()),
        ("supersede", supersede.clone()),
        ("revoke", revoke.clone()),
    ];
    let mut invalid = |name: &'static str, mut value: serde_json::Value| {
        cases.push((name, value.take()));
    };

    let mut value = replace.clone();
    value.as_object_mut().expect("object").remove("base_hash");
    invalid("replace_body missing base_hash", value);
    let mut value = create.clone();
    value["base_hash"] = json!("sha256:forbidden");
    invalid("create_object with base_hash", value);
    let mut value = replace.clone();
    value["unexpected"] = json!(true);
    invalid("unknown top-level member", value);
    let mut value = replace.clone();
    value["changes"]["status"] = json!("verified");
    invalid("operation-incompatible changes member", value);
    let mut value = create.clone();
    value["changes"]["placement"]["unexpected"] = json!(true);
    invalid("unknown placement member", value);
    let mut value = create.clone();
    value["changes"]["placement"]
        .as_object_mut()
        .expect("placement object")
        .remove("page_id");
    invalid("placement missing page_id", value);
    let mut value = create.clone();
    value["proposer"]["unexpected"] = json!(true);
    invalid("unknown proposer member", value);
    let mut value = update.clone();
    value["changes"]["fields"] = json!({});
    invalid("empty update_fields map", value);
    let mut value = update;
    value["changes"]["fields"]["owner"] = json!(7);
    invalid("non-string metadata value", value);
    let mut value = supersede;
    value["changes"]["supersedes"] = json!([]);
    invalid("empty supersedes list", value);
    let mut value = revoke;
    value["changes"]["body"] = json!("not accepted");
    invalid("revoke changes member", value);

    for (name, patch) in cases {
        let parser_accepts = parse_patch_from_value(patch.clone()).is_ok();
        let contract_accepts = schema_accepts("patch-input.json", &patch);
        assert_eq!(
            contract_accepts,
            parser_accepts,
            "schema/parser mismatch for {name}:\n{}",
            serde_json::to_string_pretty(&patch).expect("pretty patch")
        );
    }
}

#[test]
fn patch_input_schema_preflights_object_ids() {
    let patch = json!({
        "schema_version": "adoc.patch.v0",
        "op": "create_object",
        "target": "billing.created",
        "changes": {
            "kind": "claim",
            "status": "draft",
            "body": "Created claim.",
            "placement": {"page_id": "team.billing", "after": "billing.ready"}
        },
        "reason": "Create claim."
    });
    assert!(schema_accepts("patch-input.json", &patch));

    for pointer in [
        "/target",
        "/changes/placement/page_id",
        "/changes/placement/after",
    ] {
        for value in ["Billing", "billing.created\n"] {
            let mut invalid = patch.clone();
            *invalid.pointer_mut(pointer).expect("field exists") = json!(value);
            assert!(
                !schema_accepts("patch-input.json", &invalid),
                "schema accepted malformed Object ID at {pointer}"
            );
        }
    }

    let malformed_supersedes = json!({
        "schema_version": "adoc.patch.v0",
        "op": "supersede",
        "target": "billing.ready",
        "base_hash": "sha256:content",
        "changes": {"supersedes": ["Billing"]},
        "reason": "Supersede stale knowledge."
    });
    assert!(!schema_accepts("patch-input.json", &malformed_supersedes));
}

/// V6.1: `adoc_stale` envelopes with all three record categories validate
/// against `adoc.stale.v0.schema.json`.
#[test]
fn validates_adoc_stale_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(
        &root.join("docs/index.adoc"),
        concat!(
            "# Lifecycle @doc(team.lifecycle)\n",
            "\n",
            "::claim team.expired-verified\n",
            "status: verified\n",
            "owner: team-docs\n",
            "verified_at: 2020-01-01\n",
            "source: audit records 2020\n",
            "expires_at: 2024-01-01\n",
            "--\n",
            "Verified but expired claim.\n",
            "::\n",
            "\n",
            "::claim team.expired-draft\n",
            "status: draft\n",
            "owner: team-docs\n",
            "expires_at: 2026-01-15\n",
            "--\n",
            "Draft with a past expiry.\n",
            "::\n",
            "\n",
            "::policy team.review-policy\n",
            "status: active\n",
            "owner: team-docs\n",
            "approved_by: [team-docs]\n",
            "effective_at: 2020-01-01\n",
            "review_interval: 30d\n",
            "--\n",
            "Policy overdue for review.\n",
            "::\n",
            "\n",
            "::claim team.expiring\n",
            "status: verified\n",
            "owner: team-docs\n",
            "verified_at: 2026-01-01\n",
            "source: audit records 2026\n",
            "expires_at: 2120-01-01\n",
            "--\n",
            "Verified claim expiring far in the future.\n",
            "::\n",
        ),
    );
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: false,
        })
        .expect("build succeeds");

    let stale = server
        .run_stale(StaleParams {
            project_root: None,
            artifact: None,
            within_days: None,
        })
        .expect("stale succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.stale.v0.schema.json", &stale);
    let records = stale["records"].as_array().expect("records array");
    assert_eq!(
        records.len(),
        3,
        "two stale + one review_overdue: {records:#?}"
    );

    let stale_within = server
        .run_stale(StaleParams {
            project_root: None,
            artifact: None,
            within_days: Some(36500),
        })
        .expect("stale with horizon succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.stale.v0.schema.json", &stale_within);
    let within_records = stale_within["records"].as_array().expect("records array");
    assert_eq!(
        within_records.len(),
        4,
        "plus one expiring_soon: {within_records:#?}"
    );
    let categories: Vec<&str> = within_records
        .iter()
        .filter_map(|record| record["category"].as_str())
        .collect();
    assert!(categories.contains(&"stale"));
    assert!(categories.contains(&"review_overdue"));
    assert!(categories.contains(&"expiring_soon"));
}

/// V6.2: `adoc_contradictions` envelopes — populated default listing, the
/// `all: true` superset, and an orphaned authored-`contradicted` claim with an
/// empty `contradiction_ids` — validate against
/// `adoc.contradictions.v0.schema.json`.
#[test]
fn validates_adoc_contradictions_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(
        &root.join("docs/index.adoc"),
        concat!(
            "# Conflicts @doc(team.conflicts)\n",
            "\n",
            "::claim team.storage-memory\n",
            "status: contradicted\n",
            "owner: team-docs\n",
            "--\n",
            "Tokens must be stored in memory only.\n",
            "::\n",
            "\n",
            "::claim team.storage-local\n",
            "status: accepted\n",
            "owner: team-docs\n",
            "--\n",
            "Tokens may be stored in localStorage.\n",
            "::\n",
            "\n",
            "::claim team.orphaned\n",
            "status: contradicted\n",
            "owner: team-docs\n",
            "--\n",
            "Authored contradicted with no unresolved contradiction left.\n",
            "::\n",
            "\n",
            "::claim team.settled-a\n",
            "status: accepted\n",
            "--\n",
            "First settled claim.\n",
            "::\n",
            "\n",
            "::claim team.settled-b\n",
            "status: accepted\n",
            "--\n",
            "Second settled claim.\n",
            "::\n",
            "\n",
            "::contradiction team.conflict-open\n",
            "severity: high\n",
            "status: unresolved\n",
            "claims: [team.storage-memory, team.storage-local]\n",
            "--\n",
            "Memory-only storage conflicts with the localStorage allowance.\n",
            "::\n",
            "\n",
            "::contradiction team.conflict-closed\n",
            "severity: critical\n",
            "status: resolved\n",
            "claims: [team.settled-a, team.settled-b]\n",
            "--\n",
            "Resolved conflict kept for history.\n",
            "::\n",
        ),
    );
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: false,
        })
        .expect("build succeeds");

    let envelope = server
        .run_contradictions(ContradictionsParams {
            project_root: None,
            artifact: None,
            all: false,
        })
        .expect("contradictions succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.contradictions.v0.schema.json", &envelope);
    assert!(
        envelope.get("evaluated_at").is_none(),
        "the contradictions envelope is clock-free"
    );
    let contradictions = envelope["contradictions"]
        .as_array()
        .expect("contradictions array");
    assert_eq!(
        contradictions.len(),
        1,
        "default listing is unresolved-only: {contradictions:#?}"
    );
    assert_eq!(contradictions[0]["id"], "team.conflict-open");
    let claims = envelope["contradicted_claims"]
        .as_array()
        .expect("contradicted_claims array");
    assert_eq!(
        claims.len(),
        3,
        "two implicated + one orphaned authored contradicted: {claims:#?}"
    );
    let orphan = claims
        .iter()
        .find(|claim| claim["id"] == "team.orphaned")
        .expect("orphaned claim listed");
    assert_eq!(
        orphan["contradiction_ids"],
        serde_json::json!([]),
        "orphaned authored status carries an empty contradiction_ids"
    );
    assert!(orphan.get("effective_reason").is_none());

    let all_envelope = server
        .run_contradictions(ContradictionsParams {
            project_root: None,
            artifact: None,
            all: true,
        })
        .expect("contradictions --all succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.contradictions.v0.schema.json", &all_envelope);
    let all_contradictions = all_envelope["contradictions"]
        .as_array()
        .expect("contradictions array");
    assert_eq!(
        all_contradictions.len(),
        2,
        "all: true adds the resolved record: {all_contradictions:#?}"
    );
    assert_eq!(
        all_contradictions[0]["id"], "team.conflict-closed",
        "critical sorts before high"
    );
    assert_eq!(all_contradictions[0]["status"], "resolved");
    assert_eq!(
        all_envelope["contradicted_claims"], envelope["contradicted_claims"],
        "--all never changes contradicted_claims"
    );
}

#[test]
fn validates_mcp_command_envelope_against_contract_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    let command = server
        .run_init(InitParams { project_root: None })
        .expect("init succeeds");

    assert_valid("mcp-command.json", &command);
}

#[test]
fn validates_adoc_diff_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    build_two_commit_review_fixture(root);

    let envelope = run_review_diff(root);
    let value = serde_json::to_value(&envelope).expect("envelope serializes");

    assert_valid("adoc.diff.v0.schema.json", &value);
    assert_eq!(value["schema_version"], "adoc.diff.v0");
    assert!(
        value["created"]
            .as_array()
            .expect("created")
            .iter()
            .any(|node| node["id"] == "billing.holds")
    );
    assert!(
        value["deleted"]
            .as_array()
            .expect("deleted")
            .iter()
            .any(|node| node["id"] == "billing.legacy-credits")
    );
    assert!(
        value["changed"]
            .as_array()
            .expect("changed")
            .iter()
            .any(|entry| entry["id"] == "billing.credits")
    );
}

/// Build a 2-commit git fixture under `root` matching the V3.1 review
/// acceptance scenario. Mirrors the layout used by
/// `crates/adoc-cli/tests/diff_cli.rs::build_two_commit_fixture`.
fn build_two_commit_review_fixture(root: &Path) {
    let base = concat!(
        "# Billing @doc(team.billing)\n",
        "\n",
        "::claim billing.credits\n",
        "status: draft\n",
        "--\n",
        "Credits apply after payment.\n",
        "::\n",
        "\n",
        "::claim billing.legacy-credits\n",
        "status: draft\n",
        "--\n",
        "Legacy credits, slated for removal.\n",
        "::\n",
    );
    let head = concat!(
        "# Billing @doc(team.billing)\n",
        "\n",
        "::claim billing.credits\n",
        "status: draft\n",
        "--\n",
        "Credits apply after ledger commit.\n",
        "::\n",
        "\n",
        "::claim billing.holds\n",
        "status: draft\n",
        "--\n",
        "Holds delay disbursement.\n",
        "::\n",
    );

    write(&root.join("agentdoc.config.yaml"), config());
    run_git(root, &["init", "--initial-branch=main"]);
    run_git(root, &["config", "user.email", "test@adoc.dev"]);
    run_git(root, &["config", "user.name", "adoc tests"]);
    run_git(root, &["config", "commit.gpgsign", "false"]);

    write(&root.join("docs/billing.adoc"), base);
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "base"]);

    run_git(root, &["checkout", "-b", "feature"]);
    write(&root.join("docs/billing.adoc"), head);
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "head"]);
}

fn config() -> &'static str {
    "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\n"
}

fn run_git(cwd: &Path, args: &[&str]) {
    let mut command = Command::new("git");
    command.arg("-C").arg(cwd).args(args);
    // Strip inherited GIT_* env vars so fixtures stay isolated from any
    // outer git repo whose context the test runner might have set (e.g.
    // pre-commit hooks via prek).
    for var in [
        "GIT_DIR",
        "GIT_INDEX_FILE",
        "GIT_WORK_TREE",
        "GIT_NAMESPACE",
        "GIT_OBJECT_DIRECTORY",
        "GIT_COMMON_DIR",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_PREFIX",
    ] {
        command.env_remove(var);
    }
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("spawn `git {args:?}`: {error}"));
    assert!(
        output.status.success(),
        "git {args:?} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn run_review_diff(root: &Path) -> ObjectDiffEnvelope {
    let load = load_review_from_git(ReviewInput {
        project_root: root.to_path_buf(),
        docs_path: PathBuf::from("docs"),
        base: SnapshotSelector::GitRef(GitRef::new("main")),
        head: SnapshotSelector::Workdir,
    })
    .expect("load review succeeds");
    let diff = diff_objects(&load.session);
    ObjectDiffEnvelope::from_diff(diff, load.diagnostics)
}

fn run_review(root: &Path) -> ReviewEnvelope {
    let load = load_review_with_changed_files_from_git(ReviewInput {
        project_root: root.to_path_buf(),
        docs_path: PathBuf::from("docs"),
        base: SnapshotSelector::GitRef(GitRef::new("main")),
        head: SnapshotSelector::Workdir,
    })
    .expect("load review succeeds");
    ReviewEnvelope::from_session(&load.session, load.diagnostics)
}

#[test]
fn validates_adoc_review_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    build_two_commit_review_fixture(root);

    let envelope = run_review(root);
    let value = serde_json::to_value(&envelope).expect("envelope serializes");

    assert_valid("adoc.review.v0.schema.json", &value);
    assert_eq!(value["schema_version"], "adoc.review.v0");
    assert_eq!(value["diff"]["schema_version"], "adoc.diff.v0");
    assert!(value["impact"].is_array());
    assert!(value["required_reviewers"].is_array());
    assert!(value["diagnostics"].is_array());

    // The embedded diff envelope must also stand on its own against its
    // schema — the two contracts are independently consumable.
    assert_valid("adoc.diff.v0.schema.json", &value["diff"]);

    // V3.7 — when no patch is supplied, patch_check is omitted from the
    // serialized envelope (not present as `null`).
    assert!(
        value.get("patch_check").is_none(),
        "patch_check must be omitted when no patch is supplied: {value:#}"
    );
}

#[test]
fn validates_adoc_review_v0_envelope_with_patch_check_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    build_two_commit_review_fixture(root);

    let load = load_review_with_changed_files_from_git(ReviewInput {
        project_root: root.to_path_buf(),
        docs_path: PathBuf::from("docs"),
        base: SnapshotSelector::GitRef(GitRef::new("main")),
        head: SnapshotSelector::Workdir,
    })
    .expect("load review succeeds");

    // Pull the head content_hash for billing.credits so the patch validates
    // cleanly. Round-trip via the no-patch envelope so we don't reach into
    // adoc-core internals.
    let envelope_no_patch = ReviewEnvelope::from_session(&load.session, Vec::new());
    let value = serde_json::to_value(&envelope_no_patch).expect("envelope serializes");
    let base_hash = value["diff"]["changed"]
        .as_array()
        .expect("changed array")
        .iter()
        .find(|entry| entry["id"] == "billing.credits")
        .expect("billing.credits in changed")["head"]["content_hash"]
        .as_str()
        .expect("content_hash")
        .to_string();

    let patch = parse_patch_from_value(json!({
        "schema_version": "adoc.patch.v0",
        "op": "replace_body",
        "target": "billing.credits",
        "base_hash": base_hash,
        "changes": { "body": "Patched body." },
        "reason": "demo"
    }))
    .expect("patch parses");

    let envelope = review_with_patch(&load.session, load.diagnostics, Some(&patch));
    let value = serde_json::to_value(&envelope).expect("envelope serializes");

    assert_valid("adoc.review.v0.schema.json", &value);
    assert_eq!(value["patch_check"]["valid"], json!(true));
    assert_eq!(
        value["patch_check"]["schema_version"],
        "adoc.patch.check.v0"
    );
    assert_eq!(value["patch_check"]["target"], "billing.credits");
}

#[test]
fn adoc_review_mcp_tool_accepts_optional_patch_parameter() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    build_two_commit_review_fixture(root);

    let server = AgentDocMcpServer::new(root.to_path_buf());

    // Round-trip via the no-patch path first to learn the head content_hash.
    let base_envelope = server
        .run_review(AdocReviewParams {
            project_root: None,
            base_ref: "main".to_string(),
            head_ref: None,
            patch: None,
        })
        .expect("review without patch succeeds");
    let base_hash = base_envelope["diff"]["changed"]
        .as_array()
        .expect("changed array")
        .iter()
        .find(|entry| entry["id"] == "billing.credits")
        .expect("billing.credits in changed")["head"]["content_hash"]
        .as_str()
        .expect("content_hash")
        .to_string();

    let envelope = server
        .run_review(AdocReviewParams {
            project_root: None,
            base_ref: "main".to_string(),
            head_ref: None,
            patch: Some(PatchInput::Inline {
                patch: json!({
                    "schema_version": "adoc.patch.v0",
                    "op": "replace_body",
                    "target": "billing.credits",
                    "base_hash": base_hash,
                    "changes": { "body": "Patched body." },
                    "reason": "demo"
                }),
            }),
        })
        .expect("review with inline patch succeeds");

    assert_valid("adoc.review.v0.schema.json", &envelope);
    assert_eq!(envelope["patch_check"]["valid"], json!(true));
}

/// MCP serves published schema files verbatim (no transformation and no drift
/// between the bundled `include_str!` and the source-of-truth file).
#[test]
fn mcp_serves_schema_resources_byte_equal_to_on_disk_files() {
    let workspace = tempfile::tempdir().expect("workspace");
    let server = AgentDocMcpServer::new(workspace.path().to_path_buf());

    for (uri, file) in [
        (
            "adoc://agent/v0/schema/adoc.portable_projection_input.v0.schema.json",
            "adoc.portable_projection_input.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.portable_projection.v0.schema.json",
            "adoc.portable_projection.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.cloud.portable_export_preparation.v0.schema.json",
            "agentdoc.cloud.portable_export_preparation.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.cloud.portable_export_manifest.v0.schema.json",
            "agentdoc.cloud.portable_export_manifest.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.cloud.portable_export_finalization.v0.schema.json",
            "agentdoc.cloud.portable_export_finalization.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.cloud.export_native_fact.v0.schema.json",
            "agentdoc.cloud.export_native_fact.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.cloud.portable_export_receipt.v0.schema.json",
            "agentdoc.cloud.portable_export_receipt.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.managed_field_declassification.v0.schema.json",
            "adoc.managed_field_declassification.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.managed_field_provenance.v0.schema.json",
            "adoc.managed_field_provenance.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.sensitive_access.v1.schema.json",
            "adoc.sensitive_access.v1.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.sensitive_access.v0.schema.json",
            "adoc.sensitive_access.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/retrieval-envelope.json",
            "retrieval-envelope.json",
        ),
        (
            "adoc://agent/v0/schema/retrieval-envelope.v0.json",
            "retrieval-envelope.v0.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.diff.v0.schema.json",
            "adoc.diff.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.review.v0.schema.json",
            "adoc.review.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.patch.apply.v0.schema.json",
            "adoc.patch.apply.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.change_assessment.v0.schema.json",
            "adoc.change_assessment.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.repository_baseline.v0.schema.json",
            "adoc.repository_baseline.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.migrate.report.v0.schema.json",
            "adoc.migrate.report.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/search-artifact.json",
            "search-artifact.json",
        ),
        (
            "adoc://agent/v0/schema/graph-artifact.v6.json",
            "graph-artifact.v6.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.lifecycle_mapping.v0.schema.json",
            "adoc.lifecycle_mapping.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.semantic_context.v0.schema.json",
            "adoc.semantic_context.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.semantic_assessment.v0.schema.json",
            "adoc.semantic_assessment.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.executor_qualification.v0.schema.json",
            "adoc.executor_qualification.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.source_assertion.v0.schema.json",
            "adoc.source_assertion.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.source_binding.v0.schema.json",
            "adoc.source_binding.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.source_record.v0.schema.json",
            "adoc.source_record.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.source_record.v1.schema.json",
            "adoc.source_record.v1.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.work_request.v0.schema.json",
            "adoc.work_request.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/adoc.work_result.v0.schema.json",
            "adoc.work_result.v0.schema.json",
        ),
        (
            "adoc://agent/v0/schema/agentdoc.connector_capabilities.v0.schema.json",
            "agentdoc.connector_capabilities.v0.schema.json",
        ),
    ] {
        let result = server
            .read_agent_resource(uri)
            .unwrap_or_else(|error| panic!("resource {uri} reads: {error}"));
        let served = match &result.contents[0] {
            rmcp::model::ResourceContents::TextResourceContents { text, .. } => text.clone(),
            other => panic!("expected text resource for {uri}, got {other:?}"),
        };
        let disk = fs::read_to_string(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../docs/agent/v0/schema")
                .join(file),
        )
        .unwrap_or_else(|error| panic!("disk schema {file} reads: {error}"));
        assert_eq!(
            served, disk,
            "MCP-served schema {uri} drifted from docs/agent/v0/schema/{file}"
        );
    }
}

#[test]
fn validates_complete_and_error_change_assessments_and_rejects_illegal_tuples() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    run_git(root, &["init", "--initial-branch=main"]);
    run_git(root, &["config", "user.email", "test@agentdoc.dev"]);
    run_git(root, &["config", "user.name", "AgentDoc Test"]);
    run_git(root, &["config", "commit.gpgsign", "false"]);
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\nembeddings:\n  provider: none\n",
    );
    write(&root.join("docs/index.adoc"), source());
    write(&root.join("src/lib.rs"), "pub fn before() {}\n");
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "initial"]);
    write(&root.join("src/lib.rs"), "pub fn after() {}\n");
    let context = LocalContext::new(root.to_path_buf(), UnrestrictedPathPolicy);
    let complete = serde_json::to_value(
        context
            .assess_changes(AssessmentInput {
                base_ref: "HEAD".to_string(),
                head_ref: None,
                as_of: None,
            })
            .expect("complete assessment runs")
            .envelope,
    )
    .expect("complete envelope serializes");
    assert_eq!(complete["authority_promotions"]["status"], "available");
    assert_valid("adoc.change_assessment.v0.schema.json", &complete);
    let mut prior_shape = complete.clone();
    prior_shape
        .as_object_mut()
        .expect("assessment is an object")
        .remove("authority_promotions");
    assert_valid("adoc.change_assessment.v0.schema.json", &prior_shape);

    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "code change"]);
    write(&root.join("agentdoc.config.yaml"), "version: [broken\n");
    run_git(root, &["add", "agentdoc.config.yaml"]);
    run_git(root, &["commit", "-m", "broken comparison config"]);
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\nembeddings:\n  provider: none\n",
    );
    let partial = serde_json::to_value(
        context
            .assess_changes(AssessmentInput {
                base_ref: "HEAD".to_string(),
                head_ref: None,
                as_of: None,
            })
            .expect("partial assessment runs")
            .envelope,
    )
    .expect("partial envelope serializes");
    assert_eq!(partial["completeness"], "partial");
    assert_eq!(partial["authority_promotions"]["status"], "unavailable");
    assert_valid("adoc.change_assessment.v0.schema.json", &partial);

    let error = serde_json::to_value(
        context
            .assess_changes(AssessmentInput {
                base_ref: "missing-ref".to_string(),
                head_ref: None,
                as_of: None,
            })
            .expect("error assessment runs")
            .envelope,
    )
    .expect("error envelope serializes");
    assert_valid("adoc.change_assessment.v0.schema.json", &error);

    let mut illegal = complete.clone();
    illegal["completeness"] = json!("partial");
    illegal["outcome"] = json!("pass");
    let schema = schema("adoc.change_assessment.v0.schema.json");
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    assert!(!validator.is_valid(&illegal));

    let mut kind_only_promotion = complete.clone();
    kind_only_promotion["authority_promotions"] = json!({"status": "available", "value": [{
        "id": "billing.credits",
        "content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "kind": "claim",
        "before_kind": "example",
        "before_status": "verified",
        "after_status": "verified"
    }]});
    assert_valid(
        "adoc.change_assessment.v0.schema.json",
        &kind_only_promotion,
    );

    for authority_promotions in [
        json!({"status": "available"}),
        json!({"status": "unavailable", "value": []}),
        json!({"status": "available", "value": [{
            "id": "billing.policy",
            "content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "kind": "policy",
            "before_kind": "policy",
            "before_status": "draft",
            "after_status": "verified"
        }]}),
        json!({"status": "available", "value": [{
            "id": "billing.credits",
            "content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "kind": "decision",
            "before_kind": "policy",
            "before_status": "active",
            "after_status": "accepted"
        }]}),
        json!({"status": "available", "value": [{
            "id": "billing.credits",
            "content_hash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "kind": "claim",
            "before_kind": "claim",
            "before_status": "verified",
            "after_status": "verified"
        }]}),
    ] {
        let mut forged = complete.clone();
        forged["authority_promotions"] = authority_promotions;
        assert!(
            !validator.is_valid(&forged),
            "forged promotion facts passed"
        );
    }

    let mut unavailable_complete = complete.clone();
    unavailable_complete["authority_promotions"] = json!({"status": "unavailable"});
    assert!(!validator.is_valid(&unavailable_complete));

    let mut available_error = error;
    available_error["authority_promotions"] = json!({"status": "available", "value": []});
    assert!(!validator.is_valid(&available_error));
}

#[test]
fn serialized_repository_baseline_matches_published_schema_for_all_readiness_reasons() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    run_git(root, &["init", "--initial-branch=main"]);
    run_git(root, &["config", "user.email", "test@agentdoc.dev"]);
    run_git(root, &["config", "user.name", "AgentDoc Test"]);
    run_git(root, &["config", "commit.gpgsign", "false"]);
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\nembeddings:\n  provider: none\n",
    );
    write(
        &root.join("docs/index.adoc"),
        &source().replace("status: draft\n", "status: draft\nimpacts: [src/lib.rs]\n"),
    );
    write(&root.join("src/lib.rs"), "pub fn billing_ready() {}\n");
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "initial"]);
    let exact_head = fs::read_to_string(root.join(".git/refs/heads/main"))
        .expect("main ref is readable")
        .trim()
        .to_string();

    let context = LocalContext::new(root.to_path_buf(), UnrestrictedPathPolicy);
    let produced = context
        .repository_baseline(RepositoryBaselineInput {
            git_ref: exact_head.clone(),
            as_of: Some("2026-09-05".parse().expect("fixed evaluation date")),
        })
        .expect("repository baseline runs")
        .envelope;
    let produced = serde_json::to_value(produced).expect("repository baseline serializes");
    assert_eq!(produced["snapshot"]["immutable"], true);
    assert_eq!(produced["snapshot"]["requested_ref"], exact_head);
    assert_eq!(produced["snapshot"]["resolved_commit"], exact_head);
    assert_eq!(produced["evaluation_date"], "2026-09-05");
    assert_eq!(produced["readiness"]["reason"], "provisional_paths");
    assert_eq!(produced["readiness"]["ready"], false);
    let paths = produced["paths"]["value"]
        .as_array()
        .expect("baseline paths are available");
    assert!(paths.iter().any(|path| path["matches"] != json!([])));
    let objects = produced["objects"]["value"]
        .as_array()
        .expect("baseline objects are available");
    assert!(objects.iter().any(|object| object["reasons"] != json!([])));

    let unresolved = context
        .repository_baseline(RepositoryBaselineInput {
            git_ref: "missing-ref".to_string(),
            as_of: None,
        })
        .expect("unresolved repository baseline is data")
        .envelope;
    let unresolved =
        serde_json::to_value(unresolved).expect("unresolved repository baseline serializes");
    assert_eq!(unresolved["readiness"]["reason"], "invalid_source");
    assert_eq!(unresolved["paths"]["status"], "unavailable");
    assert_ne!(unresolved["diagnostics"], json!([]));

    write(
        &root.join("docs/index.adoc"),
        &source().replace(
            "status: draft\n",
            "status: verified\nowner: team-docs\nverified_at: 2026-09-05\nsource: test\nimpacts: [src/lib.rs]\n",
        ),
    );
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "verify coverage"]);
    let ready_head = fs::read_to_string(root.join(".git/refs/heads/main"))
        .expect("ready ref is readable")
        .trim()
        .to_string();
    let ready_baseline = context
        .repository_baseline(RepositoryBaselineInput {
            git_ref: ready_head,
            as_of: Some("2026-09-05".parse().expect("fixed evaluation date")),
        })
        .expect("ready repository baseline runs")
        .envelope;
    let ready_baseline =
        serde_json::to_value(ready_baseline).expect("ready repository baseline serializes");
    assert_eq!(
        ready_baseline["readiness"],
        json!({ "ready": true, "reason": "ready" })
    );

    write(&root.join("docs/index.adoc"), source());
    run_git(root, &["add", "-A"]);
    run_git(root, &["commit", "-m", "remove coverage"]);
    let uncovered_head = fs::read_to_string(root.join(".git/refs/heads/main"))
        .expect("uncovered ref is readable")
        .trim()
        .to_string();
    let uncovered_baseline = context
        .repository_baseline(RepositoryBaselineInput {
            git_ref: uncovered_head,
            as_of: Some("2026-09-05".parse().expect("fixed evaluation date")),
        })
        .expect("uncovered repository baseline runs")
        .envelope;
    let uncovered_baseline =
        serde_json::to_value(uncovered_baseline).expect("uncovered repository baseline serializes");
    assert_eq!(
        uncovered_baseline["readiness"],
        json!({ "ready": false, "reason": "uncovered_paths" })
    );

    let schema_name = "adoc.repository_baseline.v0.schema.json";
    for baseline in [&produced, &unresolved, &ready_baseline, &uncovered_baseline] {
        assert_valid(schema_name, baseline);
    }

    let mut invalid_with_validation_errors = ready_baseline.clone();
    invalid_with_validation_errors["readiness"] =
        json!({ "ready": false, "reason": "invalid_source" });
    invalid_with_validation_errors["validation"]["errors_full"] = json!(1);
    assert_valid(schema_name, &invalid_with_validation_errors);

    let mut invalid_without_requested_ref = unresolved.clone();
    invalid_without_requested_ref["snapshot"]
        .as_object_mut()
        .expect("snapshot is an object")
        .remove("requested_ref");
    assert!(!schema_accepts(schema_name, &invalid_without_requested_ref));

    let mut available_knowledge_without_replay_identity = unresolved.clone();
    available_knowledge_without_replay_identity["knowledge_snapshot"]["status"] =
        json!("available");
    assert!(!schema_accepts(
        schema_name,
        &available_knowledge_without_replay_identity
    ));

    let mut unavailable_knowledge_with_replay_identity = unresolved.clone();
    unavailable_knowledge_with_replay_identity["knowledge_snapshot"]["graph_schema_version"] =
        json!("adoc.graph.v6");
    assert!(!schema_accepts(
        schema_name,
        &unavailable_knowledge_with_replay_identity
    ));

    let mut ready_with_uncovered_classification = ready_baseline.clone();
    ready_with_uncovered_classification["paths"]["value"]
        .as_array_mut()
        .expect("ready paths are available")
        .iter_mut()
        .find(|path| path["classification"] == "covered")
        .expect("ready baseline has a covered path")["classification"] = json!("uncovered");
    assert!(!schema_accepts(
        schema_name,
        &ready_with_uncovered_classification
    ));

    let mut ready_with_provisional_classification = ready_baseline.clone();
    ready_with_provisional_classification["paths"]["value"]
        .as_array_mut()
        .expect("ready paths are available")
        .iter_mut()
        .find(|path| path["classification"] == "covered")
        .expect("ready baseline has a covered path")["classification"] = json!("provisional");
    assert!(!schema_accepts(
        schema_name,
        &ready_with_provisional_classification
    ));

    let mut provisional_without_provisional_classification = produced.clone();
    for path in provisional_without_provisional_classification["paths"]["value"]
        .as_array_mut()
        .expect("provisional paths are available")
    {
        if path["classification"] == "provisional" {
            path["classification"] = json!("covered");
        }
    }
    assert!(!schema_accepts(
        schema_name,
        &provisional_without_provisional_classification
    ));

    let mut uncovered_without_uncovered_classification = uncovered_baseline.clone();
    for path in uncovered_without_uncovered_classification["paths"]["value"]
        .as_array_mut()
        .expect("uncovered paths are available")
    {
        if path["classification"] == "uncovered" {
            path["classification"] = json!("covered");
        }
    }
    assert!(!schema_accepts(
        schema_name,
        &uncovered_without_uncovered_classification
    ));

    let mut uncovered_with_provisional_classification = uncovered_baseline.clone();
    let paths = uncovered_with_provisional_classification["paths"]["value"]
        .as_array_mut()
        .expect("uncovered paths are available");
    let mut provisional_path = paths
        .iter()
        .find(|path| path["classification"] == "uncovered")
        .expect("uncovered baseline has an uncovered path")
        .clone();
    provisional_path["path"] = json!("provisional.rs");
    provisional_path["classification"] = json!("provisional");
    paths.push(provisional_path);
    assert!(!schema_accepts(
        schema_name,
        &uncovered_with_provisional_classification
    ));

    let mut uncovered_with_provisional_count = uncovered_baseline.clone();
    uncovered_with_provisional_count["summary"]["provisional"] = json!(1);
    assert!(!schema_accepts(
        schema_name,
        &uncovered_with_provisional_count
    ));

    let mut unsupported = produced.clone();
    unsupported["schema_version"] = json!("adoc.repository_baseline.v99");
    assert!(!schema_accepts(schema_name, &unsupported));

    let mut unknown_reason = produced.clone();
    unknown_reason["readiness"]["reason"] = json!("unknown");
    assert!(!schema_accepts(schema_name, &unknown_reason));

    let mut ready_with_errors = ready_baseline.clone();
    ready_with_errors["validation"]["errors_full"] = json!(1);
    assert!(!schema_accepts(schema_name, &ready_with_errors));

    for counter in ["errors_changed", "errors_unchanged", "errors_unattributed"] {
        let mut ready_with_partitioned_errors = ready_baseline.clone();
        ready_with_partitioned_errors["validation"][counter] = json!(1);
        assert!(!schema_accepts(schema_name, &ready_with_partitioned_errors));
    }

    let mut excluded_without_evidence = ready_baseline.clone();
    let excluded_path = excluded_without_evidence["paths"]["value"]
        .as_array_mut()
        .expect("ready paths are available")
        .iter_mut()
        .find(|path| path["classification"] == "covered")
        .expect("ready baseline has a covered path");
    excluded_path["classification"] = json!("excluded");
    assert!(!schema_accepts(schema_name, &excluded_without_evidence));

    let mut excluded_with_matches = excluded_without_evidence;
    excluded_with_matches["paths"]["value"]
        .as_array_mut()
        .expect("ready paths are available")
        .iter_mut()
        .find(|path| path["path"] == "src/lib.rs")
        .expect("mutated baseline retains the covered fixture path")["exclusion_reason"] =
        json!("configured_exclusion:src/lib.rs");
    assert_ne!(
        excluded_with_matches["paths"]["value"]
            .as_array()
            .expect("ready paths are available")
            .iter()
            .find(|path| path["path"] == "src/lib.rs")
            .expect("mutated baseline retains the covered fixture path")["matches"],
        json!([])
    );
    assert!(!schema_accepts(schema_name, &excluded_with_matches));

    let mut covered_with_exclusion_reason = ready_baseline.clone();
    covered_with_exclusion_reason["paths"]["value"]
        .as_array_mut()
        .expect("ready paths are available")
        .iter_mut()
        .find(|path| path["classification"] == "covered")
        .expect("ready baseline has a covered path")["exclusion_reason"] =
        json!("configured_exclusion:src/lib.rs");
    assert!(!schema_accepts(schema_name, &covered_with_exclusion_reason));

    let mut ready_with_uncovered_paths = ready_baseline.clone();
    ready_with_uncovered_paths["summary"]["uncovered"] = json!(1);
    assert!(!schema_accepts(schema_name, &ready_with_uncovered_paths));

    let mut invalid_without_invalid_source = ready_baseline.clone();
    invalid_without_invalid_source["readiness"] =
        json!({ "ready": false, "reason": "invalid_source" });
    assert!(!schema_accepts(
        schema_name,
        &invalid_without_invalid_source
    ));

    let mut mutable_ready = ready_baseline.clone();
    mutable_ready["snapshot"]["immutable"] = json!(false);
    assert!(!schema_accepts(schema_name, &mutable_ready));

    let mut mutable_invalid_source = unresolved.clone();
    mutable_invalid_source["snapshot"]["immutable"] = json!(false);
    assert!(!schema_accepts(schema_name, &mutable_invalid_source));

    let mut worktree_invalid_source = unresolved.clone();
    worktree_invalid_source["snapshot"]["worktree_state"] = json!("dirty");
    assert!(!schema_accepts(schema_name, &worktree_invalid_source));

    let mut impossible_strategy = produced;
    impossible_strategy["snapshot"]["strategy"] = json!("merge_base");
    assert!(!schema_accepts(schema_name, &impossible_strategy));

    let mut ready_without_knowledge = ready_baseline.clone();
    ready_without_knowledge["knowledge_snapshot"] = json!({ "status": "unavailable" });
    assert!(!schema_accepts(schema_name, &ready_without_knowledge));

    let mut ready_without_objects = ready_baseline.clone();
    ready_without_objects["objects"] = json!({ "status": "unavailable" });
    assert!(!schema_accepts(schema_name, &ready_without_objects));

    let mut available_without_value = ready_baseline;
    available_without_value["paths"] = json!({ "status": "available" });
    assert!(!schema_accepts(schema_name, &available_without_value));

    let mut unavailable_with_value = unresolved;
    unavailable_with_value["objects"]["value"] = json!([]);
    assert!(!schema_accepts(schema_name, &unavailable_with_value));

    let mut provisional_without_knowledge = invalid_without_invalid_source.clone();
    provisional_without_knowledge["readiness"] =
        json!({ "ready": false, "reason": "provisional_paths" });
    provisional_without_knowledge["summary"]["provisional"] = json!(1);
    provisional_without_knowledge["knowledge_snapshot"] = json!({ "status": "unavailable" });
    assert!(!schema_accepts(schema_name, &provisional_without_knowledge));

    let mut uncovered_without_objects = invalid_without_invalid_source;
    uncovered_without_objects["readiness"] = json!({ "ready": false, "reason": "uncovered_paths" });
    uncovered_without_objects["summary"]["uncovered"] = json!(1);
    uncovered_without_objects["objects"] = json!({ "status": "unavailable" });
    assert!(!schema_accepts(schema_name, &uncovered_without_objects));
}

/// V8.1.2/V8.1.3: the `adoc migrate` report envelope — built at the same
/// `adoc-local` seam the CLI serializes (there is no MCP migrate tool) over
/// a fixture with a raw HTML block, a broken link, front matter, and a TODO
/// paragraph (so the schema validates a populated `suggestions` array) —
/// validates against `adoc.migrate.report.v0.schema.json`.
#[test]
fn validates_adoc_migrate_report_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(
        &root.join("docs/html.md"),
        "# Html\n\n<div class=\"alert\">raw</div>\n",
    );
    write(
        &root.join("docs/links.md"),
        "# Links\n\nSee [gone](./missing.md).\n",
    );
    write(
        &root.join("docs/front.md"),
        "---\ntitle: front\n---\n\n# Front\n\nProse.\n\nTODO: type this later.\n",
    );

    let context =
        adoc_local::LocalContext::new(root.to_path_buf(), adoc_local::UnrestrictedPathPolicy);
    let outcome = context
        .migrate(adoc_local::MigrateInput {
            path: Some(root.join("docs")),
            write: false,
            force: false,
            export: false,
        })
        .expect("migrate succeeds");
    let report = serde_json::to_value(&outcome.report).expect("report serializes");

    assert_valid("adoc.migrate.report.v0.schema.json", &report);
    assert_eq!(report["schema_version"], "adoc.migrate.report.v0");
    assert_eq!(report["direction"], "import");
    assert_eq!(report["counts"]["files_imported"], 3);
    assert_eq!(report["counts"]["suggested_typed_blocks"], 1);
    assert_eq!(report["suggestions"][0]["suggested_kind"], "task");
    assert_eq!(report["suggestions"][0]["matched_rule"], "todo_line");
}

/// V8.1.4: the `--export` direction reports through the same envelope — a
/// prose-mode `.adoc` fixture with an ```html quarantine carrier (so the
/// schema validates an unwrap diagnostic) validates against
/// `adoc.migrate.report.v0.schema.json` with `direction: "export"`.
#[test]
fn validates_adoc_migrate_report_v0_export_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(
        &root.join("docs/alerts.adoc"),
        "# Alerts\n\nProse first.\n\n```html\n<div class=\"alert\">raw</div>\n```\n",
    );

    let context =
        adoc_local::LocalContext::new(root.to_path_buf(), adoc_local::UnrestrictedPathPolicy);
    let outcome = context
        .migrate(adoc_local::MigrateInput {
            path: Some(root.join("docs")),
            write: false,
            force: false,
            export: true,
        })
        .expect("export succeeds");
    let report = serde_json::to_value(&outcome.report).expect("report serializes");

    assert_valid("adoc.migrate.report.v0.schema.json", &report);
    assert_eq!(report["schema_version"], "adoc.migrate.report.v0");
    assert_eq!(report["direction"], "export");
    assert_eq!(report["counts"]["files_imported"], 1);
    assert_eq!(report["counts"]["raw_html_quarantined"], 1);
    assert_eq!(report["counts"]["suggested_typed_blocks"], 0);
    assert_eq!(report["suggestions"], serde_json::json!([]));
}

/// V6.3: `adoc_impacted_by` envelopes — a populated paths-shape query hitting
/// declared impacts, inline evidence, and evidence-ref resolution; the empty
/// no-match case; and the paths-XOR-ref argument rule — validate against
/// `adoc.impacted.v0.schema.json`.
#[test]
fn validates_adoc_impacted_v0_envelope_against_schema() {
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(
        &root.join("docs/index.adoc"),
        concat!(
            "# Impact @doc(team.impact)\n",
            "\n",
            "::claim team.refunds\n",
            "status: verified\n",
            "owner: team-billing\n",
            "verified_at: 2026-05-05\n",
            "source: crates/billing/src/refund.rs\n",
            "impacts: crates/billing/src/refund.rs\n",
            "--\n",
            "Refunds process within 24 hours.\n",
            "::\n",
            "\n",
            "::decision team.ledger-first\n",
            "status: accepted\n",
            "decided_by: architecture\n",
            "owner: team-billing\n",
            "evidence_ref: team.consume-source\n",
            "--\n",
            "Ledger-first credit consumption.\n",
            "::\n",
            "\n",
            "::source team.consume-source\n",
            "kind: source_code\n",
            "path: apps/backend/src/consume.ts\n",
            "owner: team-billing\n",
            "--\n",
            "Credit consumption implementation.\n",
            "::\n",
            "\n",
            "::claim team.draft-bystander\n",
            "status: draft\n",
            "impacts: crates/billing/src/refund.rs\n",
            "--\n",
            "Draft claim outside the verified-subject scope.\n",
            "::\n",
        ),
    );
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\n",
    );
    let server = AgentDocMcpServer::new(root.to_path_buf());
    server
        .run_build(BuildParams {
            project_root: None,
            path: None,
            out: None,
            no_embeddings: false,
        })
        .expect("build succeeds");

    let impacted = server
        .run_impacted_by(ImpactedByParams {
            project_root: None,
            artifact: None,
            paths: Some(vec![
                "crates/billing/src/refund.rs".to_string(),
                "apps/backend/src/consume.ts".to_string(),
            ]),
            git_ref: None,
        })
        .expect("impacted-by succeeds")
        .structured_content
        .unwrap();
    assert_valid("adoc.impacted.v0.schema.json", &impacted);
    assert_eq!(impacted["schema_version"], "adoc.impacted.v0");
    assert_eq!(
        impacted["changed_paths"],
        json!([
            "apps/backend/src/consume.ts",
            "crates/billing/src/refund.rs"
        ]),
        "changed_paths sorted ascending"
    );
    let records = impacted["impacted"].as_array().expect("impacted array");
    assert_eq!(
        records.len(),
        2,
        "verified claim + accepted decision, draft excluded: {records:#?}"
    );
    assert_eq!(records[0]["id"], "team.ledger-first");
    assert_eq!(records[0]["reasons"][0]["kind"], "evidence_path");
    assert_eq!(
        records[0]["reasons"][0]["via_source_object"],
        "team.consume-source"
    );
    assert_eq!(records[1]["id"], "team.refunds");
    let refund_reasons = records[1]["reasons"].as_array().expect("reasons");
    assert_eq!(
        refund_reasons.len(),
        2,
        "same path via impacts: and inline source evidence: {refund_reasons:#?}"
    );
    assert_eq!(refund_reasons[0]["kind"], "impacts_path");
    assert_eq!(refund_reasons[1]["kind"], "evidence_path");
    assert_eq!(
        impacted["proof_obligations"]
            .as_array()
            .expect("obligations")
            .len(),
        2
    );

    let empty = server
        .run_impacted_by(ImpactedByParams {
            project_root: None,
            artifact: None,
            paths: Some(vec!["unrelated/path.rs".to_string()]),
            git_ref: None,
        })
        .expect("impacted-by succeeds with no matches")
        .structured_content
        .unwrap();
    assert_valid("adoc.impacted.v0.schema.json", &empty);
    assert_eq!(empty["impacted"], json!([]));
    assert_eq!(empty["proof_obligations"], json!([]));

    // Exactly one of `paths` / `ref` — both, neither, and empty `paths` are
    // argument errors. Empty `paths` mirrors the CLI, where clap treats an
    // empty Vec as "not present": an agent forwarding an empty diff must get
    // an argument error, not a silent empty envelope.
    for params in [
        ImpactedByParams {
            project_root: None,
            artifact: None,
            paths: Some(vec!["a.rs".to_string()]),
            git_ref: Some("main".to_string()),
        },
        ImpactedByParams {
            project_root: None,
            artifact: None,
            paths: None,
            git_ref: None,
        },
        ImpactedByParams {
            project_root: None,
            artifact: None,
            paths: Some(Vec::new()),
            git_ref: None,
        },
    ] {
        let error = server
            .run_impacted_by(params)
            .expect_err("paths XOR ref must be enforced");
        assert!(
            error.to_string().contains("paths"),
            "error must name the argument rule: {error}"
        );
    }
}

/// V6.4 TB4: `adoc_patch_apply` envelopes — an applied success, the
/// disabled-gate refusal, and a stale-base-hash refusal — validate against
/// `adoc.patch.apply.v0.schema.json`.
#[test]
fn validates_patch_apply_envelopes_against_contract_schema() {
    use adoc_mcp::AdocPatchApplyParams;

    fn inline_patch(base_hash: &str) -> PatchInput {
        PatchInput::Inline {
            patch: json!({
                "schema_version": "adoc.patch.v0",
                "op": "replace_body",
                "target": "billing.ready",
                "base_hash": base_hash,
                "changes": { "body": "Billing docs are ready and applied." },
                "reason": "Contract-test the apply envelope.",
                "proposer": { "type": "agent", "id": "contract-test" }
            }),
        }
    }

    // Disabled gate (the default project has no `mcp:` block).
    let (_workspace, server, base_hash) = project_with_built_graph();
    let refusal = server
        .run_patch_apply(AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_patch(&base_hash),
        })
        .expect("gate refusal is a normal envelope");
    assert_valid("adoc.patch.apply.v0.schema.json", &refusal);
    assert_eq!(refusal["applied"], false);
    assert_eq!(
        refusal["diagnostics"][0]["code"],
        "mcp.patch_apply_disabled"
    );

    // Enabled project: applied success, then a stale-base-hash refusal.
    let workspace = tempfile::tempdir().expect("workspace");
    let root = workspace.path();
    write(&root.join("docs/index.adoc"), source());
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\noutputs:\n  dir: dist\nembeddings:\n  provider: deterministic\nmcp:\n  patch_apply: enabled\n",
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
        .find(|node| node["id"] == "billing.ready")
        .expect("target node")["content_hash"]
        .as_str()
        .expect("content hash")
        .to_string();

    let applied = server
        .run_patch_apply(AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_patch(&base_hash),
        })
        .expect("apply runs");
    assert_valid("adoc.patch.apply.v0.schema.json", &applied);
    assert_eq!(applied["applied"], true);
    assert_eq!(applied["trace"]["interface"], "mcp");
    assert_eq!(applied["trace"]["proposer"]["kind"], "agent");

    let stale = server
        .run_patch_apply(AdocPatchApplyParams {
            project_root: None,
            artifact: None,
            input: inline_patch("sha256:stale"),
        })
        .expect("refusal is a normal envelope");
    assert_valid("adoc.patch.apply.v0.schema.json", &stale);
    assert_eq!(stale["applied"], false);
}

/// V1.7.1 (ADR-0040): the discriminated `adoc.retrieval.v1` envelope — the
/// blended list carrying both record types, each scope restriction, and the
/// Knowledge-Object-only `adoc_why` envelope — validates against
/// `retrieval-envelope.json`, and the legacy v0 schema stays published and
/// self-consistent.
#[test]
fn validates_retrieval_v1_envelopes_against_discriminated_schema() {
    let (_workspace, server, _base_hash) = project_with_built_graph();

    let search_params = |objects_only: bool, prose_only: bool| SearchParams {
        project_root: None,
        query: "billing".to_string(),
        artifact: None,
        search_artifact: None,
        semantic: false,
        lexical: true,
        objects_only,
        prose_only,
        kind: None,
        status: None,
        owner: None,
        source_path: None,
        related_to: None,
        relation: None,
        direction: None,
        top: Some(10),
    };

    let blended = server
        .run_search(search_params(false, false))
        .expect("blended search succeeds")
        .structured_content
        .unwrap();
    assert_valid("retrieval-envelope.json", &blended);
    let record_types: Vec<&str> = blended["records"]
        .as_array()
        .expect("records array")
        .iter()
        .filter_map(|record| record["record_type"].as_str())
        .collect();
    assert!(
        record_types.contains(&"knowledge_object") && record_types.contains(&"prose"),
        "the blended fixture must exercise both schema branches, got {record_types:?}"
    );

    let objects_only = server
        .run_search(search_params(true, false))
        .expect("objects-only search succeeds")
        .structured_content
        .unwrap();
    assert_valid("retrieval-envelope.json", &objects_only);

    let prose_only = server
        .run_search(search_params(false, true))
        .expect("prose-only search succeeds")
        .structured_content
        .unwrap();
    assert_valid("retrieval-envelope.json", &prose_only);

    let why = server
        .run_why(adoc_mcp::WhyParams {
            project_root: None,
            object_id: "billing.ready".to_string(),
            artifact: None,
        })
        .expect("why succeeds")
        .structured_content
        .unwrap();
    assert_valid("retrieval-envelope.json", &why);
    assert_eq!(why["records"][0]["record_type"], "knowledge_object");

    // The "v0 stays published" guarantee: the legacy schema still validates a
    // hand-built v0 envelope and still rejects the v1 version string.
    let legacy_instance = json!({
        "schema_version": "adoc.retrieval.v0",
        "records": [{
            "id": "billing.ready",
            "kind": "claim",
            "content_hash": "sha256:legacy",
            "body": "Billing docs are ready.",
            "source": { "path": "docs/index.adoc", "line": 3, "column": 1 },
            "relations": { "depends_on": [], "supersedes": [], "related_to": [] }
        }],
        "diagnostics": []
    });
    assert_valid("retrieval-envelope.v0.json", &legacy_instance);
    let legacy_schema = schema("retrieval-envelope.v0.json");
    let validator = jsonschema::validator_for(&legacy_schema).expect("legacy schema compiles");
    assert!(
        !validator.is_valid(&blended),
        "the legacy v0 schema must reject a v1 envelope"
    );
}

/// V1.7.1 (ADR-0040): the v0 and v1 retrieval schemas are published side by
/// side, so each `$id` must match the URI the MCP resource serves it at — a
/// client that indexes schemas by `$id` must never see a collision.
#[test]
fn retrieval_schema_ids_match_their_published_uris() {
    for (name, expected_id) in [
        (
            "retrieval-envelope.json",
            "adoc://agent/v0/schema/retrieval-envelope.json",
        ),
        (
            "retrieval-envelope.v0.json",
            "adoc://agent/v0/schema/retrieval-envelope.v0.json",
        ),
        (
            "search-artifact.json",
            "adoc://agent/v0/schema/search-artifact.json",
        ),
        (
            "graph-artifact.v6.json",
            "adoc://agent/v0/schema/graph-artifact.v6.json",
        ),
        (
            "adoc.lifecycle_mapping.v0.schema.json",
            "adoc://agent/v0/schema/adoc.lifecycle_mapping.v0.schema.json",
        ),
        (
            "adoc.semantic_context.v0.schema.json",
            "adoc://agent/v0/schema/adoc.semantic_context.v0.schema.json",
        ),
        (
            "adoc.semantic_assessment.v0.schema.json",
            "adoc://agent/v0/schema/adoc.semantic_assessment.v0.schema.json",
        ),
        (
            "adoc.executor_qualification.v0.schema.json",
            "adoc://agent/v0/schema/adoc.executor_qualification.v0.schema.json",
        ),
        (
            "adoc.source_assertion.v0.schema.json",
            "adoc://agent/v0/schema/adoc.source_assertion.v0.schema.json",
        ),
        (
            "adoc.source_binding.v0.schema.json",
            "adoc://agent/v0/schema/adoc.source_binding.v0.schema.json",
        ),
        (
            "adoc.source_record.v0.schema.json",
            "adoc://agent/v0/schema/adoc.source_record.v0.schema.json",
        ),
        (
            "adoc.source_record.v1.schema.json",
            "adoc://agent/v0/schema/adoc.source_record.v1.schema.json",
        ),
        (
            "adoc.work_request.v0.schema.json",
            "adoc://agent/v0/schema/adoc.work_request.v0.schema.json",
        ),
        (
            "adoc.work_result.v0.schema.json",
            "adoc://agent/v0/schema/adoc.work_result.v0.schema.json",
        ),
        (
            "agentdoc.connector_capabilities.v0.schema.json",
            "adoc://agent/v0/schema/agentdoc.connector_capabilities.v0.schema.json",
        ),
    ] {
        assert_eq!(
            schema(name)["$id"],
            expected_id,
            "$id of {name} must match the URI it is published at"
        );
    }
}

#[test]
fn semantic_context_schema_accepts_the_envelope_and_rejects_unknown_fields() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let instance = json!({
        "schema_version": "adoc.semantic_context.v0",
        "evaluation_date": "2026-08-24",
        "subject_revision": { "system": "git", "value": "subject-sha" },
        "source_revision": { "system": "git", "value": "source-sha" },
        "base_revision": { "system": "git", "value": "base-sha" },
        "head_revision": { "system": "git", "value": "head-sha" },
        "basis": {
            "assessment_digest": digest,
            "knowledge_basis": { "kind": "graph_artifact", "digest": digest }
        },
        "selection": {
            "algorithm": "changed-and-related",
            "version": "1",
            "authorized_scope": ["repo:billing"]
        },
        "capability_policy": {
            "version": "semantic-context-policy-v1",
            "rules": [
                { "reason": "permission", "outcome": "insufficient" },
                { "reason": "retention", "outcome": "insufficient" },
                { "reason": "source_outage", "outcome": "failed" },
                { "reason": "truncation", "outcome": "insufficient" },
                { "reason": "resource_limit", "outcome": "failed" }
            ]
        },
        "context_classes": [{
            "class_id": "changed_knowledge",
            "requirement": "required",
            "byte_budget": 4096
        }],
        "items": [
            {
                "handle_id": "billing-ready",
                "class_id": "changed_knowledge",
                "scope_ref": "repo:billing",
                "handle": {
                    "kind": "knowledge_object",
                    "object_id": "billing.ready",
                    "semantic_hash": digest
                },
                "content": { "body": "Billing is ready." },
                "truncated": false
            },
            {
                "handle_id": "billing-diff",
                "class_id": "changed_knowledge",
                "scope_ref": "repo:billing",
                "handle": {
                    "kind": "diff_hunk",
                    "changed_source_id": "docs/billing.adoc",
                    "hunk_digest": digest
                },
                "content": "diff content",
                "truncated": false
            },
            {
                "handle_id": "billing-assertion",
                "class_id": "changed_knowledge",
                "scope_ref": "repo:billing",
                "handle": {
                    "kind": "source_assertion",
                    "source_assertion_id": "assertion-1",
                    "source_record_id": "record-1"
                },
                "content": "assertion content",
                "truncated": false
            },
            {
                "handle_id": "billing-binding",
                "class_id": "changed_knowledge",
                "scope_ref": "repo:billing",
                "handle": {
                    "kind": "source_binding",
                    "object_id": "billing.ready"
                },
                "content": "binding content",
                "truncated": false
            },
            {
                "handle_id": "billing-evidence",
                "class_id": "changed_knowledge",
                "scope_ref": "repo:billing",
                "handle": {
                    "kind": "evidence",
                    "object_id": "billing.ready",
                    "evidence_index": 0
                },
                "content": "evidence content",
                "truncated": false
            }
        ],
        "unavailability": [],
        "coverage": [{
            "class_id": "changed_knowledge",
            "requirement": "required",
            "item_count": 5,
            "included_bytes": 100,
            "byte_budget": 4096,
            "truncated": false,
            "unavailable_count": 0,
            "reasons": [],
            "complete": true
        }],
        "outcome": "ready",
        "context_digest": digest
    });
    assert_valid("adoc.semantic_context.v0.schema.json", &instance);

    let mut unknown = instance;
    unknown["unexpected"] = json!(true);
    assert!(
        !schema_accepts("adoc.semantic_context.v0.schema.json", &unknown),
        "semantic context must reject unknown fields"
    );
}

#[test]
fn semantic_assessment_schema_accepts_the_envelope_and_rejects_unknown_fields() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let instance = json!({
        "schema_version": "adoc.semantic_assessment.v0",
        "context_digest": digest,
        "base_revision": { "system": "git", "value": "base-sha" },
        "head_revision": { "system": "git", "value": "head-sha" },
        "identity": { "provider": "codex", "model": "gpt-5" },
        "materiality_policy_version": "adoc.materiality.v0",
        "scope": { "handle_ids": ["hunk-a", "object-a"] },
        "findings": [{
            "finding_id": "finding-001",
            "classification": "extends_existing_knowledge",
            "affected_objects": [{
                "object_id": "billing.policy",
                "content_hash": digest
            }],
            "citations": ["hunk-a", "object-a"],
            "materiality": "material",
            "proposed_disposition": "update_existing",
            "candidate_updates": [{
                "object_id": "billing.policy",
                "body": "Updated billing policy.",
                "fields": {}
            }],
            "unresolved_questions": [],
            "explanation": "The new branch changes durable billing behavior."
        }]
    });
    assert_valid("adoc.semantic_assessment.v0.schema.json", &instance);

    let mut human = instance.clone();
    human["identity"] = json!({
        "provider": "human",
        "model": "structured-assessment-v0"
    });
    assert_valid("adoc.semantic_assessment.v0.schema.json", &human);
    human["human_review"] = json!({
        "authority": "semantic_review",
        "reviewing_principal_id": "principal:reviewer",
        "requesting_principal_id": "principal:author",
        "independence": "independent"
    });
    assert_valid("adoc.semantic_assessment.v0.schema.json", &human);
    human["human_review"]["authority"] = json!("proposal_approval");
    assert!(
        !schema_accepts("adoc.semantic_assessment.v0.schema.json", &human),
        "semantic assessment cannot carry proposal-approval authority"
    );

    let mut unknown = instance;
    unknown["unexpected"] = json!(true);
    assert!(
        !schema_accepts("adoc.semantic_assessment.v0.schema.json", &unknown),
        "semantic assessment must reject unknown fields"
    );
}

#[test]
fn executor_qualification_schema_accepts_model_and_human_records() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let mut model = json!({
        "schema_version": "adoc.executor_qualification.v0",
        "qualification_id": "qual-code-assessment-v1",
        "capability": {"name": "code_change_assessment", "version": "1"},
        "subject": {
            "kind": "model",
            "provider": "codex",
            "executor_digest": digest,
            "model_digest": digest,
            "config_digest": digest,
            "configuration": {
                "model_revision_digest": digest,
                "quantization_digest": digest,
                "system_prompt_task_digest": digest,
                "context_strategy_digest": digest,
                "output_constraints_digest": digest,
                "toolset_digest": digest,
                "inference_parameters_digest": digest,
                "safety_configuration_digest": digest,
                "adapter_implementation_digest": digest
            }
        },
        "protocol": {"valid": true, "version": "semantic-executor-v1"},
        "agentdoc_evaluation": {
            "kind": "capability",
            "qualified": true,
            "evidence_ref": "suite:code-assessment-v1"
        },
        "organization_approval": {
            "approved": true,
            "scope": ["repo:billing"],
            "risk": ["high"],
            "deployment": ["customer_worker"],
            "policy_digest": digest
        },
        "runtime_policy": {
            "eligible": true,
            "operation_digest": digest,
            "policy_digest": digest
        }
    });
    assert_valid("adoc.executor_qualification.v0.schema.json", &model);

    for invalid_name in [" ", "code\tassessment"] {
        let mut invalid_text = model.clone();
        invalid_text["capability"]["name"] = json!(invalid_name);
        assert!(!schema_accepts(
            "adoc.executor_qualification.v0.schema.json",
            &invalid_text
        ));
    }

    let mut mismatched = model.clone();
    mismatched["agentdoc_evaluation"] = json!({
        "kind": "authenticated_permission",
        "qualified": true,
        "principal_id": "principal:reviewer-1",
        "permission_policy_digest": digest
    });
    assert!(!schema_accepts(
        "adoc.executor_qualification.v0.schema.json",
        &mismatched
    ));

    model["subject"] = json!({
        "kind": "human",
        "principal_id": "principal:reviewer-1",
        "executor_digest": digest,
        "config_digest": digest,
        "permission_policy_digest": digest
    });
    model["agentdoc_evaluation"] = json!({
        "kind": "authenticated_permission",
        "qualified": true,
        "principal_id": "principal:reviewer-1",
        "permission_policy_digest": digest
    });
    assert_valid("adoc.executor_qualification.v0.schema.json", &model);

    model["agentdoc_evaluation"] = json!({
        "kind": "capability",
        "qualified": true,
        "evidence_ref": "suite:code-assessment-v1"
    });
    assert!(!schema_accepts(
        "adoc.executor_qualification.v0.schema.json",
        &model
    ));

    model["agentdoc_evaluation"] = json!({
        "kind": "authenticated_permission",
        "qualified": true,
        "principal_id": "principal:reviewer-1",
        "permission_policy_digest": digest
    });
    model["unexpected"] = json!(true);
    assert!(!schema_accepts(
        "adoc.executor_qualification.v0.schema.json",
        &model
    ));
}

#[test]
fn semantic_executor_schemas_share_closed_adapter_and_receipt_shapes() {
    let digest = format!("sha256:{}", "a".repeat(64));
    let input = json!({
        "schema_version": "adoc.semantic_context_input.v0",
        "evaluation_date": "2026-08-24",
        "subject_revision": {"system": "git", "value": "head"},
        "source_revision": {"system": "git", "value": "head"},
        "base_revision": {"system": "git", "value": "base"},
        "head_revision": {"system": "git", "value": "head"},
        "basis": {
            "assessment_digest": digest,
            "knowledge_basis": {"kind": "graph_artifact", "digest": digest}
        },
        "selection": {
            "algorithm": "action-bounded-lexical",
            "version": "1",
            "authorized_scope": []
        },
        "capability_policy": {
            "version": "semantic-context-policy-v1",
            "rules": [
                {"reason": "permission", "outcome": "insufficient"},
                {"reason": "retention", "outcome": "insufficient"},
                {"reason": "source_outage", "outcome": "failed"},
                {"reason": "truncation", "outcome": "insufficient"},
                {"reason": "resource_limit", "outcome": "insufficient"}
            ]
        },
        "context_classes": [],
        "items": [],
        "unavailability": []
    });
    assert_valid("adoc.semantic_context_input.v0.schema.json", &input);
    let mut incomplete_input = input.clone();
    incomplete_input["basis"] = json!({});
    assert!(!schema_accepts(
        "adoc.semantic_context_input.v0.schema.json",
        &incomplete_input
    ));
    let mut context = input;
    context["schema_version"] = json!("adoc.semantic_context.v0");
    context["selection"]["authorized_scope"] = json!(["repo:agentdoc/test"]);
    context["context_classes"] = json!([{
        "class_id": "changed_knowledge",
        "requirement": "required",
        "byte_budget": 4096
    }]);
    context["items"] = json!([{
        "handle_id": "hunk-a",
        "class_id": "changed_knowledge",
        "scope_ref": "repo:agentdoc/test",
        "handle": {
            "kind": "diff_hunk",
            "changed_source_id": "src/billing.rs",
            "hunk_digest": digest
        },
        "content": {"diff": "+ durable billing behavior"},
        "truncated": false
    }]);
    context["coverage"] = json!([{
        "class_id": "changed_knowledge",
        "requirement": "required",
        "item_count": 1,
        "included_bytes": 37,
        "byte_budget": 4096,
        "truncated": false,
        "unavailable_count": 0,
        "reasons": [],
        "complete": true
    }]);
    context["outcome"] = json!("ready");
    context["context_digest"] = json!(digest.clone());

    let adapter = json!({
        "kind": "codex",
        "provider": "codex",
        "model": "gpt-5.6-codex",
        "endpoint_class": "public_provider",
        "endpoint_id": "openai",
        "executor_digest": digest,
        "model_digest": digest,
        "config_digest": digest
    });
    let mut request = json!({
        "schema_version": "adoc.semantic_executor_request.v0",
        "request_id": "request-1",
        "capability": "code_change_assessment",
        "adapter": adapter,
        "task_digest": digest,
        "prompt": {
            "contract_version": "semantic-assessment-task-v1",
            "digest": digest,
            "instructions": "Return structured JSON."
        },
        "timeout_seconds": 600,
        "context": context
    });
    assert_valid("adoc.semantic_executor_request.v0.schema.json", &request);
    let mut missing_diff = request.clone();
    missing_diff["context"]["items"] = json!([]);
    assert!(!schema_accepts(
        "adoc.semantic_executor_request.v0.schema.json",
        &missing_diff
    ));
    let mut reserved_human_provider = request.clone();
    reserved_human_provider["adapter"]["provider"] = json!("human");
    assert!(!schema_accepts(
        "adoc.semantic_executor_request.v0.schema.json",
        &reserved_human_provider
    ));
    let mut reserved_human_endpoint = request.clone();
    reserved_human_endpoint["adapter"]["endpoint_class"] = json!("human");
    assert!(!schema_accepts(
        "adoc.semantic_executor_request.v0.schema.json",
        &reserved_human_endpoint
    ));
    let mut human_kind = request.clone();
    human_kind["adapter"]["kind"] = json!("human");
    assert!(!schema_accepts(
        "adoc.semantic_executor_request.v0.schema.json",
        &human_kind
    ));
    let mut human_request = request.clone();
    human_request["adapter"]["kind"] = json!("human");
    human_request["adapter"]["provider"] = json!("human");
    human_request["adapter"]["endpoint_class"] = json!("human");
    assert_valid(
        "adoc.semantic_executor_request.v0.schema.json",
        &human_request,
    );
    human_request["human_review"] = json!({
        "reviewing_principal_id": "principal:reviewer",
        "requesting_principal_id": "principal:author"
    });
    assert_valid(
        "adoc.semantic_executor_request.v0.schema.json",
        &human_request,
    );
    request["adapter"]["kind"] = json!("shell_magic");
    assert!(!schema_accepts(
        "adoc.semantic_executor_request.v0.schema.json",
        &request
    ));

    let completed = json!({
        "schema_version": "adoc.semantic_executor_receipt.v0",
        "request_id": "request-1",
        "request_digest": digest,
        "capability": "code_change_assessment",
        "adapter": {
            "kind": "human",
            "provider": "human",
            "model": "authenticated-principal",
            "endpoint_class": "human",
            "endpoint_id": "human-structured",
            "executor_digest": digest,
            "model_digest": digest,
            "config_digest": digest
        },
        "task_digest": digest,
        "prompt_digest": digest,
        "context_digest": digest,
        "outcome": "completed",
        "assessment_digest": digest
    });
    assert_valid("adoc.semantic_executor_receipt.v0.schema.json", &completed);

    let mut impossible_adapter = completed.clone();
    impossible_adapter["adapter"]["endpoint_class"] = json!("public_provider");
    assert!(!schema_accepts(
        "adoc.semantic_executor_receipt.v0.schema.json",
        &impossible_adapter
    ));
    let mut impossible_provider = completed.clone();
    impossible_provider["adapter"]["provider"] = json!("other");
    assert!(!schema_accepts(
        "adoc.semantic_executor_receipt.v0.schema.json",
        &impossible_provider
    ));

    let mut invalid = completed;
    invalid["failure_code"] = json!("executor.failed");
    assert!(!schema_accepts(
        "adoc.semantic_executor_receipt.v0.schema.json",
        &invalid
    ));
}

/// The frozen v0 schema must accept every envelope real v0 emitters produced,
/// including the additive V6.5.3 `resolved_questions` field on `adoc why`
/// records — otherwise the "v0 stays published forever" guarantee is hollow.
#[test]
fn legacy_v0_schema_accepts_resolved_questions() {
    let legacy_instance = json!({
        "schema_version": "adoc.retrieval.v0",
        "records": [{
            "id": "billing.ready",
            "kind": "claim",
            "content_hash": "sha256:legacy",
            "body": "Billing docs are ready.",
            "source": { "path": "docs/index.adoc", "line": 3, "column": 1 },
            "relations": { "depends_on": [], "supersedes": [], "related_to": [] },
            "resolved_questions": ["q.billing.launch"]
        }],
        "diagnostics": []
    });
    assert_valid("retrieval-envelope.v0.json", &legacy_instance);
}

/// V1.7.1 (ADR-0040 §1): prose record ids follow `<page-id>#block-NNNN`, so
/// the v1 schema must reject values that merely contain the block marker.
#[test]
fn v1_schema_anchors_the_prose_record_id_pattern() {
    let prose_envelope = |id: &str| {
        json!({
            "schema_version": "adoc.retrieval.v1",
            "records": [{
                "record_type": "prose",
                "id": id,
                "page_id": "guides.onboarding",
                "block_kind": "paragraph",
                "text": "Billing onboarding starts with a sandbox workspace.",
                "source": { "path": "docs/guides/onboarding.md", "line": 3 }
            }],
            "diagnostics": []
        })
    };
    assert_valid(
        "retrieval-envelope.json",
        &prose_envelope("guides.onboarding#block-0001"),
    );

    let v1_schema = schema("retrieval-envelope.json");
    let validator = jsonschema::validator_for(&v1_schema).expect("schema compiles");
    for malformed in ["#block-", "foo #block- bar", "guides.onboarding#block-"] {
        assert!(
            !validator.is_valid(&prose_envelope(malformed)),
            "the v1 schema must reject the malformed prose id {malformed:?}"
        );
    }
}

/// The adoc.search.v2 search artifact wire shape (prose entries since V1.7.2,
/// ADR-0040; v2 since E1.1.T5) —
/// entry_kind-discriminated embeddings over Knowledge Objects and prose.
/// The artifact is an internal build output (not an MCP resource), but its
/// serialized JSON shape is public and contract-guarded like the envelopes.
#[test]
fn validates_built_search_artifact_against_v1_contract_schema() {
    let (workspace, _server, _base_hash) = project_with_built_graph();

    let search_artifact: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(workspace.path().join("dist/docs.search.json"))
            .expect("search artifact is written by the build"),
    )
    .expect("search artifact parses");

    assert_valid("search-artifact.json", &search_artifact);

    let embeddings = search_artifact["embeddings"]
        .as_array()
        .expect("embeddings array");
    assert!(
        embeddings
            .iter()
            .any(|entry| entry["entry_kind"] == "knowledge_object"),
        "the fixture claim must be embedded"
    );
    assert!(
        embeddings
            .iter()
            .any(|entry| entry["entry_kind"] == "prose"),
        "the fixture .md paragraph must be embedded (adoc.search.v2)"
    );
}

#[test]
fn authorization_decision_schema_pins_replay_bindings() {
    assert!(
        CANONICAL_SOURCE_ACL_OBSERVED_AT < CANONICAL_SOURCE_ACL_EXPIRED_AT
            && CANONICAL_SOURCE_ACL_EXPIRED_AT < CANONICAL_EVALUATION_TIME
            && CANONICAL_EVALUATION_TIME < CANONICAL_SOURCE_ACL_EXPIRES_AT,
        "canonical ACL evidence must predate evaluation and remain fresh"
    );
    let mut current_authorization = canonical_source_acl_join();
    current_authorization
        .as_object_mut()
        .expect("current authorization fixture is an object")
        .extend(
            json!({
                "role": "current_authorization",
                "current_acl_id": "acl-authorization-1",
                "result": "allow",
                "principal_id": "principal-1",
                "external_identity_link_id": "external-identity-link-1",
                "expires_at": CANONICAL_SOURCE_ACL_EXPIRES_AT,
                "connector_available": true
            })
            .as_object()
            .expect("current authorization fields are an object")
            .clone(),
        );
    let resource = json!({
        "workspace_id": current_authorization["workspace_id"].clone(),
        "connector_id": current_authorization["connector_id"].clone(),
        "source_container_id": current_authorization["source_container_id"].clone(),
        "resource": current_authorization["source"].clone(),
        "knowledge_kind": "policy",
        "object_id": "policy.refunds.enterprise"
    });
    let role_scope = json!({
        "workspace_id": resource["workspace_id"].clone(),
        "knowledge_kind": resource["knowledge_kind"].clone()
    });
    let direct_scope = json!({
        "workspace_id": resource["workspace_id"].clone()
    });
    let decision = json!({
        "schema_version": "adoc.authorization_decision.v0",
        "principal": {
            "id": current_authorization["principal_id"].clone(),
            "type": "human",
            "freshness": "current"
        },
        "permission": "proposal.approve",
        "resource": resource,
        "evaluation_time": CANONICAL_EVALUATION_TIME,
        "consequential": true,
        "hard_deny": false,
        "grants": [{
            "grant_id": "assignment-1",
            "source": "role_assignment",
            "effect": "allow",
            "permission": "proposal.approve",
            "scope": role_scope.clone(),
            "role": { "id": "builtin:curator", "version": 1 }
        }],
        "membership_evidence": "not_applicable",
        "source_acl_ceiling": {
            "required": true,
            "result": "allow",
            "check_context": {
                "role": "attempted_check",
                "external_identity_link_id": current_authorization["external_identity_link_id"].clone(),
                "acl_policy_version": "github-acl-v1"
            },
            "snapshot_id": current_authorization["snapshot_id"].clone(),
            "current_authorization": current_authorization
        },
        "visibility": "allow",
        "action_policy": "allow",
        "policy_version": "authz-policy-v1",
        "result": "allow",
        "reason": "allowed",
        "basis": {
            "grant_id": "assignment-1",
            "source": "role_assignment",
            "effect": "allow",
            "scope_match": role_scope,
            "role": { "id": "builtin:curator", "version": 1 }
        }
    });
    let mut direct_human = decision.clone();
    direct_human["grants"][0] = json!({
        "grant_id": "exceptional-human-grant",
        "source": "direct_grant",
        "effect": "allow",
        "permission": "proposal.approve",
        "scope": direct_scope.clone(),
        "expires_at": "2026-08-24T12:00:00Z",
        "exceptional_reason": "incident response"
    });
    direct_human["basis"] = json!({
        "grant_id": "exceptional-human-grant",
        "source": "direct_grant",
        "effect": "allow",
        "scope_match": direct_scope
    });
    let mut direct_human_with_multiline_reason = direct_human.clone();
    direct_human_with_multiline_reason["grants"][0]["exceptional_reason"] =
        json!("suspected breach\npayments pipeline");
    let mut direct_human_with_blank_reason = direct_human.clone();
    direct_human_with_blank_reason["grants"][0]["exceptional_reason"] = json!("   ");
    let mut expiring_role = decision.clone();
    expiring_role["grants"][0]["expires_at"] = json!("2026-08-24T12:00:00Z");

    let external_group = json!({
        "id": "group-1",
        "name": "Curators",
        "membership_source": "external",
        "binding_id": "github-team-binding-1",
        "binding_mode": "authoritative_sync",
        "source_kind": "github_team",
        "binding_mode_effective_at": "2026-08-23T10:00:00Z",
        "membership_observation": {
            "id": "membership-observation-1",
            "external_identity_link_id": "external-identity-link-1",
            "source_event_id": "github-team-event-1",
            "observed_at": "2026-08-23T11:00:00Z",
            "effective_at": "2026-08-23T11:00:30Z",
            "fresh_until": "2026-08-23T12:05:00Z",
            "member_present": true,
            "nested": false
        }
    });
    let mut external_group_grant = decision.clone();
    external_group_grant["membership_evidence"] = json!("current");
    external_group_grant["grants"][0]["group"] = external_group.clone();
    external_group_grant["basis"]["group"] = external_group;

    let manual_group = json!({
        "id": "group-2",
        "name": "Incident responders",
        "membership_source": "manual",
        "membership_id": "manual-membership-1",
        "membership_created_at": "2026-08-23T10:00:00Z"
    });
    let mut manual_group_grant = decision.clone();
    manual_group_grant["membership_evidence"] = json!("current");
    manual_group_grant["grants"][0]["group"] = manual_group.clone();
    manual_group_grant["basis"]["group"] = manual_group;

    let mut manual_group_direct_grant = decision.clone();
    manual_group_direct_grant["membership_evidence"] = json!("current");
    manual_group_direct_grant["principal"]["type"] = json!("service");
    manual_group_direct_grant["grants"][0] = json!({
        "grant_id": "group-scoped-grant-1",
        "source": "direct_grant",
        "effect": "allow",
        "permission": "proposal.approve",
        "scope": { "workspace_id": "workspace-1" },
        "expires_at": "2026-08-24T12:00:00Z",
        "group": manual_group_grant["grants"][0]["group"].clone()
    });
    manual_group_direct_grant["basis"] = json!({
        "grant_id": "group-scoped-grant-1",
        "source": "direct_grant",
        "effect": "allow",
        "scope_match": { "workspace_id": "workspace-1" },
        "group": manual_group_grant["basis"]["group"].clone()
    });

    let mut external_group_without_binding = external_group_grant.clone();
    external_group_without_binding["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("binding_id");
    let mut external_basis_without_mode = external_group_grant.clone();
    external_basis_without_mode["basis"]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("binding_mode");
    let mut external_group_without_source_kind = external_group_grant.clone();
    external_group_without_source_kind["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("source_kind");
    let mut external_group_without_observation = external_group_grant.clone();
    external_group_without_observation["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("membership_observation");
    let mut external_group_without_mode_effectivity = external_group_grant.clone();
    external_group_without_mode_effectivity["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("binding_mode_effective_at");
    let mut absent_membership_observation = external_group_grant.clone();
    absent_membership_observation["grants"][0]["group"]["membership_observation"]["member_present"] =
        json!(false);
    let mut nested_membership_observation = external_group_grant.clone();
    nested_membership_observation["grants"][0]["group"]["membership_observation"]["nested"] =
        json!(true);
    let mut invalid_external_observed_at = external_group_grant.clone();
    invalid_external_observed_at["grants"][0]["group"]["membership_observation"]["observed_at"] =
        json!("not-a-time");
    let mut external_group_without_effective_at = external_group_grant.clone();
    external_group_without_effective_at["grants"][0]["group"]["membership_observation"]
        .as_object_mut()
        .expect("membership observation")
        .remove("effective_at");
    let mut invalid_external_effective_at = external_group_grant.clone();
    invalid_external_effective_at["grants"][0]["group"]["membership_observation"]["effective_at"] =
        json!("not-a-time");
    let mut external_group_without_fresh_until = external_group_grant.clone();
    external_group_without_fresh_until["grants"][0]["group"]["membership_observation"]
        .as_object_mut()
        .expect("membership observation")
        .remove("fresh_until");
    let mut invalid_external_fresh_until = external_group_grant.clone();
    invalid_external_fresh_until["grants"][0]["group"]["membership_observation"]["fresh_until"] =
        json!("not-a-time");
    let mut multiline_group_name = manual_group_grant.clone();
    multiline_group_name["grants"][0]["group"]["name"] = json!("Curators\nadmin");
    let mut suggestion_group_grant = external_group_grant.clone();
    suggestion_group_grant["grants"][0]["group"]["binding_mode"] = json!("suggestion_only");
    let mut disabled_group_basis = external_group_grant.clone();
    disabled_group_basis["basis"]["group"]["binding_mode"] = json!("disabled");
    let mut unknown_group_source = external_group_grant.clone();
    unknown_group_source["grants"][0]["group"]["source_kind"] = json!("custom_directory");
    let mut manual_group_with_binding = manual_group_grant.clone();
    manual_group_with_binding["grants"][0]["group"]["binding_id"] = json!("github-team-binding-1");
    let mut manual_group_without_membership_created_at = manual_group_grant.clone();
    manual_group_without_membership_created_at["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("membership_created_at");
    let mut manual_group_without_membership_id = manual_group_grant.clone();
    manual_group_without_membership_id["grants"][0]["group"]
        .as_object_mut()
        .expect("group object")
        .remove("membership_id");
    let mut manual_group_with_external_observation = manual_group_grant.clone();
    manual_group_with_external_observation["grants"][0]["group"]["membership_observation"] =
        external_group_grant["grants"][0]["group"]["membership_observation"].clone();
    let mut external_group_with_manual_membership_time = external_group_grant.clone();
    external_group_with_manual_membership_time["grants"][0]["group"]["membership_created_at"] =
        json!("2026-08-23T10:00:00Z");
    let mut basis_group_without_group_grant = decision.clone();
    basis_group_without_group_grant["basis"]["group"] =
        manual_group_grant["basis"]["group"].clone();
    let mut group_with_not_applicable_membership_evidence = external_group_grant.clone();
    group_with_not_applicable_membership_evidence["membership_evidence"] = json!("not_applicable");

    let mut decision_without_membership_evidence = decision.clone();
    decision_without_membership_evidence
        .as_object_mut()
        .expect("decision object")
        .remove("membership_evidence");

    let mut no_acl_ceiling = decision.clone();
    no_acl_ceiling["source_acl_ceiling"] = json!({
        "required": false,
        "result": "not_applicable"
    });
    let mut optional_acl_with_snapshot = no_acl_ceiling.clone();
    optional_acl_with_snapshot["source_acl_ceiling"]["snapshot_id"] =
        decision["source_acl_ceiling"]["snapshot_id"].clone();
    let mut optional_snapshot_without_source_scope = optional_acl_with_snapshot.clone();
    optional_snapshot_without_source_scope["resource"] = json!({
        "workspace_id": decision["resource"]["workspace_id"].clone(),
        "knowledge_kind": decision["resource"]["knowledge_kind"].clone(),
        "object_id": decision["resource"]["object_id"].clone()
    });

    let mut denied = no_acl_ceiling.clone();
    denied["grants"] = json!([]);
    denied["visibility"] = json!("not_applicable");
    denied["action_policy"] = json!("not_applicable");
    denied["result"] = json!("deny");
    denied["reason"] = json!("no_grant");
    denied["basis"] = json!(null);
    let external_absence = json!({
        "group_id": "group-1",
        "group_name": "Curators",
        "membership_source": "external",
        "binding_id": "github-team-binding-1",
        "binding_mode": "authoritative_sync",
        "source_kind": "github_team",
        "binding_mode_effective_at": "2026-08-23T10:00:00Z",
        "membership_observation": {
            "id": "membership-observation-absent-1",
            "external_identity_link_id": "external-identity-link-1",
            "source_event_id": "github-team-event-absent-1",
            "observed_at": "2026-08-23T11:00:00Z",
            "effective_at": "2026-08-23T11:00:30Z",
            "fresh_until": "2026-08-23T12:05:00Z",
            "member_present": false,
            "nested": false
        }
    });
    let mut no_grant_with_external_absence = denied.clone();
    no_grant_with_external_absence["membership_evidence"] = json!("current");
    no_grant_with_external_absence["membership_absence_evidence"] = json!([external_absence]);
    let mut no_grant_with_manual_absence = denied.clone();
    no_grant_with_manual_absence["membership_evidence"] = json!("current");
    no_grant_with_manual_absence["membership_absence_evidence"] = json!([{
        "group_id": "group-2",
        "group_name": "Incident responders",
        "membership_source": "manual"
    }]);
    let no_grant_without_absence_evidence = {
        let mut instance = no_grant_with_external_absence.clone();
        instance
            .as_object_mut()
            .expect("decision object")
            .remove("membership_absence_evidence");
        instance
    };
    let mut no_grant_with_positive_absence_observation = no_grant_with_external_absence.clone();
    no_grant_with_positive_absence_observation["membership_absence_evidence"][0]["membership_observation"]
        ["member_present"] = json!(true);
    let mut no_grant_not_applicable_with_absence = no_grant_with_external_absence.clone();
    no_grant_not_applicable_with_absence["membership_evidence"] = json!("not_applicable");
    let mut no_grant_with_group_but_no_absence = denied.clone();
    no_grant_with_group_but_no_absence["grants"] = external_group_grant["grants"].clone();
    no_grant_with_group_but_no_absence["grants"][0]["permission"] = json!("workspace.read");
    no_grant_with_group_but_no_absence["membership_evidence"] = json!("current");
    no_grant_with_group_but_no_absence["membership_absence_evidence"] = json!([]);
    let mut no_grant_with_no_membership_facts = denied.clone();
    no_grant_with_no_membership_facts["membership_evidence"] = json!("current");
    no_grant_with_no_membership_facts["membership_absence_evidence"] = json!([]);
    let external_unavailability_evidence = json!([{
        "group_id": "group-1",
        "group_name": "Curators",
        "membership_source": "external",
        "binding_id": "github-team-binding-1",
        "binding_mode": "authoritative_sync",
        "binding_mode_effective_at": "2026-08-23T10:00:00Z",
        "source_kind": "github_team",
        "external_identity_link_id": "external-identity-link-1",
        "state": "connector_read_failed",
        "state_record_id": "membership-read-failure-1"
    }]);
    let mut membership_evidence_unavailable = denied.clone();
    membership_evidence_unavailable["consequential"] = json!(true);
    membership_evidence_unavailable["result"] = json!("insufficient_context");
    membership_evidence_unavailable["reason"] = json!("membership_evidence_unavailable");
    membership_evidence_unavailable["membership_evidence"] = json!("insufficient_context");
    membership_evidence_unavailable["membership_unavailability_evidence"] =
        external_unavailability_evidence.clone();
    let mut manual_membership_evidence_unavailable = membership_evidence_unavailable.clone();
    manual_membership_evidence_unavailable["membership_unavailability_evidence"] = json!([{
        "group_id": "group-2",
        "group_name": "Incident responders",
        "membership_source": "manual",
        "state": "lifecycle_unavailable",
        "state_record_id": "manual-membership-read-failure-1"
    }]);
    let mut membership_evidence_unavailable_without_provenance =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_provenance
        .as_object_mut()
        .expect("decision object")
        .remove("membership_unavailability_evidence");
    let mut membership_evidence_unavailable_with_empty_provenance =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_with_empty_provenance["membership_unavailability_evidence"] =
        json!([]);
    let mut resolved_membership_with_unavailability = decision.clone();
    resolved_membership_with_unavailability["membership_unavailability_evidence"] =
        external_unavailability_evidence.clone();
    let mut membership_evidence_unavailable_without_state_record =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_state_record["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("unavailability evidence object")
        .remove("state_record_id");
    let mut membership_evidence_unavailable_without_group_name =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_group_name["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("unavailability evidence object")
        .remove("group_name");
    let mut membership_evidence_unavailable_without_binding_mode =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_binding_mode["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("unavailability evidence object")
        .remove("binding_mode");
    let mut membership_evidence_unavailable_without_binding_epoch =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_binding_epoch["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("unavailability evidence object")
        .remove("binding_mode_effective_at");
    let mut manual_unavailability_without_group_name =
        manual_membership_evidence_unavailable.clone();
    manual_unavailability_without_group_name["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("manual unavailability evidence object")
        .remove("group_name");
    let mut unavailable_membership_from_disabled_binding = membership_evidence_unavailable.clone();
    unavailable_membership_from_disabled_binding["membership_unavailability_evidence"][0]["binding_mode"] =
        json!("disabled");
    let mut membership_evidence_unavailable_with_unknown_state =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_with_unknown_state["membership_unavailability_evidence"][0]["state"] =
        json!("silent_retry");
    let mut mixed_group_membership_evidence_unavailable = membership_evidence_unavailable.clone();
    mixed_group_membership_evidence_unavailable["grants"] = external_group_grant["grants"].clone();
    mixed_group_membership_evidence_unavailable["grants"][0]["permission"] =
        json!("workspace.read");
    let mut nonconsequential_membership_evidence_unavailable =
        membership_evidence_unavailable.clone();
    nonconsequential_membership_evidence_unavailable["consequential"] = json!(false);
    nonconsequential_membership_evidence_unavailable["result"] = json!("deny");
    let mut membership_evidence_unavailable_with_basis = membership_evidence_unavailable.clone();
    membership_evidence_unavailable_with_basis["basis"] = decision["basis"].clone();
    let mut consequential_membership_evidence_unavailable_with_deny =
        membership_evidence_unavailable.clone();
    consequential_membership_evidence_unavailable_with_deny["result"] = json!("deny");
    let mut nonconsequential_membership_evidence_unavailable_with_insufficient =
        nonconsequential_membership_evidence_unavailable.clone();
    nonconsequential_membership_evidence_unavailable_with_insufficient["result"] =
        json!("insufficient_context");
    let mut membership_evidence_unavailable_without_input = membership_evidence_unavailable.clone();
    membership_evidence_unavailable_without_input
        .as_object_mut()
        .expect("decision object")
        .remove("membership_evidence");
    let mut membership_evidence_unavailable_with_current_input =
        membership_evidence_unavailable.clone();
    membership_evidence_unavailable_with_current_input["membership_evidence"] = json!("current");
    let mut no_grant_with_unavailable_membership_evidence = denied.clone();
    no_grant_with_unavailable_membership_evidence["membership_evidence"] =
        json!("insufficient_context");
    no_grant_with_unavailable_membership_evidence["membership_unavailability_evidence"] =
        external_unavailability_evidence.clone();

    let mut insufficient = decision.clone();
    insufficient["source_acl_ceiling"] = json!({
        "required": true,
        "result": "insufficient_context",
        "check_context": decision["source_acl_ceiling"]["check_context"].clone()
    });
    insufficient["result"] = json!("insufficient_context");
    insufficient["reason"] = json!("source_acl_unavailable");
    insufficient["basis"] = json!(null);
    let insufficient_check_context = insufficient.clone();
    let mut required_check_without_context = insufficient.clone();
    required_check_without_context["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("check_context");

    let mut insufficient_with_current_acl = decision.clone();
    insufficient_with_current_acl["source_acl_ceiling"]["result"] = json!("insufficient_context");
    insufficient_with_current_acl["result"] = json!("insufficient_context");
    insufficient_with_current_acl["reason"] = json!("source_acl_unavailable");
    insufficient_with_current_acl["basis"] = json!(null);

    let mut not_applicable_with_current_acl = decision.clone();
    not_applicable_with_current_acl["source_acl_ceiling"]["required"] = json!(false);
    not_applicable_with_current_acl["source_acl_ceiling"]["result"] = json!("not_applicable");
    let mut optional_check_with_context = not_applicable_with_current_acl.clone();
    optional_check_with_context["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("current_authorization");

    let mut current_acl_result_mismatch = decision.clone();
    current_acl_result_mismatch["source_acl_ceiling"]["result"] = json!("deny");
    current_acl_result_mismatch["result"] = json!("deny");
    current_acl_result_mismatch["reason"] = json!("source_acl_denied");
    current_acl_result_mismatch["basis"] = json!(null);

    let mut allowing_ceiling_with_denying_current_acl = decision.clone();
    allowing_ceiling_with_denying_current_acl["source_acl_ceiling"]["current_authorization"]["result"] =
        json!("deny");

    let mut legacy_allow_without_current_acl = decision.clone();
    legacy_allow_without_current_acl["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("current_authorization");
    let mut definitive_source_acl_evidence_without_context = decision.clone();
    definitive_source_acl_evidence_without_context["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("check_context");

    let mut source_acl_outage = decision.clone();
    source_acl_outage["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(false);
    source_acl_outage["result"] = json!("insufficient_context");
    source_acl_outage["reason"] = json!("source_acl_unavailable");
    source_acl_outage["basis"] = json!(null);
    let mut available_connector_with_unavailable_reason = source_acl_outage.clone();
    available_connector_with_unavailable_reason["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(true);

    let mut source_acl_stale = decision.clone();
    source_acl_stale["source_acl_ceiling"]["current_authorization"]["expires_at"] =
        json!(CANONICAL_SOURCE_ACL_EXPIRED_AT);
    source_acl_stale["source_acl_ceiling"]["stale_cause"] = json!("expired");
    source_acl_stale["result"] = json!("deny");
    source_acl_stale["reason"] = json!("source_acl_stale");
    source_acl_stale["basis"] = json!(null);
    assert_eq!(
        source_acl_stale["source_acl_ceiling"]["current_authorization"]["acl_policy_version"],
        source_acl_stale["source_acl_ceiling"]["check_context"]["acl_policy_version"],
        "unchanged-policy expiry must keep observation-time and evaluation-time versions equal"
    );
    let mut source_acl_policy_superseded = decision.clone();
    source_acl_policy_superseded["source_acl_ceiling"]["stale_cause"] = json!("policy_superseded");
    source_acl_policy_superseded["source_acl_ceiling"]["check_context"]["acl_policy_version"] =
        json!("github-acl-v2");
    source_acl_policy_superseded["result"] = json!("deny");
    source_acl_policy_superseded["reason"] = json!("source_acl_stale");
    source_acl_policy_superseded["basis"] = json!(null);
    assert_ne!(
        source_acl_policy_superseded["source_acl_ceiling"]["current_authorization"]["acl_policy_version"],
        source_acl_policy_superseded["source_acl_ceiling"]["check_context"]["acl_policy_version"],
        "policy-supersession replay must retain distinct observation-time and evaluation-time versions"
    );
    let mut stale_without_recorded_cause = source_acl_policy_superseded.clone();
    stale_without_recorded_cause["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL ceiling")
        .remove("stale_cause");
    let mut supersession_without_check_context = source_acl_policy_superseded.clone();
    supersession_without_check_context["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL ceiling")
        .remove("check_context");
    // A changed-policy expired record has this same JSON shape; only the
    // sourceAclCheckContext evaluator obligation forbids omitting its context.
    let mut unchanged_policy_expiry_without_check_context = source_acl_stale.clone();
    unchanged_policy_expiry_without_check_context["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL ceiling")
        .remove("check_context");
    let mut expired_and_policy_superseded = source_acl_policy_superseded.clone();
    expired_and_policy_superseded["source_acl_ceiling"]["stale_cause"] = json!("expired");
    expired_and_policy_superseded["source_acl_ceiling"]["current_authorization"]["expires_at"] =
        json!(CANONICAL_SOURCE_ACL_EXPIRED_AT);
    assert_ne!(
        expired_and_policy_superseded["source_acl_ceiling"]["current_authorization"]["acl_policy_version"],
        expired_and_policy_superseded["source_acl_ceiling"]["check_context"]["acl_policy_version"],
        "expiry-wins replay must retain the different evaluation-time governing version"
    );
    let mut false_stale_cause = decision.clone();
    false_stale_cause["source_acl_ceiling"]["stale_cause"] = json!("expired");

    let mut source_acl_invalidated = decision.clone();
    source_acl_invalidated["source_acl_ceiling"]["current_authorization"]["invalidated_at"] =
        json!(CANONICAL_SOURCE_ACL_EXPIRED_AT);
    source_acl_invalidated["result"] = json!("deny");
    source_acl_invalidated["reason"] = json!("source_acl_invalidated");
    source_acl_invalidated["basis"] = json!(null);

    let mut denying_source_acl_stale = source_acl_stale.clone();
    denying_source_acl_stale["source_acl_ceiling"]["result"] = json!("deny");
    denying_source_acl_stale["source_acl_ceiling"]["current_authorization"]["result"] =
        json!("deny");
    let mut denying_source_acl_invalidated = source_acl_invalidated.clone();
    denying_source_acl_invalidated["source_acl_ceiling"]["result"] = json!("deny");
    denying_source_acl_invalidated["source_acl_ceiling"]["current_authorization"]["result"] =
        json!("deny");
    let mut denying_source_acl_outage = source_acl_outage.clone();
    denying_source_acl_outage["source_acl_ceiling"]["result"] = json!("deny");
    denying_source_acl_outage["source_acl_ceiling"]["current_authorization"]["result"] =
        json!("deny");

    let mut invalidated_source_acl_during_outage = source_acl_invalidated.clone();
    invalidated_source_acl_during_outage["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(false);
    let mut unavailable_reason_with_invalidated_source_acl =
        invalidated_source_acl_during_outage.clone();
    unavailable_reason_with_invalidated_source_acl["result"] = json!("insufficient_context");
    unavailable_reason_with_invalidated_source_acl["reason"] = json!("source_acl_unavailable");
    let mut stale_source_acl_during_outage = source_acl_stale.clone();
    stale_source_acl_during_outage["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(false);

    let mut explicit_deny_during_source_acl_outage = source_acl_outage.clone();
    explicit_deny_during_source_acl_outage["grants"][0]["effect"] = json!("deny");
    explicit_deny_during_source_acl_outage["result"] = json!("deny");
    explicit_deny_during_source_acl_outage["reason"] = json!("explicit_deny");
    explicit_deny_during_source_acl_outage["basis"] = decision["basis"].clone();
    explicit_deny_during_source_acl_outage["basis"]["effect"] = json!("deny");

    let mut explicit_deny_with_invalidated_source_acl = source_acl_invalidated.clone();
    explicit_deny_with_invalidated_source_acl["grants"][0]["effect"] = json!("deny");
    explicit_deny_with_invalidated_source_acl["reason"] = json!("explicit_deny");
    explicit_deny_with_invalidated_source_acl["basis"] = decision["basis"].clone();
    explicit_deny_with_invalidated_source_acl["basis"]["effect"] = json!("deny");
    let mut hard_deny_recorded_as_lower_gate_reason =
        explicit_deny_with_invalidated_source_acl.clone();
    hard_deny_recorded_as_lower_gate_reason["hard_deny"] = json!(true);

    for field in ["connector_id", "source_container_id", "resource"] {
        let mut missing_source_scope = decision.clone();
        missing_source_scope["resource"]
            .as_object_mut()
            .expect("resource scope")
            .remove(field);
        assert!(
            !schema_accepts(
                "adoc.authorization_decision.v0.schema.json",
                &missing_source_scope
            ),
            "required ACL check with a decision resource missing {field} must be rejected"
        );
    }
    for field in ["connector_id", "source_container_id", "resource"] {
        let mut unavailable_without_source_scope = insufficient.clone();
        unavailable_without_source_scope["resource"]
            .as_object_mut()
            .expect("resource scope")
            .remove(field);
        assert!(
            !schema_accepts(
                "adoc.authorization_decision.v0.schema.json",
                &unavailable_without_source_scope
            ),
            "required ACL outage with a decision resource missing {field} must be rejected"
        );
    }
    let mut optional_acl_without_source_scope = no_acl_ceiling.clone();
    optional_acl_without_source_scope["resource"] = json!({
        "workspace_id": decision["resource"]["workspace_id"].clone(),
        "knowledge_kind": decision["resource"]["knowledge_kind"].clone(),
        "object_id": decision["resource"]["object_id"].clone()
    });
    assert!(
        schema_accepts(
            "adoc.authorization_decision.v0.schema.json",
            &optional_acl_without_source_scope
        ),
        "an optional ACL check must not require source resource scope"
    );

    let mut allow_during_source_acl_outage = decision.clone();
    allow_during_source_acl_outage["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(false);
    let mut allow_with_invalidated_source_acl = decision.clone();
    allow_with_invalidated_source_acl["source_acl_ceiling"]["current_authorization"]["invalidated_at"] =
        json!(CANONICAL_SOURCE_ACL_EXPIRED_AT);
    let mut stale_reason_without_current_acl = source_acl_stale.clone();
    stale_reason_without_current_acl["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("current_authorization");
    let mut invalidated_reason_without_marker = source_acl_invalidated.clone();
    invalidated_reason_without_marker["source_acl_ceiling"]["current_authorization"]
        .as_object_mut()
        .expect("current ACL object")
        .remove("invalidated_at");
    let mut current_acl_with_invalid_observed_at = decision.clone();
    current_acl_with_invalid_observed_at["source_acl_ceiling"]["current_authorization"]["observed_at"] =
        json!("not-a-time");
    let mut missing_policy_version = decision.clone();
    missing_policy_version
        .as_object_mut()
        .expect("object")
        .remove("policy_version");
    let mut blank_policy_version = decision.clone();
    blank_policy_version["policy_version"] = json!("   ");
    let mut multiline_workspace_id = decision.clone();
    multiline_workspace_id["resource"]["workspace_id"] = json!("workspace-1\nworkspace-2");
    let mut carriage_return_workspace_id = decision.clone();
    carriage_return_workspace_id["resource"]["workspace_id"] = json!("workspace-1\rworkspace-2");
    let mut line_separator_workspace_id = decision.clone();
    line_separator_workspace_id["resource"]["workspace_id"] =
        json!("workspace-1\u{2028}workspace-2");

    let mut unknown_result = decision.clone();
    unknown_result["result"] = json!("maybe");

    let mut direct_without_expiry = decision.clone();
    direct_without_expiry["grants"][0]["source"] = json!("direct_grant");

    let mut role_with_exception = decision.clone();
    role_with_exception["grants"][0]["exceptional_reason"] = json!("incident response");

    let mut direct_with_role = direct_human.clone();
    direct_with_role["grants"][0]["role"] = json!({ "id": "builtin:curator", "version": 1 });

    let mut human_direct_without_reason = direct_human.clone();
    human_direct_without_reason["grants"][0]
        .as_object_mut()
        .expect("grant object")
        .remove("exceptional_reason");

    let mut service_direct_without_reason = direct_human.clone();
    service_direct_without_reason["principal"]["type"] = json!("service");
    service_direct_without_reason["grants"][0]
        .as_object_mut()
        .expect("grant object")
        .remove("exceptional_reason");

    let mut direct_deny = direct_human.clone();
    direct_deny["grants"][0]["effect"] = json!("deny");
    direct_deny["result"] = json!("deny");
    direct_deny["reason"] = json!("explicit_deny");
    direct_deny["basis"]["effect"] = json!("deny");
    let mut direct_deny_without_expiry = direct_deny.clone();
    direct_deny_without_expiry["grants"][0]
        .as_object_mut()
        .expect("grant object")
        .remove("expires_at");

    let mut human_direct_deny_without_reason = direct_deny.clone();
    human_direct_deny_without_reason["grants"][0]
        .as_object_mut()
        .expect("grant object")
        .remove("exceptional_reason");

    let mut direct_basis_with_role = decision.clone();
    direct_basis_with_role["basis"]["source"] = json!("direct_grant");

    let mut role_basis_without_role = decision.clone();
    role_basis_without_role["basis"]
        .as_object_mut()
        .expect("basis object")
        .remove("role");

    let mut unknown_permission = decision.clone();
    unknown_permission["permission"] = json!("proposal.aprove");
    unknown_permission["grants"][0]["permission"] = json!("proposal.aprove");

    let mut optional_acl_allow = decision.clone();
    optional_acl_allow["source_acl_ceiling"]["required"] = json!(false);

    let mut acl_without_snapshot = decision.clone();
    acl_without_snapshot["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("snapshot_id");

    let mut hard_deny_allow = decision.clone();
    hard_deny_allow["hard_deny"] = json!(true);

    let mut stale_principal_allow = decision.clone();
    stale_principal_allow["principal"]["freshness"] = json!("expired");

    let mut denied_acl_allow = decision.clone();
    denied_acl_allow["source_acl_ceiling"]["result"] = json!("deny");

    let mut denied_visibility_allow = decision.clone();
    denied_visibility_allow["visibility"] = json!("deny");

    let mut uncertain_action_allow = decision.clone();
    uncertain_action_allow["action_policy"] = json!("insufficient_context");

    let mut allow_without_basis = decision.clone();
    allow_without_basis["basis"] = json!(null);

    let mut allowed_reason_on_deny = denied.clone();
    allowed_reason_on_deny["reason"] = json!("allowed");

    let mut no_policy_inputs = no_acl_ceiling.clone();
    no_policy_inputs["visibility"] = json!("not_applicable");
    no_policy_inputs["action_policy"] = json!("not_applicable");

    let mut unknown_scope_member = decision.clone();
    unknown_scope_member["resource"]["unknown"] = json!(true);

    let mut allow_without_grants = decision.clone();
    allow_without_grants["grants"] = json!([]);

    let mut deny_basis_on_allow = decision.clone();
    deny_basis_on_allow["basis"]["effect"] = json!("deny");

    let mut deny_only_grants_on_allow = decision.clone();
    deny_only_grants_on_allow["grants"][0]["effect"] = json!("deny");

    let mut hard_deny = denied.clone();
    hard_deny["hard_deny"] = json!(true);
    hard_deny["reason"] = json!("hard_deny");
    let mut hard_deny_before_required_acl = hard_deny.clone();
    hard_deny_before_required_acl["source_acl_ceiling"] = json!({
        "required": true,
        "result": "insufficient_context"
    });
    let mut hard_deny_with_source_acl_evidence_only = hard_deny.clone();
    hard_deny_with_source_acl_evidence_only["source_acl_ceiling"] =
        decision["source_acl_ceiling"].clone();
    hard_deny_with_source_acl_evidence_only["source_acl_ceiling"]
        .as_object_mut()
        .expect("source ACL ceiling")
        .remove("check_context");
    let mut hard_deny_with_expired_identity = hard_deny.clone();
    hard_deny_with_expired_identity["principal"]["freshness"] = json!("expired");
    hard_deny_with_expired_identity["reason"] = json!("identity_expired");
    let mut hard_deny_with_invalid_time = hard_deny.clone();
    hard_deny_with_invalid_time["evaluation_time"] = json!("");
    hard_deny_with_invalid_time["result"] = json!("insufficient_context");
    hard_deny_with_invalid_time["reason"] = json!("evaluation_time_invalid");
    let mut hard_deny_recorded_as_no_grant = hard_deny.clone();
    hard_deny_recorded_as_no_grant["reason"] = json!("no_grant");
    let mut false_hard_deny_reason = hard_deny.clone();
    false_hard_deny_reason["hard_deny"] = json!(false);
    let mut hard_deny_with_basis = hard_deny.clone();
    hard_deny_with_basis["basis"] = decision["basis"].clone();

    let mut source_acl_denied = decision.clone();
    source_acl_denied["source_acl_ceiling"]["result"] = json!("deny");
    source_acl_denied["source_acl_ceiling"]["current_authorization"]["result"] = json!("deny");
    source_acl_denied["result"] = json!("deny");
    source_acl_denied["reason"] = json!("source_acl_denied");
    source_acl_denied["basis"] = json!(null);
    let mut source_acl_denied_without_current_acl = source_acl_denied.clone();
    source_acl_denied_without_current_acl["source_acl_ceiling"]
        .as_object_mut()
        .expect("ACL object")
        .remove("current_authorization");
    let mut source_acl_denied_during_outage = source_acl_denied.clone();
    source_acl_denied_during_outage["source_acl_ceiling"]["current_authorization"]["connector_available"] =
        json!(false);
    let mut source_acl_denied_with_invalidated_acl = source_acl_denied.clone();
    source_acl_denied_with_invalidated_acl["source_acl_ceiling"]["current_authorization"]["invalidated_at"] =
        json!(CANONICAL_SOURCE_ACL_EXPIRED_AT);
    let mut explicit_deny_with_denied_source_acl = source_acl_denied.clone();
    explicit_deny_with_denied_source_acl["grants"][0]["effect"] = json!("deny");
    explicit_deny_with_denied_source_acl["reason"] = json!("explicit_deny");
    explicit_deny_with_denied_source_acl["basis"] = decision["basis"].clone();
    explicit_deny_with_denied_source_acl["basis"]["effect"] = json!("deny");
    let mut false_source_acl_denied_reason = source_acl_denied.clone();
    false_source_acl_denied_reason["source_acl_ceiling"] = json!({
        "required": false,
        "result": "not_applicable"
    });
    let mut source_acl_denied_with_basis = source_acl_denied.clone();
    source_acl_denied_with_basis["basis"] = decision["basis"].clone();
    let mut false_source_acl_unavailable_reason = insufficient.clone();
    false_source_acl_unavailable_reason["source_acl_ceiling"] = json!({
        "required": false,
        "result": "not_applicable"
    });
    let mut source_acl_unavailable_with_basis = insufficient.clone();
    source_acl_unavailable_with_basis["basis"] = decision["basis"].clone();
    let mut missing_source_acl_verdict_recorded_as_no_grant = insufficient.clone();
    missing_source_acl_verdict_recorded_as_no_grant["grants"] = json!([]);
    missing_source_acl_verdict_recorded_as_no_grant["result"] = json!("deny");
    missing_source_acl_verdict_recorded_as_no_grant["reason"] = json!("no_grant");

    let mut visibility_denied = decision.clone();
    visibility_denied["visibility"] = json!("deny");
    visibility_denied["result"] = json!("deny");
    visibility_denied["reason"] = json!("visibility_denied");
    let mut unresolved_membership_at_visibility_gate = external_group_grant.clone();
    unresolved_membership_at_visibility_gate["membership_evidence"] = json!("insufficient_context");
    unresolved_membership_at_visibility_gate["membership_unavailability_evidence"] =
        external_unavailability_evidence.clone();
    unresolved_membership_at_visibility_gate["visibility"] = json!("deny");
    unresolved_membership_at_visibility_gate["result"] = json!("deny");
    unresolved_membership_at_visibility_gate["reason"] = json!("visibility_denied");
    let mut false_visibility_denied_reason = visibility_denied.clone();
    false_visibility_denied_reason["visibility"] = json!("allow");
    let mut visibility_denied_without_basis = visibility_denied.clone();
    visibility_denied_without_basis["basis"] = json!(null);

    let mut visibility_unavailable = decision.clone();
    visibility_unavailable["visibility"] = json!("insufficient_context");
    visibility_unavailable["result"] = json!("insufficient_context");
    visibility_unavailable["reason"] = json!("visibility_unavailable");
    let mut false_visibility_unavailable_reason = visibility_unavailable.clone();
    false_visibility_unavailable_reason["visibility"] = json!("allow");
    let mut visibility_unavailable_without_basis = visibility_unavailable.clone();
    visibility_unavailable_without_basis["basis"] = json!(null);

    let mut action_policy_denied = decision.clone();
    action_policy_denied["action_policy"] = json!("deny");
    action_policy_denied["result"] = json!("deny");
    action_policy_denied["reason"] = json!("action_policy_denied");
    let mut false_action_policy_denied_reason = action_policy_denied.clone();
    false_action_policy_denied_reason["action_policy"] = json!("allow");
    let mut action_policy_denied_without_basis = action_policy_denied.clone();
    action_policy_denied_without_basis["basis"] = json!(null);

    let mut action_policy_unavailable = decision.clone();
    action_policy_unavailable["action_policy"] = json!("insufficient_context");
    action_policy_unavailable["result"] = json!("insufficient_context");
    action_policy_unavailable["reason"] = json!("action_policy_unavailable");
    let mut false_action_policy_unavailable_reason = action_policy_unavailable.clone();
    false_action_policy_unavailable_reason["action_policy"] = json!("allow");
    let mut action_policy_unavailable_without_basis = action_policy_unavailable.clone();
    action_policy_unavailable_without_basis["basis"] = json!(null);

    let mut identity_expired = denied.clone();
    identity_expired["principal"]["freshness"] = json!("expired");
    identity_expired["reason"] = json!("identity_expired");
    let mut false_identity_expired_reason = identity_expired.clone();
    false_identity_expired_reason["principal"]["freshness"] = json!("current");
    let mut identity_expired_without_deny_result = identity_expired.clone();
    identity_expired_without_deny_result["result"] = json!("insufficient_context");
    let mut identity_expired_with_raw_evaluation_time = identity_expired.clone();
    identity_expired_with_raw_evaluation_time["evaluation_time"] = json!("not-a-time");
    let mut identity_expired_with_basis = identity_expired.clone();
    identity_expired_with_basis["basis"] = decision["basis"].clone();

    let mut identity_context_missing = denied.clone();
    identity_context_missing["principal"]["freshness"] = json!("insufficient_context");
    identity_context_missing["consequential"] = json!(true);
    identity_context_missing["result"] = json!("insufficient_context");
    identity_context_missing["reason"] = json!("identity_context_missing");
    let mut identity_context_missing_before_required_acl = identity_context_missing.clone();
    identity_context_missing_before_required_acl["source_acl_ceiling"] = json!({
        "required": true,
        "result": "insufficient_context"
    });
    let mut expired_identity_before_required_acl =
        identity_context_missing_before_required_acl.clone();
    expired_identity_before_required_acl["principal"]["freshness"] = json!("expired");
    expired_identity_before_required_acl["result"] = json!("deny");
    expired_identity_before_required_acl["reason"] = json!("identity_expired");
    let mut invalid_time_before_required_acl = identity_context_missing_before_required_acl.clone();
    invalid_time_before_required_acl["principal"]["freshness"] = json!("current");
    invalid_time_before_required_acl["evaluation_time"] = json!("");
    invalid_time_before_required_acl["reason"] = json!("evaluation_time_invalid");
    let mut hard_deny_with_missing_identity = identity_context_missing.clone();
    hard_deny_with_missing_identity["hard_deny"] = json!(true);
    let mut false_identity_context_missing_reason = identity_context_missing.clone();
    false_identity_context_missing_reason["principal"]["freshness"] = json!("current");
    let mut consequential_identity_context_with_deny = identity_context_missing.clone();
    consequential_identity_context_with_deny["result"] = json!("deny");
    let mut nonconsequential_identity_context = identity_context_missing.clone();
    nonconsequential_identity_context["consequential"] = json!(false);
    nonconsequential_identity_context["result"] = json!("deny");
    let mut nonconsequential_identity_context_with_insufficient =
        nonconsequential_identity_context.clone();
    nonconsequential_identity_context_with_insufficient["result"] = json!("insufficient_context");
    let mut identity_context_missing_with_raw_evaluation_time = identity_context_missing.clone();
    identity_context_missing_with_raw_evaluation_time["evaluation_time"] = json!("not-a-time");
    let mut identity_context_missing_with_basis = identity_context_missing.clone();
    identity_context_missing_with_basis["basis"] = decision["basis"].clone();

    let mut expired_identity_with_later_reason = visibility_denied.clone();
    expired_identity_with_later_reason["principal"]["freshness"] = json!("expired");
    let mut missing_identity_with_later_reason = hard_deny.clone();
    missing_identity_with_later_reason["principal"]["freshness"] = json!("insufficient_context");

    let mut no_grant_with_basis = denied.clone();
    no_grant_with_basis["basis"] = decision["basis"].clone();
    let mut no_grant_without_deny_result = denied.clone();
    no_grant_without_deny_result["result"] = json!("insufficient_context");

    let mut explicit_deny_without_deny_grant = direct_deny.clone();
    explicit_deny_without_deny_grant["grants"][0]["effect"] = json!("allow");
    let mut explicit_deny_with_allow_basis = direct_deny.clone();
    explicit_deny_with_allow_basis["basis"]["effect"] = json!("allow");
    let mut explicit_deny_without_deny_result = direct_deny.clone();
    explicit_deny_without_deny_result["result"] = json!("insufficient_context");

    let mut hard_deny_without_deny_result = hard_deny.clone();
    hard_deny_without_deny_result["result"] = json!("insufficient_context");
    let mut source_acl_denied_without_deny_result = source_acl_denied.clone();
    source_acl_denied_without_deny_result["result"] = json!("insufficient_context");
    let mut visibility_denied_without_deny_result = visibility_denied.clone();
    visibility_denied_without_deny_result["result"] = json!("insufficient_context");
    let mut action_policy_denied_without_deny_result = action_policy_denied.clone();
    action_policy_denied_without_deny_result["result"] = json!("insufficient_context");

    let mut consequential_source_acl_unavailable_with_deny = insufficient.clone();
    consequential_source_acl_unavailable_with_deny["result"] = json!("deny");
    let mut nonconsequential_source_acl_unavailable = insufficient.clone();
    nonconsequential_source_acl_unavailable["consequential"] = json!(false);
    nonconsequential_source_acl_unavailable["result"] = json!("deny");

    let mut consequential_visibility_unavailable_with_deny = visibility_unavailable.clone();
    consequential_visibility_unavailable_with_deny["result"] = json!("deny");
    let mut nonconsequential_visibility_unavailable = visibility_unavailable.clone();
    nonconsequential_visibility_unavailable["consequential"] = json!(false);
    nonconsequential_visibility_unavailable["result"] = json!("deny");

    let mut consequential_action_unavailable_with_deny = action_policy_unavailable.clone();
    consequential_action_unavailable_with_deny["result"] = json!("deny");
    let mut nonconsequential_action_unavailable = action_policy_unavailable.clone();
    nonconsequential_action_unavailable["consequential"] = json!(false);
    nonconsequential_action_unavailable["result"] = json!("deny");

    let mut nonconsequential_source_acl_unavailable_with_insufficient =
        nonconsequential_source_acl_unavailable.clone();
    nonconsequential_source_acl_unavailable_with_insufficient["result"] =
        json!("insufficient_context");
    let mut nonconsequential_visibility_unavailable_with_insufficient =
        nonconsequential_visibility_unavailable.clone();
    nonconsequential_visibility_unavailable_with_insufficient["result"] =
        json!("insufficient_context");
    let mut nonconsequential_action_unavailable_with_insufficient =
        nonconsequential_action_unavailable.clone();
    nonconsequential_action_unavailable_with_insufficient["result"] = json!("insufficient_context");

    let mut invalid_time = denied.clone();
    invalid_time["principal"]["freshness"] = json!("current");
    invalid_time["evaluation_time"] = json!("");
    invalid_time["consequential"] = json!(true);
    invalid_time["result"] = json!("insufficient_context");
    invalid_time["reason"] = json!("evaluation_time_invalid");
    let mut nonconsequential_invalid_time = invalid_time.clone();
    nonconsequential_invalid_time["consequential"] = json!(false);
    nonconsequential_invalid_time["result"] = json!("deny");
    let mut invalid_time_with_expired_identity = invalid_time.clone();
    invalid_time_with_expired_identity["principal"]["freshness"] = json!("expired");
    let mut invalid_time_with_basis = invalid_time.clone();
    invalid_time_with_basis["basis"] = decision["basis"].clone();
    let mut consequential_invalid_time_with_deny = invalid_time.clone();
    consequential_invalid_time_with_deny["result"] = json!("deny");
    let mut nonconsequential_invalid_time_with_insufficient = nonconsequential_invalid_time.clone();
    nonconsequential_invalid_time_with_insufficient["result"] = json!("insufficient_context");
    let mut invalid_time_with_source_acl_outage = source_acl_outage.clone();
    invalid_time_with_source_acl_outage["evaluation_time"] = json!("");
    invalid_time_with_source_acl_outage["reason"] = json!("evaluation_time_invalid");
    let mut invalid_time_with_source_acl_evidence_only =
        invalid_time_with_source_acl_outage.clone();
    invalid_time_with_source_acl_evidence_only["source_acl_ceiling"]
        .as_object_mut()
        .expect("source ACL ceiling")
        .remove("check_context");
    let mut invalid_time_with_invalidated_source_acl = source_acl_invalidated.clone();
    invalid_time_with_invalidated_source_acl["evaluation_time"] = json!("");
    invalid_time_with_invalidated_source_acl["result"] = json!("insufficient_context");
    invalid_time_with_invalidated_source_acl["reason"] = json!("evaluation_time_invalid");
    let mut invalid_time_with_denied_source_acl = source_acl_denied.clone();
    invalid_time_with_denied_source_acl["evaluation_time"] = json!("");
    invalid_time_with_denied_source_acl["result"] = json!("insufficient_context");
    invalid_time_with_denied_source_acl["reason"] = json!("evaluation_time_invalid");

    for (reason, base) in [
        ("identity_context_missing", &identity_context_missing),
        ("identity_expired", &identity_expired),
        ("evaluation_time_invalid", &invalid_time),
        ("hard_deny", &hard_deny),
        ("source_acl_denied", &source_acl_denied),
        ("source_acl_stale", &source_acl_stale),
        ("source_acl_invalidated", &source_acl_invalidated),
        ("source_acl_unavailable", &insufficient),
        ("explicit_deny", &direct_deny),
    ] {
        let mut unresolved = base.clone();
        if reason == "explicit_deny" {
            unresolved["grants"]
                .as_array_mut()
                .expect("grants array")
                .push(external_group_grant["grants"][0].clone());
        } else {
            unresolved["grants"] = external_group_grant["grants"].clone();
        }
        unresolved["membership_evidence"] = json!("insufficient_context");
        unresolved["membership_unavailability_evidence"] = external_unavailability_evidence.clone();
        assert!(
            schema_accepts("adoc.authorization_decision.v0.schema.json", &unresolved),
            "reason {reason:?} must retain precedence over unresolved membership"
        );
    }
    let mut invalid_time_unresolved_without_group_name = invalid_time.clone();
    invalid_time_unresolved_without_group_name["grants"] = external_group_grant["grants"].clone();
    invalid_time_unresolved_without_group_name["membership_evidence"] =
        json!("insufficient_context");
    invalid_time_unresolved_without_group_name["membership_unavailability_evidence"] =
        external_unavailability_evidence.clone();
    invalid_time_unresolved_without_group_name["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("invalid-time unavailability evidence")
        .remove("group_name");
    assert!(
        schema_accepts(
            "adoc.authorization_decision.v0.schema.json",
            &invalid_time_unresolved_without_group_name
        ),
        "an invalid evaluation time cannot require an evaluation-time group name"
    );
    let mut invalid_time_manual_without_group_name = invalid_time.clone();
    invalid_time_manual_without_group_name["grants"] = external_group_grant["grants"].clone();
    invalid_time_manual_without_group_name["membership_evidence"] = json!("insufficient_context");
    invalid_time_manual_without_group_name["membership_unavailability_evidence"] =
        manual_membership_evidence_unavailable["membership_unavailability_evidence"].clone();
    invalid_time_manual_without_group_name["membership_unavailability_evidence"][0]
        .as_object_mut()
        .expect("invalid-time manual unavailability evidence")
        .remove("group_name");
    assert!(
        schema_accepts(
            "adoc.authorization_decision.v0.schema.json",
            &invalid_time_manual_without_group_name
        ),
        "an invalid evaluation time cannot require a manual evaluation-time group name"
    );

    let mut allow_with_empty_evaluation_time = decision.clone();
    allow_with_empty_evaluation_time["evaluation_time"] = json!("");
    let mut allow_with_blank_evaluation_time = decision.clone();
    allow_with_blank_evaluation_time["evaluation_time"] = json!("   ");
    let mut allow_with_invalid_evaluation_time = decision.clone();
    allow_with_invalid_evaluation_time["evaluation_time"] = json!("not-a-time");

    let mut direct_with_invalid_expiry = direct_human.clone();
    direct_with_invalid_expiry["grants"][0]["expires_at"] = json!("never");
    let mut direct_with_zoneless_expiry = direct_human.clone();
    direct_with_zoneless_expiry["grants"][0]["expires_at"] = json!("2026-08-24T12:00:00");
    let mut direct_with_rfc3339_edge_expiry = direct_human.clone();
    direct_with_rfc3339_edge_expiry["grants"][0]["expires_at"] = json!("2026-12-31t23:59:60z");

    let authorization_schema = schema("adoc.authorization_decision.v0.schema.json");
    let observed_at_description = authorization_schema["$defs"]["membershipObservation"]
        ["properties"]["observed_at"]["description"]
        .as_str()
        .expect("membership observation time is documented");
    assert!(
        observed_at_description.contains("source read time")
            && observed_at_description.contains("sweep start"),
        "membership freshness must use the source read time or a conservative bound"
    );
    let binding_modes = authorization_schema["$defs"]["bindingMode"]["enum"]
        .as_array()
        .expect("shared bindingMode is an enum");
    let source_kinds = authorization_schema["$defs"]["sourceKind"]["enum"]
        .as_array()
        .expect("shared sourceKind is an enum");
    let unavailability_states =
        authorization_schema["$defs"]["membershipUnavailabilityState"]["enum"]
            .as_array()
            .expect("membershipUnavailabilityState is an enum");
    let mut nonhuman_identity_session = decision.clone();
    nonhuman_identity_session["principal"]["type"] = json!("service");
    nonhuman_identity_session["principal"]["identity_session_id"] = json!("identity-session-1");

    let mut oidc_basis_without_session = external_group_grant.clone();
    oidc_basis_without_session["basis"]["group"]["source_kind"] = json!("oidc_group");
    let mut oidc_absence_without_session = no_grant_with_external_absence.clone();
    oidc_absence_without_session["membership_absence_evidence"][0]["source_kind"] =
        json!("oidc_group");
    let mut oidc_absence_with_session = oidc_absence_without_session.clone();
    oidc_absence_with_session["principal"]["identity_session_id"] = json!("identity-session-1");

    for state in unavailability_states {
        let state = state.as_str().expect("unavailability states are strings");
        let mut instance = if state == "lifecycle_unavailable" {
            manual_membership_evidence_unavailable.clone()
        } else {
            membership_evidence_unavailable.clone()
        };
        instance["membership_unavailability_evidence"][0]["state"] = json!(state);
        if state == "oidc_authentication_pending" {
            instance["membership_unavailability_evidence"][0]["source_kind"] = json!("oidc_group");
        } else if state == "epoch_observation_pending" {
            instance["membership_unavailability_evidence"][0]["binding_mode"] = json!("disabled");
        }
        assert!(
            schema_accepts("adoc.authorization_decision.v0.schema.json", &instance),
            "registered membership-unavailability state {state:?} must have a valid fixture"
        );
    }

    let mut oidc_expired_without_session = membership_evidence_unavailable.clone();
    oidc_expired_without_session["membership_unavailability_evidence"][0]["source_kind"] =
        json!("oidc_group");
    oidc_expired_without_session["membership_unavailability_evidence"][0]["state"] =
        json!("observation_expired");
    assert!(!schema_accepts(
        "adoc.authorization_decision.v0.schema.json",
        &oidc_expired_without_session
    ));
    let mut oidc_expired_with_session = oidc_expired_without_session.clone();
    oidc_expired_with_session["membership_unavailability_evidence"][0]["identity_session_id"] =
        json!("identity-session-expired-1");
    assert!(schema_accepts(
        "adoc.authorization_decision.v0.schema.json",
        &oidc_expired_with_session
    ));
    let mut oidc_expired_with_evaluation_session_only = oidc_expired_without_session.clone();
    oidc_expired_with_evaluation_session_only["principal"]["identity_session_id"] =
        json!("identity-session-current-1");
    assert!(!schema_accepts(
        "adoc.authorization_decision.v0.schema.json",
        &oidc_expired_with_evaluation_session_only
    ));
    let mut nonhuman_oidc_pending = membership_evidence_unavailable.clone();
    nonhuman_oidc_pending["principal"]["type"] = json!("service");
    nonhuman_oidc_pending["membership_unavailability_evidence"][0]["source_kind"] =
        json!("oidc_group");
    nonhuman_oidc_pending["membership_unavailability_evidence"][0]["state"] =
        json!("oidc_authentication_pending");
    assert!(!schema_accepts(
        "adoc.authorization_decision.v0.schema.json",
        &nonhuman_oidc_pending
    ));
    let mut oidc_pending_with_entry_session = membership_evidence_unavailable.clone();
    oidc_pending_with_entry_session["membership_unavailability_evidence"][0]["source_kind"] =
        json!("oidc_group");
    oidc_pending_with_entry_session["membership_unavailability_evidence"][0]["state"] =
        json!("oidc_authentication_pending");
    oidc_pending_with_entry_session["membership_unavailability_evidence"][0]["identity_session_id"] =
        json!("identity-session-1");
    assert!(!schema_accepts(
        "adoc.authorization_decision.v0.schema.json",
        &oidc_pending_with_entry_session
    ));
    let mut mixed_oidc_pending = mixed_group_membership_evidence_unavailable.clone();
    mixed_oidc_pending["grants"][0]["group"]["source_kind"] = json!("oidc_group");
    mixed_oidc_pending["principal"]["identity_session_id"] = json!("identity-session-1");
    mixed_oidc_pending["membership_unavailability_evidence"][0]["source_kind"] =
        json!("oidc_group");
    mixed_oidc_pending["membership_unavailability_evidence"][0]["state"] =
        json!("oidc_authentication_pending");
    assert!(
        schema_accepts(
            "adoc.authorization_decision.v0.schema.json",
            &mixed_oidc_pending
        ),
        "pending OIDC input may coexist with a sibling OIDC grant that requires the envelope session"
    );
    for (source_kind, state) in [
        ("github_team", "oidc_authentication_pending"),
        ("oidc_group", "connector_read_failed"),
    ] {
        let mut impossible = membership_evidence_unavailable.clone();
        impossible["membership_unavailability_evidence"][0]["source_kind"] = json!(source_kind);
        impossible["membership_unavailability_evidence"][0]["state"] = json!(state);
        assert!(
            !schema_accepts("adoc.authorization_decision.v0.schema.json", &impossible),
            "source {source_kind:?} cannot recover through state {state:?}"
        );
    }

    for mode in binding_modes {
        let mode = mode.as_str().expect("binding modes are strings");
        for source_kind in source_kinds {
            let source_kind = source_kind.as_str().expect("source kinds are strings");
            let mut instance = external_group_grant.clone();
            instance["grants"][0]["group"]["binding_mode"] = json!(mode);
            instance["grants"][0]["group"]["source_kind"] = json!(source_kind);
            instance["basis"]["group"] = instance["grants"][0]["group"].clone();
            if source_kind == "oidc_group" {
                assert!(
                    !schema_accepts("adoc.authorization_decision.v0.schema.json", &instance),
                    "claim-only OIDC must identify the exact evaluated identity session"
                );
                instance["principal"]["identity_session_id"] = json!("identity-session-1");
                let mut service_instance = instance.clone();
                service_instance["principal"]["type"] = json!("service");
                service_instance["principal"]
                    .as_object_mut()
                    .expect("principal is an object")
                    .remove("identity_session_id");
                assert!(
                    !schema_accepts(
                        "adoc.authorization_decision.v0.schema.json",
                        &service_instance
                    ),
                    "claim-only OIDC must identify an identity session"
                );
            }
            assert!(
                schema_accepts("adoc.authorization_decision.v0.schema.json", &instance),
                "external group mode {mode:?} and source {source_kind:?} must be valid"
            );
        }
    }

    for (name, instance, expected_valid) in [
        ("role assignment allow", decision.clone(), true),
        (
            "required allow without current ACL evidence",
            legacy_allow_without_current_acl,
            false,
        ),
        (
            "definitive source ACL evidence may stand without attempt context",
            definitive_source_acl_evidence_without_context,
            true,
        ),
        ("source ACL connector outage", source_acl_outage, true),
        (
            "insufficient source ACL with current verdict evidence",
            insufficient_with_current_acl,
            false,
        ),
        (
            "required source ACL check without attempt context",
            required_check_without_context,
            false,
        ),
        (
            "available connector cannot claim source ACL unavailability",
            available_connector_with_unavailable_reason,
            false,
        ),
        (
            "not-applicable source ACL with current verdict evidence",
            not_applicable_with_current_acl,
            false,
        ),
        (
            "optional source ACL with attempted-check context",
            optional_check_with_context,
            false,
        ),
        (
            "optional source ACL may retain a historical snapshot",
            optional_acl_with_snapshot,
            true,
        ),
        (
            "retained optional ACL snapshot without source scope",
            optional_snapshot_without_source_scope,
            false,
        ),
        (
            "current source ACL verdict disagrees with ceiling",
            current_acl_result_mismatch,
            false,
        ),
        (
            "allowing ceiling disagrees with denying current source ACL",
            allowing_ceiling_with_denying_current_acl,
            false,
        ),
        ("stale current source ACL evidence", source_acl_stale, true),
        (
            "governing policy change makes current source ACL evidence stale",
            source_acl_policy_superseded,
            true,
        ),
        (
            "stale source ACL without a recorded cause",
            stale_without_recorded_cause,
            false,
        ),
        (
            "policy supersession without evaluation-time check context",
            supersession_without_check_context,
            false,
        ),
        (
            "unchanged-policy expiry has no structural context requirement",
            unchanged_policy_expiry_without_check_context,
            true,
        ),
        (
            "expiry wins while retaining a superseding policy version",
            expired_and_policy_superseded,
            true,
        ),
        (
            "non-stale outcome cannot carry a stale cause",
            false_stale_cause,
            false,
        ),
        (
            "invalidated current source ACL evidence",
            source_acl_invalidated,
            true,
        ),
        ("denying source ACL stale", denying_source_acl_stale, true),
        (
            "denying source ACL invalidated",
            denying_source_acl_invalidated,
            true,
        ),
        ("denying source ACL outage", denying_source_acl_outage, true),
        (
            "known source ACL invalidation outranks connector outage",
            invalidated_source_acl_during_outage,
            true,
        ),
        (
            "connector outage cannot mask known source ACL invalidation",
            unavailable_reason_with_invalidated_source_acl,
            false,
        ),
        (
            "hard deny cannot be recorded as a lower-gate reason",
            hard_deny_recorded_as_lower_gate_reason,
            false,
        ),
        (
            "known stale source ACL outranks connector outage",
            stale_source_acl_during_outage,
            true,
        ),
        (
            "explicit deny cannot outrank source ACL outage",
            explicit_deny_during_source_acl_outage,
            false,
        ),
        (
            "explicit deny cannot outrank invalidated source ACL",
            explicit_deny_with_invalidated_source_acl,
            false,
        ),
        (
            "explicit deny cannot outrank denied source ACL",
            explicit_deny_with_denied_source_acl,
            false,
        ),
        ("direct human grant", direct_human, true),
        (
            "direct human grant with multiline reason",
            direct_human_with_multiline_reason,
            true,
        ),
        (
            "direct human grant with blank reason",
            direct_human_with_blank_reason,
            false,
        ),
        (
            "service direct grant without reason",
            service_direct_without_reason,
            true,
        ),
        (
            "nonhuman principal with identity session",
            nonhuman_identity_session,
            false,
        ),
        (
            "OIDC winning basis without identity session",
            oidc_basis_without_session,
            false,
        ),
        ("time-bounded human direct deny", direct_deny, true),
        (
            "direct deny without expiry",
            direct_deny_without_expiry,
            false,
        ),
        (
            "human direct deny without reason",
            human_direct_deny_without_reason,
            false,
        ),
        ("expiring role assignment", expiring_role, true),
        ("external group role assignment", external_group_grant, true),
        ("manual group role assignment", manual_group_grant, true),
        ("manual group direct grant", manual_group_direct_grant, true),
        (
            "external group without binding",
            external_group_without_binding,
            false,
        ),
        (
            "external group basis without binding mode",
            external_basis_without_mode,
            false,
        ),
        (
            "external group without source kind",
            external_group_without_source_kind,
            false,
        ),
        (
            "external group without membership observation",
            external_group_without_observation,
            false,
        ),
        (
            "external group without binding mode effectivity",
            external_group_without_mode_effectivity,
            false,
        ),
        (
            "absent external membership cannot confer a grant",
            absent_membership_observation,
            false,
        ),
        (
            "nested external membership cannot confer a grant",
            nested_membership_observation,
            false,
        ),
        (
            "external membership observation time is RFC 3339",
            invalid_external_observed_at,
            false,
        ),
        (
            "external membership without activation time",
            external_group_without_effective_at,
            false,
        ),
        (
            "external membership activation time is RFC 3339",
            invalid_external_effective_at,
            false,
        ),
        (
            "external membership without freshness deadline",
            external_group_without_fresh_until,
            false,
        ),
        (
            "external membership freshness deadline is RFC 3339",
            invalid_external_fresh_until,
            false,
        ),
        ("multiline group name", multiline_group_name, false),
        ("suggestion-only group grant", suggestion_group_grant, false),
        ("disabled group basis", disabled_group_basis, false),
        ("unknown external group source", unknown_group_source, false),
        (
            "manual group with binding",
            manual_group_with_binding,
            false,
        ),
        (
            "manual group without membership creation time",
            manual_group_without_membership_created_at,
            false,
        ),
        (
            "manual group without membership id",
            manual_group_without_membership_id,
            false,
        ),
        (
            "manual group with external membership observation",
            manual_group_with_external_observation,
            false,
        ),
        (
            "external group with manual membership creation time",
            external_group_with_manual_membership_time,
            false,
        ),
        (
            "basis cannot invent group provenance",
            basis_group_without_group_grant,
            false,
        ),
        (
            "group grant cannot mark membership not applicable",
            group_with_not_applicable_membership_evidence,
            false,
        ),
        (
            "decision without membership status",
            decision_without_membership_evidence,
            false,
        ),
        ("optional ACL ceiling", no_acl_ceiling, true),
        ("allow with no policy inputs", no_policy_inputs, true),
        ("deny without basis", denied, true),
        (
            "no grant with retained external absence evidence",
            no_grant_with_external_absence,
            true,
        ),
        (
            "no grant with retained manual absence evidence",
            no_grant_with_manual_absence,
            true,
        ),
        (
            "no grant with a resolved group but no absent membership",
            no_grant_with_group_but_no_absence,
            true,
        ),
        (
            "current no grant cannot omit every membership fact",
            no_grant_with_no_membership_facts,
            false,
        ),
        (
            "consequential membership evidence unavailable",
            membership_evidence_unavailable,
            true,
        ),
        (
            "manual membership evidence unavailable",
            manual_membership_evidence_unavailable,
            true,
        ),
        (
            "membership evidence unavailable with another resolved group",
            mixed_group_membership_evidence_unavailable,
            true,
        ),
        (
            "nonconsequential membership evidence unavailable",
            nonconsequential_membership_evidence_unavailable,
            true,
        ),
        (
            "OIDC absence evidence bound to an identity session",
            oidc_absence_with_session,
            true,
        ),
        ("insufficient context without basis", insufficient, true),
        ("hard-deny reason matches input", hard_deny, true),
        (
            "expired identity precedes hard deny",
            hard_deny_with_expired_identity,
            true,
        ),
        (
            "invalid evaluation time precedes hard deny",
            hard_deny_with_invalid_time,
            true,
        ),
        (
            "missing identity context precedes hard deny",
            hard_deny_with_missing_identity,
            true,
        ),
        (
            "source ACL denied reason matches input",
            source_acl_denied,
            true,
        ),
        (
            "visibility denied reason matches input",
            visibility_denied,
            true,
        ),
        (
            "unresolved membership cannot reach the visibility gate",
            unresolved_membership_at_visibility_gate,
            false,
        ),
        (
            "visibility unavailable reason matches input",
            visibility_unavailable,
            true,
        ),
        (
            "action-policy denied reason matches input",
            action_policy_denied,
            true,
        ),
        (
            "action-policy unavailable reason matches input",
            action_policy_unavailable,
            true,
        ),
        (
            "identity-expired reason matches input",
            identity_expired,
            true,
        ),
        (
            "identity-expired reason preserves raw evaluation time",
            identity_expired_with_raw_evaluation_time,
            true,
        ),
        (
            "expired identity may win before a required source ACL attempt",
            expired_identity_before_required_acl,
            true,
        ),
        (
            "consequential identity-context reason matches input",
            identity_context_missing,
            true,
        ),
        (
            "identity gate may win before a required source ACL attempt",
            identity_context_missing_before_required_acl,
            true,
        ),
        (
            "hard deny may win before a required source ACL attempt",
            hard_deny_before_required_acl,
            true,
        ),
        (
            "hard deny may retain definitive source ACL evidence for replay",
            hard_deny_with_source_acl_evidence_only,
            true,
        ),
        (
            "identity-context reason preserves raw evaluation time",
            identity_context_missing_with_raw_evaluation_time,
            true,
        ),
        (
            "nonconsequential identity-context reason matches input",
            nonconsequential_identity_context,
            true,
        ),
        (
            "nonconsequential source ACL unavailable denies",
            nonconsequential_source_acl_unavailable,
            true,
        ),
        (
            "nonconsequential visibility unavailable denies",
            nonconsequential_visibility_unavailable,
            true,
        ),
        (
            "nonconsequential action unavailable denies",
            nonconsequential_action_unavailable,
            true,
        ),
        ("consequential invalid time", invalid_time, true),
        (
            "invalid time outranks source ACL outage",
            invalid_time_with_source_acl_outage,
            true,
        ),
        (
            "invalid evaluation time may retain definitive source ACL evidence for replay",
            invalid_time_with_source_acl_evidence_only,
            true,
        ),
        (
            "invalid time may win before a required source ACL attempt",
            invalid_time_before_required_acl,
            true,
        ),
        (
            "invalid time outranks source ACL invalidation",
            invalid_time_with_invalidated_source_acl,
            true,
        ),
        (
            "invalid time outranks denied source ACL",
            invalid_time_with_denied_source_acl,
            true,
        ),
        (
            "nonconsequential invalid time",
            nonconsequential_invalid_time,
            true,
        ),
        ("missing policy version", missing_policy_version, false),
        ("blank policy version", blank_policy_version, false),
        ("multiline workspace id", multiline_workspace_id, false),
        (
            "carriage-return workspace id",
            carriage_return_workspace_id,
            false,
        ),
        (
            "line-separator workspace id",
            line_separator_workspace_id,
            false,
        ),
        ("unknown result", unknown_result, false),
        (
            "allow during source ACL outage",
            allow_during_source_acl_outage,
            false,
        ),
        (
            "allow with invalidated source ACL evidence",
            allow_with_invalidated_source_acl,
            false,
        ),
        (
            "stale reason without current ACL evidence",
            stale_reason_without_current_acl,
            false,
        ),
        (
            "invalidated reason without invalidation marker",
            invalidated_reason_without_marker,
            false,
        ),
        (
            "current ACL evidence with invalid observed time",
            current_acl_with_invalid_observed_at,
            false,
        ),
        ("direct grant without expiry", direct_without_expiry, false),
        (
            "role assignment with exceptional reason",
            role_with_exception,
            false,
        ),
        ("direct grant with role", direct_with_role, false),
        (
            "human direct grant without reason",
            human_direct_without_reason,
            false,
        ),
        (
            "direct-grant basis with role",
            direct_basis_with_role,
            false,
        ),
        (
            "role-assignment basis without role",
            role_basis_without_role,
            false,
        ),
        ("unregistered permission", unknown_permission, false),
        ("optional ACL marked allow", optional_acl_allow, false),
        ("ACL allow without snapshot", acl_without_snapshot, false),
        ("hard deny recorded as allow", hard_deny_allow, false),
        (
            "stale principal recorded as allow",
            stale_principal_allow,
            false,
        ),
        ("denied ACL recorded as allow", denied_acl_allow, false),
        (
            "denied visibility recorded as allow",
            denied_visibility_allow,
            false,
        ),
        (
            "uncertain action policy recorded as allow",
            uncertain_action_allow,
            false,
        ),
        ("allow without basis", allow_without_basis, false),
        ("allowed reason on deny", allowed_reason_on_deny, false),
        ("unknown scope member", unknown_scope_member, false),
        ("allow without grants", allow_without_grants, false),
        ("deny basis on allow", deny_basis_on_allow, false),
        (
            "deny-only grants on allow",
            deny_only_grants_on_allow,
            false,
        ),
        ("false hard-deny reason", false_hard_deny_reason, false),
        (
            "hard deny cannot be recorded as no grant",
            hard_deny_recorded_as_no_grant,
            false,
        ),
        ("hard-deny reason with basis", hard_deny_with_basis, false),
        (
            "false source ACL denied reason",
            false_source_acl_denied_reason,
            false,
        ),
        (
            "source ACL denied reason with basis",
            source_acl_denied_with_basis,
            false,
        ),
        (
            "source ACL denied without current ACL evidence",
            source_acl_denied_without_current_acl,
            false,
        ),
        (
            "source ACL denied during outage",
            source_acl_denied_during_outage,
            false,
        ),
        (
            "source ACL denied with invalidated ACL",
            source_acl_denied_with_invalidated_acl,
            false,
        ),
        (
            "false source ACL unavailable reason",
            false_source_acl_unavailable_reason,
            false,
        ),
        (
            "source ACL unavailable reason with basis",
            source_acl_unavailable_with_basis,
            false,
        ),
        (
            "missing source ACL verdict cannot be recorded as no grant",
            missing_source_acl_verdict_recorded_as_no_grant,
            false,
        ),
        (
            "false visibility denied reason",
            false_visibility_denied_reason,
            false,
        ),
        (
            "visibility denied reason without basis",
            visibility_denied_without_basis,
            false,
        ),
        (
            "false visibility unavailable reason",
            false_visibility_unavailable_reason,
            false,
        ),
        (
            "visibility unavailable reason without basis",
            visibility_unavailable_without_basis,
            false,
        ),
        (
            "false action-policy denied reason",
            false_action_policy_denied_reason,
            false,
        ),
        (
            "action-policy denied reason without basis",
            action_policy_denied_without_basis,
            false,
        ),
        (
            "false action-policy unavailable reason",
            false_action_policy_unavailable_reason,
            false,
        ),
        (
            "action-policy unavailable reason without basis",
            action_policy_unavailable_without_basis,
            false,
        ),
        (
            "false identity-expired reason",
            false_identity_expired_reason,
            false,
        ),
        (
            "identity-expired reason without deny result",
            identity_expired_without_deny_result,
            false,
        ),
        (
            "identity-expired reason with basis",
            identity_expired_with_basis,
            false,
        ),
        (
            "false identity-context reason",
            false_identity_context_missing_reason,
            false,
        ),
        (
            "identity-context reason with basis",
            identity_context_missing_with_basis,
            false,
        ),
        (
            "consequential identity-context reason with deny result",
            consequential_identity_context_with_deny,
            false,
        ),
        (
            "nonconsequential identity-context reason with insufficient result",
            nonconsequential_identity_context_with_insufficient,
            false,
        ),
        (
            "expired identity with later-precedence reason",
            expired_identity_with_later_reason,
            false,
        ),
        (
            "missing identity with later-precedence reason",
            missing_identity_with_later_reason,
            false,
        ),
        ("no-grant reason with basis", no_grant_with_basis, false),
        (
            "no-grant reason without deny result",
            no_grant_without_deny_result,
            false,
        ),
        (
            "membership-evidence-unavailable reason with basis",
            membership_evidence_unavailable_with_basis,
            false,
        ),
        (
            "membership-evidence-unavailable reason without status input",
            membership_evidence_unavailable_without_input,
            false,
        ),
        (
            "membership-evidence-unavailable reason with current status",
            membership_evidence_unavailable_with_current_input,
            false,
        ),
        (
            "unavailable membership without retained provenance",
            membership_evidence_unavailable_without_provenance,
            false,
        ),
        (
            "unavailable membership with empty retained provenance",
            membership_evidence_unavailable_with_empty_provenance,
            false,
        ),
        (
            "resolved membership cannot carry unavailability provenance",
            resolved_membership_with_unavailability,
            false,
        ),
        (
            "unavailable membership without a retained state record",
            membership_evidence_unavailable_without_state_record,
            false,
        ),
        (
            "unavailable membership without a retained group name",
            membership_evidence_unavailable_without_group_name,
            false,
        ),
        (
            "external unavailable membership without a binding mode",
            membership_evidence_unavailable_without_binding_mode,
            false,
        ),
        (
            "external unavailable membership without a binding epoch",
            membership_evidence_unavailable_without_binding_epoch,
            false,
        ),
        (
            "manual unavailable membership without a retained group name",
            manual_unavailability_without_group_name,
            false,
        ),
        (
            "disabled binding cannot be an unavailable granting input",
            unavailable_membership_from_disabled_binding,
            false,
        ),
        (
            "unavailable membership with an unknown state",
            membership_evidence_unavailable_with_unknown_state,
            false,
        ),
        (
            "no-grant reason with unavailable membership evidence",
            no_grant_with_unavailable_membership_evidence,
            false,
        ),
        (
            "current no-grant without absence evidence",
            no_grant_without_absence_evidence,
            false,
        ),
        (
            "external absence evidence cannot record membership present",
            no_grant_with_positive_absence_observation,
            false,
        ),
        (
            "not-applicable no-grant cannot carry absence evidence",
            no_grant_not_applicable_with_absence,
            false,
        ),
        (
            "OIDC absence evidence without an identity session",
            oidc_absence_without_session,
            false,
        ),
        (
            "consequential membership evidence unavailable with deny result",
            consequential_membership_evidence_unavailable_with_deny,
            false,
        ),
        (
            "nonconsequential membership evidence unavailable with insufficient result",
            nonconsequential_membership_evidence_unavailable_with_insufficient,
            false,
        ),
        (
            "explicit-deny reason without deny grant",
            explicit_deny_without_deny_grant,
            false,
        ),
        (
            "explicit-deny reason with allow basis",
            explicit_deny_with_allow_basis,
            false,
        ),
        (
            "explicit-deny reason without deny result",
            explicit_deny_without_deny_result,
            false,
        ),
        (
            "hard-deny reason without deny result",
            hard_deny_without_deny_result,
            false,
        ),
        (
            "source ACL denied reason without deny result",
            source_acl_denied_without_deny_result,
            false,
        ),
        (
            "visibility denied reason without deny result",
            visibility_denied_without_deny_result,
            false,
        ),
        (
            "action-policy denied reason without deny result",
            action_policy_denied_without_deny_result,
            false,
        ),
        (
            "consequential source ACL unavailable with deny result",
            consequential_source_acl_unavailable_with_deny,
            false,
        ),
        (
            "consequential visibility unavailable with deny result",
            consequential_visibility_unavailable_with_deny,
            false,
        ),
        (
            "consequential action unavailable with deny result",
            consequential_action_unavailable_with_deny,
            false,
        ),
        (
            "nonconsequential source ACL unavailable with insufficient result",
            nonconsequential_source_acl_unavailable_with_insufficient,
            false,
        ),
        (
            "nonconsequential visibility unavailable with insufficient result",
            nonconsequential_visibility_unavailable_with_insufficient,
            false,
        ),
        (
            "nonconsequential action unavailable with insufficient result",
            nonconsequential_action_unavailable_with_insufficient,
            false,
        ),
        (
            "invalid time with expired identity",
            invalid_time_with_expired_identity,
            false,
        ),
        ("invalid time with basis", invalid_time_with_basis, false),
        (
            "consequential invalid time with deny result",
            consequential_invalid_time_with_deny,
            false,
        ),
        (
            "nonconsequential invalid time with insufficient result",
            nonconsequential_invalid_time_with_insufficient,
            false,
        ),
        (
            "allow with empty evaluation time",
            allow_with_empty_evaluation_time,
            false,
        ),
        (
            "allow with blank evaluation time",
            allow_with_blank_evaluation_time,
            false,
        ),
        (
            "allow with invalid evaluation time",
            allow_with_invalid_evaluation_time,
            false,
        ),
        (
            "direct grant with invalid expiry",
            direct_with_invalid_expiry,
            false,
        ),
        (
            "direct grant with zoneless expiry",
            direct_with_zoneless_expiry,
            false,
        ),
        (
            "direct grant with RFC 3339 lowercase leap-second expiry",
            direct_with_rfc3339_edge_expiry,
            true,
        ),
    ] {
        assert_eq!(
            schema_accepts("adoc.authorization_decision.v0.schema.json", &instance),
            expected_valid,
            "authorization decision schema case failed: {name}\ninstance: {instance}"
        );
    }

    for field in [
        "role",
        "current_acl_id",
        "result",
        "snapshot_id",
        "workspace_id",
        "connector_id",
        "source_container_id",
        "principal_id",
        "external_identity_link_id",
        "source",
        "acl_policy_version",
        "observed_at",
        "expires_at",
        "connector_available",
    ] {
        let mut missing = decision.clone();
        missing["source_acl_ceiling"]["current_authorization"]
            .as_object_mut()
            .expect("current ACL object")
            .remove(field);
        assert!(
            !schema_accepts("adoc.authorization_decision.v0.schema.json", &missing),
            "current ACL evidence without {field} must be rejected"
        );
    }

    for field in ["role", "external_identity_link_id", "acl_policy_version"] {
        let mut missing = insufficient_check_context.clone();
        missing["source_acl_ceiling"]["check_context"]
            .as_object_mut()
            .expect("ACL check context")
            .remove(field);
        assert!(
            !schema_accepts("adoc.authorization_decision.v0.schema.json", &missing),
            "ACL attempt context without {field} must be rejected"
        );
    }
}

#[test]
fn connector_acl_policy_requires_every_activation_safety_declaration() {
    let policy = json!({
        "schema_version": "adoc.connector_acl_policy.v0",
        "connector_kind": "github",
        "policy_version": "github-acl-v1",
        "acquisition": "provider_events_and_api",
        "freshness_window_seconds": 300,
        "refresh_mechanism": "webhook_and_poll",
        "revocation_propagation": "immediate_on_observation",
        "connector_unavailable": "fail_closed",
        "invalidation": {
            "acl_cache": true,
            "active_access_sessions": true
        }
    });

    assert_eq!(
        policy["freshness_window_seconds"],
        json!(300),
        "canonical ACL policy must produce the canonical evidence expiry"
    );
    assert_eq!(
        CANONICAL_SOURCE_ACL_EXPIRES_AT, "2026-08-23T12:04:00Z",
        "canonical ACL expiry must retain the policy's 300-second window"
    );
    assert!(schema_accepts(
        "adoc.connector_acl_policy.v0.schema.json",
        &policy
    ));

    for field in [
        "acquisition",
        "freshness_window_seconds",
        "refresh_mechanism",
        "revocation_propagation",
        "connector_unavailable",
        "invalidation",
    ] {
        let mut missing = policy.clone();
        missing
            .as_object_mut()
            .expect("policy object")
            .remove(field);
        assert!(
            !schema_accepts("adoc.connector_acl_policy.v0.schema.json", &missing),
            "connector activation policy without {field} must be rejected"
        );
    }

    let mut permissive_outage = policy.clone();
    permissive_outage["connector_unavailable"] = json!("use_stale");
    let mut no_session_invalidation = policy.clone();
    no_session_invalidation["invalidation"]["active_access_sessions"] = json!(false);
    let mut api_with_webhook_refresh = policy.clone();
    api_with_webhook_refresh["acquisition"] = json!("provider_api");
    let mut events_without_webhook_refresh = policy.clone();
    events_without_webhook_refresh["refresh_mechanism"] = json!("poll");
    let mut blank_policy_version = policy.clone();
    blank_policy_version["policy_version"] = json!(" github-acl-v1");
    let mut excessive_freshness = policy.clone();
    excessive_freshness["freshness_window_seconds"] = json!(604801);
    let mut zero_freshness = policy;
    zero_freshness["freshness_window_seconds"] = json!(0);

    assert!(
        schema_accepts(
            "adoc.connector_acl_policy.v0.schema.json",
            &events_without_webhook_refresh
        ),
        "provider events may invalidate an ACL that is refreshed by polling"
    );

    for (name, invalid) in [
        ("permissive outage", permissive_outage),
        ("missing session invalidation", no_session_invalidation),
        (
            "API-only acquisition with webhook refresh",
            api_with_webhook_refresh,
        ),
        ("whitespace-padded policy version", blank_policy_version),
        ("excessive freshness", excessive_freshness),
        ("zero freshness", zero_freshness),
    ] {
        assert!(
            !schema_accepts("adoc.connector_acl_policy.v0.schema.json", &invalid),
            "connector ACL policy schema accepted {name}: {invalid}"
        );
    }
}

#[test]
fn source_acl_snapshot_is_historical_provenance_not_current_authority() {
    let mut snapshot = canonical_source_acl_join();
    snapshot
        .as_object_mut()
        .expect("source ACL snapshot fixture is an object")
        .extend(
            json!({
                "schema_version": "adoc.source_acl_snapshot.v0",
                "acl_payload_digest": format!("sha256:{}", "a".repeat(64)),
                "usage": "historical_provenance"
            })
            .as_object()
            .expect("source ACL snapshot fields are an object")
            .clone(),
        );

    assert!(schema_accepts(
        "adoc.source_acl_snapshot.v0.schema.json",
        &snapshot
    ));
    let mut current_authority = snapshot.clone();
    current_authority["usage"] = json!("current_authorization");
    let mut expiring_snapshot = snapshot.clone();
    expiring_snapshot["expires_at"] = json!("2026-08-24T12:05:00Z");
    let mut missing_policy_version = snapshot.clone();
    missing_policy_version
        .as_object_mut()
        .expect("snapshot object")
        .remove("acl_policy_version");
    let mut missing_workspace_id = snapshot.clone();
    missing_workspace_id
        .as_object_mut()
        .expect("snapshot object")
        .remove("workspace_id");
    let mut missing_source_container_id = snapshot.clone();
    missing_source_container_id
        .as_object_mut()
        .expect("snapshot object")
        .remove("source_container_id");
    let mut invalid_observed_at = snapshot.clone();
    invalid_observed_at["observed_at"] = json!("not-a-time");
    let mut whitespace_padded_connector_id = snapshot.clone();
    whitespace_padded_connector_id["connector_id"] = json!(" github-connector-1");
    let mut missing_usage = snapshot;
    missing_usage
        .as_object_mut()
        .expect("snapshot object")
        .remove("usage");

    for (name, invalid) in [
        ("current authority usage", current_authority),
        ("expiry field", expiring_snapshot),
        ("missing ACL policy version", missing_policy_version),
        ("missing workspace id", missing_workspace_id),
        ("missing source container id", missing_source_container_id),
        ("invalid observed timestamp", invalid_observed_at),
        (
            "whitespace-padded connector id",
            whitespace_padded_connector_id,
        ),
        ("missing usage", missing_usage),
    ] {
        assert!(
            !schema_accepts("adoc.source_acl_snapshot.v0.schema.json", &invalid),
            "source ACL snapshot schema accepted {name}: {invalid}"
        );
    }
}

#[test]
fn e6_5_writeback_contract_requires_an_exact_revision_precondition() {
    let name = "agentdoc.cloud.writeback_record.v0.schema.json";
    let valid = json!({
      "schema_version": "agentdoc.cloud.writeback_record.v0",
      "writeback_id": "60000000-0000-0000-0000-000000000001",
      "origin": {
        "subject": { "canonical": { "workspace_id": "10000000-0000-0000-0000-000000000001", "canonical_id": "30000000-0000-0000-0000-000000000001" }, "version_id": "40000000-0000-0000-0000-000000000001" },
        "event_ordinal": "0",
        "event_digest": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
      },
      "target": {
        "connector_id": "20000000-0000-0000-0000-000000000001",
        "source_binding_id": "binding-1",
        "source_binding_digest": "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "revision_precondition": "native-revision-42"
      },
      "idempotency_key": "projection-42",
      "payload_digest": "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"
    });
    assert_valid(name, &valid);
    let mut later = valid.clone();
    later["origin"]["event_ordinal"] = json!("9007199254740993");
    assert_valid(name, &later);
    let mut global_sequence = valid.clone();
    global_sequence["origin"]
        .as_object_mut()
        .unwrap()
        .remove("event_ordinal");
    global_sequence["origin"]["event_seq"] = json!("42");
    assert!(
        !schema_accepts(name, &global_sequence),
        "store-global sequence cannot stand in for a workspace ordinal"
    );

    let mut missing = valid.clone();
    missing["target"]
        .as_object_mut()
        .unwrap()
        .remove("revision_precondition");
    assert!(
        !schema_accepts(name, &missing),
        "writeback without a target revision must fail closed"
    );
    for precondition in [
        json!(null),
        json!(""),
        json!(" "),
        json!(" revision "),
        json!("revision\n42"),
        json!("revision\u{2028}42"),
        json!("revision\u{2029}42"),
        json!(42),
    ] {
        let mut invalid = valid.clone();
        invalid["target"]["revision_precondition"] = precondition;
        assert!(
            !schema_accepts(name, &invalid),
            "invalid target revision was accepted: {invalid}"
        );
    }

    // Every lineage reference is mandatory; no malformed sibling can be ignored.
    for pointer in [
        "/schema_version",
        "/writeback_id",
        "/origin",
        "/origin/subject",
        "/origin/subject/canonical",
        "/origin/subject/canonical/workspace_id",
        "/origin/subject/canonical/canonical_id",
        "/origin/subject/version_id",
        "/origin/event_ordinal",
        "/origin/event_digest",
        "/target",
        "/target/connector_id",
        "/target/source_binding_id",
        "/target/source_binding_digest",
        "/target/revision_precondition",
        "/idempotency_key",
        "/payload_digest",
    ] {
        let (parent, field) = pointer.rsplit_once('/').unwrap();
        let mut missing = valid.clone();
        missing
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(
            !schema_accepts(name, &missing),
            "missing {pointer} was accepted"
        );
        let mut null = valid.clone();
        *null.pointer_mut(pointer).unwrap() = json!(null);
        assert!(!schema_accepts(name, &null), "null {pointer} was accepted");
    }
    for pointer in [
        "",
        "/origin",
        "/origin/subject",
        "/origin/subject/canonical",
        "/target",
    ] {
        let mut extended = valid.clone();
        extended
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!(true));
        assert!(
            !schema_accepts(name, &extended),
            "unknown field in {pointer} was accepted"
        );
    }
    for pointer in [
        "/writeback_id",
        "/origin/subject/canonical/workspace_id",
        "/origin/subject/canonical/canonical_id",
        "/origin/subject/version_id",
        "/target/connector_id",
    ] {
        for id in [
            "not-a-uuid",
            "ABCDEFAB-0000-0000-0000-000000000001",
            "10000000-0000-0000-0000-000000000001\n",
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = json!(id);
            assert!(
                !schema_accepts(name, &invalid),
                "noncanonical UUID at {pointer} was accepted"
            );
        }
    }
    for pointer in [
        "/origin/event_digest",
        "/target/source_binding_digest",
        "/payload_digest",
    ] {
        for digest in [
            format!("sha256:{}", "a".repeat(63)),
            format!("sha256:{}", "A".repeat(64)),
            format!("sha256:{}\n", "a".repeat(64)),
            format!("sha512:{}", "a".repeat(64)),
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = json!(digest);
            assert!(
                !schema_accepts(name, &invalid),
                "malformed digest at {pointer} was accepted"
            );
        }
    }
    for ordinal in [
        json!(0),
        json!(42),
        json!("-1"),
        json!("01"),
        json!("+1"),
        json!("1\n"),
    ] {
        let mut invalid = valid.clone();
        invalid["origin"]["event_ordinal"] = ordinal;
        assert!(
            !schema_accepts(name, &invalid),
            "noncanonical event sequence was accepted"
        );
    }
    for pointer in ["/target/source_binding_id", "/idempotency_key"] {
        for value in [
            "",
            " ",
            " padded ",
            "line\nbreak",
            "line\u{2028}break",
            "line\u{2029}break",
        ] {
            let mut invalid = valid.clone();
            *invalid.pointer_mut(pointer).unwrap() = json!(value);
            assert!(
                !schema_accepts(name, &invalid),
                "unbound text at {pointer} was accepted"
            );
        }
    }
    for pointer in ["/target/source_binding_id", "/target/revision_precondition"] {
        for control in [
            '\0', '\u{b}', '\u{c}', '\u{1b}', '\u{7f}', '\u{85}', '\u{9f}',
        ] {
            for value in [control.to_string(), format!("before{control}after")] {
                let mut invalid = valid.clone();
                *invalid.pointer_mut(pointer).unwrap() = json!(value);
                assert!(
                    !schema_accepts(name, &invalid),
                    "control character at {pointer} was accepted"
                );
            }
        }
    }
    let mut longest_key = valid.clone();
    longest_key["idempotency_key"] = json!("a".repeat(200));
    assert_valid(name, &longest_key);
    for key in ["a".repeat(201), "two words".into(), "clé".into()] {
        let mut invalid = valid.clone();
        invalid["idempotency_key"] = json!(key);
        assert!(
            !schema_accepts(name, &invalid),
            "key outside the existing Cloud transport contract was accepted"
        );
    }
    let mut future = valid;
    future["schema_version"] = json!("agentdoc.cloud.writeback_record.v99");
    assert!(
        !schema_accepts(name, &future),
        "unknown operation version was accepted"
    );
}

#[test]
fn migration_request_and_actual_receipt_match_portable_schemas() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\n",
    );
    write(
        &root.join("docs/index.adoc"),
        "# Migration @doc(test.page)\n\n::claim test.claim\nstatus: draft\n--\nBody.\n::\n",
    );
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.email", "test@example.test"]);
    run_git(root, &["config", "user.name", "Test"]);
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-qm", "source"]);
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let request = json!({"schema_version":"adoc.migration_request.v0", "request_id":"r", "workspace_id":"w", "source_id":"s", "repository_identity":"repo", "revision":{"system":"git", "value":String::from_utf8(output.stdout).unwrap().trim()}, "evaluation_date":"2026-09-08"});
    let schema_name = "adoc.migration_request.v0.schema.json";
    assert_valid(schema_name, &request);
    let bytes = serde_json::to_vec(&request).unwrap();
    let receipt = adoc_local::prepare_migration(
        root,
        &bytes,
        "0.4.0".into(),
        format!("sha256:{}", "a".repeat(64)),
    )
    .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&receipt.to_canonical_json().unwrap()).unwrap();
    assert_valid("adoc.migration_receipt.v0.schema.json", &value);
    let job = json!({"schema_version":"agentdoc.cloud.migration_import_job.v0","connector_id":"connector","observed_at":"2026-09-08T12:00:00Z","source_acl_scope":{"snapshot_id":"acl","source_container_id":"s","source":{"kind":"repository","id":"repo"}},"sources":[{"path":"docs/index.adoc","source_record_id":"record","source_binding_id":"binding"}]});
    let job_schema = "agentdoc.cloud.migration_import_job.v0.schema.json";
    assert_valid(job_schema, &job);
    let import = adoc_core::import_migration_from_git(
        root,
        &bytes,
        &serde_json::to_vec(&job).unwrap(),
        "0.4.0".into(),
        format!("sha256:{}", "a".repeat(64)),
    )
    .unwrap();
    let bundle: serde_json::Value =
        serde_json::from_str(&import.to_canonical_json().unwrap()).unwrap();
    assert_valid("adoc.migration_import.v0.schema.json", &bundle);
    for (field, nested_schema) in [
        ("source_record_bytes", "adoc.source_record.v1.schema.json"),
        ("source_binding_bytes", "adoc.source_binding.v0.schema.json"),
        (
            "source_invocation_bytes",
            "agentdoc.cloud.migration_validation_invocation.v0.schema.json",
        ),
        (
            "validation_receipt_bytes",
            "adoc.validation_receipt.v1.schema.json",
        ),
    ] {
        let nested: serde_json::Value =
            serde_json::from_str(bundle["sources"][0][field].as_str().unwrap()).unwrap();
        assert_valid(nested_schema, &nested);
        let mut wrong = nested;
        wrong["foreign"] = json!(true);
        assert!(!schema_accepts(nested_schema, &wrong));
    }
    for (pointer, bad) in [
        ("/sources", json!([])),
        ("/sources/0/path", json!("../escape")),
        ("/observed_at", json!("2026-09-08T12:00:00.1Z")),
        ("/source_acl_scope/source/kind", json!("project")),
    ] {
        let mut invalid = job.clone();
        *invalid.pointer_mut(pointer).unwrap() = bad;
        assert!(!schema_accepts(job_schema, &invalid), "{pointer}");
        assert!(
            adoc_core::import_migration_from_git(
                root,
                &bytes,
                &serde_json::to_vec(&invalid).unwrap(),
                "0.4.0".into(),
                format!("sha256:{}", "a".repeat(64))
            )
            .is_err()
        );
    }
    let mut invalid_bundle = bundle;
    invalid_bundle["foreign"] = json!(true);
    assert!(!schema_accepts(
        "adoc.migration_import.v0.schema.json",
        &invalid_bundle
    ));
    let result = json!({"schema_version":"agentdoc.cloud.migration_import_result.v0","request_id":"r","candidates":[{"object_id":"test.claim","canonical_id":"canonical","version_id":"version","source_record_id":"record","source_binding_id":"binding"}]});
    assert_valid(
        "agentdoc.cloud.migration_import_result.v0.schema.json",
        &result,
    );

    for (field, bad) in [
        ("foreign", json!(true)),
        ("revision", json!({"system":"git", "value":"HEAD"})),
        ("revision", json!({"system":"git", "value":"0".repeat(40)})),
        ("workspace_id", json!(" ")),
    ] {
        let mut invalid = request.clone();
        invalid[field] = bad;
        assert!(!schema_accepts(schema_name, &invalid));
        assert!(
            adoc_core::MigrationRequest::parse(&serde_json::to_vec(&invalid).unwrap()).is_err()
        );
    }
    let mut invalid = value;
    invalid["phase"] = json!("promote");
    assert!(!schema_accepts(
        "adoc.migration_receipt.v0.schema.json",
        &invalid
    ));
}

#[test]
fn migration_qualification_actual_outputs_match_closed_schemas() {
    let workspace = tempfile::tempdir().unwrap();
    let root = workspace.path();
    write(
        &root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\n",
    );
    write(
        &root.join("docs/index.adoc"),
        "# Migration @doc(test.page)\n\n::claim test.claim\nstatus: draft\n--\nBody.\n::\n",
    );
    run_git(root, &["init", "-q"]);
    run_git(root, &["config", "user.email", "test@example.test"]);
    run_git(root, &["config", "user.name", "Test"]);
    run_git(root, &["add", "."]);
    run_git(root, &["commit", "-qm", "source"]);
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let mut request = json!({"schema_version":"adoc.migration_request.v0", "request_id":"r", "workspace_id":"w", "source_id":"s", "repository_identity":"repo", "revision":{"system":"git", "value":String::from_utf8(output.stdout).unwrap().trim()}, "evaluation_date":"2026-09-08"});
    let schema_name = "adoc.migration_request.v0.schema.json";
    assert_valid(schema_name, &request);
    let bytes = serde_json::to_vec(&request).unwrap();
    let receipt = adoc_local::prepare_migration(
        root,
        &bytes,
        "0.4.0".into(),
        format!("sha256:{}", "a".repeat(64)),
    )
    .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&receipt.to_canonical_json().unwrap()).unwrap();
    assert_valid("adoc.migration_receipt.v0.schema.json", &value);
    let job = json!({"schema_version":"agentdoc.cloud.migration_import_job.v0","connector_id":"connector","observed_at":"2026-09-08T12:00:00Z","source_acl_scope":{"snapshot_id":"acl","source_container_id":"s","source":{"kind":"repository","id":"repo"}},"sources":[{"path":"docs/index.adoc","source_record_id":"record","source_binding_id":"binding"}]});
    let job_schema = "agentdoc.cloud.migration_import_job.v0.schema.json";
    assert_valid(job_schema, &job);

    for invalid_utf8 in [false, true] {
        if invalid_utf8 {
            fs::write(root.join("docs/index.adoc"), [0xff, 0xfe]).unwrap();
            run_git(root, &["add", "."]);
            run_git(root, &["commit", "-qm", "binary"]);
            let out = Command::new("git")
                .arg("-C")
                .arg(root)
                .args(["rev-parse", "HEAD"])
                .output()
                .unwrap();
            request["revision"]["value"] = json!(String::from_utf8(out.stdout).unwrap().trim());
        }
        let result = adoc_core::qualify_migration_from_git(
            root,
            &serde_json::to_vec(&request).unwrap(),
            &serde_json::to_vec(&job).unwrap(),
            "1",
            "0.4.0".into(),
            format!("sha256:{}", "a".repeat(64)),
        )
        .unwrap();
        let output: serde_json::Value =
            serde_json::from_str(&result.to_canonical_json().unwrap()).unwrap();
        let schema = "adoc.migration_qualification.v0.schema.json";
        assert_valid(schema, &output);
        for (field, value) in [
            ("qualification_policy_version", json!("2")),
            ("foreign", json!(true)),
            ("outcome", json!("active")),
        ] {
            let mut bad = output.clone();
            bad[field] = value;
            assert!(!schema_accepts(schema, &bad));
        }
        if invalid_utf8 {
            for (field, schema) in [
                ("source_record_bytes", "adoc.source_record.v1.schema.json"),
                ("source_binding_bytes", "adoc.source_binding.v0.schema.json"),
            ] {
                assert_valid(
                    schema,
                    &serde_json::from_str::<serde_json::Value>(
                        output["sources"][0][field].as_str().unwrap(),
                    )
                    .unwrap(),
                );
            }
            let mut bad = output.clone();
            bad["sources"][0]["source_bytes_base64"] = json!("//4");
            assert!(!schema_accepts(schema, &bad));
            let mut mixed = output.clone();
            mixed["candidate_bundle_bytes"] = json!("{}");
            assert!(!schema_accepts(schema, &mixed));
        } else {
            let receipt: serde_json::Value =
                serde_json::from_str(output["qualification_receipt_bytes"].as_str().unwrap())
                    .unwrap();
            let schema = "adoc.migration_qualification_receipt.v0.schema.json";
            assert_valid(schema, &receipt);
            for pointer in [
                "/lifecycle_mapping_version",
                "/qualification_policy_version",
                "/objects/0/reasons/0/code",
            ] {
                let mut bad = receipt.clone();
                *bad.pointer_mut(pointer).unwrap() = json!("unknown");
                assert!(!schema_accepts(schema, &bad));
            }
        }
    }
    let native = json!({"schema_version":"agentdoc.cloud.migration_qualification_result.v0", "request_id":"r", "qualification_id":"00000000-0000-4000-8000-000000000001", "outcome":"flagged_source_evidence", "candidates":[], "sources":[{"path":"docs/index.adoc","source_record_id":"00000000-0000-4000-8000-000000000002","source_binding_id":"00000000-0000-4000-8000-000000000003"}]});
    assert_valid(
        "agentdoc.cloud.migration_qualification_result.v0.schema.json",
        &native,
    );
}

#[test]
fn migration_initialization_cloud_records_preserve_closed_exact_evidence() {
    let id = "00000000-0000-4000-8000-000000000001";
    let digest = format!("sha256:{}", "a".repeat(64));
    let semantic_hash = format!("sha256:{}", "b".repeat(64));
    let meaning = "accept_exact_revision_and_qualifying_history_as_initialization_evidence";
    let request = json!({
        "schema_version":"agentdoc.cloud.migration_initialization_request.v0",
        "request_id":id,"qualification_id":id,"qualification_envelope_digest":digest,
        "qualification_receipt_digest":digest,"meaning":meaning,"rationale":"Accept this exact revision and retained history."
    });
    let attestation = json!({
        "schema_version":"agentdoc.cloud.migration_initialization_attestation.v0",
        "request_id":id,"request_digest":digest,"workspace_id":id,"qualification_id":id,
        "migration_request_digest":digest,"qualification_envelope_digest":digest,
        "qualification_receipt_digest":digest,"qualification_policy_version":"1",
        "lifecycle_mapping_version":"1","initialization_policy_version":"migration-initialization-v1",
        "principal_id":id,"identity_session_id":id,"auth_session_id":id,
        "external_identity_link_id":id,"authorization_decision_id":id,
        "authorization_decision_digest":digest,"authorization_policy_version":"existing-human-policy-v1",
        "attested_at":"2026-09-08T14:23:01.123456+00:00","meaning":meaning,"rationale":request["rationale"]
    });
    let promotion = json!({
        "object_id":"test.adopted","content_hash":semantic_hash,"canonical_id":id,"version_id":id,
        "content_digest":digest,"source_record_id":id,"source_binding_id":id,
        "promotion_seq":"9007199254740993","governance_event_seq":"9007199254740994",
        "governance_event_digest":digest,"effectivity_event_seq":"9007199254740995","effectivity_event_digest":digest
    });
    let result = json!({
        "schema_version":"agentdoc.cloud.migration_initialization_result.v0","request_id":id,
        "attestation_id":id,"attestation_digest":digest,"attestation":attestation,
        "qualification_id":id,"promotions":[promotion]
    });
    for (kind, value) in [
        ("request", &request),
        ("attestation", &attestation),
        ("result", &result),
    ] {
        let name = format!("agentdoc.cloud.migration_initialization_{kind}.v0.schema.json");
        assert_valid(&name, value);
        let validator = validator_for(&schema(&name));
        for field in value.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing.as_object_mut().unwrap().remove(field);
            assert!(!validator.is_valid(&missing), "{kind} requires {field}");
        }
        for (field, bad) in [
            ("untrusted", json!(true)),
            ("schema_version", json!("unknown")),
            ("request_id", json!("external-id")),
            ("qualification_id", json!("wrong-workspace")),
        ] {
            let mut invalid = value.clone();
            invalid[field] = bad;
            assert!(!validator.is_valid(&invalid), "{kind} rejects {field}");
        }
        for field in value
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.ends_with("_digest"))
        {
            let mut invalid = value.clone();
            invalid[field] = json!("sha256:short");
            assert!(
                !validator.is_valid(&invalid),
                "{kind} rejects malformed {field}"
            );
        }
    }
    for (kind, value) in [("request", &request), ("attestation", &attestation)] {
        let validator = validator_for(&schema(&format!(
            "agentdoc.cloud.migration_initialization_{kind}.v0.schema.json"
        )));
        for (field, bad) in [
            ("meaning", json!("verified_true")),
            ("rationale", json!(" \n\t")),
            ("rationale", json!("a".repeat(4097))),
            ("rationale", json!(false)),
        ] {
            let mut invalid = value.clone();
            invalid[field] = bad;
            assert!(!validator.is_valid(&invalid), "{kind} rejects {field}");
        }
        let mut boundary = value.clone();
        boundary["rationale"] = json!("a".repeat(4096));
        assert!(validator.is_valid(&boundary));
    }
    let name = "agentdoc.cloud.migration_initialization_attestation.v0.schema.json";
    let validator = validator_for(&schema(name));
    for (field, bad) in [
        ("qualification_policy_version", json!("2")),
        ("lifecycle_mapping_version", json!(1)),
        ("initialization_policy_version", json!("other")),
        ("authorization_policy_version", json!(" ")),
        ("authorization_policy_version", json!("a".repeat(1025))),
        ("attested_at", json!("2026-09-08")),
        ("principal_id", json!("caller-supplied-name")),
        ("identity_session_id", json!(null)),
        ("auth_session_id", json!("session")),
        ("external_identity_link_id", json!("link")),
        ("authorization_decision_id", json!("decision")),
    ] {
        let mut invalid = attestation.clone();
        invalid[field] = bad;
        assert!(!validator.is_valid(&invalid), "attestation rejects {field}");
    }
    let name = "agentdoc.cloud.migration_initialization_result.v0.schema.json";
    let validator = validator_for(&schema(name));
    for field in promotion.as_object().unwrap().keys() {
        let mut invalid = result.clone();
        invalid["promotions"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(!validator.is_valid(&invalid), "promotion requires {field}");
    }
    for field in [
        "promotion_seq",
        "governance_event_seq",
        "effectivity_event_seq",
    ] {
        for bad in [
            json!(0),
            json!(1),
            json!("0"),
            json!("01"),
            json!("-1"),
            json!("1.5"),
            json!("1e3"),
        ] {
            let mut invalid = result.clone();
            invalid["promotions"][0][field] = bad;
            assert!(
                !validator.is_valid(&invalid),
                "positive decimal string {field}"
            );
        }
    }
    for (field, bad) in [
        ("content_hash", json!("sha256:legacy")),
        ("content_digest", json!("sha256:legacy")),
        ("canonical_id", json!("external-id")),
        ("source_record_id", json!("job-record-id")),
        ("source_binding_id", json!("job-binding-id")),
        ("verified", json!(true)),
    ] {
        let mut invalid = result.clone();
        invalid["promotions"][0][field] = bad;
        assert!(!validator.is_valid(&invalid), "promotion rejects {field}");
    }
    for bad in [json!([]), json!([promotion, promotion])] {
        let mut invalid = result.clone();
        invalid["promotions"] = bad;
        assert!(!validator.is_valid(&invalid));
    }
    let mut boundary = result.clone();
    boundary["promotions"] = json!(
        (0..512)
            .map(|i| {
                let mut entry = promotion.clone();
                entry["object_id"] = json!(format!("test.{i:03}"));
                entry
            })
            .collect::<Vec<_>>()
    );
    assert!(validator.is_valid(&boundary));
    let mut extra = promotion.clone();
    extra["object_id"] = json!("test.extra");
    boundary["promotions"].as_array_mut().unwrap().push(extra);
    assert!(!validator.is_valid(&boundary));
    let mut nested = result.clone();
    nested["attestation"]["verified"] = json!(true);
    assert!(!validator.is_valid(&nested));
}

#[test]
fn migration_completion_preserves_original_bindings_and_native_correspondence() {
    let id = "00000000-0000-4000-8000-000000000001";
    let digest = format!("sha256:{}", "a".repeat(64));
    let mapping = json!({
        "object_id":"test.one","content_hash":format!("sha256:{}", "b".repeat(64)),
        "object_source_binding":{"connector":"git","source":"docs/index.adoc","path":"docs/index.adoc","anchor":"test.one","source_revision_digest":digest},
        "source_path":"docs/index.adoc","producer_source_record_id":"producer-record-one","producer_source_binding_id":"producer-binding-one",
        "canonical_id":id,"version_id":id,"content_digest":digest,"source_record_id":id,"source_binding_id":id,
        "promotion_seq":"9007199254740993","governance_event_seq":"9007199254740994","governance_event_digest":digest,
        "effectivity_event_seq":"9007199254740995","effectivity_event_digest":digest
    });
    let receipt = json!({
        "schema_version":"agentdoc.cloud.migration_completion_receipt.v0","workspace_id":id,
        "migration_request":{"schema_version":"adoc.migration_request.v0","request_id":"original-request","workspace_id":id,"source_id":"source-one","repository_identity":"repository-one","revision":{"system":"git","value":"a".repeat(40)},"evaluation_date":"2026-09-08"},
        "migration_request_digest":digest,"qualification_id":id,"qualification_envelope_digest":digest,
        "qualification_receipt_digest":digest,"attestation_id":id,"attestation_digest":digest,"mappings":[mapping]
    });
    let name = "agentdoc.cloud.migration_completion_receipt.v0.schema.json";
    assert_valid(name, &receipt);
    let validator = validator_for(&schema(name));
    // Revision is optional in the original object binding; adding it preserves validity.
    let mut revision = receipt.clone();
    revision["mappings"][0]["object_source_binding"]["revision"] = json!("a".repeat(40));
    assert!(validator.is_valid(&revision));
    for pointer in ["", "/mappings/0", "/mappings/0/object_source_binding"] {
        for key in receipt
            .pointer(pointer)
            .unwrap()
            .as_object()
            .unwrap()
            .keys()
        {
            let mut missing = receipt.clone();
            missing
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(key);
            assert!(!validator.is_valid(&missing), "required {pointer}/{key}");
        }
        let mut extra = receipt.clone();
        extra.pointer_mut(pointer).unwrap()["activation_authority"] = json!(true);
        assert!(!validator.is_valid(&extra), "closed {pointer}");
    }
    for (pointer, bad) in [
        ("/workspace_id", json!("producer-workspace")),
        ("/migration_request/revision/value", json!("main")),
        ("/attestation_digest", json!("sha256:short")),
        ("/mappings/0/content_hash", json!("sha256:legacy")),
        ("/mappings/0/content_digest", json!("sha256:legacy")),
        ("/mappings/0/source_record_id", json!("producer-record-one")),
        (
            "/mappings/0/source_binding_id",
            json!("producer-binding-one"),
        ),
        ("/mappings/0/producer_source_record_id", json!(null)),
        ("/mappings/0/producer_source_binding_id", json!("")),
        (
            "/mappings/0/object_source_binding/source_revision_digest",
            json!("short"),
        ),
    ] {
        let mut invalid = receipt.clone();
        *invalid.pointer_mut(pointer).unwrap() = bad;
        assert!(!validator.is_valid(&invalid), "reject {pointer}");
    }
    for path in [
        "../secret",
        "/absolute",
        "C:/drive",
        "docs\\file",
        "docs/../file",
        "docs/\nfile",
    ] {
        let mut invalid = receipt.clone();
        invalid["mappings"][0]["source_path"] = json!(path);
        assert!(!validator.is_valid(&invalid), "unsafe path {path:?}");
    }
    for field in [
        "promotion_seq",
        "governance_event_seq",
        "effectivity_event_seq",
    ] {
        for bad in [
            json!(1),
            json!("0"),
            json!("01"),
            json!("-1"),
            json!("1.5"),
            json!(null),
        ] {
            let mut invalid = receipt.clone();
            invalid["mappings"][0][field] = bad;
            assert!(!validator.is_valid(&invalid), "sequence {field}");
        }
    }
    for mappings in [json!([]), json!([mapping, mapping])] {
        let mut invalid = receipt.clone();
        invalid["mappings"] = mappings;
        assert!(!validator.is_valid(&invalid));
    }
    let mut boundary = receipt.clone();
    boundary["mappings"] = json!(
        (0..512)
            .map(|i| {
                let mut entry = mapping.clone();
                entry["object_id"] = json!(format!("test.{i:03}"));
                entry
            })
            .collect::<Vec<_>>()
    );
    assert!(validator.is_valid(&boundary));
    let mut extra = mapping.clone();
    extra["object_id"] = json!("test.extra");
    boundary["mappings"].as_array_mut().unwrap().push(extra);
    assert!(!validator.is_valid(&boundary));
    // The E4.4 pass-through receipt remains a separate contract.
    assert_valid(
        "agentdoc.cloud.migration_receipt.v0.schema.json",
        &json!({"schema_version":"agentdoc.cloud.migration_receipt.v0","payload":{"legacy":true}}),
    );
}

#[test]
fn migration_native_facts_are_closed_nonnullable_and_preserve_sequence_precision() {
    let id = "00000000-0000-4000-8000-000000000001";
    let digest = format!("sha256:{}", "a".repeat(64));
    let source = json!({"path":"docs/index.adoc","source_record_id":id,"source_binding_id":id});
    let candidate = json!({"object_id":"test.one","canonical_id":id,"version_id":id,"source_record_id":id,"source_binding_id":id});
    let import = json!({"schema_version":"agentdoc.cloud.migration_import_result.v0","request_id":"external-request","candidates":[candidate]});
    let qualification = json!({"schema_version":"agentdoc.cloud.migration_qualification_result.v0","request_id":"external-request","qualification_id":id,"outcome":"evaluated","candidates":[candidate],"sources":[source]});
    let cases = [
        (
            "migration_candidate_import",
            json!({"workspace_id":id,"request_id":"external-request","request_digest":digest,"admission_digest":digest,"principal":id,"result":import,"created_at":"2026-09-08T14:23:01.123456+00:00"}),
        ),
        (
            "migration_qualification",
            json!({"workspace_id":id,"id":id,"request_id":"external-request","principal":id,"admission_digest":digest,"envelope_digest":digest,"outcome":"evaluated","qualification_policy_version":"1","result":qualification,"created_at":"2026-09-08T14:23:01Z"}),
        ),
        (
            "migration_qualification_source",
            json!({"workspace_id":id,"qualification_id":id,"path":"docs/index.adoc","source_record_id":id,"source_binding_id":id}),
        ),
        (
            "migration_initialization",
            json!({"workspace_id":id,"id":id,"qualification_id":id,"request_id":id,"request_digest":digest,"principal":id,"identity_session_id":id,"auth_session_id":id,"external_identity_link_id":id,"authorization_decision_id":id,"attestation_digest":digest}),
        ),
        (
            "migration_initialization_target",
            json!({"workspace_id":id,"initialization_id":id,"object_id":"test.one","canonical_id":id,"version_id":id,"content_hash":digest,"content_digest":digest,"source_record_id":id,"source_binding_id":id,"promotion_seq":"9007199254740993","governance_event_seq":"9007199254740994","effectivity_event_seq":"9007199254740995"}),
        ),
    ];
    let name = "agentdoc.cloud.export_native_fact.v0.schema.json";
    let validator = validator_for(&schema(name));
    for (kind, record) in cases {
        let value = json!({"schema_version":"agentdoc.cloud.export_native_fact.v0","kind":kind,"record":record});
        assert_valid(name, &value);
        for key in record.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing["record"].as_object_mut().unwrap().remove(key);
            assert!(!validator.is_valid(&missing), "{kind} requires {key}");
            let mut null = value.clone();
            null["record"][key] = json!(null);
            assert!(!validator.is_valid(&null), "{kind} nonnull {key}");
            if key.ends_with("_seq") {
                for bad in [json!(1), json!("0"), json!("01")] {
                    let mut invalid = value.clone();
                    invalid["record"][key] = bad;
                    assert!(!validator.is_valid(&invalid), "{kind} decimal {key}");
                }
            }
        }
        for pointer in ["", "/record"] {
            let mut extra = value.clone();
            extra.pointer_mut(pointer).unwrap()["raw_source_bytes"] = json!("payload");
            assert!(!validator.is_valid(&extra), "{kind} closed {pointer}");
        }
        let mut invalid = value.clone();
        invalid["record"]["workspace_id"] = json!("external-workspace");
        assert!(!validator.is_valid(&invalid));
        if kind == "migration_qualification" {
            let mut flagged = value.clone();
            flagged["record"]["outcome"] = json!("flagged_source_evidence");
            flagged["record"]["result"]["outcome"] = json!("flagged_source_evidence");
            flagged["record"]["result"]["candidates"] = json!([]);
            assert!(validator.is_valid(&flagged));
            flagged["record"]["qualification_policy_version"] = json!("2");
            assert!(!validator.is_valid(&flagged));
        }
        if kind == "migration_initialization" {
            invalid = value.clone();
            invalid["record"]["request_id"] = json!("external-request");
            assert!(!validator.is_valid(&invalid));
        }
    }
}

#[test]
fn migration_export_actual_native_outputs_match_published_contracts() {
    // Unmodified JSON values from real pinned-runtime and human-authorized Cloud
    // exports. This checks wire shape, not native authorization or causal joins.
    let records: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/migration-export-native.json")).unwrap();
    let mut kinds = BTreeSet::new();
    let mut outcomes = BTreeSet::new();
    let mut completion_count = 0;
    for record in &records {
        let version = record["schema_version"].as_str().unwrap();
        assert_valid(&format!("{version}.schema.json"), record);
        if version == "agentdoc.cloud.export_native_fact.v0" {
            kinds.insert(record["kind"].as_str().unwrap());
            if record["kind"] == "migration_qualification" {
                outcomes.insert(record["record"]["outcome"].as_str().unwrap());
            }
        } else {
            assert_eq!(version, "agentdoc.cloud.migration_completion_receipt.v0");
            completion_count += 1;
            let mappings = record["mappings"].as_array().unwrap();
            assert_eq!(mappings.len(), 2);
            for mapping in mappings {
                assert_ne!(mapping["content_hash"], mapping["content_digest"]);
                assert_ne!(
                    mapping["producer_source_record_id"],
                    mapping["source_record_id"]
                );
                assert_ne!(
                    mapping["producer_source_binding_id"],
                    mapping["source_binding_id"]
                );
                assert_eq!(
                    mapping["object_source_binding"]["anchor"],
                    mapping["object_id"]
                );
                assert_eq!(mapping["object_source_binding"]["connector"], "local_fs");
                assert!(mapping["object_source_binding"].get("revision").is_none());
            }
        }
    }
    assert_eq!(completion_count, 1);
    assert_eq!(
        kinds,
        BTreeSet::from([
            "migration_candidate_import",
            "migration_qualification",
            "migration_qualification_source",
            "migration_initialization",
            "migration_initialization_target",
        ])
    );
    assert_eq!(
        outcomes,
        BTreeSet::from(["evaluated", "flagged_source_evidence"])
    );
}

#[test]
fn migration_transition_contracts_preserve_closed_evidence_and_initial_form() {
    let id = "00000000-0000-4000-8000-000000000001";
    let digest = format!("sha256:{}", "a".repeat(64));
    let request = json!({"schema_version":"agentdoc.cloud.migration_transition_request.v0",
        "migration_id":id,"transition_id":id,"expected":{"ordinal":"1","receipt_digest":digest},
        "to_state":"snapshot_bound","evidence":{"kind":"preparation","digest":digest}});
    let evidence = [
        json!({"kind":"preparation","digest":digest}),
        json!({"kind":"import_job","digest":digest}),
        json!({"kind":"qualification","qualification_id":id,"envelope_digest":digest}),
        json!({"kind":"initialization","initialization_id":id,"attestation_digest":digest}),
        json!({"kind":"failure","reason":"worker_refused"}),
        json!({"kind":"failure","reason":"validation_failed","qualification_id":id,"envelope_digest":digest}),
        json!({"kind":"failure","reason":"no_eligible_targets","qualification_id":id,"envelope_digest":digest}),
    ];
    let request_name = "agentdoc.cloud.migration_transition_request.v0.schema.json";
    let request_validator = validator_for(&schema(request_name));
    for entry in &evidence {
        let mut value = request.clone();
        value["evidence"] = entry.clone();
        assert_valid(request_name, &value);
        for key in entry.as_object().unwrap().keys() {
            let mut missing = value.clone();
            missing["evidence"].as_object_mut().unwrap().remove(key);
            assert!(
                !request_validator.is_valid(&missing),
                "evidence requires {key}"
            );
        }
        value["evidence"]["freeform"] = json!("untrusted source text");
        assert!(!request_validator.is_valid(&value));
    }
    for bad in [
        json!({"kind":"candidate_import","digest":digest}),
        json!({"kind":"failure","reason":"worker_refused","qualification_id":id,"envelope_digest":digest}),
        json!({"kind":"failure","reason":"unknown"}),
        json!({"kind":"qualification","qualification_id":id,"envelope_digest":digest,"digest":digest}),
    ] {
        let mut value = request.clone();
        value["evidence"] = bad;
        assert!(!request_validator.is_valid(&value));
    }
    let scope = json!({"workspace_id":id,"connector_id":id,"source_container_id":"source-one","resource":{"kind":"repository","id":"repo-one"}});
    let receipt = json!({"schema_version":"agentdoc.cloud.migration_transition_receipt.v0",
        "workspace_id":id,"migration_id":id,"transition_id":id,"ordinal":"1","previous_receipt_digest":null,
        "command_digest":digest,"request_digest":digest,"job_digest":digest,"preparation_receipt_digest":digest,
        "scope":scope,"initial_revision":"a".repeat(40),"from_state":null,"to_state":"prepared",
        "transition_policy_version":"1","evidence":evidence[0],"principal_id":id,
        "authorization_decision_id":id,"recorded_at":"2026-09-08T14:23:01.123456+00:00"});
    let receipt_name = "agentdoc.cloud.migration_transition_receipt.v0.schema.json";
    let receipt_validator = validator_for(&schema(receipt_name));
    assert_valid(receipt_name, &receipt);
    let mut successor = receipt.clone();
    successor["ordinal"] = json!("9007199254740993");
    successor["previous_receipt_digest"] = json!(digest);
    successor["from_state"] = json!("prepared");
    successor["to_state"] = json!("snapshot_bound");
    assert!(receipt_validator.is_valid(&successor));
    for (field, bad) in [
        ("from_state", json!("prepared")),
        ("previous_receipt_digest", json!(digest)),
        ("to_state", json!("snapshot_bound")),
        ("evidence", evidence[1].clone()),
    ] {
        let mut invalid = receipt.clone();
        invalid[field] = bad;
        assert!(!receipt_validator.is_valid(&invalid), "initial {field}");
    }
    for field in ["from_state", "previous_receipt_digest"] {
        let mut invalid = successor.clone();
        invalid[field] = json!(null);
        assert!(!receipt_validator.is_valid(&invalid), "successor {field}");
    }
    for bad in [json!(1), json!("0"), json!("01"), json!("-1"), json!("1.5")] {
        let mut invalid = receipt.clone();
        invalid["ordinal"] = bad.clone();
        assert!(!receipt_validator.is_valid(&invalid));
        let mut invalid = request.clone();
        invalid["expected"]["ordinal"] = bad;
        assert!(!request_validator.is_valid(&invalid));
    }
    for (field, bad) in [
        ("initial_revision", json!("0".repeat(40))),
        ("initial_revision", json!("main")),
        ("transition_policy_version", json!("2")),
        ("to_state", json!("unknown")),
        ("principal_id", json!("actor")),
        ("recorded_at", json!("2026-09-08")),
    ] {
        let mut invalid = successor.clone();
        invalid[field] = bad;
        assert!(!receipt_validator.is_valid(&invalid), "reject {field}");
    }
    for pointer in ["/scope", "/scope/resource"] {
        let mut invalid = receipt.clone();
        invalid.pointer_mut(pointer).unwrap()["extra"] = json!(true);
        assert!(!receipt_validator.is_valid(&invalid));
    }
    let result = json!({"schema_version":"agentdoc.cloud.migration_transition_result.v0","receipt_bytes_base64":"e30K","receipt_digest":digest});
    let result_name = "agentdoc.cloud.migration_transition_result.v0.schema.json";
    assert_valid(result_name, &result);
    let result_validator = validator_for(&schema(result_name));
    for bad in ["", "e30K\n", "e30_", "abc", "===="] {
        let mut invalid = result.clone();
        invalid["receipt_bytes_base64"] = json!(bad);
        assert!(!result_validator.is_valid(&invalid));
    }
    for (name, value) in [
        (request_name, &request),
        (receipt_name, &receipt),
        (result_name, &result),
    ] {
        let validator = validator_for(&schema(name));
        for field in value.as_object().unwrap().keys() {
            let mut invalid = value.clone();
            invalid.as_object_mut().unwrap().remove(field);
            assert!(!validator.is_valid(&invalid), "{name} requires {field}");
        }
        let mut extra = value.clone();
        extra["receipt"] = json!({});
        assert!(!validator.is_valid(&extra));
        let mut unknown = value.clone();
        unknown["schema_version"] = json!("unknown");
        assert!(!validator.is_valid(&unknown));
    }
    let validation = json!({"schema_version":"adoc.validation_receipt.v0","runtime":{"version":"0.4.0","binary_digest":digest},"contract_versions":{"graph":"adoc.graph.v6"},"evaluation_date":"2026-09-08","inputs":[{"path":"docs/index.adoc","digest":digest}],"context":[{"name":"config","digest":digest}],"result":"pass","diagnostics_digest":digest});
    let parent = json!({"workspace_id":id,"migration_id":id,"source_request_id":"external-request","request_digest":digest,"job_digest":digest,"preparation_receipt_digest":digest,"preparation_validation":validation,"connector_id":id,"scope":scope,"initial_revision":"a".repeat(40),"principal":id,"authorization_decision_id":id,"created_at":receipt["recorded_at"]});
    let mut child = receipt.clone();
    for field in [
        "schema_version",
        "request_digest",
        "job_digest",
        "preparation_receipt_digest",
        "scope",
        "initial_revision",
    ] {
        child.as_object_mut().unwrap().remove(field);
    }
    let principal = child
        .as_object_mut()
        .unwrap()
        .remove("principal_id")
        .unwrap();
    child["principal"] = principal;
    child["receipt_digest"] = json!(digest);
    let native_name = "agentdoc.cloud.export_native_fact.v0.schema.json";
    let native_validator = validator_for(&schema(native_name));
    for (kind, record) in [
        ("migration_lifecycle", parent),
        ("migration_lifecycle_transition", child),
    ] {
        let value = json!({"schema_version":"agentdoc.cloud.export_native_fact.v0","kind":kind,"record":record});
        assert_valid(native_name, &value);
        for key in record.as_object().unwrap().keys() {
            let mut invalid = value.clone();
            invalid["record"].as_object_mut().unwrap().remove(key);
            assert!(
                !native_validator.is_valid(&invalid),
                "{kind} requires {key}"
            );
        }
        for key in [
            "request_bytes",
            "job_bytes",
            "command_bytes",
            "receipt_bytes",
            "diagnostics",
            "input_metadata",
        ] {
            let mut invalid = value.clone();
            invalid["record"][key] = json!("source canary");
            assert!(!native_validator.is_valid(&invalid));
        }
        if kind == "migration_lifecycle_transition" {
            let mut invalid = value.clone();
            invalid["record"]["ordinal"] = json!("2");
            assert!(!native_validator.is_valid(&invalid));
        }
    }
}

#[test]
fn migration_lifecycle_actual_native_outputs_match_closed_contracts() {
    let records: Vec<serde_json::Value> =
        serde_json::from_str(include_str!("fixtures/migration-lifecycle-native.json")).unwrap();
    assert_eq!(records.len(), 24);
    for record in &records {
        assert_valid(
            &format!("{}.schema.json", record["schema_version"].as_str().unwrap()),
            record,
        );
    }
    let select = |version: &str| {
        records
            .iter()
            .filter(|r| r["schema_version"] == version)
            .collect::<Vec<_>>()
    };
    let results = select("agentdoc.cloud.migration_transition_result.v0");
    let receipts = select("agentdoc.cloud.migration_transition_receipt.v0");
    let commands = select("agentdoc.cloud.migration_transition_request.v0");
    assert_eq!(results.len(), 6);
    assert_eq!(receipts.len(), 6);
    assert_eq!(commands.len(), 5);
    let states = [
        "prepared",
        "snapshot_bound",
        "importing",
        "validated",
        "awaiting_attestation",
        "catching_up",
    ];
    for (i, receipt) in receipts.iter().enumerate() {
        assert_eq!(receipt["ordinal"], (i + 1).to_string());
        assert_eq!(receipt["to_state"], states[i]);
        assert_eq!(receipt["scope"], receipts[0]["scope"]);
        assert_eq!(receipt["initial_revision"], receipts[0]["initial_revision"]);
        if i == 0 {
            assert!(receipt["from_state"].is_null());
            assert!(receipt["previous_receipt_digest"].is_null());
            assert_eq!(receipt["transition_id"], receipt["migration_id"]);
        } else {
            assert_eq!(receipt["from_state"], states[i - 1]);
            assert_eq!(
                receipt["previous_receipt_digest"],
                results[i - 1]["receipt_digest"]
            );
            assert_eq!(
                commands[i - 1]["expected"]["ordinal"],
                receipts[i - 1]["ordinal"]
            );
            assert_eq!(
                commands[i - 1]["expected"]["receipt_digest"],
                results[i - 1]["receipt_digest"]
            );
            assert_eq!(commands[i - 1]["evidence"], receipt["evidence"]);
            assert_eq!(commands[i - 1]["transition_id"], receipt["transition_id"]);
        }
    }
    let facts = select("agentdoc.cloud.export_native_fact.v0");
    assert_eq!(facts.len(), 7);
    let parents = facts
        .iter()
        .filter(|f| f["kind"] == "migration_lifecycle")
        .collect::<Vec<_>>();
    assert_eq!(parents.len(), 1);
    let parent = &parents[0]["record"];
    assert_eq!(parent["scope"], receipts[0]["scope"]);
    assert_eq!(
        parent["preparation_receipt_digest"],
        receipts[0]["preparation_receipt_digest"]
    );
    assert_eq!(parent["preparation_validation"]["result"], "pass");
    assert!(
        parent["preparation_validation"]
            .get("diagnostics")
            .is_none()
    );
    assert!(parent.get("preparation_receipt_bytes").is_none());
    let children = facts
        .iter()
        .filter(|f| f["kind"] == "migration_lifecycle_transition")
        .collect::<Vec<_>>();
    assert_eq!(children.len(), 6);
    for child in children {
        let record = &child["record"];
        let i = record["ordinal"]
            .as_str()
            .unwrap()
            .parse::<usize>()
            .unwrap()
            - 1;
        assert_eq!(record["receipt_digest"], results[i]["receipt_digest"]);
        assert_eq!(record["command_digest"], receipts[i]["command_digest"]);
        assert_eq!(record["principal"], receipts[i]["principal_id"]);
        assert_eq!(record["evidence"], receipts[i]["evidence"]);
    }
}
