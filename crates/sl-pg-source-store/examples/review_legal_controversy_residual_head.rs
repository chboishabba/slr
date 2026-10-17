use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewItem, ReviewItemKind, ReviewStatus,
};
use sensiblaw_pg_source_store::{
    apply_persisted_review_command, load_database_config, load_legal_controversy_matter,
    load_legal_controversy_residual, load_legal_proposition_fibre, load_review_item,
    persist_review_item,
};

const REVIEWER_REF: &str = "reviewer:codex:legal-review-v1";
const REVIEW_REASON: &str =
    "select or qualify the reviewed current residual head without promoting legal or semantic truth";

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

fn canonical(mut values: Vec<String>) -> Vec<String> {
    values.retain(|value| !value.trim().is_empty());
    values.sort();
    values.dedup();
    values
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

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 4 || args.len() > 5 {
        return Err(
            "usage: review_legal_controversy_residual_head \
<controversy_ref> <successor_residual_ref> <accept|qualify> [qualification_ref]"
                .into(),
        );
    }

    let controversy_ref = required_arg(&args, 1, "controversy_ref")?;
    let successor_residual_ref = required_arg(&args, 2, "successor_residual_ref")?;
    let action = parse_action(&required_arg(&args, 3, "action")?)?;
    let qualification_ref = match action {
        ReviewAction::Accept => {
            if args.get(4).is_some() {
                return Err("accept does not take qualification_ref".into());
            }
            None
        }
        ReviewAction::Qualify => Some(required_arg(&args, 4, "qualification_ref")?),
        _ => unreachable!(),
    };

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let residual = load_legal_controversy_residual(&config, &successor_residual_ref)
        .map_err(|error| error.to_string())?;
    if residual.controversy_ref != controversy_ref {
        return Err("successor residual is not owned by the named controversy".into());
    }
    if !residual.candidate_only
        || residual.creates_semantic_authority
        || residual.creates_legal_authority
        || residual.applicability_promoted
        || residual.claim_truth_promoted
    {
        return Err("successor residual crossed the non-promotion boundary".into());
    }

    let matter = load_legal_controversy_matter(&config, &controversy_ref)
        .map_err(|error| error.to_string())?;
    let applicant = load_legal_proposition_fibre(&config, &residual.applicant_fibre_ref)
        .map_err(|error| error.to_string())?;
    let respondent = load_legal_proposition_fibre(&config, &residual.respondent_fibre_ref)
        .map_err(|error| error.to_string())?;

    let provenance_refs = canonical(vec![
        controversy_ref.clone(),
        successor_residual_ref.clone(),
        residual.applicant_fibre_ref.clone(),
        residual.respondent_fibre_ref.clone(),
        applicant.reviewed_evidence_ref.clone().unwrap_or_default(),
        respondent.reviewed_evidence_ref.clone().unwrap_or_default(),
    ]);
    let source_refs = canonical(vec![
        applicant.source_reference.clone(),
        respondent.source_reference.clone(),
    ]);

    let review_item_ref = stable_ref(
        "review-item:legal-residual-current-head",
        &[&controversy_ref, &successor_residual_ref],
    );
    let expected_shape = ReviewItem {
        review_item_ref: review_item_ref.clone(),
        semantic_ref: successor_residual_ref.clone(),
        item_kind: ReviewItemKind::ClaimContestation,
        reason: REVIEW_REASON.into(),
        provenance_refs,
        source_refs,
        current_status: ReviewStatus::Pending,
        available_actions: vec![
            ReviewAction::Accept,
            ReviewAction::Qualify,
            ReviewAction::RequestEvidence,
            ReviewAction::OpenSource,
        ],
        affected_consumer_refs: vec![matter.matter_ref.clone()],
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };

    let existing = load_review_item(&config, &review_item_ref).map_err(|error| error.to_string())?;
    match existing {
        None => {
            persist_review_item(&config, &expected_shape).map_err(|error| error.to_string())?;
        }
        Some(mut item) => {
            let existing_status = item.current_status;
            item.current_status = ReviewStatus::Pending;
            if item != expected_shape {
                return Err("existing residual review item conflicts with canonical coordinates".into());
            }
            if !matches!(existing_status, ReviewStatus::Pending | ReviewStatus::Accepted | ReviewStatus::Qualified) {
                return Err("existing residual review item is in an incompatible review state".into());
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
        "review-command:legal-residual-current-head",
        &[
            &review_item_ref,
            REVIEWER_REF,
            action_tag,
            qualification_part,
        ],
    );
    let command = ReviewCommand {
        command_ref: command_ref.clone(),
        review_item_ref: review_item_ref.clone(),
        action,
        reviewer_ref: REVIEWER_REF.into(),
        qualification_ref,
        evidence_request_ref: None,
    };

    let (receipt, reviewed_item) =
        apply_persisted_review_command(&config, &command).map_err(|error| error.to_string())?;
    if receipt.command_ref != command_ref
        || receipt.semantic_ref != successor_residual_ref
        || receipt.reviewer_ref != REVIEWER_REF
        || receipt.action != action
        || reviewed_item.current_status != target_status(action)
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err("persisted residual review receipt failed exact boundary validation".into());
    }

    println!("review_item_ref={review_item_ref}");
    println!("review_command_ref={command_ref}");
    println!("semantic_ref={successor_residual_ref}");
    println!("reviewer_ref={REVIEWER_REF}");
    println!("action={action_tag}");
    println!("review_status={:?}", reviewed_item.current_status);
    println!("candidate_only=true");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
