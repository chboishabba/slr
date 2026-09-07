//! Offline proof-search runtime spine.
//!
//! The validated single-gap return loop remains available through a preserved
//! legacy module; the additional modules implement the Agda-aligned Offline
//! Research Engine v0.1 surface around it.

pub mod frontier;
pub mod hypothesis;
pub mod local;
pub mod query;
pub mod reasoning;
pub mod receipt;
pub mod world;

#[path = "legacy.rs"]
mod legacy;
pub use legacy::*;
