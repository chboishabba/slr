use sensiblaw_atomic_attribution::{
    cullen_gold_registry, AtomicGate, ProvenanceStage, CULLEN_CONTEXT,
};

fn parse_gate(raw: &str) -> AtomicGate {
    match raw {
        "+1" => AtomicGate::FitsThisAtom,
        "0" => AtomicGate::UnresolvedThisAtom,
        "-1" => AtomicGate::FailsThisAtom,
        other => panic!("unknown gate {other}"),
    }
}

fn stage_name(stage: ProvenanceStage) -> &'static str {
    match stage {
        ProvenanceStage::ExternalSourceClaim => "external-source-claim",
        ProvenanceStage::SecondaryInterpretation => "secondary-interpretation",
        ProvenanceStage::RepositoryReconstruction => "repository-reconstruction",
        ProvenanceStage::CrossSourceInference => "cross-source-inference",
        ProvenanceStage::RepositoryTheoremExtension => "repository-theorem-extension",
        ProvenanceStage::PromotionOrExternalAdjudication => "promotion-or-external-adjudication",
    }
}

#[test]
fn cullen_registry_matches_reviewable_gold_fixture() {
    let registry = cullen_gold_registry();
    let rows: Vec<_> = include_str!("../../../fixtures/cullen_atomic_attribution_v0_1.tsv")
        .lines()
        .skip(1)
        .filter(|line| !line.trim().is_empty())
        .collect();
    assert_eq!(rows.len(), registry.len());

    for row in rows {
        let fields: Vec<_> = row.split('\t').collect();
        assert_eq!(fields.len(), 12, "bad Cullen gold row: {row}");
        let atom_id = fields[0];
        let entry = registry
            .get(CULLEN_CONTEXT, atom_id)
            .unwrap_or_else(|| panic!("missing registered atom {atom_id}"));

        assert_eq!(entry.gate, parse_gate(fields[1]));

        let definition_source = entry
            .definition_lineage
            .supporting_source
            .as_ref()
            .expect("gold definition must retain source");
        assert_eq!(definition_source.source_id, fields[2]);
        assert_eq!(definition_source.exact_locator, fields[3]);
        assert_eq!(definition_source.proposition_authority_role, fields[4]);

        let outcome = entry
            .outcome_lineage
            .as_ref()
            .expect("resolved Cullen atom must retain outcome evidence");
        assert_eq!(outcome.proposition_id, fields[5]);
        let outcome_source = outcome
            .supporting_source
            .as_ref()
            .expect("gold outcome must retain source");
        assert_eq!(outcome_source.source_id, fields[6]);
        assert_eq!(outcome_source.exact_locator, fields[7]);
        assert_eq!(outcome_source.proposition_authority_role, fields[8]);

        assert_eq!(stage_name(entry.definition_lineage.stage), fields[9]);
        assert_eq!(stage_name(outcome.stage), fields[10]);
        assert_eq!(stage_name(entry.evaluation_lineage.stage), fields[11]);
    }
}
