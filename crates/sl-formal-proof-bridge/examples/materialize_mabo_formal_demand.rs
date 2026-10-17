use postgres::{Client, NoTls};
use sensiblaw_formal_proof_bridge::{persist_formal_demand, FormalProofDemand};
use sha2::{Digest, Sha256};

fn required_arg(args: &[String], index: usize, name: &'static str) -> Result<String, String> {
    args.get(index)
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("missing {name}"))
}

fn stable_ref(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_bytes();
        hasher.update((bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    format!("{prefix}:sha256:{:x}", hasher.finalize())
}

fn sanitize(value: &str) -> String {
    value.replace('\n', " ").replace('\r', " ").replace('\t', " ")
}

fn main() -> Result<(), String> {
    let args = std::env::args().collect::<Vec<_>>();
    if !(4..=5).contains(&args.len()) {
        return Err(
            "usage: materialize_mabo_formal_demand \
<obligation_ref> <pnf_revision_ref> <residual_ref> [<existing_da51_address|->]"
                .into(),
        );
    }

    let obligation_ref = required_arg(&args, 1, "obligation_ref")?;
    let pnf_revision_ref = required_arg(&args, 2, "pnf_revision_ref")?;
    let residual_ref = required_arg(&args, 3, "residual_ref")?;
    let da51_address = args
        .get(4)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty() && *value != "-")
        .map(str::to_owned);

    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| "DATABASE_URL must identify the existing SLR PostgreSQL store".to_string())?;
    let mut client = Client::connect(&database_url, NoTls).map_err(|error| error.to_string())?;

    let obligation = client
        .query_opt(
            r#"
            SELECT o.controversy_ref,
                   o.proposition_fibre_ref,
                   o.kind_ref,
                   o.required_by,
                   f.reviewed_evidence_ref
            FROM legal_controversy.proof_obligation o
            JOIN legal_controversy.proposition_fibre f
              ON f.fibre_ref = o.proposition_fibre_ref
            WHERE o.obligation_ref=$1
              AND o.is_open
              AND o.candidate_only
              AND NOT o.creates_semantic_authority
              AND NOT o.creates_legal_authority
              AND NOT o.applicability_promoted
              AND NOT o.claim_truth_promoted
            "#,
            &[&obligation_ref],
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "obligation_ref does not reopen one open non-promoting proof obligation".to_string())?;

    let controversy_ref: String = obligation.get(0);
    let proposition_fibre_ref: String = obligation.get(1);
    let obligation_kind_ref: String = obligation.get(2);
    let required_by: String = obligation.get(3);
    let reviewed_evidence_ref: Option<String> = obligation.get(4);
    let reviewed_evidence_ref = reviewed_evidence_ref
        .ok_or_else(|| "proof-obligation fibre has no reviewed evidence coordinate".to_string())?;

    let reviewed = client
        .query_opt(
            r#"
            SELECT consumer_ref,
                   proposition_ref,
                   source_revision_ref,
                   pnf_revision_ref,
                   candidate_only,
                   creates_semantic_authority,
                   applicability_promoted,
                   claim_truth_promoted
            FROM semantic.reviewed_evidence_coordinate
            WHERE reviewed_evidence_ref=$1
            "#,
            &[&reviewed_evidence_ref],
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "reviewed evidence coordinate is missing".to_string())?;

    let consumer_ref: String = reviewed.get(0);
    let proposition_ref: String = reviewed.get(1);
    let source_revision_ref: String = reviewed.get(2);
    let reviewed_pnf_revision_ref: String = reviewed.get(3);
    let candidate_only: bool = reviewed.get(4);
    let creates_semantic_authority: bool = reviewed.get(5);
    let applicability_promoted: bool = reviewed.get(6);
    let claim_truth_promoted: bool = reviewed.get(7);
    if reviewed_pnf_revision_ref != pnf_revision_ref
        || !candidate_only
        || creates_semantic_authority
        || applicability_promoted
        || claim_truth_promoted
    {
        return Err("reviewed evidence does not match the requested non-promoting PNF revision".into());
    }

    let residual = client
        .query_opt(
            r#"
            SELECT unresolved_question,
                   target_evidence_query,
                   applicant_fibre_ref,
                   respondent_fibre_ref,
                   candidate_only,
                   creates_semantic_authority,
                   creates_legal_authority,
                   applicability_promoted,
                   claim_truth_promoted
            FROM legal_controversy.residual
            WHERE residual_ref=$1 AND controversy_ref=$2
            "#,
            &[&residual_ref, &controversy_ref],
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "residual_ref does not belong to the obligation controversy".to_string())?;
    let unresolved_question: String = residual.get(0);
    let target_evidence_query: Option<String> = residual.get(1);
    let applicant_fibre_ref: String = residual.get(2);
    let respondent_fibre_ref: String = residual.get(3);
    let residual_candidate_only: bool = residual.get(4);
    let residual_semantic_authority: bool = residual.get(5);
    let residual_legal_authority: bool = residual.get(6);
    let residual_applicability: bool = residual.get(7);
    let residual_truth: bool = residual.get(8);
    if proposition_fibre_ref != applicant_fibre_ref
        && proposition_fibre_ref != respondent_fibre_ref
    {
        return Err("obligation proposition fibre is not one side of the requested residual".into());
    }
    if !residual_candidate_only
        || residual_semantic_authority
        || residual_legal_authority
        || residual_applicability
        || residual_truth
    {
        return Err("residual crosses an authority/promotion boundary".into());
    }

    let factor = client
        .query_opt(
            "SELECT encode(factor_sha256,'hex') FROM algebra.factor_revision WHERE factor_revision_ref=$1",
            &[&pnf_revision_ref],
        )
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "reviewed PNF factor revision is missing".to_string())?;
    let factor_digest_hex: String = factor.get(0);
    if factor_digest_hex.len() != 64 || !factor_digest_hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err("persisted factor revision digest is not a SHA-256 value".into());
    }
    let statement_digest = format!("sha256:{}", factor_digest_hex.to_ascii_lowercase());

    let query = target_evidence_query
        .filter(|value| !value.trim().is_empty())
        .unwrap_or(unresolved_question);
    if query.trim().is_empty() {
        return Err("persisted residual does not supply a formal routing query".into());
    }

    let da51_part = da51_address.as_deref().unwrap_or("");
    let demand_ref = stable_ref(
        "formal-demand:mabo",
        &[
            "slr-formal-demand:mabo:v1",
            &obligation_ref,
            &pnf_revision_ref,
            &residual_ref,
            &consumer_ref,
            &proposition_ref,
            &source_revision_ref,
            &statement_digest,
            "reviewed-pnf",
            "agda",
            "formal-proof",
            da51_part,
        ],
    );

    let demand = FormalProofDemand {
        demand_ref: demand_ref.clone(),
        obligation_ref: obligation_ref.clone(),
        consumer_ref: consumer_ref.clone(),
        pnf_revision_ref: pnf_revision_ref.clone(),
        residual_ref: residual_ref.clone(),
        proposition_ref: Some(proposition_ref.clone()),
        statement_digest: statement_digest.clone(),
        statement_language: "reviewed-pnf".into(),
        target_prover: "agda".into(),
        da51_address: da51_address.clone(),
        source_revision_refs: vec![source_revision_ref.clone()],
        requested_result_kind: "formal-proof".into(),
        dependent_refs: vec![residual_ref.clone()],
        candidate_only: true,
        creates_semantic_authority: false,
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    persist_formal_demand(&database_url, &demand).map_err(|error| error.to_string())?;

    println!("formal_demand_ref\t{demand_ref}");
    println!("obligation_ref\t{obligation_ref}");
    println!("controversy_ref\t{controversy_ref}");
    println!("residual_ref\t{residual_ref}");
    println!("pnf_revision_ref\t{pnf_revision_ref}");
    println!("consumer_ref\t{consumer_ref}");
    println!("proposition_ref\t{proposition_ref}");
    println!("source_revision_ref\t{source_revision_ref}");
    println!("statement_digest\t{statement_digest}");
    println!("statement_language\treviewed-pnf");
    println!("target_prover\tagda");
    println!("requested_result_kind\tformal-proof");
    println!("da51_address\t{}", da51_address.as_deref().unwrap_or("-"));
    println!("route_query\t{}", sanitize(&query));
    println!("obligation_kind_ref\t{obligation_kind_ref}");
    println!("required_by\t{}", sanitize(&required_by));
    Ok(())
}
