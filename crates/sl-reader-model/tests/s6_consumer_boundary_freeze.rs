//! S6 Freeze Pass: Pin consumer boundary stability for `sensiblaw-reader-model`.
//!
//! Confirms that:
//! 1. `PropositionPayment`, `ReaderDisposition`, and `ExplanationCone` remain storage-neutral.
//! 2. `applicability_paid` and `claim_truth_paid` are never promoted by any reader constructor or dispatch.
//! 3. No PostgreSQL DTOs, Dioxus UI, wgpu renderer, or Python bindings leak into `sensiblaw-reader-model`.
//! 4. Residual coordinates remain explicit and cannot be converted into positive authority.

use sensiblaw_reader_model::{
    CoordinateCoverage, PropositionPayment, ReaderDisposition, ReaderIntent, ResidualRef,
    SemanticRef, SourcePayment, SpanRef,
};

const PROPOSITION: &str = "mabo:proposition:radical-title-native-title";
const SOURCE_REVISION: &str = "source-revision:mabo-hca23";
const SPAN: &str = "span:mabo:brennan:radical-title:no-automatic-beneficial-ownership";

#[test]
fn s6_freeze_storage_neutral_and_no_authority_promotion() {
    let source = SourcePayment::paid(
        SemanticRef::new(PROPOSITION),
        SOURCE_REVISION,
        SpanRef::new(SPAN),
    );

    // 1. Source-only state
    let source_only = PropositionPayment::source_only(source.clone());
    assert!(!source_only.proposition_chain_paid());
    assert!(!source_only.applicability_paid());
    assert!(!source_only.claim_truth_paid());

    // Source can execute while Why defers
    let open_source_disp = source_only.resolve(ReaderIntent::OpenSource);
    assert!(matches!(open_source_disp, ReaderDisposition::ExecuteSource { .. }));

    let why_disp = source_only.resolve(ReaderIntent::WhyClaim);
    assert!(matches!(why_disp, ReaderDisposition::Defer(_)));

    // 2. Bounded explanation state
    let bounded = PropositionPayment::bounded(
        source,
        vec!["observation:mabo:radical-title:brennan-p39".into()],
        CoordinateCoverage::Residualised(ResidualRef::new("reader-residual:qualifier")),
        CoordinateCoverage::Residualised(ResidualRef::new("reader-residual:defeater")),
        CoordinateCoverage::Residualised(ResidualRef::new("reader-residual:comparator")),
    );
    assert!(bounded.proposition_chain_paid());
    assert!(!bounded.applicability_paid());
    assert!(!bounded.claim_truth_paid());

    let why_bounded_disp = bounded.resolve(ReaderIntent::WhyClaim);
    match why_bounded_disp {
        ReaderDisposition::ExecuteBoundedWhy(cone) => {
            assert_eq!(cone.proposition_ref.as_str(), PROPOSITION);
            assert_eq!(cone.support_refs.len(), 1);
            assert!(!cone.applicability_paid());
            assert!(!cone.claim_truth_paid());
            assert_eq!(
                cone.qualifier.residual_ref().unwrap().as_str(),
                "reader-residual:qualifier"
            );
            assert_eq!(
                cone.defeater.residual_ref().unwrap().as_str(),
                "reader-residual:defeater"
            );
            assert_eq!(
                cone.comparator.residual_ref().unwrap().as_str(),
                "reader-residual:comparator"
            );
        }
        other => panic!("expected ExecuteBoundedWhy, got {other:?}"),
    }

    // 3. Unselected / navigation intents reject safely
    assert!(matches!(bounded.resolve(ReaderIntent::Explain), ReaderDisposition::Reject { .. }));
    assert!(matches!(bounded.resolve(ReaderIntent::Back), ReaderDisposition::Reject { .. }));
}

#[test]
fn s6_freeze_manifest_dependency_firewall() {
    let manifest = include_str!("../Cargo.toml");

    // Must not depend on any storage or persistence engine
    assert!(!manifest.contains("postgres"));
    assert!(!manifest.contains("tokio-postgres"));
    assert!(!manifest.contains("sqlx"));

    // Must not depend on downstream presentation or rendering engines
    assert!(!manifest.contains("dioxus"));
    assert!(!manifest.contains("wgpu"));

    // Must not depend on Python or foreign runtimes
    assert!(!manifest.contains("pyo3"));
}
