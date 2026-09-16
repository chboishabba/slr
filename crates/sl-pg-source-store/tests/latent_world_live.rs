use sensiblaw_pg_source_store::{load_database_config, load_latent_world_rows};

const MABO_PROPOSITION: &str = "mabo:proposition:radical-title-native-title";

#[test]
#[ignore = "requires the live PostgreSQL semantic persistence spine"]
fn live_mabo_world_is_bounded_read_only_projection() {
    let config = load_database_config(None).expect("DATABASE_URL must identify the live PG store");
    let world = load_latent_world_rows(&config, MABO_PROPOSITION, 100, 20_000)
        .expect("latent Mabo neighbourhood must be readable from existing semantic tables");
    assert_eq!(world.seed_ref, MABO_PROPOSITION);
    assert_eq!(world.max_hops, 100);
    assert!(!world.creates_semantic_authority);
    assert!(world.edges.len() <= 20_000);
}
