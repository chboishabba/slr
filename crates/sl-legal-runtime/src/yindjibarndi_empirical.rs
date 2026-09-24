//! S21.A empirical Yindjibarndi/Yunupingu/Mabo source packet.
//!
//! Primary-source grounding:
//! - Federal Court WAD37/2022 applicant reply on Yunupingu (25 June 2025);
//! - State reply on Yunupingu (23 May 2025);
//! - FMG submissions on Yunupingu (16 May 2025);
//! - Applicant closing submissions in reply (3 February 2025), including the
//!   parties' opposed use of Mabo v Queensland (No 2).
//!
//! This module records reviewed *argument coordinates*.  It does not decide
//! the compensation dispute and it deliberately rejects a generic
//! "native-title => Mabo" join.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{
    review_shared_join, ConsumerDependencySlice, JoinProposalBasis,
    ReviewedSharedJoinWitness, SharedJoinProposal,
};

pub const YINDJIBARNDI_CONSUMER: &str = "consumer:yindjibarndi-compensation";
pub const YUNUPINGU_ACQUISITION_COORDINATE: &str =
    "coordinate:authority:yunupingu-2025-hca6:s51xxxi-native-title-acquisition";
pub const MABO_ACQUISITION_DISTINCTION_COORDINATE: &str =
    "coordinate:authority:mabo-no2:brennan-60-acquisition-distinction";
pub const GENERIC_MABO_NATIVE_TITLE_COORDINATE: &str =
    "coordinate:authority:mabo-no2:generic-native-title";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmpiricalParty {
    Applicant,
    StateOfWesternAustralia,
    FmgRespondents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EmpiricalArgumentRole {
    Support,
    Defeater,
    CounterDefeater,
    AuthorityScope,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceGroundedArgumentCoordinate {
    pub coordinate_ref: String,
    pub source_ref: String,
    pub party: EmpiricalParty,
    pub role: EmpiricalArgumentRole,
    pub proposition_ref: String,
    pub cited_authority_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl SourceGroundedArgumentCoordinate {
    pub fn validate(&self) -> Result<(), String> {
        if self.coordinate_ref.trim().is_empty()
            || self.source_ref.trim().is_empty()
            || self.proposition_ref.trim().is_empty()
            || self.cited_authority_ref.trim().is_empty()
        {
            return Err("empirical argument coordinate must be fully source-grounded".into());
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("empirical packet crossed the non-promotion boundary".into());
        }
        Ok(())
    }
}

pub fn yindjibarndi_primary_source_packet() -> Vec<SourceGroundedArgumentCoordinate> {
    vec![
        SourceGroundedArgumentCoordinate {
            coordinate_ref: YUNUPINGU_ACQUISITION_COORDINATE.into(),
            source_ref: "fedcourt:WAD37/2022:applicant-yunupingu-reply:2025-06-25".into(),
            party: EmpiricalParty::Applicant,
            role: EmpiricalArgumentRole::Support,
            proposition_ref:
                "proposition:yindjibarndi:yunupingu-supports-acquisition-through-diminution-or-impairment"
                    .into(),
            cited_authority_ref: "case:au:hca:2025:6".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        SourceGroundedArgumentCoordinate {
            coordinate_ref: YUNUPINGU_ACQUISITION_COORDINATE.into(),
            source_ref: "fedcourt:WAD37/2022:state-yunupingu-reply:2025-05-23".into(),
            party: EmpiricalParty::StateOfWesternAustralia,
            role: EmpiricalArgumentRole::AuthorityScope,
            proposition_ref:
                "proposition:yindjibarndi:yunupingu-did-not-decide-nonextinguishment-act-acquisition"
                    .into(),
            cited_authority_ref: "case:au:hca:2025:6".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        SourceGroundedArgumentCoordinate {
            coordinate_ref: YUNUPINGU_ACQUISITION_COORDINATE.into(),
            source_ref: "fedcourt:WAD37/2022:fmg-yunupingu-submissions:2025-05-16".into(),
            party: EmpiricalParty::FmgRespondents,
            role: EmpiricalArgumentRole::Defeater,
            proposition_ref:
                "proposition:yindjibarndi:yunupingu-does-not-establish-every-mining-lease-acquisition"
                    .into(),
            cited_authority_ref: "case:au:hca:2025:6".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        SourceGroundedArgumentCoordinate {
            coordinate_ref: MABO_ACQUISITION_DISTINCTION_COORDINATE.into(),
            source_ref: "fedcourt:WAD37/2022:applicant-closing-reply:2025-02-03".into(),
            party: EmpiricalParty::FmgRespondents,
            role: EmpiricalArgumentRole::Defeater,
            proposition_ref:
                "proposition:yindjibarndi:fmg-invokes-mabo-brennan-60-against-acquisition"
                    .into(),
            cited_authority_ref: "case:au:hca:1992:23".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        SourceGroundedArgumentCoordinate {
            coordinate_ref: MABO_ACQUISITION_DISTINCTION_COORDINATE.into(),
            source_ref: "fedcourt:WAD37/2022:applicant-closing-reply:2025-02-03".into(),
            party: EmpiricalParty::Applicant,
            role: EmpiricalArgumentRole::CounterDefeater,
            proposition_ref:
                "proposition:yindjibarndi:applicant-distinguishes-voluntary-assignment-from-crown-compulsory-acquisition"
                    .into(),
            cited_authority_ref: "case:au:hca:1992:23".into(),
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
    ]
}

pub fn yindjibarndi_reviewed_dependency_slice() -> ConsumerDependencySlice {
    ConsumerDependencySlice {
        consumer_ref: YINDJIBARNDI_CONSUMER.into(),
        required_coordinate_refs: BTreeSet::from([
            YUNUPINGU_ACQUISITION_COORDINATE.into(),
            MABO_ACQUISITION_DISTINCTION_COORDINATE.into(),
        ]),
        optional_coordinate_refs: BTreeSet::new(),
        dependency_slice_ref: "slice:yindjibarndi:empirical-authority-treatment".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn proposal(
    coordinate_ref: &str,
    source_consumer_ref: &str,
    evidence_refs: BTreeSet<String>,
) -> SharedJoinProposal {
    SharedJoinProposal {
        proposal_ref: format!("proposal:yindjibarndi:{coordinate_ref}"),
        source_consumer_ref: source_consumer_ref.into(),
        target_consumer_ref: YINDJIBARNDI_CONSUMER.into(),
        coordinate_ref: coordinate_ref.into(),
        basis: JoinProposalBasis::Treatment,
        evidence_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn reviewed_yindjibarndi_authority_joins(
) -> Result<Vec<ReviewedSharedJoinWitness>, String> {
    let packet = yindjibarndi_primary_source_packet();
    for coordinate in &packet {
        coordinate.validate()?;
    }

    let slice = yindjibarndi_reviewed_dependency_slice();

    let yunupingu_evidence = packet
        .iter()
        .filter(|item| item.coordinate_ref == YUNUPINGU_ACQUISITION_COORDINATE)
        .map(|item| item.source_ref.clone())
        .collect::<BTreeSet<_>>();

    let mabo_evidence = packet
        .iter()
        .filter(|item| item.coordinate_ref == MABO_ACQUISITION_DISTINCTION_COORDINATE)
        .map(|item| item.source_ref.clone())
        .collect::<BTreeSet<_>>();

    Ok(vec![
        review_shared_join(
            &proposal(
                YUNUPINGU_ACQUISITION_COORDINATE,
                "consumer:shared-native-title-authorities",
                yunupingu_evidence,
            ),
            &slice,
            "review:yindjibarndi:yunupingu-treatment",
        )?,
        review_shared_join(
            &proposal(
                MABO_ACQUISITION_DISTINCTION_COORDINATE,
                "consumer:mabo",
                mabo_evidence,
            ),
            &slice,
            "review:yindjibarndi:mabo-specific-treatment",
        )?,
    ])
}

pub fn generic_mabo_join_is_rejected() -> bool {
    let slice = yindjibarndi_reviewed_dependency_slice();
    review_shared_join(
        &proposal(
            GENERIC_MABO_NATIVE_TITLE_COORDINATE,
            "consumer:mabo",
            BTreeSet::from([
                "candidate:ontology-native-title-adjacency".into(),
            ]),
        ),
        &slice,
        "review:yindjibarndi:generic-mabo",
    )
    .is_err()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empirical_packet_is_primary_source_grounded_and_adversarial() {
        let packet = yindjibarndi_primary_source_packet();
        assert_eq!(packet.len(), 5);
        for item in &packet {
            item.validate().unwrap();
            assert!(item.source_ref.starts_with("fedcourt:WAD37/2022:"));
        }
        assert!(packet.iter().any(|item| item.role == EmpiricalArgumentRole::Support));
        assert!(packet.iter().any(|item| item.role == EmpiricalArgumentRole::Defeater));
        assert!(packet
            .iter()
            .any(|item| item.role == EmpiricalArgumentRole::CounterDefeater));
    }

    #[test]
    fn yunupingu_and_specific_mabo_treatment_earn_reviewed_joins() {
        let joins = reviewed_yindjibarndi_authority_joins().unwrap();
        assert_eq!(joins.len(), 2);
        assert!(joins
            .iter()
            .any(|join| join.coordinate_ref == YUNUPINGU_ACQUISITION_COORDINATE));
        assert!(joins
            .iter()
            .any(|join| join.coordinate_ref == MABO_ACQUISITION_DISTINCTION_COORDINATE));
        assert!(joins.iter().all(|join| join.dependency_membership_reviewed));
        assert!(joins.iter().all(|join| !join.creates_claim_truth));
    }

    #[test]
    fn generic_native_title_adjacency_still_does_not_earn_mabo_payment() {
        assert!(generic_mabo_join_is_rejected());
        assert!(!yindjibarndi_reviewed_dependency_slice()
            .required_coordinate_refs
            .contains(GENERIC_MABO_NATIVE_TITLE_COORDINATE));
    }
}
