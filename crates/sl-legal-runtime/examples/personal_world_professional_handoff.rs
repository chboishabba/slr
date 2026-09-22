//! M9 executable personal-world -> professional-consumer handoff.

use sensiblaw_legal_runtime::{
    run_personal_world_handoff_experiment, ADVOCATE_CONSUMER, DOCTOR_CONSUMER,
    LAWYER_CONSUMER, PERSONAL_CONSUMER, REGULATOR_CONSUMER,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let run = run_personal_world_handoff_experiment()?;

    for consumer in [
        PERSONAL_CONSUMER,
        LAWYER_CONSUMER,
        DOCTOR_CONSUMER,
        ADVOCATE_CONSUMER,
        REGULATOR_CONSUMER,
    ] {
        let projection = run
            .projections
            .get(consumer)
            .ok_or("projection missing")?;
        println!("consumer={consumer}");
        println!(
            "included={}",
            projection
                .included_coordinate_refs
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        );
        println!(
            "excluded={}",
            projection
                .excluded_coordinate_refs
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        );
        for (coordinate, reason) in &projection.excluded_reason_refs {
            println!("excluded_reason={coordinate}|{reason}");
        }
        println!(
            "unresolved={}",
            projection
                .unresolved_coordinate_refs
                .iter()
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        );
    }

    for (coordinate, consumers) in &run.affected_consumers_by_coordinate {
        println!(
            "affected_consumers={coordinate}|{}",
            consumers.iter().cloned().collect::<Vec<_>>().join(",")
        );
    }

    println!("candidate_only={}", run.candidate_only);
    println!(
        "creates_semantic_authority={}",
        run.creates_semantic_authority
    );
    println!("creates_claim_truth={}", run.creates_claim_truth);

    Ok(())
}
