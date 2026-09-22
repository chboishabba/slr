//! Offline/online proof-search runtime spine.
//!
//! The validated single-gap return loop remains available through a preserved
//! legacy module; the additional modules implement the Agda-aligned research
//! engine around it. Governed provider acquisition rejoins the same spine only
//! after local ingestion through `acquisition`, and live acquisition scheduling
//! must first bind to the exact open proof residual through `bound_acquisition`.

pub mod acquisition;
pub mod bound_acquisition;
pub mod cullen_premise_audit;
pub mod contract_review_expansion;
pub mod engine;
pub mod frontier;
pub mod historical_legislation;
pub mod hypothesis;
pub mod judgment_candidates;
pub mod judgment_pnf;
pub mod judgment_review;
pub mod live_artifact;
pub mod live_artifact_validation;
pub mod local;
pub mod online;
pub mod oalc_judgment_materialization;
pub mod planner;
pub mod provider;
pub mod provider_access_policy;
pub mod query;
pub mod reader_retry;
pub mod reasoning;
pub mod receipt;
pub mod residual_review_shortlist;
pub mod review_unit_review;
pub mod review_units;
pub mod transition;
pub mod treatment_genealogy;
pub mod waltons_proposition_review;
pub mod world;
pub mod world_acquisition;
pub mod world_expansion;
pub mod world_expansion_adapters;
pub mod world_expansion_identity_session;
pub mod world_expansion_reentry;
pub mod world_expansion_runner;
pub mod world_expansion_session;
pub mod world_expansion_step;
pub mod world_identity;
pub mod world_identity_guard;
pub mod world_identity_lineage;
pub mod world_known_identity_payment;
pub mod world_observation;
pub mod world_observation_adapters;

#[path = "legacy.rs"]
mod legacy;
pub use legacy::*;
