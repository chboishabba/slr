//! S21.2 Munkara / Tipakalippa WrongType non-collapse run.

use sensiblaw_legal_runtime::{
    exact_munkara_payment_is_reusable, munkara_tipakalippa_source_packet,
    munkara_wrong_type_gap, tipakalippa_rule_wrong_type_for_munkara_reg17_6,
};

fn main() {
    let packet = munkara_tipakalippa_source_packet();
    let wrong_type = munkara_wrong_type_gap();
    let tipakalippa_rule = tipakalippa_rule_wrong_type_for_munkara_reg17_6();
    let exact = exact_munkara_payment_is_reusable();

    println!("source_coordinates={}", packet.len());
    println!("wrong_type_gap={}", wrong_type.gap_ref);
    println!("wrong_type_kind={:?}", wrong_type.kind);
    println!("offered_coordinate={}", wrong_type.coordinate_ref);
    println!("required_proposition={}", wrong_type.proposition_ref);
    println!(
        "tipakalippa_consultation_disposition={:?}",
        tipakalippa_rule.disposition
    );
    println!("exact_munkara_disposition={:?}", exact.disposition);
    println!("candidate_only=true");
    println!("creates_claim_truth=false");
}
