use sensiblaw_pg_source_store::GwbHopLedgerRow;
use sensiblaw_world_expansion_runtime::gwb_analysis::{
    analyze_gwb_hops, render_gwb_analysis_receipt,
};

fn row(
    index: usize,
    kind: &str,
    producer: &str,
    source: &str,
    outcome: &str,
    move_ref: &str,
) -> GwbHopLedgerRow {
    GwbHopLedgerRow {
        campaign_ref: "campaign:gwb-ambiguity-directed-v1".into(),
        hop_index: index,
        prior_receipt_sha256: (index > 0).then(|| format!("{:064x}", index)),
        world_before_sha256: format!("sha256:{:064x}", index + 10),
        frontier_sha256: format!("sha256:{:064x}", index + 20),
        selected_move_ref: move_ref.into(),
        investigation_kind_ref: kind.into(),
        producer_ref: producer.into(),
        source_revision_ref: source.into(),
        evidence_digest_ref: format!("sha256:{:064x}", index + 30),
        review_ref: format!("review:{index}"),
        outcome_ref: outcome.into(),
        residual_effect_ref: "keep-open".into(),
        world_after_sha256: format!("sha256:{:064x}", index + 40),
        closed_residual_refs: if index == 0 { vec!["r:0".into()] } else { vec![] },
        opened_residual_refs: vec![format!("r:new:{index}")],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        receipt_authority: "gwb_adaptive_runtime_review_only",
        receipt_sha256: format!("{:064x}", index + 50),
    }
}

#[test]
fn analysis_counts_switches_negative_redirects_and_residual_flow() {
    let rows = vec![
        row(
            0,
            "type-class",
            "producer:wikidata-classification|operation:gwb:selected-question",
            "wikidata:Q207:oldid:1",
            "wrong-type",
            "move:0",
        ),
        row(
            1,
            "cross-language-surface",
            "producer:wikipedia-surface|operation:gwb:selected-question",
            "wikipedia:es:etag-1",
            "resolved",
            "move:1",
        ),
        row(
            2,
            "property",
            "producer:wikidata-property|operation:gwb:selected-question",
            "wikidata:Q207:oldid:2",
            "new-related-object",
            "move:2",
        ),
    ];
    let receipt = analyze_gwb_hops(&rows);

    assert_eq!(receipt.committed_hops, 3);
    assert_eq!(receipt.type_superclass_hops, 1);
    assert_eq!(receipt.cross_language_hops, 1);
    assert_eq!(receipt.negative_outcomes, 1);
    assert_eq!(receipt.negative_redirects, 1);
    assert_eq!(receipt.producer_family_switches, 2);
    assert_eq!(receipt.source_family_switches, 2);
    assert_eq!(receipt.residuals_closed, 1);
    assert_eq!(receipt.residuals_opened, 3);
    assert!(!receipt.target_reached);
    assert!(receipt.candidate_only);
    assert!(!receipt.claim_truth_promoted);

    let rendered = render_gwb_analysis_receipt(&receipt);
    assert!(rendered.contains("committed_hops\t3"));
    assert!(rendered.contains("producer_family_switches\t2"));
}


#[test]
fn supervised_type_closure_is_a_distinct_source_family_switch() {
    let rows = vec![
        row(
            0,
            "type-class",
            "producer:wikidata-classification|operation:gwb:selected-question",
            "wikidata:Q7725634:oldid:1",
            "same-object",
            "move:classification",
        ),
        row(
            1,
            "external-ontology-fallback",
            "producer:external-ontology-advisory|operation:gwb:selected-question",
            "wikidata-type-closure:Q7725634:sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "shared-superclass",
            "move:external",
        ),
    ];
    let receipt = analyze_gwb_hops(&rows);

    assert_eq!(receipt.unique_source_families, 2);
    assert_eq!(receipt.source_family_switches, 1);
    assert_eq!(receipt.unique_producer_families, 2);
    assert_eq!(receipt.producer_family_switches, 1);
}
