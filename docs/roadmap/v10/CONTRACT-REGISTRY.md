# Product V1 Contract and Wire-Code Registry

**Status:** Accepted — canonical wire inventory (E0.3)
**Date:** 2026-08-22
**Registry version:** 1
**Authority:** [`EXECUTION-MAP.md`](EXECUTION-MAP.md) §E0.3 · corrections provenance [`RED-TEAM-CLOSURE.md §RT-21`](RED-TEAM-CLOSURE.md#rt-21--contract-inventory-corrections-from-original-pr-review)
**Guards:** `crates/adoc-mcp/tests/contract_registry_guard.rs` (`adoc`) and the completeness-scan CI referencing this file from `agentdoc-dev/action` and `agentdoc-dev/cloud` (E0.3.T5)

## Registry rule

No externally observable V1 wire code or contract exists outside this registry (E0.3 exit gate). A new envelope, Diagnostic Code, event code, state vocabulary entry, retention class, or replay posture ships only together with its row here; the guards fail any repository emitting a code this file does not carry.

The executable planning surface this registry governs is `docs/roadmap/v10`; preserved historical documents (non-executable per [`EXECUTION-MAP.md`](EXECUTION-MAP.md) §1) may cite retired codes as provenance without a row here.

Field vocabularies enclosed by an envelope (statuses, enum-valued fields) are governed by that envelope's schema and registered through its row; vocabularies observable independently of a single envelope are registered explicitly in the vocabulary sections at the end of this file.

**Statuses:** `unreleased` — implemented and tested in the current development surface, with no release/deployment claim; `shipped` — emitted by a released surface; `planned` — reserved id with an owning E-slice (name adjustments before first implementation are registry edits at slice start); `historical` — no longer emitted, documentation retained; `removed` — never to be emitted again, carried as a disposition (see “Dispositions”).

**Version cells** name the minimum–maximum tested producer/consumer releases; a single value means minimum = maximum. **v0-additive** as a migration posture means fields may be added JSON-optionally within the version and any breaking change requires a new registered version id.

## Envelopes — implemented, owner `adoc`

This table inventories the implemented `adoc` release-train surfaces (CLI, MCP server, and local gateway), including validators of externally produced inputs. Each row records its tested producer/consumer versions and actual release status. The existing `envelopes-shipped-adoc` machine anchor is retained for the source-completeness guard; an `unreleased` row is not evidence of a released binary or deployment.

<!-- registry:envelopes-shipped-adoc -->
| id | status | producer (min–max tested) | consumers (min–max tested) | migration posture |
| --- | --- | --- | --- | --- |
| `adoc.change_assessment.v0` | shipped | adoc 0.3.4 | Action v2.0.0-alpha.19; Cloud ingestion planned (E4.6) | v0-additive |
| `adoc.contradictions.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.diff.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.executor_qualification.v0` | shipped | adoc 0.4.x | adoc 0.4.x authoritative validator and MCP schema resource; Cloud v0.1.0 store/route (E3.3) | exact-match reader; four ordered eligibility layers are protocol-valid under the currently accepted protocol version, AgentDoc-evaluated for the named capability, organization-approved for the exact requested scope/risk/deployment, and runtime-policy-eligible for the exact operation; gate authority additionally requires caller-supplied bindings from the trusted immutable store for the exact qualification ID/record digest, requested capability and approval dimensions, accepted protocol version, and current organization/runtime policy digests, plus an unchanged exact executor configuration; protocol-valid-only or stale/untrusted output is advisory; model records bind exact executor/model/config digests and every named requalification input; human records bind an authenticated principal and permission-policy digest instead of benchmark evidence |
| `adoc.gate_result.v0` | shipped | adoc 0.4.x | Cloud E5.3 producer; Action E5.4 consumer; adoc 0.4.x authoritative validator and MCP schema resource | exact-match reader; deterministic record over one lowercase 40-hex head SHA, canonical nonblank policy version, sorted unique input-digest set, optional configured mode, derived effective mode, pass/block result, and sorted unique reasons from the closed 12-code `gate.*` set; effective advisory mode is pass-only; a required-mode pass carries at least one input digest, while advisory passes and fail-closed blocks may carry none; every block names at least one reason and every pass has none because registered reasons denote blockers; absent configured mode derives `advisory`, a known configured mode exact-matches the effective mode, and an admitted unknown string derives no effective mode and blocks only with `gate.mode_unknown`; configured text is preserved verbatim but is limited to 128 Unicode scalar values without C0/C1 controls; text outside those bounds is rejected before result construction and every producer must fail the run closed without retaining or falling back to a prior/default mode; no timestamp or explanatory prose is admitted; Cloud owns policy evaluation and audit persistence |
| `adoc.graph.traversal.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.graph.v6` | shipped | adoc 0.4.0 | adoc 0.4.0 (CLI/MCP/local gateway surfaces) | exact-match reader; v5 rejected with `schema.unsupported_version` + rebuild guidance; migration is deterministic regeneration from source (ADR-0058); E6.6.T5 export descriptors may name this contract for an exact retained Knowledge Object node fragment, explicitly not a standalone Graph Artifact envelope |
| `adoc.impacted.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.lifecycle_mapping.v0` | shipped | adoc 0.4.0 | adoc 0.4.0 (domain contract-tested; schema `adoc.lifecycle_mapping.v0.schema.json`); Cloud v0.1.0 (pre-release) import route — data-only consumer, contract-tested (E1.5.T3) | exact-match versions: an unknown recorded mapping/projection version is rejected with `schema.unsupported_version`; a rule change requires a version bump (the serialized version-1 contract is pinned in domain tests); historical applications replay under their recorded version; mapping alone never establishes authority and approval is never mapped to verification (KNOWLEDGE-MODEL §K5) |
| `adoc.materiality.v0` | shipped | adoc 0.4.x | enclosed by `adoc.semantic_assessment.v0`; Cloud gate consumer planned (E5.3) | exact-match typed projection policy (ADR-0059): `consistent → immaterial`, extension/contradiction → `material`, insufficient evidence → `undetermined`; input also requires an exact cited diff-hunk fact; explanatory prose is not policy input; changing the mapping requires a new registered version |
| `adoc.mcp.command.v0` | shipped | adoc 0.3.4 | MCP agent clients (contract-tested at adoc 0.3.4) | v0-additive |
| `adoc.migrate.report.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.patch.apply.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4; Action v2.0.0-alpha.19 | v0-additive |
| `adoc.patch.check.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4; Action v2.0.0-alpha.19 | v0-additive |
| `adoc.patch.v0` | shipped | adoc 0.3.4 (validator; input authored by agents) | adoc 0.3.4; Action v2.0.0-alpha.19 | v0-additive; source body and field blank checks trim ASCII edges only, matching source value objects, so Unicode whitespace remains authored data |
| `adoc.project.status.v0` | shipped | adoc 0.3.4 | MCP agent clients (contract-tested at adoc 0.3.4) | v0-additive |
| `adoc.proof_obligation.v0` | shipped | adoc 0.4.0 | adoc 0.4.0 (domain contract-tested; schema `adoc.proof_obligation.v0.schema.json`); Cloud approval surface — data-only consumer (E1.6.T4) | v0-additive; stage-bound stateful obligation record + waiver + classification policy (KNOWLEDGE-MODEL §K8, D16): obligation states and `required_at` stages are registered closed vocabularies (see “Proof obligation states” / “Proof obligation stages”); informational-vs-blocking per stage/risk/action is classification-policy data enclosed by this envelope; a waiver binds the exact obligation + workspace-qualified managed-version subject + principal + policy version and never converts unverified to verified — an expired waiver reopens its obligation as blocking; the record object embeds neither the waiver nor the policy — both are envelope-governed `$defs` subschemas carried by the obligation ledger's event stream (their enclosing audit envelope is registered when the E1.4/E4.2 enclosure lands), so a `waived` record is interpretable only alongside its binding waiver's ordinal bound; the stateless `ProofObligation` shape embedded in `adoc.review.v0`/`adoc.patch.check.v0` is a separate, unchanged contract related by a bridge constructor |
| `adoc.proposal.v0` | shipped | adoc 0.4.x | adoc 0.4.x (`adoc proposal-record`, domain contract-tested; schema `adoc.proposal.v0.schema.json`); Action v2 E5.1 producer (`propose.sh` retained record); Cloud E5.1 consumer as the `agentdoc.cloud.proposal_command.v0` payload | exact-match reader (ADR-0062; owner adjusted from the E0.3 `cloud` reservation to `adoc` at E5.1 slice start because the record is a shared domain contract produced by the Action and the CLI and consumed by Cloud); canonical proposal record keyed by the proposal-set digest over exact patch bytes ordered by patch digest alone (ADR-0062 §2, superseding ADR-0053 §8 in part), bound to exact base/head revisions, change-request system + id (never branch name or title), deterministic assessment digest, semantic-context digest, semantic-assessment digest, and the exact `content_hash` of every edited Knowledge Object; any patch byte change mints a new record whose `supersedes` names the prior digest; every embedded patch declares the Action-owned `agent` proposer with a non-empty identifier and is closed to `create_object`/`update_fields`/`replace_body` at the ADR-0053/ADR-0054 non-authoritative floors, every existing-object edit carrying an `update_fields` that sets a reviewable status (`proposal_record.authority_rejected`), so a proposal can never carry candidate activation or governance-record mutation; every patch passes graph-independent patch validation plus the proposal-only semantic-text reason and verbatim create-status floors, contains no semantically absent but digest-visible null member, and pairs an Object ID target and entry page with a project-relative slash-normalized placement path; creates require matching placement and cannot use a same-set create as an anchor, while every patch editing one target carries the same exact-head coordinate so any patch entry supplies its unambiguous coordinate, and one target is created once or edited in the closed update/body sequence (`proposal_record.patch_invalid`); no equivalent record read compares different targets, so both consistency and correctness of cross-target page-to-path coordinates remain an exact-head E5.1 Cloud consumer preflight, checked by ingestion tests against `adoc` output; E5.3.T3 adds optional, finding-sorted `no_change_required` evidence referencing a server-minted acceptance-receipt digest, omitted when empty and excluded from patch-set identity; Cloud exact-matches the receipt's workspace/repository/change-request/head/semantic-assessment/finding/human-principal/authorization/policy bindings and persists it separately before proposal-set deduplication |
| `adoc.repository_baseline.v0` | shipped | adoc 0.3.4 | Action v2.0.0-alpha.19 | v0-additive; unchanged PR #140 wire, retroactively accepted by [ADR-0064](../../adr/0064-repository-baseline-contract-true-up.md); exact-head/evaluation-date projection with deterministic readiness precedence `invalid_source` → `provisional_paths` → `uncovered_paths` → `ready` and digest-bearing graph/Object-set/config/policy facts; Action bootstrap is the first consumer and retains a SHA-256 digest of the exact bytes; published schema `adoc.repository_baseline.v0.schema.json` plus producer/schema parity and Action consumer tests satisfy permanent obligation O-01 |
| `adoc.managed_field_declassification.v0` | unreleased | native authenticated knowledge.declassify admission (E6.2.T4) | Adoc validating constructor/consumer and MCP schema resource | exact-match [schema](../../agent/v0/schema/adoc.managed_field_declassification.v0.schema.json) and [guide](../../agent/v0/managed-field-declassification.md); immutable exact-version/manifest/event binding, strictly descending selected-field classification, contributing assertion IDs, authenticated principal/session/decision/policy, rationale/date and true-only evidence-withholding posture. Shape validity grants no authority; existing bare state event bytes remain unchanged |
| `adoc.managed_field_provenance.v0` | unreleased | trusted Cloud policy-administrator manifest producer (E6.2.T1) | Adoc validating constructor/consumer and MCP schema resource | exact-match [schema](../../agent/v0/schema/adoc.managed_field_provenance.v0.schema.json) and [guide](../../agent/v0/managed-field-provenance.md); immutable workspace/canonical/version/content-digest binding plus distinct field selectors and native workspace assertion row UUIDs, bounded 100 fields × 100 assertions. Canonical serialization sorts fields and contributors; native approval stores exact submitted bytes. Existing source/assertion/snapshot contracts remain unchanged. Shape validity grants no authority; field projection remains E6.2.T2 |
| `adoc.sensitive_access.v0` | unreleased | trusted Cloud producer (E6.1.T6) | Adoc domain validator and MCP schema resource | exact-match [schema](../../agent/v0/schema/adoc.sensitive_access.v0.schema.json); clock-free managed-workspace access event binds actual workspace, authenticated principal/session, command, policy revision, scoped sequence and ordered distinct returned sensitive KO coordinates. Both internal and restricted are sensitive; no bodies/query/timestamps. Schema validity grants no authority; E6.3 owns future MCP authenticated emission and sink/spool delivery |
| `adoc.sensitive_access.v1` | unreleased | authenticated MCP gateway (E6.3.T1) | Cloud gateway-reported admission and Adoc validator/resource | exact-match [schema](../../agent/v0/schema/adoc.sensitive_access.v1.schema.json); actual registered repository/human/session binding, complete local policy digest, six retrieval commands, ordered sensitive subjects and JS-safe sequence; 1 MiB/1000 subjects; preserves v0 managed event bytes; no native managed authorization claim |
| `adoc.managed_retrieval_input.v0` | unreleased | trusted Cloud producer planned (E6.1.T5); local exact-input fixtures | adoc 0.4.0 E6.1.T5 working tree, CLI runtime and MCP schema resource | exact-match private input; [schema](../../agent/v0/schema/adoc.managed_retrieval_input.v0.schema.json) and [runtime guide](../../agent/v0/managed-retrieval.md). Validity never grants access. The trusted caller supplies current authorized canonical/version/receipt bindings; core performs field projection and receipt-local closure before the existing retrieval predicate/ranking and returns a separate private contributor manifest for final authorization. The optional exact-bound field_projection is mandatory on provenance-bearing bindings; accessed_object marks actual released roots and copied metadata sources with their direct classification (E6.2.T2). E6.2.T4 optionally carries an exact native declassification event/detail reference per field; capable callers require --require-field-declassification. Ordinary no-visibility/no-provenance bytes remain unchanged. No deployment claim |
| `adoc.retrieval.v1` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v1 additive; v0 is historical (see “Envelopes — historical”) |
| `adoc.review.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.search.v2` | shipped | adoc 0.4.0 | adoc 0.4.0 | exact-match reader; v1 rejected — the bump deliberately invalidates v1 embedding caches so the Graph Artifact v6 wave forces a full re-embed (E1.1.T5, ADR-0058); wire shape unchanged from v1 |
| `adoc.semantic_assessment.v0` | shipped | adoc 0.4.x | adoc 0.4.x (authoritative domain validator and MCP schema resource); Action/Cloud adapters (E3.4); Cloud human-independence policy (E3.6) | exact-match reader with additive human-review facts; provider-neutral assessments contain at least one finding and bind exact base/head revisions plus the canonical `adoc.semantic_context.v0` digest; affected Object ID/hash pairs and every citation resolve inside the declared assessment scope and supplied context; update candidates target cited affected Objects, candidate `body` is required but nullable, and `create_knowledge` carries no candidate until a trusted creation scope exists; provider + model identity mandatory; legacy human submissions without review facts remain base-valid but establish no review authority, while authoritative human validation exact-matches reviewing/requesting Principal IDs to trusted request bindings and derives the `self_assessment | independent` fact (ADR-0060); policy eligibility remains Cloud-owned; materiality policy `adoc.materiality.v0` deterministically projects `consistent → immaterial`, extension/contradiction → `material`, and insufficient evidence → `undetermined` from typed classification plus an exact cited diff hunk (ADR-0059); `no_change_required` is immaterial-only and carries exact context/scope; explanatory prose is never gate input; JSON Schema is transport preflight only and unvalidated JSON has no typed core representation |
| `adoc.semantic_context.v0` | shipped | adoc 0.4.0 | adoc 0.4.0 (domain validator, MCP schema resource, and receipt integration) | exact-match reader; deterministic digest-bound exact revisions; callers supply trusted revision, assessment, selection algorithm/version, complete context-class definitions (ID, required/optional requirement, and byte budget), authorized-scope, and capability-policy expectations; authorized scope is a duplicate-free set, supports multiple scopes, and may be empty when none is authorized; every included closed citation's context class, scope, and canonical content digest resolves against the caller-supplied projection; the projection authenticates included citations but is not an exhaustive retrieval proof; local Graph Artifact projection accepts an empty mapping for citation-free context, otherwise requires an explicit trusted Object ID → class/scope mapping, and maps `knowledge_object` to `{"body": <body>}`, `source_binding` to the exact serialized GraphSourceBinding, and `evidence` to the exact indexed evidence entry; truncated content requires an explicitly permitted truncated digest and therefore fails closed in local receipt mode, whose graph projection permits none; managed-revision validation requires a caller-supplied managed digest/projection and fails closed when absent; diff-hunk and Source Assertion citations require the E4.1 Source Record projections and therefore fail closed in local receipt mode until E4.1; JSON Schema is preflight only and `adoc-core` owns semantic validation |
| `adoc.semantic_context_input.v0` | shipped | adoc 0.4.x | Action v2 E3.4 adapters; generic/customer-hosted semantic executors | exact-match producer input; adoc-core sorts and validates the closed context input then derives coverage, outcome, and `adoc.semantic_context.v0` digest; producers cannot supply derived authority fields |
| `adoc.semantic_executor_request.v0` | shipped | adoc 0.4.x | Action v2 Claude/Codex/generic/human adapters; customer-hosted/local executors | exact-match reader with additive human-review claims; one request shape embeds an integrity-validated ready semantic context, closed adapter and endpoint classes, 60–3600 second timeout, prompt contract, and exact executor/model/config/task/prompt digests; the prompt digest is SHA-256 over compact canonical JSON containing exactly its contract version and instructions; human uses the identical boundary with closed `human` adapter/provider/endpoint bindings; request and assessment Principal IDs are untrusted claims, while authoritative completion separately requires authenticated reviewing/requesting Principal bindings from the invoking platform and exact-matches both documents to them |
| `adoc.semantic_executor_receipt.v0` | shipped | adoc 0.4.x | Action v2 adapter orchestration; Cloud ingestion planned (E4.6) | exact-match validator-owned receipt; completed digests the validator-owned canonical assessment serialization and binds exact request/context/adapter digests, so callers cannot pair a typed assessment with different bytes; failed requires a typed failure code and cannot carry an assessment digest; no wall-clock timestamp |
| `adoc.source_assertion.v0` | shipped | adoc 0.4.x | adoc 0.4.x authoritative validator and MCP schema resource; Cloud immutable observation store (E4.1) | exact-match reader; one small immutable assertion binds exact assertion bytes by SHA-256 digest and byte length to one Source Record, standalone Source Binding, historical Source ACL Snapshot, and versioned extractor; it is an observation and never automatically a canonical Knowledge Object; current authorization and mutable integrity state remain Cloud-owned |
| `adoc.source_binding.v0` | shipped | adoc 0.4.x | adoc 0.4.x authoritative validator and MCP schema resource; Cloud immutable observation store (E4.1) | exact-match reader; standalone envelope reuses the graph-v6 Source Binding shape and binds its connector/source/optional revision/path-or-coordinate/anchor/source-revision digest to one Source Record; placement remains independent of semantic content hashes |
| `adoc.source_record.v0` | shipped | adoc 0.4.x | adoc 0.4.x retained exact-match reader and MCP schema resource | retained reader for source observations emitted before exact ACL Snapshot binding became required; new producers emit v1; digest or length mismatch still fails closed |
| `adoc.source_record.v1` | shipped | adoc 0.4.x | adoc 0.4.x (authoritative domain validator and MCP schema resource); Cloud immutable observation store (E4.1) | exact-match reader; provider-neutral source identity and its exact historical ACL Snapshot ID plus parent-container/bare-resource scope, whole-second observation time, media type, and explicit K9 retention class bind separately stored exact payload bytes by SHA-256 digest and byte length; digest or length mismatch fails closed; the JSON Schema is transport preflight only and unvalidated JSON has no typed core representation |
| `adoc.stale.v0` | shipped | adoc 0.3.4 | CLI/MCP agent clients at adoc 0.3.4 | v0-additive |
| `adoc.work_request.v0` | shipped | adoc 0.4.x | Cloud `source_ci`/`customer_worker` dispatch; external workers | exact-match reader and canonical digest builder using recursively ASCII-key-sorted compact UTF-8 JSON; binds request ID/nonce, Workspace/repository/source, exact revision/change request, ASCII-only contract/capability requirements already serialized in ascending ASCII order, expiry, and authorized workload Principal/subject/audience; unknown versions reject before exact-version decoding and carry remediation (ADR-0061) |
| `adoc.work_result.v0` | shipped | adoc 0.4.x | external workers; Cloud verifier and Action hand-off | exact-match reader and canonical digest builder using recursively ASCII-key-sorted compact UTF-8 JSON; repeats request ID/digest, Workspace/repository/revision and authorized workload identity, then binds runtime name/version, completion nonce, named output digests, and result digest; output names are unique lower-snake-case keys serialized in ascending ASCII order for cross-runtime digest stability; any cross-request/repository/revision/Workspace substitution fails (ADR-0061) |
| `adoc.validation_receipt.v0` | shipped | adoc 0.4.0 | adoc 0.4.0 (CLI receipt mode, contract-tested; schema `adoc.validation_receipt.v0.schema.json`); checksum-pinned CI harness and Cloud driver consume receipt bytes (E1.7.T2/T3) | exact-match retained receipt; digest-bound AgentDoc Validation Runtime receipt (SEMANTICS §S6) with closed context names `config` / `context_artifact` / `semantic_context`; closed result vocabulary `pass` / `fail`; deterministic explicit evaluation date and validator-only construction; Evidence Anchor reads stay advisory and outside the digest binding |
| `adoc.validation_receipt.v1` | shipped | adoc 0.4.x | adoc 0.4.x Cloud-bound receipt mode; Cloud canonical store (E4.2) | exact-match successor adding the closed `source_invocation` context name; otherwise preserves v0 semantics and exact runtime/input/context/diagnostics digest binding; the invocation digest binds immutable source namespace/revision/Binding/ACL/config/evaluation-date evidence without making Cloud data domain authority |
| `agentdoc.connector_capabilities.v0` | shipped | adoc 0.4.x | Action adapter manifests; Cloud capability-policy validation (E4.5) | exact-match reader; binds one claimed publisher and exact adapter version to a non-empty capability-name-keyed manifest, making duplicate authoritative capability identities unrepresentable; each value carries one version, closed maturity and processing-mode vocabularies, explicit dependency/contract ranges, limitations, deployment modes, and optional qualification evidence; `ga` structurally requires an evidence reference, while Cloud independently authenticates publisher/qualification authority; overall stage is display-only and never policy input |
| `adoc.portable_projection_input.v0` | unreleased | Cloud E6.6.T5 retained-byte adapter | adoc 0.4.x pure core and bounded CLI | exact-match private input; native-derived history completeness, exact UTF-8 node and retained event bytes, original decimal workspace ordinals and preservation references; no caller-computed lifecycle or authority claims |
| `adoc.portable_projection.v0` | unreleased | adoc 0.4.x pure core and bounded CLI | Cloud E6.6.T5 archive writer | exact-match complete/partial/failed envelope; existing lifecycle mapping/projection version 1 and strict workspace compiler; canonical identity bindings and explicit losses; valid partial/failed outcomes preserve native records |
| `adoc.migration_request.v0` | shipped | adoc 0.4.0 | adoc 0.4.0 CLI and Cloud E7.1 worker; MCP schema resource | exact-match portable preparation request; bounded complete source bindings and explicit evaluation date; full nonzero lowercase Git commit SHA-1 only; Cloud transport wrapper remains separate |
| `adoc.migration_receipt.v0` | shipped | adoc 0.4.0 | Cloud E7.1 worker; MCP schema resource | exact-match validator-only prepare receipt; binds exact request bytes digest, detached commit validation receipt and full diagnostics; grants no import, promotion or cutover authority |
| `adoc.migration_import.v0` | shipped | adoc 0.4.0 | Cloud E7.1 native candidate importer; MCP schema resource | exact-match bounded candidate input bundle; exact request/job digests, original graph/config/source bytes and full-snapshot per-source validation receipts; no qualification, activation or promotion authority |
| `agentdoc.cloud.migration_import_job.v0` | shipped | Cloud E7.1 trusted job controller (Cloud-owned contract) | adoc 0.4.0 bounded import CLI; MCP schema resource | exact-match stable per-source metadata; complete source path set, observation time and repository ACL scope; inventoried here as an implemented Adoc input, not portable domain authority |
| `agentdoc.cloud.migration_validation_invocation.v0` | shipped | adoc 0.4.0 migration adapter (Cloud-owned contract) | Cloud E7.1 native candidate importer; MCP schema resource | exact-version Cloud source invocation extends the existing nine fields with source_path and request_digest; full-snapshot runtime receipts remain v1; no Cloud policy interpreted by Adoc |
| `adoc.migration_qualification.v0` | shipped | adoc 0.4.0 | Cloud E7.1 qualification admission; MCP schema resource | exact-version bounded actual-runtime outcome: unchanged T2 candidate bundle plus qualification receipt, or failed receipt/diagnostics and exact raw source evidence without candidates; no lifecycle authority |
| `adoc.migration_qualification_receipt.v0` | shipped | adoc 0.4.0 | Cloud E7.1 qualification admission; MCP schema resource | exact policy1 and lifecycle mapping1; deterministic original object/hash/source eligibility with request/job/bundle/graph/config bindings; later authorized attestation remains separate |
<!-- /registry:envelopes-shipped-adoc -->

## Envelopes — shipped, owner `action`

<!-- registry:envelopes-shipped-action -->
| id | status | producer (min–max tested) | consumers (min–max tested) | migration posture |
| --- | --- | --- | --- | --- |
| `adoc.pr_assessment_receipt.v0` | shipped | Action v2.0.0-alpha.19 (ADR-0051) | Action report/enforce surfaces v2.0.0-alpha.19; Cloud ingestion planned (E4.6) | v0-additive |
| `adoc.semantic_review.v0` | shipped | Action v2.0.0-alpha.19 (ADR-0052) | Action report surfaces v2.0.0-alpha.19 | v0-additive; deprecation only via the E8.6 machinery |
<!-- /registry:envelopes-shipped-action -->

## Envelopes — historical

<!-- registry:envelopes-historical -->
| id | status | notes |
| --- | --- | --- |
| `adoc.graph.v2` | historical | legacy V5 graph artifact cited by a rejected-version integration fixture; production emission ended before the V10 registry, and current readers fail closed rather than dropping newer knowledge kinds |
| `adoc.graph.v5` | historical | superseded by `adoc.graph.v6` (E1.1, ADR-0058); production emission stopped; the v5 schema stays published at `docs/agent/v0/schema/graph-artifact.v5.json` for the historical record; rejection fixtures cite it from test scope only |
| `adoc.retrieval.v0` | historical | superseded by `adoc.retrieval.v1`; the v0 schema stays published at `docs/agent/v0/schema/retrieval-envelope.v0.json` for readers of retained output |
| `adoc.search.v1` | historical | superseded by `adoc.search.v2` (E1.1.T5, ADR-0058); production emission stopped; the wire shape is unchanged — the bump exists to invalidate v1 embedding caches for the v6 full re-embed; `docs/agent/v0/schema/search-artifact.json` is updated in place to v2 (unversioned filename) |
<!-- /registry:envelopes-historical -->

## Test-fixture ids — never emitted

Deliberately invalid version fixtures cited from test modules in `crates/*/src`, integration tests, or UTF-8 fixture inputs in `crates/*/tests`, proving rejected-version handling. The completeness scan splits source files at their `#[cfg(test)] mod` boundary and treats integration tests and their textual inputs as test scope: production literals must match the shipped table exactly, and a fixture id emitted from production scope fails the scan. Back-compat tests citing a historical id need no fixture row — the historical table already registers the id — so a fixture id must never collide with any real contract row (guard-enforced).

<!-- registry:test-fixture-ids -->
| id | status | notes |
| --- | --- | --- |
| `adoc.graph.v99` | fixture | rejected-version fixture proving the Validation Runtime's exact-match context-artifact gating (E1.7.T4): neither an older nor a newer unknown graph version is consumed |
| `adoc.gate_result.v99` | fixture | rejected-version fixture proving deterministic gate results fail closed on unsupported versions (E5.3) |
| `adoc.repository_baseline.v99` | fixture | rejected-version fixture proving repository-baseline consumers accept only the governed exact version (ADR-0064) |
| `adoc.search.v99` | fixture | rejected-version fixture for Search Artifact version gating |
| `adoc.proposal.v99` | fixture | rejected-version fixture proving canonical proposal records fail closed with exact-version remediation (E5.1) |
| `adoc.semantic_assessment.v99` | fixture | rejected-version fixture proving semantic assessments fail with exact-version remediation |
| `adoc.semantic_context.v1` | fixture | rejected-version fixture proving the first unsupported semantic-context successor fails closed |
| `adoc.semantic_context.v99` | fixture | rejected-version fixture proving arbitrary future semantic-context versions fail closed |
| `adoc.source_record.v99` | fixture | rejected-version fixture proving Source Records fail with exact-version remediation |
| `adoc.work_request.v99` | fixture | rejected-version fixture proving external work requests fail with exact-version remediation |
| `adoc.work_result.v99` | fixture | rejected-version fixture proving external work results fail with exact-version remediation |
| `agentdoc.connector_capabilities.v1` | fixture | rejected-version fixture proving connector capability manifests fail closed on an unsupported exact successor |
| `agentdoc.cloud.proposal_command.v99` | fixture | rejected-version fixture proving `/api/v1` rejects unknown or superseded proposal-command contracts with registered remediation (E5.1) |
| `agentdoc.cloud.assessment_submission.v99` | fixture | rejected-version fixture proving `/api/v1` rejects unknown or superseded assessment-submission contracts with registered remediation |
| `agentdoc.cloud.writeback_record.v99` | fixture | rejected-version fixture for the closed E6.5.T1 writeback record schema; never emitted |
| `agentdoc.cloud.egress_policy.v99` | fixture | rejected-version fixture for the closed E6.6.T1 category policy; never emitted |
| `adoc.sensitive_access.v2` | fixture | rejected unknown exact successor for the gateway-sensitive-access v1 consumer |
| `adoc.managed_field_provenance.v99` | fixture | rejected-version fixture proving managed field provenance consumers refuse unsupported versions (E6.2.T1) |
<!-- /registry:test-fixture-ids -->

## Envelopes and contracts — planned

Reserved ids for the accepted V1 contract set (E0.3.T2). Each row names its owning repository and the E-slice that implements it; producer/consumer versions are recorded when the owning slice ships its first tested implementation. A name adjustment before first implementation is a registry edit at slice start, never an unregistered rename afterwards.

The 13 `agentdoc.cloud.*` rows below whose owning slice is E5 also inventory implementations already present in Cloud v0.1.0 pre-release at `53f4c3e`; they remain in this block until Cloud's first versioned release, without claiming production deployment. This is an inherited E5 inventory correction required before E6.4.T1, not new milestone completion. Retained receipts keep their exact producer bytes and digests; internal gate facts keep their existing canonical digest inputs. Incompatible payload or digest-semantics changes require a new registered version, not reinterpretation of retained v0 evidence.

<!-- registry:envelopes-planned -->
| id | owner | planned by | notes |
| --- | --- | --- | --- |
| `adoc.pr_assessment_receipt.v1` | action | E2.5 | exact-match successor adding server-bound GitHub Actions workload identity; v0 remains shipped until the successor is released |
| `adoc.pr_assessment_receipt.v2` | action | E3.5 | exact-match successor adding validator-bound semantic assessment, closed status `required` / `completed` / `skipped` / `fell_back` / `failed`, and one independently eligible fallback with both provider identities on `fell_back`; v1 remains available if released, otherwise moves to Dispositions as never shipped |
| `adoc.pr_assessment_receipt.v3` | action | E3.7 | exact-match successor adding the authenticated Cloud work-result hand-off outcome and surfacing the registered `action.cloud_sync_failed` failure code; v2 remains available if released, otherwise moves to Dispositions as never shipped |
| `adoc.pr_assessment_receipt.v4` | action | E3.8 | exact-match successor adding split untrusted/trusted phase provenance, the registered S8 untrusted-change states, and the fork-safe write-path refusal code `delivery.fork_branch_read_only`; v3 remains available if released, otherwise moves to Dispositions as never shipped |
| `adoc.connector_acl_policy.v0` | adoc | E2.6 | activation-time ACL acquisition, freshness, refresh, revocation, outage, and cache/session invalidation declaration; contract-tested schema `adoc.connector_acl_policy.v0.schema.json` |
| `adoc.source_acl_snapshot.v0` | adoc | E2.6 | immutable historical ACL provenance only; `source_acl_ceiling.snapshot_id` records the consulted snapshot while the nested `current_authorization` input in `adoc.authorization_decision.v0` independently proves freshness-bounded current access; contract-tested schema `adoc.source_acl_snapshot.v0.schema.json` |
| `adoc.egress_policy.v0` | adoc | E6.6 | shared AgentDoc egress-policy domain contract reserved by E0.3; the separate `agentdoc.cloud.egress_policy.v0` row owns Cloud's external operation wrapper; provenance RT-21 |
| `adoc.authorization_decision.v0` | adoc | E2.2 | `allow`/`deny`/`insufficient_context` decision record; extended at E2.4 with AgentDoc group and external-binding provenance |
| `agentdoc.cloud.assessment_submission.v0` | cloud | E4.4 | exact-version `/api/v1` assessment-submission transport; payload semantics and durable ingestion remain E4.6-owned |
| `agentdoc.cloud.ingestion_result.v0` | cloud | E4.4 | exact-version ingestion-result transport; disposition semantics remain E4.6-owned |
| `agentdoc.cloud.repository_config.v0` | cloud | E4.4 | exact-version external repository-configuration transport |
| `agentdoc.cloud.work_request.v0` | cloud | E4.4 | exact-version Cloud operation wrapper for dispatching the shared `adoc.work_request.v0` domain contract |
| `agentdoc.cloud.work_result.v0` | cloud | E4.4 | exact-version Cloud operation wrapper for receiving the shared `adoc.work_result.v0` domain contract |
| `agentdoc.cloud.gate_decision.v0` | cloud | E4.4 | exact-version gate-decision transport; its `adoc://` payload reference requires the adjacent registered `adoc.gate_result.v0` schema; gate semantics remain E5.3-owned |
| `agentdoc.cloud.proposal_command.v0` | cloud | E4.4 | exact-version proposal-command transport; proposal semantics remain E5.1-owned |
| `agentdoc.cloud.approval_command.v0` | cloud | E4.4 | exact-version approval-command transport; approval semantics remain E5.2-owned |
| `agentdoc.cloud.migration_request.v0` | cloud | E4.4 | exact-version migration-request transport; migration semantics remain E7.1-owned |
| `agentdoc.cloud.migration_receipt.v0` | cloud | E4.4 | exact-version migration-receipt transport; qualification and cutover semantics remain E7.1-owned |
| `agentdoc.cloud.egress_policy.v0` | cloud | E4.4 | registered transport with E6.6.T1 exact-scope payload and seven required category booleans defined by the [closed schema](../../agent/v0/schema/agentdoc.cloud.egress_policy.v0.schema.json). Scope uses a Cloud subset of the existing authorization vocabulary: canonical Cloud repository or connector instance, optionally narrowed to a native source container/resource. Its `adoc://` scope references require the adjacent registered `adoc.authorization_decision.v0` schema; publishing that schema does not change its planned status. Unknown keys are structural refusals; category additions require a version decision. Cloud enforces current policy authority on the exact scope. A missing or failed scope fetch returns all-disabled plus `egress.policy_unavailable`, never a cached or wider policy. Sender/source binding, gate compatibility and policy-change receipts remain later E6.6 slices; provenance RT-21 and PRD v1.0 §27 |
| `agentdoc.cloud.validation_invocation.v0` | cloud | E4.2 | closed Cloud invocation manifest whose exact bytes bind an AgentDoc validation receipt to one immutable Workspace/Source Record/Source Binding/ACL snapshot/config/evaluation-date tuple |
| `agentdoc.cloud.connector_authority_policy_receipt.v0` | cloud | E4.3 | immutable receipted connector-authority policy change binding exact scope, closed authority mode, promotion rule, policy ID/version, prior effective policy ID/version (or explicit none), policy effect ordinal and policy effect transaction ID, authenticated changing principal, and an allow authorization decision whose principal, `connector.configure` permission, and evaluated scope exact-match the change and that was evaluated under authorization state effective for the policy-change transaction; every governed use binds a strictly later governed-operation ordinal and distinct governed transaction ID, accepts only a receipt visible from a prior committed transaction, and exact-matches that receipt to the policy effective for its exact scope at that operation ordinal, so stale authorization, same-transaction change-then-use, and superseded-policy use fail closed |
| `agentdoc.cloud.candidate_authority.v0` | cloud | E4.3 | immutable candidate provenance binding the exact managed candidate version ID and content digest, authenticated submitting principal, exact policy receipt, policy effect ordinal and transaction ID, strictly later candidate-operation ordinal and transaction ID, authority mode, and proposal origin; AgentDoc-origin candidates additionally bind a current allow authorization decision whose principal, `knowledge.propose` permission, and evaluated scope exact-matches the submission and that was evaluated under authorization state effective for the candidate-operation transaction; proposal origin is server-derived from authenticated ingress and principal class, never caller-supplied; connector-origin candidates require the exact connector Source Assertion, while AgentDoc-origin candidates carry no connector Source Assertion |
| `agentdoc.cloud.external_promotion_attestation.v0` | cloud | E4.3 | closed provider attestation binding the exact managed candidate version ID and content digest, exact effective policy receipt digest, service principal, policy-declared issuer and attestation type, and trusted issuer evidence established independently by a provider-authenticated connector/service or cryptographic verification; its immutable Source Assertion proves exact-byte provenance only |
| `agentdoc.cloud.external_promotion_receipt.v0` | cloud | E4.3 | immutable external-promotion receipt binding the exact attestation digest and Source Assertion, exact managed candidate version ID and content digest, exact policy receipt, exact Governance Event sequence and digest, policy effect ordinal and transaction ID, strictly later promotion-operation ordinal and transaction ID, service principal, and policy-declared issuer and attestation type; promotion accepts only a policy receipt visible from a prior committed transaction and exact-matches the attested policy receipt digest, so same-transaction policy-change-then-promote and attestation reuse across policy versions fail closed |
| `agentdoc.cloud.proposal_disposition_acceptance.v0` | cloud | E5.3.T3 | Cloud `accept_proposal_disposition` produces an immutable receipt consumed by proposal-disposition evidence validation and gate coverage; binds Workspace/repository/change request/head, semantic-assessment digest, finding, human reviewer, authorization-decision digest, and policy version to `no_change_required`; receipt bytes and digest are checked against the v0 byte builder |
| `agentdoc.cloud.proposal_coverage.v0` | cloud | E5.3.T3 | Cloud `gate_proposal_facts_v0` produces an internal digest input consumed by `gate_facts_with_proposal_v0`; binds proposal-set/record/semantic-assessment digests, sorted patch finding IDs, and accepted per-finding disposition evidence; coverage is derived from stored validated facts, not a caller assertion |
| `agentdoc.cloud.approval_gate_fact.v0` | cloud | E5.3.T4 | Cloud `gate_approval_facts_v0` produces an internal digest input consumed by `gate_facts_with_approval_v0`; binds the proposal version/set, approval status/record digest, invalidation or proposal-integrity failure, obligation-policy digest, evaluation ordinal, blockers, and discharged resolutions |
| `agentdoc.cloud.promotion_gate_fact.v0` | cloud | E5.3.T6 | Cloud `gate_promotion_facts_v0` produces an internal digest input consumed by promotion-gate evaluation and emergency-receipt binding; binds assessment-ingestion ID, promotion availability/status, and exact authority-promotion facts from the deterministic assessment |
| `agentdoc.cloud.gate_emergency_receipt.v0` | cloud | E5.3.T6 | Cloud `record_gate_emergency_receipt` produces immutable canonical receipt bytes consumed by `gate_emergency_facts_v0`; binds Workspace/repository/change request/head, assessment/proposal/promotion digests, policy, authenticated principal/session/authorization, justification, creation time, and expiry; expired receipts do not supply active emergency authority |
| `agentdoc.cloud.gate_emergency_fact.v0` | cloud | E5.3.T6 | Cloud `gate_emergency_facts_v0` produces an internal digest input consumed by `gate_facts_with_promotion_v0`; binds the selected exact-scope emergency receipt digest and its `active` or `expired` status at the explicit evaluation time; expiry changes gate evidence rather than rewriting the receipt |
| `agentdoc.cloud.gate_evidence_set.v0` | cloud | E5.3.T6 | Cloud `gate_facts_with_promotion_v0` produces an internal digest input consumed by gate-decision evaluation and stored-fact validation; combines approval status/digest, promotion digest/failure, active emergency receipt ID (nullable), and emergency-fact digest for a valid complete proposal |
| `agentdoc.cloud.github_negative_verdict_acceptance.v0` | cloud | E5.4.T4 | Cloud `record_github_negative_verdict_acceptance`, called from the verified GitHub merge-webhook path, produces immutable acceptance bytes checked by the acceptance table and fact trigger; binds merge/head/commit, provider merging identity, assessment and verdict-receipt digest, and webhook installation/delivery/payload digest; post-hoc merge acceptance does not create proposal approval |
| `agentdoc.cloud.github_check_publication_failure.v0` | cloud | E5.4.T5 | Cloud `record_github_check_publication_failure`, called by `publishGitHubCheck`, produces immutable failure bytes consumed by publication-fact validation and the failure ledger; binds gate decision, attempt, rendered-request digest, observed head, publication posture, failure kind, and optional provider status under `gate.check_publish_failed`; no successful publication receipt is implied |
| `agentdoc.cloud.approved_proposal_promotion.v0` | cloud | E5.5.T1 | Cloud `promote_approved_proposal_candidate` produces immutable causal receipt bytes checked by the promotion trigger and consumed by activation-gate evidence and trace archival; binds exact proposal/patches, native approval, candidate/content/authority/validation receipt, managed promotion, and Governance Event |
| `agentdoc.cloud.activation_gate_evidence.v0` | cloud | E5.5.T1 | Cloud `gate_activation_evidence_v0` produces an internal digest input consumed by gate-decision evaluation and its fact trigger; binds canonical-ID-ordered currently active promotion receipts to their candidate-validation and Governance Event digests for the exact Workspace/approval |
| `agentdoc.cloud.terminal_gate_evidence.v0` | cloud | E5.5.T1 | Cloud `gate_activation_evidence_v0` produces the terminal gate-evidence digest consumed by gate-decision evaluation and its fact trigger; combines the prior gate-evidence digest with activation-evidence digest; without matching active receipts the prior digest is retained rather than inventing activation evidence |
| `agentdoc.cloud.internal_integrated_trace.v0` | cloud | E5.5.T3 | Cloud `archive_internal_integrated_trace` produces immutable canonical bytes consumed by the archive completeness validator and retained evidence store; ten linked hops cover assessment, semantic validation, qualification, proposal, approval, activation, Governance Event, active version, gate decision, and publication receipt; exact version and `internal_synthetic` / `not_external` posture are enforced, never external release evidence |
| `adoc.governance_event.v0` | cloud | E4.2 | append-only governance transition record |
| `adoc.semantic_endpoint_policy.v0` | cloud | E3.4 | immutable declaration binding one generic semantic endpoint id, endpoint class, exact URL, and allowed state; Action rejects a missing or non-matching declaration before invocation; moves to shipped at Cloud's first versioned release |
| `adoc.approval.v0` | cloud | E5.2 | native approval bound to exact proposal digest, principal, policy version |
| `adoc.reconciliation_candidate.v0` | adoc | E1.2 | typed same-Object-ID collision record (ADR-0057 invariant 1, RT-03/D36): names both parties by workspace canonical identity, repository identity, latest immutable version id, and content hash; reason vocabulary closed to `object_id_collision` — hash/title/similarity never produce a candidate and never merge; the record ships in `adoc-core` since E1.2.T1 with its serialized shape pinned in domain tests; moves to shipped when a surface emits it on the wire |
| `adoc.reconciliation_decision.v0` | adoc | E1.3 | principal-bound reconciliation decision record (RT-03; MILESTONES §E1.3): closed verb set `keep_distinct` / `link_alias` / `supersede` / `merge_rehome`; binds subject and counterpart by workspace canonical identity plus exact managed version id and carries non-optional principal and policy version — a decision missing any binding is unconstructible in `adoc-core`, and recording rejects fail-closed: unknown parties, non-latest version bindings, parties that never formed a reconciliation candidate pair, and decisions conflicting with a standing merge (a merged-away party or a merge chain); no wall-clock field, so replaying the recorded decisions over the same import history yields byte-identical reconciliation state; ships in `adoc-core` since E1.3.T1 with its serialized shape pinned in domain tests; E4.2's `adoc.governance_event.v0` (cloud) will enclose it as the event payload; moves to shipped when a surface emits it on the wire |
| `adoc.managed_object_identity.v0` | cloud | E1.2 | workspace-qualified managed Object identity record, served by the Cloud object-identities route since E1.2.T3 (payload: `schema_version`, `canonical_id`, `workspace_id`, `object_id`); the canonical identity is server-minted and never derived from the human-readable Object ID, so the same unqualified Object ID in two Workspaces stays unlinkable (RT-03; MILESTONES §E1.2 stop-ship); v0-additive; moves to shipped at Cloud's first versioned release |
| `agentdoc.cloud.writeback_record.v0` | cloud | E6.5 | planned Cloud producer; contract-tested closed schema and MCP publication are the E6.5.T1 prerequisite, not dispatch implementation. Exact Workspace-qualified managed subject and zero-based Workspace-local event ordinal/digest (never the store-global sequence), writeback ID, connector, exported textual Source Binding ID and record digest, mandatory native target revision precondition, idempotency key (1–200 visible ASCII characters, no spaces) and exact outgoing payload digest. [Cloud binding references](../../agent/v0/writeback-record.md) pin UUID-backed subjects/connector instances, the provider-label join, and the retained event digest. Cloud verifies stored joins, payload bytes and authorization independently; missing/invalid precondition is refused with `api.invalid_request` before any side effect; changed lineage/payload under a reused key is `api.idempotency_conflict`. Unknown versions are rejected; incompatible lineage or digest semantics require a new version, never reinterpretation of retained v0 records |
| `agentdoc.cloud.portable_export_preparation.v0` | cloud | E6.6.T5 | Audited authorized-scope native byte release with frozen immutable descriptors, omissions and projection subjects; exact replay reauthorizes and audits; exact version and closed schema |
| `agentdoc.cloud.portable_export_manifest.v0` | cloud | E6.6.T5 | Exact UTF-8 archive manifest binding frozen raw descriptors, omissions, runtime-pinned projections and preparation receipt; conventional SHA256SUMS remains detached; exact version and closed schema |
| `agentdoc.cloud.export_native_fact.v0` | cloud | E6.6.T5 | Closed per-kind native_json facts for inventory members without retained wire bytes; no historical byte-attestation claim; exact version and closed schema |
| `agentdoc.cloud.portable_export_receipt.v0` | cloud | E6.6.T5 | Immutable native prepared/release_authorized audit receipt; binds selection and optional exact manifest hash, current actor/session/scope and authorization; never client download acknowledgment; exact version and closed schema |
| `agentdoc.cloud.portable_export_finalization.v0` | cloud | E6.6.T5 | Closed release_authorized/selection_changed response; changed access or frozen content releases no archive and requires a fresh request; exact version and closed schema |
| `agentdoc.cloud.migration_import_result.v0` | cloud | E7.1 | native import RPC transport response maps the request to inactive candidate IDs; no portable domain or activation authority; schema published |
| `agentdoc.cloud.migration_qualification_result.v0` | cloud | E7.1 | native qualification admission response; exact request/qualification IDs, outcome, unchanged T2 candidate mapping and sorted internal Source Record/Binding UUID references; no portable domain authority; schema published |
| `agentdoc.cloud.migration_initialization_request.v0` | cloud | E7.1.T4 | closed authenticated human command binding exact qualification and explicit acceptance meaning/rationale; native complete eligible-set and current migration.approve checks; schema and MCP resource published |
| `agentdoc.cloud.migration_initialization_attestation.v0` | cloud | E7.1.T4 | immutable exact-command/qualification/human/session/authorization attestation; canonical native JSON plus newline defines digest; no verification assertion; schema and MCP resource published |
| `agentdoc.cloud.migration_initialization_result.v0` | cloud | E7.1.T4 | retained attestation plus sorted native promotion/governance/initial-effectivity references preserving original Object ID and semantic hash; decimal sequence strings; native identity joins and authorization remain required; complete portable receipt and cutover remain downstream; schema and MCP resource published |
| `agentdoc.cloud.migration_completion_receipt.v0` | cloud | E7.1.T5 | generated portable correspondence joining original request, qualification, human attestation and native promotion/governance/effectivity evidence; preserves original object bindings and semantic hashes; closed schema and MCP resource published; no activation authority |
| `agentdoc.cloud.migration_transition_request.v0` | cloud | E7.2.T1 | closed expected-head command and evidence union; native admission owns adjacency and authorization; schema and MCP published |
| `agentdoc.cloud.migration_transition_receipt.v0` | cloud | E7.2.T1 | exact append-only migration lifecycle receipt with frozen scope/revision, predecessor, native actor and evidence; no cutover or rollback by label alone; schema and MCP published |
| `agentdoc.cloud.migration_transition_result.v0` | cloud | E7.2.T1 | closed exact receipt-byte base64 and SHA256 response; no parsed authoritative duplicate; schema and MCP published |
<!-- /registry:envelopes-planned -->

## Diagnostic Codes — shipped, owner `adoc`

Shared row values: producer `adoc` 0.4.0 (`adoc-core` `diagnostic_codes!` table, the single declaring source); consumers are every envelope embedding `Diagnostic` records (CLI/MCP surfaces at adoc 0.4.0, Action v2.0.0-alpha.19 report rendering). Migration posture for every row: wire-stable string — a meaning change or removal requires a row in “Dispositions”, never reuse.

Explicit mapping (RT-21, like the attestation family): `audit.persistence_failed` is the operation-level code the owning operation surfaces when a state transition's audit record cannot be persisted (E1.4.T4); the gate-level `gate.audit_persistence_failed` (E5.3, “Gate codes” below) is a distinct surface that consumes it. Both stay registered; neither is a respelling of the other.

<!-- registry:diagnostic-codes -->
| code |
| --- |
| `api.verified_missing_schema_evidence` |
| `assessment.base_partial` |
| `assessment.changed_set_failed` |
| `assessment.comparison_base_unavailable` |
| `assessment.graph_failed` |
| `assessment.head_invalid` |
| `assessment.invalid_changed_path` |
| `assessment.invalid_config_path` |
| `assessment.ref_unresolved` |
| `assessment.semantic_citation_invalid` |
| `assessment.semantic_classification_unknown` |
| `assessment.semantic_identity_missing` |
| `assessment.semantic_identity_mismatch` |
| `assessment.semantic_revision_mismatch` |
| `assessment.semantic_schema_invalid` |
| `assessment.semantic_version_unsupported` |
| `assessment.snapshot_failed` |
| `audit.persistence_failed` |
| `build.embeddings_cache_ignored` |
| `build.embeddings_cached` |
| `build.embeddings_skipped` |
| `claim.evidence_quality_low` |
| `claim.status_casing` |
| `claim.verified_missing_evidence` |
| `compat.raw_html_quarantined` |
| `compat.unknown_extension` |
| `compat.unsafe_image_src_dropped` |
| `compat.unsafe_link_dropped` |
| `embed.compute_failed` |
| `embed.model_load_failed` |
| `embed.unexpected_dim` |
| `evidence.hash_drift` |
| `evidence.hash_invalid` |
| `evidence.hash_target_missing` |
| `evidence.hash_unverifiable` |
| `governance.record_conflict` |
| `graph.object_not_found` |
| `id.duplicate` |
| `id.duplicate_in_artifact` |
| `id.invalid` |
| `impacted.git_unavailable` |
| `impacted.invalid_path` |
| `impacted.ref_unresolvable` |
| `io.artifact_malformed` |
| `io.artifact_missing` |
| `io.artifact_unreadable` |
| `io.source_path_unsafe` |
| `io.unreadable_directory` |
| `io.unreadable_file` |
| `io.unsupported_source_extension` |
| `lifecycle.expired` |
| `lifecycle.invalid_expires_at` |
| `mcp.patch_apply_disabled` |
| `migrate.broken_link` |
| `migrate.export_typed_blocks_present` |
| `migrate.raw_html_quarantined` |
| `migrate.source_not_committed` |
| `migrate.target_exists` |
| `migrate.unrecognized_extension` |
| `parse.malformed_field` |
| `parse.malformed_markdown` |
| `parse.malformed_open_fence` |
| `parse.malformed_page_annotation` |
| `parse.nested_typed_block` |
| `parse.raw_html` |
| `parse.unclosed_fence` |
| `parse.unsafe_link` |
| `patch.base_hash_mismatch` |
| `patch.create_missing_placement` |
| `patch.invalid_document` |
| `patch.placement_invalid` |
| `patch.placement_not_adoc` |
| `patch.source_binding_stale` |
| `patch.source_drift` |
| `patch.target_already_exists` |
| `patch.validation_failed` |
| `procedure.verified_missing_evidence` |
| `proposal_record.authority_rejected` |
| `proposal_record.binding_invalid` |
| `proposal_record.invalid_document` |
| `proposal_record.patch_invalid` |
| `proposal_record.revision_unchanged` |
| `ref.broken` |
| `retrieval.audience_unresolved` |
| `retrieval.no_knowledge_objects_consider_migration` |
| `retrieval.object_not_found` |
| `retrieval.policy_invalid` |
| `retrieval.visibility_unavailable` |
| `schema.agent_instruction_actions_not_disjoint` |
| `schema.agent_instruction_invalid_trust` |
| `schema.agent_instruction_missing_allowed_actions` |
| `schema.agent_instruction_missing_forbidden_actions` |
| `schema.agent_instruction_missing_scope` |
| `schema.agent_instruction_missing_trust` |
| `schema.api_conflicting_method_and_interface_type` |
| `schema.api_conflicting_path_and_symbol` |
| `schema.api_invalid_method` |
| `schema.api_invalid_path` |
| `schema.api_missing_method_or_interface_type` |
| `schema.api_missing_path_or_symbol` |
| `schema.claim_contradicted_by_unresolved` |
| `schema.constraint_invalid_severity` |
| `schema.constraint_missing_severity` |
| `schema.contradiction_claim_not_a_claim` |
| `schema.contradiction_claim_not_found` |
| `schema.contradiction_claims_too_few` |
| `schema.contradiction_invalid_severity` |
| `schema.contradiction_invalid_status` |
| `schema.contradiction_missing_claims` |
| `schema.contradiction_missing_severity` |
| `schema.contradiction_missing_status` |
| `schema.duplicate_field` |
| `schema.evidence_target_not_a_source` |
| `schema.evidence_target_not_found` |
| `schema.example_invalid_lang` |
| `schema.example_invalid_sandbox` |
| `schema.example_missing_lang` |
| `schema.example_verified_requires_checks` |
| `schema.example_verified_requires_sandbox` |
| `schema.impacts_empty` |
| `schema.impacts_invalid_path` |
| `schema.invalid_status` |
| `schema.missing_field` |
| `schema.observation_invalid_observed_at` |
| `schema.observation_invalid_sample_size` |
| `schema.observation_invalid_status` |
| `schema.observation_missing_status` |
| `schema.policy_future_effective_at` |
| `schema.policy_invalid_effective_at` |
| `schema.policy_invalid_review_interval` |
| `schema.policy_missing_approved_by` |
| `schema.policy_missing_body` |
| `schema.policy_missing_effective_at` |
| `schema.policy_missing_owner` |
| `schema.policy_missing_status` |
| `schema.policy_review_overdue` |
| `schema.procedure_body_must_start_with_ordered_list` |
| `schema.procedure_missing_body` |
| `schema.procedure_missing_status` |
| `schema.question_answered_missing_resolved_by` |
| `schema.question_missing_status` |
| `schema.question_resolved_by_not_found` |
| `schema.question_resolved_by_wrong_kind` |
| `schema.question_unexpected_resolved_by` |
| `schema.source_conflicting_path_and_url` |
| `schema.source_invalid_kind` |
| `schema.source_invalid_path` |
| `schema.source_invalid_url` |
| `schema.source_kind_target_mismatch` |
| `schema.source_missing_kind` |
| `schema.source_missing_path_or_url` |
| `schema.task_invalid_due` |
| `schema.task_invalid_status` |
| `schema.task_missing_owner` |
| `schema.task_missing_status` |
| `schema.unknown_field` |
| `schema.unknown_kind` |
| `schema.unsupported_version` |
| `schema.visibility_invalid` |
| `search.artifact_missing` |
| `search.deterministic_quality` |
| `search.hash_drift` |
| `search.invalid_filter` |
| `search.invalid_scope` |
| `search.model_mismatch` |
| `semantic_context.basis_mismatch` |
| `semantic_context.digest_mismatch` |
| `semantic_context.failed` |
| `semantic_context.insufficient_context` |
| `semantic_context.invalid_document` |
| `store.retention_floor_violation` |
| `task.overdue` |
| `validation.context_artifact_drift` |
| `projection.unavailable` |
| `privacy.export_digest_mismatch` |
| `migration.invalid_request` |
| `migration.exact_revision_required` |
| `migration.snapshot_unavailable` |
| `migration.unsafe_source` |
| `migration.validation_unavailable` |
| `migration.invalid_job` |
| `migration.validation_failed` |
| `migration.output_limit` |
<!-- /registry:diagnostic-codes -->

## Gateway audit codes — owner `adoc`

These adapter codes are emitted by the MCP gateway and consumed by its clients; native Cloud admission also uses the refusal internally. They are separate from the core Diagnostic table. Meanings are wire-stable; changes require a disposition.

<!-- registry:gateway-audit-codes -->
| code | status | meaning |
| --- | --- | --- |
| `retrieval.audit_sink_unavailable` | unreleased (E6.3.T1) | synchronous sensitive retrieval is refused because trusted audit admission or a matching durable recording receipt is unavailable; no sensitive response is released |
| `retrieval.sensitive_access_unrecorded` | unreleased (E6.3.T2) | sensitive MCP output was permitted under current authority after exact metadata was durably spooled; Cloud recording remains pending and the response includes a visible warning |
| `retrieval.audit_spool_corrupt` | unreleased (E6.3.T2) | journal integrity failed; the next application request is refused without sensitive content or journal details, and retained bytes require operator investigation |
<!-- /registry:gateway-audit-codes -->

## Action codes — owner `action`

Shared row values for shipped rows: producer Action v2.0.0-alpha.19 (workflow annotations, check conclusions, receipt `reason_codes`); consumers are GitHub check/annotation readers and receipt consumers. Migration posture: wire-stable string — meaning change or removal requires a row in “Dispositions”.

<!-- registry:action-codes -->
| code | status | meaning |
| --- | --- | --- |
| `action.assessment_contract_failed` | shipped | assessment envelope violated its contract |
| `action.assessment_not_evaluated` | shipped | assessment did not run for the change set |
| `action.assessment_partial` | shipped | assessment completed with partial coverage |
| `action.assessment_ref_failed` | shipped | assessment base/head ref resolution failed |
| `action.baseline_contract_failed` | shipped | repository baseline envelope violated its contract |
| `action.baseline_not_ready` | shipped | repository baseline not yet available for this head |
| `action.baseline_unavailable` | shipped | repository baseline could not be produced |
| `action.bootstrap_dirty` | shipped | bootstrap found a dirty working tree |
| `action.install_failed` | shipped | toolchain/provider installation failed |
| `action.invalid_input` | shipped | Action inputs invalid |
| `action.knowledge_delivery_failed` | shipped | knowledge proposal delivery failed |
| `action.knowledge_proposal_incomplete` | shipped | knowledge proposal set incomplete |
| `action.knowledge_review_incomplete` | shipped | knowledge review incomplete |
| `action.knowledge_sync_pending` | shipped | knowledge synchronization still pending |
| `action.path_limit_exceeded` | shipped | changed-path limit exceeded |
| `action.proposal_failed` | shipped | proposal creation failed |
| `action.proposal_rejected` | shipped | proposal rejected by validation |
| `action.provider_integrity_failed` | shipped | provider binary integrity verification failed |
| `action.receipt_failed` | shipped | receipt finalization failed |
| `action.semantic_review_failed` | shipped | semantic review failed; the single canonical Action semantic-failure reason code (see “Dispositions”) |
| `action.structural_errors_changed` | shipped | structural errors in changed objects |
| `action.structural_errors_full` | shipped | structural errors in the full graph |
| `action.unsupported_event` | shipped | unsupported triggering event |
| `action.cloud_sync_failed` | shipped | Action v2 E3.7 Cloud hand-off failed; local assessment preserved and annotated, never failed retroactively |
| `action.attestation_bot_rejected` | planned (E8.1) | Action check wrapper for the canonical Cloud code `attestation.bot_approver_rejected` — one documented mapping, no competing suffix |
| `delivery.fork_branch_read_only` | planned (E3.8) | Action refuses writes to a fork branch and names the separate base-repository PR alternative |
<!-- /registry:action-codes -->

## Gate codes — owner `adoc`

Contract codes for the four-mode gate evaluator (E5.3; check publication E5.4). The E5.3 failure matrix is the closed 12-code shipped set below; `gate.check_publish_failed` remains the separate E5.4 publication code and is not a valid `adoc.gate_result.v0` reason. Cloud-owned operational `gate.*` codes are registered under “Cloud codes” below and do not extend this closed reason set.

<!-- registry:gate-codes -->
| code | status | planned by | meaning |
| --- | --- | --- | --- |
| `gate.assessment_missing` | shipped | E5.3 | required valid complete deterministic or semantic assessment is absent |
| `gate.provider_failed_no_fallback` | shipped | E5.3 | required semantic provider failed and no eligible fallback completed |
| `gate.semantic_invalid` | shipped | E5.3 | semantic assessment present but invalid/incomplete |
| `gate.proposal_missing` | shipped | E5.3 | materially affected finding without a proposal or accepted no-change disposition |
| `gate.proposal_hash_mismatch` | shipped | E5.3 | the proposal record's declared digest does not match the digest re-derived from delivered proposal content; integrity failure, not an approved-vs-current semantic change — when both facts are present, `gate.approval_invalidated` takes precedence and this code is not emitted |
| `gate.approval_invalidated` | shipped | E5.3 | gate-time surface that consumes the `approval.invalidated_proposal_changed` outcome; distinct code, not a respelling; semantic content change invalidated a prior approval |
| `gate.approval_missing` | shipped | E5.3 | `approval_required` without a valid current approval |
| `gate.assessment_stale` | shipped | E5.3 | assessment head binding does not match the evaluated head SHA |
| `gate.promotion_unapproved` | shipped | E5.3 | an authoritative lifecycle promotion lacks valid current approval or emergency authority |
| `gate.cloud_unavailable` | shipped | E5.3 | required Cloud decision input unavailable — blocks, never defaults |
| `gate.audit_persistence_failed` | shipped | E5.3 | decision audit record could not be persisted — blocks; gate-level surface consuming the operation-level `audit.persistence_failed` (E1.4.T4), explicit mapping per the note above the diagnostic-codes table |
| `gate.mode_unknown` | shipped | E5.3 | unknown gate mode string is a configuration error, never a fallback |
| `gate.check_publish_failed` | planned | E5.4 | required check could not publish; blocks by absence, recorded for diagnosability |
<!-- /registry:gate-codes -->

## Permission primitives — planned, owner `cloud`

The immutable version-1 permission vocabulary implemented by E2.2. Policy evaluates these primitives, never role names; changing a primitive's meaning requires a new registry version.

<!-- registry:permission-primitives -->
| permission | registry version | status | planned by |
| --- | --- | --- | --- |
| `audit.export` | 1 | planned | E2.2 |
| `audit.read` | 1 | planned | E2.2 |
| `connector.configure` | 1 | planned | E2.2 |
| `connector.create` | 1 | planned | E2.2 |
| `connector.delete` | 1 | planned | E2.2 |
| `connector.read` | 1 | planned | E2.2 |
| `knowledge.declassify` | 1 | planned | E2.2 |
| `knowledge.propose` | 1 | planned | E2.2 |
| `knowledge.read` | 1 | planned | E2.2 |
| `migration.approve` | 1 | planned | E2.2 |
| `migration.execute` | 1 | planned | E2.2 |
| `obligation.read` | 1 | planned | E2.2 |
| `obligation.satisfy` | 1 | planned | E2.2 |
| `obligation.waive` | 1 | planned | E2.2 |
| `policy.manage` | 1 | planned | E2.2 |
| `policy.read` | 1 | planned | E2.2 |
| `proposal.approve` | 1 | planned | E2.2 |
| `proposal.edit` | 1 | planned | E2.2 |
| `proposal.read` | 1 | planned | E2.2 |
| `proposal.reject` | 1 | planned | E2.2 |
| `proposal.review` | 1 | planned | E2.2 |
| `semantic_executor.configure` | 1 | planned | E2.2 |
| `semantic_executor.qualify` | 1 | planned | E2.2 |
| `semantic_executor.read` | 1 | planned | E2.2 |
| `source.manage` | 1 | planned | E2.2 |
| `source.read` | 1 | planned | E2.2 |
| `source.sync` | 1 | planned | E2.2 |
| `workspace.configure` | 1 | planned | E2.2 |
| `workspace.manage_members` | 1 | planned | E2.2 |
| `workspace.read` | 1 | planned | E2.2 |
<!-- /registry:permission-primitives -->

## Native permission additions - vocabulary version 2, owner `cloud`

Version 2 is the immutable version-1 set plus this addition. Introduction version
never changes an existing primitive. The addition is evaluated by current native
scoped grants and retained in private dispatch facts; it is not a member of frozen
`adoc.authorization_decision.v0`. No built-in role receives it. A public shared
decision for this permission requires a registered successor before emission.

<!-- registry:permission-primitives-v2-additions -->
| permission | registry version | status | introduced by |
| --- | --- | --- | --- |
| `source.writeback` | 2 | planned | E6.5.T2: outbound write to an explicitly enrolled source target; current identity/delegation, scoped grant, provider write ceiling and exact qualified adapter required; source.sync/manage confer no implicit writeback grant |
<!-- /registry:permission-primitives-v2-additions -->

## External group binding modes — planned, owner `cloud`

The complete E2.4 external-binding state vocabulary from `AUTHORIZATION.md` §A7. Only rows marked `yes` can confer a grant and therefore appear in authorization-decision provenance.

<!-- registry:group-binding-modes -->
| mode | status | planned by | confers grant | meaning |
| --- | --- | --- | --- | --- |
| `authoritative_sync` | planned | E2.4 | yes | external membership is authoritative for the binding epoch |
| `additive_sync` | planned | E2.4 | yes | external membership adds to manual membership for the binding epoch |
| `suggestion_only` | planned | E2.4 | no | external membership is advisory and never grants authorization |
| `disabled` | planned | E2.4 | no | the binding is inactive and never grants authorization |
<!-- /registry:group-binding-modes -->

## External group source kinds — planned, owner `cloud`

The closed E2.4 `source_kind` vocabulary carried by an external AgentDoc-group membership in authorization provenance. The sibling `membership_source` discriminator (`manual` / `external`) is structural and governed by the `adoc.authorization_decision.v0` row. `MILESTONES.md` §E2.4 names the OIDC/SCIM category; the contract records its two protocol-specific values separately. `scim_group` is registered for provenance while SCIM sync remains deferred to P4, and a future enterprise-directory adapter requires a new registered value.

<!-- registry:group-source-kinds -->
| source kind | status | planned by | meaning |
| --- | --- | --- | --- |
| `github_team` | planned | E2.4 | GitHub team membership |
| `gitlab_group` | planned | E2.4 | GitLab group membership |
| `slack_user_group` | planned | E2.4 | Slack user-group membership |
| `oidc_group` | planned | E2.4 | the source kind is claim-only in V1 and valid only for a human principal; group claim comes from a freshly issued and verified ID token, refreshes per principal at authentication with no out-of-band lookup or sweep, retains token issuance, validation/ingestion-commit, and session-expiry instants, requires the decision principal to identify that exact session, and confers no later than the cited identity session's expiry |
| `scim_group` | planned | E2.4 | SCIM group membership |
<!-- /registry:group-source-kinds -->

## Group membership-unavailability states — planned, owner `cloud`

The closed E2.4 state vocabulary retained when an authorization decision cannot establish a potentially grant-conferring membership input. Each state identifies an immutable record through `state_record_id`; source compatibility is schema-enforced.

<!-- registry:group-membership-unavailability-states -->
| state | status | planned by | compatible input | meaning |
| --- | --- | --- | --- | --- |
| `lifecycle_unavailable` | planned | E2.4 | manual membership | the manual-membership lifecycle read failed or remained unresolved |
| `observation_expired` | planned | E2.4 | connector or OIDC external membership | the retained positive observation reached its effective freshness or identity-session deadline; OIDC evidence cites that observation's exact historical identity session on the unavailable entry |
| `connector_read_failed` | planned | E2.4 | connector-read external membership | the retained current-state connector read failed |
| `link_read_pending` | planned | E2.4 | connector-read external membership | the retained post-link or post-relink binding read is pending |
| `link_read_failed` | planned | E2.4 | connector-read external membership | the retained post-link or post-relink binding read failed |
| `epoch_observation_pending` | planned | E2.4 | connector-read external membership | a requested grant-conferring transition is awaiting its sweep while the prior effective mode epoch, including `suggestion_only` or `disabled`, remains in force |
| `oidc_authentication_pending` | planned | E2.4 | claim-only OIDC external membership | a validated grant-conferring OIDC epoch is awaiting the principal's next authentication; the unavailable entry cites no session of its own |
<!-- /registry:group-membership-unavailability-states -->

## Cloud codes — owner `cloud`

Operation labels and typed failure codes owned by the private Cloud service. New Cloud wire codes register here before they ship: `workspace.bootstrap` names the identity-bootstrap operation, the E2.1 `workspace.*` failures cover repository registration and tenant isolation, `governance.decision_binding_missing` belongs to the E1.3 reconciliation-decision route, and Cloud-owned operational `gate.*` codes belong here rather than in the closed gate-result reason set.

The pre-release implementation annotations mark only the ten code rows added by the E5 inventory correction, verified against Cloud commit `53f4c3e`; their absence on another planned row does not mean that contract is unimplemented.

<!-- registry:cloud-codes -->
| code | status | meaning |
| --- | --- | --- |
| `workspace.bootstrap` | planned (in-flight scaffold work) | identity-bootstrap ledger operation label recorded when a workspace is created |
| `workspace.repository_limit_reached` | planned (E2.1) | repository registration would exceed the Workspace's repository limit |
| `workspace.duplicate_repository` | planned (E2.1) | the same external repository is already registered in the Workspace |
| `workspace.cross_tenant_denied` | planned (E2.1) | a Workspace-scoped operation is outside the authenticated principal's memberships; a nonexistent target emits this same code, so foreign and nonexistent targets remain indistinguishable to the caller |
| `governance.decision_binding_missing` | planned (E1.3, in-flight cloud cut) | Cloud reconciliation-decision route rejects a record whose subject/counterpart exact version binding or policy version is missing or padded (deny-by-default; MILESTONES §E1.3.T4); the principal binding is never client-supplied — the store binds the authenticated session, and absent authority context maps to the `insufficient_context` outcome value, envelope-governed vocabulary rather than a standalone code; E1.4 widens Cloud's contract-scan grep to the `governance.` family |
| `governance.proposal_invalid` | planned (E5.1) | Cloud proposal-command route rejects an `adoc.proposal.v0` payload that fails the pinned schema or graph-independent patch rules, whose patch or proposal-set digests do not re-derive from the embedded patch bytes, whose patches are out of canonical order, whose create placement or per-target sequence violates ADR-0062 §6–§7, or that would mint authority (ADR-0053 §2–§3, ADR-0054 §3 floors); remediation names the failing rule or derivation |
| `governance.proposal_conflict` | planned (E5.1) | a proposal command names a proposal-set digest already recorded in the Workspace whose stored digest-ordered patch set differs from the delivered one (a re-derivation guard, unreachable without tampering or a digest collision); the stored record is immutable and the delivery is rejected, never merged. Bindings and per-patch finding/placement metadata are not digest-covered (ADR-0062 §2): a same-digest delivery that differs only there — a source-placement move or a new head on the same change request — is acknowledged as `ingest.duplicate_delivery` and the first-stored record stays authoritative (MILESTONES §E5.1: idempotent by proposal-set digest, one record) |
| `governance.approved_promotion_invalid` | planned (E5.5.T1, implemented in Cloud v0.1.0 pre-release) | promotion-table validation rejects inconsistent proposal/approval/candidate/validation/Governance Event bindings or receipt bytes that differ from the v0 builder; consumed by the inserting promotion operation; no invalid causal receipt is stored |
| `governance.promotion_not_authorized` | planned (E5.5.T1, implemented in Cloud v0.1.0 pre-release) | `promote_approved_proposal_candidate` refuses missing or mismatched proposal lineage/candidate authority, non-current approval or authorization/ACL/runtime admission, or blocking obligations; returned to the promotion caller before activation |
| `governance.trace_incomplete` | planned (E5.5.T3, implemented in Cloud v0.1.0 pre-release) | `archive_internal_integrated_trace` cannot select exactly one complete linked trace for the requested publication receipt; returned to the archival caller without creating an archive |
| `gate.check_publish_prepare_failed` | planned (E5.4, implemented in Cloud v0.1.0 pre-release) | `publishGitHubCheck` logs this fixed operational code and stops when the assessment- or approval-triggered publication-preparation RPC errors or throws; consumed by Cloud operator logs, not an `adoc.gate_result.v0` reason |
| `gate.check_publish_context_invalid` | planned (E5.4, implemented in Cloud v0.1.0 pre-release) | `publishGitHubCheck` logs this fixed operational code and stops for a malformed preparation response, unexpected status, or invalid ready context; expected `stale` / `already_published` responses exit silently; consumed by Cloud operator logs, not a gate-result reason |
| `gate.check_render_failed` | planned (E5.4, implemented in Cloud v0.1.0 pre-release) | `publishGitHubCheck` logs this fixed operational code and stops when `renderGitHubCheck` throws before provider publication; consumed by Cloud operator logs, not a gate-result reason |
| `gate.check_decision_head_mismatch` | planned (E5.4, implemented in Cloud v0.1.0 pre-release) | `publishGitHubCheck` logs this fixed operational code and stops when the rendered check head differs from the prepared publication head; consumed by Cloud operator logs, not a gate-result reason |
| `gate.check_publish_failure_record_failed` | planned (E5.4.T5, implemented in Cloud v0.1.0 pre-release) | `recordFailure` logs this fixed operational code when the publication-failure recording RPC errors, throws, or does not return `recorded`; consumed by Cloud operator logs and does not claim durable failure evidence or successful publication |
| `gate.check_publish_receipt_failed` | planned (E5.4, implemented in Cloud v0.1.0 pre-release) | `publishGitHubCheck` logs this fixed operational code after the provider path when receipt recording errors/throws or returns an unexpected status, disposition, or check-run ID; consumed by Cloud operator logs; a provider check may exist without a confirmed Cloud receipt |
| `gate.negative_verdict_unrenderable` | planned (E5.4.T2, implemented in Cloud v0.1.0 pre-release) | `github_negative_verdict_projection` rejects an all-`no_change_required` verdict whose deterministic completeness, changed-path count, available knowledge-scope digests, or semantic validation cannot support the rendered claim; includes counts beyond the JavaScript safe-integer limit; refusal aborts publication preparation, not a gate-result reason |
| `approval.ineligible_approver` | planned (E5.2) | authenticated principal is not eligible to approve the proposal under the current authorization and independence policy |
| `approval.proposal_hash_mismatch` | planned (E5.2) | approval command does not bind the exact current proposal digest, so no approval record is created; distinct from `gate.proposal_hash_mismatch`, a gate-time stored-proposal integrity check comparing its declared digest with the digest re-derived from delivered content, never a command rejection |
| `approval.scope_mismatch` | planned (E5.2) | approval command's declared object scope does not exact-match the proposal's complete affected-object `(object_id, content_hash)` tuple set for edited existing objects plus its `create_object` targets (object identifiers only; no exact-head hash by construction) |
| `approval.policy_version_stale` | planned (E5.2) | approval command binds a policy version other than the current version evaluated by Cloud |
| `approval.invalidated_proposal_changed` | planned (E5.2) | approval-record counterpart of `gate.approval_invalidated`: semantic proposal content changed after approval, so the approval is invalidated monotonically |
| `approval.concurrent_write_rejected` | planned (E5.2) | expected approval record version or digest does not match the current record, preventing last-write-wins |
| `approval.model_identity_rejected` | planned (E5.2) | provider or model identity was presented as an approver principal and is ineligible for approval |
| `authority.scope_invalid` | planned (E4.3) | connector-authority scope is malformed, references a nonexistent connector, or skips a hierarchy level |
| `authority.policy_change_not_authorized` | planned (E4.3) | the authenticated principal lacks current `connector.configure` authority for the exact policy scope |
| `authority.policy_missing` | planned (E4.3) | no committed connector-authority policy applies to the requested exact scope |
| `authority.mode_unknown` | planned (E4.3) | requested authority mode is outside the closed five-mode vocabulary |
| `authority.promotion_rule_invalid` | planned (E4.3) | promotion rule does not exactly match the selected authority mode |
| `authority.candidate_rejected` | planned (E4.3) | the effective authority mode forbids that AgentDoc or connector proposal origin |
| `authority.source_assertion_required` | planned (E4.3) | connector proposal lacks the exact Source Assertion for its Source Record; the candidate-authority record independently binds the authenticated submitting principal |
| `authority.candidate_origin_conflict` | planned (E4.3) | an idempotent candidate replay attempts to change any immutable candidate-authority binding: managed candidate version ID and content digest, authenticated submitting principal, current allow authorization decision, policy receipt, effect ordinal, and effect transaction ID, candidate-operation ordinal and transaction ID, authority mode, proposal origin, or conditional Source Assertion |
| `authority.promotion_rejected` | planned (E4.3) | Cloud governance attempted to promote a candidate origin forbidden by the effective authority mode |
| `authority.external_active_rejected` | planned (E4.3) | external activation was attempted outside `externally_canonical` authority |
| `authority.attestation_invalid` | planned (E4.3) | external-promotion attestation digest, immutable Source Assertion, independent trusted issuer evidence, or exact candidate/policy/issuer/type binding is invalid |
| `egress.policy_unknown_category` | planned (E6.6) | egress policy names a category outside the closed seven-category vocabulary; reject structurally, never ignore the key |
| `egress.policy_unavailable` | planned (E6.6) | policy fetch fails or the latest stored policy is missing or invalid; disable all categories with a visible error, never reuse a cached wider policy |
| `egress.payload_rejected` | planned (E6.6.T2) | Cloud ingestion refuses payload bytes in a category disabled by the current egress policy as defense-in-depth behind sender filtering; prohibited bytes are not stored, a content-free audit record preserves the refusal, and every production firing is triaged as a defect |
| `egress.category_disabled` | planned (E6.6.T3) | status carried by the Cloud record in place of content the sender skips transmitting for a disabled category; local deterministic and semantic assessment still run; never render this condition as assessment failure or as coverage |
| `egress.policy_gate_conflict` | planned (E6.6.T3) | proposed gate-mode or egress policy configuration would prevent a required gate from receiving its required inputs; reject the configuration write with remediation to change policy or gate mode explicitly, never silently downgrade the gate |
| `api.unauthenticated` | planned (E4.4) | `/api/v1` request lacks a verifiable bearer identity |
| `api.invalid_request` | planned (E4.4) | `/api/v1` request fails the transport generation's closed structural requirements |
| `api.idempotency_conflict` | planned (E4.4) | an idempotency key is replayed with different request bytes |
| `api.internal_error` | planned (E4.4) | the external API could not durably complete its transport-level operation; request correlation is returned for support |
| `api.rate_limited` | planned (E4.4) | a rate limit rejects an external request and carries standard `Retry-After` semantics; quota enforcement remains E8.7-owned |
| `ingest.envelope_version_unsupported` | planned (E4.4) | assessment submission names an unknown or superseded exact operation/envelope version; remediation names the accepted version |
| `ingest.duplicate_delivery` | planned (E4.6) | a byte-identical replay of an already-complete assessment delivery (E4.6), or a proposal command whose proposal-set digest is already recorded with the same patch set (E5.1; bindings and placement metadata may differ), is acknowledged without another ingestion record, activation event, or latest-state mutation; redelivery may complete an explicitly partial record |
| `ingest.stale_run` | planned (E4.6) | an older observed head is retained in history but cannot replace the repository's newer lineage head |
| `ingest.digest_mismatch` | planned (E4.6) | a claimed assessment or receipt digest does not match the submitted envelope bytes; the attempt is recorded and the delivery rejected |
| `connect.permission_exceeds_manifest` | planned (E4.6) | a connector grant exceeds its authenticated capability manifest; the connection becomes unhealthy and ingestion pauses until re-consent |
| `connect.unknown_config_field` | planned (E7.3) | strict settings parsing rejects an unknown field instead of silently accepting it |
| `connect.credential_store_violation` | planned (E7.3) | a service identity attempts to read both semantic-provider and source/write credential stores |
| `connector.manifest_invalid` | planned (E4.5) | connector capability manifest fails exact-version decoding or its adapter, connector, or authenticated-publisher binding is invalid |
| `connector.publisher_unqualified` | planned (E4.5) | the authenticated publisher lacks trusted qualification for a claimed capability maturity; customer publishers cannot self-claim AgentDoc GA |
| `connector.configuration_ineligible` | planned (E4.5) | a connector configuration cannot activate because its dependency closure, capability maturity, or new-use deprecation policy is unsatisfied; remediation carries eligible alternatives rather than weakening the requested gate |
| `connector.capability_ineligible` | planned (E4.5) | runtime policy rejects a required capability after applying its current per-capability maturity, any immediate incident demotion/suspension, and a current scoped exception |
| `connector.exception_invalid` | planned (E4.5) | a maturity exception is missing immutable scope, expiry, current permission approval, or receipt binding, or attempts to authorize after expiry |
| `delivery.reference_missing` | planned (E8.2) | Cloud rejects a proposal record whose Action-owned knowledge-PR reference block is absent or incomplete |
| `delivery.reference_stale` | planned (E8.2) | Cloud rejects a proposal record whose reference block is bound to a superseded assessment head |
| `writeback.not_authorized` | planned (E6.5.T2) | current authenticated scoped writeback, target enrollment, provider ceiling or workload authority is unavailable; no send permit |
| `writeback.precondition_failed` | planned (E6.5.T2) | exact retained target revision cannot be applied; never overwrite or rebase onto the current revision |
| `writeback.outcome_unknown` | planned (E6.5.T2) | a claimed attempt has no confirmed outcome; read-only recovery only, never automatic resend |
| `writeback.provider_unavailable` | planned (E6.5.T2) | current target-scoped provider credential or observation is unavailable; sanitized refusal without fallback |
| `writeback.own_projection` | planned (E6.5.T3) | legacy UUID candidate admission reports exact equivalent own-projection suppression after current authorization; use structured reconciliation to retain an observation in the same transaction; no candidate UUID or synchronization claim |
| `writeback.applied` | planned (E6.5.T2) | exact prepared target commit and payload observed; no approval, verification or effectivity claim |
| `privacy.deletion_incomplete` | planned (E6.6.T4) | selected source evidence deletion retains unprotected residue or cannot verify the required copy scope; no completed-erasure claim; an authorized fresh retry may complete the sweep while prior outcomes remain immutable |
| `privacy.export_selection_changed` | planned (E6.6.T5) | frozen export evidence or current access changed; require a fresh request and release no archive bytes; legitimate deletion is not corruption |
| `privacy.export_limit_exceeded` | planned (E6.6.T5) | synchronous export exceeds the descriptor, decoded-byte, projection or archive bound; refuse explicitly without truncating a successful bundle |
| `privacy.export_unavailable` | planned (E6.6.T5) | required export storage, locking, transport or auditing is unavailable; release no archive and expose no retained source bytes in diagnostics |
| `privacy.export_invalid_envelope` | planned (E6.6.T5) | authorized retained JSON/object is malformed or its embedded self-digest binding is invalid; preserve the same omission fields with actual canonical-domain observed_digest when computable (possibly equal to claimed_digest), otherwise null; never substitute file SHA. Existing privacy.export_digest_mismatch continues to mean a computed non-null domain digest different from the claimed digest |
| `migration.illegal_transition` | planned (E7.2.T1) | syntactically valid but unavailable/illegal migration state edge or unmet native evidence precondition; no state or authority advance |
<!-- /registry:cloud-codes -->

## Attestation codes — planned, owner `cloud`

The canonical bot-attestation code family root (RT-21, E0.3.T3). The Action never mints its own bot-attestation family: its check surface wraps this code via the registered `action.attestation_bot_rejected` row above. The E8.1 attestation record contract and the sibling codes (`attestation.binding_mismatch`, `attestation.requirements_unmet`) register at E8.1.T1 as a registry edit, flipping this row from planned to implemented rather than re-registering it.

<!-- registry:attestation-codes -->
| code | status | planned by | meaning |
| --- | --- | --- | --- |
| `attestation.bot_approver_rejected` | planned | E8.1 | bot/service approver rejected by default for approval attestation |
<!-- /registry:attestation-codes -->

## Dispositions

Codes and contract ids resolved out of existence (RT-21). A disposition is permanent: the id is never reused with another meaning.

<!-- registry:dispositions -->
| code or id | disposition |
| --- | --- |
| `action.semantic_failed` | removed — appeared only in pre-V10 planning text and never shipped (no occurrence in Action v2.0.0-alpha.19 sources); the shipped registered code `action.semantic_review_failed` is the single canonical Action semantic-failure reason code, and any gate-matrix or planning reference resolves there |
<!-- /registry:dispositions -->

## Untrusted-change states — owner `adoc` contracts, produced by `action`/`cloud`

The closed S8 state vocabulary for the base-controlled trusted workflow ([`SEMANTICS.md §S8`](SEMANTICS.md#s8-base-controlled-trusted-workflow-for-untrusted-changes), E3.8). A new state is a registry edit plus an S8 amendment, never an ad hoc string.

<!-- registry:untrusted-change-states -->
| state |
| --- |
| `not_required` |
| `awaiting_authorization` |
| `authorized` |
| `running` |
| `completed` |
| `denied` |
| `failed` |
| `expired_after_head_change` |
<!-- /registry:untrusted-change-states -->

## Source retention classes — owner `adoc` contracts, enforced by `cloud`

The closed K9 retention-class vocabulary ([`KNOWLEDGE-MODEL.md §K9`](KNOWLEDGE-MODEL.md#k9-policy-driven-layered-source-retention), E6.6). Full source mirroring stays exceptional and disabled by default.

<!-- registry:retention-classes -->
| class |
| --- |
| `digest_only` |
| `bounded_evidence` |
| `exact_candidate_input` |
| `temporary_processing` |
| `full_source_snapshot` |
<!-- /registry:retention-classes -->

## Replay postures — owner `adoc` contracts, recorded by `cloud`

The closed K9 replay-posture vocabulary. A digest-only record is never `fully_replayable`; deleting retained evidence appends a deletion/tombstone event and updates the posture without rewriting governance history.

<!-- registry:replay-postures -->
| posture |
| --- |
| `fully_replayable` |
| `source_access_required` |
| `intentionally_non_replayable` |
| `no_longer_replayable_after_deletion` |
<!-- /registry:replay-postures -->

## Managed state dimensions — owner `adoc` contracts, recorded by `cloud`

The closed six-dimension managed state vocabularies ([`KNOWLEDGE-MODEL.md §K4`](KNOWLEDGE-MODEL.md#k4-governance-effectivity-and-synchronization-are-separate), E1.4). Entries are dimension-qualified (`dimension.state`) so the six vocabularies stay separate in one table — dimensions are never conflated (D07/D15), and a value's spelling is scoped to its own dimension. Synchronization is always per connector, and every synchronization event also carries the boolean `required_before_effective` (§K4). A new value is a registry edit plus a §K4 amendment, never an ad hoc string.

<!-- registry:managed-state-dimensions -->
| dimension.state |
| --- |
| `governance.proposed` |
| `governance.approved` |
| `governance.rejected` |
| `governance.revoked` |
| `verification.unverified` |
| `verification.partially_verified` |
| `verification.verified` |
| `verification.failed` |
| `effectivity.pending` |
| `effectivity.scheduled` |
| `effectivity.effective` |
| `effectivity.suspended` |
| `effectivity.expired` |
| `freshness.current` |
| `freshness.needs_review` |
| `freshness.stale` |
| `integrity.clear` |
| `integrity.potentially_conflicting` |
| `integrity.contradicted` |
| `synchronization.in_sync` |
| `synchronization.pending_writeback` |
| `synchronization.pending_external_approval` |
| `synchronization.writeback_failed` |
| `synchronization.source_ahead` |
| `synchronization.source_diverged` |
| `synchronization.paused` |
| `synchronization.not_applicable` |
<!-- /registry:managed-state-dimensions -->

## Proof obligation states — owner `adoc` contracts

The closed K8 obligation-state vocabulary ([`KNOWLEDGE-MODEL.md §K8`](KNOWLEDGE-MODEL.md#k8-stage-aware-proof-obligations), E1.6), carried by `adoc.proof_obligation.v0`. `waived` is reachable only through an Obligation Waiver record — exact obligation/managed-version-subject/principal/policy bound, justified, and time-bounded where appropriate; a waiver never converts unverified to verified, and an expired waiver reopens its obligation as blocking. A new state is a registry edit plus a §K8 amendment, never an ad hoc string.

<!-- registry:proof-obligation-states -->
| state |
| --- |
| `open` |
| `satisfied` |
| `waived` |
| `failed` |
| `expired` |
<!-- /registry:proof-obligation-states -->

## Proof obligation stages — owner `adoc` contracts

The closed K8 `required_at` stage vocabulary. Whether an obligation is informational or blocking at a stage/risk/action is classification-policy data enclosed by `adoc.proof_obligation.v0`, never code; `approval_required` blocks only obligations explicitly required before gate passage (§K8) — other obligations may block verification, effectivity, synchronization, or high-risk actions instead. A new stage is a registry edit plus a §K8 amendment, never an ad hoc string.

<!-- registry:proof-obligation-stages -->
| stage |
| --- |
| `proposal_validation` |
| `approval` |
| `verification` |
| `effectivity` |
| `connector_synchronization` |
| `agent_action` |
<!-- /registry:proof-obligation-stages -->

## Migration lifecycle states — owner `cloud`

E7.2.T1 publishes the RT-13 state vocabulary. Version1 implemented edges stop at catching_up or explicit failed; later readiness, atomic cutover and rollback require their real operations. These states are separate from Knowledge Object governance dimensions.

<!-- registry:migration-lifecycle-states -->
| state |
| --- |
| `prepared` |
| `snapshot_bound` |
| `importing` |
| `validated` |
| `awaiting_attestation` |
| `catching_up` |
| `ready_to_cutover` |
| `cutover_committed` |
| `rolled_back` |
| `failed` |
<!-- /registry:migration-lifecycle-states -->
