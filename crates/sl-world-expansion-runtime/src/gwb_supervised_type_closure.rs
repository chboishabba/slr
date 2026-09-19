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
use std::io::Cursor;

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
    let mut hasher = Sha256::new();
    hasher.update(b"gwb-supervised-type-closure:v1\0");
    for value in [
        request.root_qid.as_str(),
        request.question.as_str(),
        &request.max_depth.to_string(),
        &request.max_nodes.to_string(),
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

/// Acquire one bounded multi-revision type-closure view.
///
/// Each visited entity is fetched at an exact revision and carries its own
/// digest. The aggregate is therefore reproducible as a manifest, but it is
/// deliberately *not* described as a simultaneous global Wikidata snapshot.
pub fn acquire_supervised_type_closure(
    request: &TypeClosureRequest,
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

        let acquired = fetch_latest_entity_rdf_revision_receipt(&qid)?;
        let rows = type_observations_from_rdf(
            &qid,
            &acquired.source_revision_ref,
            &acquired.content_digest_ref,
            &acquired.rdf_bytes,
        )?;
        node_receipts.push(TypeNodeReceipt {
            qid: qid.clone(),
            source_revision_ref: acquired.source_revision_ref.clone(),
            evidence_digest_ref: acquired.content_digest_ref.clone(),
        });

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
