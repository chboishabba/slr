//! SLR bridge from persisted proof obligations to externally routed formal proof work.
//!
//! The bridge is intentionally authority-neutral. It persists only demand/route/evidence
//! joins and reopens existing SLR-owned proof-obligation / reviewed-PNF identities.
//! A route, DA51 address, artifact, CID, runtime trace, or translator capability never
//! establishes proof. Only an exact, independently certified evidence receipt can pay the
//! formal premise of the exact demand it was issued for.

use postgres::{Client, NoTls, Transaction};
use thiserror::Error;

pub const FORMAL_PROOF_BRIDGE_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS formal_bridge;

CREATE TABLE IF NOT EXISTS formal_bridge.demand (
  demand_ref TEXT PRIMARY KEY,
  obligation_ref TEXT NOT NULL REFERENCES legal_controversy.proof_obligation(obligation_ref),
  consumer_ref TEXT NOT NULL,
  pnf_revision_ref TEXT NOT NULL REFERENCES algebra.factor_revision(factor_revision_ref),
  residual_ref TEXT NOT NULL,
  proposition_ref TEXT NULL,
  statement_digest TEXT NOT NULL,
  statement_language TEXT NOT NULL,
  target_prover TEXT NOT NULL,
  da51_address TEXT NULL,
  source_revision_refs TEXT[] NOT NULL,
  requested_result_kind TEXT NOT NULL,
  dependent_refs TEXT[] NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS formal_bridge.route_receipt (
  route_ref TEXT PRIMARY KEY,
  demand_ref TEXT NOT NULL REFERENCES formal_bridge.demand(demand_ref),
  obligation_ref TEXT NOT NULL,
  statement_digest TEXT NOT NULL,
  establishes_proof BOOLEAN NOT NULL CHECK (NOT establishes_proof)
);

CREATE TABLE IF NOT EXISTS formal_bridge.evidence (
  evidence_ref TEXT PRIMARY KEY,
  demand_ref TEXT NOT NULL REFERENCES formal_bridge.demand(demand_ref),
  obligation_ref TEXT NOT NULL,
  da51_address TEXT NULL,
  producer TEXT NOT NULL,
  artifact_ref TEXT NOT NULL,
  artifact_digest TEXT NOT NULL,
  statement_digest TEXT NOT NULL,
  checker TEXT NOT NULL,
  checker_revision TEXT NOT NULL,
  source_revision TEXT NOT NULL,
  result_kind TEXT NOT NULL,
  certification_state TEXT NOT NULL,
  dependencies TEXT[] NOT NULL,
  establishes_formal_premise BOOLEAN NOT NULL,
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);

CREATE TABLE IF NOT EXISTS formal_bridge.payment (
  evidence_ref TEXT PRIMARY KEY REFERENCES formal_bridge.evidence(evidence_ref),
  demand_ref TEXT NOT NULL REFERENCES formal_bridge.demand(demand_ref),
  obligation_ref TEXT NOT NULL,
  statement_digest TEXT NOT NULL,
  reopen_refs TEXT[] NOT NULL,
  formal_premise_paid BOOLEAN NOT NULL,
  creates_legal_authority BOOLEAN NOT NULL CHECK (NOT creates_legal_authority),
  applicability_promoted BOOLEAN NOT NULL CHECK (NOT applicability_promoted),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

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
    pub const fn can_pay_formal_obligation(&self) -> bool { false }
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

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum FormalProofBridgeError {
    #[error("empty bridge coordinate: {0}")]
    EmptyCoordinate(&'static str),
    #[error("source revision set is empty")]
    EmptySourceRevisionSet,
    #[error("authority escalation is forbidden")]
    AuthorityEscalation,
    #[error("demand mismatch")]
    DemandMismatch,
    #[error("obligation mismatch")]
    ObligationMismatch,
    #[error("statement digest mismatch")]
    StatementDigestMismatch,
    #[error("DA51 address mismatch")]
    Da51Mismatch,
    #[error("result kind mismatch")]
    ResultKindMismatch,
    #[error("evidence is not independently kernel-certified")]
    UncertifiedEvidence,
    #[error("route receipts never pay formal obligations")]
    RouteIsNotProof,
    #[error("upstream obligation is missing or closed")]
    MissingOpenObligation,
    #[error("reviewed PNF revision is missing")]
    MissingPnfRevision,
    #[error("persisted row conflicts with immutable coordinates")]
    ExistingRowConflict,
    #[error("postgres error: {0}")]
    Postgres(String),
}

impl From<postgres::Error> for FormalProofBridgeError {
    fn from(value: postgres::Error) -> Self { Self::Postgres(value.to_string()) }
}

fn require_non_empty(name: &'static str, value: &str) -> Result<(), FormalProofBridgeError> {
    if value.trim().is_empty() { Err(FormalProofBridgeError::EmptyCoordinate(name)) } else { Ok(()) }
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
        ] { require_non_empty(name, value)?; }
        if self.source_revision_refs.is_empty() { return Err(FormalProofBridgeError::EmptySourceRevisionSet); }
        if !self.candidate_only || self.creates_semantic_authority || self.creates_legal_authority || self.applicability_promoted || self.claim_truth_promoted {
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
        ] { require_non_empty(name, value)?; }
        if self.creates_legal_authority || self.applicability_promoted || self.claim_truth_promoted {
            return Err(FormalProofBridgeError::AuthorityEscalation);
        }
        if !self.establishes_formal_premise || self.certification_state != "kernel-certified" {
            return Err(FormalProofBridgeError::UncertifiedEvidence);
        }
        Ok(())
    }
}

pub fn validate_formal_evidence_payment(demand: &FormalProofDemand, evidence: &FormalEvidenceReceiptInput) -> Result<FormalEvidencePayment, FormalProofBridgeError> {
    demand.validate()?;
    evidence.validate()?;
    if demand.demand_ref != evidence.demand_ref { return Err(FormalProofBridgeError::DemandMismatch); }
    if demand.obligation_ref != evidence.obligation_ref { return Err(FormalProofBridgeError::ObligationMismatch); }
    if demand.statement_digest != evidence.statement_digest { return Err(FormalProofBridgeError::StatementDigestMismatch); }
    if demand.da51_address.is_some() && demand.da51_address != evidence.da51_address { return Err(FormalProofBridgeError::Da51Mismatch); }
    if demand.requested_result_kind != evidence.result_kind { return Err(FormalProofBridgeError::ResultKindMismatch); }
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

pub fn install_schema(database_url: &str) -> Result<(), FormalProofBridgeError> {
    let mut client = Client::connect(database_url, NoTls)?;
    client.batch_execute(FORMAL_PROOF_BRIDGE_SQL)?;
    Ok(())
}

fn require_existing_inputs(tx: &mut Transaction<'_>, demand: &FormalProofDemand) -> Result<(), FormalProofBridgeError> {
    let obligation_open: bool = tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM legal_controversy.proof_obligation WHERE obligation_ref=$1 AND is_open)",
        &[&demand.obligation_ref],
    )?.get(0);
    if !obligation_open { return Err(FormalProofBridgeError::MissingOpenObligation); }
    let pnf_present: bool = tx.query_one(
        "SELECT EXISTS (SELECT 1 FROM algebra.factor_revision WHERE factor_revision_ref=$1)",
        &[&demand.pnf_revision_ref],
    )?.get(0);
    if !pnf_present { return Err(FormalProofBridgeError::MissingPnfRevision); }
    Ok(())
}

pub fn persist_formal_demand(database_url: &str, demand: &FormalProofDemand) -> Result<(), FormalProofBridgeError> {
    demand.validate()?;
    let mut client = Client::connect(database_url, NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(FORMAL_PROOF_BRIDGE_SQL)?;
    require_existing_inputs(&mut tx, demand)?;
    tx.execute(
        r#"INSERT INTO formal_bridge.demand
        (demand_ref,obligation_ref,consumer_ref,pnf_revision_ref,residual_ref,proposition_ref,statement_digest,statement_language,target_prover,da51_address,source_revision_refs,requested_result_kind,dependent_refs,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,TRUE,FALSE,FALSE,FALSE,FALSE)
        ON CONFLICT (demand_ref) DO NOTHING"#,
        &[&demand.demand_ref,&demand.obligation_ref,&demand.consumer_ref,&demand.pnf_revision_ref,&demand.residual_ref,&demand.proposition_ref,&demand.statement_digest,&demand.statement_language,&demand.target_prover,&demand.da51_address,&demand.source_revision_refs,&demand.requested_result_kind,&demand.dependent_refs],
    )?;
    let row = tx.query_one("SELECT obligation_ref,pnf_revision_ref,statement_digest FROM formal_bridge.demand WHERE demand_ref=$1", &[&demand.demand_ref])?;
    let got_obligation: String = row.get(0);
    let got_pnf: String = row.get(1);
    let got_digest: String = row.get(2);
    if got_obligation != demand.obligation_ref || got_pnf != demand.pnf_revision_ref || got_digest != demand.statement_digest {
        return Err(FormalProofBridgeError::ExistingRowConflict);
    }
    tx.commit()?;
    Ok(())
}

pub fn persist_route_receipt(database_url: &str, route: &FormalProofRouteReceiptInput) -> Result<(), FormalProofBridgeError> {
    if route.establishes_proof { return Err(FormalProofBridgeError::RouteIsNotProof); }
    for (name, value) in [("route_ref", route.route_ref.as_str()), ("demand_ref", route.demand_ref.as_str()), ("obligation_ref", route.obligation_ref.as_str()), ("statement_digest", route.statement_digest.as_str())] { require_non_empty(name, value)?; }
    let mut client = Client::connect(database_url, NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(FORMAL_PROOF_BRIDGE_SQL)?;
    let d = tx.query_one("SELECT obligation_ref,statement_digest FROM formal_bridge.demand WHERE demand_ref=$1", &[&route.demand_ref])?;
    let obligation_ref: String = d.get(0);
    let statement_digest: String = d.get(1);
    if obligation_ref != route.obligation_ref { return Err(FormalProofBridgeError::ObligationMismatch); }
    if statement_digest != route.statement_digest { return Err(FormalProofBridgeError::StatementDigestMismatch); }
    tx.execute("INSERT INTO formal_bridge.route_receipt (route_ref,demand_ref,obligation_ref,statement_digest,establishes_proof) VALUES ($1,$2,$3,$4,FALSE) ON CONFLICT (route_ref) DO NOTHING", &[&route.route_ref,&route.demand_ref,&route.obligation_ref,&route.statement_digest])?;
    tx.commit()?;
    Ok(())
}

pub fn persist_and_validate_formal_evidence(database_url: &str, evidence: &FormalEvidenceReceiptInput) -> Result<FormalEvidencePayment, FormalProofBridgeError> {
    evidence.validate()?;
    let mut client = Client::connect(database_url, NoTls)?;
    let mut tx = client.transaction()?;
    tx.batch_execute(FORMAL_PROOF_BRIDGE_SQL)?;
    let row = tx.query_one(
        "SELECT obligation_ref,consumer_ref,pnf_revision_ref,residual_ref,proposition_ref,statement_digest,statement_language,target_prover,da51_address,source_revision_refs,requested_result_kind,dependent_refs,candidate_only,creates_semantic_authority,creates_legal_authority,applicability_promoted,claim_truth_promoted FROM formal_bridge.demand WHERE demand_ref=$1",
        &[&evidence.demand_ref],
    )?;
    let demand = FormalProofDemand {
        demand_ref: evidence.demand_ref.clone(), obligation_ref: row.get(0), consumer_ref: row.get(1), pnf_revision_ref: row.get(2), residual_ref: row.get(3), proposition_ref: row.get(4), statement_digest: row.get(5), statement_language: row.get(6), target_prover: row.get(7), da51_address: row.get(8), source_revision_refs: row.get(9), requested_result_kind: row.get(10), dependent_refs: row.get(11), candidate_only: row.get(12), creates_semantic_authority: row.get(13), creates_legal_authority: row.get(14), applicability_promoted: row.get(15), claim_truth_promoted: row.get(16),
    };
    let payment = validate_formal_evidence_payment(&demand, evidence)?;
    tx.execute(
        r#"INSERT INTO formal_bridge.evidence
        (evidence_ref,demand_ref,obligation_ref,da51_address,producer,artifact_ref,artifact_digest,statement_digest,checker,checker_revision,source_revision,result_kind,certification_state,dependencies,establishes_formal_premise,creates_legal_authority,applicability_promoted,claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,TRUE,FALSE,FALSE,FALSE)
        ON CONFLICT (evidence_ref) DO NOTHING"#,
        &[&evidence.evidence_ref,&evidence.demand_ref,&evidence.obligation_ref,&evidence.da51_address,&evidence.producer,&evidence.artifact_ref,&evidence.artifact_digest,&evidence.statement_digest,&evidence.checker,&evidence.checker_revision,&evidence.source_revision,&evidence.result_kind,&evidence.certification_state,&evidence.dependencies],
    )?;
    tx.execute(
        "INSERT INTO formal_bridge.payment (evidence_ref,demand_ref,obligation_ref,statement_digest,reopen_refs,formal_premise_paid,creates_legal_authority,applicability_promoted,claim_truth_promoted) VALUES ($1,$2,$3,$4,$5,TRUE,FALSE,FALSE,FALSE) ON CONFLICT (evidence_ref) DO NOTHING",
        &[&payment.evidence_ref,&payment.demand_ref,&payment.obligation_ref,&payment.statement_digest,&payment.reopen_refs],
    )?;
    tx.commit()?;
    Ok(payment)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn demand() -> FormalProofDemand {
        FormalProofDemand {
            demand_ref: "formal-demand:1".into(), obligation_ref: "obligation:1".into(), consumer_ref: "consumer:1".into(), pnf_revision_ref: "pnf:1".into(), residual_ref: "residual:1".into(), proposition_ref: Some("proposition:1".into()), statement_digest: "sha256:statement-a".into(), statement_language: "lean4".into(), target_prover: "lean".into(), da51_address: Some("0xDA51831100800000".into()), source_revision_refs: vec!["source:1".into()], requested_result_kind: "formal-proof".into(), dependent_refs: vec!["dependent:1".into()], candidate_only: true, creates_semantic_authority: false, creates_legal_authority: false, applicability_promoted: false, claim_truth_promoted: false,
        }
    }

    fn evidence() -> FormalEvidenceReceiptInput {
        FormalEvidenceReceiptInput {
            evidence_ref: "evidence:1".into(), demand_ref: "formal-demand:1".into(), obligation_ref: "obligation:1".into(), da51_address: Some("0xDA51831100800000".into()), producer: "lean-kernel".into(), artifact_ref: "artifact:1".into(), artifact_digest: "sha256:artifact".into(), statement_digest: "sha256:statement-a".into(), checker: "lean".into(), checker_revision: "lean:head".into(), source_revision: "source:1".into(), result_kind: "formal-proof".into(), certification_state: "kernel-certified".into(), dependencies: vec![], establishes_formal_premise: true, creates_legal_authority: false, applicability_promoted: false, claim_truth_promoted: false,
        }
    }

    #[test]
    fn wrong_obligation_fails_closed() {
        let d = demand();
        let mut e = evidence();
        e.obligation_ref = "obligation:other".into();
        assert_eq!(validate_formal_evidence_payment(&d, &e), Err(FormalProofBridgeError::ObligationMismatch));
    }

    #[test]
    fn wrong_statement_digest_fails_closed() {
        let d = demand();
        let mut e = evidence();
        e.statement_digest = "sha256:changed".into();
        assert_eq!(validate_formal_evidence_payment(&d, &e), Err(FormalProofBridgeError::StatementDigestMismatch));
    }

    #[test]
    fn route_is_never_proof() {
        let route = FormalProofRouteReceiptInput { route_ref: "route:1".into(), demand_ref: "formal-demand:1".into(), obligation_ref: "obligation:1".into(), statement_digest: "sha256:statement-a".into(), establishes_proof: false };
        assert!(!route.can_pay_formal_obligation());
    }

    #[test]
    fn payment_reopens_only_declared_refs_and_never_legal_authority() {
        let payment = validate_formal_evidence_payment(&demand(), &evidence()).unwrap();
        assert_eq!(payment.reopen_refs, vec!["dependent:1"]);
        assert!(payment.formal_premise_paid);
        assert!(!payment.creates_legal_authority);
        assert!(!payment.applicability_promoted);
        assert!(!payment.claim_truth_promoted);
    }
}
