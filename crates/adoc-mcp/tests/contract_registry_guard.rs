//! Docs-truth guard (E0.3): `docs/roadmap/v10/CONTRACT-REGISTRY.md` is the
//! single canonical inventory of externally observable wire surfaces — no
//! envelope schema-version id or Diagnostic Code may be emitted by `adoc`
//! source or tests without a registry row, and no shipped registry row may
//! outlive the code that emitted it. The parse targets pinned HTML comment
//! anchors and backticked first table cells, never free prose.

mod support;

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

const REGISTRY: &str = "docs/roadmap/v10/CONTRACT-REGISTRY.md";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_repo_doc(relative: &str) -> String {
    let path = repo_root().join(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

/// The registry document, refusing a tree where a fence never closes —
/// the span from the opener to EOF would be invisible to every check here.
fn registry() -> String {
    let content = read_repo_doc(REGISTRY);
    if let Some(opener) = support::doc_scan::unclosed_fence(&content) {
        panic!("{REGISTRY}:{opener}: fence never closes — every check below is blind past it");
    }
    content
}

/// Backticked ids from the first cell of the table rows between
/// `<!-- {anchor} -->` and `<!-- /{anchor} -->`. Between the anchors only a
/// header row, a separator row, a blank line, or a `| `id` |…` data row is
/// legal — anything else fails loudly rather than silently dropping a row.
fn anchored_ids(doc: &str, anchor: &str) -> BTreeSet<String> {
    let block = support::doc_scan::anchored_block(doc, REGISTRY, anchor);

    let mut ids = BTreeSet::new();
    let mut saw_header = false;
    for line in block.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        let Some(row) = line.strip_prefix('|') else {
            panic!("{REGISTRY}: `{anchor}` block contains a non-table line: {line:?}");
        };
        let first_cell = row.split('|').next().unwrap_or_default().trim();
        // An empty first cell must fall through and fail loudly below —
        // `all()` is vacuously true on "", which would silently drop the row.
        if !first_cell.is_empty() && first_cell.chars().all(|c| c == '-' || c == ':') {
            continue; // separator row
        }
        if let Some(id) = first_cell
            .strip_prefix('`')
            .and_then(|rest| rest.strip_suffix('`'))
        {
            if !ids.insert(id.to_string()) {
                panic!("{REGISTRY}: `{anchor}` registers {id:?} twice");
            }
        } else if saw_header {
            panic!("{REGISTRY}: `{anchor}` data row's first cell is not a backticked id: {line:?}");
        } else {
            saw_header = true; // exactly one unbackticked header row is legal
        }
    }
    ids
}

/// Every id registered anywhere in the registry, across all anchored tables.
fn all_registered_ids(doc: &str) -> BTreeSet<String> {
    ANCHORS
        .iter()
        .flat_map(|anchor| anchored_ids(doc, anchor))
        .collect()
}

/// Every anchored id table in the registry. A new table means a new entry
/// here — `all_anchors_present` fails until the two lists agree.
const ANCHORS: &[&str] = &[
    "registry:envelopes-shipped-adoc",
    "registry:envelopes-shipped-action",
    "registry:envelopes-historical",
    "registry:test-fixture-ids",
    "registry:envelopes-planned",
    "registry:diagnostic-codes",
    "registry:gateway-audit-codes",
    "registry:action-codes",
    "registry:gate-codes",
    "registry:permission-primitives",
    "registry:permission-primitives-v2-additions",
    "registry:group-binding-modes",
    "registry:group-source-kinds",
    "registry:group-membership-unavailability-states",
    "registry:cloud-codes",
    "registry:attestation-codes",
    "registry:dispositions",
    "registry:untrusted-change-states",
    "registry:retention-classes",
    "registry:replay-postures",
    "registry:managed-state-dimensions",
    "registry:proof-obligation-states",
    "registry:proof-obligation-stages",
    "registry:migration-lifecycle-states",
];

/// Tables whose ids are section-scoped bare words, not globally unique ids.
/// Namespaced vocabularies such as `registry:managed-state-dimensions` stay
/// outside this list and remain subject to the disposition collision check.
const VOCABULARY_ANCHORS: &[&str] = &[
    "registry:group-binding-modes",
    "registry:group-source-kinds",
    "registry:group-membership-unavailability-states",
    "registry:untrusted-change-states",
    "registry:retention-classes",
    "registry:replay-postures",
    "registry:proof-obligation-states",
    "registry:proof-obligation-stages",
    "registry:migration-lifecycle-states",
];

/// True for `adoc.<path>.v<digits>` or `agentdoc.<path>.v<digits>` — the envelope
/// schema-version id shape and nothing else.
fn is_envelope_id(candidate: &str) -> bool {
    let Some(rest) = candidate
        .strip_prefix("adoc.")
        .or_else(|| candidate.strip_prefix("agentdoc."))
    else {
        return false;
    };
    let Some((path, version)) = rest.rsplit_once(".v") else {
        return false;
    };
    !path.is_empty()
        && path
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_' || c == '.')
        && !version.is_empty()
        && version.chars().all(|c| c.is_ascii_digit())
}

/// Envelope ids appearing in quote-delimited chunks of one source text.
/// `//` comment lines are skipped (mirroring
/// `diagnostic_codes_in`) so an id surviving only in a comment cannot keep
/// a stale shipped row alive. Every quote-delimited chunk is inspected so
/// escaped JSON string values are visible too; trailing escape slashes are
/// removed before the exact-id check. Whole files are scanned,
/// `#[cfg(test)]` modules included: fixture ids there are registered rows
/// (the test-fixture table), so a new unregistered literal fails loudly
/// wherever it sits.
// ponytail: ids built with format!/concat are outside any textual net —
// the workspace convention is whole-literal schema-version consts.
fn envelope_ids_in(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(|line| line.split('"').skip(1))
        .map(|chunk| chunk.trim_end_matches('\\'))
        .filter(|chunk| is_envelope_id(chunk))
        .map(str::to_string)
        .collect()
}

fn text_files_under(dir: &Path, sources: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|error| panic!("failed to list {}: {error}", dir.display()))
            .path();
        if path.is_dir() {
            text_files_under(&path, sources);
        } else {
            sources.push(path);
        }
    }
}

/// One source file split at its `#[cfg(test)] mod` boundary: everything
/// above is production scope, the module and everything after it is test
/// scope. A `#[cfg(test)]` on a single item (a test-only `use` at the file
/// top) does not split. An out-of-line `#[cfg(test)] mod tests;` would put
/// production code below it into the wrong scope, so it fails loudly
/// instead of misclassifying.
// ponytail: tail-scope split, not brace matching — the workspace convention
// keeps inline test modules at the file end; production code after one
// would be counted as test scope (still registered-checked, but outside
// the shipped equality).
fn split_test_scope(source: &str) -> (String, String) {
    let lines: Vec<&str> = source.lines().collect();
    let boundary = lines.iter().enumerate().position(|(i, line)| {
        line.trim_start().starts_with("#[cfg(test)]")
            && lines[i + 1..]
                .iter()
                .find(|next| !next.trim().is_empty())
                .is_some_and(|next| {
                    let next = next.trim();
                    let is_module = next.starts_with("mod ") || next.starts_with("pub mod ");
                    assert!(
                        !(is_module && next.ends_with(';')),
                        "out-of-line #[cfg(test)] mod is unsupported by the scan — \
                         production code after it would be misclassified; use an inline module"
                    );
                    is_module
                })
    });
    let split = boundary.unwrap_or(lines.len());
    (lines[..split].join("\n"), lines[split..].join("\n"))
}

struct EmittedIds {
    production: BTreeSet<String>,
    test_scoped: BTreeSet<String>,
}

/// Every envelope id appearing in `crates/*/{src,tests}`, split by scope:
/// production literals must match the shipped table exactly; test-scoped
/// literals may additionally cite historical rows (back-compat tests) and
/// registered fixture ids, but never an unregistered id.
// ponytail: a build.rs or a nested crate would sit outside this walk; neither
// exists in this workspace.
fn emitted_envelope_ids() -> EmittedIds {
    let crates_dir = repo_root().join("crates");
    let mut sources = Vec::new();
    let mut test_sources = Vec::new();
    let entries = fs::read_dir(&crates_dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", crates_dir.display()));
    for entry in entries {
        let crate_dir = entry
            .unwrap_or_else(|error| panic!("failed to list {}: {error}", crates_dir.display()))
            .path();
        let src = crate_dir.join("src");
        if src.is_dir() {
            text_files_under(&src, &mut sources);
        }
        let tests = crate_dir.join("tests");
        if tests.is_dir() {
            text_files_under(&tests, &mut test_sources);
        }
    }
    assert!(
        !sources.is_empty(),
        "no Rust sources found under crates/*/src — the scan would pass vacuously"
    );
    let mut emitted = EmittedIds {
        production: BTreeSet::new(),
        test_scoped: BTreeSet::new(),
    };
    for path in &sources {
        let content = fs::read_to_string(path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        let (production, test) = split_test_scope(&content);
        emitted.production.extend(envelope_ids_in(&production));
        emitted.test_scoped.extend(envelope_ids_in(&test));
    }

    for path in test_sources {
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => continue,
            Err(error) => panic!("failed to read {}: {error}", path.display()),
        };
        emitted.test_scoped.extend(envelope_ids_in(&content));
    }
    emitted
}

/// `Variant = "wire.string" =>` rows from one source text. Comment lines
/// are stripped first (the macro's doc comment shows a placeholder row),
/// then the text is flattened so a rustfmt-wrapped row still parses.
fn diagnostic_codes_in(content: &str) -> BTreeSet<String> {
    let flattened = content
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ");
    let mut codes = BTreeSet::new();
    let mut rest = flattened.as_str();
    while let Some(start) = rest.find("= \"") {
        rest = &rest[start + 3..];
        let Some(end) = rest.find('"') else { break };
        let code = &rest[..end];
        rest = &rest[end + 1..];
        if rest.trim_start().starts_with("=>") && code.contains('.') {
            codes.insert(code.to_string());
        }
    }
    codes
}

/// Every wire code declared in the `diagnostic_codes!` table — one
/// `Variant = "wire.string" =>` row per code, the single source of truth
/// the macro expands from.
fn diagnostic_code_table() -> BTreeSet<String> {
    let content = read_repo_doc("crates/adoc-core/src/domain/diagnostic.rs");
    let codes = diagnostic_codes_in(&content);
    assert!(
        codes.len() > 100,
        "diagnostic_codes! parse found only {} rows — the row pattern drifted",
        codes.len()
    );
    codes
}

#[test]
fn all_anchors_present() {
    let registry = registry();
    for anchor in ANCHORS {
        anchored_ids(&registry, anchor); // panics on a missing anchor
    }
}

#[test]
fn vocabulary_anchors_are_declared_anchors() {
    for anchor in VOCABULARY_ANCHORS {
        assert!(
            ANCHORS.contains(anchor),
            "{anchor:?} is not a declared registry anchor — a stale or typo'd \
             exclusion would silently weaken the disposition check"
        );
    }
}

#[test]
fn vocabulary_anchors_hold_bare_words() {
    let registry = registry();
    for anchor in VOCABULARY_ANCHORS {
        for id in anchored_ids(&registry, anchor) {
            assert!(
                !id.contains('.'),
                "{anchor}: {id:?} is namespaced, not a section-scoped bare word — \
                 a namespaced table belongs in the disposition scan"
            );
        }
    }
}

#[test]
fn anchors_cover_every_registry_table() {
    let registry = registry();
    let found: BTreeSet<String> = support::doc_scan::structural_lines(&registry)
        .filter_map(|(_, line)| {
            line.trim()
                .strip_prefix("<!-- registry:")
                .and_then(|rest| rest.strip_suffix(" -->"))
                .map(|name| format!("registry:{name}"))
        })
        .collect();
    let declared: BTreeSet<String> = ANCHORS.iter().map(|anchor| anchor.to_string()).collect();
    assert_eq!(
        found, declared,
        "{REGISTRY}: the anchors in the document and the ANCHORS list drifted — \
         a table outside the list would be invisible to every id check"
    );
}

#[test]
fn shipped_adoc_envelope_rows_match_the_source_scan() {
    let registry = registry();
    let registered = anchored_ids(&registry, "registry:envelopes-shipped-adoc");
    let emitted = emitted_envelope_ids();

    let unregistered: Vec<_> = emitted.production.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "envelope ids emitted by production code in crates/*/src without a \
         shipped registry row in {REGISTRY}: {unregistered:?}"
    );
    let stale: Vec<_> = registered.difference(&emitted.production).collect();
    assert!(
        stale.is_empty(),
        "shipped adoc envelope rows in {REGISTRY} no longer appear in any \
         production string literal in crates/*/src: {stale:?}"
    );

    let all = all_registered_ids(&registry);
    let rogue: Vec<_> = emitted.test_scoped.difference(&all).collect();
    assert!(
        rogue.is_empty(),
        "test-scoped envelope ids in crates/*/src or crates/*/tests without any registry row \
         (shipped, historical, or test-fixture) in {REGISTRY}: {rogue:?}"
    );
}

#[test]
fn historical_envelope_rows_are_no_longer_emitted() {
    let registry = registry();
    let historical = anchored_ids(&registry, "registry:envelopes-historical");
    assert!(
        !historical.is_empty(),
        "{REGISTRY}: the historical table lost its rows — adoc.retrieval.v0 \
         is a retained documented version"
    );
    let emitted = emitted_envelope_ids();
    let resurrected: Vec<_> = historical.intersection(&emitted.production).collect();
    assert!(
        resurrected.is_empty(),
        "historical envelope rows are emitted from production code again: \
         {resurrected:?} — a genuinely re-shipped envelope registers a new \
         version, never revives a retired one (back-compat tests belong in \
         a test module, where the historical row already covers them)"
    );
}

#[test]
fn action_owned_envelope_rows_are_pinned() {
    let registry = registry();
    let registered = anchored_ids(&registry, "registry:envelopes-shipped-action");
    let expected: BTreeSet<String> = ["adoc.pr_assessment_receipt.v0", "adoc.semantic_review.v0"]
        .iter()
        .map(|id| id.to_string())
        .collect();
    assert_eq!(
        registered, expected,
        "Action-owned shipped envelopes (ADR-0051/ADR-0052) drifted in {REGISTRY}"
    );
}

#[test]
fn diagnostic_code_rows_match_the_code_table() {
    let registry = registry();
    let registered = anchored_ids(&registry, "registry:diagnostic-codes");
    let declared = diagnostic_code_table();

    let unregistered: Vec<_> = declared.difference(&registered).collect();
    assert!(
        unregistered.is_empty(),
        "Diagnostic Codes declared in diagnostic.rs without a registry row \
         in {REGISTRY}: {unregistered:?}"
    );
    let stale: Vec<_> = registered.difference(&declared).collect();
    assert!(
        stale.is_empty(),
        "Diagnostic Code rows in {REGISTRY} with no declaring code table row: {stale:?}"
    );
}

#[test]
fn gateway_audit_refusal_is_registered() {
    let rendered = adoc_mcp::McpAdapterError::AuditSinkUnavailable.to_string();
    let (code, _) = rendered
        .strip_prefix("error[")
        .unwrap()
        .split_once(']')
        .unwrap();
    assert_eq!(
        anchored_ids(&registry(), "registry:gateway-audit-codes"),
        BTreeSet::from([
            code.to_string(),
            "retrieval.sensitive_access_unrecorded".into(),
            "retrieval.audit_spool_corrupt".into()
        ])
    );
}

#[test]
fn scan_flags_an_unregistered_wire_code_fixture() {
    for rogue in [
        concat!("adoc.", "unregistered.v0"),
        concat!("agentdoc.cloud.", "unregistered.v0"),
    ] {
        let fixture = format!(r#"pub const ROGUE_SCHEMA_VERSION: &str = "{rogue}";"#);
        let emitted = envelope_ids_in(&fixture);
        let registered = all_registered_ids(&registry());
        let unregistered: Vec<_> = emitted.difference(&registered).collect();
        assert_eq!(
            unregistered,
            [rogue],
            "the completeness scan must fail on an unregistered {rogue} fixture"
        );
    }
}

#[test]
fn fixture_ids_are_registered_and_disjoint_from_real_rows() {
    let registry = registry();
    let fixtures = anchored_ids(&registry, "registry:test-fixture-ids");
    assert!(
        fixtures.contains("adoc.search.v99"),
        "the rejected-version fixture id lost its registry row"
    );
    // Scope does the separating now: back-compat tests cite historical rows
    // from test scope with no fixture row at all, so a fixture id colliding
    // with ANY real row would only ever blur the inventory.
    let real: BTreeSet<String> = ANCHORS
        .iter()
        .filter(|anchor| **anchor != "registry:test-fixture-ids")
        .flat_map(|anchor| anchored_ids(&registry, anchor))
        .collect();
    let colliding: Vec<_> = fixtures.intersection(&real).collect();
    assert!(
        colliding.is_empty(),
        "a test-fixture id may never collide with a real contract row: {colliding:?}"
    );
    let leaked: Vec<_> = fixtures
        .intersection(&emitted_envelope_ids().production)
        .cloned()
        .collect();
    assert!(
        leaked.is_empty(),
        "test-fixture ids emitted from production code — the fixture table \
         registers test-scope literals only: {leaked:?}"
    );
}

#[test]
fn e4_4_cloud_operation_contracts_are_registered_exactly() {
    let registered = all_registered_ids(&registry());
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
        assert!(registered.contains(id), "E4.4 contract missing: {id}");
    }
}

#[test]
fn schema_publication_keeps_planned_contract_ownership() {
    let doc = registry();
    for (id, owner, slice) in [
        ("agentdoc.cloud.writeback_record.v0", "cloud", "E6.5"),
        ("adoc.authorization_decision.v0", "adoc", "E2.2"),
        ("agentdoc.cloud.egress_policy.v0", "cloud", "E4.4"),
    ] {
        assert!(anchored_ids(&doc, "registry:envelopes-planned").contains(id));
        for anchor in ANCHORS.iter().copied().filter(|anchor| {
            anchor.starts_with("registry:envelopes-") && *anchor != "registry:envelopes-planned"
        }) {
            assert!(!anchored_ids(&doc, anchor).contains(id));
        }
        let block = support::doc_scan::anchored_block(&doc, REGISTRY, "registry:envelopes-planned");
        let row = block
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("| `{id}` |")))
            .expect("planned contract row");
        assert_eq!(row.split('|').nth(2).map(str::trim), Some(owner), "{id}");
        assert_eq!(row.split('|').nth(3).map(str::trim), Some(slice), "{id}");
    }
}

#[test]
fn egress_policy_codes_are_registered_cloud_codes() {
    let codes = anchored_ids(&registry(), "registry:cloud-codes");
    for code in [
        "egress.policy_unknown_category",
        "egress.policy_unavailable",
        "egress.payload_rejected",
        "egress.category_disabled",
        "egress.policy_gate_conflict",
    ] {
        assert!(
            codes.contains(code),
            "missing Cloud egress policy code: {code}"
        );
    }
}

#[test]
fn connector_capability_manifest_is_shipped_by_its_contract_owner() {
    let doc = registry();
    let shipped = anchored_ids(&doc, "registry:envelopes-shipped-adoc");
    let planned = anchored_ids(&doc, "registry:envelopes-planned");

    assert!(shipped.contains("agentdoc.connector_capabilities.v0"));
    assert!(!planned.contains("agentdoc.connector_capabilities.v0"));
}

#[test]
fn e5_1_proposal_record_is_shipped_by_its_contract_owner() {
    let doc = registry();
    let shipped = anchored_ids(&doc, "registry:envelopes-shipped-adoc");
    let planned = anchored_ids(&doc, "registry:envelopes-planned");
    let codes = anchored_ids(&doc, "registry:diagnostic-codes");

    assert!(shipped.contains("adoc.proposal.v0"));
    assert!(!planned.contains("adoc.proposal.v0"));
    for code in [
        "proposal_record.invalid_document",
        "proposal_record.binding_invalid",
        "proposal_record.patch_invalid",
        "proposal_record.authority_rejected",
        "proposal_record.revision_unchanged",
    ] {
        assert!(codes.contains(code), "E5.1 diagnostic code missing: {code}");
    }
}

#[test]
fn e5_2_cloud_approval_codes_are_registered_exactly() {
    let registry = registry();
    let cloud_block =
        support::doc_scan::anchored_block(&registry, REGISTRY, "registry:cloud-codes");
    let registered: BTreeSet<_> = anchored_ids(&registry, "registry:cloud-codes")
        .into_iter()
        .filter(|code| code.starts_with("approval."))
        .collect();
    let expected = BTreeSet::from([
        "approval.ineligible_approver".to_string(),
        "approval.proposal_hash_mismatch".to_string(),
        "approval.scope_mismatch".to_string(),
        "approval.policy_version_stale".to_string(),
        "approval.invalidated_proposal_changed".to_string(),
        "approval.concurrent_write_rejected".to_string(),
        "approval.model_identity_rejected".to_string(),
    ]);

    assert_eq!(registered, expected);
    let scope_row = cloud_block
        .lines()
        .find(|line| {
            line.trim().split('|').nth(1).map(str::trim) == Some("`approval.scope_mismatch`")
        })
        .expect("approval scope-mismatch row");
    assert!(scope_row.contains("complete affected-object `(object_id, content_hash)` tuple set"));
    assert!(scope_row.contains("`create_object` targets"));
    assert!(scope_row.contains("does not exact-match"));

    let proposal_hash_approval_row = cloud_block
        .lines()
        .find(|line| {
            line.trim().split('|').nth(1).map(str::trim)
                == Some("`approval.proposal_hash_mismatch`")
        })
        .expect("approval proposal-hash-mismatch row");
    assert!(proposal_hash_approval_row.contains("no approval record is created"));
    assert!(proposal_hash_approval_row.contains("distinct from `gate.proposal_hash_mismatch`"));
    assert!(proposal_hash_approval_row.contains("gate-time stored-proposal integrity check"));
    assert!(proposal_hash_approval_row.contains("digest re-derived from delivered content"));
    assert!(proposal_hash_approval_row.contains("never a command rejection"));

    let gate_block = support::doc_scan::anchored_block(&registry, REGISTRY, "registry:gate-codes");
    let proposal_hash_row = gate_block
        .lines()
        .find(|line| {
            line.trim().split('|').nth(1).map(str::trim) == Some("`gate.proposal_hash_mismatch`")
        })
        .expect("gate proposal-hash-mismatch row");
    assert!(proposal_hash_row.contains("declared digest"));
    assert!(proposal_hash_row.contains("re-derived from delivered proposal content"));
    assert!(proposal_hash_row.contains("`gate.approval_invalidated` takes precedence"));
    assert!(proposal_hash_row.contains("this code is not emitted"));
    assert!(!proposal_hash_row.contains("`approval.proposal_hash_mismatch`"));

    let invalidated_row = gate_block
        .lines()
        .find(|line| {
            line.trim().split('|').nth(1).map(str::trim) == Some("`gate.approval_invalidated`")
        })
        .expect("gate approval-invalidated row");
    assert!(invalidated_row.contains("`approval.invalidated_proposal_changed`"));
    assert!(invalidated_row.contains("consumes"));
    assert!(invalidated_row.contains("distinct code, not a respelling"));
}

#[test]
fn e5_3_gate_result_and_closed_reason_set_are_shipped() {
    let doc = registry();
    let shipped = anchored_ids(&doc, "registry:envelopes-shipped-adoc");
    let planned = anchored_ids(&doc, "registry:envelopes-planned");
    assert!(shipped.contains("adoc.gate_result.v0"));
    assert!(!planned.contains("adoc.gate_result.v0"));

    let reasons = anchored_ids(&doc, "registry:gate-codes");
    let gate_result_reasons = BTreeSet::from([
        "gate.approval_invalidated".to_string(),
        "gate.approval_missing".to_string(),
        "gate.assessment_missing".to_string(),
        "gate.assessment_stale".to_string(),
        "gate.audit_persistence_failed".to_string(),
        "gate.cloud_unavailable".to_string(),
        "gate.mode_unknown".to_string(),
        "gate.proposal_hash_mismatch".to_string(),
        "gate.proposal_missing".to_string(),
        "gate.provider_failed_no_fallback".to_string(),
        "gate.promotion_unapproved".to_string(),
        "gate.semantic_invalid".to_string(),
    ]);
    assert!(
        gate_result_reasons.is_subset(&reasons),
        "E5.3 gate-result reasons are missing: {:?}",
        gate_result_reasons.difference(&reasons).collect::<Vec<_>>()
    );
    for reason in &gate_result_reasons {
        let row = doc
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("| `{reason}` |")))
            .expect("gate reason row");
        assert!(row.contains("| shipped | E5.3 |"), "unshipped row: {row}");
    }
    let shipped_e5_3: BTreeSet<_> = reasons
        .iter()
        .filter(|reason| {
            doc.lines()
                .find(|line| line.trim_start().starts_with(&format!("| `{reason}` |")))
                .is_some_and(|row| row.contains("| shipped | E5.3 |"))
        })
        .cloned()
        .collect();
    assert_eq!(shipped_e5_3, gate_result_reasons);
    assert!(reasons.contains("gate.check_publish_failed"));
}

#[test]
fn e5_cloud_prerelease_contracts_are_registered_exactly() {
    let doc = registry();
    let planned = anchored_ids(&doc, "registry:envelopes-planned");
    let planned_block =
        support::doc_scan::anchored_block(&doc, REGISTRY, "registry:envelopes-planned");
    let expected_envelopes = BTreeSet::from([
        "agentdoc.cloud.activation_gate_evidence.v0",
        "agentdoc.cloud.approval_gate_fact.v0",
        "agentdoc.cloud.approved_proposal_promotion.v0",
        "agentdoc.cloud.gate_emergency_fact.v0",
        "agentdoc.cloud.gate_emergency_receipt.v0",
        "agentdoc.cloud.gate_evidence_set.v0",
        "agentdoc.cloud.github_check_publication_failure.v0",
        "agentdoc.cloud.github_negative_verdict_acceptance.v0",
        "agentdoc.cloud.internal_integrated_trace.v0",
        "agentdoc.cloud.promotion_gate_fact.v0",
        "agentdoc.cloud.proposal_coverage.v0",
        "agentdoc.cloud.proposal_disposition_acceptance.v0",
        "agentdoc.cloud.terminal_gate_evidence.v0",
    ]);
    for &id in &expected_envelopes {
        assert!(
            planned.contains(id),
            "E5 Cloud pre-release envelope missing: {id}"
        );
        let row = planned_block
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("| `{id}` |")))
            .expect("planned Cloud envelope row");
        assert_eq!(
            row.split('|').nth(2).map(str::trim),
            Some("cloud"),
            "E5 Cloud pre-release envelope has wrong owner: {row}"
        );
    }
    let actual_envelopes: BTreeSet<_> = planned
        .iter()
        .map(String::as_str)
        .filter(|id| {
            id.starts_with("agentdoc.cloud.")
                && planned_block
                    .lines()
                    .find(|line| line.trim_start().starts_with(&format!("| `{id}` |")))
                    .is_some_and(|row| {
                        row.split('|').nth(2).map(str::trim) == Some("cloud")
                            && row
                                .split('|')
                                .nth(3)
                                .is_some_and(|slice| slice.trim().starts_with("E5"))
                    })
        })
        .collect();
    assert_eq!(actual_envelopes, expected_envelopes);

    let cloud_codes = anchored_ids(&doc, "registry:cloud-codes");
    let gate_codes = anchored_ids(&doc, "registry:gate-codes");
    let expected_codes = BTreeSet::from([
        "gate.check_decision_head_mismatch",
        "gate.check_publish_context_invalid",
        "gate.check_publish_failure_record_failed",
        "gate.check_publish_prepare_failed",
        "gate.check_publish_receipt_failed",
        "gate.check_render_failed",
        "gate.negative_verdict_unrenderable",
        "governance.approved_promotion_invalid",
        "governance.promotion_not_authorized",
        "governance.trace_incomplete",
    ]);
    for &code in &expected_codes {
        assert!(
            cloud_codes.contains(code),
            "E5 Cloud pre-release code missing: {code}"
        );
        assert!(
            !gate_codes.contains(code),
            "Cloud code must not enter the gate-result reason set: {code}"
        );
    }
    let cloud_block = support::doc_scan::anchored_block(&doc, REGISTRY, "registry:cloud-codes");
    let actual_codes: BTreeSet<_> = cloud_codes
        .iter()
        .map(String::as_str)
        .filter(|code| {
            cloud_block
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("| `{code}` |")))
                .is_some_and(|row| row.contains("implemented in Cloud v0.1.0 pre-release"))
        })
        .collect();
    assert_eq!(actual_codes, expected_codes);
}

#[test]
fn e4_5_cloud_capability_trust_codes_are_registered_exactly() {
    let registered = anchored_ids(&registry(), "registry:cloud-codes");
    for code in [
        "connector.manifest_invalid",
        "connector.publisher_unqualified",
        "connector.configuration_ineligible",
        "connector.capability_ineligible",
        "connector.exception_invalid",
    ] {
        assert!(registered.contains(code), "E4.5 Cloud code missing: {code}");
    }
}

#[test]
fn e4_6_ingestion_codes_are_registered_exactly() {
    let registered = anchored_ids(&registry(), "registry:cloud-codes");
    for code in [
        "ingest.duplicate_delivery",
        "ingest.stale_run",
        "ingest.digest_mismatch",
        "connect.permission_exceeds_manifest",
    ] {
        assert!(registered.contains(code), "E4.6 Cloud code missing: {code}");
    }
}

#[test]
fn planned_delivery_and_connect_codes_stay_with_their_owner() {
    let registry = registry();
    let action = anchored_ids(&registry, "registry:action-codes");
    let cloud = anchored_ids(&registry, "registry:cloud-codes");

    let action_owned = "delivery.fork_branch_read_only";
    assert!(
        action.contains(action_owned),
        "E3.8 Action code missing: {action_owned}"
    );
    assert!(
        !cloud.contains(action_owned),
        "E3.8 Action code owned by Cloud: {action_owned}"
    );
    for (slice, code) in [
        ("E8.2", "delivery.reference_missing"),
        ("E8.2", "delivery.reference_stale"),
        ("E7.3", "connect.unknown_config_field"),
        ("E7.3", "connect.credential_store_violation"),
    ] {
        assert!(cloud.contains(code), "{slice} Cloud code missing: {code}");
        assert!(
            !action.contains(code),
            "{slice} Cloud code owned by Action: {code}"
        );
    }
}

#[test]
fn test_only_use_does_not_split_the_scope() {
    let fixture = r#"
        #[cfg(test)]
        use std::fmt;

        pub const REAL: &str = "adoc.diff.v0";
    "#;
    let (production, _) = split_test_scope(fixture);
    assert!(
        envelope_ids_in(&production).contains("adoc.diff.v0"),
        "a test-only use at the file top must not push real emitters into test scope"
    );
}

#[test]
fn tail_test_module_is_test_scope() {
    let fixture = r#"
        pub const REAL: &str = "adoc.diff.v0";

        #[cfg(test)]
        mod tests {
            const REJECTED_VERSION_FIXTURE: &str = "adoc.search.v99";
        }
    "#;
    let (production, test) = split_test_scope(fixture);
    assert!(envelope_ids_in(&production).contains("adoc.diff.v0"));
    assert!(
        !envelope_ids_in(&production).contains("adoc.search.v99"),
        "rejected-version fixtures live in test scope"
    );
    assert!(envelope_ids_in(&test).contains("adoc.search.v99"));
}

#[test]
fn out_of_line_test_module_fails_loudly() {
    let fixture = "#[cfg(test)]\nmod tests;\npub const LATE: &str = \"adoc.diff.v0\";";
    let result = std::panic::catch_unwind(|| split_test_scope(fixture));
    assert!(
        result.is_err(),
        "an out-of-line test module would misclassify the code after it — must fail loudly"
    );
}

#[test]
fn quote_desync_stays_bounded_to_its_line() {
    let fixture = r#"
        const QUOTE: char = '"';
        pub const REAL: &str = "adoc.diff.v0";
    "#;
    assert!(
        envelope_ids_in(fixture).contains("adoc.diff.v0"),
        "a char-literal quote on an earlier line must not hide later ids"
    );
}

#[test]
fn escaped_json_fixture_ids_are_scanned() {
    let fixture = r#"let json = "{\"schema_version\":\"adoc.graph.v2\"}";"#;
    assert!(
        envelope_ids_in(fixture).contains("adoc.graph.v2"),
        "an envelope id inside escaped JSON must not bypass the registry guard"
    );
}

#[test]
fn textual_test_fixture_ids_are_scanned() {
    let mut files = Vec::new();
    text_files_under(
        &repo_root().join("crates/adoc-cli/tests/fixtures/v1_1_why"),
        &mut files,
    );
    let fixture = files
        .iter()
        .find(|path| path.ends_with("unsupported_version.graph.json"))
        .expect("JSON inputs consumed by integration tests join the file walk");
    let content = fs::read_to_string(fixture).expect("fixture is UTF-8");
    assert!(envelope_ids_in(&content).contains("adoc.graph.v99"));
}

#[test]
fn wrapped_diagnostic_row_still_parses() {
    let wrapped = "SomeCode =\n    \"assessment.wrapped_row\" =>\n    \"help\";";
    assert!(
        diagnostic_codes_in(wrapped).contains("assessment.wrapped_row"),
        "a rustfmt-wrapped diagnostic_codes! row must still scan"
    );
    let commented = r#"/// carrying `(Variant = "wire.string" => "default help";)`"#;
    assert!(
        diagnostic_codes_in(commented).is_empty(),
        "the macro doc-comment placeholder must never scan as a code"
    );
}

#[test]
fn envelope_id_shape_rejects_prose_and_paths() {
    for not_an_id in [
        "adoc.graph",          // no version suffix
        "adoc.graph.v5.json",  // file name, version not terminal
        "adoc.Graph.v5",       // uppercase
        "docs/adoc.graph.v5",  // path prefix
        "prose adoc.graph.v5", // embedded in prose
        "adoc..v5",            // empty path
        "adoc.graph.v",        // empty version
    ] {
        assert!(
            !is_envelope_id(not_an_id),
            "{not_an_id:?} must not scan as an envelope id"
        );
    }
    assert!(is_envelope_id("adoc.graph.v5"));
    assert!(is_envelope_id("adoc.mcp.command.v0"));
}

#[test]
fn guard_fires_on_a_malformed_registry_row() {
    let broken = "\
<!-- registry:broken -->\n\
| id | status |\n\
| --- | --- |\n\
| adoc.graph.v5 | shipped |\n\
<!-- /registry:broken -->\n";
    let result = std::panic::catch_unwind(|| anchored_ids(broken, "registry:broken"));
    assert!(
        result.is_err(),
        "a data row whose first cell is not backticked must fail loudly, never drop silently"
    );
}

#[test]
fn guard_fires_on_a_duplicated_anchor_block() {
    let duplicated = "\
<!-- registry:dup -->\n\
| id | status |\n\
| --- | --- |\n\
| `adoc.graph.v5` | shipped |\n\
<!-- /registry:dup -->\n\
<!-- registry:dup -->\n\
| `adoc.diff.v0` | shipped |\n\
<!-- /registry:dup -->\n";
    let result = std::panic::catch_unwind(|| anchored_ids(duplicated, "registry:dup"));
    assert!(
        result.is_err(),
        "a duplicated anchor block must fail loudly — later rows are invisible to the scan"
    );
}

#[test]
fn guard_fires_on_an_empty_first_cell() {
    let broken = "\
<!-- registry:empty -->\n\
| id | status |\n\
| --- | --- |\n\
|  | shipped |\n\
<!-- /registry:empty -->\n";
    let result = std::panic::catch_unwind(|| anchored_ids(broken, "registry:empty"));
    assert!(
        result.is_err(),
        "a data row with an empty first cell must fail loudly, never scan as a separator"
    );
}

#[test]
fn comment_lines_do_not_scan_as_emissions() {
    let commented = r#"
        // wire id: "adoc.diff.v0"
    "#;
    assert!(
        envelope_ids_in(commented).is_empty(),
        "an id surviving only in a line comment must not keep a stale shipped row alive"
    );
}

/// The execution map's E0.3 must-include list, resolved to registered ids.
/// One entry per item in the map's sentence, in its order; the milestone's
/// T2 planned set adds the remaining reserved names.
const MUST_INCLUDE_IDS: &[&str] = &[
    "adoc.semantic_context.v0",           // semantic context
    "adoc.semantic_assessment.v0",        // semantic assessment
    "adoc.validation_receipt.v0",         // validation receipt
    "adoc.lifecycle_mapping.v0",          // lifecycle mapping
    "adoc.source_record.v0",              // source record
    "adoc.source_assertion.v0",           // source assertion
    "adoc.source_acl_snapshot.v0",        // ACL snapshot
    "adoc.source_binding.v0",             // source binding
    "adoc.sensitive_access.v0",           // sensitive-access event (RT-08, RT-21)
    "adoc.egress_policy.v0",              // egress policy (RT-21)
    "adoc.authorization_decision.v0",     // authorization decision
    "adoc.work_request.v0",               // work request
    "adoc.work_result.v0",                // work result
    "adoc.migration_request.v0",          // migration request
    "adoc.migration_receipt.v0",          // migration receipt
    "agentdoc.connector_capabilities.v0", // connector capability manifest
    "adoc.governance_event.v0",           // governance contract
    "adoc.proposal.v0",                   // proposal contract
    "adoc.approval.v0",                   // approval contract
    "adoc.gate_result.v0",                // gate contract
];

#[test]
fn must_include_contracts_are_registered() {
    let registered = all_registered_ids(&registry());
    let missing: Vec<_> = MUST_INCLUDE_IDS
        .iter()
        .filter(|id| !registered.contains(**id))
        .collect();
    assert!(
        missing.is_empty(),
        "EXECUTION-MAP §E0.3 must-include items without a registry row: {missing:?}"
    );
}

#[test]
fn planned_rows_name_exactly_one_owner_repo() {
    let registry = registry();
    let block =
        support::doc_scan::anchored_block(&registry, REGISTRY, "registry:envelopes-planned");
    let mut data_rows = 0;
    for line in block.lines().map(str::trim) {
        if !line.starts_with("| `") {
            continue; // header/separator; malformed rows already fail anchored_ids
        }
        data_rows += 1;
        let owner = line.split('|').nth(2).map(str::trim).unwrap_or_default();
        assert!(
            matches!(owner, "adoc" | "action" | "cloud"),
            "planned row must name exactly one owner repo (adoc|action|cloud): {line:?}"
        );
    }
    assert!(data_rows > 0, "the planned table lost its rows");
}

#[test]
fn candidate_authority_contract_binds_the_authenticated_submitting_principal() {
    let registry = registry();
    let block =
        support::doc_scan::anchored_block(&registry, REGISTRY, "registry:envelopes-planned");
    let row = block
        .lines()
        .find(|line| line.contains("`agentdoc.cloud.candidate_authority.v0`"))
        .expect("candidate authority contract row");
    assert!(row.contains("authenticated submitting principal"));
    assert!(row.contains("exact managed candidate version ID and content digest"));
    assert!(row.contains("strictly later candidate-operation ordinal"));
    assert!(row.contains("server-derived from authenticated ingress"));
    assert!(row.contains("never caller-supplied"));
    assert!(row.contains("connector-origin candidates require"));
    assert!(row.contains("AgentDoc-origin candidates carry no connector Source Assertion"));
    assert!(row.contains("current allow authorization decision"));
    assert!(row.contains("`knowledge.propose` permission"));
    assert!(row.contains("evaluated scope exact-matches the submission"));
    assert!(row.contains("authorization state effective for the candidate-operation transaction"));

    let conflict = registry
        .lines()
        .find(|line| line.contains("`authority.candidate_origin_conflict`"))
        .expect("candidate replay-conflict code row");
    assert!(conflict.contains("submitting principal"));
    assert!(conflict.contains("authorization decision"));
    assert!(conflict.contains("managed candidate version ID and content digest"));
    assert!(conflict.contains("policy receipt, effect ordinal, and effect transaction ID"));
    assert!(conflict.contains("candidate-operation ordinal and transaction ID"));
    assert!(conflict.contains("authority mode"));

    let attestation = block
        .lines()
        .find(|line| line.contains("`agentdoc.cloud.external_promotion_attestation.v0`"))
        .expect("external promotion attestation contract row");
    assert!(attestation.contains("trusted issuer evidence"));
    assert!(attestation.contains("exact managed candidate version ID and content digest"));
    assert!(attestation.contains("exact effective policy receipt digest"));

    let receipt = block
        .lines()
        .find(|line| line.contains("`agentdoc.cloud.external_promotion_receipt.v0`"))
        .expect("external promotion receipt contract row");
    assert!(receipt.contains("exact attestation digest and Source Assertion"));
    assert!(receipt.contains("exact managed candidate version ID and content digest"));
    assert!(receipt.contains("exact Governance Event sequence and digest"));
    assert!(receipt.contains("exact-matches the attested policy receipt digest"));
    assert!(receipt.contains("policy effect ordinal and transaction ID"));
    assert!(receipt.contains("strictly later promotion-operation ordinal and transaction ID"));
    assert!(receipt.contains("prior committed transaction"));
}

#[test]
fn connector_authority_policy_receipt_orders_effect_and_exact_matches_authorization() {
    let registry = registry();
    let block =
        support::doc_scan::anchored_block(&registry, REGISTRY, "registry:envelopes-planned");
    let row = block
        .lines()
        .find(|line| line.contains("`agentdoc.cloud.connector_authority_policy_receipt.v0`"))
        .expect("connector authority policy receipt contract row");

    assert!(row.contains("prior effective policy ID/version"));
    assert!(row.contains("policy effect ordinal"));
    assert!(row.contains("policy effect transaction ID"));
    assert!(row.contains("strictly later governed-operation ordinal"));
    assert!(row.contains("prior committed transaction"));
    assert!(row.contains("distinct governed transaction ID"));
    assert!(row.contains("policy effective for its exact scope at that operation ordinal"));
    assert!(row.contains("`connector.configure` permission"));
    assert!(row.contains("exact-match the change"));
    assert!(row.contains("authorization state effective for the policy-change transaction"));
}

#[test]
fn native_permission_additions_preserve_frozen_v0() {
    let doc = registry();
    let original = anchored_ids(&doc, "registry:permission-primitives");
    let added = anchored_ids(&doc, "registry:permission-primitives-v2-additions");
    assert_eq!(added, BTreeSet::from(["source.writeback".to_owned()]));
    assert!(original.is_disjoint(&added));
    assert_eq!(original.union(&added).count(), 31);
    let row = doc
        .lines()
        .find(|line| line.starts_with("| `source.writeback` |"))
        .unwrap();
    assert!(row.contains("| 2 |"));
    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json",
    ))
    .unwrap();
    assert!(
        !schema["$defs"]["permission"]["enum"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("source.writeback"))
    );
}

#[test]
fn permission_primitives_match_the_e2_2_registry() {
    let actual = anchored_ids(&registry(), "registry:permission-primitives");
    let expected = [
        "audit.export",
        "audit.read",
        "connector.configure",
        "connector.create",
        "connector.delete",
        "connector.read",
        "knowledge.declassify",
        "knowledge.propose",
        "knowledge.read",
        "migration.approve",
        "migration.execute",
        "obligation.read",
        "obligation.satisfy",
        "obligation.waive",
        "policy.manage",
        "policy.read",
        "proposal.approve",
        "proposal.edit",
        "proposal.read",
        "proposal.reject",
        "proposal.review",
        "semantic_executor.configure",
        "semantic_executor.qualify",
        "semantic_executor.read",
        "source.manage",
        "source.read",
        "source.sync",
        "workspace.configure",
        "workspace.manage_members",
        "workspace.read",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    assert_eq!(actual, expected);

    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json",
    ))
    .expect("authorization decision schema is json");
    let schema_permissions = schema["$defs"]["permission"]["enum"]
        .as_array()
        .expect("authorization decision permission is an enum")
        .iter()
        .map(|permission| {
            permission
                .as_str()
                .expect("permission enum values are strings")
                .to_owned()
        })
        .collect();
    assert_eq!(actual, schema_permissions);
}

#[test]
fn membership_unavailability_precedence_classifies_every_reason() {
    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json",
    ))
    .expect("authorization decision schema is json");
    let precedence =
        schema["allOf"]
            .as_array()
            .expect("authorization decision allOf")
            .iter()
            .find(|branch| {
                branch["if"]["properties"]["membership_evidence"]["const"] == "insufficient_context"
                    && branch["if"]["properties"]["reason"]["not"]["enum"].is_array()
            })
            .expect("unavailable membership reason-precedence branch")["if"]["properties"]["reason"]
            ["not"]["enum"]
            .as_array()
            .expect("precedence reasons are an enum")
            .iter()
            .map(|reason| reason.as_str().expect("reason is a string").to_owned())
            .collect::<BTreeSet<_>>();
    let downstream = [
        "membership_evidence_unavailable",
        "no_grant",
        "visibility_denied",
        "visibility_unavailable",
        "action_policy_denied",
        "action_policy_unavailable",
        "allowed",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect::<BTreeSet<_>>();
    let reasons = schema["properties"]["reason"]["enum"]
        .as_array()
        .expect("decision reasons are an enum")
        .iter()
        .map(|reason| reason.as_str().expect("reason is a string").to_owned())
        .collect::<BTreeSet<_>>();

    assert!(
        precedence.is_disjoint(&downstream),
        "a reason cannot sit on both sides of the scoped-grants stage"
    );
    assert_eq!(
        precedence
            .union(&downstream)
            .cloned()
            .collect::<BTreeSet<_>>(),
        reasons,
        "every reason must be classified relative to the scoped-grants stage"
    );
}

#[test]
fn group_vocabularies_match_the_e2_4_registry() {
    let registry = registry();
    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json",
    ))
    .expect("authorization decision schema is json");
    let external_group = &schema["$defs"]["group"]["oneOf"]
        .as_array()
        .expect("group is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "external")
        .expect("group has an external membership branch")["properties"];
    let external_absence = &schema["$defs"]["membershipAbsenceEvidence"]["oneOf"]
        .as_array()
        .expect("membership absence evidence is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "external")
        .expect("membership absence evidence has an external branch")["properties"];
    let schema_values = |name: &str| {
        schema["$defs"][name]["enum"]
            .as_array()
            .unwrap_or_else(|| panic!("shared {name} is an enum"))
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .unwrap_or_else(|| panic!("external group {name} values are strings"))
                    .to_owned()
            })
            .collect::<BTreeSet<_>>()
    };

    let registered_modes = anchored_ids(&registry, "registry:group-binding-modes");
    let authorization = read_repo_doc("docs/roadmap/v10/AUTHORIZATION.md");
    let a7 = annex_section(&authorization, "AUTHORIZATION.md", "## A7.");
    let mode_lists = fenced_lists_in(
        a7.split_once("Membership binding modes:")
            .expect("AUTHORIZATION.md §A7 labels its membership binding modes")
            .1,
    );
    assert_eq!(
        mode_lists.len(),
        1,
        "AUTHORIZATION.md §A7 must carry exactly one fenced list after its \
         `Membership binding modes:` lead-in, found {}",
        mode_lists.len()
    );
    let mode_list = &mode_lists[0];
    let authority_modes: BTreeSet<_> = mode_list.iter().cloned().collect();
    assert_eq!(
        authority_modes.len(),
        mode_list.len(),
        "binding modes repeat"
    );
    assert!(!authority_modes.is_empty(), "binding modes are empty");
    assert_eq!(
        registered_modes, authority_modes,
        "every §A7 binding mode needs a `registry:group-binding-modes` row, and vice versa"
    );

    let mode_rows =
        support::doc_scan::anchored_block(&registry, REGISTRY, "registry:group-binding-modes");
    let header = mode_rows
        .lines()
        .map(str::trim)
        .find(|line| line.starts_with('|'))
        .expect("binding modes have a table header");
    let header = header
        .trim_matches('|')
        .split('|')
        .map(str::trim)
        .collect::<Vec<_>>();
    let confers_grant = header
        .iter()
        .position(|cell| *cell == "confers grant")
        .expect("binding-mode table has a confers grant column");
    let mut grant_conferring_modes = BTreeSet::new();
    for line in mode_rows.lines().map(str::trim) {
        if !line.starts_with("| `") {
            continue;
        }
        let cells = line
            .trim_matches('|')
            .split('|')
            .map(str::trim)
            .collect::<Vec<_>>();
        let mode = cells[0]
            .strip_prefix('`')
            .and_then(|value| value.strip_suffix('`'))
            .expect("binding mode is backticked");
        match cells.get(confers_grant).copied() {
            Some("yes") => {
                grant_conferring_modes.insert(mode.to_owned());
            }
            Some("no") => {}
            value => panic!("binding mode {mode:?} has invalid confers-grant value {value:?}"),
        }
    }
    for properties in [external_group, external_absence] {
        assert_eq!(properties["binding_mode"]["$ref"], "#/$defs/bindingMode");
        assert_eq!(properties["source_kind"]["$ref"], "#/$defs/sourceKind");
    }
    assert_eq!(grant_conferring_modes, schema_values("bindingMode"));
    assert_eq!(registered_modes, schema_values("bindingModeAtEvaluation"));
    assert_eq!(
        anchored_ids(&registry, "registry:group-source-kinds"),
        schema_values("sourceKind")
    );
    assert_eq!(
        anchored_ids(&registry, "registry:group-membership-unavailability-states"),
        schema_values("membershipUnavailabilityState")
    );
}

#[test]
fn group_retention_rules_match_the_e2_4_authority() {
    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/adoc.authorization_decision.v0.schema.json",
    ))
    .expect("authorization decision schema is json");
    let external_group = schema["$defs"]["group"]["oneOf"]
        .as_array()
        .expect("group is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "external")
        .expect("group has an external membership branch");
    let external_group_properties = &external_group["properties"];
    let manual_group = schema["$defs"]["group"]["oneOf"]
        .as_array()
        .expect("group is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "manual")
        .expect("group has a manual membership branch");
    let authorization = read_repo_doc("docs/roadmap/v10/AUTHORIZATION.md");
    let a7 = annex_section(&authorization, "AUTHORIZATION.md", "## A7.");
    let effective_history_claim = "complete effective mode history";
    assert!(
        a7.contains(effective_history_claim),
        "AUTHORIZATION.md §A7 must retain only effective binding mode epochs"
    );
    assert!(
        external_group_properties["binding_id"]["description"]
            .as_str()
            .expect("external binding replay is documented")
            .contains(effective_history_claim),
        "binding_id must point replay at the effective history §A7 defines"
    );
    assert!(
        a7.contains("superseded by any later reconfiguration"),
        "AUTHORIZATION.md §A7 must stop superseded sweeps from creating epochs"
    );

    let observation = &schema["$defs"]["membershipObservation"]["properties"];
    let observation_required = schema["$defs"]["membershipObservation"]["required"]
        .as_array()
        .expect("membership observation required fields");
    assert!(
        observation_required
            .iter()
            .any(|field| field == "fresh_until"),
        "external membership observations must carry a freshness deadline"
    );
    let fresh_until = observation["fresh_until"]["description"]
        .as_str()
        .expect("membership freshness deadline is documented");
    assert!(
        fresh_until.contains("versioned freshness policy retained by the exact binding")
            && fresh_until
                .contains("requires fresh_until to equal the historical-version recomputation")
            && fresh_until.contains("shorter of retained fresh_until")
            && fresh_until
                .contains("shortened policy therefore caps existing observations immediately")
            && fresh_until.contains("connector unavailability cannot extend")
            && fresh_until.contains("uncapped observation")
            && fresh_until.contains("born inert")
            && fresh_until.contains("observation_expired"),
        "membership freshness must be binding-owned, tightening-aware, bounded, and fail closed"
    );
    let freshness_history_claim = "complete effective membership-freshness policy history";
    let scheduled_refresh_claim =
        "completes each run before the deadline of the observations it replaces";
    let distinct_policy_claim =
        "membership-freshness policy is distinct from the connector source-ACL policy";
    let failed_reenable_claim = "failed re-enable remains non-granting";
    let membership_unavailable_claim = "`no_grant` remains reserved for a confirmed absence";
    let group_name_history_claim =
        "complete effective group-name history, each version recording its effective instant";
    let membership_absence_claim =
        "`no_grant` with `current` retains `membership_absence_evidence`";
    let membership_unavailability_claim =
        "`insufficient_context` status retains `membership_unavailability_evidence`";
    let unavailability = &schema["properties"]["membership_unavailability_evidence"];
    assert_eq!(unavailability["minItems"], 1);
    assert_eq!(
        unavailability["items"]["$ref"],
        "#/$defs/membershipUnavailabilityEvidence"
    );
    let unavailable_external = schema["$defs"]["membershipUnavailabilityEvidence"]["oneOf"]
        .as_array()
        .expect("membership unavailability evidence is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "external")
        .expect("membership unavailability evidence has an external branch");
    let unavailable_manual = schema["$defs"]["membershipUnavailabilityEvidence"]["oneOf"]
        .as_array()
        .expect("membership unavailability evidence is a oneOf")
        .iter()
        .find(|branch| branch["properties"]["membership_source"]["const"] == "manual")
        .expect("membership unavailability evidence has a manual branch");
    assert_eq!(
        unavailable_external["properties"]["binding_mode"]["$ref"],
        "#/$defs/bindingModeAtEvaluation"
    );
    assert!(unavailable_manual["properties"]["group_name"].is_object());
    for field in [
        "group_id",
        "binding_id",
        "binding_mode",
        "binding_mode_effective_at",
        "source_kind",
        "external_identity_link_id",
        "state",
        "state_record_id",
    ] {
        assert!(
            unavailable_external["required"]
                .as_array()
                .expect("external unavailability required fields")
                .iter()
                .any(|required| required == field),
            "external unavailability must retain {field}"
        );
    }
    let effective_at = observation["effective_at"]["description"]
        .as_str()
        .expect("membership effective time is documented");
    assert!(
        effective_at.contains("must equal")
            && effective_at.contains("identified by source_event_id"),
        "membership effective_at must belong to its cited event or run"
    );
    assert!(
        observation["source_event_id"]["description"]
            .as_str()
            .expect("membership source event is documented")
            .contains("current-state read and ingestion-commit instants"),
        "source_event_id must resolve to retained event endpoints"
    );
    let link_lifecycle_claim = "continuously active from";
    let link_endpoint_claim = "link and unlink instants";
    let link_description = observation["external_identity_link_id"]["description"]
        .as_str()
        .expect("external identity link replay is documented");
    assert!(
        link_description.contains(link_lifecycle_claim)
            && link_description.contains(link_endpoint_claim),
        "membership observations must retain and verify the link interval"
    );
    let link_principal_claim = "link belongs to the authorization envelope principal";
    assert!(
        link_description.contains(link_principal_claim) && a7.contains(link_principal_claim),
        "membership observations must reject another principal's identity link"
    );
    let observation_binding_claim = "observation's retained binding";
    let observation_group_claim = "retained binding's AgentDoc group";
    let observation_link_claim =
        "observation's retained identity link equals the enclosing identity-link identifier";
    assert!(
        observation["id"]["description"]
            .as_str()
            .expect("membership observation replay binding is documented")
            .contains(observation_binding_claim)
            && observation["id"]["description"]
                .as_str()
                .expect("membership observation replay group is documented")
                .contains(observation_group_claim)
            && a7.contains(observation_binding_claim)
            && a7.contains(observation_group_claim),
        "membership replay must reject observation substitution across bindings or groups"
    );
    assert!(
        observation["id"]["description"]
            .as_str()
            .expect("membership observation identity-link binding is documented")
            .contains(observation_link_claim)
            && a7.contains(observation_link_claim),
        "membership replay must reject observation substitution across principals"
    );
    let source_kind_binding_claim = "source kind recorded on the retained binding";
    assert!(
        schema["$defs"]["sourceKind"]["description"]
            .as_str()
            .expect("external source-kind replay is documented")
            .contains(source_kind_binding_claim)
            && a7.contains(source_kind_binding_claim),
        "membership replay must reject source-kind substitution"
    );
    let manual_membership_binding_claim = "authorization envelope principal and enclosing group id";
    assert!(
        manual_group["properties"]["membership_id"]["description"]
            .as_str()
            .expect("manual membership replay binding is documented")
            .contains(manual_membership_binding_claim)
            && a7.contains(manual_membership_binding_claim),
        "manual membership replay must reject principal or group substitution"
    );

    let same_source_claim = "A connector membership observation resolves against exactly one retained source event or synchronization run";
    assert!(
        a7.contains(same_source_claim),
        "AUTHORIZATION.md §A7 must bind both observation times to one source record"
    );
    let source_record_binding_claim = "source record belongs to the observation's retained binding";
    let source_subject_claim =
        "membership subject resolves through the observation's retained external identity link";
    let source_event_description = observation["source_event_id"]["description"]
        .as_str()
        .expect("membership source-record replay binding is documented");
    assert!(
        source_event_description.contains(source_record_binding_claim)
            && source_event_description.contains(source_subject_claim)
            && a7.contains(source_record_binding_claim)
            && a7.contains(source_subject_claim),
        "membership replay must bind source evidence to the binding and observed principal"
    );
    let event_polarity_claim = "polarity only triggers";
    let positive_absence_claim =
        "positive event whose read confirms absence records a negative observation";
    assert!(
        observation["observed_at"]["description"]
            .as_str()
            .expect("membership observation time is documented")
            .contains(event_polarity_claim)
            && observation["observed_at"]["description"]
                .as_str()
                .expect("membership observation polarity is documented")
                .contains(positive_absence_claim)
            && a7.contains(event_polarity_claim)
            && a7.contains(positive_absence_claim),
        "current-state reads, not event polarity, must determine membership observations"
    );
    let removal_membership_claim =
        "removal event whose read still shows membership records a positive observation";
    let removal_cap_schema_claim =
        "fresh_until is capped at the deadline of the latest prior positive observation";
    let removal_cap_doc_claim =
        "`fresh_until` is capped at the deadline of the latest prior positive observation";
    let no_prior_cap_claim = "no prior positive observation exists";
    let born_inert_claim = "born inert";
    let contradiction_claim = "contradiction is retained and operator-visible";
    let resync_bound_claim = "next scheduled resynchronization bounds revocation propagation";
    assert!(
        observation["observed_at"]["description"]
            .as_str()
            .expect("membership observation time is documented")
            .contains(removal_membership_claim)
            && observation["observed_at"]["description"]
                .as_str()
                .expect("removal-event freshness cap is documented")
                .contains(removal_cap_schema_claim)
            && observation["fresh_until"]["description"]
                .as_str()
                .expect("freshness cap is documented")
                .contains(removal_cap_schema_claim)
            && observation["fresh_until"]["description"]
                .as_str()
                .expect("no-prior cap behavior is documented")
                .contains(no_prior_cap_claim)
            && observation["fresh_until"]["description"]
                .as_str()
                .expect("capped expiry behavior is documented")
                .contains(born_inert_claim)
            && observation["observed_at"]["description"]
                .as_str()
                .expect("event contradiction retention is documented")
                .contains(contradiction_claim)
            && observation["observed_at"]["description"]
                .as_str()
                .expect("revocation propagation bound is documented")
                .contains(resync_bound_claim)
            && a7.contains(removal_membership_claim)
            && a7.contains(removal_cap_doc_claim)
            && a7.contains(no_prior_cap_claim)
            && a7.contains(born_inert_claim)
            && a7.contains(contradiction_claim)
            && a7.contains(resync_bound_claim),
        "contradicted removals must remain replayable, visible, and propagation-bounded"
    );
    let transition_latitude_claim = "transition sweep that opened the epoch";
    assert!(
        external_group["description"]
            .as_str()
            .expect("external membership epoch rules are documented")
            .contains(transition_latitude_claim)
            && a7.contains(transition_latitude_claim),
        "only the epoch-opening sweep may carry a pre-epoch observation"
    );

    let source_timing_claim = "each retained connector source event carrying its current-state read and ingestion-commit instants and each retained synchronization run carrying its start and completion instants";
    assert!(
        a7.contains(source_timing_claim),
        "AUTHORIZATION.md §A7 must retain the timing endpoints needed for replay"
    );

    let pending_retention_claim = "requested reconfiguration, its request instant, and its resynchronization outcome and outcome instant are retained for every decision recorded while it was pending";
    assert!(
        a7.contains(pending_retention_claim),
        "AUTHORIZATION.md §A7 must retain the pending-window endpoints"
    );
    let milestones = read_repo_doc("docs/roadmap/v10/MILESTONES.md");
    let e2_4 = heading_section(&milestones, "MILESTONES.md", "### E2.4 ", "\n### ");
    assert!(
        e2_4.contains(pending_retention_claim),
        "the E2.4 exit gate must retain the pending-window endpoints"
    );
    assert!(
        e2_4.contains(source_timing_claim),
        "the E2.4 exit gate must retain source timing endpoints"
    );
    assert!(
        e2_4.contains(event_polarity_claim)
            && e2_4.contains(positive_absence_claim)
            && e2_4.contains(removal_membership_claim)
            && e2_4.contains(removal_cap_doc_claim)
            && e2_4.contains(no_prior_cap_claim)
            && e2_4.contains(born_inert_claim)
            && e2_4.contains(contradiction_claim)
            && e2_4.contains(resync_bound_claim),
        "the E2.4 exit gate must make current state authoritative and bound contradictions"
    );
    for state in schema["$defs"]["membershipUnavailabilityState"]["enum"]
        .as_array()
        .expect("membership-unavailability states are an enum")
    {
        let state = state
            .as_str()
            .expect("membership-unavailability states are strings");
        let token = format!("`{state}`");
        assert!(
            a7.contains(&token) && e2_4.contains(&token),
            "A7 and E2.4 must define registered unavailability state {state:?}"
        );
    }
    let empty_absence_claim = "necessary but not sufficient condition for an empty array";
    assert!(
        schema["properties"]["membership_absence_evidence"]["description"]
            .as_str()
            .expect("membership absence completeness is documented")
            .contains(empty_absence_claim)
            && a7.contains(empty_absence_claim)
            && e2_4.contains(empty_absence_claim),
        "group presence must not erase a different group's confirmed absence"
    );
    let oidc_epoch_claim = "Claim-only `oidc_group` instead opens a grant-conferring epoch after provider-configuration validation";
    assert!(
        a7.contains(oidc_epoch_claim) && e2_4.contains(oidc_epoch_claim),
        "claim-only OIDC must not wait for an impossible resynchronization"
    );
    for (surface_name, surface) in [("AUTHORIZATION.md §A7", a7), ("MILESTONES.md E2.4", e2_4)] {
        for claim in [
            "versioned freshness policy retained by the exact binding",
            "requires `fresh_until` to equal that recomputation",
            "shorter of retained `fresh_until`",
            "shorter policy caps existing observations immediately",
            "connector unavailability cannot extend",
            "effective deadline must follow `effective_at`",
            freshness_history_claim,
            scheduled_refresh_claim,
            distinct_policy_claim,
            failed_reenable_claim,
            membership_unavailable_claim,
            group_name_history_claim,
            membership_absence_claim,
            membership_unavailability_claim,
        ] {
            assert!(
                surface.contains(claim),
                "{surface_name} must define membership retention and replay: {claim}"
            );
        }
    }
    assert!(
        e2_4.contains(source_kind_binding_claim),
        "E2.4.T2 must reject a source kind that differs from the retained binding"
    );
    assert!(
        e2_4.contains(observation_link_claim)
            && e2_4.contains(source_record_binding_claim)
            && e2_4.contains(source_subject_claim),
        "E2.4.T2 must reject observation or source-record substitution"
    );
    assert!(
        e2_4.contains(manual_membership_binding_claim),
        "E2.4.T1 must reject a manual membership from another principal or group"
    );
    assert!(
        e2_4.contains("does not equal the commit or completion instant of the event or run identified by that same"),
        "E2.4.T2 must reject an observation made effective by a different event or run"
    );
    assert!(
        e2_4.contains(transition_latitude_claim),
        "E2.4.T2 must reject routine sweeps that cross an epoch boundary"
    );
    assert!(
        a7.contains(link_lifecycle_claim)
            && a7.contains(link_endpoint_claim)
            && e2_4.contains(link_lifecycle_claim)
            && e2_4.contains(link_endpoint_claim)
            && e2_4.contains(link_principal_claim),
        "A7 and E2.4 must reject wrong-principal and inactive identity links"
    );
    let oidc_claim = "freshly issued and verified ID token";
    let oidc_timing_claim =
        "token issuance, validation/ingestion-commit, and session-expiry instants";
    let oidc_lifetime_claim = "no later than the cited identity session's expiry";
    let oidc_session_binding_claim = "must equal the identity session retained by";
    let oidc_human_claim = "source kind is claim-only in V1 and valid only for a human principal";
    let relink_recovery_claim = "Linking or relinking an external identity completes the link and records a pending current-state membership read for that principal against every grant-conferring binding of the linked source kind";
    let relink_outcome_claim =
        "each binding read and its outcome are recorded and operator-visible";
    let relink_retry_claim = "failed read is surfaced with an explicit operator retry";
    let relink_retention_claim = "request instant and outcome instant are retained for every decision recorded while it was pending";
    let oidc_recovery_claim = "Claim-only `oidc_group` instead recovers at the next authentication";
    let registry_doc = registry();
    let registry_group_sources =
        support::doc_scan::anchored_block(&registry_doc, REGISTRY, "registry:group-source-kinds");
    assert!(
        a7.contains(oidc_claim)
            && e2_4.contains(oidc_claim)
            && registry_group_sources.contains(oidc_claim),
        "OIDC group membership must define claim-only freshness and recovery"
    );
    assert!(
        observation["source_event_id"]["description"]
            .as_str()
            .expect("membership source provenance is documented")
            .contains(oidc_timing_claim)
            && observation["observed_at"]["description"]
                .as_str()
                .expect("membership observation time is documented")
                .contains("token issuance instant")
            && effective_at.contains("validation/ingestion-commit instant")
            && a7.contains(oidc_timing_claim),
        "claim-only OIDC must map issuance, validation, commit, and expiry to retained provenance"
    );
    assert!(
        effective_at.contains(oidc_lifetime_claim)
            && a7.contains(oidc_lifetime_claim)
            && e2_4.contains(oidc_lifetime_claim)
            && registry_group_sources.contains(oidc_lifetime_claim),
        "claim-only OIDC membership must not outlive its exact identity session"
    );
    assert!(
        schema["$defs"]["principal"]["properties"]["identity_session_id"]["description"]
            .as_str()
            .expect("principal identity-session replay is documented")
            .contains(oidc_session_binding_claim)
            && effective_at.contains(oidc_session_binding_claim)
            && a7.contains(oidc_session_binding_claim)
            && e2_4.contains(oidc_session_binding_claim),
        "claim-only OIDC must bind the observation to the evaluated identity session"
    );
    assert!(
        schema["$defs"]["principal"]["properties"]["identity_session_id"]["description"]
            .as_str()
            .expect("principal identity-session ownership is documented")
            .contains(oidc_human_claim)
            && a7.contains(oidc_human_claim)
            && e2_4.contains(oidc_human_claim)
            && registry_group_sources.contains(oidc_human_claim),
        "claim-only OIDC group membership must be human-session-only"
    );
    assert!(
        a7.contains(relink_recovery_claim)
            && e2_4.contains(relink_recovery_claim)
            && a7.contains(oidc_recovery_claim)
            && e2_4.contains(oidc_recovery_claim)
            && a7.contains(relink_outcome_claim)
            && e2_4.contains(relink_outcome_claim)
            && a7.contains(relink_retry_claim)
            && e2_4.contains(relink_retry_claim)
            && a7.contains(relink_retention_claim)
            && e2_4.contains(relink_retention_claim),
        "relink recovery must refresh event-driven memberships without treating OIDC as a connector"
    );
}

/// Backticked codes cited by the executable planning surface, one
/// `(document, line, code)` per citation. Covers `gate.*`/`action.*`/
/// `workspace.*`/`delivery.*`/`connect.*` codes and envelope ids. `delivery.*`
/// intentionally spans the Action and Cloud owner tables. `attestation.*`
/// siblings are deliberately outside the net — E8.1.T1 registers them as a
/// registry edit in that slice. `workspace.*` permission primitives belong
/// under their own registry anchor, never `registry:cloud-codes`.
fn cited_codes() -> Vec<(String, usize, String)> {
    let dir = repo_root().join("docs/roadmap/v10");
    let mut cited = Vec::new();
    let mut entries: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "md"))
        .collect();
    entries.sort();
    assert!(!entries.is_empty(), "no v10 planning documents found");
    for path in entries {
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let content = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        if let Some(opener) = support::doc_scan::unclosed_fence(&content) {
            panic!("{name}:{opener}: fence never closes — citations past it are invisible");
        }
        for (number, line) in support::doc_scan::structural_lines(&content) {
            for span in line.split('`').skip(1).step_by(2) {
                let is_registered_code =
                    ["gate.", "action.", "workspace.", "delivery.", "connect."]
                        .iter()
                        .any(|prefix| {
                            span.strip_prefix(prefix).is_some_and(|rest| {
                                !rest.is_empty()
                                    && rest.chars().all(|c| c.is_ascii_lowercase() || c == '_')
                            })
                        });
                if is_registered_code || is_envelope_id(span) {
                    cited.push((name.clone(), number, span.to_string()));
                }
            }
        }
    }
    for prefix in ["gate.", "action.", "workspace.", "delivery.", "connect."] {
        assert!(
            cited.iter().any(|(_, _, code)| code.starts_with(prefix)),
            "no `{prefix}*` citation found"
        );
    }
    cited
}

#[test]
fn cited_codes_resolve_to_registered_entries_only() {
    let registered = all_registered_ids(&registry());
    let unresolved: Vec<_> = cited_codes()
        .into_iter()
        .filter(|(_, _, code)| !registered.contains(code))
        .map(|(name, number, code)| format!("{name}:{number}: `{code}`"))
        .collect();
    assert!(
        unresolved.is_empty(),
        "planning surfaces cite wire codes with no registry row (dispositions count as rows): \n{}",
        unresolved.join("\n")
    );
}

/// The decision register's executable obligations table claims O-01 "cannot
/// silently lapse" — this is what makes that claim true: deleting the row or
/// blanking its "Owed at" cell fails here (E0.3.T5).
#[test]
fn obligations_table_pins_o01() {
    const DECISION_REGISTER: &str = "docs/roadmap/v10/DECISION-REGISTER.md";
    let content = read_repo_doc(DECISION_REGISTER);
    if let Some(opener) = support::doc_scan::unclosed_fence(&content) {
        panic!(
            "{DECISION_REGISTER}:{opener}: fence never closes — the table below may be invisible"
        );
    }
    let block =
        support::doc_scan::anchored_block(&content, DECISION_REGISTER, "decisions:obligations");
    let row = support::doc_scan::structural_lines(block)
        .map(|(_, line)| line.trim())
        .find(|line| {
            line.strip_prefix('|')
                .and_then(|rest| rest.split('|').next())
                .is_some_and(|cell| cell.trim() == "O-01")
        })
        .unwrap_or_else(|| {
            panic!("{DECISION_REGISTER}: the O-01 obligation row is gone — the E4.6 true-up lapsed")
        });
    let owed_at = row.split('|').nth(3).map(str::trim).unwrap_or_default();
    let is_slice_start = owed_at
        .strip_prefix('E')
        .and_then(|rest| rest.strip_suffix(" slice start"))
        .and_then(|slice| slice.split_once('.'))
        .is_some_and(|(major, minor)| {
            !major.is_empty()
                && major.chars().all(|c| c.is_ascii_digit())
                && !minor.is_empty()
                && minor.chars().all(|c| c.is_ascii_digit())
        });
    assert!(
        is_slice_start,
        "{DECISION_REGISTER}: O-01's \"Owed at\" cell must name an executable \
         `E<major>.<minor> slice start`, got {owed_at:?}"
    );
    let slice_id = owed_at.strip_suffix(" slice start").unwrap_or_default();
    let map = read_repo_doc("docs/roadmap/v10/EXECUTION-MAP.md");
    let heading = format!("## {slice_id} ");
    assert!(
        support::doc_scan::structural_lines(&map).any(|(_, line)| line.starts_with(&heading)),
        "{DECISION_REGISTER}: O-01 is owed at {slice_id}, but the execution map \
         has no such slice — the obligation points at nothing"
    );
}

#[test]
fn guard_fires_on_a_duplicated_close_marker() {
    let broken = "\
<!-- registry:dup-close -->\n\
| id |\n\
| --- |\n\
| `adoc.kept.v0` |\n\
<!-- /registry:dup-close -->\n\
| `adoc.lost.v0` | shipped |\n\
<!-- /registry:dup-close -->\n";
    let result = std::panic::catch_unwind(|| {
        support::doc_scan::anchored_block(broken, "fixture", "registry:dup-close").len()
    });
    assert!(
        result.is_err(),
        "a duplicated close marker orphans the rows between the closes — it must fail loudly"
    );
}

#[test]
fn disposed_ids_are_not_registered_elsewhere() {
    let registry = registry();
    let dispositions = anchored_ids(&registry, "registry:dispositions");
    let still_registered: Vec<_> = ANCHORS
        .iter()
        .filter(|anchor| {
            **anchor != "registry:dispositions" && !VOCABULARY_ANCHORS.contains(*anchor)
        })
        .flat_map(|anchor| {
            anchored_ids(&registry, anchor)
                .into_iter()
                .filter(|id| dispositions.contains(id))
                .map(move |id| format!("{anchor}: {id}"))
        })
        .collect();
    assert!(
        still_registered.is_empty(),
        "disposed ids may not remain registered elsewhere: {still_registered:?}"
    );
}

#[test]
fn semantic_failed_has_exactly_one_disposition() {
    let registry = registry();
    let dispositions = anchored_ids(&registry, "registry:dispositions");
    assert!(
        dispositions.contains("action.semantic_failed"),
        "action.semantic_failed lost its disposition row (RT-21: one canonical resolution)"
    );
    let row = registry
        .lines()
        .find(|line| line.starts_with("| `action.semantic_failed`"))
        .expect("disposition row");
    assert!(
        row.contains("`action.semantic_review_failed`"),
        "the disposition must name the surviving canonical code: {row:?}"
    );
    let action_codes = anchored_ids(&registry, "registry:action-codes");
    assert!(
        action_codes.contains("action.semantic_review_failed"),
        "the surviving code must itself be a registered Action code"
    );
}

#[test]
fn bot_attestation_codes_are_exactly_the_canonical_pair() {
    let registry = registry();
    let bot_attestation: BTreeSet<String> = all_registered_ids(&registry)
        .into_iter()
        .filter(|id| id.contains("bot") && (id.contains("attestation") || id.contains("approver")))
        .collect();
    let canonical: BTreeSet<String> = [
        "attestation.bot_approver_rejected",
        "action.attestation_bot_rejected",
    ]
    .iter()
    .map(|id| id.to_string())
    .collect();
    assert_eq!(
        bot_attestation, canonical,
        "competing bot-attestation suffixes across the registry — the E0.3 \
         acceptance allows exactly one canonical Cloud code and its one \
         documented Action wrapper"
    );
}

#[test]
fn bot_attestation_family_has_one_documented_wrapper_mapping() {
    let registry = registry();
    let attestation = anchored_ids(&registry, "registry:attestation-codes");
    assert!(
        attestation.contains("attestation.bot_approver_rejected"),
        "the canonical bot-attestation family root must stay registered (RT-21)"
    );
    let wrapper_row = registry
        .lines()
        .find(|line| line.starts_with("| `action.attestation_bot_rejected`"))
        .expect("the Action wrapper code row (E8.1.T3) is registered");
    assert!(
        wrapper_row.contains("`attestation.bot_approver_rejected`"),
        "the Action wrapper row must document its mapping to the canonical code: {wrapper_row:?}"
    );
}

/// The annex section opened by `heading` (a `## …` line), up to the next
/// `## ` heading or EOF. Loud when the heading is missing.
fn annex_section<'doc>(doc: &'doc str, doc_name: &str, heading: &str) -> &'doc str {
    heading_section(doc, doc_name, heading, "\n## ")
}

/// The section opened by `heading`, up to `next_heading` or EOF.
fn heading_section<'doc>(
    doc: &'doc str,
    doc_name: &str,
    heading: &str,
    next_heading: &str,
) -> &'doc str {
    let start = doc
        .find(heading)
        .unwrap_or_else(|| panic!("{doc_name} lost its `{heading}` section"));
    let body = &doc[start + heading.len()..];
    match body.find(next_heading) {
        Some(end) => &body[..end],
        None => body,
    }
}

/// Fenced blocks inside an annex section or labeled fragment, in document
/// order, each as its non-empty trimmed lines. Handles bare and tagged fences.
fn fenced_lists_in(section: &str) -> Vec<Vec<String>> {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<String>> = None;
    for line in section.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("```") {
            match current.take() {
                Some(block) => blocks.push(block),
                None => current = Some(Vec::new()),
            }
        } else if let Some(block) = current.as_mut()
            && !trimmed.is_empty()
        {
            block.push(trimmed.to_string());
        }
    }
    assert!(
        current.is_none(),
        "annex section ends inside a fence — the list is invisible past it"
    );
    blocks
}

/// K9's two fenced lists (retention classes, then replay postures), in
/// document order.
fn k9_fenced_lists() -> Vec<Vec<String>> {
    let knowledge_model = read_repo_doc("docs/roadmap/v10/KNOWLEDGE-MODEL.md");
    let section = annex_section(&knowledge_model, "KNOWLEDGE-MODEL.md", "## K9.");
    let lists = fenced_lists_in(section);
    assert_eq!(
        lists.len(),
        2,
        "KNOWLEDGE-MODEL.md §K9 must carry exactly its two fenced lists \
         (retention classes, replay postures), found {}",
        lists.len()
    );
    lists
}

/// One closed vocabulary pinned exactly: registry rows equal the annex
/// list, no additions and no losses (sets — document order is not checked).
fn assert_vocabulary(anchor: &str, annex: &str, expected: &[&str]) {
    let registered = anchored_ids(&registry(), anchor);
    let expected_set: BTreeSet<String> = expected.iter().map(|s| s.to_string()).collect();
    assert_eq!(
        registered, expected_set,
        "`{anchor}` drifted from the closed vocabulary in {annex} — \
         a change there is a registry edit plus an annex amendment"
    );
}

#[test]
fn untrusted_change_states_match_s8() {
    assert_vocabulary(
        "registry:untrusted-change-states",
        "SEMANTICS.md §S8",
        &STATES,
    );
    // The annex itself must still carry every state — an S8 rename with an
    // unchanged registry may not pass silently. (S8 lists the states in
    // prose, so this is a citation check, not a list parse.)
    let semantics = read_repo_doc("docs/roadmap/v10/SEMANTICS.md");
    let section = annex_section(&semantics, "SEMANTICS.md", "## S8.");
    for state in STATES {
        assert!(
            section.contains(&format!("`{state}`")),
            "SEMANTICS.md §S8 no longer cites `{state}` — amend the registry \
             and the annex together, never one alone"
        );
    }
}

const STATES: [&str; 8] = [
    "not_required",
    "awaiting_authorization",
    "authorized",
    "running",
    "completed",
    "denied",
    "failed",
    "expired_after_head_change",
];

#[test]
fn retention_classes_match_k9() {
    assert_vocabulary(
        "registry:retention-classes",
        "KNOWLEDGE-MODEL.md §K9",
        &RETENTION_CLASSES,
    );
    assert_eq!(
        k9_fenced_lists()[0],
        RETENTION_CLASSES,
        "KNOWLEDGE-MODEL.md §K9's retention-class list drifted from the \
         registry — amend both together, never one alone"
    );
}

const RETENTION_CLASSES: [&str; 5] = [
    "digest_only",
    "bounded_evidence",
    "exact_candidate_input",
    "temporary_processing",
    "full_source_snapshot",
];

/// The wire strings of one closed-vocabulary enum's `as_str` in an
/// adoc-core source file, parsed in declaration order — the domain enum
/// for a registered vocabulary must not drift from its registry table.
fn as_str_wire_strings(relative: &str, impl_marker: &str, arms: usize) -> Vec<String> {
    let content = read_repo_doc(relative);
    let start = content
        .find(impl_marker)
        .unwrap_or_else(|| panic!("{relative} no longer carries `{impl_marker}`"));
    let block = &content[start..];
    let end = block
        .find("\n}")
        .unwrap_or_else(|| panic!("{relative}: `{impl_marker}` block never closes"));
    let strings: Vec<String> = block[..end]
        .lines()
        .filter_map(|line| line.split_once("=> \"")?.1.split_once('"'))
        .map(|(value, _)| value.to_string())
        .collect();
    assert_eq!(
        strings.len(),
        arms,
        "`{impl_marker}` as_str parse found {} arms — the pattern drifted",
        strings.len()
    );
    strings
}

/// The wire strings of `ReplayPosture::as_str` in adoc-core — the E1.4
/// domain enum for the vocabulary this table registered in E0.3.T4.
fn replay_posture_wire_strings() -> Vec<String> {
    as_str_wire_strings(
        "crates/adoc-core/src/domain/managed_state.rs",
        "impl ReplayPosture",
        4,
    )
}

#[test]
fn replay_postures_match_k9() {
    assert_vocabulary(
        "registry:replay-postures",
        "KNOWLEDGE-MODEL.md §K9",
        &REPLAY_POSTURES,
    );
    assert_eq!(
        k9_fenced_lists()[1],
        REPLAY_POSTURES,
        "KNOWLEDGE-MODEL.md §K9's replay-posture list drifted from the \
         registry — amend both together, never one alone"
    );
    assert_eq!(
        replay_posture_wire_strings(),
        REPLAY_POSTURES.map(String::from),
        "adoc-core's ReplayPosture wire strings drifted from the registry — \
         amend the registry and KNOWLEDGE-MODEL §K9 together with the code, \
         never one alone"
    );
}

const REPLAY_POSTURES: [&str; 4] = [
    "fully_replayable",
    "source_access_required",
    "intentionally_non_replayable",
    "no_longer_replayable_after_deletion",
];

/// The `dimension.state` pairs in KNOWLEDGE-MODEL §K4's fenced block:
/// each top-level key opens a dimension, its `state:` line (plus bare
/// continuation lines carrying only `|`-separated values) lists the
/// closed vocabulary. The nested `<connector>:` key stays inside the
/// synchronization dimension.
fn k4_dimension_states() -> BTreeSet<String> {
    let knowledge_model = read_repo_doc("docs/roadmap/v10/KNOWLEDGE-MODEL.md");
    let section = annex_section(&knowledge_model, "KNOWLEDGE-MODEL.md", "## K4.");
    assert!(
        section.contains("required_before_effective: true | false"),
        "KNOWLEDGE-MODEL.md §K4 lost the per-connector `required_before_effective` flag"
    );
    let lists = fenced_lists_in(section);
    assert_eq!(
        lists.len(),
        1,
        "KNOWLEDGE-MODEL.md §K4 must carry exactly its one fenced dimension block, found {}",
        lists.len()
    );

    let mut pairs = BTreeSet::new();
    let mut dimension: Option<String> = None;
    let mut collecting = false;
    for line in &lists[0] {
        if let Some(values) = line.strip_prefix("state:") {
            collecting = true;
            extend_states(&mut pairs, dimension.as_deref(), values);
        } else if let Some(name) = line.strip_suffix(':') {
            collecting = false;
            if name != "<connector>" {
                dimension = Some(name.to_string());
            }
        } else if collecting && line.contains('|') && !line.contains(':') {
            extend_states(&mut pairs, dimension.as_deref(), line);
        } else {
            collecting = false;
        }
    }
    pairs
}

fn extend_states(pairs: &mut BTreeSet<String>, dimension: Option<&str>, values: &str) {
    let dimension = dimension.expect("KNOWLEDGE-MODEL.md §K4: a `state:` line outside a dimension");
    pairs.extend(
        values
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| format!("{dimension}.{value}")),
    );
}

/// The `wired("<dimension>", "<state>")` rows of adoc-core's
/// `AUDIT_EMITTER_REGISTRY`, parsed from the source like
/// [`diagnostic_code_table`] parses `diagnostic.rs`. adoc-core's own
/// coverage tests force that registry to equal the six vocabulary enums
/// exactly (both directions), so pinning the registry table to these
/// rows transitively pins it to the domain enums — a vocabulary edit in
/// `managed_state.rs` cannot pass while the registry document and §K4
/// stay stale.
fn audit_emitter_wired_pairs() -> BTreeSet<String> {
    let content = read_repo_doc("crates/adoc-core/src/domain/managed_state.rs");
    let mut pairs = BTreeSet::new();
    for line in content.lines() {
        let Some(rest) = line.trim().strip_prefix("wired(\"") else {
            continue;
        };
        let Some((dimension, rest)) = rest.split_once("\", \"") else {
            continue;
        };
        let Some((state, _)) = rest.split_once('"') else {
            continue;
        };
        pairs.insert(format!("{dimension}.{state}"));
    }
    assert!(
        pairs.len() >= 27,
        "wired(...) parse found only {} rows in managed_state.rs — the row pattern drifted",
        pairs.len()
    );
    pairs
}

/// The six-dimension managed state vocabularies (E1.4, decision D07/D15):
/// registry rows, the pinned list here, KNOWLEDGE-MODEL §K4's fenced
/// block, and adoc-core's audit emitter registry (transitively, the
/// domain enums) must agree exactly — a change anywhere is a registry
/// edit plus a §K4 amendment, never one alone.
#[test]
fn managed_state_dimensions_match_k4() {
    assert_vocabulary(
        "registry:managed-state-dimensions",
        "KNOWLEDGE-MODEL.md §K4",
        &MANAGED_STATE_DIMENSIONS,
    );
    let expected: BTreeSet<String> = MANAGED_STATE_DIMENSIONS
        .iter()
        .map(|pair| pair.to_string())
        .collect();
    assert_eq!(
        k4_dimension_states(),
        expected,
        "KNOWLEDGE-MODEL.md §K4's dimension block drifted from the registry — \
         amend both together, never one alone"
    );
    assert_eq!(
        audit_emitter_wired_pairs(),
        expected,
        "adoc-core's AUDIT_EMITTER_REGISTRY (and with it the domain vocabulary \
         enums) drifted from the registry table — amend the registry and \
         KNOWLEDGE-MODEL §K4 together with the code, never one alone"
    );
    assert!(
        registry().contains("required_before_effective"),
        "the registry's managed-state-dimensions section must carry the \
         per-connector `required_before_effective` flag"
    );
}

/// The two closed vocabularies in KNOWLEDGE-MODEL §K8's first fenced
/// block: the `state:` line's values, then the `required_at:`
/// continuation lines' values (both `|`-separated, in document order).
fn k8_obligation_vocabularies() -> (Vec<String>, Vec<String>) {
    let knowledge_model = read_repo_doc("docs/roadmap/v10/KNOWLEDGE-MODEL.md");
    let section = annex_section(&knowledge_model, "KNOWLEDGE-MODEL.md", "## K8.");
    let lists = fenced_lists_in(section);
    assert_eq!(
        lists.len(),
        2,
        "KNOWLEDGE-MODEL.md §K8 must carry exactly its two fenced blocks \
         (state/required_at vocabularies, the Approved · Not verified · \
         Pending effectivity composite), found {}",
        lists.len()
    );
    let split = |values: &str| -> Vec<String> {
        values
            .split('|')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .collect()
    };
    let mut states = Vec::new();
    let mut stages = Vec::new();
    let mut in_stages = false;
    for line in &lists[0] {
        if let Some(values) = line.strip_prefix("state:") {
            in_stages = false;
            states.extend(split(values));
        } else if line == "required_at:" {
            in_stages = true;
        } else if in_stages {
            stages.extend(split(line));
        } else {
            panic!("KNOWLEDGE-MODEL.md §K8: unrecognized vocabulary line {line:?}");
        }
    }
    (states, stages)
}

/// The K8 proof-obligation vocabularies (E1.6, decision D16): registry
/// rows, the pinned lists here, KNOWLEDGE-MODEL §K8's fenced block, and
/// adoc-core's `ObligationState`/`ObligationStage` wire strings must
/// agree exactly — a change anywhere is a registry edit plus a §K8
/// amendment, never one alone.
#[test]
fn proof_obligation_vocabularies_match_k8() {
    assert_vocabulary(
        "registry:proof-obligation-states",
        "KNOWLEDGE-MODEL.md §K8",
        &OBLIGATION_STATES,
    );
    assert_vocabulary(
        "registry:proof-obligation-stages",
        "KNOWLEDGE-MODEL.md §K8",
        &OBLIGATION_STAGES,
    );
    let (states, stages) = k8_obligation_vocabularies();
    assert_eq!(
        states,
        OBLIGATION_STATES.map(String::from),
        "KNOWLEDGE-MODEL.md §K8's state vocabulary drifted from the \
         registry — amend both together, never one alone"
    );
    assert_eq!(
        stages,
        OBLIGATION_STAGES.map(String::from),
        "KNOWLEDGE-MODEL.md §K8's required_at vocabulary drifted from the \
         registry — amend both together, never one alone"
    );
    const OBLIGATION_RECORD: &str = "crates/adoc-core/src/domain/obligation_record.rs";
    assert_eq!(
        as_str_wire_strings(OBLIGATION_RECORD, "impl ObligationState", 5),
        OBLIGATION_STATES.map(String::from),
        "adoc-core's ObligationState wire strings drifted from the registry — \
         amend the registry and KNOWLEDGE-MODEL §K8 together with the code, \
         never one alone"
    );
    assert_eq!(
        as_str_wire_strings(OBLIGATION_RECORD, "impl ObligationStage", 6),
        OBLIGATION_STAGES.map(String::from),
        "adoc-core's ObligationStage wire strings drifted from the registry — \
         amend the registry and KNOWLEDGE-MODEL §K8 together with the code, \
         never one alone"
    );
}

const OBLIGATION_STATES: [&str; 5] = ["open", "satisfied", "waived", "failed", "expired"];

const OBLIGATION_STAGES: [&str; 6] = [
    "proposal_validation",
    "approval",
    "verification",
    "effectivity",
    "connector_synchronization",
    "agent_action",
];

const MANAGED_STATE_DIMENSIONS: [&str; 27] = [
    "governance.proposed",
    "governance.approved",
    "governance.rejected",
    "governance.revoked",
    "verification.unverified",
    "verification.partially_verified",
    "verification.verified",
    "verification.failed",
    "effectivity.pending",
    "effectivity.scheduled",
    "effectivity.effective",
    "effectivity.suspended",
    "effectivity.expired",
    "freshness.current",
    "freshness.needs_review",
    "freshness.stale",
    "integrity.clear",
    "integrity.potentially_conflicting",
    "integrity.contradicted",
    "synchronization.in_sync",
    "synchronization.pending_writeback",
    "synchronization.pending_external_approval",
    "synchronization.writeback_failed",
    "synchronization.source_ahead",
    "synchronization.source_diverged",
    "synchronization.paused",
    "synchronization.not_applicable",
];

#[test]
fn own_projection_diagnostic_is_registered_only_as_cloud_code() {
    let doc = registry();
    assert!(anchored_ids(&doc, "registry:cloud-codes").contains("writeback.own_projection"));
    assert!(!anchored_ids(&doc, "registry:gate-codes").contains("writeback.own_projection"));
}

#[test]
fn deletion_incomplete_is_registered_only_as_cloud_code() {
    let doc = registry();
    assert!(anchored_ids(&doc, "registry:cloud-codes").contains("privacy.deletion_incomplete"));
    assert!(!anchored_ids(&doc, "registry:gate-codes").contains("privacy.deletion_incomplete"));
}

#[test]
fn migration_lifecycle_vocabulary_and_cloud_refusal_are_registered() {
    let doc = registry();
    let schema: serde_json::Value = serde_json::from_str(&read_repo_doc(
        "docs/agent/v0/schema/agentdoc.cloud.migration_transition_request.v0.schema.json",
    ))
    .unwrap();
    let states: BTreeSet<String> = schema["$defs"]["state"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(states.len(), 10);
    assert_eq!(
        states,
        anchored_ids(&doc, "registry:migration-lifecycle-states")
    );
    assert!(anchored_ids(&doc, "registry:cloud-codes").contains("migration.illegal_transition"));
    assert!(
        !anchored_ids(&doc, "registry:diagnostic-codes").contains("migration.illegal_transition")
    );
}
