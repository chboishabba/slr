//! S28.SB reader projection for operational/work chronology.

use std::collections::BTreeMap;

use sensiblaw_core::operational_state::{
    OperationalEvent, OperationalSemanticLink,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationalTimelineEntry {
    pub event: OperationalEvent,
    pub links: Vec<OperationalSemanticLink>,
    pub target_refs: Vec<String>,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct OperationalTimelineProjection {
    pub entries: Vec<OperationalTimelineEntry>,
    pub linked_target_count: usize,
    pub creates_semantic_authority: bool,
    pub pays_evidence: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OperationalTimelineError {
    InvalidEvent(String),
    InvalidLink(String),
    LinkEventMissing(String),
}

pub fn project_operational_timeline(
    events: &[OperationalEvent],
    links: &[OperationalSemanticLink],
) -> Result<OperationalTimelineProjection, OperationalTimelineError> {
    let mut links_by_event: BTreeMap<String, Vec<OperationalSemanticLink>> = BTreeMap::new();
    for link in links {
        link.validate()
            .map_err(|_| OperationalTimelineError::InvalidLink(link.link_ref.clone()))?;
        links_by_event
            .entry(link.operational_event_ref.clone())
            .or_default()
            .push(link.clone());
    }

    let event_refs = events
        .iter()
        .map(|event| event.operational_event_ref.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for event_ref in links_by_event.keys() {
        if !event_refs.contains(event_ref.as_str()) {
            return Err(OperationalTimelineError::LinkEventMissing(event_ref.clone()));
        }
    }

    let mut entries = Vec::with_capacity(events.len());
    let mut target_refs = std::collections::BTreeSet::new();

    for event in events {
        event.validate()
            .map_err(|_| OperationalTimelineError::InvalidEvent(
                event.operational_event_ref.clone(),
            ))?;
        let mut event_links = links_by_event
            .remove(&event.operational_event_ref)
            .unwrap_or_default();
        event_links.sort_by(|left, right| left.link_ref.cmp(&right.link_ref));

        let mut event_targets = event_links
            .iter()
            .map(|link| link.target_ref.clone())
            .collect::<Vec<_>>();
        event_targets.sort();
        event_targets.dedup();
        target_refs.extend(event_targets.iter().cloned());

        entries.push(OperationalTimelineEntry {
            event: event.clone(),
            links: event_links,
            target_refs: event_targets,
            creates_semantic_authority: false,
            pays_evidence: false,
            claim_truth_promoted: false,
        });
    }

    entries.sort_by(|left, right| {
        left.event
            .state_date
            .cmp(&right.event.state_date)
            .then_with(|| left.event.start_time_ref.cmp(&right.event.start_time_ref))
            .then_with(|| {
                left.event
                    .operational_event_ref
                    .cmp(&right.event.operational_event_ref)
            })
    });

    Ok(OperationalTimelineProjection {
        entries,
        linked_target_count: target_refs.len(),
        creates_semantic_authority: false,
        pays_evidence: false,
        claim_truth_promoted: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_core::operational_state::{
        OperationalEventKind, OperationalSemanticRelationKind, OperationalTargetKind,
    };

    fn event(reference: &str, start: &str) -> OperationalEvent {
        OperationalEvent {
            operational_event_ref: reference.into(),
            producer_event_ref: format!("producer:{reference}"),
            producer_ref: "statibaker:sb.sessionize.v0".into(),
            state_date: "2026-09-24".into(),
            start_time_ref: start.into(),
            end_time_ref: start.into(),
            primary_app_ref: Some("chatgpt".into()),
            label: "work".into(),
            provenance_refs: vec!["sha256:x".into()],
            producer_observed: true,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            kind: OperationalEventKind::Session,
        }
    }

    #[test]
    fn operational_timeline_sorts_without_collapsing_into_semantic_event() {
        let a = event("operational:b", "2026-09-24T14:02:00+10:00");
        let b = event("operational:a", "2026-09-24T14:01:00+10:00");
        let link = OperationalSemanticLink {
            link_ref: "link:1".into(),
            operational_event_ref: b.operational_event_ref.clone(),
            target_ref: "event:gwb:1".into(),
            target_kind: OperationalTargetKind::SemanticEvent,
            relation_kind: OperationalSemanticRelationKind::Researched,
            relationship_receipt_ref: "review:link:1".into(),
            reviewed_link: true,
            creates_semantic_identity: false,
            creates_semantic_authority: false,
            pays_evidence: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };

        let projection = project_operational_timeline(&[a, b], &[link]).unwrap();
        assert_eq!(
            projection.entries[0].event.operational_event_ref,
            "operational:a"
        );
        assert_eq!(projection.entries[0].target_refs, vec!["event:gwb:1"]);
        assert!(!projection.pays_evidence);
        assert!(!projection.claim_truth_promoted);
    }
}
