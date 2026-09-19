use sensiblaw_pg_source_store::{gwb_ambiguity_state_row, GwbAmbiguityStateInput};
use sensiblaw_world_expansion_runtime::gwb_ambiguity_campaign::{
    gwb_ambiguity_world_sha256, project_open_gwb_state_after_review,
};

fn row(residual: &str) -> sensiblaw_pg_source_store::GwbAmbiguityStateRow {
    gwb_ambiguity_state_row(&GwbAmbiguityStateInput {
        campaign_ref: "campaign:gwb-ambiguity-directed-v1".into(),
        residual_ref: residual.into(),
        subject_ref: "Q207".into(),
        proposition_ref: format!("prop:{residual}"),
        kind_ref: "type-class".into(),
        root_qid: Some("Q207".into()),
        salience: 10,
        dependency_refs: vec![],
        opened_by_hop: None,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
    .unwrap()
}

#[test]
fn projected_post_review_world_removes_closed_and_adds_new_open_residuals() {
    let before = vec![row("r:a"), row("r:b")];
    let opened = vec![GwbAmbiguityStateInput {
        campaign_ref: "campaign:gwb-ambiguity-directed-v1".into(),
        residual_ref: "r:c".into(),
        subject_ref: "Q5".into(),
        proposition_ref: "prop:r:c".into(),
        kind_ref: "superclass".into(),
        root_qid: Some("Q5".into()),
        salience: 8,
        dependency_refs: vec![],
        opened_by_hop: Some(0),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }];

    let after = project_open_gwb_state_after_review(&before, &["r:a".into()], &opened).unwrap();
    let refs = after.iter().map(|row| row.residual_ref.as_str()).collect::<Vec<_>>();
    assert_eq!(refs, vec!["r:b", "r:c"]);
    assert_ne!(gwb_ambiguity_world_sha256(&before), gwb_ambiguity_world_sha256(&after));
}
