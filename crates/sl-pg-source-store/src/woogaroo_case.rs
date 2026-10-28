//! Woogaroo EPBC 2019/8575: bounded production-side case intake.
//!
//! This is an adapter OVER the existing SLR MatterControversyDraft API,
//! not a second evidence engine. SensibLaw (Python) is historical prototype;
//! newly authored runtime case logic belongs here in SLR (Rust).
//!
//! A source URL is a locator, NOT retained source bytes, canonical text,
//! span payment, review, ministerial receipt, evidence of a contravention,
//! a respondent admission, or a filed action.
//!
//! Dated original ownership: Commonwealth EPBC Act / DCCEEW / TSSC;
//! Taylor, Maguire, Lewis (QUT); SWFG / Keith McCosh; Stockland;
//! AGS (2024 practitioner analysis). DASHI PR #1276 owns the formal
//! source-gated candidate grounds; SLR PR #59 owns native Matter contracts.
//! This code makes NO claim of Agda/kernel certification or case outcome.

use std::collections::BTreeSet;

use crate::{
    validate_matter_controversy_draft, MatterControversyDraft,
    MatterControversyError, MatterControversyResidualDraft,
    MatterDisagreementKind, MatterEpistemicStatus, MatterEvidenceKind,
    MatterLegalRole, MatterObligationDraft, MatterObligationKind,
    MatterPartyRole, MatterPropositionDraft, MatterResponseDraft,
    MatterResponseMode,
};

pub const WOOGAROO_MATTER_REF: &str = "matter:au:epbc:2019-8575:postapproval";
pub const WOOGAROO_ACTION_REF: &str = "2019/8575";
pub const SCENIC_ACTION_REF: &str = "2020/8651";
pub const WOOGAROO_CASE_CONTRACT: &str = "slr:woogaroo:source-scoped-matter:v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoogarooSourceRole {
    PrimaryLegislation,
    GovernmentApprovalNotice,
    ApprovedConservationAdvice,
    NationalRecoveryPlan,
    AuthoredSubmission,
    DeveloperPublication,
    PractitionerLegalAnalysis,
    OriginalInvitation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooSourceLocator {
    pub key: &'static str,
    pub epbc_action: &'static str,
    pub originator: &'static str,
    pub original_url: &'static str,
    pub date_label: &'static str,
    pub role: WoogarooSourceRole,
    /// This only means the authored activity may be examined for s487;
    /// it does not establish that a qualifying 2-year series exists.
    pub standing_activity_candidate: bool,
    /// Locators are not durable source revisions, even for statutes.
    pub retained_source_revision_ref: Option<&'static str>,
    pub canonical_content_sha256: Option<&'static str>,
    pub exact_source_span_ref: Option<&'static str>,
    pub reviewed_case_evidence_ref: Option<&'static str>,
}

impl WoogarooSourceLocator {
    /// This checks reference identity only, not source admission,
    /// decision-maker consideration, admissibility, or the truth of merits.
    pub fn matches_springview_action(self) -> bool {
        self.epbc_action == WOOGAROO_ACTION_REF
    }

    pub fn source_or_case_evidence_paid(self) -> bool {
        self.retained_source_revision_ref.is_some()
            || self.canonical_content_sha256.is_some()
            || self.exact_source_span_ref.is_some()
            || self.reviewed_case_evidence_ref.is_some()
    }
}

macro_rules! source {
    ($key:expr, $action:expr, $author:expr, $url:expr, $date:expr, $role:ident, $activity:expr) => {
        WoogarooSourceLocator {
            key: $key,
            epbc_action: $action,
            originator: $author,
            original_url: $url,
            date_label: $date,
            role: WoogarooSourceRole::$role,
            standing_activity_candidate: $activity,
            retained_source_revision_ref: None,
            canonical_content_sha256: None,
            exact_source_span_ref: None,
            reviewed_case_evidence_ref: None,
        }
    };
}

/// Public original locators, not a source store snapshot.
/// Same-project QUT letter remains a submitted author's public claim until
/// original proponent inbox receipt and ministerial 95B transmission are paid.
pub const WOOGAROO_SOURCE_LOCATORS: [WoogarooSourceLocator; 9] = [
    source!(
        "epbc-act", WOOGAROO_ACTION_REF, "Commonwealth Parliament",
        "https://www.legislation.gov.au/C2004A00485/2026-08-24",
        "24 August 2026 compilation; saved s139 transition requires review",
        PrimaryLegislation, false
    ),
    source!(
        "epbc-decision-status", WOOGAROO_ACTION_REF, "DCCEEW EPBC Public Portal",
        "https://eap-cp.powerappsportals.com/all-notices/",
        "15 October 2026 approved with conditions; NOT signed notice",
        GovernmentApprovalNotice, false
    ),
    source!(
        "koala-approved-advice", WOOGAROO_ACTION_REF,
        "DCCEEW / Threatened Species Scientific Committee",
        "https://epbcpublicportal.environment.gov.au/_entity/sharepointdocumentlocation/ca34e3c6-b216-ed11-b83e-00224818a528/2ab10dab-d681-4911-b881-cc99413f07b6?file=Att+H+-+DAWE+%282022%29+Conservation+Advice+for+Phascolarctos+cinereus+%28Koala%29.pdf",
        "12 February 2022; original advice, not officer-consideration receipt",
        ApprovedConservationAdvice, false
    ),
    source!(
        "national-koala-recovery-plan", WOOGAROO_ACTION_REF,
        "Department of Agriculture, Water and the Environment",
        "https://www.dcceew.gov.au/sites/default/files/documents/recovery-plan-koala-2022.pdf",
        "2022, operative plan applicability to be checked",
        NationalRecoveryPlan, false
    ),
    source!(
        "qut-springview-letter", WOOGAROO_ACTION_REF,
        "Monica Taylor, Rowena Maguire and Bridget Lewis (QUT)",
        "https://img1.wsimg.com/blobby/go/c204ebbc-02bb-46d9-ad71-ed7d494e752a/QUT%20LTR%20EPBC%20referral%20Woogaroo%20-%2012032026.pdf",
        "12 March 2026; actual recipient delivery not established",
        AuthoredSubmission, false
    ),
    source!(
        "stockland-springview", WOOGAROO_ACTION_REF,
        "Stockland / project website publisher",
        "https://www.stockland.com.au/residential/qld/springview",
        "October 2026 page; no certified revision acquired",
        DeveloperPublication, false
    ),
    source!(
        "swfg-scenic-letter", SCENIC_ACTION_REF,
        "Save Woogaroo Forest Group Inc; Keith McCosh printed attribution",
        "https://img1.wsimg.com/blobby/go/c204ebbc-02bb-46d9-ad71-ed7d494e752a/downloads/5b140ad8-accf-4ef6-971d-07dc07d315ea/Response%20by%20SWF%20on%20Scenic%208651.pdf?ver=1792152994924",
        "15 December 2025; distinct Scenic 2020/8651",
        AuthoredSubmission, true
    ),
    source!(
        "ags-cumulative-baseline", WOOGAROO_ACTION_REF,
        "Australian Government Solicitor",
        "https://www.ags.gov.au/publications/legal-briefing/lb-20240219",
        "23 February 2024; practitioner explanation, not Court judgment",
        PractitionerLegalAnalysis, false
    ),
    source!(
        "springview-comment-invitation", WOOGAROO_ACTION_REF,
        "Cherish Enterprises / Saunders Havill; EPBC portal hosting",
        "https://epbcpublicportal.environment.gov.au/_entity/sharepointdocumentlocation/4f8edc14-9ffb-ee11-9f89-00224892a860/2ab10dab-d681-4911-b881-cc99413f07b6?file=2019-8575-Draft-PD.pdf",
        "13 February–12 March 2026; 1-page invitation, not full PD",
        OriginalInvitation, false
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoogarooGroundPriority {
    UrgentProcedural,
    PrimaryMerits,
    SecondaryMerits,
    SourceOnlySpeculation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooGround {
    pub key: &'static str,
    pub statutory_hook: &'static str,
    pub next_original: &'static str,
    pub priority: WoogarooGroundPriority,
    /// 0 means evidence unresolved, following original SLR atomic
    /// legal-test design. This does not copy the unmerged Python registry.
    pub evidence_gate: i8,
    pub positive_witness_ref: Option<&'static str>,
    pub negative_witness_ref: Option<&'static str>,
    pub court_filing_ref: Option<&'static str>,
}

macro_rules! ground {
    ($key:expr, $hook:expr, $original:expr, $priority:ident) => {
        WoogarooGround {
            key: $key,
            statutory_hook: $hook,
            next_original: $original,
            priority: WoogarooGroundPriority::$priority,
            evidence_gate: 0,
            positive_witness_ref: None,
            negative_witness_ref: None,
            court_filing_ref: None,
        }
    };
}

pub const WOOGAROO_GROUNDS: [WoogarooGround; 7] = [
    ground!("advice-consideration", "saved EPBC s139(2)",
        "Signed 15 October conditions, s95C, delegate brief and genuine consideration of 2022 koala advice",
        UrgentProcedural),
    ground!("recovery-plan", "saved EPBC s139(1)",
        "Actual binding plan provisions and consistent signed approval conditions",
        PrimaryMerits),
    ground!("incremental-habitat", "saved EPBC s136 and s139",
        "Same-2019/8575 impact geometry, final Item 4.6(c), local koala connectivity and corridor function",
        PrimaryMerits),
    ground!("comment-transmission", "saved EPBC ss95B and 95C",
        "Original proponent recipient inbox, transmitted copies, Secretary recommendation chronology",
        UrgentProcedural),
    ground!("approval-scope", "saved EPBC ss133–134, transitional law",
        "Signed approval, final conditions, delegation, referred-action and stage polygon",
        UrgentProcedural),
    ground!("local-prestart", "Ipswich 9281/2024/OW Condition 6(a)",
        "Original Council satisfaction letter, same-stage commencement and fauna-prestart compliance",
        UrgentProcedural),
    ground!("improper-influence", "ADJR s5 subject to genuine review ground",
        "Actual same-decision decision-maker conflict, direct communication and causal/legal element",
        SourceOnlySpeculation),
];

/// Separate procedural prerequisites. A signed instrument or legal standing
/// is not a legal-error ground, and no recorded source pays these obligations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooProceduralPrecondition {
    pub key: &'static str,
    pub required_original: &'static str,
    pub document_paid: bool,
    pub competent_decision_paid: bool,
}

pub const WOOGAROO_PRECONDITIONS: [WoogarooProceduralPrecondition; 4] = [
    WoogarooProceduralPrecondition {
        key: "named-applicant-and-organisation-authority",
        required_original: "SWFG current and 15 October 2026 historic governing rules, authorised officers, real activity series, authorised applicant or another independently eligible person",
        document_paid: false, competent_decision_paid: false,
    },
    WoogarooProceduralPrecondition {
        key: "applicant-specific-adjr-clock",
        required_original: "Actual written decision/reasons furnishing, named applicant's filing triggers, extension or independent jurisdiction analysis by counsel",
        document_paid: false, competent_decision_paid: false,
    },
    WoogarooProceduralPrecondition {
        key: "actual-stage-works-and-urgency",
        required_original: "Dated/geolocated project-stage works and imminent clearing, signed same-stage condition compliance and proportionate protective-order evidence",
        document_paid: false, competent_decision_paid: false,
    },
    WoogarooProceduralPrecondition {
        key: "signed-federal-decision-package",
        required_original: "Original signed 15 October federal approval, conditions, approving officer, expiry, Secretary section95C and complete transmitted section95B",
        document_paid: false, competent_decision_paid: false,
    },
];

/// Deterministic max-cut/Pareto queue. This is a documented acquisition
/// priority, NOT a probability of legal success or a numerical impact score.
pub fn woogaroo_priority_frontier() -> Vec<WoogarooGround> {
    let priority_order = [
        WoogarooGroundPriority::UrgentProcedural,
        WoogarooGroundPriority::PrimaryMerits,
        WoogarooGroundPriority::SecondaryMerits,
        WoogarooGroundPriority::SourceOnlySpeculation,
    ];
    let mut frontier = Vec::with_capacity(WOOGAROO_GROUNDS.len());
    for priority in priority_order {
        frontier.extend(WOOGAROO_GROUNDS.iter().copied()
            .filter(|ground| ground.priority == priority));
    }
    frontier
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WoogarooCaseCutError {
    EmptySourceCoordinate,
    DuplicateSource,
    WrongAction,
    UnpaidSourcePresentedAsPaid,
    DuplicateGround,
    GroundPurportsToBePaid,
    ExpectedCaseElementsMissing,
}

/// Validates THIS bounded source-written case. It does not perform a DB read,
/// review the legal evidence or claim that any facts are true in litigation.
pub fn validate_woogaroo_case_cut(
    sources: &[WoogarooSourceLocator],
    grounds: &[WoogarooGround],
) -> Result<(), WoogarooCaseCutError> {
    if sources.len() != WOOGAROO_SOURCE_LOCATORS.len() ||
        grounds.len() != WOOGAROO_GROUNDS.len() {
        return Err(WoogarooCaseCutError::ExpectedCaseElementsMissing);
    }
    if WOOGAROO_PRECONDITIONS.iter().any(|p| {
        p.key.is_empty() || p.required_original.is_empty()
            || p.document_paid || p.competent_decision_paid
    }) {
        return Err(WoogarooCaseCutError::ExpectedCaseElementsMissing);
    }
    let mut source_keys = BTreeSet::new();
    for source in sources {
        if source.key.trim().is_empty()
            || source.originator.trim().is_empty()
            || source.date_label.trim().is_empty()
            || !source.original_url.starts_with("https://")
        {
            return Err(WoogarooCaseCutError::EmptySourceCoordinate);
        }
        if !source_keys.insert(source.key) {
            return Err(WoogarooCaseCutError::DuplicateSource);
        }
        if source.epbc_action != WOOGAROO_ACTION_REF
            && source.epbc_action != SCENIC_ACTION_REF
        {
            return Err(WoogarooCaseCutError::WrongAction);
        }
        if (source.epbc_action == SCENIC_ACTION_REF)
            != source.standing_activity_candidate
        {
            return Err(WoogarooCaseCutError::WrongAction);
        }
        if source.source_or_case_evidence_paid() {
            return Err(WoogarooCaseCutError::UnpaidSourcePresentedAsPaid);
        }
    }
    for required in WOOGAROO_SOURCE_LOCATORS {
        if !sources.iter().any(|s| *s == required) {
            return Err(WoogarooCaseCutError::ExpectedCaseElementsMissing);
        }
    }
    let mut ground_keys = BTreeSet::new();
    for ground in grounds {
        if ground.key.trim().is_empty() || ground.next_original.trim().is_empty()
            || ground.statutory_hook.trim().is_empty() {
            return Err(WoogarooCaseCutError::ExpectedCaseElementsMissing);
        }
        if !ground_keys.insert(ground.key) {
            return Err(WoogarooCaseCutError::DuplicateGround);
        }
        if ground.evidence_gate != 0 || ground.positive_witness_ref.is_some()
            || ground.negative_witness_ref.is_some()
            || ground.court_filing_ref.is_some()
        {
            return Err(WoogarooCaseCutError::GroundPurportsToBePaid);
        }
    }
    for required in WOOGAROO_GROUNDS {
        if !grounds.iter().any(|g| *g == required) {
            return Err(WoogarooCaseCutError::ExpectedCaseElementsMissing);
        }
    }
    Ok(())
}

pub fn woogaroo_locator_matches_springview(key: &str) -> Option<bool> {
    WOOGAROO_SOURCE_LOCATORS.iter().find(|s| s.key == key)
        .map(|source| source.matches_springview_action())
}

const APPROVAL: &str = "prop:woogaroo:public-approval-status";
const ADVICE: &str = "prop:woogaroo:possible-139-2-advice-error";
const HYPOTHETICAL_DEFENCE: &str = "prop:woogaroo:potential-government-consideration";
const BASELINE: &str = "prop:woogaroo:action-specific-local-connectivity";
const DEVELOPER: &str = "prop:woogaroo:stockland-corridor-claims";

fn candidate_proposition(
    ref_id: &str,
    text: &str,
    party: MatterPartyRole,
    role: MatterLegalRole,
    status: MatterEpistemicStatus,
    original_url: &str,
    temporal_ref: &str,
) -> MatterPropositionDraft {
    MatterPropositionDraft {
        proposition_ref: ref_id.into(),
        display_text: text.into(),
        matter_ref: WOOGAROO_MATTER_REF.into(),
        party,
        legal_role: role,
        epistemic_status: status,
        // This is a public URL, NOT the durable source-revision UUID.
        source_ref: original_url.into(),
        reviewed_evidence_ref: None,
        evidence_kind: MatterEvidenceKind::SourceText,
        temporal_ref: temporal_ref.into(),
        relation_ref: "relation:unreviewed-procedural-hypothesis".into(),
        normative_order_ref: "australia:epbc:transitional-s139-unresolved".into(),
    }
}

/// Constructs a typed candidate; does not call persist_matter_controversy.
/// Every hypothetical applicant/respondent proposition says so explicitly.
pub fn woogaroo_candidate_matter() -> MatterControversyDraft {
    let sources = WOOGAROO_SOURCE_LOCATORS;
    MatterControversyDraft {
        controversy_ref: "matter-controversy:woogaroo:pre-filing:v1".into(),
        matter_ref: WOOGAROO_MATTER_REF.into(),
        stage_ref: "stage:source-written-unreviewed-pre-filing".into(),
        supersedes_controversy_ref: None,
        root_proposition_ref: ADVICE.into(),
        propositions: vec![
            candidate_proposition(
                APPROVAL,
                "Commonwealth original portal publishes EPBC 2019/8575 approval with conditions on 15 October 2026. The signed approval/conditions and s95C remain unacquired.",
                MatterPartyRole::ExternalWitness, MatterLegalRole::EvidentiaryFact,
                MatterEpistemicStatus::Supported,
                sources[1].original_url, "time:2026-10-15:portal-decision",
            ),
            candidate_proposition(
                ADVICE,
                "APPLICANT CANDIDATE ONLY: investigate whether the decision-maker failed to consider the applicable 2022 approved endangered-koala conservation advice under the saved s139(2). NOT A FILED PLEADING.",
                MatterPartyRole::Applicant, MatterLegalRole::LegalProposition,
                MatterEpistemicStatus::Unresolved,
                sources[0].original_url, "time:2026-10-15:decision",
            ),
            candidate_proposition(
                HYPOTHETICAL_DEFENCE,
                "HYPOTHETICAL DEFENCE ONLY, NOT A GOVERNMENT ADMISSION: the withheld signed record, 95C or delegate brief may show the proper 2022 advice was actually considered.",
                MatterPartyRole::Respondent, MatterLegalRole::LegalProposition,
                MatterEpistemicStatus::Unresolved,
                sources[3].original_url, "time:unknown:unacquired-decision-file",
            ),
            candidate_proposition(
                BASELINE,
                "APPLICANT CANDIDATE ONLY: action-specific marginal loss of functional koala habitat in an already fragmented ecosystem, excluding other-project impacts as automatically attributable consequences.",
                MatterPartyRole::Applicant, MatterLegalRole::CausalLink,
                MatterEpistemicStatus::Unresolved,
                sources[7].original_url, "time:2026-10-15:habitat-baseline-open",
            ),
            candidate_proposition(
                DEVELOPER,
                "STOCKLAND PUBLISHER'S CLAIM ONLY: retained creek corridor width, staged fauna management and unclear construction commencement. NOT APPROVER'S FACT FINDING.",
                MatterPartyRole::ExternalWitness, MatterLegalRole::EvidentiaryFact,
                MatterEpistemicStatus::Alleged,
                sources[5].original_url, "time:2026-10:developer-publication",
            ),
        ],
        responses: vec![
            MatterResponseDraft {
                response_ref: "response:woogaroo:possible-advice-consideration".into(),
                target_proposition_ref: ADVICE.into(),
                response_proposition_ref: HYPOTHETICAL_DEFENCE.into(),
                mode: MatterResponseMode::ChallengeEvidenceReliability,
            },
            MatterResponseDraft {
                response_ref: "response:woogaroo:published-corridor".into(),
                target_proposition_ref: BASELINE.into(),
                response_proposition_ref: DEVELOPER.into(),
                mode: MatterResponseMode::ChallengeEvidenceReliability,
            },
        ],
        residuals: vec![
            MatterControversyResidualDraft {
                residual_ref: "residual:woogaroo:correct-advice-consideration".into(),
                kind: MatterDisagreementKind::Evidence,
                applicant_proposition_ref: ADVICE.into(),
                respondent_proposition_ref: HYPOTHETICAL_DEFENCE.into(),
                unresolved_question: "Did the actual 15 October EPBC 2019/8575 deciding officer genuinely consider the legally applicable 2022 endangered-koala advice with a correct s139 trigger and saved law?".into(),
                relational_comparison_ref: None,
                relational_obligation_ref: None,
                requested_discriminator_ref: "discriminator:signed-approval:95c:advice-briefing".into(),
                target_evidence_query: "Obtain the exact signed 15 October Part 9 approval, operative conditions, Secretary's 95C report, delegate brief and 2022 koala advice consideration record.".into(),
                potential_reopening_refs: vec![ADVICE.into(), HYPOTHETICAL_DEFENCE.into()],
            },
            MatterControversyResidualDraft {
                residual_ref: "residual:woogaroo:habitat-function-vs-retained-width".into(),
                kind: MatterDisagreementKind::Causal,
                applicant_proposition_ref: BASELINE.into(),
                respondent_proposition_ref: DEVELOPER.into(),
                unresolved_question: "Do signed retained corridor geometry, feeding and refuge trees, offsets, roads and stage-specific works preserve functioning koala movement and viable local connections?".into(),
                relational_comparison_ref: None,
                relational_obligation_ref: None,
                requested_discriminator_ref: "discriminator:approved-corridor:tree-baseline".into(),
                target_evidence_query: "Obtain final Item 4.6(c), signed impact/corridor polygons, original ecological surveys and expert stage-specific connectivity evidence.".into(),
                potential_reopening_refs: vec![BASELINE.into(), DEVELOPER.into()],
            },
        ],
        obligations: vec![
            MatterObligationDraft {
                obligation_ref: "obligation:woogaroo:original-95c-and-advice".into(),
                proposition_ref: ADVICE.into(),
                kind: MatterObligationKind::Evidence,
                required_by_ref: "residual:woogaroo:correct-advice-consideration".into(),
                discharge_ref: None,
            },
            MatterObligationDraft {
                obligation_ref: "obligation:woogaroo:condition-and-field-proof".into(),
                proposition_ref: BASELINE.into(),
                kind: MatterObligationKind::Discriminator,
                required_by_ref: "residual:woogaroo:habitat-function-vs-retained-width".into(),
                discharge_ref: None,
            },
        ],
    }
}

pub fn validate_woogaroo_candidate_matter() -> Result<(), MatterControversyError> {
    validate_matter_controversy_draft(&woogaroo_candidate_matter())
}
