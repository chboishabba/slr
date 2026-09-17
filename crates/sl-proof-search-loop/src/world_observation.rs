#[cfg(test)]
mod tests {
    use super::*;

    fn mabo_observation(backend: GetterBackend, value_ref: &str) -> WorldObservation {
        WorldObservation {
            request_ref: "query:mabo:P710".into(),
            object_ref: "Q1501525".into(),
            relation_ref: "P710".into(),
            source_ref: "wikidata".into(),
            source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
            content_digest_ref: "sha256:mabo-fixture".into(),
            value_ref: value_ref.into(),
            retrieval_status: RetrievalStatus::Retrieved,
            freshness_status: FreshnessStatus::Current,
            provenance_class: ProvenanceClass::RevisionPinnedExternalSource,
            backend,
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        }
    }

    #[test]
    fn same_mabo_property_from_two_backends_agrees_after_normalization() {
        let slr = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let lean = mabo_observation(GetterBackend::LeanInterop, "Q975866");
        assert_eq!(compare_observations(&slr, &lean), ObservationParity::Agreement);
    }

    #[test]
    fn backend_identity_does_not_change_observation_semantics() {
        let slr = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let lean = mabo_observation(GetterBackend::LeanInterop, "Q975866");
        assert_ne!(slr.backend, lean.backend);
        assert_eq!(slr.normalized(), lean.normalized());
    }

    #[test]
    fn mismatched_value_becomes_parity_residual_not_truth_judgment() {
        let slr = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let lean = mabo_observation(GetterBackend::LeanInterop, "Q36074");
        let parity = compare_observations(&slr, &lean);
        assert!(matches!(
            parity,
            ObservationParity::Residual(GetterParityResidual {
                kind: MismatchKind::ValueMismatch,
                ..
            })
        ));
    }

    #[test]
    fn nat_climate_fixture_can_use_same_observation_abi() {
        let observation = WorldObservation {
            request_ref: "query:nat-climate:p5991-p14143".into(),
            object_ref: "Q10884".into(),
            relation_ref: "migration:P5991->P14143".into(),
            source_ref: "sensiblaw:nat-climate-fixture".into(),
            source_revision_ref: "provided_snapshot_2026-04-01".into(),
            content_digest_ref: "fixture-content-hash-not-pinned-in-this-owner".into(),
            value_ref: "review-required".into(),
            retrieval_status: RetrievalStatus::Retrieved,
            freshness_status: FreshnessStatus::Unknown,
            provenance_class: ProvenanceClass::RevisionPinnedFixture,
            backend: GetterBackend::SlrNative,
            candidate_only: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
        };
        assert!(observation.validate().is_ok());
        assert!(!observation.creates_semantic_authority);
        assert!(!observation.claim_truth_promoted);
    }
}
