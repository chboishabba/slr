//! Exact candidate/admission/source-span weld for atomic attribution.
//!
//! This is the runtime analogue of the Agda attributed-source / PNF-candidate
//! weld.  Parser or expanded-candidate identity is not enough: the source span
//! and stable candidate address must match an already admitted semantic delta.
//! The weld creates no atomic gate, truth, legal authority or promotion.

use sensiblaw_core::{FibreAddress, TextSpan};
use sensiblaw_semantic_admission::AdmittedNormativeDelta;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpanAnchor {
    pub source_id: String,
    pub source_revision: String,
    pub exact_locator: String,
    pub address: FibreAddress,
    pub span: TextSpan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmittedAtomicSourceWeld {
    pub atom_id: String,
    pub source_anchor: SourceSpanAnchor,
    pub admitted: AdmittedNormativeDelta,
    pub weld_reference: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceWeldError {
    AddressMismatch,
    SpanMismatch,
    MissingSourceId,
    MissingSourceRevision,
    MissingExactLocator,
    MissingAtomId,
    MissingWeldReference,
}

pub fn weld_admitted_candidate_to_source(
    atom_id: impl Into<String>,
    source_anchor: SourceSpanAnchor,
    admitted: AdmittedNormativeDelta,
    weld_reference: impl Into<String>,
) -> Result<AdmittedAtomicSourceWeld, SourceWeldError> {
    let atom_id = atom_id.into();
    let weld_reference = weld_reference.into();
    if atom_id.trim().is_empty() {
        return Err(SourceWeldError::MissingAtomId);
    }
    if source_anchor.source_id.trim().is_empty() {
        return Err(SourceWeldError::MissingSourceId);
    }
    if source_anchor.source_revision.trim().is_empty() {
        return Err(SourceWeldError::MissingSourceRevision);
    }
    if source_anchor.exact_locator.trim().is_empty() {
        return Err(SourceWeldError::MissingExactLocator);
    }
    if weld_reference.trim().is_empty() {
        return Err(SourceWeldError::MissingWeldReference);
    }
    if source_anchor.address != admitted.address {
        return Err(SourceWeldError::AddressMismatch);
    }
    if source_anchor.span != admitted.source_span {
        return Err(SourceWeldError::SpanMismatch);
    }
    Ok(AdmittedAtomicSourceWeld {
        atom_id,
        source_anchor,
        admitted,
        weld_reference,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_semantic_admission::{ResolutionAuthority, ResolvedScope};
    use sensiblaw_semantic_expansion::{ExpandedCandidateKind, StableHeadRelation};

    fn admitted() -> AdmittedNormativeDelta {
        AdmittedNormativeDelta {
            kind: ExpandedCandidateKind::ReferenceRelation,
            source_span: TextSpan::new(1, 100, 140).unwrap(),
            address: FibreAddress {
                sentence_id: 39,
                local_ordinal: 2,
            },
            head: StableHeadRelation::Root,
            resolved_scope: ResolvedScope::ContextResolved,
            authority: ResolutionAuthority::HumanReview,
            policy_reference: "policy:cullen-gold-v1".into(),
            resolver_reference: "resolver:fixture-review".into(),
        }
    }

    fn anchor() -> SourceSpanAnchor {
        SourceSpanAnchor {
            source_id: "source:HCA:Cullen-v-NSW:2026:HCA19".into(),
            source_revision: "HCA-2026-19-official-pdf".into(),
            exact_locator: "joint reasons [39]".into(),
            address: FibreAddress {
                sentence_id: 39,
                local_ordinal: 2,
            },
            span: TextSpan::new(1, 100, 140).unwrap(),
        }
    }

    #[test]
    fn exact_admitted_span_and_address_weld() {
        let weld = weld_admitted_candidate_to_source(
            "atom:NSW:CLA:s5B1a:risk-foreseeable",
            anchor(),
            admitted(),
            "weld:Cullen:39:foreseeable",
        )
        .unwrap();
        assert_eq!(weld.source_anchor.exact_locator, "joint reasons [39]");
    }

    #[test]
    fn matching_locator_text_cannot_hide_span_mismatch() {
        let mut wrong = anchor();
        wrong.span = TextSpan::new(1, 101, 140).unwrap();
        assert_eq!(
            weld_admitted_candidate_to_source(
                "atom:NSW:CLA:s5B1a:risk-foreseeable",
                wrong,
                admitted(),
                "weld:Cullen:39:foreseeable",
            ),
            Err(SourceWeldError::SpanMismatch)
        );
    }

    #[test]
    fn matching_span_cannot_hide_candidate_address_mismatch() {
        let mut wrong = anchor();
        wrong.address.local_ordinal = 3;
        assert_eq!(
            weld_admitted_candidate_to_source(
                "atom:NSW:CLA:s5B1a:risk-foreseeable",
                wrong,
                admitted(),
                "weld:Cullen:39:foreseeable",
            ),
            Err(SourceWeldError::AddressMismatch)
        );
    }
}
