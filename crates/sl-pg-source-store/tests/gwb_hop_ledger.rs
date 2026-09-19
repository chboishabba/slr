use sensiblaw_pg_source_store::{
    gwb_hop_ledger_row, GwbHopLedgerError, GwbHopLedgerInput,
};

fn input(hop_index: usize, prior: Option<&str>) -> GwbHopLedgerInput {
    GwbHopLedgerInput {
        campaign_ref: "campaign:gwb-ambiguity-directed-v1".into(),
        hop_index,
        prior_receipt_sha256: prior.map(str::to_owned),
        world_before_sha256: format!("sha256:{:064x}", hop_index + 1),
        frontier_sha256: format!("sha256:{:064x}", hop_index + 101),
        selected_move_ref: format!("move:gwb:{hop_index}"),
        investigation_kind_ref: "type-class".into(),
        producer_ref: "producer:wikidata-classification".into(),
        source_revision_ref: format!("wikidata:Q207:oldid:{}", hop_index + 1),
        evidence_digest_ref: format!("sha256:{:064x}", hop_index + 201),
        review_ref: format!("review:gwb:{hop_index}"),
        outcome_ref: "new-conceptual-parent".into(),
        residual_effect_ref: "keep-open".into(),
        world_after_sha256: format!("sha256:{:064x}", hop_index + 301),
        closed_residual_refs: vec![],
        opened_residual_refs: vec![format!("residual:gwb:new:{hop_index}")],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

#[test]
fn hop_row_is_deterministic_non_promoting_and_chainable() {
    let row = gwb_hop_ledger_row(&input(1, Some("a".repeat(64).as_str()))).unwrap();
    assert_eq!(row.hop_index, 1);
    assert_eq!(row.receipt_sha256.len(), 64);
    assert!(row.candidate_only);
    assert!(!row.creates_semantic_authority);
    assert!(!row.applicability_promoted);
    assert!(!row.claim_truth_promoted);

    let again = gwb_hop_ledger_row(&input(1, Some("a".repeat(64).as_str()))).unwrap();
    assert_eq!(row.receipt_sha256, again.receipt_sha256);
}

#[test]
fn first_hop_has_no_prior_and_later_hops_require_one() {
    assert!(gwb_hop_ledger_row(&input(0, None)).is_ok());
    assert_eq!(
        gwb_hop_ledger_row(&input(0, Some("a".repeat(64).as_str()))),
        Err(GwbHopLedgerError::UnexpectedPriorAtHopZero)
    );
    assert_eq!(
        gwb_hop_ledger_row(&input(1, None)),
        Err(GwbHopLedgerError::MissingPriorReceipt)
    );
}

#[test]
fn ledger_rejects_semantic_promotion() {
    let mut bad = input(0, None);
    bad.claim_truth_promoted = true;
    assert_eq!(
        gwb_hop_ledger_row(&bad),
        Err(GwbHopLedgerError::LedgerMayNotPromote)
    );
}
