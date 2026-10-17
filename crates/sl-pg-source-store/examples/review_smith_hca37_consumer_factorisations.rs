use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewItem, ReviewItemKind, ReviewStatus,
};
use sensiblaw_pg_source_store::{
    apply_persisted_review_command, load_database_config, load_legal_controversy_matter,
    load_review_item, persist_review_item, persist_reviewed_informativeness_factorisation,
    persist_reviewed_representation_factorisation, ReviewedInformativenessFactorisation,
    ReviewedRepresentationFactorisation,
};

const REVIEWER_REF: &str = "reviewer:codex:legal-review-v1";
const CONSUMER_REF: &str = "consumer:smith:affected-person";
const QUERY_FAMILY_REF: &str = "query-family:understand-live-smith-interpretive-controversy";

fn distinctions() -> BTreeSet<String> {
    [
        "smith:distinction:purpose-connected-v-direct-assistance",
        "smith:distinction:joint-reasons-v-reported-party-submission",
        "smith:distinction:meaning-v-severance-partial-disapplication",
        "smith:distinction:opinion-level-provenance",
        "smith:distinction:open-residual-v-decided-merits",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn canonical_refs(mut values: Vec<String>) -> Vec<String> {
    values.retain(|value| !value.trim().is_empty());
    values.sort();
    values.dedup();
    values
}

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing required {name}"))
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update((part.len() as u64).to_be_bytes());
        hasher.update(part.as_bytes());
    }
    format!("{prefix}:sha256:{:x}", hasher.finalize())
}

fn parse_action(value: &str) -> Result<ReviewAction, String> {
    match value {
        "accept" => Ok(ReviewAction::Accept),
        "qualify" => Ok(ReviewAction::Qualify),
        other => Err(format!("unsupported action {other:?}; expected accept or qualify")),
    }
}

fn target_status(action: ReviewAction) -> ReviewStatus {
    match action {
        ReviewAction::Accept => ReviewStatus::Accepted,
        ReviewAction::Qualify => ReviewStatus::Qualified,
        _ => unreachable!("operator accepts only accept or qualify"),
    }
}

fn review_semantic_ref(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    semantic_ref: &str,
    matter_ref: &str,
    controversy_ref: &str,
    provenance_refs: Vec<String>,
    reason: &str,
    action: ReviewAction,
    qualification_ref: Option<String>,
) -> Result<String, String> {
    let review_item_ref = stable_ref(
        "review-item:consumer-factorisation",
        &[matter_ref, controversy_ref, semantic_ref],
    );
    let expected = ReviewItem {
        review_item_ref: review_item_ref.clone(),
        semantic_ref: semantic_ref.to_owned(),
        item_kind: ReviewItemKind::SourceCorrespondence,
        reason: reason.to_owned(),
        provenance_refs: canonical_refs(provenance_refs),
        source_refs: Vec::new(),
        current_status: ReviewStatus::Pending,
        available_actions: vec![
            ReviewAction::Accept,
            ReviewAction::OpenSource,
            ReviewAction::Qualify,
            ReviewAction::RequestEvidence,
        ],
        affected_consumer_refs: canonical_refs(vec![CONSUMER_REF.into(), matter_ref.to_owned()]),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    match load_review_item(config, &review_item_ref).map_err(|error| error.to_string())? {
        None => {
            persist_review_item(config, &expected).map_err(|error| error.to_string())?;
        }
        Some(mut item) => {
            let status = item.current_status;
            item.current_status = ReviewStatus::Pending;
            if item != expected {
                return Err(format!(
                    "existing review item for {semantic_ref} conflicts with canonical coordinates"
                ));
            }
            if !matches!(status, ReviewStatus::Pending | ReviewStatus::Accepted | ReviewStatus::Qualified) {
                return Err(format!("existing review item for {semantic_ref} is not replayable"));
            }
        }
    }

    let action_tag = match action {
        ReviewAction::Accept => "accept",
        ReviewAction::Qualify => "qualify",
        _ => unreachable!(),
    };
    let qualification_part = qualification_ref.as_deref().unwrap_or("");
    let command_ref = stable_ref(
        "review-command:consumer-factorisation",
        &[
            &review_item_ref,
            REVIEWER_REF,
            action_tag,
            qualification_part,
        ],
    );
    let command = ReviewCommand {
        command_ref: command_ref.clone(),
        review_item_ref,
        action,
        reviewer_ref: REVIEWER_REF.into(),
        qualification_ref,
        evidence_request_ref: None,
    };
    let (receipt, item) =
        apply_persisted_review_command(config, &command).map_err(|error| error.to_string())?;
    if receipt.command_ref != command_ref
        || receipt.semantic_ref != semantic_ref
        || receipt.reviewer_ref != REVIEWER_REF
        || receipt.action != action
        || item.current_status != target_status(action)
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(format!("review receipt for {semantic_ref} failed boundary validation"));
    }
    Ok(command_ref)
}

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 8 || args.len() > 9 {
        return Err(
            "usage: review_smith_hca37_consumer_factorisations \
<matter_ref> <controversy_ref> <accessible_representation_ref> <full_representation_ref> \
<accessible_admission_ref> <full_admission_ref> <accept|qualify> [qualification_ref]"
                .into(),
        );
    }

    let matter_ref = required_arg(&args, 1, "matter_ref")?;
    let controversy_ref = required_arg(&args, 2, "controversy_ref")?;
    let accessible_representation_ref = required_arg(&args, 3, "accessible_representation_ref")?;
    let full_representation_ref = required_arg(&args, 4, "full_representation_ref")?;
    let accessible_admission_ref = required_arg(&args, 5, "accessible_admission_ref")?;
    let full_admission_ref = required_arg(&args, 6, "full_admission_ref")?;
    let action = parse_action(&required_arg(&args, 7, "action")?)?;
    let qualification_ref = match action {
        ReviewAction::Accept => {
            if args.get(8).is_some() {
                return Err("accept does not take qualification_ref".into());
            }
            None
        }
        ReviewAction::Qualify => Some(required_arg(&args, 8, "qualification_ref")?),
        _ => unreachable!(),
    };

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let persisted_matter = load_legal_controversy_matter(&config, &controversy_ref)
        .map_err(|error| error.to_string())?;
    if persisted_matter.matter_ref != matter_ref {
        return Err("matter_ref does not match the persisted controversy".into());
    }

    let required = distinctions();
    let distinction_key = required.iter().cloned().collect::<Vec<_>>().join("\u{1f}");
    let accessible_factor_ref = stable_ref(
        "consumer-factorisation:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &accessible_representation_ref,
            CONSUMER_REF,
            QUERY_FAMILY_REF,
            &accessible_admission_ref,
            &distinction_key,
        ],
    );
    let full_factor_ref = stable_ref(
        "consumer-factorisation:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &full_representation_ref,
            CONSUMER_REF,
            QUERY_FAMILY_REF,
            &full_admission_ref,
            &distinction_key,
        ],
    );

    let accessible_review_ref = review_semantic_ref(
        &config,
        &accessible_factor_ref,
        &matter_ref,
        &controversy_ref,
        vec![
            matter_ref.clone(),
            controversy_ref.clone(),
            accessible_representation_ref.clone(),
            accessible_admission_ref.clone(),
        ],
        "review whether the accessible Smith representation preserves every required affected-person distinction",
        action,
        qualification_ref.clone(),
    )?;
    let full_review_ref = review_semantic_ref(
        &config,
        &full_factor_ref,
        &matter_ref,
        &controversy_ref,
        vec![
            matter_ref.clone(),
            controversy_ref.clone(),
            full_representation_ref.clone(),
            full_admission_ref.clone(),
        ],
        "review whether the full Smith legal representation preserves every required affected-person distinction",
        action,
        qualification_ref.clone(),
    )?;

    let accessible_factor = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: accessible_factor_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        representation_ref: accessible_representation_ref.clone(),
        consumer_ref: CONSUMER_REF.into(),
        query_family_ref: QUERY_FAMILY_REF.into(),
        admissibility_receipt_ref: accessible_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: REVIEWER_REF.into(),
        review_decision_ref: accessible_review_ref.clone(),
        accepted: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    let full_factor = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: full_factor_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        representation_ref: full_representation_ref.clone(),
        consumer_ref: CONSUMER_REF.into(),
        query_family_ref: QUERY_FAMILY_REF.into(),
        admissibility_receipt_ref: full_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: REVIEWER_REF.into(),
        review_decision_ref: full_review_ref.clone(),
        accepted: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    persist_reviewed_representation_factorisation(&config, &accessible_factor)
        .map_err(|error| error.to_string())?;
    persist_reviewed_representation_factorisation(&config, &full_factor)
        .map_err(|error| error.to_string())?;

    let comparison_ref = stable_ref(
        "consumer-informativeness-factorisation:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &accessible_factor_ref,
            &full_factor_ref,
            CONSUMER_REF,
            QUERY_FAMILY_REF,
            &distinction_key,
        ],
    );
    let comparison_review_ref = review_semantic_ref(
        &config,
        &comparison_ref,
        &matter_ref,
        &controversy_ref,
        vec![
            matter_ref.clone(),
            controversy_ref.clone(),
            accessible_factor_ref.clone(),
            full_factor_ref.clone(),
        ],
        "review whether the accessible Smith representation retains at least every distinction retained by the full representation for the affected-person query",
        action,
        qualification_ref,
    )?;

    let comparison = ReviewedInformativenessFactorisation {
        reviewed_factorisation_receipt_ref: comparison_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        left_factorisation_receipt_ref: accessible_factor_ref.clone(),
        right_factorisation_receipt_ref: full_factor_ref.clone(),
        consumer_ref: CONSUMER_REF.into(),
        query_family_ref: QUERY_FAMILY_REF.into(),
        right_preserved_distinction_refs: required.clone(),
        left_preserved_distinction_refs: required,
        reviewer_ref: REVIEWER_REF.into(),
        review_decision_ref: comparison_review_ref.clone(),
        accepted: true,
        candidate_only: true,
        creates_universal_superiority: false,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        determines_credibility: false,
        enters_final_judgment: false,
        promotes_claim_truth: false,
    };
    persist_reviewed_informativeness_factorisation(&config, &comparison)
        .map_err(|error| error.to_string())?;

    let accessible_projection_ref = stable_ref(
        "matter-representation-projection:smith-hca37",
        &[&matter_ref, &controversy_ref, &accessible_representation_ref, &accessible_factor_ref],
    );
    let full_projection_ref = stable_ref(
        "matter-representation-projection:smith-hca37",
        &[&matter_ref, &controversy_ref, &full_representation_ref, &full_factor_ref],
    );
    let same_matter_receipt_ref = stable_ref(
        "same-matter-informativeness:smith-hca37",
        &[&matter_ref, &controversy_ref, &accessible_projection_ref, &full_projection_ref, &comparison_ref],
    );

    println!("ITIR_SMITH_HCA37_MATTER_REF={matter_ref}");
    println!("ITIR_SMITH_HCA37_CONTROVERSY_REF={controversy_ref}");
    println!("ITIR_SMITH_HCA37_ACCESSIBLE_REPRESENTATION_REF={accessible_representation_ref}");
    println!("ITIR_SMITH_HCA37_FULL_REPRESENTATION_REF={full_representation_ref}");
    println!("ITIR_SMITH_HCA37_ACCESSIBLE_ADMISSION_REF={accessible_admission_ref}");
    println!("ITIR_SMITH_HCA37_FULL_ADMISSION_REF={full_admission_ref}");
    println!("ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REF={accessible_factor_ref}");
    println!("ITIR_SMITH_HCA37_FULL_FACTORISATION_REF={full_factor_ref}");
    println!("ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REVIEW_REF={accessible_review_ref}");
    println!("ITIR_SMITH_HCA37_FULL_FACTORISATION_REVIEW_REF={full_review_ref}");
    println!("ITIR_SMITH_HCA37_REVIEWED_INFORMATIVENESS_FACTORISATION_REF={comparison_ref}");
    println!("ITIR_SMITH_HCA37_INFORMATIVENESS_REVIEW_REF={comparison_review_ref}");
    println!("ITIR_SMITH_HCA37_REPRESENTATION_REVIEWER_REF={REVIEWER_REF}");
    println!("ITIR_SMITH_HCA37_ACCESSIBLE_PROJECTION_RECEIPT_REF={accessible_projection_ref}");
    println!("ITIR_SMITH_HCA37_FULL_PROJECTION_RECEIPT_REF={full_projection_ref}");
    println!("ITIR_SMITH_HCA37_SAME_MATTER_INFORMATIVENESS_RECEIPT_REF={same_matter_receipt_ref}");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("determines_credibility=false");
    println!("enters_final_judgment=false");
    println!("promotes_claim_truth=false");
    Ok(())
}
