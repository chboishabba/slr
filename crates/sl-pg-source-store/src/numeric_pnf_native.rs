//! Native numeric PNF canonical reduction (non-admitting Rust kernel).
//!
//! This ports the *strict, canonical* SensibLaw factor-proposal reduction
//! contract, NOT the historical document-wide compatibility replay.
//! Semantic authority belongs to the original SensibLaw numeric-PNF runtime.
//!
//! Critical safety boundaries:
//! - source revision and exact canonical manifest are checked before reducing
//! - proposal identity uses ITIR typed-canonical v1 (not JSON bytes)
//! - canonically sorted proposal refs, four-component signature buckets
//! - greedy FIRST compatible group in stable proposal order
//! - global observation/dependency validation; missing inputs stay residual
//! - absent/unknown candidates and resource pruning NEVER imply refutation
//! - outputs are candidate/factor carriers, not accepted identity witnesses
//!
//! No database mutation, no identity/QID, no Level-3 factor substitution.
//! The original SQL identity admission, H9 occurrence, retraction and native
//! sink remain separate stages until reference parity has been established.

use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use thiserror::Error;

const CANONICAL_CONTRACT: &str = "itir.typed-canonical.v1";
const REDUCTION_CONTRACT: &str = "deterministic-fibrewise-pnf:v0_1";

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct NativeManifest {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_content_sha256: String,
    pub source_byte_length: usize,
    pub source_codepoint_length: usize,
    pub coordinate_contract_ref: String,
    pub parser_run_ref: String,
    pub rule_set_revision: String,
}

impl NativeManifest {
    pub fn validate(&self) -> Result<(), NumericPnfError> {
        if self.document_ref.is_empty()
            || self.source_revision_ref.is_empty()
            || self.parser_run_ref.is_empty()
            || self.rule_set_revision.is_empty()
            || self.coordinate_contract_ref.is_empty()
            || self.canonical_content_sha256.len() != 64
            || !self.canonical_content_sha256.bytes().all(|c| c.is_ascii_hexdigit())
        {
            return Err(NumericPnfError::InvalidManifest);
        }
        Ok(())
    }

    pub fn verify_source_bytes(&self, bytes: &[u8]) -> Result<(), NumericPnfError> {
        self.validate()?;
        let content = std::str::from_utf8(bytes)
            .map_err(|_| NumericPnfError::InvalidUtf8)?;
        let sha = format!("{:x}", Sha256::digest(bytes));
        if self.canonical_content_sha256 != sha
            || self.source_byte_length != bytes.len()
            || self.source_codepoint_length != content.chars().count()
        {
            return Err(NumericPnfError::SourceMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Error)]
pub enum NumericPnfError {
    #[error("invalid original source manifest")]
    InvalidManifest,
    #[error("source is not UTF-8")]
    InvalidUtf8,
    #[error("original source hash/length/coordinate mismatch")]
    SourceMismatch,
    #[error("factor proposal lacks canonical field: {0}")]
    MissingField(&'static str),
    #[error("factor proposal has invalid type for field: {0}")]
    InvalidField(&'static str),
    #[error("foreign document or source revision in proposal")]
    ForeignSource,
    #[error("proposal id does not match typed-canonical payload")]
    ForgedProposalRef,
    #[error("canonical integer range exceeds supported u64/i64")]
    UnsupportedCanonicalInteger,
    #[error("duplicate proposal ref has conflicting original evidence")]
    ConflictingProposal,
    #[error("unknown/unchecked semantic reduction mode")]
    InvalidMode,
    #[error("canonical owner would exceed bounded semantic comparison budget")]
    SemanticWorkBudget,
}

fn prefixed_len(buf: &mut Vec<u8>, n: usize) {
    buf.extend_from_slice(&(n as u64).to_be_bytes());
}

fn encode_signed(buf: &mut Vec<u8>, n: i64) {
    buf.push(b'I');
    buf.push(if n < 0 { b'-' } else { b'+' });
    let magnitude = (n as i128).unsigned_abs();
    encode_magnitude(buf, magnitude);
}

fn encode_unsigned(buf: &mut Vec<u8>, n: u64) {
    buf.extend_from_slice(b"I+");
    encode_magnitude(buf, n as u128);
}

fn encode_magnitude(buf: &mut Vec<u8>, n: u128) {
    let bytes = n.to_be_bytes();
    let first = bytes.iter().position(|&v| v != 0).unwrap_or(bytes.len() - 1);
    prefixed_len(buf, bytes.len() - first);
    buf.extend_from_slice(&bytes[first..]);
}

fn encode_value(buf: &mut Vec<u8>, v: &Value) -> Result<(), NumericPnfError> {
    match v {
        Value::Null => buf.push(b'N'),
        Value::Bool(b) => buf.extend_from_slice(if *b { b"B1" } else { b"B0" }),
        Value::Number(n) => {
            if let Some(s) = n.as_i64() {
                encode_signed(buf, s);
            } else if let Some(u) = n.as_u64() {
                encode_unsigned(buf, u);
            } else if let Some(f) = n.as_f64() {
                buf.push(b'F');
                buf.extend_from_slice(&f.to_be_bytes());
            } else {
                return Err(NumericPnfError::UnsupportedCanonicalInteger);
            }
        }
        Value::String(s) => {
            buf.push(b'S');
            prefixed_len(buf, s.len());
            buf.extend_from_slice(s.as_bytes());
        }
        Value::Array(xs) => {
            buf.push(b'Q');
            prefixed_len(buf, xs.len());
            for x in xs {
                encode_value(buf, x)?;
            }
        }
        Value::Object(map) => {
            buf.push(b'M');
            prefixed_len(buf, map.len());
            let mut ordered = map.iter().collect::<Vec<_>>();
            ordered.sort_by(|(a, _), (b, _)| a.cmp(b));
            for (k, v) in ordered {
                encode_value(buf, &Value::String(k.clone()))?;
                encode_value(buf, v)?;
            }
        }
    }
    Ok(())
}

/// Exact for the JSON-representable typed canonical subset used by the
/// original factor-proposal schema. Python arbitrary-precision non-JSON
/// integers / bytes must be added as explicit typed inputs, never coerced.
pub fn typed_canonical_sha256(v: &Value) -> Result<String, NumericPnfError> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"ITIR");
    prefixed_len(&mut bytes, CANONICAL_CONTRACT.len());
    bytes.extend_from_slice(CANONICAL_CONTRACT.as_bytes());
    encode_value(&mut bytes, v)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

fn get_str<'a>(v: &'a Value, key: &'static str) -> Result<&'a str, NumericPnfError> {
    v.get(key)
        .ok_or(NumericPnfError::MissingField(key))?
        .as_str()
        .ok_or(NumericPnfError::InvalidField(key))
}

fn get_string_array(v: &Value, key: &'static str) -> Result<Vec<String>, NumericPnfError> {
    v.get(key)
        .ok_or(NumericPnfError::MissingField(key))?
        .as_array()
        .ok_or(NumericPnfError::InvalidField(key))?
        .iter()
        .map(|s| s.as_str().map(str::to_owned)
            .ok_or(NumericPnfError::InvalidField(key)))
        .collect()
}

fn get_str_map(v: &Value, key: &'static str) -> Result<BTreeMap<String, String>, NumericPnfError> {
    let obj = v.get(key)
        .ok_or(NumericPnfError::MissingField(key))?
        .as_object()
        .ok_or(NumericPnfError::InvalidField(key))?;
    obj.iter().map(|(k, s)| s.as_str()
        .map(|value| (k.clone(), value.to_owned()))
        .ok_or(NumericPnfError::InvalidField(key))).collect()
}

const IDENTITY_FIELDS: &[&str] = &[
    "document_ref", "source_revision_ref", "semantic_coordinate_ref",
    "scope_ref", "statement_role", "coordinate_kind", "fibre_kind",
    "derivation_role", "factor_type_ref", "source_span_refs",
    "input_observation_refs", "dependency_factor_refs",
    "structural_signature", "role_bindings", "qualifier_state",
    "producer_contract", "producer_scope", "operation_contract",
    "declaration_revision", "ontology_axis_refs", "transport_refs",
    "support_state", "confidence", "assumptions",
    "coverage_requirements", "candidate_payload", "residuals",
];

fn identity_payload(proposal: &Value) -> Result<Value, NumericPnfError> {
    let mut result = Map::new();
    for &key in IDENTITY_FIELDS {
        let v = proposal.get(key)
            .ok_or(NumericPnfError::MissingField(key))?;
        result.insert(key.to_owned(), v.clone());
    }
    Ok(Value::Object(result))
}

#[derive(Debug, Clone)]
struct Proposal {
    ref_id: String,
    value: Value,
    bucket: Bucket,
    roles: BTreeMap<String, String>,
    observations: Vec<String>,
    dependencies: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Bucket {
    coordinate: String,
    fibre: String,
    factor_type: String,
    signature: String,
}

impl Proposal {
    fn checked(value: Value, manifest: &NativeManifest) -> Result<Self, NumericPnfError> {
        if get_str(&value, "document_ref")? != manifest.document_ref
            || get_str(&value, "source_revision_ref")? != manifest.source_revision_ref
        {
            return Err(NumericPnfError::ForeignSource);
        }
        let ref_id = get_str(&value, "proposal_ref")?.to_owned();
        let expected = format!("factor-proposal:{}",
            typed_canonical_sha256(&identity_payload(&value)?)?);
        if ref_id != expected {
            return Err(NumericPnfError::ForgedProposalRef);
        }
        let bucket = Bucket {
            coordinate: get_str(&value, "semantic_coordinate_ref")?.to_owned(),
            fibre: get_str(&value, "fibre_kind")?.to_owned(),
            factor_type: get_str(&value, "factor_type_ref")?.to_owned(),
            signature: get_str(&value, "structural_signature")?.to_owned(),
        };
        Ok(Self {
            ref_id, roles: get_str_map(&value, "role_bindings")?,
            observations: get_string_array(&value, "input_observation_refs")?,
            dependencies: get_string_array(&value, "dependency_factor_refs")?,
            bucket, value,
        })
    }
}

/// Strictly validate a producer proposal and expose its *four-component*
/// grouping coordinate. The incremental owner must never infer an owner from
/// a lexical name, abbreviated signature or unverified proposal reference.
pub fn checked_numeric_proposal_owner(
    manifest: &NativeManifest,
    proposal: &Value,
) -> Result<(String, String, String, String), NumericPnfError> {
    manifest.validate()?;
    let p = Proposal::checked(proposal.clone(), manifest)?;
    Ok((p.bucket.coordinate, p.bucket.fibre,
        p.bucket.factor_type, p.bucket.signature))
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DependencyMode {
    /// Even an empty known set means no declared inputs are available.
    Strict,
    /// Explicit parity mode for original Python behaviour where an empty
    /// known set disables that particular dependency validity check.
    HistoricalEmptyMeansUnchecked,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct NativeWorkCounter {
    pub submitted_proposals: usize,
    pub validated_proposals: usize,
    pub original_proposals_scanned: usize,
    pub dependency_edges_checked: usize,
    pub candidate_group_comparisons: usize,
    pub signature_buckets_visited: usize,
    pub factors_emitted: usize,
    pub residuals_emitted: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct NativeReduction {
    pub schema_version: &'static str,
    pub reduction_contract: &'static str,
    pub document_ref: String,
    pub factors: Vec<Value>,
    pub residuals: Vec<Value>,
    pub proposal_count: usize,
    pub deduplicated_count: usize,
    pub work: NativeWorkCounter,
    pub semantic_authority: &'static str,
    pub identity_promoted: bool,
    pub external_qid_paid: bool,
    pub historical_truth_paid: bool,
}

fn strings_to_value(xs: impl IntoIterator<Item = String>) -> Value {
    Value::Array(xs.into_iter().map(Value::String).collect())
}

fn missing_input_residual(
    document: &str, p: &Proposal, observations: &[String], dependencies: &[String],
) -> Result<Value, NumericPnfError> {
    let identity = json!({
        "proposal_ref": p.ref_id,
        "semantic_coordinate_ref": p.bucket.coordinate,
        "missing_observations": observations,
        "missing_dependencies": dependencies,
    });
    let ref_id = format!("reduction-residual:{}", typed_canonical_sha256(&identity)?);
    Ok(json!({
        "residual_ref": ref_id,
        "document_ref": document,
        "residual_type": "missing_reduction_input",
        "proposal_refs": [p.ref_id],
        "message": "proposal retained outside reduction because declared inputs are unavailable",
        "semantic_coordinate_ref": p.bucket.coordinate,
        "boundary_kind": "input_frontier",
    }))
}

fn compatible(occupied: &BTreeMap<String, String>,
              incoming: &BTreeMap<String, String>) -> bool {
    incoming.iter().all(|(k, v)| occupied.get(k).map_or(true, |old| old == v))
}

fn factor(document: &str, key: &Bucket, group: &[Proposal])
    -> Result<Value, NumericPnfError>
{
    let refs: Vec<String> = group.iter().map(|p| p.ref_id.clone()).collect();
    let identity = json!({
        "document_ref": document,
        "semantic_coordinate_ref": key.coordinate,
        "fibre_kind": key.fibre,
        "factor_type_ref": key.factor_type,
        "structural_signature": key.signature,
        "proposal_refs": refs,
    });
    let id = format!("factor:{}", typed_canonical_sha256(&identity)?);
    let mut roles = Map::new();
    let mut qualifier = Map::new();
    let mut alternatives = Vec::new();
    let mut residuals = BTreeSet::new();
    let mut derivations = BTreeSet::new();
    let mut axes = BTreeSet::new();
    let mut transports = BTreeSet::new();
    let mut supports = BTreeSet::new();
    for p in group {
        for (role, value) in &p.roles {
            roles.insert(role.to_owned(), Value::String(value.to_owned()));
        }
        let q = p.value["qualifier_state"].as_object()
            .ok_or(NumericPnfError::InvalidField("qualifier_state"))?;
        for (key, value) in q { qualifier.insert(key.clone(), value.clone()); }
        let mut alt = p.value["candidate_payload"].as_object()
            .ok_or(NumericPnfError::InvalidField("candidate_payload"))?.clone();
        alt.insert("proposal_ref".into(), Value::String(p.ref_id.clone()));
        for &name in &["derivation_role", "support_state", "confidence"] {
            alt.insert(name.into(), p.value[name].clone());
        }
        alternatives.push(Value::Object(alt));
        for key in &["residuals", "ontology_axis_refs", "transport_refs"] {
            for s in get_string_array(&p.value, key)? {
                match *key {
                    "residuals" => { residuals.insert(s); },
                    "ontology_axis_refs" => { axes.insert(s); },
                    _ => { transports.insert(s); },
                }
            }
        }
        derivations.insert(get_str(&p.value, "derivation_role")?.to_owned());
        supports.insert(get_str(&p.value, "support_state")?.to_owned());
    }
    Ok(json!({
        "factor_ref": id,
        "document_ref": document,
        "semantic_coordinate_ref": key.coordinate,
        "fibre_kind": key.fibre,
        "factor_type_ref": key.factor_type,
        "structural_signature": key.signature,
        "proposal_refs": refs,
        "alternatives": alternatives,
        "role_bindings": roles,
        "qualifier_state": qualifier,
        "residuals": strings_to_value(residuals),
        "derivation_roles": strings_to_value(derivations),
        "ontology_axis_refs": strings_to_value(axes),
        "transport_refs": strings_to_value(transports),
        "support_states": strings_to_value(supports),
        "closure_state": if group.iter().any(|p| !get_string_array(&p.value, "residuals").unwrap_or_default().is_empty()) {
            "requires_review"
        } else { "locally_closed" },
    }))
}

/// Rebuild the canonical group for the supplied owner scope. Do not assume
/// that the first-compatible greedy reducer is an associative fold.
///
/// This is a pure kernel. SQL source occurrence, H9 demand and document-level
/// admission are intentionally NOT written here.
pub fn reduce_canonical(
    manifest: &NativeManifest,
    original_proposals: &[Value],
    known_observations: &BTreeSet<String>,
    known_dependencies: &BTreeSet<String>,
    mode: DependencyMode,
) -> Result<NativeReduction, NumericPnfError> {
    manifest.validate()?;
    let mut work = NativeWorkCounter::default();
    work.submitted_proposals = original_proposals.len();

    let mut parsed = original_proposals.iter()
        .cloned().map(|v| Proposal::checked(v, manifest))
        .collect::<Result<Vec<_>, _>>()?;
    parsed.sort_by(|a, b| a.ref_id.cmp(&b.ref_id));
    work.validated_proposals = parsed.len();
    let mut valid = BTreeMap::<String, Proposal>::new();
    let mut residuals = Vec::<Value>::new();
    let mut accepted = 0usize;
    for p in parsed {
        work.original_proposals_scanned += 1;
        work.dependency_edges_checked += p.observations.len() + p.dependencies.len();
        let missing_obs = if mode == DependencyMode::HistoricalEmptyMeansUnchecked
                && known_observations.is_empty() {
            vec![]
        } else {
            p.observations.iter().filter(|s| !known_observations.contains(*s))
                .cloned().collect::<BTreeSet<_>>().into_iter().collect()
        };
        let missing_dep = if mode == DependencyMode::HistoricalEmptyMeansUnchecked
                && known_dependencies.is_empty() {
            vec![]
        } else {
            p.dependencies.iter().filter(|s| !known_dependencies.contains(*s))
                .cloned().collect::<BTreeSet<_>>().into_iter().collect()
        };
        if !missing_obs.is_empty() || !missing_dep.is_empty() {
            residuals.push(missing_input_residual(
                &manifest.document_ref, &p, &missing_obs, &missing_dep)?);
            continue;
        }
        accepted += 1;
        match valid.entry(p.ref_id.clone()) {
            std::collections::btree_map::Entry::Vacant(v) => { v.insert(p); },
            std::collections::btree_map::Entry::Occupied(existing) => {
                if existing.get().value != p.value {
                    return Err(NumericPnfError::ConflictingProposal);
                }
            }
        }
    }
    let deduplicated_count = accepted - valid.len();
    let mut buckets = BTreeMap::<Bucket, Vec<Proposal>>::new();
    for (_, p) in valid {
        buckets.entry(p.bucket.clone()).or_default().push(p);
    }
    let mut factors = Vec::new();
    for (key, proposals) in buckets {
        work.signature_buckets_visited += 1;
        let mut groups: Vec<Vec<Proposal>> = Vec::new();
        let mut occupied_roles: Vec<BTreeMap<String, String>> = Vec::new();
        for p in proposals {
            let mut target = None;
            for (index, occupied) in occupied_roles.iter().enumerate() {
                // A highly conflicting single signature is capable of
                // quadratic work even after bucket indexing. Abort the
                // COMPLETE owner without admitting a partial group.
                if work.candidate_group_comparisons>=4_000_000{
                    return Err(NumericPnfError::SemanticWorkBudget);
                }
                work.candidate_group_comparisons += 1;
                if compatible(occupied, &p.roles) {
                    target = Some(index);
                    break;
                }
            }
            if let Some(index) = target {
                occupied_roles[index].extend(p.roles.clone());
                groups[index].push(p);
            } else {
                occupied_roles.push(p.roles.clone());
                groups.push(vec![p]);
            }
        }
        if groups.len() > 1 {
            let refs = groups.iter().flat_map(|g| g.iter().map(|p| p.ref_id.clone()))
                .collect::<BTreeSet<_>>();
            let refs: Vec<String> = refs.into_iter().collect();
            let id = format!("reduction-residual:{}",
                typed_canonical_sha256(&json!({
                    "kind": "incompatible_alternatives",
                    "semantic_coordinate_ref": key.coordinate,
                    "refs": refs,
                }))?);
            residuals.push(json!({
                "residual_ref": id,
                "document_ref": manifest.document_ref,
                "residual_type": "incompatible_alternatives",
                "proposal_refs": refs,
                "message": "proposals in one semantic fibre disagree on one or more occupied coordinates",
                "semantic_coordinate_ref": key.coordinate,
                "boundary_kind": "conflicted_fibre",
            }));
        }
        for group in groups {
            factors.push(factor(&manifest.document_ref, &key, &group)?);
        }
    }
    factors.sort_by(|a, b| a["factor_ref"].as_str().cmp(&b["factor_ref"].as_str()));
    residuals.sort_by(|a, b|
        a["residual_ref"].as_str().cmp(&b["residual_ref"].as_str()));
    work.factors_emitted = factors.len();
    work.residuals_emitted = residuals.len();
    Ok(NativeReduction {
        schema_version: "sl.pnf.native_reduction.v0_1",
        reduction_contract: REDUCTION_CONTRACT,
        document_ref: manifest.document_ref.clone(),
        factors, residuals,
        proposal_count: original_proposals.len(),
        deduplicated_count,
        work,
        semantic_authority: "candidate_only",
        identity_promoted: false,
        external_qid_paid: false,
        historical_truth_paid: false,
    })
}

/// Read-only execution-fibre state. Resource limits may move active candidates
/// to deferred, but cannot create semantic refutations or close the world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VerifiedSqlCandidateRefutation {
    pub demand_id: u64,
    pub candidate_id: u64,
    pub evidence_id: u64,
    pub evidence_ref: String,
}

/// The semantic fields are deliberately PRIVATE: no public JSON or string
/// constructor can fabricate a closed world or refuted candidate. Original
/// SQL evidence must be checked before constructing semantic changes.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CandidateFibre {
    demand_id: u64,
    target_kind: i16,
    represented: BTreeSet<u64>,
    active: BTreeSet<u64>,
    deferred: BTreeSet<u64>,
    refuted_by_evidence: BTreeMap<u64, String>,
    outside_model_possible: bool,
    coverage_witness_ref: Option<String>,
    signed_residuals: BTreeMap<(u64, i16), i64>,
}

impl CandidateFibre {
    pub fn fresh_for_original_demand(demand_id: u64, target_kind: i16) -> Self {
        Self {
            demand_id, target_kind,
            represented: BTreeSet::new(), active: BTreeSet::new(),
            deferred: BTreeSet::new(), refuted_by_evidence: BTreeMap::new(),
            outside_model_possible: true, coverage_witness_ref: None,
            signed_residuals: BTreeMap::new(),
        }
    }
    pub fn fresh() -> Self { Self::fresh_for_original_demand(0,0) }
    pub fn represented_count(&self) -> usize { self.represented.len() }
    pub fn deferred_count(&self) -> usize { self.deferred.len() }
    pub fn refuted_count(&self) -> usize { self.refuted_by_evidence.len() }
    pub fn represented_ids(&self) -> impl Iterator<Item=u64> + '_ {
        self.represented.iter().copied()
    }
    pub fn refuted_ids(&self) -> impl Iterator<Item=u64> + '_ {
        self.refuted_by_evidence.keys().copied()
    }
    pub fn live_ids(&self) -> impl Iterator<Item=u64> + '_ {
        self.represented.iter().copied().filter(|id| !self.refuted_by_evidence.contains_key(id))
    }
    pub fn open_world(&self) -> bool { self.outside_model_possible }
    pub fn original_demand(&self) -> (u64,i16) {
        (self.demand_id,self.target_kind)
    }

    pub fn represent(&mut self, id: u64) {
        self.represented.insert(id);
        self.active.insert(id);
        self.deferred.remove(&id);
    }

    pub fn budget_prune(&mut self, id: u64) {
        if self.active.remove(&id) {
            self.deferred.insert(id);
        }
    }

    pub fn reopen(&mut self, id: u64) {
        if self.represented.contains(&id)
            && !self.refuted_by_evidence.contains_key(&id) {
            self.deferred.remove(&id);
            self.active.insert(id);
        }
    }

    pub(crate) fn refute_with_original_sql_evidence(
        &mut self,receipt: &VerifiedSqlCandidateRefutation
    ) -> Result<(), NumericPnfError> {
        if receipt.demand_id!=self.demand_id
            ||receipt.candidate_id==0||receipt.evidence_id==0
            ||receipt.evidence_ref.is_empty()
            ||!self.represented.contains(&receipt.candidate_id) {
            return Err(NumericPnfError::InvalidField("original_sql_candidate_evidence"));
        }
        self.refuted_by_evidence.insert(receipt.candidate_id,
            receipt.evidence_ref.clone());
        self.active.remove(&receipt.candidate_id);
        self.deferred.remove(&receipt.candidate_id);
        Ok(())
    }

    /// ONLY original SQL open-world coverage may close the semantic fibre.
    /// A cap, normalized score, S29 queue state or candidate-count estimate
    /// cannot call this through the public API.
    pub(crate) fn close_with_original_sql_coverage(
        &mut self, demand_id:u64,coverage_ref:&str,
        resource_limited:bool,represented_residual_count:u64
    ) -> Result<(),NumericPnfError> {
        if demand_id==0||demand_id!=self.demand_id
            ||coverage_ref.is_empty()||resource_limited
            ||represented_residual_count!=0 {
            return Err(NumericPnfError::InvalidField("original_sql_open_world_coverage"));
        }
        self.outside_model_possible=false;
        self.coverage_witness_ref=Some(coverage_ref.to_owned());
        Ok(())
    }

    pub(crate) fn record_original_signed_residual(
        &mut self,id:u64,horizon:i16,residual:i64
    ) -> Result<(),NumericPnfError>{
        if !self.represented.contains(&id)
            || ![3i16,6i16,9i16].contains(&horizon){
            return Err(NumericPnfError::InvalidField("original_signed_residual_horizon"));
        }
        self.signed_residuals.insert((id,horizon),residual);
        Ok(())
    }
    pub fn signed_residual(&self,id:u64,horizon:i16)->Option<i64>{
        self.signed_residuals.get(&(id,horizon)).copied()
    }

    pub fn unique_admissible(&self) -> Option<u64> {
        if self.outside_model_possible ||
            self.coverage_witness_ref.as_deref().unwrap_or("").is_empty() {
            return None;
        }
        let mut live = self.represented.iter()
            .filter(|id| !self.refuted_by_evidence.contains_key(*id));
        let first = *live.next()?;
        if live.next().is_some() {
            return None;
        }
        Some(first)
    }

    pub fn eligible_for_identity_admission(&self) -> bool {
        // Even closed-world unique candidacy is NOT an original admitted
        // numeric identity witness. Only original SQL owner can issue it.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_canonical_strings_have_stable_input_envelope() {
        let digest = typed_canonical_sha256(&json!({"a": 1, "b": ["x", true]})).unwrap();
        // Golden bytes from the original ITIR typed-canonical v1 encoding,
        // not a second hash of the Rust implementation.
        assert_eq!(digest, "d30666b9f8b947091dc4de5ecbc31380d8e699f9bb86df3a3ee82d202f2f6b12");
        assert_eq!(digest, typed_canonical_sha256(
            &json!({"b": ["x", true], "a": 1})).unwrap());
        assert_eq!(typed_canonical_sha256(&json!({
            "signed": [-1, 0, 1], "float": 0.125, "null": null
        })).unwrap(), "2231f668db25c1da34e16113012496a587fac4ec5c9d2540234a637d386e8797");
    }

    #[test]
    fn distinct_source_manifest_refused_before_reduction() {
        let m = NativeManifest {
            document_ref: "document:source".into(),
            source_revision_ref: "rev:1".into(),
            canonical_content_sha256: "0".repeat(64),
            source_byte_length: 5, source_codepoint_length: 5,
            coordinate_contract_ref: "unicode-codepoints:v1".into(),
            parser_run_ref: "parser:1".into(),
            rule_set_revision: "rules:1".into(),
        };
        assert!(matches!(m.verify_source_bytes(b"hello"),
            Err(NumericPnfError::SourceMismatch)));
    }

    #[test]
    fn pruning_never_refutes_or_closes_world() {
        let mut fibre = CandidateFibre::fresh();
        fibre.represent(1);
        fibre.represent(2);
        fibre.budget_prune(2);
        assert!(fibre.deferred.contains(&2));
        assert!(!fibre.refuted_by_evidence.contains_key(&2));
        assert_eq!(fibre.unique_admissible(), None);
        assert!(!fibre.eligible_for_identity_admission());
        fibre.reopen(2);
        assert!(fibre.active.contains(&2));
    }

    #[test]
    fn semantic_refutation_requires_candidate_evidence() {
        let mut f = CandidateFibre::fresh();
        f.represent(1);
        f.represent(2);
        assert!(f.refute_with_original_sql_evidence(
            &VerifiedSqlCandidateRefutation {
                demand_id:0,candidate_id:2,evidence_id:0,evidence_ref:"".into()
            }).is_err());
        assert_eq!(f.refuted_by_evidence.len(), 0);
        // In this fixture use a simulated original SQL demand; the public
        // runtime cannot construct the refutation credential itself.
        f.demand_id=17;
        f.refute_with_original_sql_evidence(&VerifiedSqlCandidateRefutation {
            demand_id:17,candidate_id:2,evidence_id:51,
            evidence_ref:"original-sql-evidence:51".into()
        }).unwrap();
        assert_eq!(f.unique_admissible(), None);
        f.close_with_original_sql_coverage(17,"coverage:original",false,0).unwrap();
        assert_eq!(f.unique_admissible(), Some(1));
        assert!(!f.eligible_for_identity_admission());
    }

    #[test]
    fn no_coverage_for_discarded_live_candidates() {
        let mut f = CandidateFibre::fresh();
        f.represent(1);
        f.represent(2);
        f.budget_prune(2);
        // A forged arbitrary coverage string has no public semantic effect.
        assert!(f.close_with_original_sql_coverage(
            17,"coverage:claimed",false,0).is_err());
        assert_eq!(f.unique_admissible(), None);
    }

    #[test]
    fn first_compatible_roles_are_stable() {
        let mut groups = vec![
            BTreeMap::from([("subject".into(), "a".into())]),
            BTreeMap::from([("subject".into(), "b".into())]),
        ];
        let p = BTreeMap::from([("predicate".into(), "x".into())]);
        let index = groups.iter().position(|g| compatible(g, &p));
        assert_eq!(index, Some(0));
        groups[0].extend(p);
        assert_eq!(groups[0]["predicate"], "x");
    }
}
