use std::io::{ErrorKind, Read, Write};

use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use thiserror::Error;

pub const CONSUMER_MAGIC: [u8; 4] = *b"SLRC";
pub const CONSUMER_VERSION: u16 = 2;
const LEGACY_CONSUMER_VERSION: u16 = 1;
const MAX_TEXT_BYTES: usize = 4 << 20;
const MAX_REQUIREMENTS: usize = 1 << 20;
const NEED_PNF_FRAGMENT: u8 = 1;
const NEED_EVIDENCE_COORDINATE: u8 = 2;

#[derive(Debug, Error)]
pub enum ResidualError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
    #[error("invalid consumer specification: {0}")]
    InvalidConsumer(String),
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

impl FragmentKind {
    pub fn from_u8(value: u8) -> Result<Self, ResidualError> {
        match value {
            1 => Ok(Self::Actor),
            2 => Ok(Self::Patient),
            3 => Ok(Self::Property),
            4 => Ok(Self::Relation),
            5 => Ok(Self::Conjunction),
            6 => Ok(Self::Negation),
            7 => Ok(Self::Modality),
            8 => Ok(Self::Quantifier),
            9 => Ok(Self::Temporal),
            10 => Ok(Self::ContentClause),
            11 => Ok(Self::ClauseAttachment),
            12 => Ok(Self::Unresolved),
            _ => Err(ResidualError::InvalidConsumer(format!("unknown fragment kind {value}"))),
        }
    }
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
    pub fn from_u8(value: u8) -> Result<Self, ResidualError> {
        match value {
            1 => Ok(Self::SourceIdentity),
            2 => Ok(Self::SameObject),
            3 => Ok(Self::Authority),
            4 => Ok(Self::Mechanism),
            5 => Ok(Self::Quantification),
            6 => Ok(Self::Probability),
            7 => Ok(Self::Counterfactual),
            8 => Ok(Self::InstrumentComparison),
            9 => Ok(Self::Incidence),
            10 => Ok(Self::Classification),
            _ => Err(ResidualError::InvalidConsumer(format!("unknown evidence coordinate kind {value}"))),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequirementNeed {
    PnfFragment(FragmentKind),
    EvidenceCoordinate(EvidenceCoordinateKind),
}

impl RequirementNeed {
    fn class_tag(self) -> u8 {
        match self {
            Self::PnfFragment(_) => NEED_PNF_FRAGMENT,
            Self::EvidenceCoordinate(_) => NEED_EVIDENCE_COORDINATE,
        }
    }

    fn value_tag(self) -> u8 {
        match self {
            Self::PnfFragment(value) => value as u8,
            Self::EvidenceCoordinate(value) => value as u8,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequirementScope {
    AnySource,
    SourceManifestation(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerRequirement {
    pub requirement_id: String,
    pub need: RequirementNeed,
    pub scope: RequirementScope,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsumerSpec {
    pub consumer_id: String,
    pub surface_id: String,
    pub requirements: Vec<ConsumerRequirement>,
}

fn write_u32<W: Write>(writer: &mut W, value: u32) -> Result<(), ResidualError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_text<W: Write>(writer: &mut W, value: &str) -> Result<(), ResidualError> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(ResidualError::InvalidConsumer("text field too large".into()));
    }
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, ResidualError> {
    let mut buf = [0u8; 4];
    reader.read_exact(&mut buf)?;
    Ok(u32::from_le_bytes(buf))
}

fn read_text<R: Read>(reader: &mut R) -> Result<String, ResidualError> {
    let len = read_u32(reader)? as usize;
    if len > MAX_TEXT_BYTES {
        return Err(ResidualError::InvalidConsumer("declared text field too large".into()));
    }
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| ResidualError::InvalidConsumer("text is not UTF-8".into()))
}

fn write_scope<W: Write>(writer: &mut W, scope: &RequirementScope) -> Result<(), ResidualError> {
    match scope {
        RequirementScope::AnySource => writer.write_all(&[0])?,
        RequirementScope::SourceManifestation(source) => {
            writer.write_all(&[1])?;
            write_text(writer, source)?;
        }
    }
    Ok(())
}

fn read_scope<R: Read>(reader: &mut R) -> Result<RequirementScope, ResidualError> {
    let mut scope = [0u8; 1];
    reader.read_exact(&mut scope)?;
    match scope[0] {
        0 => Ok(RequirementScope::AnySource),
        1 => Ok(RequirementScope::SourceManifestation(read_text(reader)?)),
        other => Err(ResidualError::InvalidConsumer(format!("unknown requirement scope {other}"))),
    }
}

pub fn encode_consumer_spec<W: Write>(writer: &mut W, spec: &ConsumerSpec) -> Result<(), ResidualError> {
    if spec.consumer_id.is_empty() || spec.surface_id.is_empty() {
        return Err(ResidualError::InvalidConsumer("consumer and surface identifiers must be non-empty".into()));
    }
    if spec.requirements.len() > MAX_REQUIREMENTS {
        return Err(ResidualError::InvalidConsumer("too many requirements".into()));
    }
    writer.write_all(&CONSUMER_MAGIC)?;
    writer.write_all(&CONSUMER_VERSION.to_le_bytes())?;
    write_text(writer, &spec.consumer_id)?;
    write_text(writer, &spec.surface_id)?;
    write_u32(writer, spec.requirements.len() as u32)?;
    for requirement in &spec.requirements {
        if requirement.requirement_id.is_empty() {
            return Err(ResidualError::InvalidConsumer("requirement identifier must be non-empty".into()));
        }
        write_text(writer, &requirement.requirement_id)?;
        writer.write_all(&[requirement.need.class_tag(), requirement.need.value_tag()])?;
        write_scope(writer, &requirement.scope)?;
    }
    Ok(())
}

pub fn decode_consumer_spec<R: Read>(reader: &mut R) -> Result<ConsumerSpec, ResidualError> {
    let mut magic = [0u8; 4];
    reader.read_exact(&mut magic)?;
    if magic != CONSUMER_MAGIC {
        return Err(ResidualError::InvalidConsumer("bad consumer magic".into()));
    }
    let mut version_bytes = [0u8; 2];
    reader.read_exact(&mut version_bytes)?;
    let version = u16::from_le_bytes(version_bytes);
    if version != CONSUMER_VERSION && version != LEGACY_CONSUMER_VERSION {
        return Err(ResidualError::InvalidConsumer(format!("unsupported consumer version {version}")));
    }
    let consumer_id = read_text(reader)?;
    let surface_id = read_text(reader)?;
    if consumer_id.is_empty() || surface_id.is_empty() {
        return Err(ResidualError::InvalidConsumer("consumer and surface identifiers must be non-empty".into()));
    }
    let count = read_u32(reader)? as usize;
    if count > MAX_REQUIREMENTS {
        return Err(ResidualError::InvalidConsumer("too many requirements".into()));
    }
    let mut requirements = Vec::with_capacity(count);
    for _ in 0..count {
        let requirement_id = read_text(reader)?;
        let need = if version == LEGACY_CONSUMER_VERSION {
            let mut fragment = [0u8; 1];
            reader.read_exact(&mut fragment)?;
            RequirementNeed::PnfFragment(FragmentKind::from_u8(fragment[0])?)
        } else {
            let mut tags = [0u8; 2];
            reader.read_exact(&mut tags)?;
            match tags[0] {
                NEED_PNF_FRAGMENT => RequirementNeed::PnfFragment(FragmentKind::from_u8(tags[1])?),
                NEED_EVIDENCE_COORDINATE => RequirementNeed::EvidenceCoordinate(EvidenceCoordinateKind::from_u8(tags[1])?),
                other => return Err(ResidualError::InvalidConsumer(format!("unknown requirement class {other}"))),
            }
        };
        let scope = read_scope(reader)?;
        requirements.push(ConsumerRequirement { requirement_id, need, scope });
    }
    Ok(ConsumerSpec { consumer_id, surface_id, requirements })
}

fn pnf_fragment(record: &WireRecord) -> Option<FragmentKind> {
    if record.kind != WorldRecordKind::PnfCandidate || record.payload.len() < 8 {
        return None;
    }
    if &record.payload[..4] != b"PNF1" {
        return None;
    }
    if record.payload[6] != 1 || record.payload[7] != 0 {
        return None;
    }
    FragmentKind::from_u8(record.payload[4]).ok()
}

fn scope_matches(scope: &RequirementScope, source: Option<&str>) -> bool {
    match scope {
        RequirementScope::AnySource => true,
        RequirementScope::SourceManifestation(expected) => source == Some(expected.as_str()),
    }
}

fn residual_magic(need: RequirementNeed, gap: bool) -> &'static [u8; 4] {
    match (need, gap) {
        (RequirementNeed::PnfFragment(_), true) => b"GAP1",
        (RequirementNeed::PnfFragment(_), false) => b"OBL1",
        (RequirementNeed::EvidenceCoordinate(_), true) => b"GAP2",
        (RequirementNeed::EvidenceCoordinate(_), false) => b"OBL2",
    }
}

fn residual_body(
    magic: &[u8; 4],
    need: RequirementNeed,
    consumer_id: &str,
    requirement_id: &str,
    scope: &RequirementScope,
) -> Result<Vec<u8>, ResidualError> {
    let mut body = Vec::new();
    body.extend_from_slice(magic);
    body.push(need.value_tag());
    body.push(1);
    body.push(0);
    write_text(&mut body, consumer_id)?;
    write_text(&mut body, requirement_id)?;
    write_scope(&mut body, scope)?;
    Ok(body)
}

fn payment_body(
    fragment: FragmentKind,
    consumer_id: &str,
    requirement_id: &str,
    target_residual_id: &str,
    scope: &RequirementScope,
) -> Result<Vec<u8>, ResidualError> {
    let mut body = residual_body(
        b"PAY1",
        RequirementNeed::PnfFragment(fragment),
        consumer_id,
        requirement_id,
        scope,
    )?;
    write_text(&mut body, target_residual_id)?;
    Ok(body)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualReceipt {
    pub requirements_total: u64,
    pub requirements_paid: u64,
    pub requirements_unpaid: u64,
    pub gaps_emitted: u64,
    pub obligations_emitted: u64,
    pub payments_emitted: u64,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

pub fn compile_consumer_residual_stream<R: Read, W: Write>(
    world_reader: &mut R,
    spec: &ConsumerSpec,
    writer: &mut W,
    iteration_index: i64,
) -> Result<ResidualReceipt, ResidualError> {
    let mut paid = vec![false; spec.requirements.len()];

    loop {
        match decode_record(world_reader) {
            Ok(Some(record)) => {
                if let Some(fragment) = pnf_fragment(&record) {
                    for (index, requirement) in spec.requirements.iter().enumerate() {
                        if paid[index] {
                            continue;
                        }
                        if let RequirementNeed::PnfFragment(required_fragment) = requirement.need {
                            if fragment == required_fragment
                                && scope_matches(&requirement.scope, record.aux1.as_deref())
                            {
                                paid[index] = true;
                            }
                        }
                    }
                }
                encode_record(writer, &record)?;
            }
            Ok(None) => break,
            Err(error) => return Err(error.into()),
        }
    }

    let mut gaps = 0u64;
    let mut obligations = 0u64;
    let mut payments = 0u64;
    for (index, requirement) in spec.requirements.iter().enumerate() {
        let gap_id = format!("gap:{}:{}", spec.consumer_id, requirement.requirement_id);
        let obligation_id = format!("obligation:{}:{}", spec.consumer_id, requirement.requirement_id);

        if paid[index] {
            let RequirementNeed::PnfFragment(fragment) = requirement.need else {
                return Err(ResidualError::InvalidConsumer("evidence coordinate cannot be paid implicitly by PNF".into()));
            };
            for (target_kind, target_residual_id) in [
                ("gap", gap_id.as_str()),
                ("obligation", obligation_id.as_str()),
            ] {
                let payment_id = format!(
                    "payment:{}:{}:{}:{}",
                    target_kind, spec.consumer_id, requirement.requirement_id, iteration_index
                );
                let body = payment_body(
                    fragment,
                    &spec.consumer_id,
                    &requirement.requirement_id,
                    target_residual_id,
                    &requirement.scope,
                )?;
                encode_record(
                    writer,
                    &WireRecord {
                        kind: WorldRecordKind::Payment,
                        id: payment_id,
                        iteration_index: Some(iteration_index),
                        aux1: Some(target_residual_id.to_string()),
                        payload: body,
                    },
                )?;
                payments += 1;
            }
            continue;
        }

        let gap_body = residual_body(
            residual_magic(requirement.need, true),
            requirement.need,
            &spec.consumer_id,
            &requirement.requirement_id,
            &requirement.scope,
        )?;
        encode_record(
            writer,
            &WireRecord {
                kind: WorldRecordKind::Gap,
                id: gap_id,
                iteration_index: Some(iteration_index),
                aux1: Some(spec.surface_id.clone()),
                payload: gap_body,
            },
        )?;
        gaps += 1;

        let obligation_body = residual_body(
            residual_magic(requirement.need, false),
            requirement.need,
            &spec.consumer_id,
            &requirement.requirement_id,
            &requirement.scope,
        )?;
        let obligation_kind = match requirement.need {
            RequirementNeed::PnfFragment(_) => "need-fragment-kind",
            RequirementNeed::EvidenceCoordinate(_) => "need-evidence-coordinate",
        };
        encode_record(
            writer,
            &WireRecord {
                kind: WorldRecordKind::Obligation,
                id: obligation_id,
                iteration_index: Some(iteration_index),
                aux1: Some(obligation_kind.into()),
                payload: obligation_body,
            },
        )?;
        obligations += 1;
    }

    let paid_count = paid.iter().filter(|value| **value).count() as u64;
    let total = spec.requirements.len() as u64;
    Ok(ResidualReceipt {
        requirements_total: total,
        requirements_paid: paid_count,
        requirements_unpaid: total - paid_count,
        gaps_emitted: gaps,
        obligations_emitted: obligations,
        payments_emitted: payments,
        candidate_only: true,
        semantic_promotion: false,
    })
}

pub fn read_consumer_spec_file<R: Read>(mut reader: R) -> Result<ConsumerSpec, ResidualError> {
    decode_consumer_spec(&mut reader)
}

pub fn read_to_end_exact<R: Read>(reader: &mut R, buffer: &mut Vec<u8>) -> Result<(), ResidualError> {
    loop {
        let mut chunk = [0u8; 8192];
        match reader.read(&mut chunk) {
            Ok(0) => return Ok(()),
            Ok(n) => buffer.extend_from_slice(&chunk[..n]),
            Err(error) if error.kind() == ErrorKind::Interrupted => continue,
            Err(error) => return Err(error.into()),
        }
    }
}
