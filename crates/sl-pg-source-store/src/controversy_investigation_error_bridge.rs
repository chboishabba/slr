use crate::{
    ControversyInvestigationError, ControversyInvestigationStoreError,
    InvestigationAcquisitionError,
};

impl From<InvestigationAcquisitionError> for ControversyInvestigationStoreError {
    fn from(error: InvestigationAcquisitionError) -> Self {
        let mapped = match error {
            InvestigationAcquisitionError::InvalidIdentity => {
                ControversyInvestigationError::InvalidIdentity
            }
            InvestigationAcquisitionError::WrongAvailability
            | InvestigationAcquisitionError::InvalidUpdate => {
                ControversyInvestigationError::WrongAcquisitionUpdate
            }
            InvestigationAcquisitionError::WrongObligation => {
                ControversyInvestigationError::WrongRouteObligation
            }
            InvestigationAcquisitionError::MissingAuthorityReceipt => {
                ControversyInvestigationError::MissingAuthorityReceipt
            }
            InvestigationAcquisitionError::MissingIndependenceReceipt => {
                ControversyInvestigationError::MissingIndependenceReceipt
            }
        };
        Self::Domain(mapped)
    }
}
