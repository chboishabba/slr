//! EPBC 2019/8575 source-scoped, pre-filing Matter specimen.
 //! Original document owners: Commonwealth EPBC portal, EPBC Act, DCCEEW,
 //! QCC/QUT submissions, Stockland's published FAQ, and the actual authors
 //! of each. Analysis owner: dashi_agda PR #1276. This does NOT assert an
 //! application was filed, the government admitted a defect, or any source
 //! revision was persisted/reviewed in a live database.
 //!
 //! Built only over SLR PR #59's existing MatterControversyDraft API.
use sensiblaw_pg_source_store::{
    project_matter_reverse_proof_search, validate_matter_controversy_draft,
    MatterControversyDraft, MatterControversyResidualDraft, MatterDisagreementKind,
    MatterEpistemicStatus, MatterEvidenceKind, MatterLegalRole, MatterObligationDraft,
    MatterObligationKind, MatterPartyRole, MatterProceduralGoal, MatterPropositionDraft,
    MatterResponseDraft, MatterResponseMode,
};

const MATTER: &str = "matter:au:epbc:2019-8575:postapproval";
const APPROVAL: &str = "prop:woogaroo:public-approval-status";
const APPLICANT: &str = "prop:woogaroo:possible-139-2-advice-error";
const HYPOTHETICAL_DEFENCE: &str = "prop:woogaroo:potential-government-consideration";
const LOCAL_BASELINE: &str = "prop:woogaroo:action-specific-local-connectivity";
const DEVELOPER: &str = "prop:woogaroo:stockland-corridor-claims";

fn proposition(
    id: &str,
    wording: &str,
    party: MatterPartyRole,
    role: MatterLegalRole,
    state: MatterEpistemicStatus,
    source: &str,
    temporal: &str,
) -> MatterPropositionDraft {
    MatterPropositionDraft {
        proposition_ref: id.into(),
        display_text: wording.into(),
        matter_ref: MATTER.into(),
        party,
        legal_role: role,
        epistemic_status: state,
        // URLs are PUBLIC SOURCE LOCATORS, not persisted source_revision_ref.
        // Therefore reviewed_evidence_ref is always None in this specimen.
        source_ref: source.into(),
        reviewed_evidence_ref: None,
        evidence_kind: MatterEvidenceKind::SourceText,
        temporal_ref: temporal.into(),
        relation_ref: "relation:candidate-only;not-reviewed".into(),
        normative_order_ref: "australia:commonwealth:epbc-act:applicable-transition-open".into(),
    }
}

fn specimen() -> MatterControversyDraft {
    MatterControversyDraft {
        controversy_ref: "matter-controversy:woogaroo:pre-filing:v0".into(),
        matter_ref: MATTER.into(),
        stage_ref: "stage:source-written-candidate-not-filed".into(),
        supersedes_controversy_ref: None,
        root_proposition_ref: APPLICANT.into(),
        propositions: vec![
            proposition(
                APPROVAL,
                "Commonwealth public portal records EPBC 2019/8575 approval decision; original signed 15 October instrument and conditions have not been acquired in this case bundle.",
                MatterPartyRole::ExternalWitness,
                MatterLegalRole::EvidentiaryFact,
                MatterEpistemicStatus::Supported,
                "https://epbcpublicportal.environment.gov.au/all-notices/",
                "time:2026-10-15:portal-decision",
            ),
            proposition(
                APPLICANT,
                "CANDIDATE ONLY: whether the 15 October decision-maker failed to have regard to the applicable 2022 endangered-koala conservation advice under the saved EPBC s139(2) framework.",
                MatterPartyRole::Applicant,
                MatterLegalRole::LegalProposition,
                MatterEpistemicStatus::Unresolved,
                "https://www.legislation.gov.au/C2004A00485/2026-08-24/",
                "time:2026-10-15:decision",
            ),
            proposition(
                HYPOTHETICAL_DEFENCE,
                "HYPOTHETICAL DEFENCE ONLY, NOT ACTUAL MINISTER PLEADING: the original 95C report, delegate briefing or reasons may show the applicable 2022 koala approved advice was genuinely considered.",
                MatterPartyRole::Respondent,
                MatterLegalRole::LegalProposition,
                MatterEpistemicStatus::Unresolved,
                "https://www.dcceew.gov.au/sites/default/files/documents/recovery-plan-koala-2022.pdf",
                "time:unknown:decision-record-unacquired",
            ),
            proposition(
                LOCAL_BASELINE,
                "CANDIDATE ECOLOGICAL FACT: the action's incremental impact on a fragmented Woogaroo-Opossum Creek koala movement network needs same-action geometry, baseline, condition and expert proof; neighbouring works are context not automatically consequences of this action.",
                MatterPartyRole::Applicant,
                MatterLegalRole::CausalLink,
                MatterEpistemicStatus::Unresolved,
                "https://www.ags.gov.au/publications/legal-briefing/lb-20240219",
                "time:2026-10-15:baseline-review-open",
            ),
            proposition(
                DEVELOPER,
                "STOCKLAND FIRST-PARTY CLAIM, NOT REGULATOR FINDING: the Springview FAQ describes a retained wider creek corridor, staged fauna management and no fixed published construction commencement.",
                MatterPartyRole::ExternalWitness,
                MatterLegalRole::EvidentiaryFact,
                MatterEpistemicStatus::Alleged,
                "https://www.stockland.com.au/residential/qld/springview",
                "time:2026-10:website-claim",
            ),
        ],
        responses: vec![
            MatterResponseDraft {
                response_ref: "response:woogaroo:hypothetical-government-consideration".into(),
                target_proposition_ref: APPLICANT.into(),
                response_proposition_ref: HYPOTHETICAL_DEFENCE.into(),
                mode: MatterResponseMode::ChallengeEvidenceReliability,
            },
            MatterResponseDraft {
                response_ref: "response:woogaroo:development-corridor-answer".into(),
                target_proposition_ref: LOCAL_BASELINE.into(),
                response_proposition_ref: DEVELOPER.into(),
                // An independent developer statement is an evidentiary challenge,
                // NOT a respondent admission of conduct or common ground.
                mode: MatterResponseMode::ChallengeEvidenceReliability,
            },
        ],
        residuals: vec![
            MatterControversyResidualDraft {
                residual_ref: "residual:woogaroo:correct-advice-consideration".into(),
                kind: MatterDisagreementKind::Evidence,
                applicant_proposition_ref: APPLICANT.into(),
                respondent_proposition_ref: HYPOTHETICAL_DEFENCE.into(),
                unresolved_question: "Did the actual 15 October 2019/8575 deciding officer consider the applicable 2022 endangered-koala advice, with the required s139 trigger and operative transition correctly determined?".into(),
                relational_comparison_ref: None,
                relational_obligation_ref: None,
                requested_discriminator_ref: "discriminator:signed-approval:95c:delegate-briefing:2022-advice".into(),
                target_evidence_query: "Obtain the exact signed Part 9 conditions, 95C Secretary report, delegate reasons and 2022 koala conservation-advice circulation/consideration record; compare operative s139 transitional law.".into(),
                potential_reopening_refs: vec![APPLICANT.into(), HYPOTHETICAL_DEFENCE.into()],
            },
            MatterControversyResidualDraft {
                residual_ref: "residual:woogaroo:habitat-function-vs-retained-width".into(),
                kind: MatterDisagreementKind::Causal,
                applicant_proposition_ref: LOCAL_BASELINE.into(),
                respondent_proposition_ref: DEVELOPER.into(),
                unresolved_question: "Do the actual approved corridor, mature food/shade trees, roads, offset timing and habitat connectivity support a functioning local population once this action's incremental works are separated from other projects?".into(),
                relational_comparison_ref: None,
                relational_obligation_ref: None,
                requested_discriminator_ref: "discriminator:epbc-condition-geometry:site-habitat:fauna-surveys".into(),
                target_evidence_query: "Fetch final 2019/8575 approved impact polygon and conditions, Item 4.6(c) final PD, original vegetation/koala baseline, and expert stage-specific retained corridor function.".into(),
                potential_reopening_refs: vec![LOCAL_BASELINE.into(), DEVELOPER.into()],
            },
        ],
        obligations: vec![
            MatterObligationDraft {
                obligation_ref: "obligation:woogaroo:authentic-95c".into(),
                proposition_ref: APPLICANT.into(),
                kind: MatterObligationKind::Evidence,
                required_by_ref: "residual:woogaroo:correct-advice-consideration".into(),
                discharge_ref: None,
            },
            MatterObligationDraft {
                obligation_ref: "obligation:woogaroo:field-and-conditions".into(),
                proposition_ref: LOCAL_BASELINE.into(),
                kind: MatterObligationKind::Discriminator,
                required_by_ref: "residual:woogaroo:habitat-function-vs-retained-width".into(),
                discharge_ref: None,
            },
        ],
    }
}

#[test]
fn woogaroo_is_a_source_written_unpaid_matter_not_an_approval_merits_finding() {
    let draft = specimen();
    validate_matter_controversy_draft(&draft).expect("source-scoped Matter proposal");
    assert_eq!(draft.matter_ref, MATTER);
    assert_eq!(draft.propositions.len(), 5);
    assert_eq!(draft.residuals.len(), 2);
    assert!(draft
        .propositions
        .iter()
        .all(|p| p.reviewed_evidence_ref.is_none()));
    assert!(draft
        .propositions
        .iter()
        .all(|p| p.epistemic_status != MatterEpistemicStatus::Proved));
    assert!(draft
        .obligations
        .iter()
        .all(|o| o.discharge_ref.is_none()));
    assert!(draft.responses.iter().all(|r| r.mode == MatterResponseMode::ChallengeEvidenceReliability));
    assert!(draft
        .propositions
        .iter()
        .filter(|p| p.party == MatterPartyRole::Respondent)
        .all(|p| p.display_text.contains("HYPOTHETICAL DEFENCE ONLY")));
}

#[test]
fn reverse_proof_search_preserves_unpaid_decision_record() {
    let draft = specimen();
    let projection = project_matter_reverse_proof_search(
        &draft,
        MatterProceduralGoal::DecideEvidenceNeeded,
    )
    .expect("SLR candidate reverse search");
    assert_eq!(projection.controversy_ref, draft.controversy_ref);
    assert!(projection
        .target_evidence_queries
        .iter()
        .any(|q| q.contains("95C Secretary report")));
    assert!(!projection.creates_actual_reopening);
    assert!(!projection.creates_access_authority);
    assert!(!projection.creates_semantic_authority);
    assert!(!projection.claim_truth_promoted);
}

#[test]
fn scenic_or_other_action_may_not_replace_the_epbc_matter() {
    let mut draft = specimen();
    draft.propositions[3].matter_ref = "matter:au:epbc:2020-8651:scenic".into();
    assert!(validate_matter_controversy_draft(&draft).is_err());
}

#[test]
fn no_synthetic_evidence_receipt_or_admission_can_discharge_the_ground() {
    let mut draft = specimen();
    draft.obligations[0].required_by_ref = "residual:woogaroo:not-yet-paid".into();
    assert!(validate_matter_controversy_draft(&draft).is_err());
    let mut draft = specimen();
    draft.propositions[2].display_text.clear();
    assert!(validate_matter_controversy_draft(&draft).is_err());
}
