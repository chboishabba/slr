use sensiblaw_core::canonical_evidence::{
    MatterPersona as CanonicalMatterPersona,
    MatterRepresentationProjection as CanonicalMatterRepresentationProjection,
    SameMatterInformativenessReceipt as CanonicalSameMatterInformativenessReceipt,
};
use sensiblaw_core::matter_consumer_projection::{
    MatterPersona, MatterRepresentationProjection, SameMatterInformativenessReceipt,
};

fn accepts_root_persona(_: Option<MatterPersona>) {}
fn accepts_root_projection(_: Option<MatterRepresentationProjection>) {}
fn accepts_root_receipt(_: Option<SameMatterInformativenessReceipt>) {}

#[test]
fn module_and_canonical_paths_are_the_same_projection_types() {
    let canonical_persona: Option<CanonicalMatterPersona> = None;
    let canonical_projection: Option<CanonicalMatterRepresentationProjection> = None;
    let canonical_receipt: Option<CanonicalSameMatterInformativenessReceipt> = None;

    accepts_root_persona(canonical_persona);
    accepts_root_projection(canonical_projection);
    accepts_root_receipt(canonical_receipt);
}
