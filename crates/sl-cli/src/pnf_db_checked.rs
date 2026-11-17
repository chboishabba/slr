//! DB-native M12 parser evidence bridge for the existing JMD Lean consumer.
//! Reopens SLR's canonical persisted source/parser run through
//! compile_db_statement_pnf_checked. No parallel parser, entity resolver,
//! event admission, semantic reviewer or ontology is implemented here.
use sensiblaw_pg_source_store::{
    compile_db_statement_pnf_checked, load_database_config,
    CandidatePnfRole, DbNativeParserSnapshot, ExactSourceSpan,
    SourceStatementEnvelope, StatementOrigin,
};
use sha2::{Digest, Sha256};
use std::fs;

fn role_name(role: CandidatePnfRole) -> &'static str {
    match role {
        CandidatePnfRole::Actor => "actor",
        CandidatePnfRole::Predicate => "predicate",
        CandidatePnfRole::Patient => "patient",
        CandidatePnfRole::Qualifier => "qualifier",
        CandidatePnfRole::Other => "other",
    }
}

pub fn run(args: &[String]) -> Result<(), String> {
    // A caller must name the ORIGINAL parser run and source coordinates.
    // There is deliberately no review or semantic-authority override.
    if args.len() != 8 {
        return Err(
            "usage: sensiblaw pnf-db-checked PARSER_RUN STATEMENT_REF DOCUMENT_REF \
             SOURCE_REV REGION_REF START_CHAR END_CHAR LITERAL_FILE".into(),
        );
    }
    let [run_ref, statement_ref, document_ref, source_revision_ref,
         region_ref, start_str, end_str, literal_path] = args else {
        return Err("invalid pnf-db-checked arguments".into());
    };
    let start_char = start_str.parse::<u32>().map_err(|_| "invalid START_CHAR")?;
    let end_char = end_str.parse::<u32>().map_err(|_| "invalid END_CHAR")?;
    if start_char >= end_char || run_ref.trim().is_empty()
        || statement_ref.trim().is_empty() || document_ref.trim().is_empty()
        || source_revision_ref.trim().is_empty() || region_ref.trim().is_empty()
    {
        return Err("invalid or empty source/run coordinates".into());
    }
    let literal_text = fs::read_to_string(literal_path).map_err(|e| e.to_string())?;
    if literal_text.chars().count() != (end_char - start_char) as usize {
        return Err("literal character count differs from exact source span".into());
    }

    let config = load_database_config(None).map_err(|e| e.to_string())?;
    let snapshot = DbNativeParserSnapshot::load(&config, run_ref)
        .map_err(|e| e.to_string())?;
    if snapshot.source_revision_ref() != source_revision_ref {
        return Err("parser run is for a different source revision".into());
    }
    let statement = SourceStatementEnvelope {
        statement_ref: statement_ref.clone(),
        document_ref: document_ref.clone(),
        source_revision_ref: source_revision_ref.clone(),
        span: ExactSourceSpan {
            span_ref: region_ref.clone(), start_char, end_char
        },
        literal_text: literal_text.clone(),
        origin: StatementOrigin::ResearchReentry,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    // This is the existing SLR M12 exact source / head / entity / product
    // parity compiler. It is NOT a linguistic interpretation checker.
    let (compiled, evidence) =
        compile_db_statement_pnf_checked(&config, &snapshot, statement)
            .map_err(|e| e.to_string())?;

    let factors: Vec<_> = compiled.pnf.candidates.iter().map(|c| serde_json::json!({
        "candidate_ref": c.candidate_ref,
        "role": role_name(c.role),
        "start_char": c.source_start_char,
        "end_char": c.source_end_char,
        "surface": c.surface,
        "lemma": c.lemma,
        "dependency": c.dependency_ref,
        "candidate_only": c.candidate_only,
    })).collect();
    let tokens: Vec<_> = evidence.tokens().iter().map(|t| serde_json::json!({
        "ordinal": t.token_ordinal, "start_char": t.start_char,
        "end_char": t.end_char, "surface": t.surface,
        "lemma": t.lemma, "pos": t.pos, "morph_json": t.morph_json,
        "head_ordinal": t.head_ordinal, "dependency": t.dependency_ref,
    })).collect();
    let entities: Vec<_> = evidence.entities().iter().map(|e| serde_json::json!({
        "ordinal": e.entity_ordinal, "start_char": e.start_char,
        "end_char": e.end_char, "surface": e.surface, "label": e.label_ref,
    })).collect();
    println!("{}", serde_json::json!({
        "schema": "slr-db-m12-pnf-evidence-v1",
        "statement_ref": statement_ref, "document_ref": document_ref,
        "source_revision_ref": source_revision_ref,
        "parser_run_ref": snapshot.parser_run_ref(),
        "region_ref": region_ref,
        "parser_receipt_ref": compiled.parser_receipt_ref,
        "compilation_key": evidence.compilation_key(),
        "start_char": start_char, "end_char": end_char,
        "literal_sha256": format!("{:x}", Sha256::digest(literal_text.as_bytes())),
        "source_literal_and_parser_product_checked": true,
        "token_head_entity_coverage_checked": true,
        "parser_run_source_revision_join_checked": true,
        "candidate_only": true, "semantic_admission_paid": false,
        "world_event_occurrence_paid": false, "claim_truth_paid": false,
        "candidates": factors, "tokens": tokens, "entities": entities,
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::run;

    #[test]
    fn checked_m12_requires_all_source_coordinates() {
        assert!(run(&[]).is_err());
        assert!(run(&vec!["x".to_owned(); 7]).is_err());
    }

    #[test]
    fn invalid_span_rejected_before_database_access() {
        let args = vec![
            "run", "statement", "document", "revision",
            "region", "12", "12", "/missing"
        ].into_iter().map(str::to_owned).collect::<Vec<_>>();
        assert!(run(&args).is_err());
    }
}
