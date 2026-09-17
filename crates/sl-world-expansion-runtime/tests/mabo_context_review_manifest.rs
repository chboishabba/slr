#[path = "../src/adaptive_campaign.rs"]
mod adaptive_campaign;
#[path = "../src/adaptive_context_review.rs"]
mod adaptive_context_review;

use adaptive_campaign::ParsedBoundedContextCandidate;
use adaptive_context_review::{
    bounded_context_candidate_set_sha256, parse_mabo_context_review_tsv,
    prepare_reviewed_context_expansion, ContextExpansionReviewError,
};
use sensiblaw_world_expansion_runtime::MaboIdentityReviewPlan;

fn candidate(target: &str) -> ParsedBoundedContextCandidate {
    ParsedBoundedContextCandidate {
        candidate_id: format!("wikidata:Q36074:P1001:{target}"),
        source_qid: "Q36074".into(),
        target_qid: target.into(),
        property_ref: "P1001".into(),
        source_revision_ref: "wikidata:Q36074:oldid:246813579".into(),
    }
}

#[test]
fn review_binds_exact_revision_and_sorted_candidate_set_digest() {
    let candidates = vec![candidate("Q408"), candidate("Q99")];
    let digest = bounded_context_candidate_set_sha256(
        "wikidata:Q36074:oldid:246813579",
        &candidates,
    )
    .unwrap();
    let reversed = vec![candidate("Q99"), candidate("Q408")];
    assert_eq!(
        digest,
        bounded_context_candidate_set_sha256(
            "wikidata:Q36074:oldid:246813579",
            &reversed,
        )
        .unwrap()
    );

    let manifest = format!(
        "wikidata:Q36074:oldid:246813579\t{digest}\treview:context:q36074\n"
    );
    let assignments = parse_mabo_context_review_tsv(&manifest).unwrap();
    let prepared = prepare_reviewed_context_expansion(&candidates, &assignments[0]).unwrap();
    assert_eq!(prepared.edges.len(), 2);
    assert_eq!(prepared.expansion.source_ref, "Q36074");
    assert_eq!(prepared.expansion.bounded_candidate_count, 2);
    assert_eq!(prepared.expansion.review_ref, "review:context:q36074");
}

#[test]
fn changed_candidate_set_invalidates_prior_context_review() {
    let original = vec![candidate("Q408")];
    let digest = bounded_context_candidate_set_sha256(
        "wikidata:Q36074:oldid:246813579",
        &original,
    )
    .unwrap();
    let manifest = format!(
        "wikidata:Q36074:oldid:246813579\t{digest}\treview:context:q36074\n"
    );
    let assignment = parse_mabo_context_review_tsv(&manifest).unwrap().remove(0);

    let changed = vec![candidate("Q408"), candidate("Q99")];
    assert!(matches!(
        prepare_reviewed_context_expansion(&changed, &assignment),
        Err(ContextExpansionReviewError::CandidateSetDigestMismatch { .. })
    ));
}

#[test]
fn zero_candidate_source_can_still_receive_exact_expansion_review() {
    let revision = "wikidata:Q36074:oldid:246813579";
    let digest = bounded_context_candidate_set_sha256(revision, &[]).unwrap();
    let manifest = format!("{revision}\t{digest}\treview:context:q36074:empty\n");
    let assignment = parse_mabo_context_review_tsv(&manifest).unwrap().remove(0);
    let prepared = prepare_reviewed_context_expansion(&[], &assignment).unwrap();
    assert!(prepared.edges.is_empty());
    assert_eq!(prepared.expansion.source_ref, "Q36074");
    assert_eq!(prepared.expansion.bounded_candidate_count, 0);
}
