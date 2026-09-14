use std::io::{ErrorKind, Read, Write};

use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use thiserror::Error;

pub const SLRG_MAGIC: [u8; 4] = *b"SLRG";
pub const SLRG_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 4 << 20;
const MAX_CANDIDATES: usize = 1 << 20;

#[derive(Debug, Error)]
pub enum RouteSelectorError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
    #[error("invalid route candidate: {0}")]
    InvalidCandidate(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerFamily {
    ArticleSemantic = 1,
    RevisionTemporal = 2,
    ParserRepair = 3,
    IdentitySource = 4,
    AuthoritySource = 5,
    MechanismEvidence = 6,
    MeasurementEvidence = 7,
    ComparatorEvidence = 8,
    ClassificationEvidence = 9,
}
impl ProducerFamily {
    fn from_u8(value: u8) -> Result<Self, RouteSelectorError> {
        match value {
            1 => Ok(Self::ArticleSemantic), 2 => Ok(Self::RevisionTemporal), 3 => Ok(Self::ParserRepair),
            4 => Ok(Self::IdentitySource), 5 => Ok(Self::AuthoritySource), 6 => Ok(Self::MechanismEvidence),
            7 => Ok(Self::MeasurementEvidence), 8 => Ok(Self::ComparatorEvidence), 9 => Ok(Self::ClassificationEvidence),
            _ => Err(RouteSelectorError::InvalidCandidate(format!("unknown producer family {value}"))),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteFamily {
    WikidataProperty = 1,
    WikipediaArticle = 2,
    RevisionHistory = 3,
    PrimarySourceSearch = 4,
    MeasurementSourceSearch = 5,
    ComparatorSourceSearch = 6,
    ParserRepair = 7,
}
impl RouteFamily {
    fn from_u8(value: u8) -> Result<Self, RouteSelectorError> {
        match value {
            1 => Ok(Self::WikidataProperty), 2 => Ok(Self::WikipediaArticle), 3 => Ok(Self::RevisionHistory),
            4 => Ok(Self::PrimarySourceSearch), 5 => Ok(Self::MeasurementSourceSearch),
            6 => Ok(Self::ComparatorSourceSearch), 7 => Ok(Self::ParserRepair),
            _ => Err(RouteSelectorError::InvalidCandidate(format!("unknown route family {value}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteCandidate {
    pub candidate_id: String,
    pub producer: ProducerFamily,
    pub route_family: RouteFamily,
    pub source_ref: String,
    pub target_ref: String,
    pub property_ref: String,
    pub cross_language_gap_coverage: u32,
    pub source_surface_support: u32,
    pub root_qid_support: u32,
    pub typed_property_support: u32,
    pub route_specificity: u32,
    pub yield_history_observed: u32,
    pub prior_contracted_old_gaps: u32,
    pub prior_retired_obligations: u32,
    pub prior_new_gap_atoms: u32,
    pub prior_network_requests: u32,
}

fn write_u32<W: Write>(w:&mut W,v:u32)->Result<(),RouteSelectorError>{w.write_all(&v.to_le_bytes())?;Ok(())}
fn read_u32<R: Read>(r:&mut R)->Result<u32,RouteSelectorError>{let mut b=[0u8;4];r.read_exact(&mut b)?;Ok(u32::from_le_bytes(b))}
fn write_text<W: Write>(w:&mut W,v:&str)->Result<(),RouteSelectorError>{if v.len()>MAX_TEXT_BYTES{return Err(RouteSelectorError::InvalidCandidate("text field too large".into()));}write_u32(w,v.len() as u32)?;w.write_all(v.as_bytes())?;Ok(())}
fn read_text<R: Read>(r:&mut R)->Result<String,RouteSelectorError>{let len=read_u32(r)? as usize;if len>MAX_TEXT_BYTES{return Err(RouteSelectorError::InvalidCandidate("text field too large".into()));}let mut b=vec![0u8;len];r.read_exact(&mut b)?;String::from_utf8(b).map_err(|_|RouteSelectorError::InvalidCandidate("text is not UTF-8".into()))}
fn read_exact_or_eof<R:Read>(r:&mut R,b:&mut[u8])->Result<bool,RouteSelectorError>{let mut o=0;while o<b.len(){match r.read(&mut b[o..]){Ok(0)if o==0=>return Ok(false),Ok(0)=>return Err(RouteSelectorError::InvalidCandidate("truncated route candidate".into())),Ok(n)=>o+=n,Err(e)if e.kind()==ErrorKind::Interrupted=>continue,Err(e)=>return Err(e.into())}}Ok(true)}

pub fn encode_route_candidate<W:Write>(w:&mut W,row:&RouteCandidate)->Result<(),RouteSelectorError>{
    w.write_all(&SLRG_MAGIC)?;w.write_all(&SLRG_VERSION.to_le_bytes())?;w.write_all(&[row.producer as u8,row.route_family as u8,1,0])?;
    write_text(w,&row.candidate_id)?;write_text(w,&row.source_ref)?;write_text(w,&row.target_ref)?;write_text(w,&row.property_ref)?;
    for v in [row.cross_language_gap_coverage,row.source_surface_support,row.root_qid_support,row.typed_property_support,row.route_specificity,row.yield_history_observed,row.prior_contracted_old_gaps,row.prior_retired_obligations,row.prior_new_gap_atoms,row.prior_network_requests]{write_u32(w,v)?;}Ok(())
}

pub fn decode_route_candidate<R:Read>(r:&mut R)->Result<Option<RouteCandidate>,RouteSelectorError>{
    let mut magic=[0u8;4];if !read_exact_or_eof(r,&mut magic)?{return Ok(None)}if magic!=SLRG_MAGIC{return Err(RouteSelectorError::InvalidCandidate("bad SLRG magic".into()))}
    let mut v=[0u8;2];r.read_exact(&mut v)?;if u16::from_le_bytes(v)!=SLRG_VERSION{return Err(RouteSelectorError::InvalidCandidate("unsupported SLRG version".into()))}
    let mut tags=[0u8;4];r.read_exact(&mut tags)?;if tags[2]!=1||tags[3]!=0{return Err(RouteSelectorError::InvalidCandidate("route candidate must be candidate-only and non-promoting".into()))}
    let producer=ProducerFamily::from_u8(tags[0])?;let route_family=RouteFamily::from_u8(tags[1])?;let candidate_id=read_text(r)?;let source_ref=read_text(r)?;let target_ref=read_text(r)?;let property_ref=read_text(r)?;
    if candidate_id.is_empty(){return Err(RouteSelectorError::InvalidCandidate("empty candidate id".into()))}
    let mut x=[0u32;10];for item in &mut x{*item=read_u32(r)?;}
    Ok(Some(RouteCandidate{candidate_id,producer,route_family,source_ref,target_ref,property_ref,cross_language_gap_coverage:x[0],source_surface_support:x[1],root_qid_support:x[2],typed_property_support:x[3],route_specificity:x[4],yield_history_observed:x[5],prior_contracted_old_gaps:x[6],prior_retired_obligations:x[7],prior_new_gap_atoms:x[8],prior_network_requests:x[9]}))
}

fn dominates(a:&RouteCandidate,b:&RouteCandidate)->bool{
    let max_a=[a.cross_language_gap_coverage,a.source_surface_support,a.root_qid_support,a.typed_property_support,a.route_specificity,a.yield_history_observed,a.prior_contracted_old_gaps,a.prior_retired_obligations];
    let max_b=[b.cross_language_gap_coverage,b.source_surface_support,b.root_qid_support,b.typed_property_support,b.route_specificity,b.yield_history_observed,b.prior_contracted_old_gaps,b.prior_retired_obligations];
    let min_a=[a.prior_new_gap_atoms,a.prior_network_requests];let min_b=[b.prior_new_gap_atoms,b.prior_network_requests];
    let weak=max_a.iter().zip(max_b.iter()).all(|(x,y)|x>=y)&&min_a.iter().zip(min_b.iter()).all(|(x,y)|x<=y);
    let strict=max_a.iter().zip(max_b.iter()).any(|(x,y)|x>y)||min_a.iter().zip(min_b.iter()).any(|(x,y)|x<y);weak&&strict
}

fn intent_producer(record:&WireRecord)->Option<ProducerFamily>{if record.kind!=WorldRecordKind::RouteAction||record.payload.len()<8||&record.payload[..4]!=b"RTA1"{return None}ProducerFamily::from_u8(record.payload[4]).ok()}

fn selected_body(intent:&WireRecord,candidate:&RouteCandidate)->Result<Vec<u8>,RouteSelectorError>{
    let mut out=Vec::new();out.extend_from_slice(b"RTA2");out.push(candidate.producer as u8);out.push(candidate.route_family as u8);out.push(1);out.push(0);
    write_text(&mut out,&intent.id)?;write_text(&mut out,&candidate.candidate_id)?;write_text(&mut out,&candidate.source_ref)?;write_text(&mut out,&candidate.target_ref)?;write_text(&mut out,&candidate.property_ref)?;Ok(out)
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct SelectionReceipt{pub intents_seen:u64,pub candidates_seen:u64,pub selected_actions:u64,pub pareto_dimensions_scalarized:bool,pub frontier_rank_is_truth_rank:bool,pub candidate_only:bool,pub semantic_promotion:bool,pub route_action_is_claim_truth:bool}

pub fn select_routes<RI:Read,RC:Read,W:Write>(intents:&mut RI,candidates:&mut RC,writer:&mut W,max_per_intent:usize)->Result<SelectionReceipt,RouteSelectorError>{
    let mut candidate_rows=Vec::new();while let Some(row)=decode_route_candidate(candidates)?{if candidate_rows.len()>=MAX_CANDIDATES{return Err(RouteSelectorError::InvalidCandidate("too many route candidates".into()))}candidate_rows.push(row)}
    let candidates_seen=candidate_rows.len() as u64;let mut intents_seen=0;let mut selected_actions=0;
    while let Some(intent)=decode_record(intents)?{let Some(producer)=intent_producer(&intent)else{continue};intents_seen+=1;let eligible:Vec<&RouteCandidate>=candidate_rows.iter().filter(|c|c.producer==producer).collect();let mut front:Vec<&RouteCandidate>=eligible.iter().copied().filter(|c|!eligible.iter().any(|other|other.candidate_id!=c.candidate_id&&dominates(other,c))).collect();front.sort_by(|a,b|a.candidate_id.cmp(&b.candidate_id));for candidate in front.into_iter().take(max_per_intent){encode_record(writer,&WireRecord{kind:WorldRecordKind::RouteAction,id:format!("selected:{}:{}",intent.id,candidate.candidate_id),iteration_index:intent.iteration_index,aux1:Some(candidate.target_ref.clone()),payload:selected_body(&intent,candidate)?})?;selected_actions+=1;}}
    Ok(SelectionReceipt{intents_seen,candidates_seen,selected_actions,pareto_dimensions_scalarized:false,frontier_rank_is_truth_rank:false,candidate_only:true,semantic_promotion:false,route_action_is_claim_truth:false})
}
