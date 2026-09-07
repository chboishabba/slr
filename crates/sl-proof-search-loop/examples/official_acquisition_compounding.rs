use sensiblaw_governed_legal_provider::{LegalProvider, LocalIngestionReceipt, RECEIPT_AUTHORITY};
use sensiblaw_proof_search_loop::acquisition::{
    acquisition_handoff_is_semantic_payment, handoff_locally_ingested_authority,
};
use sensiblaw_proof_search_loop::frontier::{ProofFrontier, ProofResidual, ResidualStatus};
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ConditionKind, PropositionReasoningEdge, ReasoningGraphDelta,
    ReasoningRole,
};
use sensiblaw_proof_search_loop::transition::{
    apply_assessments, ResidualAssessment, ResidualAssessmentKind, ResearchTermination,
};
use sensiblaw_proof_search_loop::world::ResearchWorldSnapshot;

fn main() {
    let acquisition = LocalIngestionReceipt {
        provider: LegalProvider::HighCourtAustralia,
        source_identity_ref: "case:[2026]-HCA-19".into(),
        source_revision_ref: "source:hca:2026:19:rev:fixture".into(),
        explicit_reference: "https://www.hcourt.gov.au/fixture".into(),
        canonical_bytes_digest: "sha256:official-fixture-bytes".into(),
        locally_ingested: true,
        network_requests_used_to_acquire: 1,
        receipt_authority: RECEIPT_AUTHORITY,
    };

    let mut world = ResearchWorldSnapshot {
        snapshot_ref: "world:pre-official".into(),
        authority: "experimental_candidate_only",
        ..ResearchWorldSnapshot::default()
    };
    let handoff = handoff_locally_ingested_authority(
        &mut world,
        &acquisition,
        "sha256:official-fixture-text",
        "pnf:hca:2026:19:fixture",
        "citations:hca:2026:19:fixture",
        Some("AU".into()),
        "primary-case",
        Some("authority:[2026]-HCA-19".into()),
    )
    .expect("locally ingested official source must join the research world");
    assert!(!acquisition_handoff_is_semantic_payment(&handoff));

    // Synthetic fixture edge: this tests the normal reasoning/world route only.
    // It does not claim that the real HCA judgment contains this exact treatment.
    let delta = ReasoningGraphDelta::from_edges(vec![PropositionReasoningEdge {
        citing_document_ref: "case:[2026]-HCA-19".into(),
        citing_proposition_ref: "prop:fixture-current-treatment".into(),
        cited_document_ref: "authority:fixture-predecessor".into(),
        cited_proposition_ref: "prop:fixture-duty".into(),
        citation_use: CitationUse::Mentioned,
        reasoning_role: ReasoningRole::Unresolved,
        condition_coordinates: vec![ConditionCoordinate {
            kind: ConditionKind::Legal,
            condition_ref: "cond:fixture-treatment-context".into(),
        }],
        pinpoint_ref: None,
        judge_or_speaker_ref: None,
        court_ref: Some("HCA".into()),
        jurisdiction_ref: Some("AU".into()),
        temporal_ref: Some("2026".into()),
        outcome_ref: None,
        remedy_ref: None,
        burden_refs: vec![],
        exception_refs: vec![],
        lexical_realisation: "fixture treatment expression".into(),
        reviewed: true,
        candidate_only: true,
    }]);
    world.apply_reasoning_delta(delta);
    assert!(world
        .authority_neighbourhood
        .contains("authority:fixture-predecessor"));
    assert!(world
        .query_vocabulary
        .contains("fixture treatment expression"));

    let frontier = ProofFrontier {
        consumer_ref: "consumer:pabai-duty-route".into(),
        frontier_ref: "frontier:before-official".into(),
        residuals: vec![ProofResidual {
            residual_ref: "residual:current-treatment".into(),
            proposition_ref: "prop:current-treatment".into(),
            producer_class_ref: "authority-treatment".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("current-appellate".into()),
            salience: 5,
            dependency_refs: vec!["source:hca:2026:19:rev:fixture".into()],
            status: ResidualStatus::Open,
        }],
        satisfied_payment_refs: vec![],
        contested_coordinate_refs: vec![],
        authority_blocked_refs: vec![],
        authority: "experimental_candidate_only",
    };
    let (_, transition) = apply_assessments(
        &frontier,
        "frontier:after-official",
        &[ResidualAssessment {
            residual_ref: "residual:current-treatment".into(),
            kind: ResidualAssessmentKind::Narrowed,
            observed_proof_reduction: 1,
            assessment_ref: "assessment:official-fixture".into(),
            assessment_authority: "experimental_candidate_only",
        }],
    )
    .expect("candidate assessment must update the frontier");
    assert_eq!(transition.termination, ResearchTermination::Continue);

    println!(
        "provider=HighCourtAustralia acquisition_network=1 world_source_added=true reasoning_delta_added=true frontier=Continue authority={}",
        RECEIPT_AUTHORITY
    );
}
