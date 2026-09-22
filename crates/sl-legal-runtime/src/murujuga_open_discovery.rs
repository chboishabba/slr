//! S21.4 Murujuga open-discovery control.
//!
//! The control is intentionally weaker than a merits model.  It proves that a
//! live Federal Court source-follow run begins from the actual Murujuga/EPBC
//! materials without pre-seeding Mabo or a native-title doctrinal dependency.
//! If such an authority later appears in reviewed source-follow, it may be
//! proposed then; absence of a seed is not a claim that the authority can
//! never become relevant.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use crate::{legal_case_battery, CaseBatteryKind};

pub const MURUJUGA_CONSUMER: &str = "consumer:murujuga-rock-art";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MurujugaDiscoveryControlReceipt {
    pub consumer_ref: String,
    pub proceeding_refs: BTreeSet<String>,
    pub source_refs: BTreeSet<String>,
    pub statutory_seed_refs: BTreeSet<String>,
    pub citation_seed_refs: BTreeSet<String>,
    pub mabo_dependency_preseeded: bool,
    pub source_follow_may_discover_new_authority: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_claim_truth: bool,
}

pub fn murujuga_open_discovery_control() -> Result<MurujugaDiscoveryControlReceipt, String> {
    let specimen = legal_case_battery()
        .into_iter()
        .find(|item| item.kind == CaseBatteryKind::Murujuga)
        .ok_or_else(|| "Murujuga battery specimen missing".to_string())?;
    specimen.validate()?;

    Ok(MurujugaDiscoveryControlReceipt {
        consumer_ref: specimen.consumer_ref,
        proceeding_refs: BTreeSet::from([
            "fedcourt:VID1356/2025".into(),
            "fedcourt:VID1357/2025".into(),
            "fedcourt:VID1400/2025".into(),
        ]),
        source_refs: BTreeSet::from([
            "fedcourt:murujuga-online-file:2026-07-30".into(),
            "fedcourt:VID1357/2025:fara-originating-application".into(),
            "fedcourt:VID1357/2025:fara-outline-submissions:2026-07-07".into(),
            "fedcourt:VID1356/2025:acf-outline-submissions:2026-07-07".into(),
        ]),
        // Current public-file materials include EPBC construction questions;
        // these are navigation seeds, never semantic payment by themselves.
        statutory_seed_refs: BTreeSet::from([
            "legislation:cth:epbc-act:1999:s134".into(),
            "legislation:cth:epbc-act:1999:s136".into(),
            "legislation:cth:epbc-act:1999:s137A".into(),
        ]),
        citation_seed_refs: specimen.citation_seed_refs.into_iter().collect(),
        mabo_dependency_preseeded: false,
        source_follow_may_discover_new_authority: true,
        candidate_only: true,
        creates_semantic_authority: false,
        creates_claim_truth: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn murujuga_starts_open_without_mabo_or_native_title_dependency() {
        let receipt = murujuga_open_discovery_control().unwrap();
        assert_eq!(receipt.consumer_ref, MURUJUGA_CONSUMER);
        assert!(receipt.citation_seed_refs.is_empty());
        assert!(!receipt.mabo_dependency_preseeded);
        assert!(receipt.source_follow_may_discover_new_authority);
        assert!(receipt
            .statutory_seed_refs
            .contains("legislation:cth:epbc-act:1999:s136"));
        assert!(!receipt.creates_claim_truth);
    }
}
