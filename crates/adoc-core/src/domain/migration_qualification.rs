//! Versioned migration eligibility evidence. No attestation or state transition authority.
use super::{
    diagnostic::{Diagnostic, DiagnosticCode},
    graph::{GraphKnowledgeObjectNode, unresolved_contradiction_claim_index},
    lifecycle_mapping::LifecycleMappingContract,
    managed_state::{EffectivityState, GovernanceState},
    migration::{MigrationError, MigrationImportSource},
    source_provenance::SourceBindingCoordinates,
};
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const MIGRATION_QUALIFICATION_SCHEMA_VERSION: &str = "adoc.migration_qualification.v0";
pub const MIGRATION_QUALIFICATION_RECEIPT_SCHEMA_VERSION: &str =
    "adoc.migration_qualification_receipt.v0";
pub const MIGRATION_QUALIFICATION_POLICY_VERSION: &str = "1";
pub(crate) const MIGRATION_LIFECYCLE_MAPPING_VERSION: &str = "1";

pub(crate) fn require_policy(version: &str) -> Result<(), MigrationError> {
    if version == MIGRATION_QUALIFICATION_POLICY_VERSION {
        Ok(())
    } else {
        Err(MigrationError::UnsupportedQualificationPolicy)
    }
}

#[derive(Debug, Default)]
pub(crate) struct QualificationFreshness {
    pub stale: BTreeSet<String>,
    pub review_overdue: BTreeSet<String>,
}
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ReasonCode {
    LifecycleNotAdopted,
    Stale,
    ReviewOverdue,
    Contradicted,
    UnresolvedEvidence,
}
#[derive(Debug, Serialize)]
struct QualificationReason {
    code: ReasonCode,
    related_object_ids: Vec<String>,
    diagnostic_codes: Vec<String>,
}
impl QualificationReason {
    fn simple(code: ReasonCode) -> Self {
        Self {
            code,
            related_object_ids: Vec::new(),
            diagnostic_codes: Vec::new(),
        }
    }
}
#[derive(Debug, Serialize)]
pub(crate) struct QualifiedMigrationObject {
    object_id: String,
    content_hash: String,
    source_path: String,
    object_source_binding: SourceBindingCoordinates,
    source_record_id: String,
    source_binding_id: String,
    eligible: bool,
    reasons: Vec<QualificationReason>,
}

/// Inputs come from the actual full-snapshot validator. Application signal facts
/// enter as domain data; this module never depends on an application/query type.
pub(crate) fn evaluate(
    nodes: &[&GraphKnowledgeObjectNode],
    sources: &[MigrationImportSource],
    freshness: &QualificationFreshness,
    diagnostics: &[Diagnostic],
    version: &str,
) -> Result<Vec<QualifiedMigrationObject>, MigrationError> {
    require_policy(version)?;
    let mapping =
        LifecycleMappingContract::for_mapping_version(MIGRATION_LIFECYCLE_MAPPING_VERSION)
            .map_err(|_| MigrationError::ValidationUnavailable)?;
    let contradictions = unresolved_contradiction_claim_index(nodes.iter().copied());
    let sources: BTreeMap<_, _> = sources.iter().map(|s| (s.path.as_str(), s)).collect();
    let mut evidence_diagnostics: BTreeMap<&str, BTreeSet<String>> = BTreeMap::new();
    for diagnostic in diagnostics {
        if matches!(
            diagnostic.code,
            DiagnosticCode::EvidenceHashDrift
                | DiagnosticCode::EvidenceHashTargetMissing
                | DiagnosticCode::EvidenceHashInvalid
                | DiagnosticCode::EvidenceHashUnverifiable
        ) && let Some(id) = diagnostic.object_id.as_deref()
        {
            evidence_diagnostics
                .entry(id)
                .or_default()
                .insert(diagnostic.code.to_string());
        }
    }
    let mut output = Vec::new();
    for node in nodes {
        let source = sources
            .get(node.source_span.path.as_str())
            .ok_or(MigrationError::InvalidJob)?;
        let binding = node
            .source_binding
            .clone()
            .ok_or(MigrationError::ValidationUnavailable)?;
        let mapped = mapping
            .apply_import_mapping(&node.kind, node.status.as_deref(), None)
            .map_err(|_| MigrationError::ValidationUnavailable)?
            .mapped;
        let mut reasons = Vec::new();
        if mapped.governance != GovernanceState::Approved
            || mapped.effectivity != EffectivityState::Effective
        {
            reasons.push(QualificationReason::simple(ReasonCode::LifecycleNotAdopted));
        }
        if freshness.stale.contains(&node.id) {
            reasons.push(QualificationReason::simple(ReasonCode::Stale));
        }
        if freshness.review_overdue.contains(&node.id) {
            reasons.push(QualificationReason::simple(ReasonCode::ReviewOverdue));
        }
        let related = contradictions.get(&node.id).cloned().unwrap_or_default();
        if node.status.as_deref() == Some("contradicted") || !related.is_empty() {
            reasons.push(QualificationReason {
                code: ReasonCode::Contradicted,
                related_object_ids: related,
                diagnostic_codes: Vec::new(),
            });
        }
        let mut related = BTreeSet::new();
        let mut codes = BTreeSet::new();
        for id in std::iter::once(node.id.as_str())
            .chain(node.evidence.iter().filter_map(|e| e.reference.as_deref()))
        {
            if let Some(found) = evidence_diagnostics.get(id) {
                related.insert(id.to_string());
                codes.extend(found.iter().cloned());
            }
        }
        if !codes.is_empty() {
            reasons.push(QualificationReason {
                code: ReasonCode::UnresolvedEvidence,
                related_object_ids: related.into_iter().collect(),
                diagnostic_codes: codes.into_iter().collect(),
            });
        }
        output.push(QualifiedMigrationObject {
            object_id: node.id.clone(),
            content_hash: node.content_hash.clone(),
            source_path: source.path.clone(),
            object_source_binding: binding,
            source_record_id: source.source_record_id.clone(),
            source_binding_id: source.source_binding_id.clone(),
            eligible: reasons.is_empty(),
            reasons,
        });
    }
    output.sort_by(|a, b| a.object_id.cmp(&b.object_id));
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::graph::{GraphEvidence, GraphRelations, GraphSourceSpan};
    fn knowledge_object(
        id: &str,
        kind: &str,
        status: Option<&str>,
        content_hash: &str,
    ) -> GraphKnowledgeObjectNode {
        GraphKnowledgeObjectNode {
            id: id.to_string(),
            kind: kind.to_string(),
            content_hash: content_hash.to_string(),
            status: status.map(str::to_string),
            severity: None,
            trust: None,
            body: "Refunds require finance approval.".to_string(),
            page_id: "team.page".to_string(),
            source_span: GraphSourceSpan {
                path: "docs/team.adoc".to_string(),
                line: 1,
                column: 1,
            },
            source_binding: Some(SourceBindingCoordinates {
                connector: "local_fs".into(),
                source: "docs/team.adoc".into(),
                revision: None,
                path: "docs/team.adoc".into(),
                anchor: id.into(),
                source_revision_digest: format!("sha256:{}", "a".repeat(64)),
            }),
            visibility: None,
            field_visibility: None,
            fields: BTreeMap::new(),
            relations: GraphRelations::default(),
            impacts: Vec::new(),
            approved_by: Vec::new(),
            allowed_actions: Vec::new(),
            forbidden_actions: Vec::new(),
            contradiction_claims: Vec::new(),
            evidence: Vec::new(),
            effective_status: None,
            effective_reason: None,
            evidence_quality: None,
        }
    }

    fn sources() -> Vec<MigrationImportSource> {
        vec![MigrationImportSource {
            path: "docs/team.adoc".into(),
            source_record_id: "r1".into(),
            source_binding_id: "b1".into(),
        }]
    }
    #[test]
    fn migration_qualification_adopted_lifecycle_is_eligible_without_applying_authority() {
        for (kind, status, eligible) in [
            ("claim", "verified", true),
            ("policy", "active", true),
            ("decision", "accepted", true),
            ("claim", "draft", false),
            ("claim", "custom", false),
            ("policy", "archived", false),
            ("contradiction", "unresolved", false),
            ("task", "done", true),
        ] {
            let node = knowledge_object("test.one", kind, Some(status), "sha256:original");
            let result = evaluate(
                &[&node],
                &sources(),
                &QualificationFreshness::default(),
                &[],
                "1",
            )
            .unwrap();
            assert_eq!(result[0].eligible, eligible, "{kind}/{status}");
            assert_eq!(result[0].content_hash, "sha256:original");
            assert_eq!(
                result[0].object_source_binding,
                node.source_binding.clone().unwrap()
            );
            assert_eq!(node.status.as_deref(), Some(status));
        }
    }
    #[test]
    fn migration_qualification_keeps_all_reasons_despite_stale_projection_precedence() {
        let mut claim =
            knowledge_object("test.claim", "claim", Some("verified"), "sha256:original");
        claim.effective_status = Some("stale".into());
        let mut contradiction = knowledge_object(
            "test.contradiction",
            "contradiction",
            Some("unresolved"),
            "sha256:other",
        );
        contradiction.contradiction_claims = vec![claim.id.clone()];
        let freshness = QualificationFreshness {
            stale: [claim.id.clone()].into(),
            review_overdue: BTreeSet::new(),
        };
        let result = evaluate(&[&contradiction, &claim], &sources(), &freshness, &[], "1").unwrap();
        assert_eq!(result[0].object_id, claim.id);
        assert!(!result[0].eligible);
        assert_eq!(
            result[0]
                .reasons
                .iter()
                .map(|r| &r.code)
                .collect::<Vec<_>>(),
            vec![&ReasonCode::Stale, &ReasonCode::Contradicted]
        );
        assert_eq!(
            result[0].reasons[1].related_object_ids,
            vec![contradiction.id]
        );
    }
    #[test]
    fn migration_qualification_native_evidence_uncertainty_propagates_only_to_references() {
        let mut claim =
            knowledge_object("test.claim", "claim", Some("verified"), "sha256:original");
        claim.evidence = vec![GraphEvidence::object_ref("source_code", "test.source")];
        let evidence = Diagnostic::warning(DiagnosticCode::EvidenceHashDrift, "changed")
            .with_object_id("test.source");
        let result = evaluate(
            &[&claim],
            &sources(),
            &QualificationFreshness::default(),
            &[evidence.clone(), evidence],
            "1",
        )
        .unwrap();
        assert!(!result[0].eligible);
        assert_eq!(result[0].reasons[0].related_object_ids, vec!["test.source"]);
        assert_eq!(
            result[0].reasons[0].diagnostic_codes,
            vec!["evidence.hash_drift"]
        );
        let advisory = Diagnostic::warning(DiagnosticCode::ClaimEvidenceQualityLow, "advisory")
            .with_object_id("test.claim");
        assert!(
            evaluate(
                &[&claim],
                &sources(),
                &QualificationFreshness::default(),
                &[advisory],
                "1"
            )
            .unwrap()[0]
                .eligible
        );
    }
    #[test]
    fn migration_qualification_refuses_unknown_version_and_missing_source_binding() {
        let mut node = knowledge_object("test.claim", "claim", Some("verified"), "sha256:original");
        for version in ["", "2", " 1", "1 "] {
            assert!(
                evaluate(
                    &[&node],
                    &sources(),
                    &QualificationFreshness::default(),
                    &[],
                    version
                )
                .is_err()
            );
        }
        node.source_binding = None;
        assert!(
            evaluate(
                &[&node],
                &sources(),
                &QualificationFreshness::default(),
                &[],
                "1"
            )
            .is_err()
        );
    }
}
