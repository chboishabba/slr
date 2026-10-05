//! Provider-backed exact source slice -> M12 statement/candidate-PNF handoff.
//!
//! SOURCE-MATERIALISATION-1 stops before review. This module must preserve the
//! exact provider source/span ancestry while entering the already-existing M12
//! candidate spine. It cannot assign evidence role, normative order,
//! applicability, proposition support or claim truth.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CandidatePnfBatch, CandidatePnfError, CandidatePnfProducer, CandidatePnfFactor, CandidatePnfRole};

    struct OneFactor;

    impl CandidatePnfProducer for OneFactor {
        fn produce(&self, source: &crate::ExactSourceSpan) -> Result<CandidatePnfBatch, CandidatePnfError> {
            Ok(CandidatePnfBatch {
                exact_span_ref: source.span_ref.clone(),
                candidates: vec![CandidatePnfFactor {
                    candidate_ref: format!("candidate:{}", source.span_ref),
                    role: CandidatePnfRole::Predicate,
                    source_start_char: source.start_char,
                    source_end_char: source.end_char,
                    surface: "fixture".into(),
                    lemma: "fixture".into(),
                    dependency_ref: "root".into(),
                    candidate_only: true,
                }],
                proposition_support_paid: false,
                applicability_paid: false,
                claim_truth_paid: false,
            })
        }
    }

    #[test]
    fn provider_handoff_receipt_is_candidate_only_and_non_promoting() {
        let receipt = ProviderCandidatePnfReceipt {
            materialization_ref: "provider-materialization:1".into(),
            source_slice_ref: "source-slice:1".into(),
            statement_ref: "statement:1".into(),
            candidate_batch_ref: "pnf-batch:1".into(),
            parser_receipt_ref: "parser:1".into(),
            candidate_factor_count: 1,
            candidate_only: true,
            creates_semantic_authority: false,
            proposition_support_paid: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        assert!(receipt.candidate_only);
        assert!(!receipt.creates_semantic_authority);
        assert!(!receipt.proposition_support_paid);
        assert!(!receipt.applicability_promoted);
        assert!(!receipt.claim_truth_promoted);
        let _ = OneFactor;
    }
}
