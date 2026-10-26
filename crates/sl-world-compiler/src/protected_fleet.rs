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
    if p.len() < 28 { return Err("truncated PNF1".into()); }
    let fragment = p[4];
    let sentence = u64::from_le_bytes(p[8..16].try_into().map_err(|_| "sentence")?);
    let head = u32::from_le_bytes(p[20..24].try_into().map_err(|_| "head")?);
    let mut at = 36; // PNF1 header + u64 sentence + 5 u32 coordinates
    if p.len() < at { return Err("truncated PNF1 coordinates".into()); }
    let _orth = read_text(p, &mut at)?;
    let _lemma = read_text(p, &mut at)?;
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
#[cfg(test)]
mod tests {
    use super::*;
    fn slot(id: &str, fragment: u8, sentence: u64) -> Slot {
        Slot { candidate_id: id.into(), key: FrameKey { source:"doc".into(), sentence, head:1, lemma:"prove".into() }, fragment }
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
