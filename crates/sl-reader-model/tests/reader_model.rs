use sensiblaw_reader_model::{
    CoordinateCoverage, ExplanationCone, PropositionPayment, ReaderDisposition, ReaderIntent,
    SemanticRef, SourcePayment, SpanRef,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SOURCE_REVISION: &str = "source-revision:mabo-hca23";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

#[test]
fn why_executes_only_for_a_paid_bounded_chain() {
    let source = SourcePayment::paid(
        SemanticRef::new(PROPOSITION),
        SOURCE_REVISION,
        SpanRef::new(SPAN),
    );
    let payment = PropositionPayment::bounded(
        source,
        vec!["observation:support".into()],
        CoordinateCoverage::Residualised("residual:qualifier".into()),
        CoordinateCoverage::Residualised("residual:defeater".into()),
        CoordinateCoverage::Residualised("residual:comparator".into()),
    );

    assert!(matches!(
        payment.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::ExecuteBoundedWhy(ExplanationCone { .. })
    ));
    assert!(!payment.applicability_paid());
    assert!(!payment.claim_truth_paid());
}

#[test]
fn source_can_execute_while_why_defers() {
    let payment = PropositionPayment::source_only(SourcePayment::paid(
        SemanticRef::new(PROPOSITION),
        SOURCE_REVISION,
        SpanRef::new(SPAN),
    ));

    assert!(matches!(
        payment.resolve(ReaderIntent::OpenSource),
        ReaderDisposition::ExecuteSource { .. }
    ));
    assert!(matches!(
        payment.resolve(ReaderIntent::WhyClaim),
        ReaderDisposition::Defer(_)
    ));
    assert!(!payment.applicability_paid());
    assert!(!payment.claim_truth_paid());
}

#[test]
fn ordinary_ui_cannot_construct_truth_or_applicability_payment() {
    let source = SourcePayment::paid(
        SemanticRef::new(PROPOSITION),
        SOURCE_REVISION,
        SpanRef::new(SPAN),
    );
    let payment = PropositionPayment::source_only(source);

    assert!(!payment.applicability_paid());
    assert!(!payment.claim_truth_paid());
}
