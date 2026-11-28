use crate::frontier::{ProofResidual, ResidualStatus};
use sensiblaw_core::canonical_evidence::{EvidenceObservation, EvidenceSpan, EvidenceSubstrateError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum GetterBackend { SlrNative, LeanInterop, Other }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalStatus { Retrieved, Missing, Failed }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessStatus { Current, Stale, Unknown }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceClass {
    RevisionPinnedExternalSource,
    RevisionPinnedFixture,
    DigestPinnedExternalSource,
    OtherCandidateSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldObservation {
    pub request_ref: String,
    pub object_ref: String,
    pub relation_ref: String,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub value_ref: String,
    pub retrieval_status: RetrievalStatus,
    pub freshness_status: FreshnessStatus,
    pub provenance_class: ProvenanceClass,
    pub backend: GetterBackend,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NormalizedWorldObservation {
    pub request_ref: String,
    pub object_ref: String,
    pub relation_ref: String,
    pub source_ref: String,
    pub source_revision_ref: String,
    pub content_digest_ref: String,
    pub value_ref: String,
    pub retrieval_status: RetrievalStatus,
    pub freshness_status: FreshnessStatus,
    pub provenance_class: ProvenanceClass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldObservationError { EmptyCoordinate, MustRemainCandidateOnly, ObservationMayNotPromote }

impl WorldObservation {
    pub fn validate(&self) -> Result<(), WorldObservationError> {
        if [self.request_ref.as_str(), self.object_ref.as_str(), self.relation_ref.as_str(), self.source_ref.as_str(), self.source_revision_ref.as_str(), self.content_digest_ref.as_str(), self.value_ref.as_str()]
            .iter().any(|value| value.trim().is_empty()) {
            return Err(WorldObservationError::EmptyCoordinate);
        }
        if !self.candidate_only { return Err(WorldObservationError::MustRemainCandidateOnly); }
        if self.creates_semantic_authority || self.claim_truth_promoted { return Err(WorldObservationError::ObservationMayNotPromote); }
        Ok(())
    }

    pub fn canonical_evidence_observation(
        &self,
    ) -> Result<EvidenceObservation, EvidenceSubstrateError> {
        let span_ref = format!(
            "span:structured:{}:{}:{}",
            self.source_revision_ref, self.object_ref, self.relation_ref
        );
        let coordinate_ref = format!(
            "structured-coordinate:{}:{}:{}",
            self.object_ref, self.relation_ref, self.value_ref
        );
        let span = EvidenceSpan::structured(
            self.source_revision_ref.clone(),
            span_ref,
            coordinate_ref,
        )?;
        let observation = EvidenceObservation {
            observation_ref: format!(
                "observation:{}:{}:{}",
                self.request_ref, self.relation_ref, self.value_ref
            ),
            source_revision_ref: self.source_revision_ref.clone(),
            span,
            predicate_ref: self.relation_ref.clone(),
            value_ref: self.value_ref.clone(),
            candidate_only: self.candidate_only,
            creates_semantic_authority: self.creates_semantic_authority,
            applicability_promoted: false,
            claim_truth_promoted: self.claim_truth_promoted,
        };
        observation.validate()?;
        Ok(observation)
    }

    #[must_use]
    pub fn normalized(&self) -> NormalizedWorldObservation {
        NormalizedWorldObservation {
            request_ref: self.request_ref.clone(), object_ref: self.object_ref.clone(), relation_ref: self.relation_ref.clone(), source_ref: self.source_ref.clone(),
            source_revision_ref: self.source_revision_ref.clone(), content_digest_ref: self.content_digest_ref.clone(), value_ref: self.value_ref.clone(),
            retrieval_status: self.retrieval_status, freshness_status: self.freshness_status, provenance_class: self.provenance_class,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MismatchKind {
    RequestMismatch, ObjectMismatch, RelationMismatch, SourceMismatch, RevisionMismatch,
    DigestMismatch, ValueMismatch, RetrievalMismatch, FreshnessMismatch, ProvenanceMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetterParityResidual {
    pub residual_ref: String,
    pub left_backend: GetterBackend,
    pub right_backend: GetterBackend,
    pub kind: MismatchKind,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationParity { Agreement, Residual(GetterParityResidual) }

fn mismatch_kind(left: &NormalizedWorldObservation, right: &NormalizedWorldObservation) -> Option<MismatchKind> {
    if left.request_ref != right.request_ref { return Some(MismatchKind::RequestMismatch); }
    if left.object_ref != right.object_ref { return Some(MismatchKind::ObjectMismatch); }
    if left.relation_ref != right.relation_ref { return Some(MismatchKind::RelationMismatch); }
    if left.source_ref != right.source_ref { return Some(MismatchKind::SourceMismatch); }
    if left.source_revision_ref != right.source_revision_ref { return Some(MismatchKind::RevisionMismatch); }
    if left.content_digest_ref != right.content_digest_ref { return Some(MismatchKind::DigestMismatch); }
    if left.value_ref != right.value_ref { return Some(MismatchKind::ValueMismatch); }
    if left.retrieval_status != right.retrieval_status { return Some(MismatchKind::RetrievalMismatch); }
    if left.freshness_status != right.freshness_status { return Some(MismatchKind::FreshnessMismatch); }
    if left.provenance_class != right.provenance_class { return Some(MismatchKind::ProvenanceMismatch); }
    None
}

#[must_use]
pub fn compare_observations(left: &WorldObservation, right: &WorldObservation) -> ObservationParity {
    let Some(kind) = mismatch_kind(&left.normalized(), &right.normalized()) else { return ObservationParity::Agreement; };
    ObservationParity::Residual(GetterParityResidual {
        residual_ref: format!("getter-parity:{}:{}:{:?}", left.request_ref, left.relation_ref, kind),
        left_backend: left.backend, right_backend: right.backend, kind,
        candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
    })
}

/// Convert an observation disagreement into the same open `ProofResidual`
/// surface consumed by the P7 frontier. Agreement creates no work. This is a
/// candidate diagnostic residual, never a truth judgment about either backend.
#[must_use]
pub fn getter_parity_to_proof_residual(
    parity: &ObservationParity,
    proposition_ref: impl Into<String>,
    jurisdiction_ref: Option<&str>,
    salience: u64,
) -> Option<ProofResidual> {
    let ObservationParity::Residual(residual) = parity else { return None; };
    Some(ProofResidual {
        residual_ref: residual.residual_ref.clone(),
        proposition_ref: proposition_ref.into(),
        producer_class_ref: "producer:getter-parity".into(),
        jurisdiction_ref: jurisdiction_ref.map(ToOwned::to_owned),
        authority_requirement_ref: None,
        salience,
        dependency_refs: vec![format!("backend:{:?}", residual.left_backend), format!("backend:{:?}", residual.right_backend)],
        status: ResidualStatus::Open,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mabo_observation(backend: GetterBackend, value_ref: &str) -> WorldObservation {
        WorldObservation {
            request_ref: "query:mabo:P710".into(), object_ref: "Q1501525".into(), relation_ref: "P710".into(), source_ref: "wikidata".into(),
            source_revision_ref: "wikidata:Q1501525:oldid:2333409615".into(), content_digest_ref: "sha256:mabo-fixture".into(), value_ref: value_ref.into(),
            retrieval_status: RetrievalStatus::Retrieved, freshness_status: FreshnessStatus::Current,
            provenance_class: ProvenanceClass::RevisionPinnedExternalSource, backend,
            candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
        }
    }

    #[test]
    fn same_mabo_property_from_two_backends_agrees_after_normalization() {
        assert_eq!(compare_observations(&mabo_observation(GetterBackend::SlrNative, "Q975866"), &mabo_observation(GetterBackend::LeanInterop, "Q975866")), ObservationParity::Agreement);
    }

    #[test]
    fn backend_identity_does_not_change_observation_semantics() {
        let slr = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let lean = mabo_observation(GetterBackend::LeanInterop, "Q975866");
        assert_ne!(slr.backend, lean.backend);
        assert_eq!(slr.normalized(), lean.normalized());
    }

    #[test]
    fn mismatched_value_becomes_parity_residual_not_truth_judgment() {
        let parity = compare_observations(&mabo_observation(GetterBackend::SlrNative, "Q975866"), &mabo_observation(GetterBackend::LeanInterop, "Q36074"));
        assert!(matches!(parity, ObservationParity::Residual(GetterParityResidual { kind: MismatchKind::ValueMismatch, .. })));
    }

    #[test]
    fn nat_climate_fixture_can_use_same_observation_abi() {
        let observation = WorldObservation {
            request_ref: "query:nat-climate:p5991-p14143".into(), object_ref: "Q10884".into(), relation_ref: "migration:P5991->P14143".into(),
            source_ref: "sensiblaw:nat-climate-fixture".into(), source_revision_ref: "provided_snapshot_2026-04-01".into(),
            content_digest_ref: "fixture-content-hash-not-pinned-in-this-owner".into(), value_ref: "review-required".into(),
            retrieval_status: RetrievalStatus::Retrieved, freshness_status: FreshnessStatus::Unknown,
            provenance_class: ProvenanceClass::RevisionPinnedFixture, backend: GetterBackend::SlrNative,
            candidate_only: true, creates_semantic_authority: false, claim_truth_promoted: false,
        };
        assert!(observation.validate().is_ok());
        assert!(!observation.creates_semantic_authority);
        assert!(!observation.claim_truth_promoted);
    }
    #[test]
    fn world_observation_lowers_to_structured_canonical_observation() {
        use sensiblaw_core::canonical_evidence::EvidenceSpanKind;
        let observation = mabo_observation(GetterBackend::SlrNative, "Q975866");
        let canonical = observation.canonical_evidence_observation().unwrap();
        assert_eq!(canonical.source_revision_ref, observation.source_revision_ref);
        assert_eq!(canonical.predicate_ref, "P710");
        assert_eq!(canonical.value_ref, "Q975866");
        assert!(matches!(
            canonical.span.kind,
            EvidenceSpanKind::StructuredCoordinate { .. }
        ));
        assert!(canonical.validate().is_ok());
    }

}

/** An original member and its exact inverse into a checked inventory.
 * This is an adapter for the existing observation ABI, not an authority
 * validator. The caller must independently reopen the original checker.
 */
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedObservationMember {
    pub observation: WorldObservation,
    pub lane: String,
    pub ordinal: usize,
    pub original: serde_json::Value,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedObservationInventory {
    pub members: Vec<IndexedObservationMember>,
    pub unavailable_lanes: Vec<String>,
    pub empty_lanes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndexedObservationError { InvalidReceipt, InvalidLane, InvalidMember, InvalidObservation }

/// Read a recorded inventory as candidate-only observations. Each row
/// gets a content-addressed value and a distinct occurrence coordinate;
/// duplicate values never erase original members. No source flags alone
/// pay formal authority, semantic identity, or world truth.
pub fn indexed_inventory_observations(receipt: &serde_json::Value)
    -> Result<IndexedObservationInventory, IndexedObservationError>
{
    use serde_json::Value;
    use sha2::{Digest,Sha256};
    let bad=IndexedObservationError::InvalidReceipt;
    if receipt["schema"]!="slr.jmd-indexed-world-fibre-receipt.v1"
        || receipt["source_observation_checked"]!=true
        || receipt["cross_source_alignment_paid"]!=false
        || receipt["world_truth_paid"]!=false {return Err(bad);}
    let inv=&receipt["inventory"];
    if inv["schema"]!="slr.jmd-indexed-world-fibre.v1"
        || inv["semantic_coverage_paid"]!=false
        || inv["cross_source_alignment_paid"]!=false
        || inv["world_truth_paid"]!=false {return Err(bad);}
    let qid=inv["qid"].as_str().filter(|s|s.starts_with('Q')&&s.len()>1&&s[1..].bytes().all(|b|b.is_ascii_digit())).ok_or(IndexedObservationError::InvalidReceipt)?;
    if receipt["qid"]!=qid || receipt["title"]!=inv["title"] {return Err(IndexedObservationError::InvalidReceipt);}
    let article_revision=inv["article_revision"].as_str().filter(|s|!s.trim().is_empty()).ok_or(IndexedObservationError::InvalidReceipt)?;
    let entity_revision=receipt["entity_revision"].as_u64().filter(|v|*v>0).ok_or(IndexedObservationError::InvalidReceipt)?;
    let digest=|key:&str| -> Result<String,IndexedObservationError>{
        receipt[key].as_str().filter(|s|s.len()==64&&s.bytes().all(|b|b.is_ascii_hexdigit()))
            .map(|s|format!("sha256:{}",s.to_ascii_lowercase())).ok_or(IndexedObservationError::InvalidReceipt)
    };
    let article_digest=digest("source_sha256")?;
    let entity_digest=digest("entity_sha256")?;
    let projection_digest=digest("projection_sha256")?;
    let original_slr_digest=digest("original_slr_sha256")?;
    let entity_revision=format!("wikidata:{qid}:oldid:{entity_revision}");
    let mut out=IndexedObservationInventory{members:Vec::new(),unavailable_lanes:Vec::new(),empty_lanes:Vec::new()};
    for (chart,revision,source_digest) in [
        ("article_lanes",article_revision,article_digest.as_str()),
        ("wikidata_lanes",entity_revision.as_str(),entity_digest.as_str())
    ]{
        let lanes=inv[chart].as_object().ok_or(IndexedObservationError::InvalidLane)?;
        for (lane,value) in lanes{
            if lane.trim().is_empty(){return Err(IndexedObservationError::InvalidLane);}
            let lane_ref=format!("{chart}:{lane}");
            if value.is_null(){out.unavailable_lanes.push(lane_ref);continue;}
            let rows=value.as_array().ok_or(IndexedObservationError::InvalidLane)?;
            if rows.is_empty(){out.empty_lanes.push(lane_ref);continue;}
            for (ordinal,row) in rows.iter().enumerate(){
                if !row.is_object(){return Err(IndexedObservationError::InvalidMember);}
                let bytes=serde_json::to_vec(row).map_err(|_|IndexedObservationError::InvalidMember)?;
                let value_ref=format!("sha256:{:x}",Sha256::digest(bytes));
                let chart_digest=if chart=="article_lanes"{
                    if lane=="pnf_candidates"||lane=="named_entities"{&original_slr_digest}else{&projection_digest}
                }else{&entity_digest};
                let obs=WorldObservation{
                    request_ref:format!("indexed-member:{qid}:{revision}:{chart_digest}:{lane}:{ordinal}"),
                    object_ref:qid.to_owned(),relation_ref:lane_ref.clone(),
                    source_ref:if chart=="article_lanes"{format!("wikipedia:en:{qid}")}else{format!("wikidata:{qid}")},
                    source_revision_ref:revision.to_owned(),content_digest_ref:source_digest.to_owned(),
                    value_ref,retrieval_status:RetrievalStatus::Retrieved,freshness_status:FreshnessStatus::Unknown,
                    provenance_class:ProvenanceClass::RevisionPinnedExternalSource,backend:GetterBackend::LeanInterop,
                    candidate_only:true,creates_semantic_authority:false,claim_truth_promoted:false,
                };
                obs.validate().map_err(|_|IndexedObservationError::InvalidObservation)?;
                out.members.push(IndexedObservationMember{observation:obs,lane:lane_ref.clone(),ordinal,original:row.clone()});
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod indexed_inventory_tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn preserves_duplicate_occurrences_and_unavailable_vs_empty_lanes(){
        let h="a".repeat(64);
        let mut r=json!({"schema":"slr.jmd-indexed-world-fibre-receipt.v1","qid":"Q9","title":"Example",
            "source_observation_checked":true,"cross_source_alignment_paid":false,"world_truth_paid":false,
            "source_sha256":h,"original_slr_sha256":h,"projection_sha256":h,"entity_sha256":h,"entity_revision":42,
            "inventory":{"schema":"slr.jmd-indexed-world-fibre.v1","qid":"Q9","title":"Example","article_revision":"enwiki-revid:23",
            "article_lanes":{"event_candidates":[{"x":1},{"x":1}],"named_entities":null,"time_mentions":[]},
            "wikidata_lanes":{"P31":[{"id":"Q9$example"}]},"semantic_coverage_paid":false,
            "cross_source_alignment_paid":false,"world_truth_paid":false}});
        let out=indexed_inventory_observations(&r).unwrap();
        assert_eq!(out.members.len(),3);
        assert_eq!(out.members[0].observation.value_ref,out.members[1].observation.value_ref);
        assert_ne!(out.members[0].observation.request_ref,out.members[1].observation.request_ref);
        assert_eq!(out.unavailable_lanes,vec!["article_lanes:named_entities"]);
        assert_eq!(out.empty_lanes,vec!["article_lanes:time_mentions"]);
        assert_eq!(out.members[2].observation.source_revision_ref,"wikidata:Q9:oldid:42");
        r["world_truth_paid"]=json!(true);
        assert!(indexed_inventory_observations(&r).is_err());
    }
}
