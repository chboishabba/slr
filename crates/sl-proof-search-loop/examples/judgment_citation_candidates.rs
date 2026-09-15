use sensiblaw_proof_search_loop::judgment_candidates::{
    citation_candidate_is_current_authority, citation_candidate_is_semantic_correspondence,
    extract_judgment_citation_candidates, lexical_hint_is_citation_use, LexicalTreatmentHint,
};

fn main() {
    let canonical_text = "[42] We applied Mallonland Pty Ltd v Advanta Seeds Pty Ltd [2024] HCA 25 and distinguished Woolcock Street Investments Pty Ltd v CDG Pty Ltd [2004] HCA 16.\n[43] The question remains one of source-grounded treatment.\n";
    let candidates = extract_judgment_citation_candidates(
        "document:hca:[2026]-HCA-19:docx",
        "source-revision:fixture",
        "sha256:canonical-text-fixture",
        canonical_text,
    );
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].citation_text, "[2024] HCA 25");
    assert_eq!(candidates[1].citation_text, "[2004] HCA 16");
    assert_eq!(candidates[0].reported_paragraph_label.as_deref(), Some("[42]"));
    assert!(candidates[0]
        .lexical_treatment_hints
        .contains(&LexicalTreatmentHint::AppliedCandidate));
    assert!(candidates[0]
        .lexical_treatment_hints
        .contains(&LexicalTreatmentHint::DistinguishedCandidate));
    assert!(!citation_candidate_is_semantic_correspondence(&candidates[0]));
    assert!(!citation_candidate_is_current_authority(&candidates[0]));
    assert!(!lexical_hint_is_citation_use(
        LexicalTreatmentHint::AppliedCandidate
    ));

    for candidate in candidates {
        println!(
            "locator={} citation={} reviewed={} candidate_only={} hints={:?}",
            candidate.paragraph_locator_ref,
            candidate.citation_text,
            candidate.reviewed,
            candidate.candidate_only,
            candidate.lexical_treatment_hints,
        );
    }
}
