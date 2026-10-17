use sensiblaw_core::canonical_evidence::{
    MatterPersona as CanonicalMatterPersona,
    MatterRepresentationProjection as CanonicalMatterRepresentationProjection,
    SameMatterInformativenessReceipt as CanonicalSameMatterInformativenessReceipt,
};
use sensiblaw_core::matter_consumer_projection::{
    MatterPersona,
    MatterRepresentationProjection,
    SameMatterInformativenessReceipt,
};

#[test]
fn matter_consumer_projection_is_exposed_through_module_and_canonical_reexport() {
    let _: Option<MatterPersona> = None;
    let _: Option<MatterRepresentationProjection> = None;
    let _: Option<SameMatterInformativenessReceipt> = None;

    let _: Option<CanonicalMatterPersona> = None;
    let _: Option<CanonicalMatterRepresentationProjection> = None;
    let _: Option<CanonicalSameMatterInformativenessReceipt> = None;
}
