//! Offline proof-search runtime spine.
//!
//! The legacy single-gap return loop remains included below; the additional
//! modules implement the Agda-aligned Offline Research Engine v0.1 surface.

pub mod frontier;
pub mod hypothesis;
pub mod local;
pub mod query;
pub mod reasoning;
pub mod receipt;
pub mod world;

include!("legacy.rs");
