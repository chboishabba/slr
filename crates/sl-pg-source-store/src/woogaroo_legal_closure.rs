//! EPBC 2019/8575: case-scoped max-cut and 80 source-gated closure targets.
//! Thin consumer of existing SLR Woogaroo ground owners, not a new payment,
//! source-review, Court, or evidence-admission engine. All targets remain unpaid.
//! SensibLaw owns its historical original prototype. SLR owns production.
//! Original lawmakers, scientific authors, courts, developer and prior PR authors
//! retain separate authorship. Johl Brown directed synthesis; ChatGPT assisted.

use std::collections::BTreeSet;
use crate::WOOGAROO_GROUNDS;

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
