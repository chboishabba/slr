use sensiblaw_pg_source_store::{
    CandidatePnfProducer, CandidatePnfRole, ExactSourceSpan, SpacyTsvAdapter,
};

const MABO_SPAN_REF: &str =
    "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

fn exact_mabo_span() -> ExactSourceSpan {
    ExactSourceSpan {
        span_ref: MABO_SPAN_REF.to_owned(),
        start_char: 1530,
        end_char: 1663,
    }
}

#[test]
fn parser_neutral_candidate_producer_retains_exact_source_overlap() {
    // TSV is deliberately only a temporary compatibility manifestation of the
    // old spaCy stream.  The stable contract under test is CandidatePnfProducer
    // -> typed candidate PNF; downstream review/payment must not depend on TSV.
    let tsv = concat!(
        "D\t1\n",
        "P\t0\n",
        "S\t0\t1520\t1680\n",
        "T\t0\t1530\t1541\t1\tradical title\tradical title\tNOUN\tNN\tnsubjpass\n",
        "T\t1\t1542\t1550\t1\tacquired\tacquire\tVERB\tVBN\tROOT\n",
        "T\t2\t1637\t1645\t1\tabsolute\tabsolute\tADJ\tJJ\tamod\n",
        "T\t3\t1646\t1663\t1\tbeneficial title\tbeneficial title\tNOUN\tNN\tdobj\n",
        "E\t0\n",
        "Q\t0\n",
        "M\tspacy_parse_ns=1\n",
    );
    let exact = exact_mabo_span();
    let producer = SpacyTsvAdapter::new(tsv);

    let batch = producer
        .produce(&exact)
        .expect("temporary spaCy TSV adapter must conform to the parser-neutral candidate-PNF ABI");

    assert_eq!(batch.exact_span_ref, MABO_SPAN_REF);
    assert!(!batch.candidates.is_empty());
    assert!(batch
        .candidates
        .iter()
        .all(|candidate| candidate.source_start_char < exact.end_char
            && candidate.source_end_char > exact.start_char));
    assert!(batch.candidates.iter().all(|candidate| candidate.candidate_only));
    assert!(batch
        .candidates
        .iter()
        .any(|candidate| candidate.role == CandidatePnfRole::Actor));
    assert!(batch
        .candidates
        .iter()
        .any(|candidate| candidate.role == CandidatePnfRole::Patient));

    // Candidate production is representation-neutral and authority-free.
    assert!(!batch.proposition_support_paid);
    assert!(!batch.applicability_paid);
    assert!(!batch.claim_truth_paid);
}

#[test]
fn parser_adapter_cannot_promote_non_overlapping_input_into_review_candidates() {
    let tsv = concat!(
        "D\t1\n",
        "P\t0\n",
        "S\t0\t0\t20\n",
        "T\t0\t0\t4\t1\tThey\tthey\tPRON\tPRP\tnsubj\n",
        "T\t1\t5\t9\t1\tread\tread\tVERB\tVBD\tROOT\n",
        "T\t2\t10\t20\t1\tjudgment\tjudgment\tNOUN\tNN\tdobj\n",
        "E\t0\n",
        "Q\t0\n",
    );
    let exact = exact_mabo_span();
    let producer = SpacyTsvAdapter::new(tsv);

    let batch = producer.produce(&exact).unwrap();
    assert!(batch.candidates.is_empty());
    assert!(!batch.proposition_support_paid);
}
