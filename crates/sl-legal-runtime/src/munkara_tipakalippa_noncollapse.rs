//! S21.2 Munkara / Tipakalippa non-collapse fixture.
//!
//! This fixture tests the dual of Yindjibarndi's positive exact reuse:
//! related Sea Country material may be contextually reusable, but it cannot
//! pay a different statutory legal requirement merely because the matters are
//! factually/culturally adjacent.
//!
//! Source identities:
//! - Santos NA Barossa Pty Ltd v Tipakalippa [2022] FCAFC 193:
//!   consultation / "relevant person" under reg 11A(1)(d).
//! - Munkara v Santos NA Barossa Pty Ltd (No 3) [2024] FCA 9:
//!   revised environment plan / new significant environmental impact or risk
//!   under reg 17(6).
//!
//! This module does not promote either source into a generic Country doctrine
//! and does not treat Native Title / Sea Country adjacency as statutory payment.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{TypedRerunGap, TypedRerunGapKind};

pub const MUNKARA_CONSUMER: &str = "consumer:munkara-tipakalippa";
pub const SEA_COUNTRY_CONTEXT_COORDINATE: &str =
    "coordinate:context:tiwi-sea-country-traditional-connection";
pub const TIPAKALIPPA_RELEVANT_PERSON_COORDINATE: &str =
    "coordinate:legal:tipakalippa:reg11a1d-relevant-person-consultation";
pub const MUNKARA_REG17_6_COORDINATE: &str =
    "coordinate:legal:munkara:reg17-6-new-significant-environmental-impact-or-risk";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MunkaraSourceRole {
    Context,
    TipakalippaConsultationRule,
    MunkaraRevisionRequirement,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MunkaraSourceCoordinate {
    pub coordinate_ref: String,
    pub source_ref: String,
    pub proposition_ref: String,
    pub role: MunkaraSourceRole,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NonCollapseDisposition {
    ReuseContext,
    ReuseExactLegalCoordinate,
    WrongType,
    MissingExactPayment,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NonCollapseDecision {
    pub offered_coordinate_ref: String,
    pub required_coordinate_ref: String,
    pub disposition: NonCollapseDisposition,
    pub reason_ref: String,
    pub source_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

pub fn munkara_tipakalippa_source_packet() -> Vec<MunkaraSourceCoordinate> {
    vec![
        MunkaraSourceCoordinate {
            coordinate_ref: SEA_COUNTRY_CONTEXT_COORDINATE.into(),
            source_ref: "case:au:fcafc:2022:193".into(),
            proposition_ref:
                "proposition:tipakalippa:traditional-connection-to-sea-country-may-ground-relevant-interest"
                    .into(),
            role: MunkaraSourceRole::Context,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        MunkaraSourceCoordinate {
            coordinate_ref: TIPAKALIPPA_RELEVANT_PERSON_COORDINATE.into(),
            source_ref: "case:au:fcafc:2022:193".into(),
            proposition_ref:
                "proposition:tipakalippa:reg11a1d-requires-consultation-with-relevant-person"
                    .into(),
            role: MunkaraSourceRole::TipakalippaConsultationRule,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        MunkaraSourceCoordinate {
            coordinate_ref: MUNKARA_REG17_6_COORDINATE.into(),
            source_ref: "case:au:fca:2024:9".into(),
            proposition_ref:
                "proposition:munkara:reg17-6-revised-environment-plan-new-significant-impact-or-risk"
                    .into(),
            role: MunkaraSourceRole::MunkaraRevisionRequirement,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
    ]
}

pub fn decide_noncollapse(
    offered_coordinate_ref: &str,
    required_coordinate_ref: &str,
) -> NonCollapseDecision {
    let exact = offered_coordinate_ref == required_coordinate_ref;
    let disposition = if exact {
        NonCollapseDisposition::ReuseExactLegalCoordinate
    } else if offered_coordinate_ref == SEA_COUNTRY_CONTEXT_COORDINATE {
        NonCollapseDisposition::WrongType
    } else {
        NonCollapseDisposition::MissingExactPayment
    };
    let reason_ref = match disposition {
        NonCollapseDisposition::ReuseExactLegalCoordinate => "reason:exact-coordinate-match",
        NonCollapseDisposition::ReuseContext => "reason:context-only",
        NonCollapseDisposition::WrongType => {
            "reason:sea-country-context-cannot-pay-distinct-statutory-requirement"
        }
        NonCollapseDisposition::MissingExactPayment => "reason:exact-legal-payment-still-required",
    };

    NonCollapseDecision {
        offered_coordinate_ref: offered_coordinate_ref.into(),
        required_coordinate_ref: required_coordinate_ref.into(),
        disposition,
        reason_ref: reason_ref.into(),
        source_refs: BTreeSet::from([
            "case:au:fcafc:2022:193".into(),
            "case:au:fca:2024:9".into(),
        ]),
        candidate_only: true,
        creates_claim_truth: false,
    }
}

pub fn munkara_wrong_type_receipt() -> NonCollapseDecision {
    decide_noncollapse(
        SEA_COUNTRY_CONTEXT_COORDINATE,
        MUNKARA_REG17_6_COORDINATE,
    )
}

pub fn munkara_wrong_type_gap() -> TypedRerunGap {
    let decision = munkara_wrong_type_receipt();
    TypedRerunGap {
        gap_ref: "gap:munkara:sea-country-context-wrongtype-for-reg17-6".into(),
        kind: TypedRerunGapKind::WrongType,
        coordinate_ref: decision.offered_coordinate_ref,
        proposition_ref:
            "proposition:munkara:reg17-6-requires-exact-new-significant-impact-or-risk-payment"
                .into(),
        source_refs: decision.source_refs,
        candidate_only: true,
        creates_claim_truth: false,
    }
}

pub fn tipakalippa_rule_wrong_type_for_munkara_reg17_6() -> NonCollapseDecision {
    decide_noncollapse(
        TIPAKALIPPA_RELEVANT_PERSON_COORDINATE,
        MUNKARA_REG17_6_COORDINATE,
    )
}

pub fn exact_munkara_payment_is_reusable() -> NonCollapseDecision {
    decide_noncollapse(MUNKARA_REG17_6_COORDINATE, MUNKARA_REG17_6_COORDINATE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_packet_keeps_context_and_two_statutory_requirements_distinct() {
        let packet = munkara_tipakalippa_source_packet();
        assert_eq!(packet.len(), 3);
        assert_eq!(
            packet.iter().map(|item| item.coordinate_ref.as_str()).collect::<BTreeSet<_>>().len(),
            3
        );
        assert!(packet.iter().all(|item| item.candidate_only));
        assert!(packet.iter().all(|item| !item.creates_claim_truth));
    }

    #[test]
    fn related_sea_country_context_is_wrong_type_for_reg17_6_payment() {
        let receipt = munkara_wrong_type_receipt();
        assert_eq!(receipt.disposition, NonCollapseDisposition::WrongType);
        assert_eq!(receipt.required_coordinate_ref, MUNKARA_REG17_6_COORDINATE);
        assert!(!receipt.creates_claim_truth);
        let gap = munkara_wrong_type_gap();
        assert_eq!(gap.kind, TypedRerunGapKind::WrongType);
        assert_eq!(gap.coordinate_ref, SEA_COUNTRY_CONTEXT_COORDINATE);
        assert!(!gap.creates_claim_truth);
    }

    #[test]
    fn tipakalippa_consultation_rule_does_not_pay_munkara_revision_requirement() {
        let receipt = tipakalippa_rule_wrong_type_for_munkara_reg17_6();
        assert_eq!(
            receipt.disposition,
            NonCollapseDisposition::MissingExactPayment
        );
        assert_eq!(receipt.required_coordinate_ref, MUNKARA_REG17_6_COORDINATE);
    }

    #[test]
    fn only_exact_munkara_statutory_coordinate_pays_exact_requirement() {
        let receipt = exact_munkara_payment_is_reusable();
        assert_eq!(
            receipt.disposition,
            NonCollapseDisposition::ReuseExactLegalCoordinate
        );
    }
}
