use std::io::{Read, Write};

use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use thiserror::Error;

pub const REVIEW_MAGIC: [u8; 4] = *b"SLRE";
pub const REVIEW_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 4 << 20;

#[derive(Debug, Error)]
pub enum EvidencePaymentError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
    #[error("invalid evidence review: {0}")]
    InvalidReview(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceCoordinateKind {
    SourceIdentity = 1,
    SameObject = 2,
    Authority = 3,
    Mechanism = 4,
    Quantification = 5,
    Probability = 6,
    Counterfactual = 7,
    InstrumentComparison = 8,
    Incidence = 9,
    Classification = 10,
}
impl EvidenceCoordinateKind {
    fn from_u8(value: u8) -> Result<Self, EvidencePaymentError> {
        match value {
            1 => Ok(Self::SourceIdentity), 2 => Ok(Self::SameObject), 3 => Ok(Self::Authority),
            4 => Ok(Self::Mechanism), 5 => Ok(Self::Quantification), 6 => Ok(Self::Probability),
            7 => Ok(Self::Counterfactual), 8 => Ok(Self::InstrumentComparison), 9 => Ok(Self::Incidence),
            10 => Ok(Self::Classification),
            _ => Err(EvidencePaymentError::InvalidReview(format!("unknown evidence coordinate {value}"))),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewDisposition {
    PaysObligation = 1,
    PartialEvidenceOnly = 2,
    MeasurementBlockOnly = 3,
    RejectedWrongType = 4,
}
impl ReviewDisposition {
    fn from_u8(value: u8) -> Result<Self, EvidencePaymentError> {
        match value {
            1 => Ok(Self::PaysObligation), 2 => Ok(Self::PartialEvidenceOnly),
            3 => Ok(Self::MeasurementBlockOnly), 4 => Ok(Self::RejectedWrongType),
            _ => Err(EvidencePaymentError::InvalidReview(format!("unknown disposition {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReviewSpec {
    pub consumer_id: String,
    pub requirement_id: String,
    pub coordinate: EvidenceCoordinateKind,
    pub evidence_reference: String,
    pub reviewer_reference: String,
    pub disposition: ReviewDisposition,
}

fn write_u32<W: Write>(w: &mut W, value: u32) -> Result<(), EvidencePaymentError> { w.write_all(&value.to_le_bytes())?; Ok(()) }
fn write_text<W: Write>(w: &mut W, value: &str) -> Result<(), EvidencePaymentError> {
    if value.is_empty() || value.len() > MAX_TEXT_BYTES { return Err(EvidencePaymentError::InvalidReview("invalid text field".into())); }
    write_u32(w, value.len() as u32)?; w.write_all(value.as_bytes())?; Ok(())
}
fn read_u32<R: Read>(r: &mut R) -> Result<u32, EvidencePaymentError> { let mut b=[0u8;4]; r.read_exact(&mut b)?; Ok(u32::from_le_bytes(b)) }
fn read_text<R: Read>(r: &mut R) -> Result<String, EvidencePaymentError> {
    let len=read_u32(r)? as usize; if len==0 || len>MAX_TEXT_BYTES { return Err(EvidencePaymentError::InvalidReview("invalid text length".into())); }
    let mut b=vec![0u8;len]; r.read_exact(&mut b)?; String::from_utf8(b).map_err(|_| EvidencePaymentError::InvalidReview("text is not UTF-8".into()))
}

pub fn encode_review_spec<W: Write>(w:&mut W, spec:&EvidenceReviewSpec)->Result<(),EvidencePaymentError>{
    w.write_all(&REVIEW_MAGIC)?; w.write_all(&REVIEW_VERSION.to_le_bytes())?;
    write_text(w,&spec.consumer_id)?; write_text(w,&spec.requirement_id)?; w.write_all(&[spec.coordinate as u8])?;
    write_text(w,&spec.evidence_reference)?; write_text(w,&spec.reviewer_reference)?; w.write_all(&[spec.disposition as u8])?; Ok(())
}

pub fn decode_review_spec<R: Read>(r:&mut R)->Result<EvidenceReviewSpec,EvidencePaymentError>{
    let mut magic=[0u8;4]; r.read_exact(&mut magic)?; if magic!=REVIEW_MAGIC { return Err(EvidencePaymentError::InvalidReview("bad review magic".into())); }
    let mut v=[0u8;2]; r.read_exact(&mut v)?; if u16::from_le_bytes(v)!=REVIEW_VERSION { return Err(EvidencePaymentError::InvalidReview("unsupported review version".into())); }
    let consumer_id=read_text(r)?; let requirement_id=read_text(r)?; let mut c=[0u8;1]; r.read_exact(&mut c)?; let coordinate=EvidenceCoordinateKind::from_u8(c[0])?;
    let evidence_reference=read_text(r)?; let reviewer_reference=read_text(r)?; let mut d=[0u8;1]; r.read_exact(&mut d)?; let disposition=ReviewDisposition::from_u8(d[0])?;
    Ok(EvidenceReviewSpec{consumer_id,requirement_id,coordinate,evidence_reference,reviewer_reference,disposition})
}

fn read_text_from_payload<R: Read>(r:&mut R)->Result<String,EvidencePaymentError>{
    let len=read_u32(r)? as usize; if len>MAX_TEXT_BYTES { return Err(EvidencePaymentError::InvalidReview("payload text too large".into())); }
    let mut b=vec![0u8;len]; r.read_exact(&mut b)?; String::from_utf8(b).map_err(|_| EvidencePaymentError::InvalidReview("payload text is not UTF-8".into()))
}

fn matches_obligation(record:&WireRecord,spec:&EvidenceReviewSpec)->Result<bool,EvidencePaymentError>{
    if record.kind!=WorldRecordKind::Obligation || record.payload.len()<8 || &record.payload[..4]!=b"OBL2" { return Ok(false); }
    if record.payload[4]!=spec.coordinate as u8 || record.payload[5]!=1 || record.payload[6]!=0 { return Ok(false); }
    let mut cur=std::io::Cursor::new(&record.payload[7..]);
    let consumer=read_text_from_payload(&mut cur)?; let requirement=read_text_from_payload(&mut cur)?;
    Ok(consumer==spec.consumer_id && requirement==spec.requirement_id)
}

fn review_body(spec:&EvidenceReviewSpec, target_obligation:&str)->Result<Vec<u8>,EvidencePaymentError>{
    let mut out=Vec::new(); out.extend_from_slice(b"RVW1"); out.push(spec.coordinate as u8); out.push(spec.disposition as u8); out.push(1); out.push(0);
    write_text(&mut out,&spec.consumer_id)?; write_text(&mut out,&spec.requirement_id)?; write_text(&mut out,&spec.evidence_reference)?; write_text(&mut out,&spec.reviewer_reference)?; write_text(&mut out,target_obligation)?; Ok(out)
}

fn payment_body(spec:&EvidenceReviewSpec,target:&str)->Result<Vec<u8>,EvidencePaymentError>{
    let mut out=Vec::new(); out.extend_from_slice(b"PAY2"); out.push(spec.coordinate as u8); out.push(1); out.push(0);
    write_text(&mut out,&spec.consumer_id)?; write_text(&mut out,&spec.requirement_id)?; write_text(&mut out,&spec.evidence_reference)?; write_text(&mut out,&spec.reviewer_reference)?; write_text(&mut out,target)?; Ok(out)
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct EvidencePaymentReceipt { pub target_obligation_found:bool,pub reviews_emitted:u64,pub payments_emitted:u64,pub claim_truth_promoted:bool,pub semantic_authority_created:bool }

pub fn apply_review_to_active_frontier<R:Read,W:Write>(reader:&mut R,spec:&EvidenceReviewSpec,writer:&mut W,iteration:i64)->Result<EvidencePaymentReceipt,EvidencePaymentError>{
    let mut target:Option<String>=None;
    while let Some(record)=decode_record(reader)? { if matches_obligation(&record,spec)? { target=Some(record.id); break; } }
    let target=target.ok_or_else(|| EvidencePaymentError::InvalidReview("exact active obligation not found".into()))?;
    let review_id=format!("review:{}:{}:{}",spec.consumer_id,spec.requirement_id,iteration);
    encode_record(writer,&WireRecord{kind:WorldRecordKind::Review,id:review_id,iteration_index:Some(iteration),aux1:Some(spec.evidence_reference.clone()),payload:review_body(spec,&target)?})?;
    let mut payments=0;
    if spec.disposition==ReviewDisposition::PaysObligation {
        let gap=format!("gap:{}:{}",spec.consumer_id,spec.requirement_id);
        let obligation=format!("obligation:{}:{}",spec.consumer_id,spec.requirement_id);
        for (label,target_residual) in [("gap",gap),("obligation",obligation)] {
            encode_record(writer,&WireRecord{kind:WorldRecordKind::Payment,id:format!("payment2:{label}:{}:{}:{iteration}",spec.consumer_id,spec.requirement_id),iteration_index:Some(iteration),aux1:Some(target_residual.clone()),payload:payment_body(spec,&target_residual)?})?;
            payments+=1;
        }
    }
    Ok(EvidencePaymentReceipt{target_obligation_found:true,reviews_emitted:1,payments_emitted:payments,claim_truth_promoted:false,semantic_authority_created:false})
}

/// Bounded reader-proof roles. They do not decide applicability or truth.
#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum PropositionProofRole { Support, Qualifier, Defeater, Comparator }

/// PNF evidence retains its own provenance separately from the graph span weld.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PropositionEvidenceObservation {
    pub observation_ref:String,
    pub pnf_factor_ref:String,
    pub pnf_revision_ref:String,
    pub role:PropositionProofRole,
    pub observation_provenance_refs:Vec<String>,
    pub graph_source_span_refs:Vec<String>,
    pub residual_refs:Vec<String>,
}

/// An explicit, retained role debt may be shown in a bounded explanation.
#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PropositionRoleResidual { pub role:PropositionProofRole,pub residual_ref:String }

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct PropositionChainPayment {
    pub proposition_ref:String,
    pub exact_source_paid:bool,
    pub support_observation_refs:Vec<String>,
    pub qualifier_observation_refs:Vec<String>,
    pub defeater_observation_refs:Vec<String>,
    pub comparator_observation_refs:Vec<String>,
    pub role_residual_refs:Vec<String>,
    pub residual_refs:Vec<String>,
    pub proposition_chain_paid:bool,
    pub why_executable:bool,
    pub applicability_paid:bool,
    pub claim_truth_paid:bool,
}

fn welded(observation:&PropositionEvidenceObservation,span:&str)->bool{
    observation.observation_provenance_refs.iter().any(|value|value==span)
        && observation.graph_source_span_refs.iter().any(|value|value==span)
}

fn paid(role:PropositionProofRole,span:&str,observations:&[PropositionEvidenceObservation])->Vec<String>{
    observations.iter().filter(|observation|observation.role==role && welded(observation,span)).map(|observation|observation.observation_ref.clone()).collect()
}

fn residualised(role:PropositionProofRole,residuals:&[PropositionRoleResidual])->bool{
    residuals.iter().any(|residual|residual.role==role)
}

/// Evaluate the narrow Source + PNF + role-debt gate for a bounded Why cone.
/// No result from this function pays applicability or claim truth.
pub fn evaluate_proposition_chain_payment(proposition_ref:&str,required_span_ref:&str,exact_source_paid:bool,observations:&[PropositionEvidenceObservation],residuals:&[PropositionRoleResidual])->PropositionChainPayment{
    let support_observation_refs=paid(PropositionProofRole::Support,required_span_ref,observations);
    let qualifier_observation_refs=paid(PropositionProofRole::Qualifier,required_span_ref,observations);
    let defeater_observation_refs=paid(PropositionProofRole::Defeater,required_span_ref,observations);
    let comparator_observation_refs=paid(PropositionProofRole::Comparator,required_span_ref,observations);
    let support_seen=observations.iter().any(|observation|observation.role==PropositionProofRole::Support);
    let support_paid=!support_observation_refs.is_empty();
    let qualifier_covered=!qualifier_observation_refs.is_empty() || residualised(PropositionProofRole::Qualifier,residuals);
    let defeater_covered=!defeater_observation_refs.is_empty() || residualised(PropositionProofRole::Defeater,residuals);
    let comparator_covered=!comparator_observation_refs.is_empty() || residualised(PropositionProofRole::Comparator,residuals);
    let mut residual_refs=Vec::new();
    if !exact_source_paid { residual_refs.push("reader-residual:exact-authority-span".into()); }
    if !support_paid { residual_refs.push(if support_seen { "reader-residual:source-provenance-weld".into() } else { "reader-residual:proposition-support".into() }); }
    for (covered,label) in [(qualifier_covered,"qualifier"),(defeater_covered,"defeater"),(comparator_covered,"comparator")] {
        if !covered { residual_refs.push(format!("reader-residual:{label}")); }
    }
    let role_residual_refs=residuals.iter().filter(|residual|matches!(residual.role,PropositionProofRole::Qualifier|PropositionProofRole::Defeater|PropositionProofRole::Comparator)).map(|residual|residual.residual_ref.clone()).collect();
    let proposition_chain_paid=exact_source_paid && support_paid && qualifier_covered && defeater_covered && comparator_covered;
    PropositionChainPayment{proposition_ref:proposition_ref.into(),exact_source_paid,support_observation_refs,qualifier_observation_refs,defeater_observation_refs,comparator_observation_refs,role_residual_refs,residual_refs,proposition_chain_paid,why_executable:proposition_chain_paid,applicability_paid:false,claim_truth_paid:false}
}
