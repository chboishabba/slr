//! C4.11 CLI: prepare SLR S29 governed review from an exact C4.10 receipt.
//! Does not adjudicate a coreference or attach Wikidata identities.
use sensiblaw_pg_source_store::{
    load_database_config, prepare_native_antecedent_review,
    NativeAntecedentSourceCandidate,
};
use std::fs;

pub fn run(args: &[String]) -> Result<(), String> {
    let [subcommand, candidate_path] = args else {
        return Err(
            "usage: sensiblaw native-antecedent-review prepare ORIGINAL_C410_RECEIPT.json"
                .into(),
        );
    };
    if subcommand != "prepare" {
        return Err("only explicit prepare is supported; governed review remains separate".into());
    }
    let raw=fs::read_to_string(candidate_path)
        .map_err(|e|format!("read exact original C4.10 receipt: {e}"))?;
    let candidate:NativeAntecedentSourceCandidate=serde_json::from_str(&raw)
        .map_err(|e|format!("parse original C4.10 source-bound receipt: {e}"))?;
    let config=load_database_config(None).map_err(|e|e.to_string())?;
    let proposal=prepare_native_antecedent_review(&config,&candidate)
        .map_err(|e|e.to_string())?;
    println!("{}",serde_json::json!({
        "schema":"slr-native-antecedent-s29-review-queue-receipt-v1",
        "proposal_ref":proposal.proposal_ref,
        "review_item_ref":proposal.review_item_ref,
        "source_revision_ref":proposal.source_revision_ref,
        "focus_statement_ref":proposal.focus_statement_ref,
        "candidate_statement_ref":proposal.candidate_statement_ref,
        "s29_review_status":format!("{:?}",proposal.review_status),
        "antecedent_resolution":proposal.antecedent_resolution,
        "identity_admission_paid":proposal.identity_admission_paid,
        "external_qid_paid":proposal.external_qid_paid,
        "review_required":true,
        "candidate_only":true
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;
    #[test]
    fn no_candidate_receipt_or_implicit_review() {
        assert!(run(&[]).is_err());
        assert!(run(&["review".into(),"unreviewed.json".into()]).is_err());
    }
}
