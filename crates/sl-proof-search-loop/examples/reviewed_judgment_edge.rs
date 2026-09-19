use sensiblaw_proof_search_loop::judgment_candidates::extract_judgment_citation_candidates;
use sensiblaw_proof_search_loop::judgment_review::{
    compile_reviewed_candidate_edge, lexical_hint_can_auto_compile_reviewed_edge,
    reviewed_edge_is_binding_authority, ReviewedCitationTreatmentDecision,
};
use sensiblaw_proof_search_loop::reasoning::{
    CitationUse, ConditionCoordinate, ConditionKind, ReasoningRole,
};

fn main() {
    let candidates = extract_judgment_citation_candidates(
        "document:hca:[2026]-HCA-19:docx",
        "source-revision:fixture",
        "sha256:canonical-text-fixture",
        "[42] The Court applied Mallonland [2024] HCA 25 in the identified circumstances.\n",
    );
    let candidate = candidates.first().expect("citation candidate");
    let decision = ReviewedCitationTreatmentDecision {
        candidate_paragraph_locator_ref: candidate.paragraph_locator_ref.clone(),
        candidate_citation_text: candidate.citation_text.clone(),
        citing_proposition_ref: "prop:cullen:reviewed-step".into(),
        cited_document_ref: "case:[2024]-HCA-25".into(),
        cited_proposition_ref: "prop:mallonland:salient-features".into(),
        citation_use: CitationUse::Applied,
        reasoning_role: ReasoningRole::Rule,
        condition_coordinates: vec![ConditionCoordinate {
            kind: ConditionKind::Legal,
            condition_ref: "condition:reviewed-salient-feature-context".into(),
        }],
        judge_or_speaker_ref: Some("speaker:fixture".into()),
        court_ref: Some("HCA".into()),
        jurisdiction_ref: Some("AU".into()),
        temporal_ref: Some("2026".into()),
        outcome_ref: None,
        remedy_ref: None,
        burden_refs: vec![],
        exception_refs: vec![],
        lexical_realisation: "applied Mallonland".into(),
        reviewer_ref: "reviewer:fixture".into(),
        evidence_refs: vec![candidate.paragraph_locator_ref.clone()],
    };
    let edge = compile_reviewed_candidate_edge(candidate, &decision)
        .expect("explicit review compiles exact candidate into reasoning edge");
    assert!(edge.reviewed);
    assert!(edge.candidate_only);
    assert_eq!(edge.citation_use, CitationUse::Applied);
    assert_eq!(edge.reasoning_role, ReasoningRole::Rule);
    assert_eq!(edge.pinpoint_ref.as_deref(), Some(candidate.paragraph_locator_ref.as_str()));
    assert!(!lexical_hint_can_auto_compile_reviewed_edge());
    assert!(!reviewed_edge_is_binding_authority(&edge));

    println!(
        "pinpoint={} citation_use={:?} reasoning_role={:?} reviewed={} candidate_only={} binding_authority=false",
        edge.pinpoint_ref.as_deref().unwrap_or(""),
        edge.citation_use,
        edge.reasoning_role,
        edge.reviewed,
        edge.candidate_only,
    );
}
