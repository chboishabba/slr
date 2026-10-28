//! EPBC 2019/8575: case-scoped max-cut and 80 source-gated closure targets.
//! Thin consumer of existing SLR Woogaroo ground owners, not a new payment,
//! source-review, Court, or evidence-admission engine. All targets remain unpaid.
//! SensibLaw owns its historical original prototype. SLR owns production.
//! Original lawmakers, scientific authors, courts, developer and prior PR authors
//! retain separate authorship. Johl Brown directed synthesis; ChatGPT assisted.

use std::collections::BTreeSet;
use crate::{
    potential_reopening_cone, InvestigationAcquisitionError, PotentialReopeningCone,
    WOOGAROO_GROUNDS,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoogarooClosureClass { Identity, Provenance, Procedure, DecisionFile, Ground, Ecology, Verification, Outcome }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooClosureObligation {
    pub id: &'static str,
    pub title: &'static str,
    pub class: WoogarooClosureClass,
    /// Dominant *next* original/action, not sufficient evidence by itself.
    pub primary_packet: &'static str,
    pub ground_key: Option<&'static str>,
    /// Template never substitutes for competent source or legal review.
    pub discharged: bool,
}
macro_rules! obligation {
    ($id:expr, $title:expr, $class:ident, $packet:expr, $ground:expr) => {
        WoogarooClosureObligation {
            id: $id, title: $title, class: WoogarooClosureClass::$class,
            primary_packet: $packet, ground_key: $ground, discharged: false,
        }
    };
}
/// Eight identity, eight provenance, eight procedure, eight decision-file,
/// twenty-eight ground, eight ecology, seven verification, five outcome targets.
pub const WOOGAROO_CLOSURE_OBLIGATIONS: [WoogarooClosureObligation; 80] = [
    obligation!("I01", "Same Springview action", Identity, "signed", None),
    obligation!("I02", "Distinct Scenic merits", Identity, "signed", None),
    obligation!("I03", "Same parcel/stage/works", Identity, "works", None),
    obligation!("I04", "Public approval notice identity", Identity, "signed", None),
    obligation!("I05", "Original signed decision identity", Identity, "signed", None),
    obligation!("I06", "Actual delegate authority", Identity, "signed", None),
    obligation!("I07", "2025-26 saved law", Identity, "brief", None),
    obligation!("I08", "Controlling matter applicability", Identity, "brief", None),
    obligation!("E01", "Original bytes retained", Provenance, "source", None),
    obligation!("E02", "Immutable revision identity", Provenance, "source", None),
    obligation!("E03", "Canonical content digest", Provenance, "source", None),
    obligation!("E04", "Exact paid source span", Provenance, "source", None),
    obligation!("E05", "Author and proof ownership", Provenance, "source", None),
    obligation!("E06", "Publication not delivery", Provenance, "source", None),
    obligation!("E07", "Competent human review", Provenance, "source", None),
    obligation!("E08", "No invented authority promotion", Provenance, "source", None),
    obligation!("P01", "Applicant eligibility", Procedure, "counsel", None),
    obligation!("P02", "Two-year environmental activity", Procedure, "counsel", None),
    obligation!("P03", "Historic association objects", Procedure, "counsel", None),
    obligation!("P04", "Authority to instruct counsel", Procedure, "counsel", None),
    obligation!("P05", "Court jurisdiction", Procedure, "counsel", None),
    obligation!("P06", "Notification/reasons clock", Procedure, "counsel", None),
    obligation!("P07", "Actual filing limit or extension", Procedure, "counsel", None),
    obligation!("P08", "Urgency and interim relief", Procedure, "works", None),
    obligation!("D01", "Lawful public comment invitation", DecisionFile, "comments", None),
    obligation!("D02", "Original comments received", DecisionFile, "comments", None),
    obligation!("D03", "Final preliminary documents", DecisionFile, "comments", None),
    obligation!("D04", "Actual section 95B transmission", DecisionFile, "comments", None),
    obligation!("D05", "Section 95C report", DecisionFile, "brief", None),
    obligation!("D06", "Decision briefing chain", DecisionFile, "brief", None),
    obligation!("D07", "Section 136 consideration", DecisionFile, "brief", None),
    obligation!("D08", "Final conditions correspondence", DecisionFile, "signed", None),
    obligation!("G01", "Section 139(2) trigger", Ground, "brief", Some("advice-consideration")),
    obligation!("G02", "Applicable 2022 conservation advice", Ground, "brief", Some("advice-consideration")),
    obligation!("G03", "Genuine delegate consideration", Ground, "brief", Some("advice-consideration")),
    obligation!("G04", "Legal error materiality and remedy", Ground, "brief", Some("advice-consideration")),
    obligation!("G05", "Applicable recovery plan", Ground, "plan", Some("recovery-plan")),
    obligation!("G06", "Operative plan requirement", Ground, "plan", Some("recovery-plan")),
    obligation!("G07", "Actual conditions inconsistency", Ground, "plan", Some("recovery-plan")),
    obligation!("G08", "Reconciliation/adverse remedy", Ground, "plan", Some("recovery-plan")),
    obligation!("G09", "Local habitat baseline", Ground, "ecology", Some("incremental-habitat")),
    obligation!("G10", "Action-attributable incremental loss", Ground, "ecology", Some("incremental-habitat")),
    obligation!("G11", "Material assessment error", Ground, "ecology", Some("incremental-habitat")),
    obligation!("G12", "Mitigation/corridor countercase", Ground, "ecology", Some("incremental-habitat")),
    obligation!("G13", "Original received comment set", Ground, "comments", Some("comment-transmission")),
    obligation!("G14", "Required transmitted copies", Ground, "comments", Some("comment-transmission")),
    obligation!("G15", "Section 95C sequence", Ground, "comments", Some("comment-transmission")),
    obligation!("G16", "Procedural error and relief", Ground, "comments", Some("comment-transmission")),
    obligation!("G17", "Referred and approved action match", Ground, "signed", Some("approval-scope")),
    obligation!("G18", "Authoritative clearing polygon", Ground, "signed", Some("approval-scope")),
    obligation!("G19", "Condition certainty and enforceability", Ground, "signed", Some("approval-scope")),
    obligation!("G20", "Scope adverse-authority test", Ground, "signed", Some("approval-scope")),
    obligation!("G21", "Condition 6(a) operative text", Ground, "works", Some("local-prestart")),
    obligation!("G22", "Same-stage works timing", Ground, "works", Some("local-prestart")),
    obligation!("G23", "Prestart Council satisfaction", Ground, "works", Some("local-prestart")),
    obligation!("G24", "State breach/remedy separation", Ground, "works", Some("local-prestart")),
    obligation!("G25", "Decision actor/interest identity", Ground, "conflict", Some("improper-influence")),
    obligation!("G26", "Decision-specific contact/channel", Ground, "conflict", Some("improper-influence")),
    obligation!("G27", "Applicable improper-purpose/bias test", Ground, "conflict", Some("improper-influence")),
    obligation!("G28", "Causation and independent countercase", Ground, "conflict", Some("improper-influence")),
    obligation!("F01", "Mapped approved clearing", Ecology, "ecology", None),
    obligation!("F02", "Mature canopy loss", Ecology, "ecology", None),
    obligation!("F03", "Thermal refuge change", Ecology, "ecology", None),
    obligation!("F04", "Movement connectivity graph", Ecology, "ecology", None),
    obligation!("F05", "Crossing and road risk", Ecology, "ecology", None),
    obligation!("F06", "Regional baseline sensitivity", Ecology, "ecology", None),
    obligation!("F07", "Offset temporal additionality", Ecology, "ecology", None),
    obligation!("F08", "Ecology-to-law mapping", Ecology, "ecology", None),
    obligation!("V01", "Canonical owner reuse", Verification, "runtime", None),
    obligation!("V02", "Seven-ground exact crosswalk", Verification, "runtime", None),
    obligation!("V03", "Candidate not persisted evidence", Verification, "runtime", None),
    obligation!("V04", "Adversarial parties retained", Verification, "runtime", None),
    obligation!("V05", "Sound reverse evidence queries", Verification, "runtime", None),
    obligation!("V06", "Executed Rust/Python/Agda checks", Verification, "runtime", None),
    obligation!("V07", "DASHI root and attribution", Verification, "runtime", None),
    obligation!("O01", "Counsel pleadable/no-file decision", Outcome, "disposition", None),
    obligation!("O02", "Actual protective route disposition", Outcome, "disposition", None),
    obligation!("O03", "Actual regulatory disposition", Outcome, "disposition", None),
    obligation!("O04", "Durable habitat protection status", Outcome, "monitor", None),
    obligation!("O05", "Reproducible physical outcome ledger", Outcome, "monitor", None),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooAcquisitionPacket {
    pub key: &'static str,
    pub next_original_or_action: &'static str,
    pub non_promotion_rule: &'static str,
    /// Urgency/dependency order; not merits weight or probability.
    pub ordinal_priority: u8,
}
pub const WOOGAROO_ACQUISITION_PACKETS: [WoogarooAcquisitionPacket; 12] = [
    WoogarooAcquisitionPacket { key: "counsel", next_original_or_action: "Applicant identity, original association records, jurisdiction and counsel assessment", non_promotion_rule: "No universal filing deadline", ordinal_priority: 0 },
    WoogarooAcquisitionPacket { key: "signed", next_original_or_action: "Original signed approval, conditions, actual delegate and exact action plans", non_promotion_rule: "Publication is not the instrument", ordinal_priority: 1 },
    WoogarooAcquisitionPacket { key: "brief", next_original_or_action: "Section 95C, delegate brief, 2022 advice and applicable saved law", non_promotion_rule: "Existence does not prove genuine consideration", ordinal_priority: 2 },
    WoogarooAcquisitionPacket { key: "comments", next_original_or_action: "Section 95B actual comments, transmissions and final preliminary documentation", non_promotion_rule: "Count is not received/transmitted copies", ordinal_priority: 3 },
    WoogarooAcquisitionPacket { key: "works", next_original_or_action: "Same-stage Council acceptance, dated works and clearing evidence", non_promotion_rule: "No published start date is not no threat", ordinal_priority: 4 },
    WoogarooAcquisitionPacket { key: "plan", next_original_or_action: "Applicable 2022 recovery plan and actual condition comparison", non_promotion_rule: "Plan aspiration is not an automatic veto", ordinal_priority: 5 },
    WoogarooAcquisitionPacket { key: "ecology", next_original_or_action: "Same-site expert canopy, connectivity, offset and crossing evidence", non_promotion_rule: "Ecological disagreement is not itself legal error", ordinal_priority: 6 },
    WoogarooAcquisitionPacket { key: "source", next_original_or_action: "Retained original revisions and reviewed exact spans in existing SLR store", non_promotion_rule: "URL is not an evidence payment", ordinal_priority: 7 },
    WoogarooAcquisitionPacket { key: "runtime", next_original_or_action: "Executed Rust/Python/Agda and store acceptance checks", non_promotion_rule: "Source-written is not kernel-verified", ordinal_priority: 8 },
    WoogarooAcquisitionPacket { key: "disposition", next_original_or_action: "Counsel decision, filed order or regulator response", non_promotion_rule: "Draft is not actual proceedings", ordinal_priority: 9 },
    WoogarooAcquisitionPacket { key: "monitor", next_original_or_action: "Enforceable habitat arrangements and measured outcome", non_promotion_rule: "Mapping is not actual physical protection", ordinal_priority: 10 },
    WoogarooAcquisitionPacket { key: "conflict", next_original_or_action: "Decision-specific original communications, conflicts and legal elements", non_promotion_rule: "Financial adjacency is not corruption", ordinal_priority: 11 },
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WoogarooCutFrontier {
    pub packet_key: &'static str,
    pub next_original_or_action: &'static str,
    pub ordinal_priority: u8,
    /// Only source-written outstanding obligations, not evidence payments.
    pub unpaid_obligations: Vec<&'static str>,
}

/// Pure Pareto/max-cut view; does not access, persist, or promote evidence.
pub fn woogaroo_closure_frontier() -> Vec<WoogarooCutFrontier> {
    WOOGAROO_ACQUISITION_PACKETS.iter().map(|p| WoogarooCutFrontier {
        packet_key: p.key,
        next_original_or_action: p.next_original_or_action,
        ordinal_priority: p.ordinal_priority,
        unpaid_obligations: WOOGAROO_CLOSURE_OBLIGATIONS.iter()
            .filter(|o| o.primary_packet == p.key && !o.discharged)
            .map(|o| o.id).collect(),
    }).collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WoogarooClosureError {
    WrongCanonicalObligation, DuplicateObligation, ImproperDischarge,
    WrongCanonicalPacket, InvalidPriority, UncoveredObligation, UnknownGround,
}
/// Candidate-only manifest validation, not factual, forensic or legal review.
pub fn validate_woogaroo_closure_contract(
    obligations: &[WoogarooClosureObligation],
    packets: &[WoogarooAcquisitionPacket],
) -> Result<(), WoogarooClosureError> {
    if obligations.len() != WOOGAROO_CLOSURE_OBLIGATIONS.len() {
        return Err(WoogarooClosureError::WrongCanonicalObligation);
    }
    if packets.len() != WOOGAROO_ACQUISITION_PACKETS.len() {
        return Err(WoogarooClosureError::WrongCanonicalPacket);
    }
    let mut packet_keys = BTreeSet::new();
    for (index, p) in packets.iter().enumerate() {
        if p.key.is_empty() || p.next_original_or_action.is_empty()
            || p.non_promotion_rule.is_empty()
            || p.ordinal_priority != index as u8 || !packet_keys.insert(p.key) {
            return Err(WoogarooClosureError::InvalidPriority);
        }
        if *p != WOOGAROO_ACQUISITION_PACKETS[index] {
            return Err(WoogarooClosureError::WrongCanonicalPacket);
        }
    }
    let mut obligation_keys = BTreeSet::new();
    for o in obligations {
        if !obligation_keys.insert(o.id) {
            return Err(WoogarooClosureError::DuplicateObligation);
        }
        if o.discharged {
            return Err(WoogarooClosureError::ImproperDischarge);
        }
        if !packet_keys.contains(o.primary_packet) {
            return Err(WoogarooClosureError::UncoveredObligation);
        }
        if o.class == WoogarooClosureClass::Ground {
            if !o.ground_key.is_some_and(|key| WOOGAROO_GROUNDS.iter().any(|g| g.key == key)) {
                return Err(WoogarooClosureError::UnknownGround);
            }
        } else if o.ground_key.is_some() {
            return Err(WoogarooClosureError::UnknownGround);
        }
    }
    for original in WOOGAROO_CLOSURE_OBLIGATIONS {
        if !obligations.contains(&original) {
            return Err(WoogarooClosureError::WrongCanonicalObligation);
        }
    }
    for ground in WOOGAROO_GROUNDS {
        if obligations.iter().filter(|o| o.ground_key == Some(ground.key)).count() != 4 {
            return Err(WoogarooClosureError::UnknownGround);
        }
    }
    Ok(())
}


/// Source-specific snowball *candidate* cone through the canonical SLR
/// investigation graph. Acquiring a record would make these targets worth
/// reviewing; this projection DOES NOT acquire or review it, create a
/// persisted reopening receipt, or promote any ecological/legal claim.
pub fn woogaroo_potential_reopening_cone(
    packet_key: &str,
) -> Result<PotentialReopeningCone, InvestigationAcquisitionError> {
    if !WOOGAROO_ACQUISITION_PACKETS.iter().any(|p| p.key == packet_key) {
        return Err(InvestigationAcquisitionError::InvalidIdentity);
    }
    let mut edges = Vec::<(String, String)>::new();
    for obligation in WOOGAROO_CLOSURE_OBLIGATIONS {
        let target = format!("obligation:woogaroo:{}", obligation.id);
        edges.push((format!("packet:woogaroo:{}", obligation.primary_packet), target.clone()));
        if let Some(ground) = obligation.ground_key {
            edges.push((target, format!("ground:woogaroo:{ground}")));
        }
    }
    for ground in WOOGAROO_GROUNDS {
        edges.push((
            format!("ground:woogaroo:{}", ground.key),
            "matter:woogaroo:candidate-seven-ground-review".to_owned(),
        ));
    }
    potential_reopening_cone(&format!("packet:woogaroo:{packet_key}"), &edges)
}


// Seven-ground applicant/respondent LOGIC ONLY. This mirrors the DASHI
// counterfactual truth-table, not a legal sufficiency rule or source payment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooHypotheticalWorld {
    pub legally_applicable: bool,
    pub defect_established: bool,
    pub materiality_established: bool,
    pub government_compliance_established: bool,
    pub evidence_reviewed: bool,
    pub ecological_harm_assumed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WoogarooHypotheticalBranches {
    pub ground_key: &'static str,
    pub applicant_conditional: bool,
    pub respondent_conditional: bool,
    /// Independent source assertions can conflict. No adjudicative promotion.
    pub contradictory_assumptions_retained: bool,
    /// Always false: this function cannot materialize evidence or decide law.
    pub proves_actual_legal_merits: bool,
    pub creates_legal_evidence_review: bool,
    pub creates_court_filing: bool,
}

/// No synthetic or other-referral ground identifiers are accepted.
/// Positive branches mean only that *stipulated inputs* satisfy this
/// finite test; neither branch entails a Woogaroo fact or court judgment.
pub fn woogaroo_hypothetical_adversarial_test(
    ground_key: &str,
    world: WoogarooHypotheticalWorld,
) -> Option<WoogarooHypotheticalBranches> {
    let canonical = WOOGAROO_GROUNDS.iter()
        .find(|ground| ground.key == ground_key)?.key;
    let applicant = !world.government_compliance_established
        && world.evidence_reviewed
        && world.legally_applicable
        && world.defect_established
        && world.materiality_established;
    let contradiction = world.defect_established
        && world.government_compliance_established;
    let respondent = !world.defect_established
        && world.government_compliance_established
        && world.evidence_reviewed
        && world.legally_applicable;
    Some(WoogarooHypotheticalBranches {
        ground_key: canonical,
        applicant_conditional: applicant,
        respondent_conditional: respondent,
        contradictory_assumptions_retained: contradiction,
        proves_actual_legal_merits: false,
        creates_legal_evidence_review: false,
        creates_court_filing: false,
    })
}


// -------------------------------------------------------------------------
// Woogaroo dialectic / trialectic: synthesis of outputs, NOT of legal truth.
// Reuses the existing Woogaroo seven-ground original owner, the explicit
// hypothetical evaluator, and canonical SLR reverse-search procedural goal.
// The independent third lens is evidence/law/ecology appraisal, NOT another
// litigating party. The irreducible face stays unreviewed/underdetermined.
// Match DASHI/Law/SensibLawWoogarooDialecticTrialecticSynthesis20261028Exact.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum WoogarooSynthesisKind {
    SeekOriginals,
    ApplicantCandidateForDefeaterSearch,
    RespondentCandidateForCounterDefeaterSearch,
    ReconcileOpposingSourceAssertions,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum WoogarooDialecticEdge {
    ProvisionalApplicant,
    Unresolved,
    ProvisionalRespondent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum WoogarooTrialecticFace {
    Underdetermined,
}


// --------------------------------------------------------------------------
// Finite 369 / 27-cube semantic chart. Reuses the existing 7-ground world
// model: no learned Sophontic representation, E8/3B action or GIS coordinates.
// Original mathematical carrier and non-descent owner is DASHI Agda.
// Donor paired evaluation protocol: Sophontic Inc. public 2026 eval kit;
// DASHI PRs #1094/#1095/#1265, unmerged and NOT imported into SLR.
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct Woogaroo369GeometricChart {
    /// Three T^3 rows; 0 means hypothetical flag absent, 1 means present.
    /// This Boolean embedding touches a subset of each 27-point trit cube.
    pub applicant_cube: [i8; 3],
    pub respondent_cube: [i8; 3],
    pub independent_review_cube: [i8; 3],
    /// Ambient cardinalities, NOT measured data or a sample count.
    pub full_cube_cardinality: usize,
    pub full_fabric_cardinality: usize,
    pub learned_geometric_action_established: bool,
    pub same_object_monster_action_established: bool,
    pub actual_evidence_payment_established: bool,
}

fn woogaroo_bool_to_trit(flag: bool) -> i8 {
    if flag { 1 } else { 0 }
}

/// Three canonical 27-cell roles; the three derived coordinates in the
/// independent-review row do not create three additional source facts.
pub fn woogaroo_369_chart(world: WoogarooHypotheticalWorld) -> Woogaroo369GeometricChart {
    let b = woogaroo_bool_to_trit;
    let contradiction = world.defect_established && world.government_compliance_established;
    let applicant = !world.government_compliance_established
        && world.evidence_reviewed
        && world.legally_applicable
        && world.defect_established
        && world.materiality_established;
    let respondent = !world.defect_established
        && world.government_compliance_established
        && world.evidence_reviewed
        && world.legally_applicable;
    Woogaroo369GeometricChart {
        applicant_cube: [b(world.legally_applicable), b(world.defect_established),
            b(world.materiality_established)],
        respondent_cube: [b(world.government_compliance_established),
            b(world.ecological_harm_assumed), b(contradiction)],
        independent_review_cube: [b(world.evidence_reviewed), b(applicant), b(respondent)],
        full_cube_cardinality: 27,
        full_fabric_cardinality: 19683,
        learned_geometric_action_established: false,
        same_object_monster_action_established: false,
        actual_evidence_payment_established: false,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum WoogarooGeometricIntervention {
    /// Change only the hypothetical reviewed-evidence gate.
    ReviewGate,
    /// Change only the habitat-harm assumption, leaving the specific
    /// procedural/merits logic untouched. This is a nuisance test FOR
    /// THIS STIPULATED LOGICAL CLASSIFIER, not for ecological science.
    EcologicalHarm,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooGeometricPair {
    pub ground_key: &'static str,
    pub intervention: WoogarooGeometricIntervention,
    pub canonical: Woogaroo369GeometricChart,
    pub perturbed: Woogaroo369GeometricChart,
    pub initial_synthesis: WoogarooSynthesisKind,
    pub perturbed_synthesis: WoogarooSynthesisKind,
    pub changed_369_position: bool,
    /// Manhattan grid steps on the existing finite ternary lattice.
    /// This is NOT metres, p-value, or source evidence.
    pub ternary_grid_steps: usize,
    pub changed_logical_label: bool,
    pub model_pair_accuracy_measured: bool,
    pub actual_legal_merits_determined: bool,
}

/// A reference-model paired assay, not a trained reasoner's flip-rate.
/// Reject paired rows that change more than the selected load-bearing or
/// nuisance input. A review-gate flip need not change the output if the
/// other required legal premises are absent: this is explicitly recorded.
pub fn woogaroo_geometric_pair(
    ground_key: &str,
    before: WoogarooHypotheticalWorld,
    after: WoogarooHypotheticalWorld,
    intervention: WoogarooGeometricIntervention,
) -> Option<WoogarooGeometricPair> {
    let mut expected = before;
    match intervention {
        WoogarooGeometricIntervention::ReviewGate => {
            expected.evidence_reviewed = !expected.evidence_reviewed;
        }
        WoogarooGeometricIntervention::EcologicalHarm => {
            expected.ecological_harm_assumed = !expected.ecological_harm_assumed;
        }
    }
    if expected != after { return None; }
    let before_packet = woogaroo_synthesise_dialectic_trialectic(ground_key, before)?;
    let after_packet = woogaroo_synthesise_dialectic_trialectic(ground_key, after)?;
    let canonical = woogaroo_369_chart(before);
    let perturbed = woogaroo_369_chart(after);
    let ternary_grid_steps = [
        (canonical.applicant_cube, perturbed.applicant_cube),
        (canonical.respondent_cube, perturbed.respondent_cube),
        (canonical.independent_review_cube, perturbed.independent_review_cube),
    ].iter().flat_map(|(left, right)| left.iter().zip(right.iter()))
        .map(|(x, y)| ((*x as i32) - (*y as i32)).abs() as usize)
        .sum();
    Some(WoogarooGeometricPair {
        ground_key: before_packet.ground_key,
        intervention,
        changed_369_position: canonical != perturbed,
        ternary_grid_steps,
        changed_logical_label: before_packet.kind != after_packet.kind,
        initial_synthesis: before_packet.kind,
        perturbed_synthesis: after_packet.kind,
        canonical,
        perturbed,
        model_pair_accuracy_measured: false,
        actual_legal_merits_determined: false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSynthesisPacket {
    pub ground_key: &'static str,
    pub applicant_issue: &'static str,
    pub respondent_countercase: &'static str,
    pub decisive_original: &'static str,
    /// Position in the pre-existing 3 × 27 finite triadic carrier.
    pub geometric_369: Woogaroo369GeometricChart,
    /// Existing 80-obligation canonical max-cut acquisition packet.
    pub original_acquisition_packet_key: &'static str,
    /// Exactly four still-unpaid G01..G28 closure IDs for this ground.
    pub open_ground_obligation_ids: Vec<&'static str>,
    /// Its packet-to-ground consumer is potential-only, never a DB reopen.
    pub creates_actual_reopening: bool,
    pub kind: WoogarooSynthesisKind,
    /// Applicant/respondent disagreement. This is NOT Boolean negation.
    pub applicant_respondent_edge: WoogarooDialecticEdge,
    /// Independent evidence/legal/ecological appraisal is a third lens.
    pub respondent_independent_edge: WoogarooDialecticEdge,
    pub independent_applicant_edge: WoogarooDialecticEdge,
    /// Existing trialectic theory forbids computing this from three edges.
    pub triadic_face: WoogarooTrialecticFace,
    pub next_procedural_goal: crate::MatterProceduralGoal,
    pub candidate_only: bool,
    pub creates_actual_review: bool,
    pub creates_court_filing: bool,
    pub proves_legal_error_or_compliance: bool,
    pub orders_physical_protection: bool,
}

/// Source-attributed hypotheses, not respondent admissions. These hints
/// paraphrase the already-owned DASHI seven-ground defence/falsifier matrix.
fn woogaroo_countercase_hint(ground_key: &str) -> Option<&'static str> {
    Some(match ground_key {
        "advice-consideration" =>
            "The actual delegate may have considered the correct 2022 advice under the saved statutory trigger.",
        "recovery-plan" =>
            "The operative 2022 recovery plan may be consistent with a conditioned action; objectives are not an automatic refusal.",
        "incremental-habitat" =>
            "The action-specific baseline, retained corridor and mitigation may withstand review; unrelated projects are not automatically its impacts.",
        "comment-transmission" =>
            "The Secretary may have received and transmitted all comments and the final PD although the portal does not show the records.",
        "approval-scope" =>
            "The signed decision and stage plans may match the referral with legally ascertainable enforceable conditions.",
        "local-prestart" =>
            "Council may have accepted conditions before same-stage works, or the relevant works may not yet have commenced.",
        "improper-influence" =>
            "The record may show lawful decision-making with no decision-specific improper influence; adjacency is not proof.",
        _ => return None,
    })
}

/// Candidate synthesis: for, against, and independent reviewer remain
/// distinct. It neither changes the persisted SLR Matter nor accepts
/// invented source/review/counsel authority from a caller-supplied world.
pub fn woogaroo_synthesise_dialectic_trialectic(
    ground_key: &str,
    world: WoogarooHypotheticalWorld,
) -> Option<WoogarooSynthesisPacket> {
    let ground = WOOGAROO_GROUNDS.iter().find(|g| g.key == ground_key)?;
    let comparison = woogaroo_hypothetical_adversarial_test(ground_key, world)?;
    let obligations = WOOGAROO_CLOSURE_OBLIGATIONS.iter()
        .filter(|o| o.ground_key == Some(ground.key))
        .collect::<Vec<_>>();
    if obligations.len() != 4 || obligations.iter().any(|o| o.discharged) {
        return None;
    }
    let acquisition_key = obligations.first()?.primary_packet;
    if !obligations.iter().all(|o| o.primary_packet == acquisition_key) {
        return None;
    }
    let packet = WOOGAROO_ACQUISITION_PACKETS.iter()
        .find(|p| p.key == acquisition_key)?;
    let cone = woogaroo_potential_reopening_cone(packet.key).ok()?;
    if cone.creates_actual_reopening || cone.creates_semantic_authority
        || !obligations.iter().all(|o| cone.direct_refs.contains(
            &format!("obligation:woogaroo:{}", o.id))) {
        return None;
    }
    let kind = if comparison.contradictory_assumptions_retained {
        WoogarooSynthesisKind::ReconcileOpposingSourceAssertions
    } else if comparison.applicant_conditional {
        WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch
    } else if comparison.respondent_conditional {
        WoogarooSynthesisKind::RespondentCandidateForCounterDefeaterSearch
    } else {
        WoogarooSynthesisKind::SeekOriginals
    };
    let edge = match kind {
        WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch =>
            WoogarooDialecticEdge::ProvisionalApplicant,
        WoogarooSynthesisKind::RespondentCandidateForCounterDefeaterSearch =>
            WoogarooDialecticEdge::ProvisionalRespondent,
        _ => WoogarooDialecticEdge::Unresolved,
    };
    let goal = match kind {
        WoogarooSynthesisKind::ApplicantCandidateForDefeaterSearch
        | WoogarooSynthesisKind::RespondentCandidateForCounterDefeaterSearch =>
            crate::MatterProceduralGoal::IsolateResidualControversy,
        WoogarooSynthesisKind::SeekOriginals
        | WoogarooSynthesisKind::ReconcileOpposingSourceAssertions =>
            crate::MatterProceduralGoal::DecideEvidenceNeeded,
    };
    Some(WoogarooSynthesisPacket {
        ground_key: ground.key,
        applicant_issue: ground.statutory_hook,
        respondent_countercase: woogaroo_countercase_hint(ground.key)?,
        decisive_original: ground.next_original,
        geometric_369: woogaroo_369_chart(world),
        original_acquisition_packet_key: packet.key,
        open_ground_obligation_ids: obligations.iter().map(|o| o.id).collect(),
        creates_actual_reopening: false,
        kind,
        applicant_respondent_edge: edge,
        respondent_independent_edge: WoogarooDialecticEdge::Unresolved,
        independent_applicant_edge: WoogarooDialecticEdge::Unresolved,
        triadic_face: WoogarooTrialecticFace::Underdetermined,
        next_procedural_goal: goal,
        candidate_only: true,
        creates_actual_review: false,
        creates_court_filing: false,
        proves_legal_error_or_compliance: false,
        orders_physical_protection: false,
    })
}

/// Seven actual-current *source-state research packets*. All six false values
/// denote NO REVIEWED PROOF HERE, not factual falsehood of the legal elements.
/// This list cannot be used as a filed or legally decided controversy.
pub fn woogaroo_unpaid_trialectic_synthesis() -> Vec<WoogarooSynthesisPacket> {
    let unpaid = WoogarooHypotheticalWorld {
        legally_applicable: false,
        defect_established: false,
        materiality_established: false,
        government_compliance_established: false,
        evidence_reviewed: false,
        ecological_harm_assumed: false,
    };
    WOOGAROO_GROUNDS.iter().filter_map(|g| {
        if g.evidence_gate == 0 {
            woogaroo_synthesise_dialectic_trialectic(g.key, unpaid)
        } else {
            None
        }
    }).collect()
}


// -------------------------------------------------------------------------
// SIGNED PROPOSITION FIBRES: −P is explicit evidence for the negative
// version of SAME proposition; +P is evidence for P; 0 is coarse abstention.
// "Both" and "neither" share coarse 0 but retain separate fine receipts.
// Canonical owners are DASHI ClaimIndexedEvidencePolarityExact,
// SignedPredicateFibreTreeExact, TypedHyperfabricCore, and
// TriadicPAdicCylinderExact. No signed evidence is invented from a
// false Boolean in the older hypothetical-gate reference model.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WoogarooEvidenceStatus {
    Neither,
    PositiveOnly,
    NegativeOnly,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSignedEvidence {
    pub positive_supported: bool,
    pub negative_supported: bool,
    /// Locator strings are NOT payment receipts or proof of document review.
    pub original_reference_labels: Vec<String>,
}

impl WoogarooSignedEvidence {
    pub fn unknown() -> Self {
        Self {
            positive_supported: false,
            negative_supported: false,
            original_reference_labels: vec![],
        }
    }

    pub fn status(&self) -> WoogarooEvidenceStatus {
        match (self.positive_supported, self.negative_supported) {
            (false, false) => WoogarooEvidenceStatus::Neither,
            (true, false) => WoogarooEvidenceStatus::PositiveOnly,
            (false, true) => WoogarooEvidenceStatus::NegativeOnly,
            (true, true) => WoogarooEvidenceStatus::Both,
        }
    }

    pub fn coarse_signed_trit(&self) -> i8 {
        match self.status() {
            WoogarooEvidenceStatus::PositiveOnly => 1,
            WoogarooEvidenceStatus::NegativeOnly => -1,
            WoogarooEvidenceStatus::Neither | WoogarooEvidenceStatus::Both => 0,
        }
    }

    /// Merge only *after* the caller has supplied a same-proposition,
    /// same-ground, same-scope alignment: no global blind evidence pooling.
    fn merge_same_claim(&self, other: &Self) -> Self {
        let mut refs = self.original_reference_labels.clone();
        refs.extend(other.original_reference_labels.iter().cloned());
        refs.sort();
        refs.dedup();
        Self {
            positive_supported: self.positive_supported || other.positive_supported,
            negative_supported: self.negative_supported || other.negative_supported,
            original_reference_labels: refs,
        }
    }
}

/// Evidence union is permitted only for the same ground, SAME predicate,
/// same project/scope, and a declared alignment. Matching numeric signs
/// alone never licenses a cross-ground or cross-project join.
/// Alignment labels are candidate declarations, not actual source payments.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooScopedSignedEvidence {
    pub ground_key: String,
    pub predicate_key: String,
    pub project_and_scope_ref: String,
    pub evidence: WoogarooSignedEvidence,
}

impl WoogarooScopedSignedEvidence {
    pub fn merge_aligned(&self, other: &Self) -> Option<Self> {
        if self.ground_key != other.ground_key
            || self.predicate_key != other.predicate_key
            || self.project_and_scope_ref != other.project_and_scope_ref
            || self.project_and_scope_ref.trim().is_empty()
            || !WOOGAROO_GROUNDS.iter().any(|g| g.key == self.ground_key)
            || !WOOGAROO_SIGNED_PREDICATES.contains(&self.predicate_key.as_str())
            || self.project_and_scope_ref !=
                format!("epbc:2019/8575:{}",self.ground_key)
        { return None; }
        Some(Self {
            ground_key: self.ground_key.clone(),
            predicate_key: self.predicate_key.clone(),
            project_and_scope_ref: self.project_and_scope_ref.clone(),
            evidence: self.evidence.merge_same_claim(&other.evidence),
        })
    }
}

pub const WOOGAROO_SIGNED_PREDICATES: [&str; 9] = [
    "applicable-law", "alleged-defect", "materiality",
    "claimed-compliance", "habitat-concern", "causal-connection",
    "reviewed-original", "same-project-stage", "independent-mediation",
];

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSignedClaimFibre {
    pub ground_key: &'static str,
    pub proposition_scope: String,
    /// All nine are separate propositions with explicit polarity pairs.
    pub evidence: Vec<WoogarooSignedEvidence>,
    pub candidate_only: bool,
    pub source_reviewed_and_paid: bool,
}

impl WoogarooSignedClaimFibre {
    pub fn unpaid(ground_key: &'static str) -> Self {
        Self {
            ground_key,
            proposition_scope: format!("epbc:2019/8575:{ground_key}"),
            evidence: vec![WoogarooSignedEvidence::unknown(); 9],
            candidate_only: true,
            source_reviewed_and_paid: false,
        }
    }

    pub fn signed_chart(&self) -> Option<WoogarooSigned369Chart> {
        if self.evidence.len() != 9 { return None; }
        let values: Vec<_> = self.evidence.iter().map(|e| e.coarse_signed_trit()).collect();
        let conflicts = self.evidence.iter().enumerate()
            .filter(|(_,e)| e.status() == WoogarooEvidenceStatus::Both)
            .map(|(i,_)| WOOGAROO_SIGNED_PREDICATES[i].to_string()).collect();
        Some(WoogarooSigned369Chart {
            applicant_cube: [values[0],values[1],values[2]],
            respondent_cube: [values[3],values[4],values[5]],
            independent_cube: [values[6],values[7],values[8]],
            conflict_predicate_keys: conflicts,
            image_is_only_a_coarse_observation: true,
            actual_source_payment: false,
            actual_mediation_face_glued: false,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSigned369Chart {
    /// These are true {-1,0,+1} signed-proposition coordinates, NOT
    /// the legacy 0/1 hypothetical input-gate fixture.
    pub applicant_cube: [i8; 3],
    pub respondent_cube: [i8; 3],
    pub independent_cube: [i8; 3],
    /// The 0-valued "both" residual is retained here instead of erased.
    pub conflict_predicate_keys: Vec<String>,
    pub image_is_only_a_coarse_observation: bool,
    pub actual_source_payment: bool,
    pub actual_mediation_face_glued: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSignedDecisionTree {
    pub predicate: &'static str,
    pub evidence: WoogarooSignedEvidence,
    pub children: Vec<WoogarooSignedDecisionTree>,
}

fn signed_tree_slot(fibre: &WoogarooSignedClaimFibre, i: usize) -> WoogarooSignedDecisionTree {
    WoogarooSignedDecisionTree {
        predicate: WOOGAROO_SIGNED_PREDICATES[i],
        evidence: fibre.evidence[i].clone(),
        children: vec![],
    }
}

/// Three hierarchy branches contain nested predicates. The raw nine
/// fine-fibre assessments are preserved at EVERY leaf, not collapsed
/// into a signed cube or a global average.
pub fn woogaroo_signed_decision_tree(
    fibre: &WoogarooSignedClaimFibre,
) -> Option<Vec<WoogarooSignedDecisionTree>> {
    if fibre.evidence.len() != 9 { return None; }
    Some([0usize,3,6].iter().map(|offset| {
        let mut parent = signed_tree_slot(fibre, *offset);
        parent.children = vec![
            signed_tree_slot(fibre, offset + 1),
            signed_tree_slot(fibre, offset + 2),
        ];
        parent
    }).collect())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum WoogarooSignedSearchFocus {
    AcquireOriginals,
    TestApplicantDefeaters,
    TestRespondentCounterDefeaters,
    ReconcileContradiction,
}

/// Candidate search role for the SAME signed alleged-defect proposition.
/// "review_assumed" is a deliberately synthetic test flag. It must never
/// be accepted as a real-source receipt; all output authority flags remain false.
pub fn woogaroo_signed_search_focus(
    defect: &WoogarooSignedEvidence,
    review_assumed: bool,
) -> WoogarooSignedSearchFocus {
    match defect.status() {
        WoogarooEvidenceStatus::Both => WoogarooSignedSearchFocus::ReconcileContradiction,
        WoogarooEvidenceStatus::Neither => WoogarooSignedSearchFocus::AcquireOriginals,
        WoogarooEvidenceStatus::PositiveOnly if review_assumed =>
            WoogarooSignedSearchFocus::TestApplicantDefeaters,
        WoogarooEvidenceStatus::NegativeOnly if review_assumed =>
            WoogarooSignedSearchFocus::TestRespondentCounterDefeaters,
        _ => WoogarooSignedSearchFocus::AcquireOriginals,
    }
}

/// Combine MULTIPLE same-ground/same-scope predicate branches, instead
/// of letting a solitary defect sign determine a case-level synthesis.
/// This is explicitly counterfactual: these checks never pay original
/// evidence, standing, saved-law interpretation, or an independent face.
pub fn woogaroo_signed_nested_focus(
    fibre: &WoogarooSignedClaimFibre,
    synthetic_review_assumed: bool,
) -> Option<WoogarooSignedSearchFocus> {
    if fibre.evidence.len() != WOOGAROO_SIGNED_PREDICATES.len() {
        return None;
    }
    let e = &fibre.evidence;
    if e.iter().any(|x| x.status() == WoogarooEvidenceStatus::Both) {
        return Some(WoogarooSignedSearchFocus::ReconcileContradiction);
    }
    let pos = |i: usize| e[i].status() == WoogarooEvidenceStatus::PositiveOnly;
    let neg = |i: usize| e[i].status() == WoogarooEvidenceStatus::NegativeOnly;
    let common = synthetic_review_assumed && pos(0) && pos(6) && pos(7);
    let result = if !common {
        WoogarooSignedSearchFocus::AcquireOriginals
    } else if pos(1) && pos(2) && !pos(3) {
        WoogarooSignedSearchFocus::TestApplicantDefeaters
    } else if neg(1) && pos(3) {
        WoogarooSignedSearchFocus::TestRespondentCounterDefeaters
    } else {
        WoogarooSignedSearchFocus::AcquireOriginals
    };
    Some(result)
}

/// A finite 3-adic branch prefix, not the whole inverse limit: adding a
/// finer sign preserves the earlier low-to-high digits. A coarse 0 still
/// requires the four-valued fine-fibre residual to distinguish both/neither.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSignedPadicPrefix {
    pub low_to_high_digits: Vec<i8>,
    pub three_adic_prefix_only: bool,
    pub full_inverse_limit_established: bool,
}

pub fn woogaroo_signed_padic_prefix(
    selected_branch: &[&WoogarooSignedEvidence],
) -> WoogarooSignedPadicPrefix {
    WoogarooSignedPadicPrefix {
        low_to_high_digits: selected_branch.iter()
            .map(|e| e.coarse_signed_trit()).collect(),
        three_adic_prefix_only: true,
        full_inverse_limit_established: false,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooSignedSynthesisOutput {
    pub ground_key: &'static str,
    pub signed_fibre: WoogarooSignedClaimFibre,
    pub signed_369: WoogarooSigned369Chart,
    pub nested_predicate_tree: Vec<WoogarooSignedDecisionTree>,
    pub one_branch_padic_prefix: WoogarooSignedPadicPrefix,
    pub focus: WoogarooSignedSearchFocus,
    pub base_acquisition_packet: &'static str,
    pub four_unpaid_obligation_ids: Vec<&'static str>,
    pub triadic_face_underdetermined: bool,
    pub hypothetical_only: bool,
    pub actual_source_reviewed: bool,
    pub actual_legal_merits_decided: bool,
}

/// Reuses the canonical seven-ground + four-obligation + potential snowball
/// packet, and fails closed on noncanonical ground or malformed fibre.
pub fn woogaroo_synthesise_signed_fibre(
    fibre: WoogarooSignedClaimFibre,
    synthetic_review_assumed: bool,
) -> Option<WoogarooSignedSynthesisOutput> {
    let ground = WOOGAROO_GROUNDS.iter().find(|g| g.key == fibre.ground_key)?;
    if !fibre.candidate_only || fibre.source_reviewed_and_paid
        || fibre.proposition_scope != format!("epbc:2019/8575:{}",ground.key)
        || fibre.evidence.len() != WOOGAROO_SIGNED_PREDICATES.len()
    { return None; }
    let unpaid = WoogarooHypotheticalWorld {
        legally_applicable: false, defect_established: false,
        materiality_established: false, government_compliance_established: false,
        evidence_reviewed: false, ecological_harm_assumed: false,
    };
    let base = woogaroo_synthesise_dialectic_trialectic(ground.key, unpaid)?;
    let signed_369 = fibre.signed_chart()?;
    let nested_predicate_tree = woogaroo_signed_decision_tree(&fibre)?;
    let focus = woogaroo_signed_nested_focus(&fibre, synthetic_review_assumed)?;
    let one_branch_padic_prefix = woogaroo_signed_padic_prefix(&[
        &fibre.evidence[0], &fibre.evidence[1], &fibre.evidence[2],
    ]);
    Some(WoogarooSignedSynthesisOutput {
        ground_key: ground.key,
        signed_fibre: fibre,
        signed_369,
        nested_predicate_tree,
        one_branch_padic_prefix,
        focus,
        base_acquisition_packet: base.original_acquisition_packet_key,
        four_unpaid_obligation_ids: base.open_ground_obligation_ids,
        triadic_face_underdetermined: true,
        hypothetical_only: true,
        actual_source_reviewed: false,
        actual_legal_merits_decided: false,
    })
}

pub fn woogaroo_unpaid_signed_fibre_synthesis() -> Vec<WoogarooSignedSynthesisOutput> {
    WOOGAROO_GROUNDS.iter().filter_map(|g| {
        woogaroo_synthesise_signed_fibre(
            WoogarooSignedClaimFibre::unpaid(g.key), false)
    }).collect()
}


#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct WoogarooConvergedSynthesisPacket {
    pub canonical_ground: &'static str,
    pub candidate_dialectic: WoogarooSynthesisPacket,
    pub signed_predicate_hyperfabric: WoogarooSignedSynthesisOutput,
    pub matched_source_obligations: bool,
    pub p_adic_branch_is_only_a_prefix: bool,
    pub may_force_irreducible_face: bool,
    pub real_legal_conclusion: bool,
}

/// One joined canonical seven-ground readout. No claim-level Boolean is
/// silently reinterpreted as −P; signed fine fibre is independent, and
/// synthesis only joins via exact ground, packet and four obligation IDs.
/// A false or missing witness fails CLOSED by omitting that packet.
pub fn woogaroo_unpaid_converged_synthesis() -> Vec<WoogarooConvergedSynthesisPacket> {
    let unsigned_world = WoogarooHypotheticalWorld {
        legally_applicable:false, defect_established:false, materiality_established:false,
        government_compliance_established:false, evidence_reviewed:false,
        ecological_harm_assumed:false,
    };
    WOOGAROO_GROUNDS.iter().filter_map(|g| {
        let candidate_dialectic =
            woogaroo_synthesise_dialectic_trialectic(g.key, unsigned_world)?;
        let signed_predicate_hyperfabric =
            woogaroo_synthesise_signed_fibre(
                WoogarooSignedClaimFibre::unpaid(g.key), false)?;
        if candidate_dialectic.ground_key != signed_predicate_hyperfabric.ground_key
            || candidate_dialectic.original_acquisition_packet_key !=
                signed_predicate_hyperfabric.base_acquisition_packet
            || candidate_dialectic.open_ground_obligation_ids !=
                signed_predicate_hyperfabric.four_unpaid_obligation_ids
            || !candidate_dialectic.candidate_only
            || !signed_predicate_hyperfabric.hypothetical_only
            || !signed_predicate_hyperfabric.triadic_face_underdetermined
        { return None; }
        Some(WoogarooConvergedSynthesisPacket {
            canonical_ground: g.key,
            candidate_dialectic,
            signed_predicate_hyperfabric,
            matched_source_obligations:true,
            p_adic_branch_is_only_a_prefix:true,
            may_force_irreducible_face:false,
            real_legal_conclusion:false,
        })
    }).collect()
}
