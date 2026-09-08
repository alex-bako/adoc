mod support;
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use support::TestWorkspace;

fn git(root: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout).unwrap().trim().into()
}
fn fixture() -> (TestWorkspace, Value, Value) {
    let workspace = TestWorkspace::new("migration-import");
    let root = &workspace.root;
    git(root, &["init", "-q"]);
    git(root, &["config", "user.email", "test@example.test"]);
    git(root, &["config", "user.name", "Test"]);
    fs::create_dir(root.join("docs")).unwrap();
    fs::write(
        root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\n",
    )
    .unwrap();
    for name in ["one", "two"] {
        fs::write(root.join(format!("docs/{name}.adoc")), format!("# {name} @doc(test.{name}.page)\n\n::claim test.{name}\nstatus: draft\n--\nBody.\n::\n")).unwrap();
    }
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "source"]);
    let request = json!({"schema_version":"adoc.migration_request.v0","request_id":"request-1","workspace_id":"workspace-1","source_id":"source-1","repository_identity":"repo:test","revision":{"system":"git","value":git(root,&["rev-parse","HEAD"])},"evaluation_date":"2026-09-08"});
    let job = json!({"schema_version":"agentdoc.cloud.migration_import_job.v0","connector_id":"connector-1","observed_at":"2026-09-08T12:00:00Z","source_acl_scope":{"snapshot_id":"acl-1","source_container_id":"source-1","source":{"kind":"repository","id":"repo:test"}},"sources":[{"path":"docs/one.adoc","source_record_id":"record-1","source_binding_id":"binding-1"},{"path":"docs/two.adoc","source_record_id":"record-2","source_binding_id":"binding-2"}]});
    (workspace, request, job)
}
fn run(root: &Path, request: &Value, job: &Value) -> Output {
    let inputs = TestWorkspace::new("migration-import-input");
    let request_path = inputs.root.join("request.json");
    let job_path = inputs.root.join("job.json");
    fs::write(&request_path, serde_json::to_vec(request).unwrap()).unwrap();
    fs::write(&job_path, serde_json::to_vec(job).unwrap()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_adoc"))
        .args(["migration-import", "--request"])
        .arg(request_path)
        .arg("--job")
        .arg(job_path)
        .arg("--repository")
        .arg(root)
        .args([
            "--runtime-binary-digest",
            &format!("sha256:{}", "a".repeat(64)),
        ])
        .output()
        .unwrap()
}
fn nested(source: &Value, key: &str) -> Value {
    serde_json::from_str(source[key].as_str().unwrap()).unwrap()
}

#[test]
fn migration_import_exports_exact_sources_and_full_snapshot_receipts() {
    let (workspace, request, job) = fixture();
    let output = run(&workspace.root, &request, &job);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let bundle: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(bundle["schema_version"], "adoc.migration_import.v0");
    assert_eq!(bundle["request"], request);
    let graph: Value =
        serde_json::from_str(bundle["graph_artifact_bytes"].as_str().unwrap()).unwrap();
    let nodes = graph["nodes"].as_array().unwrap();
    for name in ["one", "two"] {
        assert!(
            nodes
                .iter()
                .any(|node| node["id"] == format!("test.{name}"))
        );
    }
    assert_eq!(bundle["sources"].as_array().unwrap().len(), 2);
    for source in bundle["sources"].as_array().unwrap() {
        let receipt = nested(source, "validation_receipt_bytes");
        assert_eq!(receipt["schema_version"], "adoc.validation_receipt.v1");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["inputs"].as_array().unwrap().len(), 2);
        assert_eq!(receipt["context"].as_array().unwrap().len(), 3);
        let record = nested(source, "source_record_bytes");
        assert_eq!(record["source"]["provider"], "git");
        assert_eq!(record["source"]["external_id"], source["path"]);
        assert_eq!(
            record["source"]["external_version"],
            request["revision"]["value"]
        );
        adoc_core::validate_source_record(
            source["source_record_bytes"].as_str().unwrap().as_bytes(),
            source["source_bytes"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        adoc_core::validate_source_binding(
            source["source_binding_bytes"].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        let invocation = nested(source, "source_invocation_bytes");
        assert_eq!(
            invocation["schema_version"],
            "agentdoc.cloud.migration_validation_invocation.v0"
        );
        assert_eq!(invocation["source_path"], source["path"]);
        assert_eq!(invocation["request_digest"], bundle["request_digest"]);
        let binding = nested(source, "source_binding_bytes");
        assert_eq!(binding["binding"]["connector"], "git");
        assert_eq!(binding["binding"]["source"], source["path"]);
        assert_eq!(binding["binding"]["anchor"], "document");
        // The ordinary CLI must produce byte-identical full-snapshot evidence from
        // these exported context bytes; this also checks their digest bindings.
        let contexts = TestWorkspace::new("migration-import-context");
        let graph_path = contexts.root.join("graph.json");
        let invocation_path = contexts.root.join("invocation.json");
        let receipt_path = contexts.root.join("receipt.json");
        fs::write(
            &graph_path,
            bundle["graph_artifact_bytes"].as_str().unwrap(),
        )
        .unwrap();
        fs::write(
            &invocation_path,
            source["source_invocation_bytes"].as_str().unwrap(),
        )
        .unwrap();
        let checked = Command::new(env!("CARGO_BIN_EXE_adoc"))
            .current_dir(&workspace.root)
            .args([
                "check",
                "--as-of",
                "2026-09-08",
                "--runtime-binary-digest",
                &format!("sha256:{}", "a".repeat(64)),
            ])
            .arg("--receipt")
            .arg(&receipt_path)
            .arg("--source-invocation")
            .arg(&invocation_path)
            .arg("--context-artifact")
            .arg(&graph_path)
            .output()
            .unwrap();
        assert!(
            checked.status.success(),
            "{}",
            String::from_utf8_lossy(&checked.stderr)
        );
        assert_eq!(
            fs::read_to_string(receipt_path).unwrap(),
            source["validation_receipt_bytes"].as_str().unwrap()
        );
    }
    let original_one = nodes.iter().find(|node| node["id"] == "test.one").unwrap();
    let original_two = nodes.iter().find(|node| node["id"] == "test.two").unwrap();
    assert_eq!(original_one["content_hash"], original_two["content_hash"]);
    assert_eq!(original_one["source_binding"]["anchor"], "test.one");
    assert_eq!(original_two["source_binding"]["anchor"], "test.two");
    assert!(!String::from_utf8_lossy(&output.stdout).contains("adoc-worktree-"));
    fs::write(
        workspace.root.join("docs/two.adoc"),
        "dirty invalid content",
    )
    .unwrap();
    fs::write(
        workspace.root.join("agentdoc.config.yaml"),
        "version: invalid\n",
    )
    .unwrap();
    assert_eq!(output.stdout, run(&workspace.root, &request, &job).stdout);
}
#[test]
fn migration_import_refuses_incomplete_metadata_and_invalid_compilation() {
    let (workspace, mut request, job) = fixture();
    let mut missing = job.clone();
    missing["sources"].as_array_mut().unwrap().pop();
    assert_eq!(
        run(&workspace.root, &request, &missing).status.code(),
        Some(2)
    );
    let mut foreign = job.clone();
    foreign["source_acl_scope"]["source"]["id"] = json!("another-repo");
    assert_eq!(
        run(&workspace.root, &request, &foreign).status.code(),
        Some(2)
    );
    let mut duplicate = job.clone();
    duplicate["sources"][1]["source_record_id"] = json!("record-1");
    assert_eq!(
        run(&workspace.root, &request, &duplicate).status.code(),
        Some(2)
    );
    fs::write(workspace.root.join("docs/two.adoc"), [0xff, 0xfe]).unwrap();
    git(&workspace.root, &["add", "."]);
    git(&workspace.root, &["commit", "-qm", "invalid UTF8"]);
    request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
    let failed = run(&workspace.root, &request, &job);
    assert_eq!(failed.status.code(), Some(2));
    assert!(failed.stdout.is_empty());
    assert!(String::from_utf8_lossy(&failed.stderr).contains("migration.validation_failed"));
}

#[test]
fn migration_import_refuses_bounded_inputs_and_output_without_partial_stdout() {
    let (workspace, mut request, job) = fixture();
    let mut huge_job = job.clone();
    huge_job["connector_id"] = json!("x".repeat(adoc_core::MIGRATION_IMPORT_JOB_MAX_BYTES));
    let out = run(&workspace.root, &request, &huge_job);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.invalid_job"));
    let mut huge_request = request.clone();
    huge_request["request_id"] = json!("x".repeat(adoc_core::MIGRATION_REQUEST_MAX_BYTES));
    assert_eq!(
        run(&workspace.root, &huge_request, &job).status.code(),
        Some(2)
    );
    fs::write(
        workspace.root.join("docs/two.adoc"),
        format!(
            "# Large @doc(test.large)\n\n{}\n",
            "x".repeat(adoc_core::MIGRATION_IMPORT_MAX_BYTES)
        ),
    )
    .unwrap();
    git(&workspace.root, &["add", "."]);
    git(&workspace.root, &["commit", "-qm", "oversized bundle"]);
    request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
    let out = run(&workspace.root, &request, &job);
    assert_eq!(out.status.code(), Some(2));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.output_limit"));
}
