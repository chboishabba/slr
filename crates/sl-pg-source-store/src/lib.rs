mod proposition_rows;
pub use proposition_rows::{
    load_mabo_proposition_rows, PropositionObservationRow, PropositionResidualRow, PropositionRows,
};

// Keep the established PostgreSQL source-store implementation byte-for-byte
// while this focused reader projection remains a separate concern.
include!("storage_core.rs");
