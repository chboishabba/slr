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

/// A recorded inventory may expose missing observation lanes without
/// implying a missing fact. Only explicitly demanded unavailable lanes
/// become open producer residuals. An observed empty lane is not a denial.
pub fn indexed_inventory_frontier(inventory:&IndexedObservationInventory,
    consumer_ref:&str, frontier_ref:&str, required_lanes:&[String])
    ->Result<crate::frontier::ProofFrontier,IndexedObservationError>{
    use crate::frontier::ProofFrontier;
    use std::collections::BTreeSet;
    if consumer_ref.trim().is_empty()||frontier_ref.trim().is_empty(){
        return Err(IndexedObservationError::InvalidObservation);
    }
    let required:BTreeSet<&str>=required_lanes.iter().map(String::as_str).collect();
    if required.len()!=required_lanes.len()||required.iter().any(|k|k.trim().is_empty()){
        return Err(IndexedObservationError::InvalidLane);
    }
    let present:BTreeSet<&str>=inventory.members.iter().map(|m|m.lane.as_str())
        .chain(inventory.empty_lanes.iter().map(String::as_str)).collect();
    let residuals=required.into_iter().filter(|lane|!present.contains(lane))
        .map(|lane|ProofResidual{
            residual_ref:format!("{frontier_ref}:unobserved:{lane}"),
            proposition_ref:format!("recorded-lane:{lane}"),
            producer_class_ref:"producer:indexed-source-observation".into(),
            jurisdiction_ref:None,authority_requirement_ref:None,salience:1,
            dependency_refs:vec![format!("consumer:{consumer_ref}"),format!("lane:{lane}")],
            status:ResidualStatus::Open,
        }).collect();
    Ok(ProofFrontier{consumer_ref:consumer_ref.into(),frontier_ref:frontier_ref.into(),
        residuals,satisfied_payment_refs:vec![],contested_coordinate_refs:vec![],
        authority_blocked_refs:vec![],authority:"experimental_candidate_only"})
}

/// Native read-only inspection of normalized members and their inverse.
pub fn inspect_indexed_inventory(receipt:&serde_json::Value,consumer:&str,
    required_lanes:&[String])->Result<serde_json::Value,IndexedObservationError>{
    use serde_json::json;
    use sha2::{Digest,Sha256};
    let inventory=indexed_inventory_observations(receipt)?;
    let digest=format!("sha256:{:x}",Sha256::digest(
        serde_json::to_vec(receipt).map_err(|_|IndexedObservationError::InvalidReceipt)?));
    let frontier=indexed_inventory_frontier(&inventory,consumer,&format!("frontier:{digest}"),required_lanes)?;
    let members:Vec<_>=inventory.members.iter().map(|m|{
        let o=m.observation.normalized();
        json!({"request_ref":o.request_ref,"object_ref":o.object_ref,"relation_ref":o.relation_ref,
            "source_ref":o.source_ref,"source_revision_ref":o.source_revision_ref,
            "content_digest_ref":o.content_digest_ref,"value_ref":o.value_ref,
            "retrieval_status":"retrieved","freshness_status":"unknown",
            "lane":m.lane,"ordinal":m.ordinal,"original":m.original})
    }).collect();
    let residuals:Vec<_>=frontier.open_residuals().map(|r|json!({
        "residual_ref":r.residual_ref,"proposition_ref":r.proposition_ref,
        "producer_class_ref":r.producer_class_ref,"dependency_refs":r.dependency_refs,
        "status":"open"})).collect();
    Ok(json!({"schema":"slr.indexed-observation-inspection.v1","receipt_sha256":digest,
        "consumer_ref":consumer,"members":members,"unavailable_lanes":inventory.unavailable_lanes,
        "empty_lanes":inventory.empty_lanes,"required_lanes":required_lanes,
        "frontier_ref":frontier.frontier_ref,"residuals":residuals,
        "candidate_only":true,"original_checker_replayed":false,
        "semantic_coverage_paid":false,"world_truth_paid":false,"writes_database":false}))
}

/// Compare the same source manifestation across the JMD event inventory and
/// native token/role coordinate chart. Group by literal source occurrence,
/// never by name, lemma, row hash, or claimed world identity. The native
/// chart includes nonpredicate tokens, so native-only rows are restricted
/// to actual checked or numeric predicate heads.
pub fn inspect_indexed_native_predicate_meet(receipt:&serde_json::Value,
    native:&serde_json::Value,consumer:&str)->Result<serde_json::Value,IndexedObservationError>{
    use serde_json::{json,Value};
    use sha2::{Digest,Sha256};
    use std::collections::{BTreeMap,BTreeSet};
    let bad=||IndexedObservationError::InvalidReceipt;
    let inv=indexed_inventory_observations(receipt)?;
    if consumer.trim().is_empty(){return Err(bad());}
    let meet=&native["source_coordinate_meet"];
    if native["schema"]!="slr.predicate-role-transport.v1"
        ||native["semantic_coverage_paid"]!=false||native["world_truth_paid"]!=false
        ||meet["schema"]!="slr.article-numeric-coordinate-meet.v1"
        ||meet["source_revision_ref"]!=receipt["inventory"]["article_revision"]
        ||meet["source_content_digest_ref"]!=format!("sha256:{}",receipt["source_sha256"].as_str().ok_or_else(bad)?)
        ||meet["common_current_authority_snapshot"]!=false{return Err(bad());}
    type Key=(u64,u64,String);
    let key=|a:&Value,b:&Value,t:&Value|->Result<Key,IndexedObservationError>{
        let a=a.as_u64().ok_or_else(bad)?;
        let b=b.as_u64().ok_or_else(bad)?;
        let t=t.as_str().ok_or_else(bad)?;
        if a>=b||t.chars().count() as u64!=b-a{return Err(bad());}
        Ok((a,b,t.to_owned()))
    };
    let mut groups:BTreeMap<Key,(Vec<Value>,Vec<Value>)>=BTreeMap::new();
    for m in inv.members.iter().filter(|m|m.lane=="article_lanes:event_candidates"){
        let p=&m.original["predicate"];
        let k=key(&p["start_char"],&p["end_char"],&p["surface"])?;
        groups.entry(k).or_default().0.push(json!({"request_ref":m.observation.request_ref,
            "ordinal":m.ordinal,"original":m.original}));
    }
    let mut checked=BTreeMap::<(String,String),Key>::new();
    let mut tokens=BTreeMap::<i64,Key>::new();
    for g in meet["coordinates"].as_array().ok_or_else(bad)?{
        let k=key(&g["span"][0],&g["span"][1],&g["literal"])?;
        for c in g["checked"].as_array().ok_or_else(bad)?{
            let id=(c["batch_ref"].as_str().ok_or_else(bad)?.to_owned(),
                c["factor_ref"].as_str().ok_or_else(bad)?.to_owned());
            if checked.insert(id,k.clone()).is_some(){return Err(bad());}
        }
        for n in g["numeric"].as_array().ok_or_else(bad)?{
            if tokens.insert(n["numeric_token_id"].as_i64().ok_or_else(bad)?,k.clone()).is_some(){return Err(bad());}
        }
    }
    for row in native["checked_predicates"].as_array().ok_or_else(bad)?{
        let id=(row["batch_ref"].as_str().ok_or_else(bad)?.to_owned(),
            row["checked_observation"]["predicate_candidate_ref"].as_str().ok_or_else(bad)?.to_owned());
        let k=checked.get(&id).ok_or_else(bad)?;
        groups.entry(k.clone()).or_default().1.push(json!({"kind":"checked_predicate","original":row}));
    }
    for row in native["numeric_only_predicates"].as_array().ok_or_else(bad)?{
        let heads:Vec<_>=row["role_inverse"].as_array().ok_or_else(bad)?.iter()
            .filter(|r|r["original_role"]["role"]=="head").collect();
        if heads.len()!=1{return Err(bad());}
        let token=heads[0]["numeric_token_id"].as_i64().ok_or_else(bad)?;
        let k=tokens.get(&token).ok_or_else(bad)?;
        groups.entry(k.clone()).or_default().1.push(json!({"kind":"numeric_only_predicate","original":row}));
    }
    let digest=format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(&(receipt,native))
        .map_err(|_|bad())?));
    let frontier_ref=format!("frontier:predicate-coordinate:{digest}");
    let mut residuals=Vec::new();
    let mut rows=Vec::new();
    for (i,(k,(left,right))) in groups.into_iter().enumerate(){
        let status=match (left.len(),right.len()){
            (1,1)=>"unique_coordinate_match",(_,0)=>"indexed_only",
            (0,_)=>"native_only",_=>"ambiguous_coordinate_match"};
        if status!="unique_coordinate_match"{
            residuals.push(ProofResidual{residual_ref:format!("{frontier_ref}:{i}"),
                proposition_ref:format!("predicate-occurrence:{}:{}",k.0,k.1),
                producer_class_ref:"producer:source-coordinate-reconciliation".into(),
                jurisdiction_ref:None,authority_requirement_ref:None,salience:1,
                dependency_refs:vec![format!("source:{}",meet["source_content_digest_ref"].as_str().ok_or_else(bad)?),
                    format!("coordinate:{}:{}",k.0,k.1),format!("status:{status}")],
                status:ResidualStatus::Open});
        }
        rows.push(json!({"span":[k.0,k.1],"literal":k.2,"indexed":left,"native":right,
            "status":status,"semantic_alignment_paid":false}));
    }
    let frontier=crate::frontier::ProofFrontier{consumer_ref:consumer.into(),frontier_ref,
        residuals,satisfied_payment_refs:vec![],contested_coordinate_refs:vec![],
        authority_blocked_refs:vec![],authority:"experimental_candidate_only"};
    let open:Vec<_>=frontier.open_residuals().map(|r|json!({
        "residual_ref":r.residual_ref,"proposition_ref":r.proposition_ref,
        "producer_class_ref":r.producer_class_ref,"dependency_refs":r.dependency_refs,
        "status":"open"})).collect();
    let outside_lanes:BTreeSet<_>=inv.members.iter().filter(|m|m.lane!="article_lanes:event_candidates")
        .map(|m|m.lane.clone()).collect();
    Ok(json!({"schema":"slr.indexed-native-predicate-meet.v1","consumer_ref":consumer,
        "source_revision_ref":meet["source_revision_ref"],"source_content_digest_ref":meet["source_content_digest_ref"],
        "frontier_ref":frontier.frontier_ref,"coordinates":rows,"residuals":open,
        "other_recorded_lanes":outside_lanes,"candidate_only":true,"original_checker_replayed":false,
        "semantic_coverage_paid":false,"world_truth_paid":false,"writes_database":false}))
}

/// Heterogeneous corpus retrieval, deliberately not source-coordinate parity.
 /// Inverted indexes retain a factorised many-to-many candidate relation;
 /// no Cartesian pair explosion and no implied semantic correspondence.
// A common *syntactic* vocabulary, not semantic Actor/Patient. In particular
// passive subjects never share the active-subject feature. Original labels
// remain in the inverse row; unsupported labels simply have no common key.
fn overlap_dependency_class(dep:&str)->Option<&'static str>{
    match dep.to_ascii_lowercase().as_str(){
        "nsubj"=>Some("nominal_subject"),
        "nsubjpass"|"nsubj:pass"=>Some("passive_nominal_subject"),
        "csubj"=>Some("clausal_subject"),
        "csubjpass"|"csubj:pass"=>Some("passive_clausal_subject"),
        "dobj"|"obj"=>Some("direct_object"),
        "iobj"=>Some("indirect_object"),
        "pobj"=>Some("prepositional_object"),
        "attr"=>Some("attribute"),
        "oprd"=>Some("object_predicative"),
        "agent"=>Some("agent_dependency"),
        "obl"=>Some("oblique"),
        "ccomp"=>Some("content_clause"),
        "xcomp"=>Some("open_complement"),
        "neg"=>Some("negation_dependency"),
        _=>None,
    }
}

fn dependent_overlap_features(head:&serde_json::Value,
    dependents:&[(&str,&serde_json::Value)])->Vec<(String,String)>{
    use std::collections::BTreeSet;
    let Some(lemma)=head["lemma"].as_str().filter(|s|!s.is_empty())else{return vec![]};
    let lemma=lemma.to_lowercase();
    let mut features=BTreeSet::new();
    for (dep,atom) in dependents{
        let Some(role)=overlap_dependency_class(dep)else{continue};
        let encode=|parts:&[&str]|serde_json::to_string(parts).expect("string tuple serialization");
        features.insert(("predicate_dependency".to_owned(),encode(&[&lemma,role])));
        if let Some(surface)=atom["surface"].as_str().or_else(||atom["text"].as_str()).filter(|s|!s.is_empty()){
            features.insert(("predicate_dependency_surface".to_owned(),encode(&[&lemma,role,surface])));
        }
        if let Some(value)=atom["lemma"].as_str().filter(|s|!s.is_empty()){
            features.insert(("predicate_dependency_lemma".to_owned(),encode(&[&lemma,role,&value.to_lowercase()])));
        }
    }
    features.into_iter().collect()
}

pub fn inspect_heterogeneous_predicate_overlap(receipt:&serde_json::Value,
    native:&serde_json::Value,consumer:&str)->Result<serde_json::Value,IndexedObservationError>{
    use serde_json::{json,Value};
    use std::collections::{BTreeMap,BTreeSet};
    use sha2::{Digest,Sha256};
    let bad=||IndexedObservationError::InvalidReceipt;
    let inv=indexed_inventory_observations(receipt)?;
    if consumer.trim().is_empty() || native["schema"]!="slr.original-relation-query.v1"
        ||native["query_relation_type"]!="predicate"||native["exact_persisted_packet_readback"]!=true
        ||native["complete_recorded_query"]!=true||native["all_live_semantic_readings_covered"]!=false
        ||native["governed_identity_paid"]!=false||native["world_truth_paid"]!=false{return Err(bad());}
    let rev=native["source_revision_ref"].as_str().filter(|x|!x.is_empty()).ok_or_else(bad)?;
    let sha=native["source_sha256"].as_str().filter(|x|x.len()==64&&x.bytes().all(|b|b.is_ascii_hexdigit())).ok_or_else(bad)?;
    if native["document_ref"]!=format!("document:sha256:{sha}"){return Err(bad());}
    // Source revisions and bytes are separate. Equal bytes at different
    // revisions are not sufficient to establish identical manifestation.
    let left_rev=receipt["inventory"]["article_revision"].as_str().ok_or_else(bad)?;
    let left_sha=receipt["source_sha256"].as_str().ok_or_else(bad)?;
    if rev==left_rev && sha==left_sha{return Err(bad());}
    let rows=native["answers"].as_array().ok_or_else(bad)?;
    if native["answer_count"].as_u64()!=Some(rows.len() as u64)
        ||native["relation_type_inventory"]["predicate"].as_u64().unwrap_or(0)!=rows.len() as u64{return Err(bad());}
    let lane="article_lanes:event_candidates";
    let left_available=!inv.unavailable_lanes.iter().any(|s|s==lane)
        &&receipt["inventory"]["article_lanes"].get("event_candidates").is_some();
    let mut left=Vec::new();
    let mut right=Vec::new();
    // Separate surface and lemma channels, case-sensitive original surface,
    // lowercase lemma diagnostic only. Role shape retains parser-local labels.
    let features=|head:&Value| -> Vec<(String,String)> {
        let mut out=Vec::new();
        if let Some(v)=head["surface"].as_str().or_else(||head["text"].as_str()).filter(|v|!v.is_empty()){
            out.push(("literal_surface".into(),v.into()));
        }
        if let Some(v)=head["lemma"].as_str().filter(|v|!v.is_empty()){
            out.push(("predicate_lemma".into(),v.to_lowercase()));
        }
        out
    };
    let mut index=BTreeMap::<(String,String),(Vec<usize>,Vec<usize>)>::new();
    for m in inv.members.iter().filter(|m|m.lane==lane){
        let head=&m.original["predicate"];
        let mut keys=features(head);
        let mut dependents=Vec::new();
        for lane in ["actors","patients"]{
            if let Some(atoms)=m.original[lane].as_array(){
                for atom in atoms{
                    if let Some(dep)=atom["dep"].as_str(){dependents.push((dep,atom));}
                }
            }
        }
        keys.extend(dependent_overlap_features(head,&dependents));
        let i=left.len();
        for k in &keys{index.entry(k.clone()).or_default().0.push(i);}
        left.push(json!({"member_ref":m.observation.request_ref,"ordinal":m.ordinal,
            "features":keys,"original":m.original}));
    }
    let mut seen=BTreeSet::new();
    for row in rows{
        if row["original_relation"]["type"]!="predicate"{return Err(bad());}
        let id=row["relation_ref"].as_str().filter(|s|!s.is_empty()).ok_or_else(bad)?;
        if !seen.insert(id){return Err(bad());}
        let roles=row["role_inverse"].as_array().ok_or_else(bad)?;
        let heads:Vec<_>=roles.iter().filter(|r|r["original_role"]["role"]=="head").collect();
        if heads.len()!=1{return Err(bad());}
        let head=&heads[0]["original_atom"];
        let mut keys=features(head);
        let dependents:Vec<_>=roles.iter().filter(|r|r["original_role"]["role"]!="head")
            .filter_map(|r|{
                let atom=&r["original_atom"];
                atom["dependency"].as_str().map(|dep|(dep,atom))
            }).collect();
        keys.extend(dependent_overlap_features(head,&dependents));
        let i=right.len();
        for k in &keys{index.entry(k.clone()).or_default().1.push(i);}
        right.push(json!({"member_ref":id,"ordinal":i,"features":keys,"original":row}));
    }
    let mut matched_l=BTreeMap::<String,BTreeSet<usize>>::new();
    let mut matched_r=BTreeMap::<String,BTreeSet<usize>>::new();
    let mut ambiguity_l=BTreeMap::<String,BTreeSet<usize>>::new();
    let mut ambiguity_r=BTreeMap::<String,BTreeSet<usize>>::new();
    let mut groups=Vec::new();
    for ((axis,value),(l,r)) in index{
        if !l.is_empty()&&!r.is_empty(){
            matched_l.entry(axis.clone()).or_default().extend(l.iter().copied());
            matched_r.entry(axis.clone()).or_default().extend(r.iter().copied());
            if l.len()!=1||r.len()!=1{
                ambiguity_l.entry(axis.clone()).or_default().extend(l.iter().copied());
                ambiguity_r.entry(axis.clone()).or_default().extend(r.iter().copied());
            }
        }
        let status=match(l.len(),r.len()){(1,1)=>"singleton_candidate",(_,0)=>"left_feature_only",
            (0,_)=>"right_feature_only",_=>"ambiguous_candidates"};
        groups.push(json!({"axis":axis,"value":value,"left_members":l,"right_members":r,
            "status":status,"semantic_alignment_paid":false}));
    }
    let mut coverage=Vec::new();
    for axis in ["literal_surface","predicate_lemma","predicate_dependency","predicate_dependency_surface","predicate_dependency_lemma"]{
        let axis_members=|members:&Vec<Value>| members.iter().enumerate()
            .filter(|(_,m)|m["features"].as_array().is_some_and(|fs|fs.iter().any(|f|f[0]==axis)))
            .map(|(i,_)|i).collect::<BTreeSet<_>>();
        let lu=axis_members(&left);let ru=axis_members(&right);
        let lm=matched_l.remove(axis).unwrap_or_default();let rm=matched_r.remove(axis).unwrap_or_default();
        let fraction=|n:usize,d:usize,available:bool|if available&&d>0{json!(n as f64/d as f64)}else{Value::Null};
        coverage.push(json!({"axis":axis,"left_observed_members":left.len(),"right_observed_members":right.len(),
            "left_feature_members":lu.len(),"right_feature_members":ru.len(),
            "left_matched_members":lm.len(),"right_matched_members":rm.len(),
            "left_coverage":fraction(lm.len(),left.len(),left_available),
            "right_coverage":fraction(rm.len(),right.len(),left_available),
            "left_unmatched":lu.difference(&lm).collect::<Vec<_>>(),
            "right_unmatched":ru.difference(&rm).collect::<Vec<_>>(),
            "left_feature_unavailable":(0..left.len()).filter(|i|!lu.contains(i)).collect::<Vec<_>>(),
            "right_feature_unavailable":(0..right.len()).filter(|i|!ru.contains(i)).collect::<Vec<_>>(),
            "left_ambiguous":ambiguity_l.remove(axis).unwrap_or_default(),
            "right_ambiguous":ambiguity_r.remove(axis).unwrap_or_default()}));
    }
    let digest=|v:&Value| -> Result<String,IndexedObservationError>{
        Ok(format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(v).map_err(|_|bad())?)))
    };
    Ok(json!({"schema":"slr.heterogeneous-predicate-overlap.v1","consumer_ref":consumer,
        "left_receipt_digest":digest(receipt)?,"right_receipt_digest":digest(native)?,
        "left_source":{"revision":left_rev,"sha256":left_sha,"lane_available":left_available,
            "recorded_scope":"indexed_event_candidates"},
        "right_source":{"revision":rev,"sha256":sha,"scope":native["scope"],
            "prefix_end_char":native["prefix_end_char"],"packet_sha256":native["packet_sha256"],
            "numeric_run_ref":native["numeric_run_ref"],"document_ref":native["document_ref"],
            "snapshot_ref":native["snapshot_ref"],"missing_original_tags":native["missing_original_tags"],
            "prefix_uncovered_nonwhitespace":native["prefix_uncovered_nonwhitespace"]},
        "left_members":left,"right_members":right,"feature_groups":groups,"directional_coverage":coverage,
        "candidate_relation_encoding":"factorised_by_feature; indices address side member arrays",
        "dependent_feature_policy":"predicate lemma plus normalized original syntactic dependency, optionally dependent surface or lemma; not semantic role alignment",
        "ambiguity_scope":"per feature group; singleton is not a unique global event correspondence",
        "same_manifestation_paid":false,"candidate_only":true,"source_independence_paid":false,
        "semantic_coverage_paid":false,"governed_identity_paid":false,"world_truth_paid":false,
        "licensed_crosswalk_count":0,"original_checker_replayed":false,"writes_database":false}))
}


/// Reverse-solve one selected heterogeneous candidate through the two
/// original source charts. This consumes the existing feature index and
/// exact indexed/native coordinate meet, not another similarity engine.
/// A common feature is a retrieval witness only. The selected pair cannot
/// create a SourceQueryLicence, semantic alignment or source independence.
pub fn select_heterogeneous_native_pair(receipt:&serde_json::Value,
    article_transport:&serde_json::Value, other_transport:&serde_json::Value,
    other_predicates:&serde_json::Value,consumer:&str,
    indexed_member:&str,other_relation:&str)->Result<serde_json::Value,IndexedObservationError>{
    use serde_json::{json,Value};
    use sha2::{Digest,Sha256};
    let bad=||IndexedObservationError::InvalidReceipt;
    if indexed_member.trim().is_empty()||other_relation.trim().is_empty(){return Err(bad());}
    let overlap=inspect_heterogeneous_predicate_overlap(receipt,other_predicates,consumer)?;
    let article=inspect_indexed_native_predicate_meet(receipt,article_transport,consumer)?;
    if article_transport["source_coordinate_meet"]["native_source_family"]!="wiki"{
        return Err(bad());
    }
    let meet=&other_transport["source_coordinate_meet"];
    if other_transport["schema"]!="slr.predicate-role-transport.v1"
        ||other_transport["semantic_coverage_paid"]!=false||other_transport["world_truth_paid"]!=false
        ||meet["schema"]!="slr.article-numeric-coordinate-meet.v1"
        ||meet["source_revision_ref"]!=overlap["right_source"]["revision"]
        ||meet["source_content_digest_ref"]!=format!("sha256:{}",overlap["right_source"]["sha256"].as_str().ok_or_else(bad)?)
        ||meet["common_current_authority_snapshot"]!=false
        ||["packet_sha256","numeric_run_ref","snapshot_ref"].iter().any(|k|
            !other_predicates[*k].as_str().is_some_and(|s|!s.is_empty()))
        ||meet["numeric_original"]["packet_sha256"]!=other_predicates["packet_sha256"]
        ||meet["numeric_original"]["numeric_run_ref"]!=other_predicates["numeric_run_ref"]
        ||meet["numeric_original"]["snapshot_ref"]!=other_predicates["snapshot_ref"]
        ||overlap["left_source"]["sha256"]==overlap["right_source"]["sha256"]{
        return Err(bad());
    }
    let members=|key:&str,id:&str|->Result<(usize,Value),IndexedObservationError>{
        let mut matches=overlap[key].as_array().ok_or_else(bad)?.iter().enumerate()
            .filter(|(_,m)|m["member_ref"]==id);
        let (i,m)=matches.next().ok_or_else(bad)?;
        if matches.next().is_some(){return Err(bad());}
        Ok((i,m.clone()))
    };
    let (li,left)=members("left_members",indexed_member)?;
    let (ri,right)=members("right_members",other_relation)?;
    // A pair may share several feature groups, but one group does not
    // choose a globally unique match. Preserve both posting-list degrees.
    let contains=|g:&Value,key:&str,i:usize|g[key].as_array().is_some_and(|a|
        a.iter().any(|n|n.as_u64()==Some(i as u64)));
    let shared:Vec<_>=overlap["feature_groups"].as_array().ok_or_else(bad)?.iter()
        .filter(|g|contains(g,"left_members",li)&&contains(g,"right_members",ri))
        .map(|g|json!({"axis":g["axis"],"value":g["value"],
            "left_posting_count":g["left_members"].as_array().map(Vec::len),
            "right_posting_count":g["right_members"].as_array().map(Vec::len),
            "status":g["status"],"semantic_alignment_paid":false})).collect();
    let mut left_inverse=Vec::new();
    for row in article["coordinates"].as_array().ok_or_else(bad)?{
        let indexed=row["indexed"].as_array().ok_or_else(bad)?;
        if !indexed.iter().any(|m|m["request_ref"]==indexed_member){continue;}
        for n in row["native"].as_array().ok_or_else(bad)?{
            if n["kind"]=="checked_predicate"{
                left_inverse.push(json!({"coordinate_status":row["status"],
                    "indexed_count":indexed.len(),"native_count":row["native"].as_array().ok_or_else(bad)?.len(),
                    "batch_ref":n["original"]["batch_ref"],
                    "predicate_candidate_ref":n["original"]["checked_observation"]["predicate_candidate_ref"],
                    "observation":n["original"]["checked_observation"],"original_transport":n["original"]}));
            }
        }
    }
    let mut right_inverse=Vec::new();
    for row in other_transport["checked_predicates"].as_array().ok_or_else(bad)?{
        for alt in row["numeric_alternatives"].as_array().ok_or_else(bad)?{
            if alt["numeric_relation"]["relation_ref"]!=other_relation{continue;}
            // Both readbacks must refer to the identical original relation
            // and role inverse, not merely a matching label or numeric ID.
            if alt["numeric_relation"]["original_relation"]!=right["original"]["original_relation"]
                ||alt["numeric_relation"]["role_inverse"]!=right["original"]["role_inverse"]{return Err(bad());}
            right_inverse.push(json!({"batch_ref":row["batch_ref"],
                "predicate_candidate_ref":row["checked_observation"]["predicate_candidate_ref"],
                "observation":row["checked_observation"],"original_transport":row,
                "selected_numeric_alternative":alt}));
        }
    }
    let mut holds=Vec::new();
    if shared.is_empty(){holds.push("no_shared_retrieval_feature");}
    if left_inverse.len()!=1||left_inverse[0]["coordinate_status"]!="unique_coordinate_match"
        ||left_inverse[0]["indexed_count"]!=1||left_inverse[0]["native_count"]!=1{
        holds.push("article_checked_predicate_inverse_not_unique");
    }
    if right_inverse.len()!=1{holds.push("other_checked_predicate_inverse_not_unique");}
    let exact_coords=|rows:&[Value]|rows.iter().all(|r|
        ["batch_ref","predicate_candidate_ref"].iter().all(|k|r[*k].as_str().is_some_and(|s|!s.is_empty())));
    if !exact_coords(&left_inverse)||!exact_coords(&right_inverse){return Err(bad());}
    let ready=holds.is_empty();
    let digest=format!("sha256:{:x}",Sha256::digest(serde_json::to_vec(&(
        &overlap["left_receipt_digest"],&overlap["right_receipt_digest"],consumer,indexed_member,other_relation,
        &left_inverse,&right_inverse,&shared)).map_err(|_|bad())?));
    Ok(json!({"schema":"slr.heterogeneous-native-selection.v1","consumer_ref":consumer,
        "selection_ref":format!("selection:{digest}"),"left_source":overlap["left_source"],
        "right_source":overlap["right_source"],"left_member":left,"right_member":right,
        "shared_features":shared,"left_inverse":left_inverse,"right_inverse":right_inverse,
        "holds":holds,"candidate_selector_ready":ready,"candidate_only":true,
        "semantic_alignment_paid":false,"source_independence_paid":false,
        "formal_consumer_equivalence_paid":false,"world_truth_paid":false,
        "original_checker_replayed":false,"writes_database":false}))
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
        let required=vec!["article_lanes:named_entities".into(),"article_lanes:time_mentions".into(),
            "article_lanes:missing".into()];
        let inspected=inspect_indexed_inventory(&r,"consumer:test",&required).unwrap();
        assert_eq!(inspected["residuals"].as_array().unwrap().len(),2);
        assert_eq!(inspected["members"].as_array().unwrap().len(),3);
        assert_eq!(inspected["original_checker_replayed"],false);
        r["world_truth_paid"]=json!(true);
        assert!(indexed_inventory_observations(&r).is_err());
    }
}

#[cfg(test)]
mod heterogeneous_overlap_tests {
    use super::*;
    use serde_json::{json,Value};

    fn fixture() -> (Value,Value) {
        let a="a".repeat(64);
        let b="b".repeat(64);
        let event=|surface:&str,dep:&str,person:&str|json!({
            "predicate":{"surface":surface,"lemma":"appoint"},
            "actors":[{"surface":person,"lemma":person,"dep":dep}],"patients":[]
        });
        let relation=|id:&str,dep:&str,person:&str|json!({
            "relation_ref":id,"original_relation":{"type":"predicate"},
            "role_inverse":[
                {"original_role":{"role":"head"},"original_atom":{"text":"appointed","lemma":"appoint"}},
                {"original_role":{"role":"subject"},"original_atom":{"text":person,"lemma":person,"dependency":dep}}
            ]
        });
        let indexed=json!({"schema":"slr.jmd-indexed-world-fibre-receipt.v1","qid":"Q9","title":"Example",
            "source_observation_checked":true,"cross_source_alignment_paid":false,"world_truth_paid":false,
            "source_sha256":a,"original_slr_sha256":a,"projection_sha256":a,"entity_sha256":a,"entity_revision":42,
            "inventory":{"schema":"slr.jmd-indexed-world-fibre.v1","qid":"Q9","title":"Example","article_revision":"enwiki-revid:23",
                "article_lanes":{"event_candidates":[event("appointed","nsubj","Alice"),
                    event("appoints","nsubjpass","Alice"),event("appointed","nsubj","Alice"),
                    {"predicate":{"surface":"left","lemma":"leave"},"actors":[],"patients":[]}]},
                "wikidata_lanes":{},"semantic_coverage_paid":false,"cross_source_alignment_paid":false,"world_truth_paid":false}});
        let native=json!({"schema":"slr.original-relation-query.v1","query_relation_type":"predicate",
            "exact_persisted_packet_readback":true,"complete_recorded_query":true,
            "all_live_semantic_readings_covered":false,"governed_identity_paid":false,"world_truth_paid":false,
            "source_revision_ref":"book:rev:1","source_sha256":b,"document_ref":format!("document:sha256:{b}"),
            "answer_count":2,"relation_type_inventory":{"predicate":2},
            "answers":[relation("r1","nsubj","Alice"),relation("r2","nsubj","Bob")]});
        (indexed,native)
    }

    #[test]
    fn dependent_overlap_retains_passive_and_missing_residuals() {
        let (indexed,native)=fixture();
        let out=inspect_heterogeneous_predicate_overlap(&indexed,&native,"consumer:test").unwrap();
        let coverage=out["directional_coverage"].as_array().unwrap().iter()
            .find(|x|x["axis"]=="predicate_dependency_surface").unwrap();
        assert_eq!(coverage["left_observed_members"],4);
        assert_eq!(coverage["left_matched_members"],2);
        assert_eq!(coverage["right_matched_members"],1);
        assert_eq!(coverage["left_unmatched"],json!([1]));
        assert_eq!(coverage["left_feature_unavailable"],json!([3]));
        assert_eq!(coverage["right_unmatched"],json!([1]));
        assert_eq!(coverage["left_ambiguous"],json!([0,2]));
        assert_eq!(out["licensed_crosswalk_count"],0);
        assert_eq!(out["world_truth_paid"],false);
        assert_eq!(out["right_members"][0]["original"],native["answers"][0]);
    }


    #[test]
    fn selected_heterogeneous_pair_retains_both_inverses_and_holds_ambiguity(){
        let (mut indexed,mut native)=fixture();
        let events=indexed["inventory"]["article_lanes"]["event_candidates"].as_array_mut().unwrap();
        for (i,e) in events.iter_mut().enumerate(){
            let n=e["predicate"]["surface"].as_str().unwrap().chars().count();
            e["predicate"]["start_char"]=json!(i*20);
            e["predicate"]["end_char"]=json!(i*20+n);
        }
        native["packet_sha256"]=json!("packet");
        native["numeric_run_ref"]=json!("book-run");
        native["snapshot_ref"]=json!("snapshot");
        let inv=indexed_inventory_observations(&indexed).unwrap();
        let member=inv.members.iter().find(|m|m.lane=="article_lanes:event_candidates"&&m.ordinal==0)
            .unwrap().observation.request_ref.clone();
        let article=json!({"schema":"slr.predicate-role-transport.v1","semantic_coverage_paid":false,
            "world_truth_paid":false,"source_coordinate_meet":{"schema":"slr.article-numeric-coordinate-meet.v1",
                "source_revision_ref":"enwiki-revid:23","source_content_digest_ref":format!("sha256:{}","a".repeat(64)),
                "native_source_family":"wiki","common_current_authority_snapshot":false,
                "coordinates":[{"span":[0,9],"literal":"appointed","checked":[{"batch_ref":"a-batch","factor_ref":"a-pred"}],"numeric":[]}]},
            "checked_predicates":[{"batch_ref":"a-batch","checked_observation":{"predicate_candidate_ref":"a-pred"}}],
            "numeric_only_predicates":[]});
        let book_row=json!({"batch_ref":"b-batch","checked_observation":{"predicate_candidate_ref":"b-pred"},
            "numeric_alternatives":[{"numeric_relation":native["answers"][0]}]});
        let mut book=json!({"schema":"slr.predicate-role-transport.v1","semantic_coverage_paid":false,"world_truth_paid":false,
            "source_coordinate_meet":{"schema":"slr.article-numeric-coordinate-meet.v1",
                "source_revision_ref":"book:rev:1","source_content_digest_ref":format!("sha256:{}","b".repeat(64)),
                "common_current_authority_snapshot":false,"numeric_original":{"packet_sha256":"packet",
                    "numeric_run_ref":"book-run","snapshot_ref":"snapshot"}},
            "checked_predicates":[book_row]});
        let out=select_heterogeneous_native_pair(&indexed,&article,&book,&native,"consumer:test",&member,"r1").unwrap();
        assert_eq!(out["candidate_selector_ready"],true);
        assert_eq!(out["left_inverse"][0]["predicate_candidate_ref"],"a-pred");
        assert_eq!(out["right_inverse"][0]["predicate_candidate_ref"],"b-pred");
        assert_eq!(out["semantic_alignment_paid"],false);
        let dup=book["checked_predicates"][0].clone();
        book["checked_predicates"].as_array_mut().unwrap().push(dup);
        let held=select_heterogeneous_native_pair(&indexed,&article,&book,&native,"consumer:test",&member,"r1").unwrap();
        assert_eq!(held["candidate_selector_ready"],false);
        assert_eq!(held["right_inverse"].as_array().unwrap().len(),2);
        book["source_coordinate_meet"]["numeric_original"]["packet_sha256"]=json!("foreign");
        assert!(select_heterogeneous_native_pair(&indexed,&article,&book,&native,"consumer:test",&member,"r1").is_err());
    }
    #[test]
    fn rejects_same_manifestation_and_promotion_and_marks_unavailable() {
        let (mut indexed,mut native)=fixture();
        indexed["inventory"]["article_lanes"]["event_candidates"]=Value::Null;
        let out=inspect_heterogeneous_predicate_overlap(&indexed,&native,"consumer:test").unwrap();
        assert!(out["directional_coverage"].as_array().unwrap().iter().all(|r|r["right_coverage"].is_null()));
        native["world_truth_paid"]=json!(true);
        assert!(inspect_heterogeneous_predicate_overlap(&indexed,&native,"consumer:test").is_err());
        native["world_truth_paid"]=json!(false);
        native["source_revision_ref"]=indexed["inventory"]["article_revision"].clone();
        native["source_sha256"]=indexed["source_sha256"].clone();
        native["document_ref"]=json!(format!("document:sha256:{}",indexed["source_sha256"].as_str().unwrap()));
        assert!(inspect_heterogeneous_predicate_overlap(&indexed,&native,"consumer:test").is_err());
    }
}
