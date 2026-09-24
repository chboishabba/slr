//! M13 empirical persistence substrate.
//!
//! This is an additive umbrella installer over the already-defined S28–M13
//! persistence schemas. It introduces no new semantic carrier or authority;
//! it only makes the existing statement trace, chat, chronology/contestation,
//! event discovery, review, and operational stores available together.

use crate::{
    install_chat_source_schema, install_chronology_contestation_schema,
    install_event_discovery_schema, install_operational_state_schema,
    install_review_workstation_schema, install_statement_trace_schema,
    DatabaseConfig,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct M13EmpiricalSchemaReceipt {
    pub statement_trace: bool,
    pub chat_source: bool,
    pub chronology_contestation: bool,
    pub event_discovery: bool,
    pub review_workstation: bool,
    pub operational_state: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
    pub canonical_world_mutated: bool,
}

pub fn install_m13_empirical_schema(
    config: &DatabaseConfig,
) -> Result<M13EmpiricalSchemaReceipt, String> {
    // Order is deliberate:
    // 1. source/trace layer over the existing corpus + pnf base,
    // 2. CHAT source materialisation over corpus,
    // 3. semantic chronology/contestation,
    // 4. AUTO/event-discovery,
    // 5. review mutation/receipts,
    // 6. operational/StatiBaker persistence.
    install_statement_trace_schema(config).map_err(|error| format!("{error:?}"))?;
    install_chat_source_schema(config).map_err(|error| format!("{error:?}"))?;
    install_chronology_contestation_schema(config)
        .map_err(|error| format!("{error:?}"))?;
    install_event_discovery_schema(config).map_err(|error| format!("{error:?}"))?;
    install_review_workstation_schema(config).map_err(|error| format!("{error:?}"))?;
    install_operational_state_schema(config).map_err(|error| format!("{error:?}"))?;

    Ok(M13EmpiricalSchemaReceipt {
        statement_trace: true,
        chat_source: true,
        chronology_contestation: true,
        event_discovery: true,
        review_workstation: true,
        operational_state: true,
        creates_semantic_authority: false,
        claim_truth_promoted: false,
        canonical_world_mutated: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receipt_shape_is_non_promoting() {
        let receipt = M13EmpiricalSchemaReceipt {
            statement_trace: true,
            chat_source: true,
            chronology_contestation: true,
            event_discovery: true,
            review_workstation: true,
            operational_state: true,
            creates_semantic_authority: false,
            claim_truth_promoted: false,
            canonical_world_mutated: false,
        };
        assert!(receipt.statement_trace);
        assert!(receipt.chat_source);
        assert!(receipt.chronology_contestation);
        assert!(receipt.event_discovery);
        assert!(receipt.review_workstation);
        assert!(receipt.operational_state);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.claim_truth_promoted);
        assert!(!receipt.canonical_world_mutated);
    }
}
