//! Export seven *candidate-only* Woogaroo dialectic/trialectic synthesis
//! packets as stable human-readable JSON from the native SLR case owner.
//! No source review, PostgreSQL mutation, filing, judgment or legal result.
//! Run: cargo run -p sensiblaw-pg-source-store --example woogaroo_trialectic_synthesis
//! Prior DASHI trialectic, SensibLaw prototype and Ibrahim PR authorship preserved.

use sensiblaw_pg_source_store::{
    woogaroo_unpaid_trialectic_synthesis, WOOGAROO_ACTION_REF,
    WOOGAROO_MATTER_REF,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let packets = woogaroo_unpaid_trialectic_synthesis();
    if packets.len() != 7 || !packets.iter().all(|p| p.candidate_only
        && !p.creates_actual_review
        && !p.creates_court_filing
        && !p.proves_legal_error_or_compliance
        && !p.orders_physical_protection)
    {
        return Err("Woogaroo candidate-only output boundary failed".into());
    }
    let payload = serde_json::json!({
        "schema_version": "1.0.0",
        "record_type": "woogaroo_dialectic_trialectic_candidate_outputs",
        "matter_ref": WOOGAROO_MATTER_REF,
        "epbc_action": WOOGAROO_ACTION_REF,
        "originals_reviewed": false,
        "ministerial_error_proved": false,
        "legal_compliance_proved": false,
        "filed_with_court": false,
        "habitat_protection_ordered": false,
        "triadic_face_reviewed": false,
        "candidate_packets": packets,
    });
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}
