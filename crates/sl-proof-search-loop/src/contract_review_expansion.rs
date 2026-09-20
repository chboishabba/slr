//! Reviewed legal receipts -> Australian contracts adaptive expansion.
//!
//! This compiler is deliberately narrow.  Review receipts may append candidate
//! treatment/support edges to an existing contracts trace, but may not invent
//! authority identities, promote legal truth, or collapse contested/context
//! review into positive doctrine.

use serde::Serialize;
use std::collections::BTreeMap;
use sensiblaw_governed_legal_provider::OalcResolvedSourceReceipt;
use sensiblaw_legal_follow_plan::{
    AuthorityLevel, AustralianContractTrace, ContractDoctrine, ContractLandscapeExpansionDelta,
    ContractTraceEdge, ContractTraceNode, SourceRole, TraceNodeKind, TreatmentKind,
};

use crate::reasoning::CitationUse;
use crate::review_unit_review::ReviewedCitationReviewUnitReceipt;
use crate::waltons_proposition_review::{
    EstoppelRequirementRole, PropositionEvidenceDisposition,
    ReviewedWaltonsPropositionEvidenceReceipt,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedContractAuthorityIdentity {
    pub semantic_ref: String,
    pub label: String,
    pub doctrine: Option<ContractDoctrine>,
    pub jurisdiction_ref: String,
    pub court_ref: Option<String>,
    pub source_role: SourceRole,
    pub authority_level: AuthorityLevel,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub source_receipt: OalcResolvedSourceReceipt,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContractPropositionDisposition {
    Supports,
    Contests,
    ContextOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedContractPropositionReceipt {
    pub review_ref: String,
    pub authority_ref: String,
    pub target_ref: String,
    pub proposition_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub disposition: ContractPropositionDisposition,
    pub reviewer_ref: String,
    pub evidence_refs: Vec<String>,
    pub evidence_coordinate_paid: bool,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ContractReviewedHopResidualKind {
    PropositionContested,
    PropositionContextOnly,
    SupportingPropositionUnpaid,
    MissingAuthorityIdentity,
    MissingRequirementIdentity,
    UnsupportedCitationUse,
    MissingTreatmentIdentity,
    ReceiptPromotedAuthority,
    SourceIdentityReviewInvalid,
    SourceIdentityConflict,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ContractReviewedHopResidual {
    pub residual_ref: String,
    pub kind: ContractReviewedHopResidualKind,
    pub source_receipt_ref: String,
    pub semantic_ref: Option<String>,
    pub related_ref: Option<String>,
    pub reviewer_ref: String,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContractReviewedHopCompilation {
    pub deltas: Vec<ContractLandscapeExpansionDelta>,
    pub residuals: Vec<ContractReviewedHopResidual>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_current_law_conclusion: bool,
}

fn embedded_medium_neutral_citation(value: &str) -> Option<String> {
    let fields = value.split_whitespace().collect::<Vec<_>>();
    for window in fields.windows(3) {
        let year = window[0];
        if year.len() != 6
            || !year.starts_with('[')
            || !year.ends_with(']')
            || !year[1..5].bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        let court = window[1].trim_matches(|ch: char| !ch.is_ascii_alphanumeric());
        let number = window[2].trim_matches(|ch: char| !ch.is_ascii_digit());
        if court.is_empty()
            || !court.bytes().all(|byte| byte.is_ascii_alphanumeric())
            || number.is_empty()
            || !number.bytes().all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        return Some(format!("{year} {court} {number}"));
    }
    None
}

fn waltons_requirement_ref(role: EstoppelRequirementRole) -> &'static str {
    role.requirement_ref()
}

fn treatment_kind(use_: CitationUse) -> Option<TreatmentKind> {
    match use_ {
        CitationUse::Applied => Some(TreatmentKind::Applies),
        CitationUse::Followed => Some(TreatmentKind::Follows),
        CitationUse::Distinguished => Some(TreatmentKind::Distinguishes),
        CitationUse::Adopted | CitationUse::ReliedOn => Some(TreatmentKind::Supports),
        CitationUse::Overruled => Some(TreatmentKind::Displaces),
        CitationUse::Mentioned
        | CitationUse::Quoted
        | CitationUse::Criticised
        | CitationUse::Rejected
        | CitationUse::PartySubmission
        | CitationUse::HistoricalBackground
        | CitationUse::Unresolved => None,
    }
}

fn residual(
    kind: ContractReviewedHopResidualKind,
    receipt_ref: impl Into<String>,
    semantic_ref: Option<String>,
    related_ref: Option<String>,
    reviewer_ref: impl Into<String>,
) -> ContractReviewedHopResidual {
    let receipt_ref = receipt_ref.into();
    ContractReviewedHopResidual {
        residual_ref: format!("contracts:reviewed-hop:residual:{receipt_ref}"),
        kind,
        source_receipt_ref: receipt_ref,
        semantic_ref,
        related_ref,
        reviewer_ref: reviewer_ref.into(),
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

pub fn compile_reviewed_authority_identity_to_contract_hop(
    trace: &AustralianContractTrace,
    reviewed: &ReviewedContractAuthorityIdentity,
) -> ContractReviewedHopCompilation {
    let mut deltas = Vec::new();
    let mut residuals = Vec::new();

    let invalid = reviewed.semantic_ref.trim().is_empty()
        || reviewed.label.trim().is_empty()
        || reviewed.jurisdiction_ref.trim().is_empty()
        || reviewed.reviewer_ref.trim().is_empty()
        || reviewed.evidence_refs.is_empty()
        || reviewed.evidence_refs.iter().any(|value| value.trim().is_empty())
        || !reviewed.candidate_only
        || reviewed.creates_legal_authority
        || !reviewed.source_receipt.candidate_only
        || reviewed.source_receipt.creates_legal_authority
        || reviewed.source_receipt.creates_claim_truth;
    if invalid {
        residuals.push(residual(
            ContractReviewedHopResidualKind::SourceIdentityReviewInvalid,
            format!("source-identity:{}", reviewed.semantic_ref),
            Some(reviewed.semantic_ref.clone()),
            None,
            &reviewed.reviewer_ref,
        ));
        return ContractReviewedHopCompilation {
            deltas,
            residuals,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        };
    }

    let kind = match reviewed.source_role {
        SourceRole::PrimaryCaseLaw => TraceNodeKind::CaseAuthority,
        SourceRole::PrimaryLegislation => TraceNodeKind::Legislation,
        _ => {
            residuals.push(residual(
                ContractReviewedHopResidualKind::SourceIdentityReviewInvalid,
                format!("source-identity:{}", reviewed.semantic_ref),
                Some(reviewed.semantic_ref.clone()),
                None,
                &reviewed.reviewer_ref,
            ));
            return ContractReviewedHopCompilation {
                deltas,
                residuals,
                candidate_only: true,
                creates_legal_authority: false,
                creates_current_law_conclusion: false,
            };
        }
    };

    let node = ContractTraceNode {
        semantic_ref: reviewed.semantic_ref.clone(),
        label: reviewed.label.clone(),
        kind,
        doctrine: reviewed.doctrine,
        jurisdiction_ref: reviewed.jurisdiction_ref.clone(),
        court_ref: reviewed.court_ref.clone(),
        decision_or_effective_date: reviewed.source_receipt.date.clone(),
        valid_from: None,
        valid_to: None,
        source_role: reviewed.source_role,
        authority_level: reviewed.authority_level,
        source_citation: reviewed.source_receipt.citation.clone(),
        candidate_only: true,
        creates_legal_authority: false,
    };

    if let Some(existing) = trace.nodes.get(&node.semantic_ref) {
        let court_compatible = existing.court_ref.is_none()
            || reviewed.court_ref.is_none()
            || existing.court_ref == reviewed.court_ref;
        let citation_compatible = embedded_medium_neutral_citation(&existing.source_citation)
            .zip(embedded_medium_neutral_citation(&reviewed.source_receipt.citation))
            .map_or_else(
                || {
                    existing.source_citation.contains(&reviewed.source_receipt.citation)
                        || reviewed
                            .source_receipt
                            .citation
                            .contains(&existing.source_citation)
                },
                |(left, right)| left == right,
            );
        let compatible = existing.kind == node.kind
            && existing.source_role == node.source_role
            && existing.authority_level == node.authority_level
            && existing.jurisdiction_ref == node.jurisdiction_ref
            && court_compatible
            && citation_compatible;
        if !compatible {
            residuals.push(residual(
                ContractReviewedHopResidualKind::SourceIdentityConflict,
                format!("source-identity:{}", reviewed.semantic_ref),
                Some(reviewed.semantic_ref.clone()),
                None,
                &reviewed.reviewer_ref,
            ));
        }
    } else {
        deltas.push(ContractLandscapeExpansionDelta {
            discovered_nodes: vec![node],
            discovered_edges: Vec::new(),
            provenance_ref: format!(
                "reviewed-source-identity:{}:{}",
                reviewed.reviewer_ref, reviewed.source_receipt.version_id
            ),
            candidate_only: true,
            creates_legal_authority: false,
        });
    }

    ContractReviewedHopCompilation {
        deltas,
        residuals,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

pub fn compile_reviewed_contract_proposition_receipts_to_hops(
    trace: &AustralianContractTrace,
    receipts: &[ReviewedContractPropositionReceipt],
) -> ContractReviewedHopCompilation {
    let mut deltas = Vec::new();
    let mut residuals = Vec::new();

    for receipt in receipts {
        if receipt.review_ref.trim().is_empty()
            || receipt.authority_ref.trim().is_empty()
            || receipt.target_ref.trim().is_empty()
            || receipt.proposition_ref.trim().is_empty()
            || receipt.source_revision_ref.trim().is_empty()
            || receipt.span_ref.trim().is_empty()
            || receipt.reviewer_ref.trim().is_empty()
            || receipt.evidence_refs.is_empty()
            || receipt.evidence_refs.iter().any(|value| value.trim().is_empty())
            || !receipt.candidate_only
            || receipt.creates_legal_authority
            || receipt.creates_current_law_conclusion
        {
            residuals.push(residual(
                ContractReviewedHopResidualKind::ReceiptPromotedAuthority,
                &receipt.review_ref,
                Some(receipt.authority_ref.clone()),
                Some(receipt.target_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }

        match receipt.disposition {
            ContractPropositionDisposition::Contests => {
                residuals.push(residual(
                    ContractReviewedHopResidualKind::PropositionContested,
                    &receipt.review_ref,
                    Some(receipt.authority_ref.clone()),
                    Some(receipt.target_ref.clone()),
                    &receipt.reviewer_ref,
                ));
                continue;
            }
            ContractPropositionDisposition::ContextOnly => {
                residuals.push(residual(
                    ContractReviewedHopResidualKind::PropositionContextOnly,
                    &receipt.review_ref,
                    Some(receipt.authority_ref.clone()),
                    Some(receipt.target_ref.clone()),
                    &receipt.reviewer_ref,
                ));
                continue;
            }
            ContractPropositionDisposition::Supports => {}
        }

        if !receipt.evidence_coordinate_paid {
            residuals.push(residual(
                ContractReviewedHopResidualKind::SupportingPropositionUnpaid,
                &receipt.review_ref,
                Some(receipt.authority_ref.clone()),
                Some(receipt.target_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }
        if !trace.nodes.contains_key(&receipt.authority_ref) {
            residuals.push(residual(
                ContractReviewedHopResidualKind::MissingAuthorityIdentity,
                &receipt.review_ref,
                Some(receipt.authority_ref.clone()),
                Some(receipt.target_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }
        if !trace.nodes.contains_key(&receipt.target_ref) {
            residuals.push(residual(
                ContractReviewedHopResidualKind::MissingRequirementIdentity,
                &receipt.review_ref,
                Some(receipt.authority_ref.clone()),
                Some(receipt.target_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }

        deltas.push(ContractLandscapeExpansionDelta {
            discovered_nodes: Vec::new(),
            discovered_edges: vec![ContractTraceEdge {
                from_ref: receipt.authority_ref.clone(),
                to_ref: receipt.target_ref.clone(),
                treatment: TreatmentKind::Supports,
                candidate_only: true,
                creates_legal_authority: false,
            }],
            provenance_ref: receipt.review_ref.clone(),
            candidate_only: true,
            creates_legal_authority: false,
        });
    }

    ContractReviewedHopCompilation {
        deltas,
        residuals,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

pub fn compile_waltons_proposition_receipts_to_contract_hops(
    trace: &AustralianContractTrace,
    authority_ref: &str,
    receipts: &[ReviewedWaltonsPropositionEvidenceReceipt],
) -> ContractReviewedHopCompilation {
    let generic = receipts
        .iter()
        .map(|receipt| ReviewedContractPropositionReceipt {
            review_ref: receipt.review_ref.clone(),
            authority_ref: authority_ref.to_string(),
            target_ref: waltons_requirement_ref(receipt.role).to_string(),
            proposition_ref: receipt.proposition_ref.clone(),
            source_revision_ref: receipt.source_revision_ref.clone(),
            span_ref: receipt.paragraph_locator_ref.clone(),
            disposition: match receipt.disposition {
                PropositionEvidenceDisposition::Supports => ContractPropositionDisposition::Supports,
                PropositionEvidenceDisposition::Contests => ContractPropositionDisposition::Contests,
                PropositionEvidenceDisposition::ContextOnly => {
                    ContractPropositionDisposition::ContextOnly
                }
            },
            reviewer_ref: receipt.reviewer_ref.clone(),
            evidence_refs: receipt.review_evidence_refs.clone(),
            evidence_coordinate_paid: receipt.reviewed_evidence.is_some()
                && receipt.payment_receipt.is_some(),
            candidate_only: receipt.candidate_only,
            creates_legal_authority: receipt.creates_legal_authority,
            creates_current_law_conclusion: receipt.claim_truth_promoted,
        })
        .collect::<Vec<_>>();
    compile_reviewed_contract_proposition_receipts_to_hops(trace, &generic)
}

pub fn compile_treatment_receipts_to_contract_hops(
    trace: &AustralianContractTrace,
    receipts: &[ReviewedCitationReviewUnitReceipt],
) -> ContractReviewedHopCompilation {
    compile_treatment_receipts_to_contract_hops_with_aliases(
        trace,
        receipts,
        &BTreeMap::new(),
    )
}

pub fn compile_treatment_receipts_to_contract_hops_with_aliases(
    trace: &AustralianContractTrace,
    receipts: &[ReviewedCitationReviewUnitReceipt],
    reviewed_document_aliases: &BTreeMap<String, String>,
) -> ContractReviewedHopCompilation {
    let mut deltas = Vec::new();
    let mut residuals = Vec::new();

    for receipt in receipts {
        let edge = &receipt.edge;
        let citing_document_ref = reviewed_document_aliases
            .get(&edge.citing_document_ref)
            .cloned()
            .unwrap_or_else(|| edge.citing_document_ref.clone());
        let cited_document_ref = reviewed_document_aliases
            .get(&edge.cited_document_ref)
            .cloned()
            .unwrap_or_else(|| edge.cited_document_ref.clone());
        if !edge.reviewed || !edge.candidate_only {
            residuals.push(residual(
                ContractReviewedHopResidualKind::ReceiptPromotedAuthority,
                &receipt.review_unit_ref,
                Some(citing_document_ref.clone()),
                Some(cited_document_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }
        let Some(treatment) = treatment_kind(edge.citation_use) else {
            residuals.push(residual(
                ContractReviewedHopResidualKind::UnsupportedCitationUse,
                &receipt.review_unit_ref,
                Some(citing_document_ref.clone()),
                Some(cited_document_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        };
        if !trace.nodes.contains_key(&citing_document_ref)
            || !trace.nodes.contains_key(&cited_document_ref)
        {
            residuals.push(residual(
                ContractReviewedHopResidualKind::MissingTreatmentIdentity,
                &receipt.review_unit_ref,
                Some(citing_document_ref.clone()),
                Some(cited_document_ref.clone()),
                &receipt.reviewer_ref,
            ));
            continue;
        }

        deltas.push(ContractLandscapeExpansionDelta {
            discovered_nodes: Vec::new(),
            discovered_edges: vec![ContractTraceEdge {
                from_ref: citing_document_ref,
                to_ref: cited_document_ref,
                treatment,
                candidate_only: true,
                creates_legal_authority: false,
            }],
            provenance_ref: receipt.review_unit_ref.clone(),
            candidate_only: true,
            creates_legal_authority: false,
        });
    }

    ContractReviewedHopCompilation {
        deltas,
        residuals,
        candidate_only: true,
        creates_legal_authority: false,
        creates_current_law_conclusion: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::{PropositionReasoningEdge, ReasoningRole};
    use sensiblaw_legal_follow_plan::waltons_estoppel_trace;

    fn treatment_receipt(use_: CitationUse) -> ReviewedCitationReviewUnitReceipt {
        ReviewedCitationReviewUnitReceipt {
            review_unit_ref: "review-unit:sidhu-waltons".into(),
            document_ref: "case:au:hca:2014:19".into(),
            source_revision_ref: "source:revision:sidhu".into(),
            canonical_text_sha256: "sha256:fixture".into(),
            citation_text: "[1988] HCA 7".into(),
            citation_locator_refs: vec!["sidhu#citation".into()],
            anchor_paragraph_locator_refs: vec!["sidhu#paragraph".into()],
            selected_anchor_paragraph_locator_ref: "sidhu#paragraph".into(),
            matched_criterion_refs: vec!["criterion:estoppel:treatment".into()],
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["evidence:fixture".into()],
            edge: PropositionReasoningEdge {
                citing_document_ref: "case:au:hca:2014:19".into(),
                citing_proposition_ref: "prop:sidhu:estoppel".into(),
                cited_document_ref: "case:au:hca:1988:7".into(),
                cited_proposition_ref: "prop:waltons:estoppel".into(),
                citation_use: use_,
                reasoning_role: ReasoningRole::Rule,
                condition_coordinates: Vec::new(),
                pinpoint_ref: Some("sidhu#paragraph".into()),
                judge_or_speaker_ref: None,
                court_ref: Some("court:HCA".into()),
                jurisdiction_ref: Some("AU".into()),
                temporal_ref: Some("2014-05-16".into()),
                outcome_ref: None,
                remedy_ref: None,
                burden_refs: Vec::new(),
                exception_refs: Vec::new(),
                lexical_realisation: "reviewed treatment fixture".into(),
                reviewed: true,
                candidate_only: true,
            },
            receipt_authority: "experimental_candidate_only",
        }
    }

    fn source_receipt(citation: &str) -> OalcResolvedSourceReceipt {
        OalcResolvedSourceReceipt {
            demand_ref: "demand:fixture".into(),
            origin_ref: "origin:fixture".into(),
            citation: citation.into(),
            version_id: "version:fixture".into(),
            corpus_revision_ref: "isaacus/open-australian-legal-corpus@fixture".into(),
            source: "high_court_of_australia".into(),
            jurisdiction: "commonwealth".into(),
            document_type: "decision".into(),
            court: Some("court:HCA".into()),
            date: Some("2020-01-01".into()),
            canonical_url: Some("https://example.invalid/fixture".into()),
            when_scraped: Some("2026-09-20".into()),
            canonical_text_digest: "sha256:fixture".into(),
            local_artifact_ref: std::path::PathBuf::from("fixture.txt"),
            temporal_coverage:
                sensiblaw_governed_legal_provider::OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path: "fixture".into(),
            network_requests: 0,
            receipt_authority:
                sensiblaw_governed_legal_provider::OALC_RECEIPT_AUTHORITY.into(),
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        }
    }

    #[test]
    fn generic_reviewed_proposition_compiler_handles_mann_without_waltons_role() {
        let trace = sensiblaw_legal_follow_plan::mann_paterson_trace();
        let receipt = ReviewedContractPropositionReceipt {
            review_ref: "review:mann:repudiation".into(),
            authority_ref: "matter:au:hca:2019:32".into(),
            target_ref: "doctrine:au:contract:repudiation-termination".into(),
            proposition_ref: "prop:mann:repudiation-termination".into(),
            source_revision_ref: "source:mann:revision".into(),
            span_ref: "case:mann#paragraph-1".into(),
            disposition: ContractPropositionDisposition::Supports,
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["evidence:mann:fixture".into()],
            evidence_coordinate_paid: true,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        };
        let compiled =
            compile_reviewed_contract_proposition_receipts_to_hops(&trace, &[receipt]);
        assert_eq!(compiled.deltas.len(), 1);
        assert!(compiled.residuals.is_empty());
        let edge = &compiled.deltas[0].discovered_edges[0];
        assert_eq!(edge.from_ref, "matter:au:hca:2019:32");
        assert_eq!(
            edge.to_ref,
            "doctrine:au:contract:repudiation-termination"
        );
        assert_eq!(edge.treatment, TreatmentKind::Supports);
    }

    #[test]
    fn generic_contested_contract_proposition_remains_residual() {
        let trace = sensiblaw_legal_follow_plan::mann_paterson_trace();
        let receipt = ReviewedContractPropositionReceipt {
            review_ref: "review:mann:contested".into(),
            authority_ref: "matter:au:hca:2019:32".into(),
            target_ref: "doctrine:au:contract:restitution-after-termination".into(),
            proposition_ref: "prop:mann:restitution".into(),
            source_revision_ref: "source:mann:revision".into(),
            span_ref: "case:mann#paragraph-2".into(),
            disposition: ContractPropositionDisposition::Contests,
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["evidence:mann:fixture".into()],
            evidence_coordinate_paid: false,
            candidate_only: true,
            creates_legal_authority: false,
            creates_current_law_conclusion: false,
        };
        let compiled =
            compile_reviewed_contract_proposition_receipts_to_hops(&trace, &[receipt]);
        assert!(compiled.deltas.is_empty());
        assert_eq!(
            compiled.residuals[0].kind,
            ContractReviewedHopResidualKind::PropositionContested
        );
    }

    #[test]
    fn reviewed_source_identity_can_bind_compatible_seeded_authority() {
        let trace = waltons_estoppel_trace();
        let reviewed = ReviewedContractAuthorityIdentity {
            semantic_ref: "case:au:hca:2014:19".into(),
            label: "Sidhu v Van Dyke".into(),
            doctrine: Some(sensiblaw_legal_follow_plan::ContractDoctrine::Estoppel),
            jurisdiction_ref: "AU".into(),
            court_ref: Some("court:HCA".into()),
            source_role: SourceRole::PrimaryCaseLaw,
            authority_level: AuthorityLevel::Official,
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["review-note:fixture".into()],
            source_receipt: source_receipt("Sidhu v Van Dyke [2014] HCA 19"),
            candidate_only: true,
            creates_legal_authority: false,
        };
        let compiled =
            compile_reviewed_authority_identity_to_contract_hop(&trace, &reviewed);
        assert!(compiled.deltas.is_empty());
        assert!(compiled.residuals.is_empty());
    }

    #[test]
    fn reviewed_source_identity_adds_new_candidate_authority_node() {
        let trace = waltons_estoppel_trace();
        let reviewed = ReviewedContractAuthorityIdentity {
            semantic_ref: "case:au:hca:2020:1".into(),
            label: "Later authority fixture".into(),
            doctrine: Some(sensiblaw_legal_follow_plan::ContractDoctrine::Estoppel),
            jurisdiction_ref: "AU".into(),
            court_ref: Some("court:HCA".into()),
            source_role: SourceRole::PrimaryCaseLaw,
            authority_level: AuthorityLevel::Official,
            reviewer_ref: "reviewer:fixture".into(),
            evidence_refs: vec!["review-note:fixture".into()],
            source_receipt: source_receipt("[2020] HCA 1"),
            candidate_only: true,
            creates_legal_authority: false,
        };
        let compiled =
            compile_reviewed_authority_identity_to_contract_hop(&trace, &reviewed);
        assert_eq!(compiled.deltas.len(), 1);
        assert!(compiled.residuals.is_empty());
        let node = &compiled.deltas[0].discovered_nodes[0];
        assert_eq!(node.semantic_ref, "case:au:hca:2020:1");
        assert_eq!(node.kind, TraceNodeKind::CaseAuthority);
        assert!(node.candidate_only);
        assert!(!node.creates_legal_authority);
    }

    #[test]
    fn reviewed_followed_edge_compiles_to_candidate_contract_delta() {
        let trace = waltons_estoppel_trace();
        let compiled =
            compile_treatment_receipts_to_contract_hops(&trace, &[treatment_receipt(CitationUse::Followed)]);
        assert_eq!(compiled.deltas.len(), 1);
        assert!(compiled.residuals.is_empty());
        let edge = &compiled.deltas[0].discovered_edges[0];
        assert_eq!(edge.from_ref, "case:au:hca:2014:19");
        assert_eq!(edge.to_ref, "case:au:hca:1988:7");
        assert_eq!(edge.treatment, TreatmentKind::Follows);
        assert!(!edge.creates_legal_authority);
    }

    #[test]
    fn reviewed_document_alias_resolves_treatment_identity() {
        let mut receipt = treatment_receipt(CitationUse::Followed);
        receipt.edge.citing_document_ref =
            "document:oalc:high_court_of_australia:2014-hca-19".into();
        let aliases = BTreeMap::from([(
            "document:oalc:high_court_of_australia:2014-hca-19".into(),
            "case:au:hca:2014:19".into(),
        )]);
        let trace = waltons_estoppel_trace();
        let compiled = compile_treatment_receipts_to_contract_hops_with_aliases(
            &trace,
            &[receipt],
            &aliases,
        );
        assert_eq!(compiled.deltas.len(), 1);
        assert!(compiled.residuals.is_empty());
        let edge = &compiled.deltas[0].discovered_edges[0];
        assert_eq!(edge.from_ref, "case:au:hca:2014:19");
        assert_eq!(edge.to_ref, "case:au:hca:1988:7");
        assert_eq!(edge.treatment, TreatmentKind::Follows);
    }

    #[test]
    fn mere_mention_remains_residual_not_treatment() {
        let trace = waltons_estoppel_trace();
        let compiled =
            compile_treatment_receipts_to_contract_hops(&trace, &[treatment_receipt(CitationUse::Mentioned)]);
        assert!(compiled.deltas.is_empty());
        assert_eq!(compiled.residuals.len(), 1);
        assert_eq!(
            compiled.residuals[0].kind,
            ContractReviewedHopResidualKind::UnsupportedCitationUse
        );
    }

    #[test]
    fn missing_authority_identity_remains_residual() {
        let mut receipt = treatment_receipt(CitationUse::Applied);
        receipt.edge.citing_document_ref = "case:unknown".into();
        let trace = waltons_estoppel_trace();
        let compiled = compile_treatment_receipts_to_contract_hops(&trace, &[receipt]);
        assert!(compiled.deltas.is_empty());
        assert_eq!(
            compiled.residuals[0].kind,
            ContractReviewedHopResidualKind::MissingTreatmentIdentity
        );
    }
}
