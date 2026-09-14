use std::io::{Read, Write};

use sensiblaw_world_store::{decode_record, encode_record, WireRecord, WorldRecordKind};
use thiserror::Error;

const MAX_TEXT_BYTES: usize = 4 << 20;

#[derive(Debug, Error)]
pub enum PlannerError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("world wire error: {0}")]
    WorldWire(#[from] sensiblaw_world_store::WorldStoreError),
    #[error("invalid obligation: {0}")]
    InvalidObligation(String),
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProducerFamily {
    ArticleSemantic = 1,
    RevisionTemporal = 2,
    ParserRepair = 3,
}

impl ProducerFamily {
    pub fn label(self) -> &'static str {
        match self {
            Self::ArticleSemantic => "article-semantic",
            Self::RevisionTemporal => "revision-temporal",
            Self::ParserRepair => "parser-repair",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ObligationCoordinate {
    fragment: u8,
    consumer_id: String,
    requirement_id: String,
    scope_tag: u8,
    source_ref: Option<String>,
}

fn producer_for_fragment(fragment: u8) -> Result<ProducerFamily, PlannerError> {
    match fragment {
        1..=8 | 10..=11 => Ok(ProducerFamily::ArticleSemantic),
        9 => Ok(ProducerFamily::RevisionTemporal),
        12 => Ok(ProducerFamily::ParserRepair),
        other => Err(PlannerError::InvalidObligation(format!("unknown fragment kind {other}"))),
    }
}

fn read_u32<R: Read>(reader: &mut R) -> Result<u32, PlannerError> {
    let mut bytes = [0u8; 4];
    reader.read_exact(&mut bytes)?;
    Ok(u32::from_le_bytes(bytes))
}

fn read_text<R: Read>(reader: &mut R) -> Result<String, PlannerError> {
    let len = read_u32(reader)? as usize;
    if len > MAX_TEXT_BYTES {
        return Err(PlannerError::InvalidObligation("text field too large".into()));
    }
    let mut bytes = vec![0u8; len];
    reader.read_exact(&mut bytes)?;
    String::from_utf8(bytes).map_err(|_| PlannerError::InvalidObligation("text is not UTF-8".into()))
}

fn write_u32<W: Write>(writer: &mut W, value: u32) -> Result<(), PlannerError> {
    writer.write_all(&value.to_le_bytes())?;
    Ok(())
}

fn write_text<W: Write>(writer: &mut W, value: &str) -> Result<(), PlannerError> {
    if value.len() > MAX_TEXT_BYTES {
        return Err(PlannerError::InvalidObligation("text field too large".into()));
    }
    write_u32(writer, value.len() as u32)?;
    writer.write_all(value.as_bytes())?;
    Ok(())
}

fn parse_obligation(record: &WireRecord) -> Result<Option<ObligationCoordinate>, PlannerError> {
    if record.kind != WorldRecordKind::Obligation {
        return Ok(None);
    }
    if record.payload.len() < 8 || &record.payload[..4] != b"OBL1" {
        return Err(PlannerError::InvalidObligation("obligation payload is not OBL1".into()));
    }
    let fragment = record.payload[4];
    if record.payload[5] != 1 || record.payload[6] != 0 {
        return Err(PlannerError::InvalidObligation(
            "obligation must remain candidate-only and non-promoting".into(),
        ));
    }
    let mut cursor = std::io::Cursor::new(&record.payload[7..]);
    let consumer_id = read_text(&mut cursor)?;
    let requirement_id = read_text(&mut cursor)?;
    let mut scope = [0u8; 1];
    cursor.read_exact(&mut scope)?;
    let source_ref = match scope[0] {
        0 => None,
        1 => Some(read_text(&mut cursor)?),
        other => return Err(PlannerError::InvalidObligation(format!("unknown scope tag {other}"))),
    };
    Ok(Some(ObligationCoordinate {
        fragment,
        consumer_id,
        requirement_id,
        scope_tag: scope[0],
        source_ref,
    }))
}

fn route_body(
    producer: ProducerFamily,
    obligation_id: &str,
    coordinate: &ObligationCoordinate,
) -> Result<Vec<u8>, PlannerError> {
    let mut body = Vec::new();
    body.extend_from_slice(b"RTA1");
    body.push(producer as u8);
    body.push(coordinate.fragment);
    body.push(1); // candidate-only
    body.push(0); // semantic promotion false
    write_text(&mut body, obligation_id)?;
    write_text(&mut body, &coordinate.consumer_id)?;
    write_text(&mut body, &coordinate.requirement_id)?;
    body.push(coordinate.scope_tag);
    if let Some(source_ref) = &coordinate.source_ref {
        write_text(&mut body, source_ref)?;
    }
    Ok(body)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanReceipt {
    pub obligations_seen: u64,
    pub route_intents_emitted: u64,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
    pub route_intent_is_claim_truth: bool,
}

pub fn plan_active_frontier_stream<R: Read, W: Write>(
    reader: &mut R,
    writer: &mut W,
) -> Result<PlanReceipt, PlannerError> {
    let mut obligations_seen = 0u64;
    let mut route_intents_emitted = 0u64;

    while let Some(record) = decode_record(reader)? {
        let Some(coordinate) = parse_obligation(&record)? else {
            continue;
        };
        obligations_seen += 1;
        let producer = producer_for_fragment(coordinate.fragment)?;
        let body = route_body(producer, &record.id, &coordinate)?;
        encode_record(
            writer,
            &WireRecord {
                kind: WorldRecordKind::RouteAction,
                id: format!("route-intent:{}", record.id),
                iteration_index: record.iteration_index,
                aux1: Some(producer.label().into()),
                payload: body,
            },
        )?;
        route_intents_emitted += 1;
    }

    Ok(PlanReceipt {
        obligations_seen,
        route_intents_emitted,
        candidate_only: true,
        semantic_promotion: false,
        route_intent_is_claim_truth: false,
    })
}
