use postgres::{Client, NoTls};
use sensiblaw_pg_source_store::{
    load_database_config, load_reviewed_evidence_coordinate, load_statement_observation_links_for_statement,
    persist_statement_observation_link, smith_reviewed_statement_link_from_candidate,
    PersistedReviewedEvidenceCoordinate, SmithReviewedAdmissionCandidate,
    SMITH_APPLICANT_STATEMENT_REF, SMITH_RESPONDENT_STATEMENT_REF,
};

fn required_arg(args: &[String], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .cloned()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing required {name}"))
}

fn require_exact_pair(applicant: &str, respondent: &str) -> Result<(), String> {
    if applicant != SMITH_APPLICANT_STATEMENT_REF {
        return Err(format!(
            "applicant statement ref is not the exact Smith Matter statement: {applicant}"
        ));
    }
    if respondent != SMITH_RESPONDENT_STATEMENT_REF {
        return Err(format!(
            "respondent statement ref is not the exact Smith Matter statement: {respondent}"
        ));
    }
    Ok(())
}

fn accepted_candidate(
    client: &mut Client,
    reviewed: &PersistedReviewedEvidenceCoordinate,
) -> Result<Option<SmithReviewedAdmissionCandidate>, String> {
    if !reviewed.candidate_only
        || reviewed.creates_semantic_authority
        || reviewed.applicability_promoted
        || reviewed.claim_truth_promoted
    {
        return Ok(None);
    }
    let row = client
        .query_opt(
            r#"SELECT semantic_ref,action_ref,effect_ref,effect_value_ref,candidate_only,
                      creates_semantic_authority,applicability_promoted,claim_truth_promoted
               FROM semantic.review_receipt WHERE command_ref=$1"#,
            &[&reviewed.review_receipt_ref],
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!(
                "reviewed evidence {} references missing review receipt {}",
                reviewed.reviewed_evidence_ref, reviewed.review_receipt_ref
            )
        })?;

    let candidate = SmithReviewedAdmissionCandidate {
        reviewed_evidence_ref: reviewed.reviewed_evidence_ref.clone(),
        statement_ref: reviewed.statement_ref.clone(),
        source_revision_ref: reviewed.source_revision_ref.clone(),
        candidate_pnf_batch_ref: reviewed.candidate_pnf_batch_ref.clone(),
        observation_ref: reviewed.observation_ref.clone(),
        review_receipt_ref: reviewed.review_receipt_ref.clone(),
        review_semantic_ref: row.get(0),
        review_action_ref: row.get(1),
        review_effect_ref: row.get(2),
        review_effect_value_ref: row.get(3),
        candidate_only: row.get(4),
        creates_semantic_authority: row.get(5),
        applicability_promoted: row.get(6),
        claim_truth_promoted: row.get(7),
    };

    match smith_reviewed_statement_link_from_candidate(&candidate) {
        Ok(_) => Ok(Some(candidate)),
        Err(_) => Ok(None),
    }
}

fn unique_accepted_reviewed_coordinate(
    config: &sensiblaw_pg_source_store::DatabaseConfig,
    client: &mut Client,
    statement_ref: &str,
) -> Result<SmithReviewedAdmissionCandidate, String> {
    let refs = client
        .query(
            r#"SELECT reviewed_evidence_ref
               FROM semantic.reviewed_evidence_coordinate
               WHERE statement_ref=$1
               ORDER BY reviewed_evidence_ref"#,
            &[&statement_ref],
        )
        .map_err(|error| error.to_string())?;
    if refs.is_empty() {
        return Err(format!(
            "no reviewed Smith evidence coordinate exists for {statement_ref}"
        ));
    }

    let mut accepted = Vec::new();
    for row in refs {
        let reviewed_evidence_ref: String = row.get(0);
        let reviewed = load_reviewed_evidence_coordinate(config, &reviewed_evidence_ref)
            .map_err(|error| error.to_string())?;
        if reviewed.statement_ref != statement_ref {
            return Err(format!(
                "reviewed evidence {reviewed_evidence_ref} reopened with a different statement ref"
            ));
        }
        if let Some(candidate) = accepted_candidate(client, &reviewed)? {
            accepted.push(candidate);
        }
    }

    match accepted.len() {
        1 => Ok(accepted.remove(0)),
        0 => Err(format!(
            "no explicitly accepted, non-promoting reviewed Smith evidence coordinate exists for {statement_ref}"
        )),
        count => Err(format!(
            "ambiguous Smith reviewed evidence for {statement_ref}: {count} accepted coordinates; refusing to choose"
        )),
    }
}

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(format!(
            "usage: materialize_smith_reviewed_statement_admissions \\\n{SMITH_APPLICANT_STATEMENT_REF} \\\n{SMITH_RESPONDENT_STATEMENT_REF}"
        ));
    }
    let applicant_statement_ref = required_arg(&args, 1, "applicant statement_ref")?;
    let respondent_statement_ref = required_arg(&args, 2, "respondent statement_ref")?;
    require_exact_pair(&applicant_statement_ref, &respondent_statement_ref)?;

    let config = load_database_config(None).map_err(|error| error.to_string())?;
    let mut client = Client::connect(config.database_url(), NoTls).map_err(|error| error.to_string())?;

    let applicant = unique_accepted_reviewed_coordinate(
        &config,
        &mut client,
        &applicant_statement_ref,
    )?;
    let respondent = unique_accepted_reviewed_coordinate(
        &config,
        &mut client,
        &respondent_statement_ref,
    )?;

    if applicant.source_revision_ref != respondent.source_revision_ref {
        return Err(format!(
            "Smith statement reviewed evidence crosses source revisions: applicant={} respondent={}",
            applicant.source_revision_ref, respondent.source_revision_ref
        ));
    }

    let applicant_link = smith_reviewed_statement_link_from_candidate(&applicant)
        .map_err(|error| error.to_string())?;
    let respondent_link = smith_reviewed_statement_link_from_candidate(&respondent)
        .map_err(|error| error.to_string())?;

    let applicant_reopened = persist_statement_observation_link(&config, &applicant_link)
        .map_err(|error| error.to_string())?;
    let respondent_reopened = persist_statement_observation_link(&config, &respondent_link)
        .map_err(|error| error.to_string())?;

    if applicant_reopened != applicant_link || respondent_reopened != respondent_link {
        return Err("Smith statement admission replay mismatch".into());
    }

    let applicant_links = load_statement_observation_links_for_statement(
        &config,
        &applicant_statement_ref,
    )
    .map_err(|error| error.to_string())?;
    let respondent_links = load_statement_observation_links_for_statement(
        &config,
        &respondent_statement_ref,
    )
    .map_err(|error| error.to_string())?;
    if !applicant_links.iter().any(|value| value == &applicant_link)
        || !respondent_links.iter().any(|value| value == &respondent_link)
    {
        return Err("persisted Smith statement admission did not reopen from the trace store".into());
    }

    println!("ITIR_SMITH_HCA37_SOURCE_REVISION_REF={}", applicant.source_revision_ref);
    println!("ITIR_SMITH_HCA37_APPLICANT_REVIEWED_EVIDENCE_REF={}", applicant.reviewed_evidence_ref);
    println!("ITIR_SMITH_HCA37_RESPONDENT_REVIEWED_EVIDENCE_REF={}", respondent.reviewed_evidence_ref);
    println!("ITIR_SMITH_HCA37_APPLICANT_STATEMENT_OBSERVATION_LINK_REF={}", applicant_link.link_ref);
    println!("ITIR_SMITH_HCA37_RESPONDENT_STATEMENT_OBSERVATION_LINK_REF={}", respondent_link.link_ref);
    println!("semantic_admission_materialized_from_existing_reviewed_evidence=true");
    println!("invented_observation=false");
    println!("unrelated_statement_used=false");
    println!("creates_semantic_authority=false");
    println!("applicability_promoted=false");
    println!("claim_truth_promoted=false");
    Ok(())
}
