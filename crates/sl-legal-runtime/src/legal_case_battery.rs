//! S21 legal case-battery specifications over S19/S20.
//!
//! These specimens declare starting consumers, source seeds and *search
//! discipline*.  They deliberately do not preload the joins the experiment is
//! meant to discover.  A source citation/treatment may propose a shared-world
//! join; only reviewed dependency evidence can establish it.

use serde::{Deserialize, Serialize};

use crate::{AdversarialSearchRole, JoinProposalBasis};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseBatteryKind {
    YindjibarndiYunupingu,
    MunkaraTipakalippa,
    Pabai,
    Murujuga,
    Colonisation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CaseBatteryJoinDiscipline {
    SubstantiveAuthorityReuseExpectedIfSourceProves,
    PartialOverlapWithoutDoctrinalCollapse,
    StructuralAnalogyOnly,
    OpenDiscoveryNoExpectedJoin,
    BroadSynthesisMultipleSourceFamilies,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegalCaseBatterySpecimen {
    pub kind: CaseBatteryKind,
    pub consumer_ref: String,
    pub question_ref: String,
    pub seed_source_refs: Vec<String>,
    pub citation_seed_refs: Vec<String>,
    pub join_discipline: CaseBatteryJoinDiscipline,
    pub allowed_join_proposal_bases: Vec<JoinProposalBasis>,
    pub required_adversarial_roles: Vec<AdversarialSearchRole>,
    pub manually_seed_mabo_dependency: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

impl LegalCaseBatterySpecimen {
    pub fn validate(&self) -> Result<(), String> {
        if self.consumer_ref.trim().is_empty()
            || self.question_ref.trim().is_empty()
            || self.seed_source_refs.is_empty()
        {
            return Err("case battery specimen requires consumer/question/source seed".into());
        }
        if self.manually_seed_mabo_dependency {
            return Err(
                "case battery may not preload Mabo dependency; source/review must discover it"
                    .into(),
            );
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_claim_truth
        {
            return Err("case battery crossed non-promotion boundary".into());
        }
        if !self
            .required_adversarial_roles
            .contains(&AdversarialSearchRole::Defeater)
        {
            return Err("case battery must include adversarial defeater search".into());
        }
        Ok(())
    }
}

pub fn legal_case_battery() -> Vec<LegalCaseBatterySpecimen> {
    vec![
        LegalCaseBatterySpecimen {
            kind: CaseBatteryKind::YindjibarndiYunupingu,
            consumer_ref: "consumer:yindjibarndi-compensation".into(),
            question_ref:
                "question:yindjibarndi:extinguishment-acquisition-compensation-current-appeal"
                    .into(),
            seed_source_refs: vec![
                "fedcourt:WAD37/2022:yindjibarndi-online-file".into(),
                "fedcourt:WAD299/2026:yindjibarndi-appeal".into(),
                "fedcourt:WAD302/2026:state-appeal".into(),
            ],
            // This is a real source-evidenced seed: the Yindjibarndi file
            // contains supplementary submissions specifically concerning
            // Commonwealth v Yunupingu [2025] HCA 6.
            citation_seed_refs: vec!["case:au:hca:2025:6".into()],
            join_discipline:
                CaseBatteryJoinDiscipline::SubstantiveAuthorityReuseExpectedIfSourceProves,
            allowed_join_proposal_bases: vec![
                JoinProposalBasis::Citation,
                JoinProposalBasis::Treatment,
                JoinProposalBasis::ExplicitDependency,
            ],
            required_adversarial_roles: vec![
                AdversarialSearchRole::Support,
                AdversarialSearchRole::Defeater,
                AdversarialSearchRole::CounterDefeater,
                AdversarialSearchRole::AuthorityTreatment,
                AdversarialSearchRole::WrongTypeDiscriminator,
            ],
            manually_seed_mabo_dependency: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        LegalCaseBatterySpecimen {
            kind: CaseBatteryKind::MunkaraTipakalippa,
            consumer_ref: "consumer:munkara-tipakalippa".into(),
            question_ref:
                "question:tiwi:sea-country-cultural-heritage-offshore-regulatory-elements"
                    .into(),
            seed_source_refs: vec![
                "case:au:fca:2024:9".into(),
                "case:au:fcafc:2022:193".into(),
            ],
            citation_seed_refs: vec![],
            join_discipline: CaseBatteryJoinDiscipline::PartialOverlapWithoutDoctrinalCollapse,
            allowed_join_proposal_bases: vec![
                JoinProposalBasis::Citation,
                JoinProposalBasis::Treatment,
                JoinProposalBasis::SameSource,
                JoinProposalBasis::ExplicitDependency,
            ],
            required_adversarial_roles: vec![
                AdversarialSearchRole::Support,
                AdversarialSearchRole::Defeater,
                AdversarialSearchRole::CounterDefeater,
                AdversarialSearchRole::WrongTypeDiscriminator,
                AdversarialSearchRole::AuthorityTreatment,
            ],
            manually_seed_mabo_dependency: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        LegalCaseBatterySpecimen {
            kind: CaseBatteryKind::Pabai,
            consumer_ref: "consumer:pabai-climate-duty".into(),
            question_ref: "question:pabai:duty-route-and-current-blockers".into(),
            seed_source_refs: vec![
                "case:au:fca:2025:796".into(),
                "fedcourt:VID1479/2025:pabai-appeal".into(),
            ],
            citation_seed_refs: vec![],
            join_discipline: CaseBatteryJoinDiscipline::StructuralAnalogyOnly,
            allowed_join_proposal_bases: vec![
                JoinProposalBasis::ConceptAdjacency,
                JoinProposalBasis::ExplicitDependency,
            ],
            required_adversarial_roles: vec![
                AdversarialSearchRole::Support,
                AdversarialSearchRole::Defeater,
                AdversarialSearchRole::CounterDefeater,
                AdversarialSearchRole::Comparator,
                AdversarialSearchRole::WrongTypeDiscriminator,
            ],
            manually_seed_mabo_dependency: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        LegalCaseBatterySpecimen {
            kind: CaseBatteryKind::Murujuga,
            consumer_ref: "consumer:murujuga-rock-art".into(),
            question_ref:
                "question:murujuga:epbc-cultural-heritage-country-current-judicial-review".into(),
            seed_source_refs: vec![
                "fedcourt:VID1357/2025:friends-of-australian-rock-art".into(),
                "fedcourt:VID1356/2025:australian-conservation-foundation".into(),
            ],
            citation_seed_refs: vec![],
            join_discipline: CaseBatteryJoinDiscipline::OpenDiscoveryNoExpectedJoin,
            allowed_join_proposal_bases: vec![
                JoinProposalBasis::Citation,
                JoinProposalBasis::Treatment,
                JoinProposalBasis::ExplicitDependency,
            ],
            required_adversarial_roles: vec![
                AdversarialSearchRole::Support,
                AdversarialSearchRole::Defeater,
                AdversarialSearchRole::CounterDefeater,
                AdversarialSearchRole::AuthorityTreatment,
                AdversarialSearchRole::WrongTypeDiscriminator,
            ],
            manually_seed_mabo_dependency: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
        LegalCaseBatterySpecimen {
            kind: CaseBatteryKind::Colonisation,
            consumer_ref: "consumer:colonisation-australia".into(),
            question_ref:
                "question:colonisation:indigenous-country-sovereignty-radical-title-native-title-compensation"
                    .into(),
            seed_source_refs: vec![
                "source-family:historical-primary".into(),
                "source-family:colonial-instruments".into(),
                "source-family:australian-case-law".into(),
                "source-family:australian-legislation".into(),
            ],
            citation_seed_refs: vec![],
            join_discipline:
                CaseBatteryJoinDiscipline::BroadSynthesisMultipleSourceFamilies,
            allowed_join_proposal_bases: vec![
                JoinProposalBasis::Citation,
                JoinProposalBasis::Treatment,
                JoinProposalBasis::SameSource,
                JoinProposalBasis::ExplicitDependency,
            ],
            required_adversarial_roles: vec![
                AdversarialSearchRole::Support,
                AdversarialSearchRole::Defeater,
                AdversarialSearchRole::CounterDefeater,
                AdversarialSearchRole::Comparator,
                AdversarialSearchRole::AuthorityTreatment,
                AdversarialSearchRole::WrongTypeDiscriminator,
            ],
            manually_seed_mabo_dependency: false,
            candidate_only: true,
            creates_semantic_authority: false,
            creates_claim_truth: false,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        munkara_wrong_type_gap, murujuga_open_discovery_control,
        run_pabai_golden_regression, CandidateRouteStatus, TypedRerunGapKind,
    };

    #[test]
    fn every_case_battery_specimen_is_source_driven_and_adversarial() {
        let battery = legal_case_battery();
        assert_eq!(battery.len(), 5);
        for specimen in battery {
            specimen.validate().unwrap();
            assert!(!specimen.manually_seed_mabo_dependency);
            assert!(specimen
                .required_adversarial_roles
                .contains(&AdversarialSearchRole::Defeater));
        }
    }

    #[test]
    fn only_yindjibarndi_starts_with_source_evidenced_yunupingu_citation_seed() {
        let battery = legal_case_battery();
        for specimen in battery {
            if specimen.kind == CaseBatteryKind::YindjibarndiYunupingu {
                assert_eq!(
                    specimen.citation_seed_refs,
                    vec!["case:au:hca:2025:6"]
                );
            } else {
                assert!(specimen.citation_seed_refs.is_empty());
            }
        }
    }

    #[test]
    fn pabai_is_structural_transfer_not_mabo_substantive_payment() {
        let pabai = legal_case_battery()
            .into_iter()
            .find(|specimen| specimen.kind == CaseBatteryKind::Pabai)
            .unwrap();
        assert_eq!(
            pabai.join_discipline,
            CaseBatteryJoinDiscipline::StructuralAnalogyOnly
        );
        assert!(!pabai
            .allowed_join_proposal_bases
            .contains(&JoinProposalBasis::Citation));
    }

    #[test]
    fn munkara_emits_machine_visible_wrong_type_for_related_but_nonpaying_context() {
        let gap = munkara_wrong_type_gap();
        assert_eq!(gap.kind, TypedRerunGapKind::WrongType);
        assert!(gap.coordinate_ref.contains("sea-country"));
        assert!(!gap.creates_claim_truth);
    }

    #[test]
    fn pabai_is_the_golden_reopen_then_attack_again_regression() {
        let receipt = run_pabai_golden_regression().unwrap();
        assert_eq!(
            receipt.route_status,
            CandidateRouteStatus::ReachableCandidate
        );
        assert!(receipt.active_defeater_refs.is_empty());
        assert_eq!(receipt.counter_defeater_refs.len(), 1);
        assert!(receipt
            .next_demands
            .iter()
            .any(|demand| demand.role == AdversarialSearchRole::Defeater));
    }

    #[test]
    fn murujuga_open_discovery_control_has_no_mabo_preseed() {
        let receipt = murujuga_open_discovery_control().unwrap();
        assert!(receipt.citation_seed_refs.is_empty());
        assert!(!receipt.mabo_dependency_preseeded);
        assert!(receipt.source_follow_may_discover_new_authority);
    }

    #[test]
    fn murujuga_has_no_expected_mabo_join() {
        let murujuga = legal_case_battery()
            .into_iter()
            .find(|specimen| specimen.kind == CaseBatteryKind::Murujuga)
            .unwrap();
        assert_eq!(
            murujuga.join_discipline,
            CaseBatteryJoinDiscipline::OpenDiscoveryNoExpectedJoin
        );
        assert!(murujuga.citation_seed_refs.is_empty());
    }
}
