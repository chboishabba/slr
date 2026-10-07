use sensiblaw_pg_source_store::{
    project_matter_personas, project_matter_reverse_proof_search,
    validate_matter_controversy_draft, MatterControversyDraft,
    MatterControversyResidualDraft, MatterDisagreementKind, MatterEpistemicStatus,
    MatterEvidenceKind, MatterLegalRole, MatterObligationDraft, MatterObligationKind,
    MatterPartyRole, MatterProceduralGoal, MatterPropositionDraft, MatterResponseDraft,
    MatterResponseMode,
};

fn proposition(
    proposition_ref: &str,
    party: MatterPartyRole,
    status: MatterEpistemicStatus,
    normative_order_ref: &str,
) -> MatterPropositionDraft {
    MatterPropositionDraft {
        proposition_ref: proposition_ref.into(),
        matter_ref: "matter:mabo:real-1".into(),
        party,
        legal_role: MatterLegalRole::LegalProposition,
        epistemic_status: status,
        source_ref: format!("source-revision:{proposition_ref}"),
        reviewed_evidence_ref: Some(format!("reviewed-evidence:{proposition_ref}")),
        evidence_kind: MatterEvidenceKind::SourceText,
        temporal_ref: "time:1992".into(),
        relation_ref: "relation:advanced-in-matter".into(),
        normative_order_ref: normative_order_ref.into(),
    }
}

fn fixture() -> MatterControversyDraft {
    MatterControversyDraft {
        controversy_ref: "matter-controversy:mabo:real-1".into(),
        matter_ref: "matter:mabo:real-1".into(),
        root_proposition_ref: "prop:applicant:continuity".into(),
        propositions: vec![
            proposition(
                "prop:applicant:continuity",
                MatterPartyRole::Applicant,
                MatterEpistemicStatus::Supported,
                "normative-order:meriam",
            ),
            proposition(
                "prop:respondent:radical-title-effect",
                MatterPartyRole::Respondent,
                MatterEpistemicStatus::Disputed,
                "normative-order:australian-municipal-law",
            ),
            proposition(
                "prop:court:common-ground",
                MatterPartyRole::Court,
                MatterEpistemicStatus::Admitted,
                "normative-order:australian-municipal-law",
            ),
        ],
        responses: vec![MatterResponseDraft {
            response_ref: "response:characterisation".into(),
            target_proposition_ref: "prop:applicant:continuity".into(),
            response_proposition_ref: "prop:respondent:radical-title-effect".into(),
            mode: MatterResponseMode::AdmitOccurrenceDisputeCharacterisation,
        }],
        residuals: vec![MatterControversyResidualDraft {
            residual_ref: "controversy-residual:characterisation".into(),
            kind: MatterDisagreementKind::Characterisation,
            applicant_proposition_ref: "prop:applicant:continuity".into(),
            respondent_proposition_ref: "prop:respondent:radical-title-effect".into(),
            unresolved_question: "What legal consequence follows from radical title without collapsing pre-existing Indigenous normative order?".into(),
            relational_comparison_ref: Some("rel-comparison:mabo:1".into()),
            relational_obligation_ref: Some("rel-obligation:mabo:1".into()),
            requested_discriminator_ref: "discriminator:recognition-v-creation".into(),
            target_evidence_query: "authority addressing recognition versus creation of pre-existing rights".into(),
            potential_reopening_refs: vec!["prop:applicant:continuity".into(), "prop:respondent:radical-title-effect".into()],
        }],
        obligations: vec![MatterObligationDraft {
            obligation_ref: "proof-obligation:characterisation".into(),
            proposition_ref: "prop:respondent:radical-title-effect".into(),
            kind: MatterObligationKind::Discriminator,
            required_by_ref: "controversy-residual:characterisation".into(),
            discharge_ref: None,
        }],
    }
}

#[test]
fn typed_controversy_is_one_matter_and_not_boolean_negation() {
    let fixture = fixture();
    validate_matter_controversy_draft(&fixture).expect("valid shared Matter controversy");
    assert_ne!(
        MatterResponseMode::AdmitOccurrenceDisputeCharacterisation,
        MatterResponseMode::DenyOccurrence
    );
    assert!(fixture
        .propositions
        .iter()
        .all(|p| p.matter_ref == fixture.matter_ref));
}

#[test]
fn cross_matter_proposition_fails_closed() {
    let mut fixture = fixture();
    fixture.propositions[1].matter_ref = "matter:other".into();
    assert!(validate_matter_controversy_draft(&fixture).is_err());
}

#[test]
fn reverse_search_targets_existing_residual_without_reopening_or_access_authority() {
    let fixture = fixture();
    let search = project_matter_reverse_proof_search(
        &fixture,
        MatterProceduralGoal::DecideEvidenceNeeded,
    )
    .expect("reverse proof projection");
    assert_eq!(search.controversy_ref, fixture.controversy_ref);
    assert_eq!(
        search.requested_discriminator_ref.as_deref(),
        Some("discriminator:recognition-v-creation")
    );
    assert!(search
        .target_evidence_queries
        .iter()
        .any(|q| q.contains("recognition versus creation")));
    assert!(!search.creates_actual_reopening);
    assert!(!search.creates_access_authority);
    assert!(!search.creates_semantic_authority);
    assert!(!search.claim_truth_promoted);
}

#[test]
fn personas_project_same_matter_without_merits_determination() {
    let fixture = fixture();
    let search = project_matter_reverse_proof_search(
        &fixture,
        MatterProceduralGoal::PrepareForAdjudication,
    )
    .expect("reverse proof projection");
    let personas = project_matter_personas(&fixture, &search).expect("persona projections");

    assert_eq!(personas.client.controversy_ref, fixture.controversy_ref);
    assert_eq!(personas.solicitor.controversy_ref, fixture.controversy_ref);
    assert_eq!(personas.court.controversy_ref, fixture.controversy_ref);
    assert!(personas
        .court
        .common_ground_proposition_refs
        .contains(&"prop:court:common-ground".to_string()));
    assert!(personas
        .court
        .disputed_characterisation_refs
        .contains(&"controversy-residual:characterisation".to_string()));
    assert!(personas
        .client
        .normative_order_refs
        .contains(&"normative-order:meriam".to_string()));
    assert!(personas
        .client
        .normative_order_refs
        .contains(&"normative-order:australian-municipal-law".to_string()));

    assert!(!personas.court.determines_credibility);
    assert!(!personas.court.determines_ultimate_fact);
    assert!(!personas.court.assigns_normative_weight);
    assert!(!personas.court.enters_final_judgment);
    assert!(!personas.creates_semantic_authority);
    assert!(!personas.claim_truth_promoted);
}
