//! Explicit candidate-only SLR producer called by the JMD Lean wiki machine.
//! This reuses the native SLR spaCy adapter; no second parser or ontology exists.
//! The TSV path is a compatibility producer, not DB-native parser attestation.
use sensiblaw_pg_source_store::{
    CandidatePnfProducer, CandidatePnfRole, ExactSourceSpan, IndexedSpacyTsvAdapter,
};
use std::fs;
use sha2::Digest;

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
    // Explicit input coordinates, never a fabricated source-revision or review receipt.
    if args.len() != 4 {
        return Err(
            "usage: sensiblaw pnf-candidate TSV_PATH SPAN_REF START_CHAR END_CHAR".into()
        );
    }
    let start: u32 = args[2].parse().map_err(|_| "invalid start coordinate")?;
    let end: u32 = args[3].parse().map_err(|_| "invalid end coordinate")?;
    let span = ExactSourceSpan {
        span_ref: args[1].clone(),
        start_char: start,
        end_char: end,
    };
    if span.span_ref.trim().is_empty() {
        return Err("missing span reference".into());
    }
    let source = fs::read_to_string(&args[0]).map_err(|e| e.to_string())?;
    let producer = IndexedSpacyTsvAdapter::parse(&source).map_err(|e| e.to_string())?;
    let batch = producer.produce(&span).map_err(|e| e.to_string())?;

    let candidates: Vec<_> = batch.candidates.iter().map(|c| serde_json::json!({
        "candidate_ref": c.candidate_ref,
        "role": role_name(c.role),
        "start_char": c.source_start_char,
        "end_char": c.source_end_char,
        "surface": c.surface,
        "lemma": c.lemma,
        "dependency": c.dependency_ref,
        "candidate_only": c.candidate_only
    })).collect();
    println!("{}", serde_json::json!({
        "version": "slr-pnf-candidate-v1",
        "producer": "sensiblaw-pg-source-store::IndexedSpacyTsvAdapter",
        "exact_span_ref": batch.exact_span_ref,
        "source_start_char": start,
        "source_end_char": end,
        "parser_input_sha256": format!("{:x}", sha2::Sha256::digest(source.as_bytes())),
        "source_revision_verified": false,
        "parser_evidence_verified": false,
        "semantic_admission_paid": false,
        "proposition_support_paid": batch.proposition_support_paid,
        "applicability_paid": batch.applicability_paid,
        "claim_truth_paid": batch.claim_truth_paid,
        "candidates": candidates
    }));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn producer_keeps_propn_role_and_does_not_pay_truth() {
        let tsv = "T\t1\t0\t5\t0\tAlice\talice\tPROPN\t2\tnsubj\nT\t2\t6\t12\t0\tcalled\tcall\tVERB\t2\tROOT\n";
        let p = IndexedSpacyTsvAdapter::parse(tsv).unwrap();
        let b = p.produce(&ExactSourceSpan {
            span_ref: "span:fixture".into(),
            start_char: 0,
            end_char: 12,
        }).unwrap();
        assert_eq!(b.candidates.len(), 2);
        assert_eq!(role_name(b.candidates[0].role), "actor");
        assert_eq!(role_name(b.candidates[1].role), "predicate");
        assert!(!b.claim_truth_paid);
        assert!(!b.applicability_paid);
    }
}
