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
fn repo() -> (TestWorkspace, Value) {
    let dir = TestWorkspace::new("migration-prepare");
    git(&dir.root, &["init", "-q"]);
    git(&dir.root, &["config", "user.email", "test@example.test"]);
    git(&dir.root, &["config", "user.name", "Test"]);
    fs::create_dir(dir.root.join("docs")).unwrap();
    fs::write(
        dir.root.join("agentdoc.config.yaml"),
        "version: 1\nmode: strict\ndocs_path: docs\n",
    )
    .unwrap();
    for name in ["one", "two"] {
        fs::write(
            dir.root.join(format!("docs/{name}.adoc")),
            format!("# {name} @doc(test.{name}.page)\n\n::claim test.{name}\nstatus: draft\n--\nBody.\n::\n"),
        )
        .unwrap();
    }
    git(&dir.root, &["add", "."]);
    git(&dir.root, &["commit", "-qm", "source"]);
    let sha = git(&dir.root, &["rev-parse", "HEAD"]);
    (
        dir,
        json!({"schema_version":"adoc.migration_request.v0", "request_id":"request-1", "workspace_id":"workspace-1", "source_id":"source-1", "repository_identity":"repo:test", "revision":{"system":"git", "value":sha}, "evaluation_date":"2026-09-08"}),
    )
}
fn run(root: &Path, request: &Value) -> Output {
    let input = TestWorkspace::new("migration-request");
    let request_path = input.root.join("request.json");
    fs::write(&request_path, serde_json::to_vec(request).unwrap()).unwrap();
    Command::new(env!("CARGO_BIN_EXE_adoc"))
        .args(["migration-prepare", "--request"])
        .arg(&request_path)
        .arg("--repository")
        .arg(root)
        .args([
            "--runtime-binary-digest",
            &format!("sha256:{}", "a".repeat(64)),
        ])
        .output()
        .unwrap()
}
#[test]
fn migration_prepare_pins_full_graph_and_ignores_dirty_and_newer_source() {
    let (dir, request) = repo();
    let first = run(&dir.root, &request);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let receipt: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(receipt["phase"], "prepare");
    assert_eq!(receipt["request"], request);
    assert_eq!(
        receipt["validation_receipt"]["inputs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    fs::write(
        dir.root.join("docs/two.adoc"),
        "# Two @doc(test.two.page)\n\n::claim test.two\n--\nInvalid.\n::\n",
    )
    .unwrap();
    assert_eq!(first.stdout, run(&dir.root, &request).stdout);
    git(&dir.root, &["add", "."]);
    git(&dir.root, &["commit", "-qm", "invalid source"]);
    assert_eq!(first.stdout, run(&dir.root, &request).stdout);
    let mut changed = request;
    changed["revision"]["value"] = json!(git(&dir.root, &["rev-parse", "HEAD"]));
    let failed = run(&dir.root, &changed);
    assert_eq!(failed.status.code(), Some(1));
    let failed: Value = serde_json::from_slice(&failed.stdout).unwrap();
    assert_eq!(failed["validation_receipt"]["result"], "fail");
    assert_eq!(
        failed["validation_receipt"]["inputs"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
#[test]
fn migration_prepare_refuses_nonexact_and_unavailable_revisions() {
    let (dir, request) = repo();
    for revision in [
        json!(null),
        json!({}),
        json!({"system":"git", "value":"HEAD"}),
        json!({"system":"git", "value":"abc1234"}),
        json!({"system":"git", "value":"0".repeat(40)}),
    ] {
        let mut bad = request.clone();
        bad["revision"] = revision;
        let out = run(&dir.root, &bad);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("migration.exact_revision_required"));
        assert!(out.stdout.is_empty());
    }
    let mut missing = request;
    missing["revision"]["value"] = json!("a".repeat(40));
    let out = run(&dir.root, &missing);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.snapshot_unavailable"));
}
#[test]
fn migration_prepare_refuses_checkout_transformations() {
    for attributes in [".gitattributes", ".GITATTRIBUTES"] {
        let (dir, mut request) = repo();
        fs::write(dir.root.join(attributes), "*.adoc filter=untrusted\n").unwrap();
        git(&dir.root, &["add", "."]);
        git(&dir.root, &["commit", "-qm", "attributes"]);
        request["revision"]["value"] = json!(git(&dir.root, &["rev-parse", "HEAD"]));
        let out = run(&dir.root, &request);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("migration.unsafe_source"));
    }
}

#[test]
fn migration_prepare_refuses_tag_object_sha_and_unknown_or_oversized_request() {
    let (dir, request) = repo();
    git(&dir.root, &["tag", "-am", "tag", "release"]);
    let mut bad = request.clone();
    bad["revision"]["value"] = json!(git(&dir.root, &["rev-parse", "release"]));
    let out = run(&dir.root, &bad);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.snapshot_unavailable"));
    for (field, value) in [
        ("schema_version", json!("unknown")),
        ("request_id", json!("a".repeat(17000))),
        ("foreign", json!(true)),
    ] {
        let mut bad = request.clone();
        bad[field] = value;
        let out = run(&dir.root, &bad);
        assert_eq!(out.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&out.stderr).contains("migration.invalid_request"));
        assert!(out.stdout.is_empty());
    }
}

#[cfg(unix)]
#[test]
fn migration_prepare_disables_hooks_and_refuses_symlinks_and_info_attributes() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let (dir, mut request) = repo();
    let hook = dir.root.join(".git/hooks/post-checkout");
    let marker = dir.root.join("executed-hook");
    fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    let out = run(&dir.root, &request);
    assert!(out.status.success());
    assert!(!marker.exists());
    fs::write(dir.root.join(".git/info/attributes"), "*.adoc ident\n").unwrap();
    let out = run(&dir.root, &request);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.unsafe_source"));
    fs::remove_file(dir.root.join(".git/info/attributes")).unwrap();
    symlink("/etc/passwd", dir.root.join("docs/outside.adoc")).unwrap();
    git(&dir.root, &["add", "."]);
    git(&dir.root, &["commit", "-qm", "symlink"]);
    request["revision"]["value"] = json!(git(&dir.root, &["rev-parse", "HEAD"]));
    let out = run(&dir.root, &request);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("migration.unsafe_source"));
    assert!(!marker.exists());
}

#[test]
fn migration_prepare_invalid_utf8_receipt_is_identical_across_snapshots() {
    let (dir, mut request) = repo();
    fs::write(dir.root.join("docs/two.adoc"), [0xff, 0xfe]).unwrap();
    git(&dir.root, &["add", "."]);
    git(&dir.root, &["commit", "-qm", "invalid UTF-8"]);
    request["revision"]["value"] = json!(git(&dir.root, &["rev-parse", "HEAD"]));
    let first = run(&dir.root, &request);
    let second = run(&dir.root, &request);
    assert_eq!(first.status.code(), Some(1));
    assert_eq!(second.status.code(), Some(1));
    assert_eq!(first.stdout, second.stdout);
    let receipt: Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(receipt["validation_receipt"]["result"], "fail");
    let diagnostic = receipt["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|diagnostic| diagnostic["code"] == "io.unreadable_file")
        .unwrap();
    assert!(
        diagnostic["message"]
            .as_str()
            .unwrap()
            .contains("docs/two.adoc")
    );
    assert!(
        !String::from_utf8(first.stdout)
            .unwrap()
            .contains("adoc-worktree-")
    );
}
