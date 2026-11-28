//! Protected fleet reduction: exact source-scoped slots, no frequency-based loss
//! of externally witnessed obligations. This is a candidate-only reduction.
use std::collections::{BTreeMap, BTreeSet};
use sensiblaw_world_store::{WireRecord, WorldRecordKind};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FrameKey {
    pub source: String,
    pub sentence: u64,
    pub head: u32,
    pub lemma: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub candidate_id: String,
    pub key: FrameKey,
    pub fragment: u8,
    pub slot_lemma: String,
    pub negated: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    pub key: FrameKey,
    pub slots: Vec<Slot>,
    pub protected: bool,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepairDemand {
    pub candidate_id: String,
    pub reason: &'static str,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reduction {
    pub admitted: Vec<Frame>,
    pub repair: Vec<RepairDemand>,
    pub invalid: Vec<String>,
}
impl Reduction {
    pub fn accounted(&self) -> usize {
        self.admitted.iter().map(|f| f.slots.len()).sum::<usize>()
            + self.repair.len() + self.invalid.len()
    }
}
fn read_text(input: &[u8], at: &mut usize) -> Result<String, String> {
    let end = at.checked_add(4).ok_or("offset overflow")?;
    let size = input.get(*at..end).ok_or("truncated text size")?;
    let len = u32::from_le_bytes(size.try_into().map_err(|_| "text size")?) as usize;
    *at = end;
    let end = at.checked_add(len).ok_or("text overflow")?;
    let value = std::str::from_utf8(input.get(*at..end).ok_or("truncated text")?)
        .map_err(|_| "invalid utf8")?.to_owned();
    *at = end;
    Ok(value)
}
pub fn decode_pnf_slot(record: &WireRecord) -> Result<Option<Slot>, String> {
    if record.kind != WorldRecordKind::PnfCandidate { return Ok(None); }
    let p = &record.payload;
    if p.get(..4) != Some(b"PNF1") { return Err("expected PNF1 payload".into()); }
    if p.len() < 32 { return Err("truncated PNF1".into()); }
    let fragment = p[4];
    let sentence = u64::from_le_bytes(p[8..16].try_into().map_err(|_| "sentence")?);
    let head = u32::from_le_bytes(p[20..24].try_into().map_err(|_| "head")?);
    let mut at = 32; // PNF1 header + u64 sentence + four u32 coordinates
    if p.len() < at { return Err("truncated PNF1 coordinates".into()); }
    let _orth = read_text(p, &mut at)?;
    let slot_lemma = read_text(p, &mut at)?;
    let _head_orth = read_text(p, &mut at)?;
    let lemma = read_text(p, &mut at)?;
    if at != p.len() { return Err("trailing PNF1 bytes".into()); }
    Ok(Some(Slot {
        candidate_id: record.id.clone(),
        key: FrameKey {
            source: record.aux1.clone().ok_or("missing source manifestation")?,
            sentence, head, lemma,
        },
        fragment,
        slot_lemma,
        negated: fragment == 6,
    }))
}
/// Protected identities come from exact formal/review receipts, never stem matches.
/// Invalid or incomplete protected frames become repair demands, not admitted facts.
pub fn reduce_slots(
    slots: impl IntoIterator<Item = Slot>,
    protected_candidate_ids: &BTreeSet<String>,
) -> Reduction {
    let mut grouped: BTreeMap<FrameKey, Vec<Slot>> = BTreeMap::new();
    for slot in slots { grouped.entry(slot.key.clone()).or_default().push(slot); }
    let mut result = Reduction { admitted: vec![], repair: vec![], invalid: vec![] };
    for (key, mut slots) in grouped {
        slots.sort_by(|a,b| a.candidate_id.cmp(&b.candidate_id));
        let protected = slots.iter().any(|s| protected_candidate_ids.contains(&s.candidate_id));
        let has_actor = slots.iter().any(|s| s.fragment == 1);
        let has_theme = slots.iter().any(|s| matches!(s.fragment, 2 | 3));
        if has_actor && has_theme {
            result.admitted.push(Frame { key, slots, protected });
        } else {
            for slot in slots {
                if protected_candidate_ids.contains(&slot.candidate_id) {
                    result.repair.push(RepairDemand { candidate_id: slot.candidate_id, reason: "incomplete_structural_signature" });
                } else {
                    result.invalid.push(slot.candidate_id);
                }
            }
        }
    }
    result
}
/// Never truncate protected frames; apply optional budget only after protection.
pub fn select_batch(mut frames: Vec<Frame>, optional_budget: usize) -> Vec<Frame> {
    frames.sort_by(|a,b| a.key.cmp(&b.key));
    let mut mandatory = Vec::new();
    let mut optional = Vec::new();
    for frame in frames {
        if frame.protected { mandatory.push(frame); } else { optional.push(frame); }
    }
    mandatory.extend(optional.into_iter().take(optional_budget));
    mandatory
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HeckeFibreKey {
    pub structural_signature: &'static str,
    pub head_lemma: String,
    pub actor_lemma: String,
    pub patient_lemma: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeckeFibre {
    pub key: HeckeFibreKey,
    pub positive_count: usize,
    pub negative_count: usize,
    pub source_manifestation_ids: BTreeSet<String>,
    pub candidate_ids: Vec<String>,
    pub protected: bool,
    pub residual_severity: u8,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedClass { ProtectedFormalOrReview, ContradictionGap, OptionalRecurrent }
pub fn classify_retained_fibres(fibres: &mut [HeckeFibre]) {
    for fibre in fibres {
        if fibre.positive_count > 0 && fibre.negative_count > 0 {
            fibre.protected = true;
            fibre.residual_severity = 9;
        }
    }
}
/// Structural quotient of already-admitted frames. No stem-only formal matching.
/// A negation fragment marks the frame's negative polarity; it does not by itself
/// establish a formally contradictory proposition without matching typed fibres.
pub fn quotient_hecke_fibres(frames: impl IntoIterator<Item = Frame>) -> Vec<HeckeFibre> {
    let mut fibres: BTreeMap<HeckeFibreKey, HeckeFibre> = BTreeMap::new();
    for frame in frames {
        let actor = frame.slots.iter().find(|s| s.fragment == 1);
        let patient = frame.slots.iter().find(|s| s.fragment == 2);
        let property = frame.slots.iter().find(|s| s.fragment == 3);
        let (signature, theme) = match (actor, patient, property) {
            (Some(_), Some(p), _) => ("sig-agent-theme", Some(p)),
            (Some(_), None, Some(p)) => ("sig-agent-goal", Some(p)),
            (None, _, Some(p)) => ("sig-theme-only", Some(p)),
            _ => continue,
        };
        let key = HeckeFibreKey {
            structural_signature: signature,
            head_lemma: frame.key.lemma.clone(),
            actor_lemma: actor.map_or_else(String::new, |s| s.slot_lemma.clone()),
            patient_lemma: theme.map_or_else(String::new, |s| s.slot_lemma.clone()),
        };
        let negative = frame.slots.iter().any(|s| s.negated);
        let fibre = fibres.entry(key.clone()).or_insert_with(|| HeckeFibre {
            key, positive_count: 0, negative_count: 0,
            source_manifestation_ids: BTreeSet::new(),
            candidate_ids: Vec::new(), protected: false, residual_severity: 0,
        });
        if negative { fibre.negative_count += 1; } else { fibre.positive_count += 1; }
        fibre.protected |= frame.protected;
        fibre.source_manifestation_ids.insert(frame.key.source);
        fibre.candidate_ids.extend(frame.slots.into_iter().map(|s| s.candidate_id));
    }
    let mut result: Vec<_> = fibres.into_values().collect();
    for fibre in &mut result { fibre.candidate_ids.sort(); }
    classify_retained_fibres(&mut result);
    result
}
/// Mandatory fibres are never subject to optional budget.
pub fn select_protected_fibres(
    mut fibres: Vec<HeckeFibre>, min_occurrences: usize, optional_budget: usize,
) -> Vec<HeckeFibre> {
    classify_retained_fibres(&mut fibres);
    fibres.sort_by(|a,b| a.key.cmp(&b.key));
    let mut mandatory = Vec::new();
    let mut optional = Vec::new();
    for fibre in fibres {
        if fibre.protected { mandatory.push(fibre); }
        else if fibre.positive_count + fibre.negative_count >= min_occurrences {
            optional.push(fibre);
        }
    }
    mandatory.extend(optional.into_iter().take(optional_budget));
    mandatory
}

#[cfg(test)]
mod tests {
    use super::*;
    fn slot(id: &str, fragment: u8, sentence: u64) -> Slot {
        Slot { candidate_id: id.into(), key: FrameKey { source:"doc".into(), sentence, head:1, lemma:"prove".into() }, fragment, slot_lemma: "term".into(), negated: fragment == 6 }
    }
    #[test]
    fn pnf1_real_observation_stream_roundtrip() {
        use crate::{compile_observation_stream, encode_observation, DependencyShape, ObservationRecord};
        use sensiblaw_world_store::decode_record;
        let mut input = Vec::new();
        encode_observation(&mut input, &ObservationRecord::Manifestation {
            document_ref: "doc".into(), qid: "q".into(), language: "en".into(),
            revision_ref: "rev".into(), source_sha256: [0;32],
        }).unwrap();
        for (ordinal, shape, lemma) in [
            (1, DependencyShape::NominalSubject, "researcher"),
            (2, DependencyShape::DirectObject, "theorem"),
            (3, DependencyShape::Negation, "not"),
        ] {
            encode_observation(&mut input, &ObservationRecord::Token {
                document_ref: "doc".into(), sentence_id: 1, local_ordinal: ordinal,
                start_char: ordinal, end_char: ordinal+1, head_ordinal: 0,
                shape, orth: lemma.into(), lemma: lemma.into(),
                head_orth: "proves".into(), head_lemma: "prove".into(),
            }).unwrap();
        }
        let mut output = Vec::new();
        let receipt = compile_observation_stream(&mut input.as_slice(), &mut output, 1).unwrap();
        let mut reader = output.as_slice();
        let mut slots = Vec::new();
        while let Some(record) = decode_record(&mut reader).unwrap() {
            if let Some(slot) = decode_pnf_slot(&record).unwrap() { slots.push(slot); }
        }
        assert_eq!(slots.len() as u64, receipt.pnf_candidates);
        assert!(slots.iter().any(|s| s.slot_lemma == "theorem"));
        assert!(slots.iter().any(|s| s.negated));
        let reduction = reduce_slots(slots, &BTreeSet::new());
        assert_eq!(reduction.accounted() as u64, receipt.pnf_candidates);
        assert_eq!(reduction.admitted.len(), 1);
    }
    #[test]
    fn contradiction_survives_zero_optional_budget() {
        let ids = BTreeSet::new();
        let mut pos = vec![slot("a",1,1),slot("b",2,1)];
        let mut neg = vec![slot("c",1,2),slot("d",2,2),slot("e",6,2)];
        pos.append(&mut neg);
        let r = reduce_slots(pos, &ids);
        let fibres = quotient_hecke_fibres(r.admitted);
        assert_eq!(fibres.len(), 1);
        assert_eq!(fibres[0].positive_count, 1);
        assert_eq!(fibres[0].negative_count, 1);
        let selected = select_protected_fibres(fibres, 2, 0);
        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].residual_severity, 9);
    }
    #[test]
    fn singleton_protected_incomplete_becomes_repair() {
        let ids = BTreeSet::from(["hole".to_owned()]);
        let r = reduce_slots([slot("hole", 1, 1)], &ids);
        assert_eq!(r.accounted(), 1);
        assert_eq!(r.repair[0].candidate_id, "hole");
    }
    #[test]
    fn protected_frame_survives_zero_optional_budget() {
        let ids = BTreeSet::from(["formal".to_owned()]);
        let r = reduce_slots([slot("formal",1,1),slot("theme",2,1),slot("x",1,2),slot("y",2,2)], &ids);
        assert_eq!(r.accounted(), 4);
        let chosen = select_batch(r.admitted, 0);
        assert_eq!(chosen.len(), 1);
        assert!(chosen[0].protected);
    }
}
