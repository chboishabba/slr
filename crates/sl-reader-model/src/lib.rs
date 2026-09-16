//! Portable, UI-independent reader contracts.
//!
//! This crate projects already-evaluated semantic payment into ordinary reader
//! actions.  It does not acquire sources, decide evidence payment, or promote
//! applicability or claim truth.

/// Stable semantic coordinate supplied by the semantic/payment layer.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SemanticRef(String);

impl SemanticRef {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Revision coordinate for a persisted source manifestation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SourceRevisionRef(String);

impl SourceRevisionRef {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Exact persisted span coordinate.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SpanRef(String);

impl SpanRef {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An unresolved reader-facing coordinate.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ResidualRef(String);

impl ResidualRef {
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<String> for ResidualRef {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for ResidualRef {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Payment for an exact source span.  This deliberately says nothing about
/// proposition support, legal applicability, or claim truth.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourcePayment {
    semantic_ref: SemanticRef,
    source_revision_ref: SourceRevisionRef,
    span_ref: SpanRef,
    exact_authority_span_paid: bool,
}

impl SourcePayment {
    #[must_use]
    pub fn paid(
        semantic_ref: SemanticRef,
        source_revision_ref: impl Into<String>,
        span_ref: SpanRef,
    ) -> Self {
        Self {
            semantic_ref,
            source_revision_ref: SourceRevisionRef::new(source_revision_ref),
            span_ref,
            exact_authority_span_paid: true,
        }
    }

    #[must_use]
    pub fn semantic_ref(&self) -> &SemanticRef {
        &self.semantic_ref
    }

    #[must_use]
    pub fn source_revision_ref(&self) -> &SourceRevisionRef {
        &self.source_revision_ref
    }

    #[must_use]
    pub fn span_ref(&self) -> &SpanRef {
        &self.span_ref
    }

    #[must_use]
    pub fn exact_authority_span_paid(&self) -> bool {
        self.exact_authority_span_paid
    }
}

/// A proposition coordinate is either backed by reviewed evidence or retained
/// as an explicit residual.  A residual is not a negative finding.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CoordinateCoverage {
    Paid { evidence_refs: Vec<String> },
    Residualised(ResidualRef),
}

impl CoordinateCoverage {
    #[must_use]
    pub fn paid(evidence_refs: Vec<String>) -> Self {
        Self::Paid { evidence_refs }
    }

    #[must_use]
    pub fn residual_ref(&self) -> Option<&ResidualRef> {
        match self {
            Self::Paid { .. } => None,
            Self::Residualised(residual_ref) => Some(residual_ref),
        }
    }
}

/// UI-independent requests that may be dispatched by Dioxus, a keyboard, or a
/// future GPU picking surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReaderIntent {
    Explain,
    WhyClaim,
    OpenSource,
    ExpandProofCone,
    Back,
}

/// A bounded, source-aware explanation suitable for reader projection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExplanationCone {
    pub proposition_ref: SemanticRef,
    pub source: SourcePayment,
    pub support_refs: Vec<String>,
    pub qualifier: CoordinateCoverage,
    pub defeater: CoordinateCoverage,
    pub comparator: CoordinateCoverage,
    applicability_paid: bool,
    claim_truth_paid: bool,
}

impl ExplanationCone {
    #[must_use]
    pub fn applicability_paid(&self) -> bool {
        self.applicability_paid
    }

    #[must_use]
    pub fn claim_truth_paid(&self) -> bool {
        self.claim_truth_paid
    }
}

/// Reader state is a projection of payment, never a source of it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReaderDisposition {
    ExecuteSource {
        proposition_ref: SemanticRef,
        source_revision_ref: SourceRevisionRef,
        span_ref: SpanRef,
    },
    ExecuteBoundedWhy(ExplanationCone),
    Defer(Vec<ResidualRef>),
    Reject {
        reason: String,
    },
}

/// Query-indexed reader payment for one proposition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PropositionPayment {
    source: SourcePayment,
    support_refs: Vec<String>,
    qualifier: CoordinateCoverage,
    defeater: CoordinateCoverage,
    comparator: CoordinateCoverage,
    proposition_chain_paid: bool,
    applicability_paid: bool,
    claim_truth_paid: bool,
}

impl PropositionPayment {
    /// Construct an exact-source-only result.  The proposition chain remains
    /// unpaid, so `WhyClaim` defers while `OpenSource` may execute.
    #[must_use]
    pub fn source_only(source: SourcePayment) -> Self {
        Self {
            source,
            support_refs: Vec::new(),
            qualifier: CoordinateCoverage::Residualised(ResidualRef::new(
                "reader-residual:proposition-support",
            )),
            defeater: CoordinateCoverage::Residualised(ResidualRef::new(
                "reader-residual:defeater",
            )),
            comparator: CoordinateCoverage::Residualised(ResidualRef::new(
                "reader-residual:comparator",
            )),
            proposition_chain_paid: false,
            applicability_paid: false,
            claim_truth_paid: false,
        }
    }

    /// Construct a bounded explanation.  The supplied support must be
    /// non-empty; qualifier, defeater and comparator may be paid or retained
    /// explicitly as residuals.
    #[must_use]
    pub fn bounded(
        source: SourcePayment,
        support_refs: Vec<String>,
        qualifier: CoordinateCoverage,
        defeater: CoordinateCoverage,
        comparator: CoordinateCoverage,
    ) -> Self {
        let proposition_chain_paid = source.exact_authority_span_paid && !support_refs.is_empty();
        Self {
            source,
            support_refs,
            qualifier,
            defeater,
            comparator,
            proposition_chain_paid,
            applicability_paid: false,
            claim_truth_paid: false,
        }
    }

    #[must_use]
    pub fn applicability_paid(&self) -> bool {
        self.applicability_paid
    }

    #[must_use]
    pub fn claim_truth_paid(&self) -> bool {
        self.claim_truth_paid
    }

    #[must_use]
    pub fn proposition_chain_paid(&self) -> bool {
        self.proposition_chain_paid
    }

    #[must_use]
    pub fn resolve(&self, intent: ReaderIntent) -> ReaderDisposition {
        match intent {
            ReaderIntent::OpenSource if self.source.exact_authority_span_paid => {
                ReaderDisposition::ExecuteSource {
                    proposition_ref: self.source.semantic_ref.clone(),
                    source_revision_ref: self.source.source_revision_ref.clone(),
                    span_ref: self.source.span_ref.clone(),
                }
            }
            ReaderIntent::WhyClaim | ReaderIntent::ExpandProofCone
                if self.proposition_chain_paid =>
            {
                ReaderDisposition::ExecuteBoundedWhy(ExplanationCone {
                    proposition_ref: self.source.semantic_ref.clone(),
                    source: self.source.clone(),
                    support_refs: self.support_refs.clone(),
                    qualifier: self.qualifier.clone(),
                    defeater: self.defeater.clone(),
                    comparator: self.comparator.clone(),
                    applicability_paid: false,
                    claim_truth_paid: false,
                })
            }
            ReaderIntent::OpenSource => ReaderDisposition::Defer(vec![ResidualRef::new(
                "reader-residual:exact-authority-span",
            )]),
            ReaderIntent::WhyClaim | ReaderIntent::ExpandProofCone => {
                ReaderDisposition::Defer(self.explanation_residuals())
            }
            ReaderIntent::Explain | ReaderIntent::Back => ReaderDisposition::Reject {
                reason: "reader intent needs a selected proposition".into(),
            },
        }
    }

    fn explanation_residuals(&self) -> Vec<ResidualRef> {
        let mut residuals = Vec::new();
        if !self.source.exact_authority_span_paid {
            residuals.push(ResidualRef::new("reader-residual:exact-authority-span"));
        }
        if self.support_refs.is_empty() {
            residuals.push(ResidualRef::new("reader-residual:proposition-support"));
        }
        for coverage in [&self.qualifier, &self.defeater, &self.comparator] {
            if let Some(residual_ref) = coverage.residual_ref() {
                residuals.push(residual_ref.clone());
            }
        }
        residuals
    }
}
