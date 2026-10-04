//! Durable acceptance/control-plane requests.
//!
//! Normal acceptance runners load these rows by stable reference.  JSON is not
//! part of this ABI: import/replay tools may construct the typed drafts below,
//! but case membership, native-product selectors, consumer fibres and expected
//! findings are PostgreSQL-owned before execution begins.

use postgres::{Client, GenericClient, NoTls};
use thiserror::Error;

use crate::{
    AccessDisposition, AcquisitionRouteCandidate, AiUseState, AlignmentDirection,
    ComparisonFinding, DatabaseConfig, DuplicateRelation, EvidenceIndependence,
    GovernanceAccessState, LicensedAlignment, ObservationContext, PrivacyExposureState,
    RelationalConsumer, RelationalSourceFamily, ResidualKind, RoleTypeDemand,
};

/// REL/common acceptance tables.  This deliberately has no reviewed-evidence
/// dependency, so a REL-only corpus can persist independently of legal/INV.
pub const REL_ACCEPTANCE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS acceptance;

CREATE TABLE IF NOT EXISTS acceptance.rel_consumer (
  consumer_ref TEXT PRIMARY KEY,
  compare_scope BOOLEAN NOT NULL,
  compare_time BOOLEAN NOT NULL,
  compare_modality BOOLEAN NOT NULL,
  compare_quantifier BOOLEAN NOT NULL,
  compare_attribution BOOLEAN NOT NULL,
  compare_ontology BOOLEAN NOT NULL,
  compare_language BOOLEAN NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_consumer_source_family (
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref) ON DELETE CASCADE,
  family_ref TEXT NOT NULL,
  PRIMARY KEY (consumer_ref, family_ref)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_consumer_required_role (
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref) ON DELETE CASCADE,
  role_ref TEXT NOT NULL,
  PRIMARY KEY (consumer_ref, role_ref)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_consumer_role_type_demand (
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref) ON DELETE CASCADE,
  role_ref TEXT NOT NULL,
  required_type_ref TEXT NOT NULL,
  contract_ref TEXT NOT NULL,
  PRIMARY KEY (consumer_ref, role_ref, required_type_ref, contract_ref)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_consumer_alignment (
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref) ON DELETE CASCADE,
  axis_ref TEXT NOT NULL,
  left_ref TEXT NOT NULL,
  right_ref TEXT NOT NULL,
  witness_ref TEXT NOT NULL,
  licence_ref TEXT NOT NULL,
  direction_ref TEXT NOT NULL,
  PRIMARY KEY (consumer_ref, axis_ref, left_ref, right_ref, witness_ref, licence_ref)
);

CREATE TABLE IF NOT EXISTS acceptance.observation_selector (
  selector_ref TEXT PRIMARY KEY,
  kind_ref TEXT NOT NULL CHECK (kind_ref IN ('candidate_pnf','wikidata_native')),
  batch_ref TEXT NULL,
  selected_predicate_candidate_ref TEXT NULL,
  diagnostic_ref TEXT NULL,
  native_statement_ref TEXT NULL,
  observation_ref TEXT NOT NULL,
  source_family_ref TEXT NULL,
  scope_ref TEXT NULL,
  time_ref TEXT NULL,
  modality_ref TEXT NULL,
  quantifier_ref TEXT NULL,
  attribution_ref TEXT NULL,
  ontology_ref TEXT NULL,
  language_ref TEXT NULL,
  provenance_refs TEXT[] NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  CHECK (
    (kind_ref='candidate_pnf' AND batch_ref IS NOT NULL
      AND selected_predicate_candidate_ref IS NOT NULL
      AND source_family_ref IS NOT NULL
      AND diagnostic_ref IS NULL AND native_statement_ref IS NULL)
    OR
    (kind_ref='wikidata_native' AND diagnostic_ref IS NOT NULL
      AND native_statement_ref IS NOT NULL
      AND batch_ref IS NULL AND selected_predicate_candidate_ref IS NULL)
  )
);

CREATE TABLE IF NOT EXISTS acceptance.rel_corpus (
  corpus_ref TEXT PRIMARY KEY,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_case (
  case_ref TEXT PRIMARY KEY,
  corpus_ref TEXT NOT NULL REFERENCES acceptance.rel_corpus(corpus_ref) ON DELETE CASCADE,
  case_ordinal INTEGER NOT NULL CHECK (case_ordinal >= 0),
  purpose_ref TEXT NOT NULL,
  left_selector_ref TEXT NOT NULL REFERENCES acceptance.observation_selector(selector_ref),
  right_selector_ref TEXT NOT NULL REFERENCES acceptance.observation_selector(selector_ref),
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref),
  expected_finding_ref TEXT NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  UNIQUE (corpus_ref, case_ordinal)
);
CREATE TABLE IF NOT EXISTS acceptance.rel_case_residual_contract (
  case_ref TEXT NOT NULL REFERENCES acceptance.rel_case(case_ref) ON DELETE CASCADE,
  disposition_ref TEXT NOT NULL CHECK (disposition_ref IN ('required','forbidden')),
  residual_kind_ref TEXT NOT NULL,
  PRIMARY KEY (case_ref, disposition_ref, residual_kind_ref)
);
"#;

/// INV acceptance tables.  These are installed only after the reviewed
/// evidence prerequisite is available, retaining the strict foreign key.
pub const INV_ACCEPTANCE_SCHEMA_SQL: &str = r#"
CREATE SCHEMA IF NOT EXISTS acceptance;
CREATE TABLE IF NOT EXISTS acceptance.inv_case (
  case_ref TEXT PRIMARY KEY,
  matter_ref TEXT NOT NULL,
  left_selector_ref TEXT NOT NULL REFERENCES acceptance.observation_selector(selector_ref),
  right_selector_ref TEXT NOT NULL REFERENCES acceptance.observation_selector(selector_ref),
  consumer_ref TEXT NOT NULL REFERENCES acceptance.rel_consumer(consumer_ref),
  residual_ordinal INTEGER NOT NULL CHECK (residual_ordinal >= 0),
  target_description TEXT NOT NULL,
  authority_or_access_constraint_ref TEXT NOT NULL,
  dependency_target_refs TEXT[] NOT NULL,
  reviewed_evidence_ref TEXT NOT NULL REFERENCES semantic.reviewed_evidence_coordinate(reviewed_evidence_ref),
  graph_binding_ref TEXT NOT NULL,
  graph_binding_evidence_refs TEXT[] NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted),
  creates_access_authority BOOLEAN NOT NULL CHECK (NOT creates_access_authority)
);
CREATE TABLE IF NOT EXISTS acceptance.inv_route (
  case_ref TEXT NOT NULL REFERENCES acceptance.inv_case(case_ref) ON DELETE CASCADE,
  route_ordinal INTEGER NOT NULL CHECK (route_ordinal >= 0),
  route_ref TEXT NOT NULL,
  route_description TEXT NOT NULL,
  source_locator_ref TEXT NOT NULL,
  access_disposition_ref TEXT NOT NULL,
  authority_receipt_ref TEXT NULL,
  provenance_genealogy_ref TEXT NOT NULL,
  independence_ref TEXT NOT NULL,
  independence_receipt_ref TEXT NULL,
  duplicate_relation_ref TEXT NOT NULL,
  information_gain BIGINT NOT NULL CHECK (information_gain >= 0),
  dependency_closure_impact BIGINT NOT NULL CHECK (dependency_closure_impact >= 0),
  residual_coverage BIGINT NOT NULL CHECK (residual_coverage >= 0),
  provenance_novelty BIGINT NOT NULL CHECK (provenance_novelty >= 0),
  acquisition_cost BIGINT NOT NULL CHECK (acquisition_cost >= 0),
  axis_estimation_receipt_ref TEXT NOT NULL,
  candidate_only BOOLEAN NOT NULL CHECK (candidate_only),
  creates_acquisition_authority BOOLEAN NOT NULL CHECK (NOT creates_acquisition_authority),
  PRIMARY KEY (case_ref, route_ordinal),
  UNIQUE (case_ref, route_ref)
);
CREATE TABLE IF NOT EXISTS acceptance.inv_governance (
  case_ref TEXT PRIMARY KEY REFERENCES acceptance.inv_case(case_ref) ON DELETE CASCADE,
  packet_ref TEXT NOT NULL,
  purpose_ref TEXT NOT NULL,
  access_state_ref TEXT NOT NULL,
  privacy_state_ref TEXT NOT NULL,
  ai_use_state_ref TEXT NOT NULL,
  service_change_state TEXT NOT NULL,
  evidence_state TEXT NOT NULL,
  control_refs TEXT[] NOT NULL,
  evidence_refs TEXT[] NOT NULL,
  security_refs TEXT[] NOT NULL,
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  creates_access_authority BOOLEAN NOT NULL CHECK (NOT creates_access_authority)
);

CREATE TABLE IF NOT EXISTS acceptance.inv_reopen_check (
  reopen_check_ref TEXT PRIMARY KEY,
  case_ref TEXT NOT NULL REFERENCES acceptance.inv_case(case_ref),
  acquisition_obligation_ref TEXT NOT NULL,
  before_availability_ref TEXT NOT NULL,
  after_availability_ref TEXT NOT NULL,
  acquired_source_revision_ref TEXT NULL,
  acquisition_receipt_ref TEXT NOT NULL,
  old_comparison_ref TEXT NOT NULL,
  replace_side_ref TEXT NOT NULL CHECK (replace_side_ref IN ('left','right')),
  new_selector_ref TEXT NULL REFERENCES acceptance.observation_selector(selector_ref),
  acceptance_only BOOLEAN NOT NULL CHECK (acceptance_only),
  creates_semantic_authority BOOLEAN NOT NULL CHECK (NOT creates_semantic_authority),
  claim_truth_promoted BOOLEAN NOT NULL CHECK (NOT claim_truth_promoted)
);
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcceptanceObservationKind {
    CandidatePnf,
    WikidataNative,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcceptanceObservationSelector {
    pub selector_ref: String,
    pub kind: AcceptanceObservationKind,
    pub batch_ref: Option<String>,
    pub selected_predicate_candidate_ref: Option<String>,
    pub diagnostic_ref: Option<String>,
    pub native_statement_ref: Option<String>,
    pub observation_ref: String,
    pub source_family: Option<RelationalSourceFamily>,
    pub context: ObservationContext,
    pub provenance_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelAcceptanceCase {
    pub case_ref: String,
    pub purpose_ref: String,
    pub left: AcceptanceObservationSelector,
    pub right: AcceptanceObservationSelector,
    pub consumer: RelationalConsumer,
    pub expected_finding: ComparisonFinding,
    pub required_residual_kinds: Vec<ResidualKind>,
    pub forbidden_residual_kinds: Vec<ResidualKind>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelAcceptanceCorpus {
    pub corpus_ref: String,
    pub cases: Vec<RelAcceptanceCase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvGovernanceControl {
    pub packet_ref: String,
    pub purpose_ref: String,
    pub access_state: GovernanceAccessState,
    pub privacy_state: PrivacyExposureState,
    pub ai_use_state: AiUseState,
    pub service_change_state: String,
    pub evidence_state: String,
    pub control_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub security_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvAcceptanceCase {
    pub case_ref: String,
    pub matter_ref: String,
    pub left: AcceptanceObservationSelector,
    pub right: AcceptanceObservationSelector,
    pub consumer: RelationalConsumer,
    pub residual_ordinal: usize,
    pub target_description: String,
    pub authority_or_access_constraint_ref: String,
    pub dependency_target_refs: Vec<String>,
    /// Routes carry a placeholder obligation ref on persistence/load.  The
    /// runner must replace it with the obligation derived from the selected
    /// persisted REL residual before validation/execution.
    pub routes: Vec<AcquisitionRouteCandidate>,
    pub reviewed_evidence_ref: String,
    pub governance: InvGovernanceControl,
    pub graph_binding_ref: String,
    pub graph_binding_evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvReopenCheck {
    pub reopen_check_ref: String,
    pub case_ref: String,
    pub acquisition_obligation_ref: String,
    pub before_availability_ref: String,
    pub after_availability_ref: String,
    pub acquired_source_revision_ref: Option<String>,
    pub acquisition_receipt_ref: String,
    pub old_comparison_ref: String,
    pub replace_side_ref: String,
    pub new_selector: Option<AcceptanceObservationSelector>,
}

#[derive(Debug, Error)]
pub enum AcceptanceControlStoreError {
    #[error("postgres error: {0}")]
    Postgres(#[from] postgres::Error),
    #[error("acceptance-control prerequisite failed: {0}")]
    Upstream(String),
    #[error("acceptance control contains an empty or invalid coordinate: {0}")]
    InvalidCoordinate(&'static str),
    #[error("unknown persisted enum coordinate {kind}: {value}")]
    UnknownEnum { kind: &'static str, value: String },
    #[error("persisted acceptance object not found: {0}")]
    NotFound(String),
    #[error("persisted acceptance row conflicts with immutable requested coordinates")]
    ExistingRowConflict,
}

/// Installs the REL/common acceptance surface only.
pub fn install_rel_acceptance_schema(
    config: &DatabaseConfig,
) -> Result<(), AcceptanceControlStoreError> {
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(REL_ACCEPTANCE_SCHEMA_SQL)?;
    Ok(())
}

/// Installs the INV acceptance surface and its reviewed-evidence prerequisite.
///
/// The reviewed-evidence foreign key remains mandatory; this function owns the
/// required installation order so callers cannot accidentally create a weak
/// INV-only schema.
pub fn install_inv_acceptance_schema(
    config: &DatabaseConfig,
) -> Result<(), AcceptanceControlStoreError> {
    crate::install_reviewed_evidence_schema(config)
        .map_err(|error| AcceptanceControlStoreError::Upstream(error.to_string()))?;
    install_rel_acceptance_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    client.batch_execute(INV_ACCEPTANCE_SCHEMA_SQL)?;
    Ok(())
}

/// Compatibility alias for callers that intentionally require the full INV
/// acceptance surface.  REL-only callers must use `install_rel_acceptance_schema`.
pub fn install_acceptance_control_schema(
    config: &DatabaseConfig,
) -> Result<(), AcceptanceControlStoreError> {
    install_inv_acceptance_schema(config)
}

pub fn persist_rel_acceptance_corpus(
    config: &DatabaseConfig,
    corpus: &RelAcceptanceCorpus,
) -> Result<(), AcceptanceControlStoreError> {
    validate_nonempty("corpus_ref", &corpus.corpus_ref)?;
    if corpus.cases.is_empty() {
        return Err(AcceptanceControlStoreError::InvalidCoordinate("cases"));
    }
    install_rel_acceptance_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    tx.execute(
        "INSERT INTO acceptance.rel_corpus (corpus_ref,acceptance_only,creates_semantic_authority,claim_truth_promoted) VALUES ($1,true,false,false) ON CONFLICT (corpus_ref) DO NOTHING",
        &[&corpus.corpus_ref],
    )?;
    for (ordinal, case) in corpus.cases.iter().enumerate() {
        persist_consumer(&mut tx, &case.consumer)?;
        persist_selector(&mut tx, &case.left)?;
        persist_selector(&mut tx, &case.right)?;
        validate_nonempty("case_ref", &case.case_ref)?;
        validate_nonempty("purpose_ref", &case.purpose_ref)?;
        tx.execute(
            r#"INSERT INTO acceptance.rel_case
               (case_ref,corpus_ref,case_ordinal,purpose_ref,left_selector_ref,right_selector_ref,
                consumer_ref,expected_finding_ref,acceptance_only,creates_semantic_authority,claim_truth_promoted)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,false,false)
               ON CONFLICT (case_ref) DO NOTHING"#,
            &[&case.case_ref,&corpus.corpus_ref,&(ordinal as i32),&case.purpose_ref,
              &case.left.selector_ref,&case.right.selector_ref,&case.consumer.consumer_ref,
              &finding_db(&case.expected_finding)],
        )?;
        for kind in &case.required_residual_kinds {
            tx.execute(
                "INSERT INTO acceptance.rel_case_residual_contract (case_ref,disposition_ref,residual_kind_ref) VALUES ($1,'required',$2) ON CONFLICT DO NOTHING",
                &[&case.case_ref, &residual_db(kind)],
            )?;
        }
        for kind in &case.forbidden_residual_kinds {
            tx.execute(
                "INSERT INTO acceptance.rel_case_residual_contract (case_ref,disposition_ref,residual_kind_ref) VALUES ($1,'forbidden',$2) ON CONFLICT DO NOTHING",
                &[&case.case_ref, &residual_db(kind)],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

pub fn load_rel_acceptance_corpus(
    config: &DatabaseConfig,
    corpus_ref: &str,
) -> Result<RelAcceptanceCorpus, AcceptanceControlStoreError> {
    validate_nonempty("corpus_ref", corpus_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let exists = client.query_opt(
        "SELECT corpus_ref,acceptance_only,creates_semantic_authority,claim_truth_promoted FROM acceptance.rel_corpus WHERE corpus_ref=$1",
        &[&corpus_ref],
    )?.ok_or_else(|| AcceptanceControlStoreError::NotFound(corpus_ref.to_owned()))?;
    if !exists.get::<_, bool>(1) || exists.get::<_, bool>(2) || exists.get::<_, bool>(3) {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    let rows = client.query(
        "SELECT case_ref,purpose_ref,left_selector_ref,right_selector_ref,consumer_ref,expected_finding_ref FROM acceptance.rel_case WHERE corpus_ref=$1 AND acceptance_only AND NOT creates_semantic_authority AND NOT claim_truth_promoted ORDER BY case_ordinal,case_ref",
        &[&corpus_ref],
    )?;
    let mut cases = Vec::with_capacity(rows.len());
    for row in rows {
        let case_ref: String = row.get(0);
        let contracts=client.query(
            "SELECT disposition_ref,residual_kind_ref FROM acceptance.rel_case_residual_contract WHERE case_ref=$1 ORDER BY disposition_ref,residual_kind_ref",
            &[&case_ref],
        )?;
        let mut required = Vec::new();
        let mut forbidden = Vec::new();
        for contract in contracts {
            let kind = residual_from_db(contract.get::<_, String>(1).as_str())?;
            match contract.get::<_, String>(0).as_str() {
                "required" => required.push(kind),
                "forbidden" => forbidden.push(kind),
                _ => return Err(AcceptanceControlStoreError::ExistingRowConflict),
            }
        }
        cases.push(RelAcceptanceCase {
            case_ref,
            purpose_ref: row.get(1),
            left: load_selector(&mut client, &row.get::<_, String>(2))?,
            right: load_selector(&mut client, &row.get::<_, String>(3))?,
            consumer: load_consumer(&mut client, &row.get::<_, String>(4))?,
            expected_finding: finding_from_db(row.get::<_, String>(5).as_str())?,
            required_residual_kinds: required,
            forbidden_residual_kinds: forbidden,
        });
    }
    Ok(RelAcceptanceCorpus {
        corpus_ref: corpus_ref.to_owned(),
        cases,
    })
}

pub fn persist_inv_acceptance_case(
    config: &DatabaseConfig,
    case: &InvAcceptanceCase,
) -> Result<(), AcceptanceControlStoreError> {
    validate_inv_case(case)?;
    install_inv_acceptance_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    persist_consumer(&mut tx, &case.consumer)?;
    persist_selector(&mut tx, &case.left)?;
    persist_selector(&mut tx, &case.right)?;
    tx.execute(
        r#"INSERT INTO acceptance.inv_case
           (case_ref,matter_ref,left_selector_ref,right_selector_ref,consumer_ref,residual_ordinal,
            target_description,authority_or_access_constraint_ref,dependency_target_refs,
            reviewed_evidence_ref,graph_binding_ref,graph_binding_evidence_refs,
            acceptance_only,creates_semantic_authority,claim_truth_promoted,creates_access_authority)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,true,false,false,false)
           ON CONFLICT (case_ref) DO NOTHING"#,
        &[&case.case_ref,&case.matter_ref,&case.left.selector_ref,&case.right.selector_ref,
          &case.consumer.consumer_ref,&(case.residual_ordinal as i32),&case.target_description,
          &case.authority_or_access_constraint_ref,&case.dependency_target_refs,
          &case.reviewed_evidence_ref,&case.graph_binding_ref,&case.graph_binding_evidence_refs],
    )?;
    for (ordinal, route) in case.routes.iter().enumerate() {
        tx.execute(
            r#"INSERT INTO acceptance.inv_route
               (case_ref,route_ordinal,route_ref,route_description,source_locator_ref,
                access_disposition_ref,authority_receipt_ref,provenance_genealogy_ref,
                independence_ref,independence_receipt_ref,duplicate_relation_ref,
                information_gain,dependency_closure_impact,residual_coverage,provenance_novelty,
                acquisition_cost,axis_estimation_receipt_ref,candidate_only,creates_acquisition_authority)
               VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,true,false)
               ON CONFLICT (case_ref,route_ordinal) DO NOTHING"#,
            &[&case.case_ref,&(ordinal as i32),&route.route_ref,&route.route_description,
              &route.source_locator_ref,&access_db(route.access_disposition),&route.authority_receipt_ref,
              &route.provenance_genealogy_ref,&independence_db(route.independence),
              &route.independence_receipt_ref,&duplicate_db(route.duplicate_relation),
              &(i64::from(route.information_gain)),&(i64::from(route.dependency_closure_impact)),
              &(i64::from(route.residual_coverage)),&(i64::from(route.provenance_novelty)),
              &(i64::from(route.acquisition_cost)),&route.axis_estimation_receipt_ref],
        )?;
    }
    let g = &case.governance;
    tx.execute(
        r#"INSERT INTO acceptance.inv_governance
           (case_ref,packet_ref,purpose_ref,access_state_ref,privacy_state_ref,ai_use_state_ref,
            service_change_state,evidence_state,control_refs,evidence_refs,security_refs,
            acceptance_only,creates_semantic_authority,creates_access_authority)
           VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,true,false,false)
           ON CONFLICT (case_ref) DO NOTHING"#,
        &[
            &case.case_ref,
            &g.packet_ref,
            &g.purpose_ref,
            &gov_access_db(g.access_state),
            &privacy_db(g.privacy_state),
            &ai_db(g.ai_use_state),
            &g.service_change_state,
            &g.evidence_state,
            &g.control_refs,
            &g.evidence_refs,
            &g.security_refs,
        ],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn load_inv_acceptance_case(
    config: &DatabaseConfig,
    case_ref: &str,
) -> Result<InvAcceptanceCase, AcceptanceControlStoreError> {
    validate_nonempty("case_ref", case_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row = client
        .query_opt(
            r#"SELECT matter_ref,left_selector_ref,right_selector_ref,consumer_ref,residual_ordinal,
           target_description,authority_or_access_constraint_ref,dependency_target_refs,
           reviewed_evidence_ref,graph_binding_ref,graph_binding_evidence_refs,
           acceptance_only,creates_semantic_authority,claim_truth_promoted,creates_access_authority
           FROM acceptance.inv_case WHERE case_ref=$1"#,
            &[&case_ref],
        )?
        .ok_or_else(|| AcceptanceControlStoreError::NotFound(case_ref.to_owned()))?;
    if !row.get::<_, bool>(11)
        || row.get::<_, bool>(12)
        || row.get::<_, bool>(13)
        || row.get::<_, bool>(14)
    {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    let route_rows = client.query(
        r#"SELECT route_ref,route_description,source_locator_ref,access_disposition_ref,
           authority_receipt_ref,provenance_genealogy_ref,independence_ref,
           independence_receipt_ref,duplicate_relation_ref,information_gain,
           dependency_closure_impact,residual_coverage,provenance_novelty,acquisition_cost,
           axis_estimation_receipt_ref,candidate_only,creates_acquisition_authority
           FROM acceptance.inv_route WHERE case_ref=$1 ORDER BY route_ordinal,route_ref"#,
        &[&case_ref],
    )?;
    let mut routes = Vec::with_capacity(route_rows.len());
    for r in route_rows {
        if !r.get::<_, bool>(15) || r.get::<_, bool>(16) {
            return Err(AcceptanceControlStoreError::ExistingRowConflict);
        }
        routes.push(AcquisitionRouteCandidate {
            route_ref: r.get(0),
            obligation_ref: "acceptance:derive-from-selected-residual".into(),
            route_description: r.get(1),
            source_locator_ref: r.get(2),
            access_disposition: access_from_db(r.get::<_, String>(3).as_str())?,
            authority_receipt_ref: r.get(4),
            provenance_genealogy_ref: r.get(5),
            independence: independence_from_db(r.get::<_, String>(6).as_str())?,
            independence_receipt_ref: r.get(7),
            duplicate_relation: duplicate_from_db(r.get::<_, String>(8).as_str())?,
            information_gain: u32_checked(r.get::<_, i64>(9))?,
            dependency_closure_impact: u32_checked(r.get::<_, i64>(10))?,
            residual_coverage: u32_checked(r.get::<_, i64>(11))?,
            provenance_novelty: u32_checked(r.get::<_, i64>(12))?,
            acquisition_cost: u32_checked(r.get::<_, i64>(13))?,
            axis_estimation_receipt_ref: r.get(14),
            candidate_only: true,
            creates_acquisition_authority: false,
        });
    }
    let g = client
        .query_opt(
            r#"SELECT packet_ref,purpose_ref,access_state_ref,privacy_state_ref,ai_use_state_ref,
           service_change_state,evidence_state,control_refs,evidence_refs,security_refs,
           acceptance_only,creates_semantic_authority,creates_access_authority
           FROM acceptance.inv_governance WHERE case_ref=$1"#,
            &[&case_ref],
        )?
        .ok_or_else(|| AcceptanceControlStoreError::NotFound(format!("governance:{case_ref}")))?;
    if !g.get::<_, bool>(10) || g.get::<_, bool>(11) || g.get::<_, bool>(12) {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    Ok(InvAcceptanceCase {
        case_ref: case_ref.to_owned(),
        matter_ref: row.get(0),
        left: load_selector(&mut client, &row.get::<_, String>(1))?,
        right: load_selector(&mut client, &row.get::<_, String>(2))?,
        consumer: load_consumer(&mut client, &row.get::<_, String>(3))?,
        residual_ordinal: usize::try_from(row.get::<_, i32>(4))
            .map_err(|_| AcceptanceControlStoreError::ExistingRowConflict)?,
        target_description: row.get(5),
        authority_or_access_constraint_ref: row.get(6),
        dependency_target_refs: row.get(7),
        reviewed_evidence_ref: row.get(8),
        graph_binding_ref: row.get(9),
        graph_binding_evidence_refs: row.get(10),
        routes,
        governance: InvGovernanceControl {
            packet_ref: g.get(0),
            purpose_ref: g.get(1),
            access_state: gov_access_from_db(g.get::<_, String>(2).as_str())?,
            privacy_state: privacy_from_db(g.get::<_, String>(3).as_str())?,
            ai_use_state: ai_from_db(g.get::<_, String>(4).as_str())?,
            service_change_state: g.get(5),
            evidence_state: g.get(6),
            control_refs: g.get(7),
            evidence_refs: g.get(8),
            security_refs: g.get(9),
        },
    })
}

pub fn persist_inv_reopen_check(
    config: &DatabaseConfig,
    check: &InvReopenCheck,
) -> Result<(), AcceptanceControlStoreError> {
    for (name, value) in [
        ("reopen_check_ref", check.reopen_check_ref.as_str()),
        ("case_ref", check.case_ref.as_str()),
        (
            "acquisition_obligation_ref",
            check.acquisition_obligation_ref.as_str(),
        ),
        (
            "before_availability_ref",
            check.before_availability_ref.as_str(),
        ),
        (
            "after_availability_ref",
            check.after_availability_ref.as_str(),
        ),
        (
            "acquisition_receipt_ref",
            check.acquisition_receipt_ref.as_str(),
        ),
        ("old_comparison_ref", check.old_comparison_ref.as_str()),
        ("replace_side_ref", check.replace_side_ref.as_str()),
    ] {
        validate_nonempty(name, value)?;
    }
    if !matches!(check.replace_side_ref.as_str(), "left" | "right") {
        return Err(AcceptanceControlStoreError::InvalidCoordinate(
            "replace_side_ref",
        ));
    }
    install_inv_acceptance_schema(config)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let mut tx = client.transaction()?;
    if let Some(selector) = &check.new_selector {
        persist_selector(&mut tx, selector)?;
    }
    tx.execute(r#"INSERT INTO acceptance.inv_reopen_check
        (reopen_check_ref,case_ref,acquisition_obligation_ref,before_availability_ref,after_availability_ref,
         acquired_source_revision_ref,acquisition_receipt_ref,old_comparison_ref,replace_side_ref,new_selector_ref,
         acceptance_only,creates_semantic_authority,claim_truth_promoted)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,true,false,false) ON CONFLICT (reopen_check_ref) DO NOTHING"#,
        &[&check.reopen_check_ref,&check.case_ref,&check.acquisition_obligation_ref,&check.before_availability_ref,&check.after_availability_ref,
          &check.acquired_source_revision_ref,&check.acquisition_receipt_ref,&check.old_comparison_ref,&check.replace_side_ref,
          &check.new_selector.as_ref().map(|s|s.selector_ref.clone())])?;
    tx.commit()?;
    Ok(())
}

pub fn load_inv_reopen_check(
    config: &DatabaseConfig,
    reopen_check_ref: &str,
) -> Result<InvReopenCheck, AcceptanceControlStoreError> {
    validate_nonempty("reopen_check_ref", reopen_check_ref)?;
    let mut client = Client::connect(config.database_url(), NoTls)?;
    let row=client.query_opt(r#"SELECT case_ref,acquisition_obligation_ref,before_availability_ref,after_availability_ref,
        acquired_source_revision_ref,acquisition_receipt_ref,old_comparison_ref,replace_side_ref,new_selector_ref,
        acceptance_only,creates_semantic_authority,claim_truth_promoted FROM acceptance.inv_reopen_check WHERE reopen_check_ref=$1"#,&[&reopen_check_ref])?
        .ok_or_else(||AcceptanceControlStoreError::NotFound(reopen_check_ref.to_owned()))?;
    if !row.get::<_, bool>(9) || row.get::<_, bool>(10) || row.get::<_, bool>(11) {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    let selector_ref: Option<String> = row.get(8);
    Ok(InvReopenCheck {
        reopen_check_ref: reopen_check_ref.to_owned(),
        case_ref: row.get(0),
        acquisition_obligation_ref: row.get(1),
        before_availability_ref: row.get(2),
        after_availability_ref: row.get(3),
        acquired_source_revision_ref: row.get(4),
        acquisition_receipt_ref: row.get(5),
        old_comparison_ref: row.get(6),
        replace_side_ref: row.get(7),
        new_selector: match selector_ref {
            Some(v) => Some(load_selector(&mut client, &v)?),
            None => None,
        },
    })
}

fn validate_inv_case(case: &InvAcceptanceCase) -> Result<(), AcceptanceControlStoreError> {
    for (name, value) in [
        ("case_ref", case.case_ref.as_str()),
        ("matter_ref", case.matter_ref.as_str()),
        ("target_description", case.target_description.as_str()),
        (
            "authority_or_access_constraint_ref",
            case.authority_or_access_constraint_ref.as_str(),
        ),
        ("reviewed_evidence_ref", case.reviewed_evidence_ref.as_str()),
        ("graph_binding_ref", case.graph_binding_ref.as_str()),
        ("governance.packet_ref", case.governance.packet_ref.as_str()),
        (
            "governance.purpose_ref",
            case.governance.purpose_ref.as_str(),
        ),
    ] {
        validate_nonempty(name, value)?;
    }
    if case.routes.len() < 2
        || case.graph_binding_evidence_refs.is_empty()
        || case.governance.control_refs.is_empty()
        || case.governance.evidence_refs.is_empty()
        || case.governance.security_refs.is_empty()
    {
        return Err(AcceptanceControlStoreError::InvalidCoordinate(
            "inv_case_required_collections",
        ));
    }
    Ok(())
}

fn persist_selector<C: GenericClient>(
    client: &mut C,
    s: &AcceptanceObservationSelector,
) -> Result<(), AcceptanceControlStoreError> {
    validate_nonempty("selector_ref", &s.selector_ref)?;
    validate_nonempty("observation_ref", &s.observation_ref)?;
    let (kind, batch, predicate, diagnostic, native, family) = match s.kind {
        AcceptanceObservationKind::CandidatePnf => {
            let b = s
                .batch_ref
                .as_ref()
                .filter(|v| !v.trim().is_empty())
                .ok_or(AcceptanceControlStoreError::InvalidCoordinate("batch_ref"))?;
            let p = s
                .selected_predicate_candidate_ref
                .as_ref()
                .filter(|v| !v.trim().is_empty())
                .ok_or(AcceptanceControlStoreError::InvalidCoordinate(
                    "selected_predicate_candidate_ref",
                ))?;
            let f = s
                .source_family
                .ok_or(AcceptanceControlStoreError::InvalidCoordinate(
                    "source_family",
                ))?;
            (
                "candidate_pnf",
                Some(b.clone()),
                Some(p.clone()),
                None,
                None,
                Some(source_family_db(f).to_owned()),
            )
        }
        AcceptanceObservationKind::WikidataNative => {
            let d = s
                .diagnostic_ref
                .as_ref()
                .filter(|v| !v.trim().is_empty())
                .ok_or(AcceptanceControlStoreError::InvalidCoordinate(
                    "diagnostic_ref",
                ))?;
            let n = s
                .native_statement_ref
                .as_ref()
                .filter(|v| !v.trim().is_empty())
                .ok_or(AcceptanceControlStoreError::InvalidCoordinate(
                    "native_statement_ref",
                ))?;
            (
                "wikidata_native",
                None,
                None,
                Some(d.clone()),
                Some(n.clone()),
                None,
            )
        }
    };
    client.execute(r#"INSERT INTO acceptance.observation_selector
      (selector_ref,kind_ref,batch_ref,selected_predicate_candidate_ref,diagnostic_ref,native_statement_ref,
       observation_ref,source_family_ref,scope_ref,time_ref,modality_ref,quantifier_ref,attribution_ref,ontology_ref,language_ref,
       provenance_refs,acceptance_only,creates_semantic_authority,claim_truth_promoted)
      VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,true,false,false)
      ON CONFLICT (selector_ref) DO NOTHING"#,
      &[&s.selector_ref,&kind,&batch,&predicate,&diagnostic,&native,&s.observation_ref,&family,
        &s.context.scope_ref,&s.context.time_ref,&s.context.modality_ref,&s.context.quantifier_ref,&s.context.attribution_ref,
        &s.context.ontology_ref,&s.context.language_ref,&s.provenance_refs])?;
    Ok(())
}

fn load_selector<C: GenericClient>(
    client: &mut C,
    selector_ref: &str,
) -> Result<AcceptanceObservationSelector, AcceptanceControlStoreError> {
    let r=client.query_opt(r#"SELECT kind_ref,batch_ref,selected_predicate_candidate_ref,diagnostic_ref,native_statement_ref,
      observation_ref,source_family_ref,scope_ref,time_ref,modality_ref,quantifier_ref,attribution_ref,ontology_ref,language_ref,
      provenance_refs,acceptance_only,creates_semantic_authority,claim_truth_promoted FROM acceptance.observation_selector WHERE selector_ref=$1"#,&[&selector_ref])?
      .ok_or_else(||AcceptanceControlStoreError::NotFound(selector_ref.to_owned()))?;
    if !r.get::<_, bool>(15) || r.get::<_, bool>(16) || r.get::<_, bool>(17) {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    let kind_s: String = r.get(0);
    let family_s: Option<String> = r.get(6);
    let kind = match kind_s.as_str() {
        "candidate_pnf" => AcceptanceObservationKind::CandidatePnf,
        "wikidata_native" => AcceptanceObservationKind::WikidataNative,
        _ => {
            return Err(AcceptanceControlStoreError::UnknownEnum {
                kind: "observation_selector_kind",
                value: kind_s,
            })
        }
    };
    Ok(AcceptanceObservationSelector {
        selector_ref: selector_ref.to_owned(),
        kind,
        batch_ref: r.get(1),
        selected_predicate_candidate_ref: r.get(2),
        diagnostic_ref: r.get(3),
        native_statement_ref: r.get(4),
        observation_ref: r.get(5),
        source_family: match family_s {
            Some(v) => Some(source_family_from_db(&v)?),
            None => None,
        },
        context: ObservationContext {
            scope_ref: r.get(7),
            time_ref: r.get(8),
            modality_ref: r.get(9),
            quantifier_ref: r.get(10),
            attribution_ref: r.get(11),
            ontology_ref: r.get(12),
            language_ref: r.get(13),
        },
        provenance_refs: r.get(14),
    })
}

fn persist_consumer<C: GenericClient>(
    client: &mut C,
    c: &RelationalConsumer,
) -> Result<(), AcceptanceControlStoreError> {
    validate_nonempty("consumer_ref", &c.consumer_ref)?;
    client.execute(r#"INSERT INTO acceptance.rel_consumer
      (consumer_ref,compare_scope,compare_time,compare_modality,compare_quantifier,compare_attribution,compare_ontology,compare_language,
       acceptance_only,creates_semantic_authority,claim_truth_promoted)
      VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true,false,false) ON CONFLICT (consumer_ref) DO NOTHING"#,
      &[&c.consumer_ref,&c.compare_scope,&c.compare_time,&c.compare_modality,&c.compare_quantifier,&c.compare_attribution,&c.compare_ontology,&c.compare_language])?;
    for f in &c.permitted_source_families {
        client.execute("INSERT INTO acceptance.rel_consumer_source_family (consumer_ref,family_ref) VALUES ($1,$2) ON CONFLICT DO NOTHING",&[&c.consumer_ref,&source_family_db(*f)])?;
    }
    for role in &c.required_roles {
        client.execute("INSERT INTO acceptance.rel_consumer_required_role (consumer_ref,role_ref) VALUES ($1,$2) ON CONFLICT DO NOTHING",&[&c.consumer_ref,role])?;
    }
    for d in &c.required_role_types {
        client.execute("INSERT INTO acceptance.rel_consumer_role_type_demand (consumer_ref,role_ref,required_type_ref,contract_ref) VALUES ($1,$2,$3,$4) ON CONFLICT DO NOTHING",&[&c.consumer_ref,&d.role_ref,&d.required_type_ref,&d.contract_ref])?;
    }
    for (axis, items) in [
        ("predicate", &c.predicate_alignments),
        ("role", &c.role_alignments),
        ("filler", &c.filler_alignments),
        ("type", &c.type_alignments),
    ] {
        for a in items {
            if a.consumer_ref != c.consumer_ref {
                return Err(AcceptanceControlStoreError::InvalidCoordinate(
                    "alignment.consumer_ref",
                ));
            }
            client.execute(r#"INSERT INTO acceptance.rel_consumer_alignment
              (consumer_ref,axis_ref,left_ref,right_ref,witness_ref,licence_ref,direction_ref) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT DO NOTHING"#,
              &[&c.consumer_ref,&axis,&a.left_ref,&a.right_ref,&a.witness_ref,&a.licence_ref,&alignment_direction_db(a.direction)])?;
        }
    }
    Ok(())
}

fn load_consumer<C: GenericClient>(
    client: &mut C,
    consumer_ref: &str,
) -> Result<RelationalConsumer, AcceptanceControlStoreError> {
    let r=client.query_opt(r#"SELECT compare_scope,compare_time,compare_modality,compare_quantifier,compare_attribution,compare_ontology,compare_language,
       acceptance_only,creates_semantic_authority,claim_truth_promoted FROM acceptance.rel_consumer WHERE consumer_ref=$1"#,&[&consumer_ref])?
       .ok_or_else(||AcceptanceControlStoreError::NotFound(consumer_ref.to_owned()))?;
    if !r.get::<_, bool>(7) || r.get::<_, bool>(8) || r.get::<_, bool>(9) {
        return Err(AcceptanceControlStoreError::ExistingRowConflict);
    }
    let families=client.query("SELECT family_ref FROM acceptance.rel_consumer_source_family WHERE consumer_ref=$1 ORDER BY family_ref",&[&consumer_ref])?
        .into_iter().map(|x|source_family_from_db(x.get::<_,String>(0).as_str())).collect::<Result<Vec<_>,_>>()?;
    let roles=client.query("SELECT role_ref FROM acceptance.rel_consumer_required_role WHERE consumer_ref=$1 ORDER BY role_ref",&[&consumer_ref])?.into_iter().map(|x|x.get(0)).collect();
    let demands=client.query("SELECT role_ref,required_type_ref,contract_ref FROM acceptance.rel_consumer_role_type_demand WHERE consumer_ref=$1 ORDER BY role_ref,required_type_ref,contract_ref",&[&consumer_ref])?
        .into_iter().map(|x|RoleTypeDemand{role_ref:x.get(0),required_type_ref:x.get(1),contract_ref:x.get(2)}).collect();
    let mut predicate = Vec::new();
    let mut role = Vec::new();
    let mut filler = Vec::new();
    let mut type_a = Vec::new();
    for x in client.query("SELECT axis_ref,left_ref,right_ref,witness_ref,licence_ref,direction_ref FROM acceptance.rel_consumer_alignment WHERE consumer_ref=$1 ORDER BY axis_ref,left_ref,right_ref,witness_ref,licence_ref",&[&consumer_ref])?{
        let a=LicensedAlignment{left_ref:x.get(1),right_ref:x.get(2),witness_ref:x.get(3),consumer_ref:consumer_ref.to_owned(),licence_ref:x.get(4),direction:alignment_direction_from_db(x.get::<_,String>(5).as_str())?};
        match x.get::<_,String>(0).as_str(){"predicate"=>predicate.push(a),"role"=>role.push(a),"filler"=>filler.push(a),"type"=>type_a.push(a),_=>return Err(AcceptanceControlStoreError::ExistingRowConflict)}
    }
    Ok(RelationalConsumer {
        consumer_ref: consumer_ref.to_owned(),
        permitted_source_families: families,
        required_roles: roles,
        required_role_types: demands,
        compare_scope: r.get(0),
        compare_time: r.get(1),
        compare_modality: r.get(2),
        compare_quantifier: r.get(3),
        compare_attribution: r.get(4),
        compare_ontology: r.get(5),
        compare_language: r.get(6),
        predicate_alignments: predicate,
        role_alignments: role,
        filler_alignments: filler,
        type_alignments: type_a,
    })
}

fn validate_nonempty(name: &'static str, value: &str) -> Result<(), AcceptanceControlStoreError> {
    if value.trim().is_empty() {
        Err(AcceptanceControlStoreError::InvalidCoordinate(name))
    } else {
        Ok(())
    }
}
fn u32_checked(v: i64) -> Result<u32, AcceptanceControlStoreError> {
    u32::try_from(v).map_err(|_| AcceptanceControlStoreError::ExistingRowConflict)
}

fn source_family_db(v: RelationalSourceFamily) -> &'static str {
    match v {
        RelationalSourceFamily::Wikidata => "wikidata",
        RelationalSourceFamily::Wikipedia => "wikipedia",
        RelationalSourceFamily::Legal => "legal",
        RelationalSourceFamily::Biomedical => "biomedical",
        RelationalSourceFamily::Transcript => "transcript",
        RelationalSourceFamily::Chat => "chat",
        RelationalSourceFamily::StructuredKnowledge => "structured_knowledge",
        RelationalSourceFamily::AnimalObservation => "animal_observation",
        RelationalSourceFamily::Other => "other",
    }
}
fn source_family_from_db(v: &str) -> Result<RelationalSourceFamily, AcceptanceControlStoreError> {
    match v {
        "wikidata" => Ok(RelationalSourceFamily::Wikidata),
        "wikipedia" => Ok(RelationalSourceFamily::Wikipedia),
        "legal" => Ok(RelationalSourceFamily::Legal),
        "biomedical" => Ok(RelationalSourceFamily::Biomedical),
        "transcript" => Ok(RelationalSourceFamily::Transcript),
        "chat" => Ok(RelationalSourceFamily::Chat),
        "structured_knowledge" => Ok(RelationalSourceFamily::StructuredKnowledge),
        "animal_observation" => Ok(RelationalSourceFamily::AnimalObservation),
        "other" => Ok(RelationalSourceFamily::Other),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "source_family",
            value: v.into(),
        }),
    }
}
fn finding_db(v: &ComparisonFinding) -> &'static str {
    match v {
        ComparisonFinding::ExactCandidateShape => "exact_candidate_shape",
        ComparisonFinding::LicensedCompatible => "licensed_compatible",
        ComparisonFinding::CompatibleQualification => "compatible_qualification",
        ComparisonFinding::PolarityConflictCandidate => "polarity_conflict_candidate",
        ComparisonFinding::PartialResidual => "partial_residual",
        ComparisonFinding::MissingTypedMeet => "missing_typed_meet",
        ComparisonFinding::Undetermined => "undetermined",
    }
}
fn finding_from_db(v: &str) -> Result<ComparisonFinding, AcceptanceControlStoreError> {
    match v {
        "exact_candidate_shape" => Ok(ComparisonFinding::ExactCandidateShape),
        "licensed_compatible" => Ok(ComparisonFinding::LicensedCompatible),
        "compatible_qualification" => Ok(ComparisonFinding::CompatibleQualification),
        "polarity_conflict_candidate" => Ok(ComparisonFinding::PolarityConflictCandidate),
        "partial_residual" => Ok(ComparisonFinding::PartialResidual),
        "missing_typed_meet" => Ok(ComparisonFinding::MissingTypedMeet),
        "undetermined" => Ok(ComparisonFinding::Undetermined),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "comparison_finding",
            value: v.into(),
        }),
    }
}
fn residual_db(v: &ResidualKind) -> &'static str {
    match v {
        ResidualKind::UnalignedPredicate => "unaligned_predicate",
        ResidualKind::UnalignedRoleFiller => "unaligned_role_filler",
        ResidualKind::MissingRequiredRole => "missing_required_role",
        ResidualKind::UnalignedType => "unaligned_type",
        ResidualKind::ScopeMismatch => "scope_mismatch",
        ResidualKind::TimeMismatch => "time_mismatch",
        ResidualKind::ModalityMismatch => "modality_mismatch",
        ResidualKind::QuantifierMismatch => "quantifier_mismatch",
        ResidualKind::AttributionMismatch => "attribution_mismatch",
        ResidualKind::OntologyMismatch => "ontology_mismatch",
        ResidualKind::LanguageMismatch => "language_mismatch",
        ResidualKind::UnresolvedPolarity => "unresolved_polarity",
        ResidualKind::MissingProvenance => "missing_provenance",
        ResidualKind::MissingRoleTypeEvidence => "missing_role_type_evidence",
        ResidualKind::RoleTypeContractPressure => "role_type_contract_pressure",
    }
}
fn residual_from_db(v: &str) -> Result<ResidualKind, AcceptanceControlStoreError> {
    match v {
        "unaligned_predicate" => Ok(ResidualKind::UnalignedPredicate),
        "unaligned_role_filler" => Ok(ResidualKind::UnalignedRoleFiller),
        "missing_required_role" => Ok(ResidualKind::MissingRequiredRole),
        "unaligned_type" => Ok(ResidualKind::UnalignedType),
        "scope_mismatch" => Ok(ResidualKind::ScopeMismatch),
        "time_mismatch" => Ok(ResidualKind::TimeMismatch),
        "modality_mismatch" => Ok(ResidualKind::ModalityMismatch),
        "quantifier_mismatch" => Ok(ResidualKind::QuantifierMismatch),
        "attribution_mismatch" => Ok(ResidualKind::AttributionMismatch),
        "ontology_mismatch" => Ok(ResidualKind::OntologyMismatch),
        "language_mismatch" => Ok(ResidualKind::LanguageMismatch),
        "unresolved_polarity" => Ok(ResidualKind::UnresolvedPolarity),
        "missing_provenance" => Ok(ResidualKind::MissingProvenance),
        "missing_role_type_evidence" => Ok(ResidualKind::MissingRoleTypeEvidence),
        "role_type_contract_pressure" => Ok(ResidualKind::RoleTypeContractPressure),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "residual_kind",
            value: v.into(),
        }),
    }
}
fn alignment_direction_db(v: AlignmentDirection) -> &'static str {
    match v {
        AlignmentDirection::Symmetric => "symmetric",
        AlignmentDirection::LeftToRight => "left_to_right",
    }
}
fn alignment_direction_from_db(v: &str) -> Result<AlignmentDirection, AcceptanceControlStoreError> {
    match v {
        "symmetric" => Ok(AlignmentDirection::Symmetric),
        "left_to_right" => Ok(AlignmentDirection::LeftToRight),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "alignment_direction",
            value: v.into(),
        }),
    }
}
fn access_db(v: AccessDisposition) -> &'static str {
    match v {
        AccessDisposition::Public => "public",
        AccessDisposition::Authorized => "authorized",
        AccessDisposition::RequiresAuthorization => "requires_authorization",
        AccessDisposition::ProhibitedOrUnavailable => "prohibited_or_unavailable",
        AccessDisposition::Unknown => "unknown",
    }
}
fn access_from_db(v: &str) -> Result<AccessDisposition, AcceptanceControlStoreError> {
    match v {
        "public" => Ok(AccessDisposition::Public),
        "authorized" => Ok(AccessDisposition::Authorized),
        "requires_authorization" => Ok(AccessDisposition::RequiresAuthorization),
        "prohibited_or_unavailable" => Ok(AccessDisposition::ProhibitedOrUnavailable),
        "unknown" => Ok(AccessDisposition::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "access_disposition",
            value: v.into(),
        }),
    }
}
fn independence_db(v: EvidenceIndependence) -> &'static str {
    match v {
        EvidenceIndependence::KnownIndependent => "known_independent",
        EvidenceIndependence::KnownDependent => "known_dependent",
        EvidenceIndependence::Unknown => "unknown",
    }
}
fn independence_from_db(v: &str) -> Result<EvidenceIndependence, AcceptanceControlStoreError> {
    match v {
        "known_independent" => Ok(EvidenceIndependence::KnownIndependent),
        "known_dependent" => Ok(EvidenceIndependence::KnownDependent),
        "unknown" => Ok(EvidenceIndependence::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "evidence_independence",
            value: v.into(),
        }),
    }
}
fn duplicate_db(v: DuplicateRelation) -> &'static str {
    match v {
        DuplicateRelation::MetadataDuplicate => "metadata_duplicate",
        DuplicateRelation::PublicationDuplicate => "publication_duplicate",
        DuplicateRelation::ReportFamilyDuplicate => "report_family_duplicate",
        DuplicateRelation::SameEmpiricalStudy => "same_empirical_study",
        DuplicateRelation::DerivativeOfCommonSource => "derivative_of_common_source",
        DuplicateRelation::NoKnownDuplicateRelation => "no_known_duplicate_relation",
        DuplicateRelation::Unknown => "unknown",
    }
}
fn duplicate_from_db(v: &str) -> Result<DuplicateRelation, AcceptanceControlStoreError> {
    match v {
        "metadata_duplicate" => Ok(DuplicateRelation::MetadataDuplicate),
        "publication_duplicate" => Ok(DuplicateRelation::PublicationDuplicate),
        "report_family_duplicate" => Ok(DuplicateRelation::ReportFamilyDuplicate),
        "same_empirical_study" => Ok(DuplicateRelation::SameEmpiricalStudy),
        "derivative_of_common_source" => Ok(DuplicateRelation::DerivativeOfCommonSource),
        "no_known_duplicate_relation" => Ok(DuplicateRelation::NoKnownDuplicateRelation),
        "unknown" => Ok(DuplicateRelation::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "duplicate_relation",
            value: v.into(),
        }),
    }
}
fn gov_access_db(v: GovernanceAccessState) -> &'static str {
    match v {
        GovernanceAccessState::Public => "public",
        GovernanceAccessState::Authorized => "authorized",
        GovernanceAccessState::RequiresAuthorization => "requires_authorization",
        GovernanceAccessState::ProhibitedOrUnavailable => "prohibited_or_unavailable",
        GovernanceAccessState::Unknown => "unknown",
    }
}
fn gov_access_from_db(v: &str) -> Result<GovernanceAccessState, AcceptanceControlStoreError> {
    match v {
        "public" => Ok(GovernanceAccessState::Public),
        "authorized" => Ok(GovernanceAccessState::Authorized),
        "requires_authorization" => Ok(GovernanceAccessState::RequiresAuthorization),
        "prohibited_or_unavailable" => Ok(GovernanceAccessState::ProhibitedOrUnavailable),
        "unknown" => Ok(GovernanceAccessState::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "governance_access",
            value: v.into(),
        }),
    }
}
fn privacy_db(v: PrivacyExposureState) -> &'static str {
    match v {
        PrivacyExposureState::NoPii => "no_pii",
        PrivacyExposureState::ContainsPii => "contains_pii",
        PrivacyExposureState::Sensitive => "sensitive",
        PrivacyExposureState::Unknown => "unknown",
    }
}
fn privacy_from_db(v: &str) -> Result<PrivacyExposureState, AcceptanceControlStoreError> {
    match v {
        "no_pii" => Ok(PrivacyExposureState::NoPii),
        "contains_pii" => Ok(PrivacyExposureState::ContainsPii),
        "sensitive" => Ok(PrivacyExposureState::Sensitive),
        "unknown" => Ok(PrivacyExposureState::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "privacy_state",
            value: v.into(),
        }),
    }
}
fn ai_db(v: AiUseState) -> &'static str {
    match v {
        AiUseState::NotApplicable => "not_applicable",
        AiUseState::IntendedUseDeclared => "intended_use_declared",
        AiUseState::RiskReviewRequired => "risk_review_required",
        AiUseState::Unknown => "unknown",
    }
}
fn ai_from_db(v: &str) -> Result<AiUseState, AcceptanceControlStoreError> {
    match v {
        "not_applicable" => Ok(AiUseState::NotApplicable),
        "intended_use_declared" => Ok(AiUseState::IntendedUseDeclared),
        "risk_review_required" => Ok(AiUseState::RiskReviewRequired),
        "unknown" => Ok(AiUseState::Unknown),
        _ => Err(AcceptanceControlStoreError::UnknownEnum {
            kind: "ai_use_state",
            value: v.into(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rel_common_schema_has_no_inv_or_reviewed_evidence_dependency() {
        assert!(!REL_ACCEPTANCE_SCHEMA_SQL.contains("acceptance.inv_case"));
        assert!(!REL_ACCEPTANCE_SCHEMA_SQL.contains("semantic.reviewed_evidence_coordinate"));
        assert!(REL_ACCEPTANCE_SCHEMA_SQL.contains("acceptance.rel_corpus"));
        assert!(REL_ACCEPTANCE_SCHEMA_SQL.contains("acceptance.observation_selector"));
    }

    #[test]
    fn inv_schema_owns_the_strict_reviewed_evidence_foreign_key() {
        assert!(INV_ACCEPTANCE_SCHEMA_SQL.contains("acceptance.inv_case"));
        assert!(INV_ACCEPTANCE_SCHEMA_SQL
            .contains("REFERENCES semantic.reviewed_evidence_coordinate(reviewed_evidence_ref)"));
        assert!(!INV_ACCEPTANCE_SCHEMA_SQL.contains("acceptance.rel_corpus"));
    }

    #[test]
    fn enum_round_trips_are_total_for_current_rel_surface() {
        for value in [
            RelationalSourceFamily::Wikidata,
            RelationalSourceFamily::Wikipedia,
            RelationalSourceFamily::Legal,
            RelationalSourceFamily::Biomedical,
            RelationalSourceFamily::Transcript,
            RelationalSourceFamily::Chat,
            RelationalSourceFamily::StructuredKnowledge,
            RelationalSourceFamily::AnimalObservation,
            RelationalSourceFamily::Other,
        ] {
            assert_eq!(
                source_family_from_db(source_family_db(value)).unwrap(),
                value
            );
        }
        for value in [
            ComparisonFinding::ExactCandidateShape,
            ComparisonFinding::LicensedCompatible,
            ComparisonFinding::CompatibleQualification,
            ComparisonFinding::PolarityConflictCandidate,
            ComparisonFinding::PartialResidual,
            ComparisonFinding::MissingTypedMeet,
            ComparisonFinding::Undetermined,
        ] {
            assert_eq!(finding_from_db(finding_db(&value)).unwrap(), value);
        }
    }
}
