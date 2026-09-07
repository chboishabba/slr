//! Offline/online proof-search runtime spine.
//!
//! The validated single-gap return loop remains available through a preserved
//! legacy module; the additional modules implement the Agda-aligned research
//! engine around it. Governed provider acquisition rejoins the same spine only
//! after local ingestion through `acquisition`, and live acquisition scheduling
//! must first bind to the exact open proof residual through `bound_acquisition`.

pub mod acquisition;
pub mod bound_acquisition;
pub mod engine;
pub mod frontier;
pub mod hypothesis;
pub mod judgment_candidates;
pub mod judgment_pnf;
pub mod judgment_review;
pub mod live_artifact;
pub mod live_artifact_validation;
pub mod local;
pub mod online;
pub mod planner;
pub mod provider;
pub mod provider_access_policy;
pub mod query;
pub mod reasoning;
pub mod receipt;
pub mod residual_review_shortlist;
pub mod review_unit_review;
pub mod review_units;
pub mod transition;
pub mod world;
pub mod world_acquisition;

#[path = "legacy.rs"]
mod legacy;
pub use legacy::*;
