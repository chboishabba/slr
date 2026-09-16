use crate::{ContextBundle, ContextLink};

pub const MABO_WIKIDATA_QID: &str = "Q1501525";
pub const MABO_WIKIPEDIA_REF: &str = "wiki:en:Mabo_v_Queensland_(No_2)";
pub const MABO_PRIMARY_SOURCE_REF: &str = "source:mabo:1992:hca:23";
pub const MABO_RADICAL_TITLE_PROPOSITION: &str =
    "mabo:proposition:radical-title-native-title";

/// Canonical progressive context navigation for the flagship Mabo proposition.
/// The constructors themselves encode the authority firewall: Wikipedia and
/// Wikidata remain background/identity context and cannot become source payment.
#[must_use]
pub fn mabo_context_bundle() -> ContextBundle {
    ContextBundle::new(
        MABO_RADICAL_TITLE_PROPOSITION,
        vec![
            ContextLink::exact_source(MABO_PRIMARY_SOURCE_REF, "Exact High Court source"),
            ContextLink::wikidata(MABO_WIKIDATA_QID, "Mabo identity"),
            ContextLink::wikipedia(MABO_WIKIPEDIA_REF, "Background context"),
            ContextLink::historical("context:mabo:history", "Historical context"),
            ContextLink::semantic_focus(
                MABO_RADICAL_TITLE_PROPOSITION,
                "Current semantic/proof focus",
            ),
        ],
    )
}
