use std::collections::BTreeSet;

use sensiblaw_pg_source_store::GwbHopLedgerRow;
use sha2::{Digest, Sha256};

use crate::gwb_ambiguity_campaign::GWB_ADAPTIVE_HOP_TARGET;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbCampaignAnalysisReceipt {
    pub committed_hops: usize,
    pub type_superclass_hops: usize,
    pub cross_language_hops: usize,
    pub negative_outcomes: usize,
    pub negative_redirects: usize,
    pub producer_family_switches: usize,
    pub source_family_switches: usize,
    pub unique_producer_families: usize,
    pub unique_source_families: usize,
    pub residuals_closed: usize,
    pub residuals_opened: usize,
    pub target_reached: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub trajectory_sha256: String,
}

fn producer_family(value: &str) -> &str {
    value.split('|').next().unwrap_or(value)
}

fn source_family(value: &str) -> &'static str {
    if value.starts_with("wikidata:") {
        "wikidata"
    } else if value.starts_with("wikipedia:") {
        "wikipedia"
    } else if value.starts_with("oalc:") {
        "oalc"
    } else {
        "other"
    }
}

fn negative_outcome(value: &str) -> bool {
    matches!(
        value,
        "wrong-type"
            | "duplicate"
            | "irrelevant-to-residual"
            | "empty"
            | "no-support"
            | "abstain"
    )
}

#[must_use]
pub fn analyze_gwb_hops(rows: &[GwbHopLedgerRow]) -> GwbCampaignAnalysisReceipt {
    let mut ordered = rows.to_vec();
    ordered.sort_by_key(|row| row.hop_index);

    let type_superclass_hops = ordered
        .iter()
        .filter(|row| {
            matches!(
                row.investigation_kind_ref.as_str(),
                "type-class" | "superclass" | "subclass"
            )
        })
        .count();
    let cross_language_hops = ordered
        .iter()
        .filter(|row| row.investigation_kind_ref == "cross-language-surface")
        .count();
    let negative_outcomes = ordered
        .iter()
        .filter(|row| negative_outcome(&row.outcome_ref))
        .count();

    let negative_redirects = ordered
        .windows(2)
        .filter(|pair| {
            negative_outcome(&pair[0].outcome_ref)
                && pair[0].selected_move_ref != pair[1].selected_move_ref
        })
        .count();
    let producer_family_switches = ordered
        .windows(2)
        .filter(|pair| producer_family(&pair[0].producer_ref) != producer_family(&pair[1].producer_ref))
        .count();
    let source_family_switches = ordered
        .windows(2)
        .filter(|pair| source_family(&pair[0].source_revision_ref) != source_family(&pair[1].source_revision_ref))
        .count();

    let producer_families = ordered
        .iter()
        .map(|row| producer_family(&row.producer_ref).to_owned())
        .collect::<BTreeSet<_>>();
    let source_families = ordered
        .iter()
        .map(|row| source_family(&row.source_revision_ref))
        .collect::<BTreeSet<_>>();

    let residuals_closed = ordered
        .iter()
        .map(|row| row.closed_residual_refs.len())
        .sum();
    let residuals_opened = ordered
        .iter()
        .map(|row| row.opened_residual_refs.len())
        .sum();

    let mut hasher = Sha256::new();
    hasher.update(b"gwb-campaign-analysis:v1\0");
    for row in &ordered {
        hasher.update(row.receipt_sha256.as_bytes());
        hasher.update([0]);
    }
    let trajectory_sha256 = format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    );

    GwbCampaignAnalysisReceipt {
        committed_hops: ordered.len(),
        type_superclass_hops,
        cross_language_hops,
        negative_outcomes,
        negative_redirects,
        producer_family_switches,
        source_family_switches,
        unique_producer_families: producer_families.len(),
        unique_source_families: source_families.len(),
        residuals_closed,
        residuals_opened,
        target_reached: ordered.len() >= GWB_ADAPTIVE_HOP_TARGET,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        trajectory_sha256,
    }
}

#[must_use]
pub fn render_gwb_analysis_receipt(receipt: &GwbCampaignAnalysisReceipt) -> String {
    [
        format!("committed_hops\t{}", receipt.committed_hops),
        format!("type_superclass_hops\t{}", receipt.type_superclass_hops),
        format!("cross_language_hops\t{}", receipt.cross_language_hops),
        format!("negative_outcomes\t{}", receipt.negative_outcomes),
        format!("negative_redirects\t{}", receipt.negative_redirects),
        format!(
            "producer_family_switches\t{}",
            receipt.producer_family_switches
        ),
        format!("source_family_switches\t{}", receipt.source_family_switches),
        format!(
            "unique_producer_families\t{}",
            receipt.unique_producer_families
        ),
        format!("unique_source_families\t{}", receipt.unique_source_families),
        format!("residuals_closed\t{}", receipt.residuals_closed),
        format!("residuals_opened\t{}", receipt.residuals_opened),
        format!("target_reached\t{}", receipt.target_reached),
        format!("candidate_only\t{}", receipt.candidate_only),
        format!(
            "creates_semantic_authority\t{}",
            receipt.creates_semantic_authority
        ),
        format!("applicability_promoted\t{}", receipt.applicability_promoted),
        format!("claim_truth_promoted\t{}", receipt.claim_truth_promoted),
        format!("trajectory_sha256\t{}", receipt.trajectory_sha256),
    ]
    .join("\n")
        + "\n"
}
