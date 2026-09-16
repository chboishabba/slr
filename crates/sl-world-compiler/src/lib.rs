use std::io::{ErrorKind, Read, Write};

use sensiblaw_world_store::{encode_record, WireRecord, WorldRecordKind};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const OBS_MAGIC: [u8; 4] = *b"SLRO";
pub const OBS_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 4 << 20;

#[derive(Debug, Error)]
pub enum CompilerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("invalid observation: {0}")]
    InvalidObservation(String),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependencyShape {
    NominalSubject = 1,
    DirectObject = 2,
    PassiveSubject = 3,
    AdjectivalModifier = 4,
    NominalModifier = 5,
    Conjunction = 6,
    Negation = 7,
    ModalAuxiliary = 8,
    Determiner = 9,
    TemporalModifier = 10,
    ClausalComplement = 11,
    OpenClausalComplement = 12,
    AdverbialClause = 13,
    ClausalModifier = 14,
    RelativeClause = 15,
    UnresolvedDependency = 16,
}

impl DependencyShape {
    fn from_u8(value: u8) -> Result<Self, CompilerError> {
        match value {
            1 => Ok(Self::NominalSubject), 2 => Ok(Self::DirectObject), 3 => Ok(Self::PassiveSubject),
            4 => Ok(Self::AdjectivalModifier), 5 => Ok(Self::NominalModifier), 6 => Ok(Self::Conjunction),
            7 => Ok(Self::Negation), 8 => Ok(Self::ModalAuxiliary), 9 => Ok(Self::Determiner),
            10 => Ok(Self::TemporalModifier), 11 => Ok(Self::ClausalComplement), 12 => Ok(Self::OpenClausalComplement),
            13 => Ok(Self::AdverbialClause), 14 => Ok(Self::ClausalModifier), 15 => Ok(Self::RelativeClause),
            16 => Ok(Self::UnresolvedDependency),
            _ => Err(CompilerError::InvalidObservation(format!("unknown dependency shape {value}"))),
        }
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FragmentKind {
    Actor = 1,
    Patient = 2,
    Property = 3,
    Relation = 4,
    Conjunction = 5,
    Negation = 6,
    Modality = 7,
    Quantifier = 8,
    Temporal = 9,
    ContentClause = 10,
    ClauseAttachment = 11,
    Unresolved = 12,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationRecord {
    Manifestation {
        document_ref: String,
        qid: String,
        language: String,
        revision_ref: String,
        source_sha256: [u8; 32],
    },
    Token {
        document_ref: String,
        sentence_id: u64,
        local_ordinal: u32,
        start_char: u32,
        end_char: u32,
        head_ordinal: u32,
        shape: DependencyShape,
        orth: String,
        lemma: String,
        head_orth: String,
        head_lemma: String,
    },
}

fn write_u32<W: Write>(w: &mut W, value: u32) -> Result<(), CompilerError> { w.write_all(&value.to_le_bytes())?; Ok(()) }
fn write_u64<W: Write>(w: &mut W, value: u64) -> Result<(), CompilerError> { w.write_all(&value.to_le_bytes())?; Ok(()) }
fn write_text<W: Write>(w: &mut W, value: &str) -> Result<(), CompilerError> {
    if value.len() > MAX_TEXT_BYTES { return Err(CompilerError::InvalidObservation("text field too large".into())); }
    write_u32(w, value.len() as u32)?;
    w.write_all(value.as_bytes())?;
    Ok(())
}

fn read_exact_or_eof<R: Read>(r: &mut R, buf: &mut [u8]) -> Result<bool, CompilerError> {
    let mut offset = 0;
    while offset < buf.len() {
        match r.read(&mut buf[offset..]) {
            Ok(0) if offset == 0 => return Ok(false),
            Ok(0) => return Err(CompilerError::InvalidObservation("truncated observation".into())),
            Ok(n) => offset += n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.into()),
        }
    }
    Ok(true)
}

fn read_u32<R: Read>(r: &mut R) -> Result<u32, CompilerError> { let mut b=[0u8;4]; r.read_exact(&mut b)?; Ok(u32::from_le_bytes(b)) }
fn read_u64<R: Read>(r: &mut R) -> Result<u64, CompilerError> { let mut b=[0u8;8]; r.read_exact(&mut b)?; Ok(u64::from_le_bytes(b)) }
fn read_text<R: Read>(r: &mut R) -> Result<String, CompilerError> {
    let len = read_u32(r)? as usize;
    if len > MAX_TEXT_BYTES { return Err(CompilerError::InvalidObservation("declared text length too large".into())); }
    let mut b = vec![0u8; len];
    r.read_exact(&mut b)?;
    String::from_utf8(b).map_err(|_| CompilerError::InvalidObservation("text is not UTF-8".into()))
}

pub fn encode_observation<W: Write>(writer: &mut W, record: &ObservationRecord) -> Result<(), CompilerError> {
    writer.write_all(&OBS_MAGIC)?;
    writer.write_all(&OBS_VERSION.to_le_bytes())?;
    match record {
        ObservationRecord::Manifestation { document_ref, qid, language, revision_ref, source_sha256 } => {
            writer.write_all(&[1])?;
            write_text(writer, document_ref)?;
            write_text(writer, qid)?;
            write_text(writer, language)?;
            write_text(writer, revision_ref)?;
            writer.write_all(source_sha256)?;
        }
        ObservationRecord::Token { document_ref, sentence_id, local_ordinal, start_char, end_char, head_ordinal, shape, orth, lemma, head_orth, head_lemma } => {
            writer.write_all(&[2])?;
            write_text(writer, document_ref)?;
            write_u64(writer, *sentence_id)?;
            write_u32(writer, *local_ordinal)?;
            write_u32(writer, *start_char)?;
            write_u32(writer, *end_char)?;
            write_u32(writer, *head_ordinal)?;
            writer.write_all(&[*shape as u8])?;
            write_text(writer, orth)?;
            write_text(writer, lemma)?;
            write_text(writer, head_orth)?;
            write_text(writer, head_lemma)?;
        }
    }
    Ok(())
}

pub fn decode_observation<R: Read>(reader: &mut R) -> Result<Option<ObservationRecord>, CompilerError> {
    let mut magic = [0u8;4];
    if !read_exact_or_eof(reader, &mut magic)? { return Ok(None); }
    if magic != OBS_MAGIC { return Err(CompilerError::InvalidObservation("bad observation magic".into())); }
    let mut v=[0u8;2]; reader.read_exact(&mut v)?;
    if u16::from_le_bytes(v) != OBS_VERSION { return Err(CompilerError::InvalidObservation("unsupported observation version".into())); }
    let mut kind=[0u8;1]; reader.read_exact(&mut kind)?;
    match kind[0] {
        1 => {
            let document_ref=read_text(reader)?; let qid=read_text(reader)?; let language=read_text(reader)?; let revision_ref=read_text(reader)?;
            let mut source_sha256=[0u8;32]; reader.read_exact(&mut source_sha256)?;
            Ok(Some(ObservationRecord::Manifestation { document_ref, qid, language, revision_ref, source_sha256 }))
        }
        2 => {
            let document_ref=read_text(reader)?; let sentence_id=read_u64(reader)?; let local_ordinal=read_u32(reader)?;
            let start_char=read_u32(reader)?; let end_char=read_u32(reader)?; let head_ordinal=read_u32(reader)?;
            let mut s=[0u8;1]; reader.read_exact(&mut s)?; let shape=DependencyShape::from_u8(s[0])?;
            let orth=read_text(reader)?; let lemma=read_text(reader)?; let head_orth=read_text(reader)?; let head_lemma=read_text(reader)?;
            Ok(Some(ObservationRecord::Token { document_ref, sentence_id, local_ordinal, start_char, end_char, head_ordinal, shape, orth, lemma, head_orth, head_lemma }))
        }
        other => Err(CompilerError::InvalidObservation(format!("unknown observation kind {other}"))),
    }
}

fn fragment_for_shape(shape: DependencyShape) -> Option<FragmentKind> {
    match shape {
        DependencyShape::NominalSubject | DependencyShape::PassiveSubject => Some(FragmentKind::Actor),
        DependencyShape::DirectObject => Some(FragmentKind::Patient),
        DependencyShape::AdjectivalModifier | DependencyShape::NominalModifier => Some(FragmentKind::Property),
        DependencyShape::Conjunction => Some(FragmentKind::Conjunction),
        DependencyShape::Negation => Some(FragmentKind::Negation),
        DependencyShape::ModalAuxiliary => Some(FragmentKind::Modality),
        DependencyShape::TemporalModifier => Some(FragmentKind::Temporal),
        DependencyShape::ClausalComplement | DependencyShape::OpenClausalComplement => Some(FragmentKind::ContentClause),
        DependencyShape::AdverbialClause | DependencyShape::ClausalModifier | DependencyShape::RelativeClause => Some(FragmentKind::ClauseAttachment),
        DependencyShape::Determiner | DependencyShape::UnresolvedDependency => None,
    }
}

fn sha_id(prefix: &str, fields: &[&[u8]]) -> String {
    let mut h=Sha256::new();
    for f in fields { h.update((*f).len().to_le_bytes()); h.update(f); }
    format!("{prefix}{:x}", h.finalize())
}

fn manifestation_body(qid:&str, language:&str, revision_ref:&str, source_sha256:&[u8;32]) -> Result<Vec<u8>, CompilerError> {
    let mut out=Vec::new(); out.extend_from_slice(b"SRC1"); write_text(&mut out,qid)?; write_text(&mut out,language)?; write_text(&mut out,revision_ref)?; out.extend_from_slice(source_sha256); out.push(1); out.push(0); Ok(out)
}

#[allow(clippy::too_many_arguments)] // This mirrors the fixed PNF1 binary body layout.
fn pnf_body(fragment:FragmentKind, shape:DependencyShape, sentence_id:u64, local_ordinal:u32, head_ordinal:u32, start_char:u32, end_char:u32, orth:&str, lemma:&str, head_orth:&str, head_lemma:&str) -> Result<Vec<u8>,CompilerError> {
    let mut out=Vec::new(); out.extend_from_slice(b"PNF1"); out.push(fragment as u8); out.push(shape as u8); out.push(1); out.push(0);
    write_u64(&mut out,sentence_id)?; write_u32(&mut out,local_ordinal)?; write_u32(&mut out,head_ordinal)?; write_u32(&mut out,start_char)?; write_u32(&mut out,end_char)?;
    write_text(&mut out,orth)?; write_text(&mut out,lemma)?; write_text(&mut out,head_orth)?; write_text(&mut out,head_lemma)?; Ok(out)
}

fn atom_body(candidate_id:&str, pnf_body:&[u8]) -> Result<Vec<u8>,CompilerError> {
    let mut out=Vec::new(); out.extend_from_slice(b"ATM1"); write_text(&mut out,candidate_id)?; write_u32(&mut out,pnf_body.len() as u32)?; out.extend_from_slice(pnf_body); out.push(1); out.push(0); Ok(out)
}

fn iteration_body(receipt:&CompileReceipt) -> Vec<u8> {
    let mut out=Vec::new(); out.extend_from_slice(b"ITR1");
    out.extend_from_slice(&receipt.manifestations.to_le_bytes()); out.extend_from_slice(&receipt.pnf_candidates.to_le_bytes());
    out.extend_from_slice(&receipt.world_atoms.to_le_bytes()); out.extend_from_slice(&receipt.unresolved_dependencies.to_le_bytes());
    out.push(1); out.push(0); out
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompileReceipt {
    pub manifestations:u64,
    pub pnf_candidates:u64,
    pub world_atoms:u64,
    pub unresolved_dependencies:u64,
    pub candidate_only:bool,
    pub semantic_promotion:bool,
}

pub fn compile_observation_stream<R:Read,W:Write>(reader:&mut R, writer:&mut W, iteration_index:i64) -> Result<CompileReceipt,CompilerError> {
    let mut receipt=CompileReceipt{manifestations:0,pnf_candidates:0,world_atoms:0,unresolved_dependencies:0,candidate_only:true,semantic_promotion:false};
    while let Some(observation)=decode_observation(reader)? {
        match observation {
            ObservationRecord::Manifestation{document_ref,qid,language,revision_ref,source_sha256} => {
                let body=manifestation_body(&qid,&language,&revision_ref,&source_sha256)?;
                encode_record(writer,&WireRecord{kind:WorldRecordKind::SourceManifestation,id:document_ref,iteration_index:Some(iteration_index),aux1:None,payload:body})?;
                receipt.manifestations+=1;
            }
            ObservationRecord::Token{document_ref,sentence_id,local_ordinal,start_char,end_char,head_ordinal,shape,orth,lemma,head_orth,head_lemma} => {
                if let Some(fragment)=fragment_for_shape(shape) {
                    let candidate_id=sha_id("pnf-candidate:",&[document_ref.as_bytes(),&sentence_id.to_le_bytes(),&local_ordinal.to_le_bytes(),&[shape as u8],orth.as_bytes(),lemma.as_bytes(),head_lemma.as_bytes()]);
                    let pnf=pnf_body(fragment,shape,sentence_id,local_ordinal,head_ordinal,start_char,end_char,&orth,&lemma,&head_orth,&head_lemma)?;
                    encode_record(writer,&WireRecord{kind:WorldRecordKind::PnfCandidate,id:candidate_id.clone(),iteration_index:Some(iteration_index),aux1:Some(document_ref.clone()),payload:pnf.clone()})?;
                    let atom_id=sha_id("atom:pnf:",&[candidate_id.as_bytes()]);
                    let atom=atom_body(&candidate_id,&pnf)?;
                    encode_record(writer,&WireRecord{kind:WorldRecordKind::WorldAtom,id:atom_id,iteration_index:Some(iteration_index),aux1:Some(document_ref),payload:atom})?;
                    receipt.pnf_candidates+=1; receipt.world_atoms+=1;
                } else {
                    receipt.unresolved_dependencies+=1;
                }
            }
        }
    }
    let iteration=iteration_body(&receipt);
    encode_record(writer,&WireRecord{kind:WorldRecordKind::Iteration,id:format!("iteration:{iteration_index}"),iteration_index:Some(iteration_index),aux1:None,payload:iteration})?;
    Ok(receipt)
}
