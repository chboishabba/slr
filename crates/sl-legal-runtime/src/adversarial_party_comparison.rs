//! M11 / S26.6 adversarial-party comparison.
//!
//! This is an explanatory comparison over reviewed argument coordinates. It
//! does not rank parties, predict an outcome, or promote submissions to
//! holdings. Shared authority identity and divergent treatment are kept
//! separate.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

use crate::{
    yindjibarndi_primary_source_packet, EmpiricalArgumentRole, EmpiricalParty,
    SourceGroundedArgumentCoordinate,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyArgumentFibre {
    pub party: EmpiricalParty,
    pub coordinate_refs: BTreeSet<String>,
    pub proposition_refs: BTreeSet<String>,
    pub source_refs: BTreeSet<String>,
    pub authority_refs: BTreeSet<String>,
    pub support_refs: BTreeSet<String>,
    pub defeater_refs: BTreeSet<String>,
    pub counter_defeater_refs: BTreeSet<String>,
    pub authority_scope_refs: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PartyPairComparison {
    pub left_party: EmpiricalParty,
    pub right_party: EmpiricalParty,
    pub shared_coordinate_refs: BTreeSet<String>,
    pub left_only_coordinate_refs: BTreeSet<String>,
    pub right_only_coordinate_refs: BTreeSet<String>,
    pub shared_authority_refs: BTreeSet<String>,
    pub left_only_proposition_refs: BTreeSet<String>,
    pub right_only_proposition_refs: BTreeSet<String>,
    pub coordinates_with_different_treatment: BTreeSet<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdversarialPartyComparativeReceipt {
    pub fibres: BTreeMap<String, PartyArgumentFibre>,
    pub applicant_vs_state: PartyPairComparison,
    pub applicant_vs_fmg: PartyPairComparison,
    pub state_vs_fmg: PartyPairComparison,
    pub submissions_are_not_holdings: bool,
    pub predicts_outcome: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

fn party_key(party: EmpiricalParty) -> &'static str {
    match party {
        EmpiricalParty::Applicant => "applicant",
        EmpiricalParty::StateOfWesternAustralia => "state-of-western-australia",
        EmpiricalParty::FmgRespondents => "fmg-respondents",
    }
}

fn fibre(
    party: EmpiricalParty,
    rows: &[SourceGroundedArgumentCoordinate],
) -> PartyArgumentFibre {
    let selected = rows.iter().filter(|row| row.party == party).collect::<Vec<_>>();
    let propositions_for_role = |role: EmpiricalArgumentRole| {
        selected
            .iter()
            .filter(|row| row.role == role)
            .map(|row| row.proposition_ref.clone())
            .collect::<BTreeSet<_>>()
    };
    PartyArgumentFibre {
        party,
        coordinate_refs: selected.iter().map(|row| row.coordinate_ref.clone()).collect(),
        proposition_refs: selected.iter().map(|row| row.proposition_ref.clone()).collect(),
        source_refs: selected.iter().map(|row| row.source_ref.clone()).collect(),
        authority_refs: selected
            .iter()
            .map(|row| row.cited_authority_ref.clone())
            .collect(),
        support_refs: propositions_for_role(EmpiricalArgumentRole::Support),
        defeater_refs: propositions_for_role(EmpiricalArgumentRole::Defeater),
        counter_defeater_refs: propositions_for_role(EmpiricalArgumentRole::CounterDefeater),
        authority_scope_refs: propositions_for_role(EmpiricalArgumentRole::AuthorityScope),
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

fn role_key(role: EmpiricalArgumentRole) -> &'static str {
    match role {
        EmpiricalArgumentRole::Support => "support",
        EmpiricalArgumentRole::Defeater => "defeater",
        EmpiricalArgumentRole::CounterDefeater => "counter-defeater",
        EmpiricalArgumentRole::AuthorityScope => "authority-scope",
    }
}

fn treatment_by_coordinate(
    party: EmpiricalParty,
    rows: &[SourceGroundedArgumentCoordinate],
) -> BTreeMap<String, BTreeSet<String>> {
    let mut map = BTreeMap::<String, BTreeSet<String>>::new();
    for row in rows.iter().filter(|row| row.party == party) {
        map.entry(row.coordinate_ref.clone())
            .or_default()
            .insert(role_key(row.role).to_owned());
    }
    map
}

fn compare(
    left: &PartyArgumentFibre,
    right: &PartyArgumentFibre,
    rows: &[SourceGroundedArgumentCoordinate],
) -> PartyPairComparison {
    let left_treatments = treatment_by_coordinate(left.party, rows);
    let right_treatments = treatment_by_coordinate(right.party, rows);
    let shared_coordinate_refs = left
        .coordinate_refs
        .intersection(&right.coordinate_refs)
        .cloned()
        .collect::<BTreeSet<_>>();
    let coordinates_with_different_treatment = shared_coordinate_refs
        .iter()
        .filter(|coordinate| left_treatments.get(*coordinate) != right_treatments.get(*coordinate))
        .cloned()
        .collect();

    PartyPairComparison {
        left_party: left.party,
        right_party: right.party,
        shared_coordinate_refs,
        left_only_coordinate_refs: left
            .coordinate_refs
            .difference(&right.coordinate_refs)
            .cloned()
            .collect(),
        right_only_coordinate_refs: right
            .coordinate_refs
            .difference(&left.coordinate_refs)
            .cloned()
            .collect(),
        shared_authority_refs: left
            .authority_refs
            .intersection(&right.authority_refs)
            .cloned()
            .collect(),
        left_only_proposition_refs: left
            .proposition_refs
            .difference(&right.proposition_refs)
            .cloned()
            .collect(),
        right_only_proposition_refs: right
            .proposition_refs
            .difference(&left.proposition_refs)
            .cloned()
            .collect(),
        coordinates_with_different_treatment,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    }
}

pub fn run_yindjibarndi_party_comparison(
) -> Result<AdversarialPartyComparativeReceipt, String> {
    let rows = yindjibarndi_primary_source_packet();
    for row in &rows {
        row.validate()?;
    }

    let applicant = fibre(EmpiricalParty::Applicant, &rows);
    let state = fibre(EmpiricalParty::StateOfWesternAustralia, &rows);
    let fmg = fibre(EmpiricalParty::FmgRespondents, &rows);

    let fibres = BTreeMap::from([
        (party_key(applicant.party).to_owned(), applicant.clone()),
        (party_key(state.party).to_owned(), state.clone()),
        (party_key(fmg.party).to_owned(), fmg.clone()),
    ]);

    Ok(AdversarialPartyComparativeReceipt {
        fibres,
        applicant_vs_state: compare(&applicant, &state, &rows),
        applicant_vs_fmg: compare(&applicant, &fmg, &rows),
        state_vs_fmg: compare(&state, &fmg, &rows),
        submissions_are_not_holdings: true,
        predicts_outcome: false,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        MABO_ACQUISITION_DISTINCTION_COORDINATE,
        YUNUPINGU_ACQUISITION_COORDINATE,
    };

    #[test]
    fn shared_authority_identity_does_not_collapse_opposed_treatment() {
        let receipt = run_yindjibarndi_party_comparison().unwrap();

        assert!(receipt
            .applicant_vs_state
            .shared_coordinate_refs
            .contains(YUNUPINGU_ACQUISITION_COORDINATE));
        assert!(receipt
            .applicant_vs_state
            .coordinates_with_different_treatment
            .contains(YUNUPINGU_ACQUISITION_COORDINATE));

        assert!(receipt
            .applicant_vs_fmg
            .shared_coordinate_refs
            .contains(MABO_ACQUISITION_DISTINCTION_COORDINATE));
        assert!(receipt
            .applicant_vs_fmg
            .coordinates_with_different_treatment
            .contains(MABO_ACQUISITION_DISTINCTION_COORDINATE));

        assert!(receipt.submissions_are_not_holdings);
        assert!(!receipt.predicts_outcome);
        assert!(!receipt.creates_claim_truth);
    }

    #[test]
    fn party_comparison_preserves_one_sided_propositions_instead_of_ranking() {
        let receipt = run_yindjibarndi_party_comparison().unwrap();
        assert!(!receipt
            .applicant_vs_fmg
            .left_only_proposition_refs
            .is_empty());
        assert!(!receipt
            .applicant_vs_fmg
            .right_only_proposition_refs
            .is_empty());
    }
}
