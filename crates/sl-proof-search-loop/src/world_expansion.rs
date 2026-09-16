#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        candidate_ref: &str,
        object_ref: &str,
        residual_class: ResidualClass,
        producer_lane: ProducerLane,
        contraction: u64,
    ) -> ExpansionCandidate {
        ExpansionCandidate {
            candidate_ref: candidate_ref.into(),
            object_ref: object_ref.into(),
            object_kind: KnowledgeObjectKind::Qid,
            discovery_parent_ref: "Q1501525".into(),
            triggering_residual_ref: "residual:mabo:test".into(),
            residual_class,
            producer_lane,
            source_revision_ref: None,
            expected_residual_contraction: contraction,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 5,
            acquisition_cost: 1,
            admissible: true,
        }
    }

    #[test]
    fn legal_residual_prefers_legal_lane_when_contraction_is_equal() {
        let legal = candidate(
            "legal",
            "source:mabo:hca:1992:23",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            3,
        );
        let wiki = candidate(
            "wiki",
            "wiki:Mabo_v_Queensland_(No_2)",
            ResidualClass::Legal,
            ProducerLane::WikipediaContext,
            3,
        );
        let selected = select_expansion_candidate(&[wiki, legal], 1).unwrap();
        assert_eq!(selected.candidate_ref, "legal");
    }

    #[test]
    fn higher_contraction_beats_domain_prior() {
        let legal = candidate(
            "legal",
            "source:mabo:hca:1992:23",
            ResidualClass::Legal,
            ProducerLane::GovernedLegal,
            2,
        );
        let identity = candidate(
            "identity",
            "Q975866",
            ResidualClass::Legal,
            ProducerLane::WikidataIdentity,
            4,
        );
        let selected = select_expansion_candidate(&[legal, identity], 1).unwrap();
        assert_eq!(selected.candidate_ref, "identity");
    }

    #[test]
    fn identity_residual_prefers_wikidata_on_equal_contraction() {
        let wiki = candidate(
            "wiki",
            "wiki:Eddie_Mabo",
            ResidualClass::Identity,
            ProducerLane::WikipediaContext,
            2,
        );
        let qid = candidate(
            "qid",
            "Q975866",
            ResidualClass::Identity,
            ProducerLane::WikidataIdentity,
            2,
        );
        let selected = select_expansion_candidate(&[wiki, qid], 1).unwrap();
        assert_eq!(selected.candidate_ref, "qid");
    }

    #[test]
    fn context_residual_prefers_wikipedia_on_equal_contraction() {
        let qid = candidate(
            "qid",
            "Q975866",
            ResidualClass::Context,
            ProducerLane::WikidataIdentity,
            2,
        );
        let wiki = candidate(
            "wiki",
            "wiki:Eddie_Mabo",
            ResidualClass::Context,
            ProducerLane::WikipediaContext,
            2,
        );
        let selected = select_expansion_candidate(&[qid, wiki], 1).unwrap();
        assert_eq!(selected.candidate_ref, "wiki");
    }

    #[test]
    fn ambiguous_wrong_type_duplicate_and_irrelevant_do_not_count() {
        let mut ledger = WorldExpansionLedger::default();
        for outcome in [
            DisambiguationOutcome::Ambiguous,
            DisambiguationOutcome::WrongType,
            DisambiguationOutcome::Duplicate,
            DisambiguationOutcome::IrrelevantToResidual,
        ] {
            let receipt = review_admission(
                &mut ledger,
                &candidate(
                    "c",
                    "Q1",
                    ResidualClass::Identity,
                    ProducerLane::WikidataIdentity,
                    1,
                ),
                ReviewDecision::Reviewed,
                outcome,
            );
            assert!(!receipt.admitted);
        }
        assert_eq!(ledger.total_new_world_objects, 0);
    }

    #[test]
    fn duplicate_object_identity_counts_once() {
        let mut ledger = WorldExpansionLedger::default();
        let first = candidate(
            "first",
            "Q975866",
            ResidualClass::Identity,
            ProducerLane::WikidataIdentity,
            1,
        );
        let second = candidate(
            "second",
            "Q975866",
            ResidualClass::Context,
            ProducerLane::WikipediaContext,
            1,
        );
        assert!(review_admission(
            &mut ledger,
            &first,
            ReviewDecision::Reviewed,
            DisambiguationOutcome::NewRelatedObject,
        )
        .admitted);
        assert!(!review_admission(
            &mut ledger,
            &second,
            ReviewDecision::Reviewed,
            DisambiguationOutcome::SameObject,
        )
        .admitted);
        assert_eq!(ledger.total_new_world_objects, 1);
        assert_eq!(ledger.duplicates_seen, 1);
    }

    #[test]
    fn target_is_object_cardinality_not_depth() {
        let policy = mabo_world_expansion_policy();
        assert_eq!(policy.target_novel_objects, 100);
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..100 {
            let row = candidate(
                &format!("c:{index}"),
                &format!("Q{index}"),
                ResidualClass::Identity,
                ProducerLane::WikidataIdentity,
                1,
            );
            review_admission(
                &mut ledger,
                &row,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewRelatedObject,
            );
        }
        assert!(policy.complete(&ledger));
        assert_eq!(ledger.total_new_world_objects, 100);
    }

    #[test]
    fn ninety_nine_objects_is_not_complete() {
        let policy = mabo_world_expansion_policy();
        let mut ledger = WorldExpansionLedger::default();
        for index in 0..99 {
            let row = candidate(
                &format!("c:{index}"),
                &format!("Q{index}"),
                ResidualClass::Identity,
                ProducerLane::WikidataIdentity,
                1,
            );
            review_admission(
                &mut ledger,
                &row,
                ReviewDecision::Reviewed,
                DisambiguationOutcome::NewRelatedObject,
            );
        }
        assert!(!policy.complete(&ledger));
    }
}
