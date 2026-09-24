use std::{env, process};

use postgres::{Client, NoTls};
use sensiblaw_pg_source_store::{
    consecutive_projection_pairs, consecutive_projection_triples, list_legal_follow_projection_summaries,
    load_database_config,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("comparative projection discovery failed: {error}");
        process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let limit = env::args()
        .nth(1)
        .map(|value| value.parse::<i64>())
        .transpose()?
        .unwrap_or(200);

    let config = load_database_config(None)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;

    let summaries = list_legal_follow_projection_summaries(&mut client, limit)?;
    let pairs = consecutive_projection_pairs(&summaries);
    let triples = consecutive_projection_triples(&summaries);

    println!("carrier=typed-rust");
    println!("database_source=postgres");
    println!("projection_summary_count={}", summaries.len());
    println!("consecutive_pair_count={}", pairs.len());
    println!("consecutive_triple_count={}", triples.len());

    for summary in &summaries {
        println!(
            "projection	document_ref={}	projection_ref={}	created_at={}	nodes={}	edges={}",
            summary.document_ref,
            summary.projection_ref,
            summary.created_at,
            summary.node_count,
            summary.edge_count,
        );
    }

    for pair in &pairs {
        println!(
            "pair	document_ref={}	before={}	after={}	before_created_at={}	after_created_at={}",
            pair.document_ref,
            pair.before_projection_ref,
            pair.after_projection_ref,
            pair.before_created_at,
            pair.after_created_at,
        );
    }

    for triple in &triples {
        println!(
            "triple	document_ref={}	w0={}	w1={}	w2={}	w0_created_at={}	w1_created_at={}	w2_created_at={}",
            triple.document_ref,
            triple.w0_projection_ref,
            triple.w1_projection_ref,
            triple.w2_projection_ref,
            triple.w0_created_at,
            triple.w1_created_at,
            triple.w2_created_at,
        );
    }

    Ok(())
}
