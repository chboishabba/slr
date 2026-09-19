//! Bounded supervised Wikidata type closure for GWB external-classification fallback.
//!
//! Semantics intentionally mirror the executable Wikidata worker in dashi_lean4:
//! direct P279 edges generate subclass reachability and P31 edges lift an entity
//! into the P279 closure of its observed direct types. This module is only an
//! evidence producer. It never turns Wikidata truthy/query semantics into
//! epistemic truth, residual payment, or edit authority.
//!
//! SensibLaw Nat/Climate discipline is retained:
//! - revision-pinned evidence only;
//! - observed absence is not global absence;
//! - incomplete/truncated coverage abstains;
//! - dimensional/type mismatch creates review pressure, not automatic action.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::env;
use std::fs;
use std::io::{Cursor, Write};
use std::process::{Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};

use sensiblaw_route_selector::{decode_route_candidate, ProducerFamily, RouteCandidate, RouteFamily};
use sensiblaw_wikimedia_candidate_provider::{
    emit_candidates_from_rdf, fetch_latest_entity_rdf_revision_receipt, ProviderError,
};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeClosureQuestion {
    TypeClass,
    Superclass,
}

impl TypeClosureQuestion {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TypeClass => "type-class",
            Self::Superclass => "superclass",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TypeObservationProperty {
    InstanceOf,
    SubclassOf,
}

impl TypeObservationProperty {
    #[must_use]
    pub const fn property_ref(self) -> &'static str {
        match self {
            Self::InstanceOf => "P31",
            Self::SubclassOf => "P279",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TypeObservation {
    pub subject_qid: String,
    pub property: TypeObservationProperty,
    pub target_qid: String,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeClosureRequest {
    pub root_qid: String,
    pub question: TypeClosureQuestion,
    pub max_depth: usize,
    pub max_nodes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeClosureDisposition {
    DirectSuperclassObserved,
    InstanceTypeClosureObserved,
    InstanceShapedForSuperclassQuestion,
    ClassShapedForTypeQuestion,
    MixedDirectTypingObserved,
    ObservedNoTypedEdge,
    Truncated,
}

impl TypeClosureDisposition {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DirectSuperclassObserved => "direct-superclass-observed",
            Self::InstanceTypeClosureObserved => "instance-type-closure-observed",
            Self::InstanceShapedForSuperclassQuestion => {
                "instance-shaped-for-superclass-question"
            }
            Self::ClassShapedForTypeQuestion => "class-shaped-for-type-question",
            Self::MixedDirectTypingObserved => "mixed-direct-typing-observed",
            Self::ObservedNoTypedEdge => "observed-no-typed-edge",
            Self::Truncated => "truncated",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeNodeReceipt {
    pub qid: String,
    pub source_revision_ref: String,
    pub evidence_digest_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservedTypeClosure {
    pub request: TypeClosureRequest,
    pub disposition: TypeClosureDisposition,
    pub direct_instance_types: Vec<String>,
    pub direct_superclasses: Vec<String>,
    pub observed_instance_type_closure: Vec<String>,
    pub observed_superclass_closure: Vec<String>,
    pub observations: Vec<TypeObservation>,
    pub node_receipts: Vec<TypeNodeReceipt>,
    pub truncated: bool,
    pub observed_direct_surface_complete: bool,
    pub global_ontology_complete: bool,
    pub snapshot_simultaneous: bool,
    pub superclass_residual_paid: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub evidence_digest_ref: String,
    pub source_manifest_ref: String,
}

#[derive(Debug, Error)]
pub enum TypeClosureError {
    #[error("invalid root QID: {0}")]
    InvalidQid(String),
    #[error("type-closure max_depth must be at least one")]
    InvalidDepth,
    #[error("type-closure max_nodes must be at least one")]
    InvalidNodeLimit,
    #[error("wikimedia provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("route decoding error: {0}")]
    Route(#[from] sensiblaw_route_selector::RouteSelectorError),
}

fn valid_qid(value: &str) -> bool {
    let Some(rest) = value.strip_prefix('Q') else {
        return false;
    };
    !rest.is_empty() && rest.bytes().all(|byte| byte.is_ascii_digit())
}

fn normalize_observations(observations: &[TypeObservation]) -> Vec<TypeObservation> {
    let mut rows = observations.to_vec();
    rows.sort();
    rows.dedup();
    rows
}

fn adjacency(
    observations: &[TypeObservation],
    property: TypeObservationProperty,
) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::<String, Vec<String>>::new();
    for observation in observations {
        if observation.property != property {
            continue;
        }
        out.entry(observation.subject_qid.clone())
            .or_default()
            .push(observation.target_qid.clone());
    }
    for targets in out.values_mut() {
        targets.sort();
        targets.dedup();
    }
    out
}

fn p279_closure(
    starts: &[String],
    subclass: &BTreeMap<String, Vec<String>>,
) -> Vec<String> {
    let mut seen = BTreeSet::new();
    let mut queue = VecDeque::new();
    for start in starts {
        if seen.insert(start.clone()) {
            queue.push_back(start.clone());
        }
    }
    while let Some(current) = queue.pop_front() {
        if let Some(parents) = subclass.get(&current) {
            for parent in parents {
                if seen.insert(parent.clone()) {
                    queue.push_back(parent.clone());
                }
            }
        }
    }
    seen.into_iter().collect()
}

fn aggregate_digest(
    request: &TypeClosureRequest,
    observations: &[TypeObservation],
    receipts: &[TypeNodeReceipt],
    truncated: bool,
) -> String {
    let max_depth = request.max_depth.to_string();
    let max_nodes = request.max_nodes.to_string();
    let mut hasher = Sha256::new();
    hasher.update(b"gwb-supervised-type-closure:v1\0");
    for value in [
        request.root_qid.as_str(),
        request.question.as_str(),
        max_depth.as_str(),
        max_nodes.as_str(),
        if truncated { "truncated" } else { "bounded-complete" },
    ] {
        hasher.update((value.len() as u64).to_be_bytes());
        hasher.update(value.as_bytes());
    }
    for observation in observations {
        for value in [
            observation.subject_qid.as_str(),
            observation.property.property_ref(),
            observation.target_qid.as_str(),
            observation.source_revision_ref.as_str(),
            observation.evidence_digest_ref.as_str(),
        ] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
    }
    let mut receipts = receipts.to_vec();
    receipts.sort_by(|left, right| left.qid.cmp(&right.qid));
    for receipt in receipts {
        for value in [
            receipt.qid.as_str(),
            receipt.source_revision_ref.as_str(),
            receipt.evidence_digest_ref.as_str(),
        ] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
    }
    format!(
        "sha256:{}",
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub fn evaluate_observed_type_closure(
    request: &TypeClosureRequest,
    observations: &[TypeObservation],
    truncated: bool,
) -> Result<ObservedTypeClosure, TypeClosureError> {
    if !valid_qid(&request.root_qid) {
        return Err(TypeClosureError::InvalidQid(request.root_qid.clone()));
    }
    if request.max_depth == 0 {
        return Err(TypeClosureError::InvalidDepth);
    }
    if request.max_nodes == 0 {
        return Err(TypeClosureError::InvalidNodeLimit);
    }

    let observations = normalize_observations(observations);
    let instance = adjacency(&observations, TypeObservationProperty::InstanceOf);
    let subclass = adjacency(&observations, TypeObservationProperty::SubclassOf);

    let direct_instance_types = instance
        .get(&request.root_qid)
        .cloned()
        .unwrap_or_default();
    let direct_superclasses = subclass
        .get(&request.root_qid)
        .cloned()
        .unwrap_or_default();

    let observed_instance_type_closure = p279_closure(&direct_instance_types, &subclass);
    let observed_superclass_closure = p279_closure(&direct_superclasses, &subclass);

    let disposition = if truncated {
        TypeClosureDisposition::Truncated
    } else if !direct_instance_types.is_empty() && !direct_superclasses.is_empty() {
        TypeClosureDisposition::MixedDirectTypingObserved
    } else {
        match request.question {
            TypeClosureQuestion::Superclass if !direct_superclasses.is_empty() => {
                TypeClosureDisposition::DirectSuperclassObserved
            }
            TypeClosureQuestion::Superclass if !direct_instance_types.is_empty() => {
                TypeClosureDisposition::InstanceShapedForSuperclassQuestion
            }
            TypeClosureQuestion::TypeClass if !direct_instance_types.is_empty() => {
                TypeClosureDisposition::InstanceTypeClosureObserved
            }
            TypeClosureQuestion::TypeClass if !direct_superclasses.is_empty() => {
                TypeClosureDisposition::ClassShapedForTypeQuestion
            }
            _ => TypeClosureDisposition::ObservedNoTypedEdge,
        }
    };

    let mut node_receipts = observations
        .iter()
        .map(|row| TypeNodeReceipt {
            qid: row.subject_qid.clone(),
            source_revision_ref: row.source_revision_ref.clone(),
            evidence_digest_ref: row.evidence_digest_ref.clone(),
        })
        .collect::<Vec<_>>();
    node_receipts.sort_by(|left, right| left.qid.cmp(&right.qid));
    node_receipts.dedup_by(|left, right| left.qid == right.qid);

    let evidence_digest_ref =
        aggregate_digest(request, &observations, &node_receipts, truncated);
    let source_manifest_ref = format!(
        "wikidata-type-closure:{}:{}",
        request.root_qid, evidence_digest_ref
    );

    Ok(ObservedTypeClosure {
        request: request.clone(),
        disposition,
        direct_instance_types,
        direct_superclasses,
        observed_instance_type_closure,
        observed_superclass_closure,
        observations,
        node_receipts,
        truncated,
        observed_direct_surface_complete: true,
        global_ontology_complete: false,
        snapshot_simultaneous: false,
        superclass_residual_paid: false,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
        evidence_digest_ref,
        source_manifest_ref,
    })
}

fn type_observations_from_rdf(
    qid: &str,
    source_revision_ref: &str,
    evidence_digest_ref: &str,
    rdf_bytes: &[u8],
) -> Result<Vec<TypeObservation>, TypeClosureError> {
    let mut encoded = Vec::new();
    emit_candidates_from_rdf(qid, Cursor::new(rdf_bytes), &mut encoded)?;
    let mut cursor = Cursor::new(encoded);
    let mut out = Vec::new();
    while let Some(route) = decode_route_candidate(&mut cursor)? {
        if route.route_family != RouteFamily::WikidataProperty {
            continue;
        }
        let property = match route.property_ref.as_str() {
            "P31" => TypeObservationProperty::InstanceOf,
            "P279" => TypeObservationProperty::SubclassOf,
            _ => continue,
        };
        out.push(TypeObservation {
            subject_qid: qid.to_owned(),
            property,
            target_qid: route.target_ref,
            source_revision_ref: source_revision_ref.to_owned(),
            evidence_digest_ref: evidence_digest_ref.to_owned(),
        });
    }
    out.sort();
    out.dedup();
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LiveAcquisitionPolicy {
    pub max_retries: usize,
    pub backoff_base_seconds: f64,
    pub max_backoff_seconds: f64,
    pub min_request_interval_seconds: f64,
}

impl Default for LiveAcquisitionPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            backoff_base_seconds: 2.0,
            max_backoff_seconds: 60.0,
            min_request_interval_seconds: 0.35,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TypeProviderStats {
    pub provider_calls: usize,
    pub retries: usize,
    pub rate_limit_retries: usize,
    pub preferred_slice_hits: usize,
    pub snapshot_hits: usize,
    pub live_fallbacks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeNodeAcquisition {
    pub qid: String,
    pub observations: Vec<TypeObservation>,
    pub node_receipt: TypeNodeReceipt,
}

pub trait TypeClosureNodeProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError>;

    fn snapshot_simultaneous(&self) -> bool {
        false
    }

    fn stats(&self) -> TypeProviderStats {
        TypeProviderStats::default()
    }
}

pub struct RevisionPinnedLiveTypeProvider {
    policy: LiveAcquisitionPolicy,
    last_request_at: Option<Instant>,
    stats: TypeProviderStats,
}

impl RevisionPinnedLiveTypeProvider {
    #[must_use]
    pub fn new(policy: LiveAcquisitionPolicy) -> Self {
        Self {
            policy,
            last_request_at: None,
            stats: TypeProviderStats::default(),
        }
    }

    fn throttle(&mut self) {
        let minimum = self.policy.min_request_interval_seconds.max(0.0);
        if minimum == 0.0 {
            return;
        }
        if let Some(last) = self.last_request_at {
            let elapsed = last.elapsed().as_secs_f64();
            if elapsed < minimum {
                sleep(Duration::from_secs_f64(minimum - elapsed));
            }
        }
    }

    fn backoff_seconds(&self, attempt: usize) -> f64 {
        let exponential = self.policy.backoff_base_seconds.max(0.0)
            * 2f64.powi(attempt.min(i32::MAX as usize) as i32);
        exponential.min(self.policy.max_backoff_seconds.max(0.0))
    }
}

impl Default for RevisionPinnedLiveTypeProvider {
    fn default() -> Self {
        Self::new(LiveAcquisitionPolicy::default())
    }
}

impl TypeClosureNodeProvider for RevisionPinnedLiveTypeProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        for attempt in 0..=self.policy.max_retries {
            self.throttle();
            self.stats.provider_calls = self.stats.provider_calls.saturating_add(1);
            self.last_request_at = Some(Instant::now());
            match fetch_latest_entity_rdf_revision_receipt(qid) {
                Ok(acquired) => {
                    let rows = type_observations_from_rdf(
                        qid,
                        &acquired.source_revision_ref,
                        &acquired.content_digest_ref,
                        &acquired.rdf_bytes,
                    )?;
                    return Ok(Some(TypeNodeAcquisition {
                        qid: qid.to_owned(),
                        observations: rows,
                        node_receipt: TypeNodeReceipt {
                            qid: qid.to_owned(),
                            source_revision_ref: acquired.source_revision_ref,
                            evidence_digest_ref: acquired.content_digest_ref,
                        },
                    }));
                }
                Err(error)
                    if error.network_is_retryable() && attempt < self.policy.max_retries =>
                {
                    self.stats.retries = self.stats.retries.saturating_add(1);
                    if error.network_status_code() == Some(429) {
                        self.stats.rate_limit_retries =
                            self.stats.rate_limit_retries.saturating_add(1);
                    }
                    let delay = error
                        .network_retry_after_seconds()
                        .unwrap_or_else(|| self.backoff_seconds(attempt));
                    if delay > 0.0 {
                        sleep(Duration::from_secs_f64(delay));
                    }
                }
                Err(error) => return Err(TypeClosureError::Provider(error)),
            }
        }
        unreachable!("bounded retry loop must return")
    }

    fn stats(&self) -> TypeProviderStats {
        self.stats
    }
}


/// Preferred local/revisioned P31/P279 slice.
///
/// Schema:
/// {
///   "nodes": {
///     "Q1": { "P31": ["Q2"], "P279": ["Q3"] }
///   }
/// }
///
/// This deliberately carries only the two classification predicates. A miss
/// is not a negative ontology fact; it falls through to the general snapshot
/// or governed live provider.
pub struct PredicateSliceTypeProvider {
    snapshot_ref: String,
    nodes: BTreeMap<String, (Vec<String>, Vec<String>)>,
    stats: TypeProviderStats,
}

impl PredicateSliceTypeProvider {
    pub fn from_json(
        json: &str,
        snapshot_ref: impl Into<String>,
    ) -> Result<Self, TypeClosureError> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(|error| {
            TypeClosureError::Provider(ProviderError::InvalidInput(format!(
                "invalid P31/P279 slice JSON: {error}"
            )))
        })?;
        let object = value
            .get("nodes")
            .and_then(serde_json::Value::as_object)
            .ok_or_else(|| {
                TypeClosureError::Provider(ProviderError::InvalidInput(
                    "P31/P279 slice must contain an object field named nodes".into(),
                ))
            })?;
        let mut nodes = BTreeMap::new();
        for (qid, row) in object {
            if !valid_qid(qid) {
                return Err(TypeClosureError::InvalidQid(qid.clone()));
            }
            let parse = |pid: &str| -> Result<Vec<String>, TypeClosureError> {
                let mut values = row
                    .get(pid)
                    .and_then(serde_json::Value::as_array)
                    .map(|rows| {
                        rows.iter()
                            .filter_map(serde_json::Value::as_str)
                            .map(str::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if let Some(invalid) = values.iter().find(|value| !valid_qid(value)) {
                    return Err(TypeClosureError::InvalidQid(invalid.clone()));
                }
                values.sort();
                values.dedup();
                Ok(values)
            };
            nodes.insert(qid.clone(), (parse("P31")?, parse("P279")?));
        }
        Ok(Self {
            snapshot_ref: snapshot_ref.into(),
            nodes,
            stats: TypeProviderStats::default(),
        })
    }

    pub fn from_env() -> Result<Option<Self>, TypeClosureError> {
        let Ok(path) = env::var("SLR_WIKIDATA_P31_P279_SLICE") else {
            return Ok(None);
        };
        if path.trim().is_empty() {
            return Ok(None);
        }
        let snapshot_ref = env::var("SLR_WIKIDATA_P31_P279_SLICE_REF").map_err(|_| {
            TypeClosureError::Provider(ProviderError::InvalidInput(
                "SLR_WIKIDATA_P31_P279_SLICE_REF is required when the specialised slice is configured"
                    .into(),
            ))
        })?;
        if snapshot_ref.trim().is_empty() {
            return Err(TypeClosureError::Provider(ProviderError::InvalidInput(
                "SLR_WIKIDATA_P31_P279_SLICE_REF must not be empty".into(),
            )));
        }
        let json = fs::read_to_string(&path).map_err(|error| {
            TypeClosureError::Provider(ProviderError::InvalidInput(format!(
                "failed to read P31/P279 slice {path}: {error}"
            )))
        })?;
        Self::from_json(&json, snapshot_ref).map(Some)
    }
}

impl TypeClosureNodeProvider for PredicateSliceTypeProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        self.stats.provider_calls = self.stats.provider_calls.saturating_add(1);
        let Some((p31, p279)) = self.nodes.get(qid) else {
            return Ok(None);
        };
        let source_revision_ref = format!("wikidata-p31-p279-slice:{}:{qid}", self.snapshot_ref);
        let evidence_digest_ref =
            ZelphHfTypeProvider::digest_rows(qid, p31, p279, &source_revision_ref);
        let mut observations = Vec::new();
        for target_qid in p31 {
            observations.push(TypeObservation {
                subject_qid: qid.to_owned(),
                property: TypeObservationProperty::InstanceOf,
                target_qid: target_qid.clone(),
                source_revision_ref: source_revision_ref.clone(),
                evidence_digest_ref: evidence_digest_ref.clone(),
            });
        }
        for target_qid in p279 {
            observations.push(TypeObservation {
                subject_qid: qid.to_owned(),
                property: TypeObservationProperty::SubclassOf,
                target_qid: target_qid.clone(),
                source_revision_ref: source_revision_ref.clone(),
                evidence_digest_ref: evidence_digest_ref.clone(),
            });
        }
        Ok(Some(TypeNodeAcquisition {
            qid: qid.to_owned(),
            observations,
            node_receipt: TypeNodeReceipt {
                qid: qid.to_owned(),
                source_revision_ref,
                evidence_digest_ref,
            },
        }))
    }

    fn snapshot_simultaneous(&self) -> bool {
        true
    }

    fn stats(&self) -> TypeProviderStats {
        self.stats
    }
}

pub struct ZelphHfTypeProvider {
    executable: String,
    source: String,
    snapshot_ref: String,
    language: String,
    stats: TypeProviderStats,
}

impl ZelphHfTypeProvider {
    #[must_use]
    pub fn new(
        executable: impl Into<String>,
        source: impl Into<String>,
        snapshot_ref: impl Into<String>,
        language: impl Into<String>,
    ) -> Self {
        Self {
            executable: executable.into(),
            source: source.into(),
            snapshot_ref: snapshot_ref.into(),
            language: language.into(),
            stats: TypeProviderStats::default(),
        }
    }

    #[must_use]
    pub fn from_env() -> Option<Self> {
        let source = env::var("SLR_ZELPH_WIKIDATA_SOURCE").ok()?;
        if source.trim().is_empty() {
            return None;
        }
        let executable = env::var("SLR_ZELPH_EXECUTABLE").unwrap_or_else(|_| "zelph".into());
        let snapshot_ref = env::var("SLR_ZELPH_SNAPSHOT_REF").ok()?;
        if snapshot_ref.trim().is_empty() {
            return None;
        }
        let language = env::var("SLR_ZELPH_LANGUAGE").unwrap_or_else(|_| "en".into());
        Some(Self::new(executable, source, snapshot_ref, language))
    }

    fn quote(value: &str) -> String {
        let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
        format!("\"{escaped}\"")
    }

    fn routeable_manifest(&self) -> bool {
        self.source.ends_with(".json") || self.source.starts_with("hf://")
    }

    fn route_name_load_command(&self, qid: &str) -> String {
        let source = Self::quote(&self.source);
        format!(
            ".load-partial {source} route-name={qid} route-lang=wikidata left=none right=none nameOfNode=none"
        )
    }

    fn route_node_load_command(&self, qid: &str, node_id: u64) -> String {
        let source = Self::quote(&self.source);
        format!(
            ".load-partial {source} route-node={node_id} route-name={qid} route-lang=wikidata"
        )
    }

    fn run_commands(&mut self, commands: &[String]) -> Result<String, TypeClosureError> {
        let mut child = Command::new(&self.executable)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                TypeClosureError::Provider(ProviderError::InvalidInput(format!(
                    "Zelph executable unavailable ({}): {error}",
                    self.executable
                )))
            })?;
        if let Some(stdin) = child.stdin.as_mut() {
            stdin
                .write_all(commands.join("\n").as_bytes())
                .map_err(|error| TypeClosureError::Provider(ProviderError::Io(error)))?;
        }
        let output = child
            .wait_with_output()
            .map_err(|error| TypeClosureError::Provider(ProviderError::Io(error)))?;
        self.stats.provider_calls = self.stats.provider_calls.saturating_add(1);
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        if !output.status.success() || combined.contains("Error in line") {
            return Err(TypeClosureError::Provider(ProviderError::InvalidInput(
                format!(
                    "Zelph route-aware snapshot query failed: {}",
                    combined
                        .chars()
                        .rev()
                        .take(2000)
                        .collect::<String>()
                        .chars()
                        .rev()
                        .collect::<String>()
                ),
            )));
        }
        Ok(combined)
    }

    fn parse_resolved_node_id(output: &str) -> Option<u64> {
        output.lines().find_map(|line| {
            line.trim()
                .strip_prefix("Resolved to node ID:")
                .or_else(|| line.trim().strip_prefix("Node ID:"))
                .and_then(|value| value.trim().parse::<u64>().ok())
        })
    }

    fn resolve_route_node(&mut self, qid: &str) -> Result<Option<u64>, TypeClosureError> {
        if !self.routeable_manifest() {
            return Ok(None);
        }
        let output = match self.run_commands(&[
            ".lang wikidata".into(),
            self.route_name_load_command(qid),
            format!(".node {qid}"),
            ".quit".into(),
            String::new(),
        ]) {
            Ok(output) => output,
            Err(error)
                if error
                    .to_string()
                    .contains("nodeRouteIndex resolved no matching chunks")
                    || error.to_string().contains("Unknown node") =>
            {
                return Ok(None);
            }
            Err(error) => return Err(error),
        };
        Ok(Self::parse_resolved_node_id(&output))
    }

    fn property_query(qid: &str, pid: &str) -> String {
        format!("sparql\nSELECT ?value WHERE {{ wd:{qid} wdt:{pid} ?value . }}")
    }

    fn parse_qids(section: &str) -> Vec<String> {
        let mut rows = Vec::new();
        for raw in section.lines() {
            let value = raw.trim().split('\t').next().unwrap_or("").trim();
            let qid = value.split_whitespace().next().unwrap_or("");
            if valid_qid(qid) && !rows.iter().any(|existing| existing == qid) {
                rows.push(qid.to_owned());
            }
        }
        rows.sort();
        rows
    }

    pub(crate) fn digest_rows(qid: &str, p31: &[String], p279: &[String], source_ref: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(b"gwb-zelph-hf-type-node:v1\0");
        for value in [qid, source_ref] {
            hasher.update((value.len() as u64).to_be_bytes());
            hasher.update(value.as_bytes());
        }
        for (property, rows) in [("P31", p31), ("P279", p279)] {
            for target in rows {
                for value in [property, target.as_str()] {
                    hasher.update((value.len() as u64).to_be_bytes());
                    hasher.update(value.as_bytes());
                }
            }
        }
        format!(
            "sha256:{}",
            hasher
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        )
    }
}

impl TypeClosureNodeProvider for ZelphHfTypeProvider {
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        if !valid_qid(qid) {
            return Err(TypeClosureError::InvalidQid(qid.to_owned()));
        }
        let Some(node_id) = self.resolve_route_node(qid)? else {
            // A local .bin or a manifest without a resolvable route is not an
            // admissible remote snapshot fast path. Returning a miss allows
            // the tiered provider to use the governed revision-pinned fallback
            // rather than silently loading all adjacency.
            return Ok(None);
        };
        let commands = [
            format!(".lang {}", self.language),
            self.route_node_load_command(qid, node_id),
            ".import sparql".into(),
            Self::property_query(qid, "P31"),
            String::new(),
            Self::property_query(qid, "P279"),
            String::new(),
            ".quit".into(),
            String::new(),
        ];
        let combined = self.run_commands(&commands)?;
        let sections = combined.split("?value\n").skip(1).collect::<Vec<_>>();
        let p31 = sections
            .first()
            .map(|section| Self::parse_qids(section.split("--").next().unwrap_or(section)))
            .unwrap_or_default();
        let p279 = sections
            .get(1)
            .map(|section| Self::parse_qids(section.split("--").next().unwrap_or(section)))
            .unwrap_or_default();
        if p31.is_empty() && p279.is_empty() {
            return Ok(None);
        }

        let source_revision_ref = format!("zelph-hf:{}:{}", self.snapshot_ref, qid);
        let evidence_digest_ref = Self::digest_rows(qid, &p31, &p279, &source_revision_ref);
        let mut observations = Vec::new();
        for target_qid in p31 {
            observations.push(TypeObservation {
                subject_qid: qid.to_owned(),
                property: TypeObservationProperty::InstanceOf,
                target_qid,
                source_revision_ref: source_revision_ref.clone(),
                evidence_digest_ref: evidence_digest_ref.clone(),
            });
        }
        for target_qid in p279 {
            observations.push(TypeObservation {
                subject_qid: qid.to_owned(),
                property: TypeObservationProperty::SubclassOf,
                target_qid,
                source_revision_ref: source_revision_ref.clone(),
                evidence_digest_ref: evidence_digest_ref.clone(),
            });
        }
        observations.sort();
        observations.dedup();
        Ok(Some(TypeNodeAcquisition {
            qid: qid.to_owned(),
            observations,
            node_receipt: TypeNodeReceipt {
                qid: qid.to_owned(),
                source_revision_ref,
                evidence_digest_ref,
            },
        }))
    }

    fn snapshot_simultaneous(&self) -> bool {
        true
    }

    fn stats(&self) -> TypeProviderStats {
        self.stats
    }
}

pub struct TieredTypeClosureProvider<S, L> {
    snapshot: S,
    live: L,
    stats: TypeProviderStats,
}

impl<S, L> TieredTypeClosureProvider<S, L> {
    #[must_use]
    pub fn new(snapshot: S, live: L) -> Self {
        Self {
            snapshot,
            live,
            stats: TypeProviderStats::default(),
        }
    }

    #[must_use]
    pub fn snapshot(&self) -> &S {
        &self.snapshot
    }

    #[must_use]
    pub fn live(&self) -> &L {
        &self.live
    }
}

impl<S, L> TypeClosureNodeProvider for TieredTypeClosureProvider<S, L>
where
    S: TypeClosureNodeProvider,
    L: TypeClosureNodeProvider,
{
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        if let Some(acquired) = self.snapshot.acquire_type_node(qid)? {
            self.stats.snapshot_hits = self.stats.snapshot_hits.saturating_add(1);
            return Ok(Some(acquired));
        }
        self.stats.live_fallbacks = self.stats.live_fallbacks.saturating_add(1);
        self.live.acquire_type_node(qid)
    }

    fn snapshot_simultaneous(&self) -> bool {
        self.stats.live_fallbacks == 0 && self.snapshot.snapshot_simultaneous()
    }

    fn stats(&self) -> TypeProviderStats {
        let snapshot = self.snapshot.stats();
        let live = self.live.stats();
        TypeProviderStats {
            provider_calls: snapshot.provider_calls.saturating_add(live.provider_calls),
            retries: snapshot.retries.saturating_add(live.retries),
            rate_limit_retries: snapshot
                .rate_limit_retries
                .saturating_add(live.rate_limit_retries),
            preferred_slice_hits: snapshot
                .preferred_slice_hits
                .saturating_add(live.preferred_slice_hits),
            snapshot_hits: self.stats.snapshot_hits,
            live_fallbacks: self.stats.live_fallbacks,
        }
    }
}

pub fn acquire_supervised_type_closure_with<P>(
    request: &TypeClosureRequest,
    provider: &mut P,
) -> Result<ObservedTypeClosure, TypeClosureError>
where
    P: TypeClosureNodeProvider,
{
    if !valid_qid(&request.root_qid) {
        return Err(TypeClosureError::InvalidQid(request.root_qid.clone()));
    }
    if request.max_depth == 0 {
        return Err(TypeClosureError::InvalidDepth);
    }
    if request.max_nodes == 0 {
        return Err(TypeClosureError::InvalidNodeLimit);
    }

    let mut queue = VecDeque::from([(request.root_qid.clone(), 0usize)]);
    let mut visited = BTreeSet::new();
    let mut observations = Vec::new();
    let mut node_receipts = Vec::new();
    let mut truncated = false;

    while let Some((qid, depth)) = queue.pop_front() {
        if visited.contains(&qid) {
            continue;
        }
        if visited.len() >= request.max_nodes {
            truncated = true;
            break;
        }
        visited.insert(qid.clone());

        let Some(acquired) = provider.acquire_type_node(&qid)? else {
            truncated = true;
            continue;
        };
        let rows = acquired.observations;
        node_receipts.push(acquired.node_receipt);

        let expand = depth < request.max_depth;
        for row in &rows {
            let should_follow = if qid == request.root_qid {
                matches!(
                    row.property,
                    TypeObservationProperty::InstanceOf | TypeObservationProperty::SubclassOf
                )
            } else {
                row.property == TypeObservationProperty::SubclassOf
            };
            if should_follow {
                if expand {
                    if !visited.contains(&row.target_qid) {
                        queue.push_back((row.target_qid.clone(), depth.saturating_add(1)));
                    }
                } else {
                    truncated = true;
                }
            }
        }
        observations.extend(rows);
    }

    if !queue.is_empty() {
        truncated = true;
    }

    let mut closure = evaluate_observed_type_closure(request, &observations, truncated)?;
    node_receipts.sort_by(|left, right| left.qid.cmp(&right.qid));
    node_receipts.dedup_by(|left, right| left.qid == right.qid);
    closure.node_receipts = node_receipts;
    closure.snapshot_simultaneous = provider.snapshot_simultaneous();
    closure.evidence_digest_ref = aggregate_digest(
        request,
        &closure.observations,
        &closure.node_receipts,
        closure.truncated,
    );
    closure.source_manifest_ref = format!(
        "wikidata-type-closure:{}:{}",
        request.root_qid, closure.evidence_digest_ref
    );
    Ok(closure)
}

/// Acquire one bounded multi-revision type-closure view.
///
/// Each visited entity is fetched at an exact revision and carries its own
/// digest. The aggregate is therefore reproducible as a manifest, but it is
/// deliberately *not* described as a simultaneous global Wikidata snapshot.

pub struct ThreeTierTypeClosureProvider<P, S, L> {
    preferred_slice: P,
    snapshot: S,
    live: L,
    stats: TypeProviderStats,
}

impl<P, S, L> ThreeTierTypeClosureProvider<P, S, L> {
    #[must_use]
    pub fn new(preferred_slice: P, snapshot: S, live: L) -> Self {
        Self {
            preferred_slice,
            snapshot,
            live,
            stats: TypeProviderStats::default(),
        }
    }
}

impl<P, S, L> TypeClosureNodeProvider for ThreeTierTypeClosureProvider<P, S, L>
where
    P: TypeClosureNodeProvider,
    S: TypeClosureNodeProvider,
    L: TypeClosureNodeProvider,
{
    fn acquire_type_node(
        &mut self,
        qid: &str,
    ) -> Result<Option<TypeNodeAcquisition>, TypeClosureError> {
        if let Some(acquired) = self.preferred_slice.acquire_type_node(qid)? {
            self.stats.preferred_slice_hits =
                self.stats.preferred_slice_hits.saturating_add(1);
            return Ok(Some(acquired));
        }
        if let Some(acquired) = self.snapshot.acquire_type_node(qid)? {
            self.stats.snapshot_hits = self.stats.snapshot_hits.saturating_add(1);
            return Ok(Some(acquired));
        }
        self.stats.live_fallbacks = self.stats.live_fallbacks.saturating_add(1);
        self.live.acquire_type_node(qid)
    }

    fn snapshot_simultaneous(&self) -> bool {
        self.stats.live_fallbacks == 0
            && self.preferred_slice.snapshot_simultaneous()
            && self.snapshot.snapshot_simultaneous()
    }

    fn stats(&self) -> TypeProviderStats {
        let preferred = self.preferred_slice.stats();
        let snapshot = self.snapshot.stats();
        let live = self.live.stats();
        TypeProviderStats {
            provider_calls: preferred
                .provider_calls
                .saturating_add(snapshot.provider_calls)
                .saturating_add(live.provider_calls),
            retries: preferred
                .retries
                .saturating_add(snapshot.retries)
                .saturating_add(live.retries),
            rate_limit_retries: preferred
                .rate_limit_retries
                .saturating_add(snapshot.rate_limit_retries)
                .saturating_add(live.rate_limit_retries),
            preferred_slice_hits: self.stats.preferred_slice_hits,
            snapshot_hits: self.stats.snapshot_hits,
            live_fallbacks: self.stats.live_fallbacks,
        }
    }
}

pub fn acquire_supervised_type_closure(
    request: &TypeClosureRequest,
) -> Result<ObservedTypeClosure, TypeClosureError> {
    let live = RevisionPinnedLiveTypeProvider::default();
    let preferred_slice = PredicateSliceTypeProvider::from_env()?;
    let snapshot = ZelphHfTypeProvider::from_env();

    match (preferred_slice, snapshot) {
        (Some(preferred_slice), Some(snapshot)) => {
            let mut provider =
                ThreeTierTypeClosureProvider::new(preferred_slice, snapshot, live);
            acquire_supervised_type_closure_with(request, &mut provider)
        }
        (Some(preferred_slice), None) => {
            let mut provider = TieredTypeClosureProvider::new(preferred_slice, live);
            acquire_supervised_type_closure_with(request, &mut provider)
        }
        (None, Some(snapshot)) => {
            let mut provider = TieredTypeClosureProvider::new(snapshot, live);
            acquire_supervised_type_closure_with(request, &mut provider)
        }
        (None, None) => {
            let mut provider = live;
            acquire_supervised_type_closure_with(request, &mut provider)
        }
    }
}

#[must_use]
pub fn type_closure_route_candidates(closure: &ObservedTypeClosure) -> Vec<RouteCandidate> {
    let mut rows = closure
        .observations
        .iter()
        .map(|observation| RouteCandidate {
            candidate_id: format!(
                "wikidata-type-closure:{}:{}:{}",
                observation.subject_qid,
                observation.property.property_ref(),
                observation.target_qid
            ),
            producer: ProducerFamily::ClassificationEvidence,
            route_family: RouteFamily::WikidataProperty,
            source_ref: observation.subject_qid.clone(),
            target_ref: observation.target_qid.clone(),
            property_ref: observation.property.property_ref().into(),
            cross_language_gap_coverage: 0,
            source_surface_support: 1,
            root_qid_support: if observation.subject_qid == closure.request.root_qid {
                1
            } else {
                0
            },
            typed_property_support: 1,
            route_specificity: 5,
            yield_history_observed: 1,
            prior_contracted_old_gaps: 0,
            prior_retired_obligations: 0,
            prior_new_gap_atoms: 0,
            prior_network_requests: 0,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.candidate_id.cmp(&right.candidate_id));
    rows.dedup_by(|left, right| left.candidate_id == right.candidate_id);
    rows
}

#[must_use]
pub fn render_type_closure_review_evidence(closure: &ObservedTypeClosure) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# supervised_type_closure_disposition={}\n",
        closure.disposition.as_str()
    ));
    out.push_str(&format!(
        "# supervised_type_closure_root={}\n",
        closure.request.root_qid
    ));
    out.push_str(&format!(
        "# supervised_type_closure_question={}\n",
        closure.request.question.as_str()
    ));
    out.push_str(&format!(
        "# supervised_type_closure_truncated={}\n",
        closure.truncated
    ));
    out.push_str(&format!(
        "# observed_direct_surface_complete={}\n",
        closure.observed_direct_surface_complete
    ));
    out.push_str(&format!(
        "# global_ontology_complete={}\n",
        closure.global_ontology_complete
    ));
    out.push_str(&format!(
        "# snapshot_simultaneous={}\n",
        closure.snapshot_simultaneous
    ));
    out.push_str(&format!(
        "# direct_instance_types={}\n",
        closure.direct_instance_types.join(",")
    ));
    out.push_str(&format!(
        "# direct_superclasses={}\n",
        closure.direct_superclasses.join(",")
    ));
    out.push_str(&format!(
        "# observed_instance_type_closure={}\n",
        closure.observed_instance_type_closure.join(",")
    ));
    out.push_str(&format!(
        "# observed_superclass_closure={}\n",
        closure.observed_superclass_closure.join(",")
    ));
    for receipt in &closure.node_receipts {
        out.push_str(&format!(
            "# type_closure_source\t{}\t{}\t{}\n",
            receipt.qid, receipt.source_revision_ref, receipt.evidence_digest_ref
        ));
    }
    out
}
