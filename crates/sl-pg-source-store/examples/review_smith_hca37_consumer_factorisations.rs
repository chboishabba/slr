use std::collections::{BTreeSet, HashMap};

use sha2::{Digest, Sha256};
use sensiblaw_core::review_workstation::{
    ReviewAction, ReviewCommand, ReviewItem, ReviewItemKind, ReviewStatus,
};
use sensiblaw_pg_source_store::{
    apply_persisted_review_command, load_database_config, load_legal_controversy_matter,
    load_review_item, materialize_smith_representation_evidence,
    persist_review_item, persist_reviewed_informativeness_factorisation,
    persist_reviewed_representation_admission, persist_reviewed_representation_factorisation,
    smith_required_distinctions, RepresentationEvidence, RepresentationEvidenceKind,
    ReviewedInformativenessFactorisation, ReviewedRepresentationFactorisation,
    SMITH_AFFECTED_PERSON_CONSUMER_REF, SMITH_INTERPRETIVE_QUERY_FAMILY_REF,
};

const REVIEWER_REF: &str = "reviewer:codex:legal-review-v1";

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

fn parse_statement_refs(value: &str) -> Result<BTreeSet<String>, String> {
    let refs = value
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();
    if refs.is_empty() {
        return Err("at least one persisted Smith source statement ref is required".into());
    }
    if refs.iter().any(|value| !value.starts_with("statement:")) {
        return Err("every source coordinate must be a persisted statement:* ref".into());
    }
    Ok(refs)
}

fn review_semantic_ref(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    semantic_ref: &str,
    matter_ref: &str,
    provenance_refs: Vec<String>,
    source_refs: Vec<String>,
    reason: &str,
) -> Result<String, String> {
    let review_item_ref = stable_ref(
        "review-item:consumer-factorisation",
        &[matter_ref, semantic_ref],
    );
    let expected = ReviewItem {
        review_item_ref: review_item_ref.clone(),
        semantic_ref: semantic_ref.to_owned(),
        item_kind: ReviewItemKind::SourceCorrespondence,
        reason: reason.to_owned(),
        provenance_refs: canonical_refs(provenance_refs),
        source_refs: canonical_refs(source_refs),
        current_status: ReviewStatus::Pending,
        available_actions: vec![
            ReviewAction::Accept,
            ReviewAction::OpenSource,
            ReviewAction::RequestEvidence,
        ],
        affected_consumer_refs: canonical_refs(vec![
            SMITH_AFFECTED_PERSON_CONSUMER_REF.into(),
            matter_ref.to_owned(),
        ]),
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
            if !matches!(status, ReviewStatus::Pending | ReviewStatus::Accepted) {
                return Err(format!("existing review item for {semantic_ref} is not replayable"));
            }
        }
    }

    let command_ref = stable_ref(
        "review-command:consumer-factorisation",
        &[&review_item_ref, REVIEWER_REF, "accept"],
    );
    let command = ReviewCommand {
        command_ref: command_ref.clone(),
        review_item_ref,
        action: ReviewAction::Accept,
        reviewer_ref: REVIEWER_REF.into(),
        qualification_ref: None,
        evidence_request_ref: None,
    };
    let (receipt, item) =
        apply_persisted_review_command(config, &command).map_err(|error| error.to_string())?;
    if receipt.command_ref != command_ref
        || receipt.semantic_ref != semantic_ref
        || receipt.reviewer_ref != REVIEWER_REF
        || receipt.action != ReviewAction::Accept
        || item.current_status != ReviewStatus::Accepted
        || !receipt.candidate_only
        || receipt.creates_semantic_authority
        || receipt.applicability_promoted
        || receipt.claim_truth_promoted
    {
        return Err(format!("review receipt for {semantic_ref} failed boundary validation"));
    }
    Ok(command_ref)
}

fn review_representation(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    representation: &RepresentationEvidence,
    reason: &str,
) -> Result<(String, String), String> {
    let review_command_ref = review_semantic_ref(
        config,
        &representation.representation_ref,
        &representation.matter_ref,
        representation
            .source_admission_receipt_refs
            .iter()
            .cloned()
            .chain([representation.controversy_ref.clone()])
            .collect(),
        representation.source_statement_refs.iter().cloned().collect(),
        reason,
    )?;
    let admission = persist_reviewed_representation_admission(
        config,
        &representation.representation_ref,
        &review_command_ref,
    )
    .map_err(|error| error.to_string())?;
    Ok((review_command_ref, admission.admission_receipt_ref))
}

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(
            "usage: review_smith_hca37_consumer_factorisations \
<matter_ref> <controversy_ref> <statement_ref[,statement_ref...]> accept"
                .into(),
        );
    }

    let matter_ref = required_arg(&args, 1, "matter_ref")?;
    let controversy_ref = required_arg(&args, 2, "controversy_ref")?;
    let statement_refs = parse_statement_refs(&required_arg(&args, 3, "source statement refs")?)?;
    if required_arg(&args, 4, "review flag")? != "accept" {
        return Err("Smith factorisation producer permits only explicit accept".into());
    }

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let persisted_matter = load_legal_controversy_matter(&config, &controversy_ref)
        .map_err(|error| error.to_string())?;
    if persisted_matter.matter_ref != matter_ref {
        return Err("matter_ref does not match the persisted controversy".into());
    }

    // These two representation objects are immutable and source-backed. The caller cannot
    // supply representation/admission identities. The store reopens every exact statement
    // and requires an existing semantically-admitted statement-observation receipt first.
    let accessible = materialize_smith_representation_evidence(
        &config,
        &matter_ref,
        &controversy_ref,
        RepresentationEvidenceKind::AccessibleDistinctionLedger,
        statement_refs.clone(),
    )
    .map_err(|error| error.to_string())?;
    let full = materialize_smith_representation_evidence(
        &config,
        &matter_ref,
        &controversy_ref,
        RepresentationEvidenceKind::FullExactSourceBundle,
        statement_refs,
    )
    .map_err(|error| error.to_string())?;

    let (accessible_representation_review_ref, accessible_admission_ref) = review_representation(
        &config,
        &accessible,
        "review the source-backed accessible Smith distinction ledger for admission to the affected-person comparison",
    )?;
    let (full_representation_review_ref, full_admission_ref) = review_representation(
        &config,
        &full,
        "review the exact source-backed Smith legal-text bundle for admission to the affected-person comparison",
    )?;

    let required = smith_required_distinctions();
    let distinction_key = required.iter().cloned().collect::<Vec<_>>().join("\u{1f}");
    let accessible_factor_ref = stable_ref(
        "consumer-factorisation:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &accessible.representation_ref,
            SMITH_AFFECTED_PERSON_CONSUMER_REF,
            SMITH_INTERPRETIVE_QUERY_FAMILY_REF,
            &accessible_admission_ref,
            &distinction_key,
        ],
    );
    let full_factor_ref = stable_ref(
        "consumer-factorisation:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &full.representation_ref,
            SMITH_AFFECTED_PERSON_CONSUMER_REF,
            SMITH_INTERPRETIVE_QUERY_FAMILY_REF,
            &full_admission_ref,
            &distinction_key,
        ],
    );

    let accessible_factor_review_ref = review_semantic_ref(
        &config,
        &accessible_factor_ref,
        &matter_ref,
        vec![
            accessible.representation_ref.clone(),
            accessible_admission_ref.clone(),
            accessible_representation_review_ref.clone(),
        ],
        accessible.source_statement_refs.iter().cloned().collect(),
        "review whether the admitted accessible Smith representation preserves every required affected-person distinction",
    )?;
    let full_factor_review_ref = review_semantic_ref(
        &config,
        &full_factor_ref,
        &matter_ref,
        vec![
            full.representation_ref.clone(),
            full_admission_ref.clone(),
            full_representation_review_ref.clone(),
        ],
        full.source_statement_refs.iter().cloned().collect(),
        "review whether the admitted full Smith representation preserves every required affected-person distinction",
    )?;

    let accessible_factor = ReviewedRepresentationFactorisation {
        factorisation_receipt_ref: accessible_factor_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        representation_ref: accessible.representation_ref.clone(),
        consumer_ref: SMITH_AFFECTED_PERSON_CONSUMER_REF.into(),
        query_family_ref: SMITH_INTERPRETIVE_QUERY_FAMILY_REF.into(),
        admissibility_receipt_ref: accessible_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: REVIEWER_REF.into(),
        review_decision_ref: accessible_factor_review_ref.clone(),
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
        representation_ref: full.representation_ref.clone(),
        consumer_ref: SMITH_AFFECTED_PERSON_CONSUMER_REF.into(),
        query_family_ref: SMITH_INTERPRETIVE_QUERY_FAMILY_REF.into(),
        admissibility_receipt_ref: full_admission_ref.clone(),
        preserved_distinction_refs: required.clone(),
        reviewer_ref: REVIEWER_REF.into(),
        review_decision_ref: full_factor_review_ref.clone(),
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
            SMITH_AFFECTED_PERSON_CONSUMER_REF,
            SMITH_INTERPRETIVE_QUERY_FAMILY_REF,
            &distinction_key,
        ],
    );
    let comparison_review_ref = review_semantic_ref(
        &config,
        &comparison_ref,
        &matter_ref,
        vec![accessible_factor_ref.clone(), full_factor_ref.clone()],
        accessible
            .source_statement_refs
            .union(&full.source_statement_refs)
            .cloned()
            .collect(),
        "review whether the admitted accessible Smith representation retains at least every distinction retained by the admitted full representation for the affected-person query",
    )?;

    let comparison = ReviewedInformativenessFactorisation {
        reviewed_factorisation_receipt_ref: comparison_ref.clone(),
        matter_ref: matter_ref.clone(),
        controversy_ref: controversy_ref.clone(),
        left_factorisation_receipt_ref: accessible_factor_ref.clone(),
        right_factorisation_receipt_ref: full_factor_ref.clone(),
        consumer_ref: SMITH_AFFECTED_PERSON_CONSUMER_REF.into(),
        query_family_ref: SMITH_INTERPRETIVE_QUERY_FAMILY_REF.into(),
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
        &[
            &matter_ref,
            &controversy_ref,
            &accessible.representation_ref,
            &accessible_factor_ref,
        ],
    );
    let full_projection_ref = stable_ref(
        "matter-representation-projection:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &full.representation_ref,
            &full_factor_ref,
        ],
    );
    let same_matter_receipt_ref = stable_ref(
        "same-matter-informativeness:smith-hca37",
        &[
            &matter_ref,
            &controversy_ref,
            &accessible_projection_ref,
            &full_projection_ref,
            &comparison_ref,
        ],
    );

    let mut output = HashMap::new();
    output.insert("ITIR_SMITH_HCA37_MATTER_REF", matter_ref);
    output.insert("ITIR_SMITH_HCA37_CONTROVERSY_REF", controversy_ref);
    output.insert(
        "ITIR_SMITH_HCA37_ACCESSIBLE_REPRESENTATION_REF",
        accessible.representation_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_FULL_REPRESENTATION_REF",
        full.representation_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_ACCESSIBLE_ADMISSION_REF",
        accessible_admission_ref,
    );
    output.insert("ITIR_SMITH_HCA37_FULL_ADMISSION_REF", full_admission_ref);
    output.insert(
        "ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REF",
        accessible_factor_ref,
    );
    output.insert("ITIR_SMITH_HCA37_FULL_FACTORISATION_REF", full_factor_ref);
    output.insert(
        "ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REVIEW_REF",
        accessible_factor_review_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_FULL_FACTORISATION_REVIEW_REF",
        full_factor_review_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_REVIEWED_INFORMATIVENESS_FACTORISATION_REF",
        comparison_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_INFORMATIVENESS_REVIEW_REF",
        comparison_review_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_REPRESENTATION_REVIEWER_REF",
        REVIEWER_REF.into(),
    );
    output.insert(
        "ITIR_SMITH_HCA37_ACCESSIBLE_PROJECTION_RECEIPT_REF",
        accessible_projection_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_FULL_PROJECTION_RECEIPT_REF",
        full_projection_ref,
    );
    output.insert(
        "ITIR_SMITH_HCA37_SAME_MATTER_INFORMATIVENESS_RECEIPT_REF",
        same_matter_receipt_ref,
    );
    for key in [
        "ITIR_SMITH_HCA37_MATTER_REF",
        "ITIR_SMITH_HCA37_CONTROVERSY_REF",
        "ITIR_SMITH_HCA37_ACCESSIBLE_REPRESENTATION_REF",
        "ITIR_SMITH_HCA37_FULL_REPRESENTATION_REF",
        "ITIR_SMITH_HCA37_ACCESSIBLE_ADMISSION_REF",
        "ITIR_SMITH_HCA37_FULL_ADMISSION_REF",
        "ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REF",
        "ITIR_SMITH_HCA37_FULL_FACTORISATION_REF",
        "ITIR_SMITH_HCA37_ACCESSIBLE_FACTORISATION_REVIEW_REF",
        "ITIR_SMITH_HCA37_FULL_FACTORISATION_REVIEW_REF",
        "ITIR_SMITH_HCA37_REVIEWED_INFORMATIVENESS_FACTORISATION_REF",
        "ITIR_SMITH_HCA37_INFORMATIVENESS_REVIEW_REF",
        "ITIR_SMITH_HCA37_REPRESENTATION_REVIEWER_REF",
        "ITIR_SMITH_HCA37_ACCESSIBLE_PROJECTION_RECEIPT_REF",
        "ITIR_SMITH_HCA37_FULL_PROJECTION_RECEIPT_REF",
        "ITIR_SMITH_HCA37_SAME_MATTER_INFORMATIVENESS_RECEIPT_REF",
    ] {
        println!("{key}={}", output.get(key).expect("fixed output key"));
    }
    println!("representation_source_backed=true");
    println!("representation_admission_reviewed=true");
    println!("creates_semantic_authority=false");
    println!("creates_legal_authority=false");
    println!("determines_credibility=false");
    println!("enters_final_judgment=false");
    println!("promotes_claim_truth=false");
    Ok(())
}
