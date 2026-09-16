mod legal_ir_materialization;
mod proposition_rows;

pub use legal_ir_materialization::{
    materialize_reviewed_proposition_support, LegalIrMaterializationError,
    MaterializedLegalIrRefs, ReviewedPropositionSupport,
};
pub use proposition_rows::{load_mabo_proposition_rows, PropositionObservationRow, PropositionRows};

// Keep the established PostgreSQL source-store implementation byte-for-byte
// while focused reader/materialisation projections remain separate concerns.
include!("storage_core.rs");
