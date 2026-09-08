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
fn run_command(root: &Path, request: &Value, job: &Value, command: &str, policy: &str) -> Output {
    let inputs = TestWorkspace::new("migration-import-input");
    let request_path = inputs.root.join("request.json");
    let job_path = inputs.root.join("job.json");
    fs::write(&request_path, serde_json::to_vec(request).unwrap()).unwrap();
    fs::write(&job_path, serde_json::to_vec(job).unwrap()).unwrap();
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_adoc"));
    cmd.arg(command);
    if command == "migration-qualify" {
        cmd.args(["--qualification-policy-version", policy]);
    }
    cmd.arg("--request")
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

fn run(root: &Path, request: &Value, job: &Value) -> Output {
    run_command(root, request, job, "migration-qualify", "1")
}
#[test]
fn qualification_retains_t2_bytes_and_draft_candidates_without_authority() {
    let (workspace, request, job) = fixture();
    let imported = run_command(&workspace.root, &request, &job, "migration-import", "1");
    assert!(imported.status.success());
    let output = run(&workspace.root, &request, &job);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        envelope["candidate_bundle_bytes"]
            .as_str()
            .unwrap()
            .as_bytes(),
        imported.stdout
    );
    let receipt = nested(&envelope, "qualification_receipt_bytes");
    assert_eq!(receipt["objects"].as_array().unwrap().len(), 2);
    for object in receipt["objects"].as_array().unwrap() {
        assert_eq!(object["eligible"], false);
        assert_eq!(object["reasons"][0]["code"], "lifecycle_not_adopted");
    }
    assert_eq!(output.stdout, run(&workspace.root, &request, &job).stdout);
    fs::write(workspace.root.join("docs/two.adoc"), "dirty").unwrap();
    assert_eq!(output.stdout, run(&workspace.root, &request, &job).stdout);
}
#[test]
fn qualification_preserves_failed_raw_evidence_including_invalid_utf8() {
    let (workspace, mut request, job) = fixture();
    for bytes in [
        b"# Invalid @doc(invalid.page)\n\n::claim missing.status\n--\nBody\n::\n".as_slice(),
        &[0xff, 0xfe],
    ] {
        fs::write(workspace.root.join("docs/two.adoc"), bytes).unwrap();
        git(&workspace.root, &["add", "."]);
        git(&workspace.root, &["commit", "-qm", "invalid source"]);
        request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
        let output = run(&workspace.root, &request, &job);
        assert_eq!(
            output.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(envelope["outcome"], "flagged_source_evidence");
        assert!(envelope.get("candidate_bundle_bytes").is_none());
        assert!(envelope.get("graph_artifact_bytes").is_none());
        let receipt = nested(&envelope, "validation_receipt_bytes");
        assert_eq!(receipt["result"], "fail");
        assert_eq!(envelope["sources"].as_array().unwrap().len(), 2);
        assert!(
            !nested(&envelope, "diagnostics_bytes")
                .as_array()
                .unwrap()
                .is_empty()
        );
        if bytes == [0xff, 0xfe] {
            assert_eq!(envelope["sources"][1]["source_bytes_base64"], "//4=");
        }
        assert_eq!(output.stdout, run(&workspace.root, &request, &job).stdout);
        assert_eq!(
            run_command(&workspace.root, &request, &job, "migration-import", "1")
                .status
                .code(),
            Some(2)
        );
    }
}
#[test]
fn qualification_refuses_unknown_policy_and_incomplete_coverage() {
    let (workspace, request, mut job) = fixture();
    let output = run_command(&workspace.root, &request, &job, "migration-qualify", "2");
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    job["sources"].as_array_mut().unwrap().pop();
    let output = run(&workspace.root, &request, &job);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
}

#[test]
fn qualification_uses_fixed_date_and_cross_source_contradiction_facts() {
    let (workspace, mut request, job) = fixture();
    let verified = |id: &str, expiry: &str| {
        format!(
            "::claim {id}\nstatus: verified\nowner: team\nverified_at: 2026-01-01\ntest: cargo test\nexpires_at: {expiry}\n--\nBody.\n::\n\n"
        )
    };
    fs::write(workspace.root.join("docs/one.adoc"), format!("# One @doc(test.one.page)\n\n{}{}{}::policy test.policy\nstatus: active\nowner: team\napproved_by: team\neffective_at: 2026-01-01\nreview_interval: 30d\n--\nPolicy.\n::\n", verified("test.current", "2026-09-08"), verified("test.expired", "2026-09-07"), verified("test.clear", "2026-09-09"))).unwrap();
    fs::write(workspace.root.join("docs/two.adoc"), "# Two @doc(test.two.page)\n\n::contradiction test.conflict\nseverity: high\nstatus: unresolved\nclaims: [test.current, test.expired]\n--\nConflict.\n::\n").unwrap();
    git(&workspace.root, &["add", "."]);
    git(&workspace.root, &["commit", "-qm", "qualification facts"]);
    request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
    let output = run(&workspace.root, &request, &job);
    assert_eq!(
        output.status.code(),
        Some(0),
        "{} {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    let envelope: Value = serde_json::from_slice(&output.stdout).unwrap();
    let receipt = nested(&envelope, "qualification_receipt_bytes");
    let objects = receipt["objects"].as_array().unwrap();
    let find = |id: &str| {
        objects
            .iter()
            .find(|object| object["object_id"] == id)
            .unwrap()
    };
    assert_eq!(find("test.clear")["eligible"], true);
    assert_eq!(
        find("test.current")["reasons"],
        json!([{"code":"contradicted","related_object_ids":["test.conflict"],"diagnostic_codes":[]}])
    );
    assert_eq!(
        find("test.expired")["reasons"],
        json!([{"code":"stale","related_object_ids":[],"diagnostic_codes":[]},{"code":"contradicted","related_object_ids":["test.conflict"],"diagnostic_codes":[]}])
    );
    assert_eq!(find("test.policy")["reasons"][0]["code"], "review_overdue");
    assert_eq!(find("test.conflict")["source_record_id"], "record-2");
}

#[test]
fn qualification_refuses_encoded_output_overflow_without_partial_stdout() {
    let (workspace, mut request, job) = fixture();
    // Raw input is below the 8MiB cap, but canonical padded base64 exceeds it.
    fs::write(
        workspace.root.join("docs/two.adoc"),
        vec![0xff; 7 * 1024 * 1024],
    )
    .unwrap();
    git(&workspace.root, &["add", "."]);
    git(&workspace.root, &["commit", "-qm", "large invalid binary"]);
    request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
    let output = run(&workspace.root, &request, &job);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("migration.output_limit"));
}

#[test]
fn qualification_refuses_unsafe_checkout_instead_of_retaining_it() {
    let (workspace, mut request, job) = fixture();
    fs::write(
        workspace.root.join(".gitattributes"),
        "*.adoc filter=external\n",
    )
    .unwrap();
    git(&workspace.root, &["add", "."]);
    git(&workspace.root, &["commit", "-qm", "unsafe attributes"]);
    request["revision"]["value"] = json!(git(&workspace.root, &["rev-parse", "HEAD"]));
    let output = run(&workspace.root, &request, &job);
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("migration.unsafe_source"));
}
