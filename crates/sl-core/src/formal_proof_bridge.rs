//! Typed bridge between SLR proof obligations and external formal backends.
//!
//! This module carries routing/payment coordinates only. A demand, route receipt,
//! DA51 address, artifact reference, or runtime observation is never proof by
//! itself. Formal payment is accepted only for the exact demand, obligation and
//! statement digest and never promotes legal authority/applicability/claim truth.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalProofDemand {
    pub demand_ref: String,
    pub obligation_ref: String,
    pub consumer_ref: String,
    pub pnf_revision_ref: String,
    pub residual_ref: String,
    pub proposition_ref: Option<String>,
    pub statement_digest: String,
    pub statement_language: String,
    pub target_prover: String,
    pub da51_address: Option<String>,
    pub source_revision_refs: Vec<String>,
    pub requested_result_kind: String,
    pub dependent_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalProofRouteReceiptInput {
    pub route_ref: String,
    pub demand_ref: String,
    pub obligation_ref: String,
    pub statement_digest: String,
    pub establishes_proof: bool,
}

impl FormalProofRouteReceiptInput {
    #[must_use]
    pub const fn can_pay_formal_obligation(&self) -> bool {
        false
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalEvidenceReceiptInput {
    pub evidence_ref: String,
    pub demand_ref: String,
    pub obligation_ref: String,
    pub da51_address: Option<String>,
    pub producer: String,
    pub artifact_ref: String,
    pub artifact_digest: String,
    pub statement_digest: String,
    pub checker: String,
    pub checker_revision: String,
    pub source_revision: String,
    pub result_kind: String,
    pub certification_state: String,
    pub dependencies: Vec<String>,
    pub establishes_formal_premise: bool,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormalEvidencePayment {
    pub demand_ref: String,
    pub obligation_ref: String,
    pub evidence_ref: String,
    pub statement_digest: String,
    pub formal_premise_paid: bool,
    pub reopen_refs: Vec<String>,
    pub creates_legal_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormalProofBridgeError {
    EmptyCoordinate(&'static str),
    EmptySourceRevisionSet,
    AuthorityEscalation,
    DemandMismatch,
    ObligationMismatch,
    StatementDigestMismatch,
    Da51Mismatch,
    ResultKindMismatch,
    UncertifiedEvidence,
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), FormalProofBridgeError> {
    if value.trim().is_empty() {
        Err(FormalProofBridgeError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

impl FormalProofDemand {
    pub fn validate(&self) -> Result<(), FormalProofBridgeError> {
        for (name, value) in [
            ("demand_ref", self.demand_ref.as_str()),
            ("obligation_ref", self.obligation_ref.as_str()),
            ("consumer_ref", self.consumer_ref.as_str()),
            ("pnf_revision_ref", self.pnf_revision_ref.as_str()),
            ("residual_ref", self.residual_ref.as_str()),
            ("statement_digest", self.statement_digest.as_str()),
            ("statement_language", self.statement_language.as_str()),
            ("target_prover", self.target_prover.as_str()),
            ("requested_result_kind", self.requested_result_kind.as_str()),
        ] {
            require_non_empty(name, value)?;
        }
        if self.source_revision_refs.is_empty() {
            return Err(FormalProofBridgeError::EmptySourceRevisionSet);
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.creates_legal_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(FormalProofBridgeError::AuthorityEscalation);
        }
        Ok(())
    }
}

impl FormalEvidenceReceiptInput {
    pub fn validate(&self) -> Result<(), FormalProofBridgeError> {
        for (name, value) in [
            ("evidence_ref", self.evidence_ref.as_str()),
            ("demand_ref", self.demand_ref.as_str()),
            ("obligation_ref", self.obligation_ref.as_str()),
            ("producer", self.producer.as_str()),
            ("artifact_ref", self.artifact_ref.as_str()),
            ("artifact_digest", self.artifact_digest.as_str()),
            ("statement_digest", self.statement_digest.as_str()),
            ("checker", self.checker.as_str()),
            ("checker_revision", self.checker_revision.as_str()),
            ("source_revision", self.source_revision.as_str()),
            ("result_kind", self.result_kind.as_str()),
            ("certification_state", self.certification_state.as_str()),
        ] {
            require_non_empty(name, value)?;
        }
        if self.creates_legal_authority || self.applicability_promoted || self.claim_truth_promoted {
            return Err(FormalProofBridgeError::AuthorityEscalation);
        }
        if !self.establishes_formal_premise || self.certification_state != "kernel-certified" {
            return Err(FormalProofBridgeError::UncertifiedEvidence);
        }
        Ok(())
    }
}

pub fn validate_formal_evidence_payment(
    demand: &FormalProofDemand,
    evidence: &FormalEvidenceReceiptInput,
) -> Result<FormalEvidencePayment, FormalProofBridgeError> {
    demand.validate()?;
    evidence.validate()?;
    if demand.demand_ref != evidence.demand_ref {
        return Err(FormalProofBridgeError::DemandMismatch);
    }
    if demand.obligation_ref != evidence.obligation_ref {
        return Err(FormalProofBridgeError::ObligationMismatch);
    }
    if demand.statement_digest != evidence.statement_digest {
        return Err(FormalProofBridgeError::StatementDigestMismatch);
    }
    if demand.da51_address.is_some() && demand.da51_address != evidence.da51_address {
        return Err(FormalProofBridgeError::Da51Mismatch);
    }
    if demand.requested_result_kind != evidence.result_kind {
        return Err(FormalProofBridgeError::ResultKindMismatch);
    }

    Ok(FormalEvidencePayment {
        demand_ref: demand.demand_ref.clone(),
        obligation_ref: demand.obligation_ref.clone(),
        evidence_ref: evidence.evidence_ref.clone(),
        statement_digest: demand.statement_digest.clone(),
        formal_premise_paid: true,
        reopen_refs: demand.dependent_refs.clone(),
        creates_legal_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}
