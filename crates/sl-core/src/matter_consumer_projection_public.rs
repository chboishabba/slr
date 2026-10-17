//! Public module path for the canonical same-Matter consumer projection receipts.
//!
//! The concrete types remain owned by `canonical_evidence`; this module is an aliasing
//! surface only, so callers through either path receive the exact same Rust types.

pub use crate::canonical_evidence::{
    MatterConsumerProjectionError, MatterPersona, MatterRepresentationProjection,
    SameMatterInformativenessReceipt,
};
