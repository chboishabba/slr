use std::{env, process};

use postgres::{Client, NoTls};
use sensiblaw_legal_runtime::{
    project_typed_three_way_workbench_comparison, project_typed_workbench_comparison,
};
use sensiblaw_pg_source_store::{
    load_database_config, load_persisted_workbench_projection,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("typed comparative receipt failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 && args.len() != 3 {
        return Err(
            "usage: m11_typed_workbench_comparison <w0-projection-ref> <w1-projection-ref> [w2-projection-ref]"
                .into(),
        );
    }

    let config = load_database_config(None)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;

    let w0 = load_persisted_workbench_projection(
        &mut client,
        &args[0],
        &format!("world:postgres:{}", args[0]),
    )?;
    let w1 = load_persisted_workbench_projection(
        &mut client,
        &args[1],
        &format!("world:postgres:{}", args[1]),
    )?;

    if args.len() == 2 {
        let comparison =
            project_typed_workbench_comparison("comparison:postgres:w0-w1", &w0, &w1, None)?;
        println!("carrier=typed-rust");
        println!("left_world_ref={}", comparison.left_world_ref);
        println!("right_world_ref={}", comparison.right_world_ref);
        println!("left_graph_ref={}", comparison.left_graph.projection_ref);
        println!("right_graph_ref={}", comparison.right_graph.projection_ref);
        println!("shared_semantic_refs={}", comparison.shared_semantic_refs.len());
        println!("changed_semantic_refs={}", comparison.changed_semantic_refs.len());
        println!("left_only_semantic_refs={}", comparison.left_only_semantic_refs.len());
        println!("right_only_semantic_refs={}", comparison.right_only_semantic_refs.len());
        println!("creates_semantic_authority={}", comparison.creates_semantic_authority);
        println!("creates_claim_truth={}", comparison.creates_claim_truth);
        println!("predicts_outcome={}", comparison.predicts_outcome);
        return Ok(());
    }

    let w2 = load_persisted_workbench_projection(
        &mut client,
        &args[2],
        &format!("world:postgres:{}", args[2]),
    )?;
    let sequence = project_typed_three_way_workbench_comparison(
        "comparison:postgres:w0-w1-w2",
        w0,
        w1,
        w2,
        None,
        None,
    )?;

    println!("carrier=typed-rust");
    println!("w0_graph_ref={}", sequence.w0.legal_follow_graph.projection_ref);
    println!("w1_graph_ref={}", sequence.w1.legal_follow_graph.projection_ref);
    println!("w2_graph_ref={}", sequence.w2.legal_follow_graph.projection_ref);
    println!(
        "w0_w1_changed_semantic_refs={}",
        sequence.w0_to_w1.changed_semantic_refs.len()
    );
    println!(
        "w1_w2_changed_semantic_refs={}",
        sequence.w1_to_w2.changed_semantic_refs.len()
    );
    println!("creates_semantic_authority={}", sequence.creates_semantic_authority);
    println!("creates_claim_truth={}", sequence.creates_claim_truth);
    println!("predicts_outcome={}", sequence.predicts_outcome);
    Ok(())
}
