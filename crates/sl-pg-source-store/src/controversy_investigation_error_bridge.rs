use crate::{
    ControversyInvestigationError, ControversyInvestigationStoreError,
    InvestigationAcquisitionError,
};

impl From<InvestigationAcquisitionError> for ControversyInvestigationStoreError {
    fn from(_error: InvestigationAcquisitionError) -> Self {
        Self::Domain(ControversyInvestigationError::InvalidIdentity)
    }
}
