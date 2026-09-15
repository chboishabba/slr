use sensiblaw_evidential_reopen::{BridgeError, EvidentialBridgeReceipt};

pub const OFFICIAL_JUDGMENT_TEXT_PARSER_CONTRACT: &str =
    "sl.official_judgment.canonical_text.v0_1";
pub const NUMERIC_PNF_COMPILER_CONTRACT: &str = "sl.numeric_pnf_compiler.v0_1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalJudgmentTextReceipt {
    pub document_ref: String,
    pub canonical_text_sha256: String,
    pub paragraph_count: u64,
    pub source_revision_ref: String,
    pub receipt_authority: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgmentPnfBridgeError {
    NonCandidateAuthority,
    EmptyCanonicalTextDigest,
    EmptySourceRevision,
    ZeroParagraphs,
    Bridge(BridgeError),
}

impl From<BridgeError> for JudgmentPnfBridgeError {
    fn from(value: BridgeError) -> Self { Self::Bridge(value) }
}

pub fn compile_canonical_judgment_text_to_pnf_bridge(
    run_ref: impl Into<String>,
    text: &CanonicalJudgmentTextReceipt,
    graph_ref: impl Into<String>,
    residual_demand_refs: Vec<String>,
) -> Result<EvidentialBridgeReceipt, JudgmentPnfBridgeError> {
    if text.receipt_authority != "experimental_candidate_only" {
        return Err(JudgmentPnfBridgeError::NonCandidateAuthority);
    }
    if text.canonical_text_sha256.is_empty() {
        return Err(JudgmentPnfBridgeError::EmptyCanonicalTextDigest);
    }
    if text.source_revision_ref.is_empty() {
        return Err(JudgmentPnfBridgeError::EmptySourceRevision);
    }
    if text.paragraph_count == 0 {
        return Err(JudgmentPnfBridgeError::ZeroParagraphs);
    }

    Ok(EvidentialBridgeReceipt::new(
        run_ref.into(),
        text.document_ref.clone(),
        text.canonical_text_sha256.clone(),
        OFFICIAL_JUDGMENT_TEXT_PARSER_CONTRACT.into(),
        NUMERIC_PNF_COMPILER_CONTRACT.into(),
        graph_ref.into(),
        residual_demand_refs,
        "canonical-official-judgment-text".into(),
        true,
        false,
        false,
    )?)
}

pub const fn canonical_text_bridge_is_semantic_correspondence() -> bool { false }
pub const fn canonical_text_bridge_is_world_truth() -> bool { false }
pub const fn canonical_text_bridge_is_legal_holding() -> bool { false }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_judgment_text_reuses_existing_fail_closed_pnf_bridge() {
        let text = CanonicalJudgmentTextReceipt {
            document_ref: "document:hca:[2026]-HCA-19:canonical-text".into(),
            canonical_text_sha256: "sha256:text-fixture".into(),
            paragraph_count: 321,
            source_revision_ref: "source-revision:sha256:docx-fixture".into(),
            receipt_authority: "experimental_candidate_only",
        };
        let bridge = compile_canonical_judgment_text_to_pnf_bridge(
            "run:hca-cullen-pnf:fixture",
            &text,
            "graph:cullen:source-grounded",
            vec!["residual:current-treatment".into()],
        )
        .unwrap();
        assert_eq!(bridge.document_ref, text.document_ref);
        assert_eq!(bridge.canonical_text_sha256, text.canonical_text_sha256);
        assert_eq!(bridge.parser_contract_ref, OFFICIAL_JUDGMENT_TEXT_PARSER_CONTRACT);
        assert_eq!(bridge.numeric_pnf_compiler_contract_ref, NUMERIC_PNF_COMPILER_CONTRACT);
        assert!(bridge.world_resolution_deferred);
        assert!(!bridge.cross_document_identity_closed);
        assert!(!bridge.parser_observation_is_semantic_authority);
        assert!(bridge.semantic_correspondence_required);
        assert!(!canonical_text_bridge_is_semantic_correspondence());
        assert!(!canonical_text_bridge_is_world_truth());
        assert!(!canonical_text_bridge_is_legal_holding());
    }
}
