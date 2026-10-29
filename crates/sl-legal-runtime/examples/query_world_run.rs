//! Generic headless S15/S18 query-world runner.
//!
//! Usage:
//!   cargo run -p sensiblaw-legal-runtime --example query_world_run -- \
//!     <input.json> [output.json]
//!
//! The input contains only explicit world/query/projection/dependency state and
//! formal typecheck receipts.  Raw theorem metadata is never accepted as a
//! checked witness here.

use serde::Deserialize;
use sensiblaw_legal_runtime::{
    decide_query_world_run, kernel_checked_factors_through_witness,
    kernel_checked_nonfactorability_witness, query_world_run_receipt,
    AgdaFactorsThroughTypecheckReceipt, AgdaNonFactorabilityTypecheckReceipt,
    ConsumerCoverage, ConsumerQueryDemand, LegalWorldCoordinate,
    OperationalResearchState, ProjectionGraph, QueryDependencySlice,
    RevisionDependencyIndex,
};
use std::{env, fs, path::PathBuf};

#[derive(Debug, Deserialize)]
struct QueryWorldRunInput {
    old_world: LegalWorldCoordinate,
    new_world: LegalWorldCoordinate,
    dependencies: RevisionDependencyIndex,
    slice: QueryDependencySlice,
    demand: ConsumerQueryDemand,
    graph: ProjectionGraph,
    coverage: ConsumerCoverage,
    operational_state: OperationalResearchState,
    #[serde(default)]
    factors_through_receipt: Option<AgdaFactorsThroughTypecheckReceipt>,
    #[serde(default)]
    nonfactorability_receipts: Vec<AgdaNonFactorabilityTypecheckReceipt>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        return Err("usage: query_world_run <input.json> [output.json]".into());
    }

    let input_path = PathBuf::from(&args[0]);
    let output_path = args.get(1).map(PathBuf::from);
    let input: QueryWorldRunInput =
        serde_json::from_slice(&fs::read(&input_path)?)?;

    let positive = input
        .factors_through_receipt
        .as_ref()
        .map(kernel_checked_factors_through_witness)
        .transpose()?;

    let negatives = input
        .nonfactorability_receipts
        .iter()
        .map(kernel_checked_nonfactorability_witness)
        .collect::<Result<Vec<_>, _>>()?;

    let decision = decide_query_world_run(
        &input.old_world,
        &input.new_world,
        &input.dependencies,
        &input.slice,
        &input.demand,
        &input.graph,
        &input.coverage,
        input.operational_state,
        positive.as_ref(),
        &negatives,
    )?;
    let receipt = query_world_run_receipt(&decision);
    let encoded = serde_json::to_vec_pretty(&receipt)?;

    println!("{}", String::from_utf8_lossy(&encoded));
    if let Some(path) = output_path {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)?;
            }
        }
        fs::write(&path, &encoded)?;
        eprintln!("receipt={}", path.display());
    }

    Ok(())
}
