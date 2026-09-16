use sensiblaw_proof_search_loop::reader_retry::{
    plan_reader_retry, retry_after_extension, ReaderRetryOutcome,
};
use sensiblaw_reader_model::{ReaderDisposition, ResidualRef};

const PROP: &str = "mabo:proposition:radical-title-native-title";

#[test]
fn reader_defer_compiles_to_existing_proof_frontier_producers() {
    let disposition = ReaderDisposition::Defer(vec![
        ResidualRef::new("reader-residual:exact-authority-span"),
        ResidualRef::new("reader-residual:proposition-support"),
        ResidualRef::new("reader-residual:defeater"),
    ]);
    let plan = plan_reader_retry(&disposition, PROP, Some("AU")).unwrap();
    assert_eq!(plan.frontier.residuals.len(), 3);
    assert!(plan.frontier.residuals.iter().any(|residual| {
        residual.producer_class_ref == "producer:exact-primary-authority"
    }));
    assert!(plan.frontier.residuals.iter().any(|residual| {
        residual.producer_class_ref == "producer:reviewed-pnf-proposition-support"
    }));
    assert!(plan.frontier.residuals.iter().any(|residual| {
        residual.producer_class_ref == "producer:proposition-defeater"
    }));
    assert!(!plan.acquisition_is_semantic_payment);
}

#[test]
fn acquisition_or_world_extension_must_be_followed_by_re_evaluation() {
    let before = ReaderDisposition::Defer(vec![ResidualRef::new(
        "reader-residual:proposition-support",
    )]);
    let outcome = retry_after_extension(before, || {
        ReaderDisposition::Defer(vec![ResidualRef::new(
            "reader-residual:source-provenance-weld",
        )])
    })
    .unwrap();
    assert!(matches!(&outcome, ReaderRetryOutcome::StillDeferred { .. }));
    assert!(outcome.re_evaluated());
    assert!(!outcome.acquisition_receipt_is_payment());
}
