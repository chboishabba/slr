//! Print the S21 source-driven adversarial case-battery plans.
//!
//! This is a dry-run compiler receipt: it emits only the initial consumer
//! frontier, source/citation seeds and adversarial search roles. No cross-matter
//! join is admitted and no legal conclusion is created.
//!
//! Run:
//!   cargo run -p sensiblaw-legal-runtime --example legal_case_battery_plan

use sensiblaw_legal_runtime::{
    compile_case_battery_initial_plan, legal_case_battery,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for specimen in legal_case_battery() {
        let plan = compile_case_battery_initial_plan(&specimen)?;
        println!("---");
        println!("kind={:?}", plan.kind);
        println!("consumer_ref={}", plan.consumer_ref);
        println!("question_ref={}", plan.question_ref);
        println!("source_seed_refs={}", plan.source_seed_refs.join(","));
        println!("citation_seed_refs={}", plan.citation_seed_refs.join(","));
        println!("frontier_ref={}", plan.frontier.frontier_ref);
        println!(
            "open_residual_count={}",
            plan.frontier.open_residuals().count()
        );
        let roles = plan
            .adversarial_demands
            .iter()
            .map(|demand| format!("{:?}", demand.role))
            .collect::<Vec<_>>();
        println!("adversarial_roles={}", roles.join(","));
        println!("candidate_only={}", plan.candidate_only);
        println!(
            "creates_semantic_authority={}",
            plan.creates_semantic_authority
        );
        println!("creates_claim_truth={}", plan.creates_claim_truth);
    }
    Ok(())
}
