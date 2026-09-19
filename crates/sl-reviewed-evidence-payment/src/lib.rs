use std::collections::BTreeSet;
use std::io::{Read, Write};

use sensiblaw_consumer_residual::{
    compile_consumer_residual_stream, ConsumerRequirement, ConsumerSpec, EvidenceCoordinateKind,
    RequirementNeed, RequirementScope, ResidualError, ResidualReceipt,
};
use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewedEvidenceCoordinate {
    pub review_ref: String,
    pub consumer_id: String,
    pub requirement_id: String,
    pub coordinate: EvidenceCoordinateKind,
    pub source_ref: Option<String>,
    pub evidence_ref: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReviewedEvidencePaymentReceipt {
    pub review_emitted: bool,
    pub payments_emitted: u64,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug, Error)]
pub enum ReviewedEvidencePaymentError {
    #[error("reviewed evidence coordinate must remain candidate-only and non-promoting")]
    PromotionNotAllowed,
    #[error("reviewed evidence coordinate contains an empty required reference")]
    EmptyReference,
    #[error("review consumer does not match consumer specification")]
    ConsumerMismatch,
    #[error("review requirement does not exist in consumer specification")]
    RequirementNotFound,
    #[error("reviewed coordinate does not match the evidence requirement")]
    CoordinateMismatch,
    #[error("reviewed evidence source does not match requirement scope")]
    ScopeMismatch,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("consumer residual error: {0}")]
    Residual(#[from] ResidualError),
    #[error("world record error: {0}")]
    World(#[from] sensiblaw_world_store::WorldStoreError),
}

fn write_u32<W: Write>(writer: &mut W, value: u32) -> Result<(), std::io::Error> {
    writer.write_all(&value.to_le_bytes())
}

fn write_text<W: Write>(writer: &mut W, value: &str) -> Result<(), std::io::Error> {
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value.as_bytes())
}

fn matching_requirement<'a>(
    spec: &'a ConsumerSpec,
    review: &ReviewedEvidenceCoordinate,
) -> Result<&'a ConsumerRequirement, ReviewedEvidencePaymentError> {
    if review.consumer_id != spec.consumer_id {
        return Err(ReviewedEvidencePaymentError::ConsumerMismatch);
    }
    spec.requirements
        .iter()
        .find(|requirement| requirement.requirement_id == review.requirement_id)
        .ok_or(ReviewedEvidencePaymentError::RequirementNotFound)
}

fn scope_matches(scope: &RequirementScope, source_ref: Option<&str>) -> bool {
    match scope {
        RequirementScope::AnySource => true,
        RequirementScope::SourceManifestation(expected) => source_ref == Some(expected.as_str()),
    }
}

fn review_payload(review: &ReviewedEvidenceCoordinate) -> Result<Vec<u8>, std::io::Error> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"REV2");
    payload.push(review.coordinate as u8);
    payload.push(1);
    payload.push(0);
    write_text(&mut payload, &review.consumer_id)?;
    write_text(&mut payload, &review.requirement_id)?;
    write_text(&mut payload, &review.evidence_ref)?;
    write_text(&mut payload, review.source_ref.as_deref().unwrap_or(""))?;
    Ok(payload)
}

fn payment_payload(review: &ReviewedEvidenceCoordinate) -> Result<Vec<u8>, std::io::Error> {
    let mut payload = Vec::new();
    payload.extend_from_slice(b"PAY2");
    payload.push(review.coordinate as u8);
    payload.push(1);
    payload.push(0);
    write_text(&mut payload, &review.review_ref)?;
    write_text(&mut payload, &review.evidence_ref)?;
    Ok(payload)
}

pub fn compile_reviewed_evidence_payment<W: Write>(
    spec: &ConsumerSpec,
    review: &ReviewedEvidenceCoordinate,
    writer: &mut W,
    iteration_index: i64,
) -> Result<ReviewedEvidencePaymentReceipt, ReviewedEvidencePaymentError> {
    if !review.candidate_only
        || review.creates_semantic_authority
        || review.applicability_promoted
        || review.claim_truth_promoted
    {
        return Err(ReviewedEvidencePaymentError::PromotionNotAllowed);
    }
    if review.review_ref.trim().is_empty()
        || review.consumer_id.trim().is_empty()
        || review.requirement_id.trim().is_empty()
        || review.evidence_ref.trim().is_empty()
    {
        return Err(ReviewedEvidencePaymentError::EmptyReference);
    }

    let requirement = matching_requirement(spec, review)?;
    match requirement.need {
        RequirementNeed::EvidenceCoordinate(coordinate) if coordinate == review.coordinate => {}
        _ => return Err(ReviewedEvidencePaymentError::CoordinateMismatch),
    }
    if !scope_matches(&requirement.scope, review.source_ref.as_deref()) {
        return Err(ReviewedEvidencePaymentError::ScopeMismatch);
    }

    encode_record(
        writer,
        &WireRecord {
            kind: WorldRecordKind::Review,
            id: review.review_ref.clone(),
            iteration_index: Some(iteration_index),
            aux1: Some(review.evidence_ref.clone()),
            payload: review_payload(review)?,
        },
    )?;

    let payload = payment_payload(review)?;
    for (target_kind, target_residual_id) in [
        (
            "gap",
            format!("gap:{}:{}", spec.consumer_id, review.requirement_id),
        ),
        (
            "obligation",
            format!("obligation:{}:{}", spec.consumer_id, review.requirement_id),
        ),
    ] {
        encode_record(
            writer,
            &WireRecord {
                kind: WorldRecordKind::Payment,
                id: format!(
                    "payment:reviewed-evidence:{target_kind}:{}:{}:{}",
                    spec.consumer_id, review.requirement_id, iteration_index
                ),
                iteration_index: Some(iteration_index),
                aux1: Some(target_residual_id),
                payload: payload.clone(),
            },
        )?;
    }

    Ok(ReviewedEvidencePaymentReceipt {
        review_emitted: true,
        payments_emitted: 2,
        candidate_only: true,
        semantic_promotion: false,
    })
}

fn explicitly_paid_requirement_ids(
    records: &[WireRecord],
    spec: &ConsumerSpec,
) -> BTreeSet<String> {
    let targets: BTreeSet<&str> = records
        .iter()
        .filter(|record| record.kind == WorldRecordKind::Payment)
        .filter_map(|record| record.aux1.as_deref())
        .collect();
    spec.requirements
        .iter()
        .filter(|requirement| {
            let gap = format!("gap:{}:{}", spec.consumer_id, requirement.requirement_id);
            let obligation = format!("obligation:{}:{}", spec.consumer_id, requirement.requirement_id);
            targets.contains(gap.as_str()) && targets.contains(obligation.as_str())
        })
        .map(|requirement| requirement.requirement_id.clone())
        .collect()
}

pub fn compile_consumer_residual_stream_review_aware<R: Read, W: Write>(
    world_reader: &mut R,
    spec: &ConsumerSpec,
    writer: &mut W,
    iteration_index: i64,
) -> Result<ResidualReceipt, ReviewedEvidencePaymentError> {
    let mut records = Vec::new();
    while let Some(record) = decode_record(world_reader)? {
        records.push(record);
    }
    let explicitly_paid = explicitly_paid_requirement_ids(&records, spec);
    let reduced = ConsumerSpec {
        consumer_id: spec.consumer_id.clone(),
        surface_id: spec.surface_id.clone(),
        requirements: spec
            .requirements
            .iter()
            .filter(|requirement| !explicitly_paid.contains(&requirement.requirement_id))
            .cloned()
            .collect(),
    };

    let mut replay = Vec::new();
    for record in &records {
        encode_record(&mut replay, record)?;
    }
    let inner = compile_consumer_residual_stream(
        &mut std::io::Cursor::new(replay),
        &reduced,
        writer,
        iteration_index,
    )?;
    let explicit_paid_count = explicitly_paid.len() as u64;
    let total = spec.requirements.len() as u64;
    let requirements_paid = explicit_paid_count + inner.requirements_paid;
    Ok(ResidualReceipt {
        requirements_total: total,
        requirements_paid,
        requirements_unpaid: total.saturating_sub(requirements_paid),
        gaps_emitted: inner.gaps_emitted,
        obligations_emitted: inner.obligations_emitted,
        payments_emitted: inner.payments_emitted,
        candidate_only: true,
        semantic_promotion: false,
    })
}

mod shared_reducer;
pub use shared_reducer::{
    reduce_reviewed_canonical_evidence, CanonicalEvidenceProjection,
    CanonicalProjectionReceipt, ProjectionDisposition, ProjectionFamily,
    ReviewedCanonicalEvidence, SharedEvidenceReductionReceipt,
    SharedEvidenceReducerError,
};
