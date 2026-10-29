//! Run the persisted Mabo/public-law identity campaign through the generic
//! LegalFollow campaign kernel.
//!
//! Usage:
//!   cargo run -p sensiblaw-world-expansion-runtime --example mabo_generic_legal_follow -- //!     <seed_ref> <review_manifest.tsv> [max_hops] [max_nodes] [max_edges] [max_reviewed_deltas]
//!
//! DATABASE_URL (or the standard local .env) supplies the existing semantic
//! world store.  Review decisions remain explicit TSV input.

use std::{env, fs, path::PathBuf};

use sensiblaw_pg_source_store::{
    load_database_config, load_discovery_identity_baseline,
    load_latent_world_rows_with_budget, LatentWorldBudget,
};
use sensiblaw_world_expansion_runtime::{
    mabo_generic_legal_follow::{
        apply_reviewed_mabo_sequence, mabo_generic_campaign_from_world,
        mabo_generic_campaign_receipt,
    },
    parse_mabo_identity_review_tsv,
};

fn parse_or<T: std::str::FromStr>(arg: Option<&String>, default: T) -> T {
    arg.and_then(|value| value.parse::<T>().ok()).unwrap_or(default)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() < 2 {
        return Err(
            "usage: mabo_generic_legal_follow <seed_ref> <review_manifest.tsv> [max_hops] [max_nodes] [max_edges] [max_reviewed_deltas]"
                .into(),
        );
    }

    let seed_ref = &args[0];
    let review_path = PathBuf::from(&args[1]);
    let max_hops = parse_or(args.get(2), 100_u32);
    let max_nodes = parse_or(args.get(3), 10_000_usize);
    let max_edges = parse_or(args.get(4), 50_000_usize);
    let max_reviewed_deltas = parse_or(args.get(5), 1_000_usize);

    let config = load_database_config(None)?;
    let latent = load_latent_world_rows_with_budget(
        &config,
        seed_ref,
        LatentWorldBudget {
            max_hops,
            max_nodes,
            max_edges,
        },
    )?;
    let baseline = load_discovery_identity_baseline(&config)?;
    let mut campaign =
        mabo_generic_campaign_from_world(&latent, &baseline, max_reviewed_deltas)?;

    let review_text = fs::read_to_string(&review_path)?;
    let reviews = parse_mabo_identity_review_tsv(&review_text)?;
    let applied = apply_reviewed_mabo_sequence(&mut campaign, &reviews)?;
    let receipt = mabo_generic_campaign_receipt(&campaign);

    println!("domain=mabo-public-law");
    println!("seed_ref={seed_ref}");
    println!("generic_kernel=true");
    println!("review_manifest={}", review_path.display());
    println!("reviewed_deltas_applied={applied}");
    println!("reviewed_identity_count={}", receipt.reviewed_identity_count);
    println!("residuals_remaining={}", receipt.residuals_remaining);
    println!("candidate_only={}", receipt.candidate_only);
    println!("creates_semantic_authority={}", receipt.creates_semantic_authority);
    println!("applicability_promoted={}", receipt.applicability_promoted);
    println!("claim_truth_promoted={}", receipt.claim_truth_promoted);

    match campaign.next_demand() {
        Ok(next) => {
            println!("stop=await-reviewed-delta");
            println!("next_representation_ref={}", next.representation_ref);
            println!("next_residual_ref={}", next.residual_ref);
        }
        Err(stop) => {
            println!("stop={stop:?}");
        }
    }

    Ok(())
}
