#[cfg(test)]
mod tests {
    use super::*;

    fn query() -> ObservationQuery {
        ObservationQuery {
            query_ref: "query:mabo:P710".into(),
            object_ref: "Q1501525".into(),
            relation_ref: "P710".into(),
            source_ref: "wikidata".into(),
            requested_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(),
        }
    }

    #[test]
    fn native_and_external_backends_normalize_to_same_semantic_observation() {
        let native = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:mabo-revision",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:slr:mabo:P710",
        );
        let external = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:mabo-revision",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:lean:mabo:P710",
        );

        let comparison = compare_normalized_observations(&native, &external).unwrap();
        assert_eq!(comparison.status, ObservationParityStatus::Agreement);
        assert!(comparison.mismatches.is_empty());
        assert!(!comparison.creates_claim_truth);
        assert!(!comparison.creates_semantic_authority);
    }

    #[test]
    fn revision_mismatch_becomes_residual_not_truth_judgment() {
        let native = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409615",
            "sha256:a",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:slr",
        );
        let external = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            query(),
            "Q975866",
            "wikidata:Q1501525:oldid:2333409000",
            "sha256:b",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:external",
        );

        let comparison = compare_normalized_observations(&native, &external).unwrap();
        assert_eq!(comparison.status, ObservationParityStatus::Residual);
        assert!(comparison.mismatches.contains(&GetterMismatchKind::RevisionMismatch));
        assert!(comparison.mismatches.contains(&GetterMismatchKind::DigestMismatch));
        assert_eq!(comparison.residual_ref.as_deref(), Some("getter-parity:query:mabo:P710"));
    }

    #[test]
    fn comparison_requires_same_query_and_candidate_only_receipts() {
        let left = GetterObservation::candidate_only(
            GetterBackend::SlrNative,
            query(),
            "Q975866",
            "rev",
            "digest",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:left",
        );
        let mut other_query = query();
        other_query.relation_ref = "P31".into();
        let right = GetterObservation::candidate_only(
            GetterBackend::ExternalFormalMachine,
            other_query,
            "Q5",
            "rev",
            "digest",
            RetrievalStatus::Retrieved,
            FreshnessStatus::Pinned,
            "receipt:right",
        );
        assert_eq!(
            compare_normalized_observations(&left, &right),
            Err(GetterParityError::QueryMismatch)
        );
    }
}
