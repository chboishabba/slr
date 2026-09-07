//! Revision-locked source-unit / observation / review handoff ABI.
//!
//! This crate ports the reusable boundary contracts from mature SensibLaw and
//! the formal DASHI.Wikimedia interface without importing Python, HTTP, database,
//! parser, or publication machinery into the direct-delta runtime.
//!
//! Core law: consuming a source artifact never grants source authority or
//! semantic-promotion authority. Review and promotion remain separate stages.

pub mod bundle;
pub mod external_reference;

pub const SOURCE_UNIT_SCHEMA: &str = "sl.source_unit.v1";
pub const SOURCE_HANDOFF_SCHEMA: &str = "sl.source_handoff.v0_1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevisionId {
    Text(String),
    Integer(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetrievalMethod {
    PdfSnapshot,
    HtmlSnapshot,
    WikiRevision,
    CsvSnapshot,
    ChatCapture,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceType {
    Pdf,
    Html,
    Wiki,
    Csv,
    Chat,
    Text,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentFormat {
    Text,
    Html,
    Markdown,
    Csv,
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRevision {
    pub revision_id: RevisionId,
    pub revision_timestamp: String,
    pub retrieval_method: RetrievalMethod,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOrigin {
    pub source_type: SourceType,
    pub source_url: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAnchor {
    pub anchor_id: String,
    pub start: usize,
    pub end: usize,
    pub label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceUnit {
    pub schema_version: &'static str,
    pub source_id: String,
    pub entity_qid: String,
    pub source_unit_id: String,
    pub revision: SourceRevision,
    pub origin: SourceOrigin,
    pub content_format: ContentFormat,
    pub text: String,
    pub anchors: Vec<SourceAnchor>,
    pub metadata_ref: String,
    pub source_receipt_ref: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceUnitError {
    EmptyRequiredField,
    InvalidQid,
    EmptyText,
    InvalidAnchor,
}

impl SourceUnit {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        source_id: String,
        entity_qid: String,
        source_unit_id: String,
        revision: SourceRevision,
        origin: SourceOrigin,
        content_format: ContentFormat,
        text: String,
        anchors: Vec<SourceAnchor>,
        metadata_ref: String,
        source_receipt_ref: String,
    ) -> Result<Self, SourceUnitError> {
        if [&source_id, &source_unit_id, &revision.revision_timestamp, &metadata_ref, &source_receipt_ref]
            .iter()
            .any(|value| value.is_empty())
        {
            return Err(SourceUnitError::EmptyRequiredField);
        }
        if !is_qid(&entity_qid) {
            return Err(SourceUnitError::InvalidQid);
        }
        if text.is_empty() {
            return Err(SourceUnitError::EmptyText);
        }
        if anchors.iter().any(|anchor| {
            anchor.anchor_id.is_empty() || anchor.start > anchor.end || anchor.end > text.len()
        }) {
            return Err(SourceUnitError::InvalidAnchor);
        }
        Ok(Self {
            schema_version: SOURCE_UNIT_SCHEMA,
            source_id,
            entity_qid,
            source_unit_id,
            revision,
            origin,
            content_format,
            text,
            anchors,
            metadata_ref,
            source_receipt_ref,
        })
    }
}

fn is_qid(value: &str) -> bool {
    let Some(digits) = value.strip_prefix('Q') else {
        return false;
    };
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferenceRole {
    SourceCandidate,
    ProvenanceOnly,
    Unresolved,
}

pub fn wikidata_reference_role(property_id: &str) -> ReferenceRole {
    match property_id {
        "P248" | "P854" => ReferenceRole::SourceCandidate,
        "P143" => ReferenceRole::ProvenanceOnly,
        _ => ReferenceRole::Unresolved,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidatePolarity {
    Positive,
    Negative,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationClaim {
    pub observation_id: String,
    pub source_unit_id: String,
    pub anchor_refs: Vec<String>,
    pub subject_qid_candidate: String,
    pub predicate_ref: String,
    pub object_ref: String,
    pub qualifier_refs: Vec<String>,
    pub source_reference_properties: Vec<String>,
    pub polarity: CandidatePolarity,
    pub extraction_ref: String,
    pub source_identity_preserved: bool,
    pub anchors_preserved: bool,
    pub creates_world_truth: bool,
    pub owns_source_authority: bool,
    pub owns_semantic_promotion: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObservationError {
    EmptyRequiredField,
    SourceMismatch,
    UnknownAnchor,
    InvalidQid,
}

impl ObservationClaim {
    #[allow(clippy::too_many_arguments)]
    pub fn from_source_unit(
        source: &SourceUnit,
        observation_id: String,
        anchor_refs: Vec<String>,
        subject_qid_candidate: String,
        predicate_ref: String,
        object_ref: String,
        qualifier_refs: Vec<String>,
        source_reference_properties: Vec<String>,
        polarity: CandidatePolarity,
        extraction_ref: String,
    ) -> Result<Self, ObservationError> {
        if [&observation_id, &predicate_ref, &object_ref, &extraction_ref]
            .iter()
            .any(|value| value.is_empty())
        {
            return Err(ObservationError::EmptyRequiredField);
        }
        if subject_qid_candidate != source.entity_qid || !is_qid(&subject_qid_candidate) {
            return Err(ObservationError::SourceMismatch);
        }
        if anchor_refs.iter().any(|anchor_ref| {
            !source.anchors.iter().any(|anchor| &anchor.anchor_id == anchor_ref)
        }) {
            return Err(ObservationError::UnknownAnchor);
        }
        Ok(Self {
            observation_id,
            source_unit_id: source.source_unit_id.clone(),
            anchor_refs,
            subject_qid_candidate,
            predicate_ref,
            object_ref,
            qualifier_refs,
            source_reference_properties,
            polarity,
            extraction_ref,
            source_identity_preserved: true,
            anchors_preserved: true,
            creates_world_truth: false,
            owns_source_authority: false,
            owns_semantic_promotion: false,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewRoute {
    FullAuto,
    SplitAuto,
    RepairPlusMigrateReview,
    ReviewOnlyTypedHold,
    ManualReconstruction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MigrationDisposition {
    SafeEquivalent,
    SafeWithReferenceTransfer,
    QualifierDrift,
    ReferenceDrift,
    AmbiguousSemantics,
    NonEquivalent,
    NeedsHumanReview,
    Abstain,
    SplitRequired,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewPacket {
    pub packet_id: String,
    pub source_unit_id: String,
    pub split_plan_id: String,
    pub route: ReviewRoute,
    pub disposition: MigrationDisposition,
    pub split_axes: Vec<String>,
    pub expected_qualifier_properties: Vec<String>,
    pub expected_reference_properties: Vec<String>,
    pub follow_receipt_refs: Vec<String>,
    pub uncertainty_flags: Vec<String>,
    pub recommended_next_step: String,
    pub split_plan_remains_execution_baseline: bool,
    pub unresolved_uncertainty_visible: bool,
    pub owns_source_authority: bool,
    pub owns_semantic_promotion: bool,
}

impl ReviewPacket {
    #[allow(clippy::too_many_arguments)]
    pub fn review_only(
        source: &SourceUnit,
        packet_id: String,
        split_plan_id: String,
        route: ReviewRoute,
        disposition: MigrationDisposition,
        split_axes: Vec<String>,
        expected_qualifier_properties: Vec<String>,
        expected_reference_properties: Vec<String>,
        follow_receipt_refs: Vec<String>,
        uncertainty_flags: Vec<String>,
        recommended_next_step: String,
    ) -> Self {
        Self {
            packet_id,
            source_unit_id: source.source_unit_id.clone(),
            split_plan_id,
            route,
            disposition,
            split_axes,
            expected_qualifier_properties,
            expected_reference_properties,
            follow_receipt_refs,
            uncertainty_flags,
            recommended_next_step,
            split_plan_remains_execution_baseline: true,
            unresolved_uncertainty_visible: true,
            owns_source_authority: false,
            owns_semantic_promotion: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn nat_source_unit() -> SourceUnit {
        let text = "Since the cretion of P14143 I have to migrate most of the P5991 statements with their references and qualifiers to use this new property."
            .repeat(20);
        SourceUnit::new(
            "wikidata_user_sandbox:nat_wdu:p5991_p14143:provided_snapshot:2026-04-01".into(),
            "Q10884".into(),
            "unit:wikidata_user_sandbox:nat_wdu:p5991_p14143:2026-04-01".into(),
            SourceRevision {
                revision_id: RevisionId::Text("provided_snapshot_2026-04-01".into()),
                revision_timestamp: "2026-04-01T00:00:00+10:00".into(),
                retrieval_method: RetrievalMethod::WikiRevision,
            },
            SourceOrigin {
                source_type: SourceType::Wiki,
                source_url: Some("https://www.wikidata.org/wiki/User:Nat_(WDU)/Sandbox/Fossil_fuel_industries/Migrate_from_carbon_footprint_to_GHG_emissions".into()),
                title: Some("User:Nat (WDU)/Sandbox/Fossil fuel industries/Migrate from carbon footprint to GHG emissions".into()),
            },
            ContentFormat::Text,
            text,
            vec![
                SourceAnchor { anchor_id: "goal".into(), start: 0, end: 257, label: Some("migration_goal".into()) },
                SourceAnchor { anchor_id: "qualifier_family".into(), start: 588, end: 847, label: Some("expected_qualifier_family".into()) },
                SourceAnchor { anchor_id: "reference_family".into(), start: 1047, end: 1289, label: Some("expected_reference_family".into()) },
                SourceAnchor { anchor_id: "query_anchor".into(), start: 1655, end: 1737, label: Some("query_anchor".into()) },
            ],
            "metadata:P5991->P14143".into(),
            "sensiblaw-fixture:d25cddf".into(),
        )
        .unwrap()
    }

    #[test]
    fn p143_is_provenance_only_while_p248_p854_are_source_candidates() {
        assert_eq!(wikidata_reference_role("P248"), ReferenceRole::SourceCandidate);
        assert_eq!(wikidata_reference_role("P854"), ReferenceRole::SourceCandidate);
        assert_eq!(wikidata_reference_role("P143"), ReferenceRole::ProvenanceOnly);
    }

    #[test]
    fn nat_observation_preserves_source_and_never_promotes() {
        let source = nat_source_unit();
        let observation = ObservationClaim::from_source_unit(
            &source,
            "nat:p5991-p14143:observation".into(),
            vec!["goal".into(), "qualifier_family".into(), "reference_family".into()],
            "Q10884".into(),
            "candidate migration relation P5991 -> P14143".into(),
            "target semantics unresolved".into(),
            vec!["P459".into(), "P3831".into(), "P518".into(), "P580".into(), "P582".into()],
            vec!["P854".into()],
            CandidatePolarity::Unresolved,
            "sl.source_unit.v1 -> ObservationClaimPayload".into(),
        )
        .unwrap();
        assert_eq!(observation.source_unit_id, source.source_unit_id);
        assert!(observation.source_identity_preserved);
        assert!(observation.anchors_preserved);
        assert!(!observation.creates_world_truth);
        assert!(!observation.owns_source_authority);
        assert!(!observation.owns_semantic_promotion);
    }

    #[test]
    fn nat_review_packet_stays_split_required_and_review_only() {
        let source = nat_source_unit();
        let packet = ReviewPacket::review_only(
            &source,
            "nat:packet".into(),
            "nat:split-plan".into(),
            ReviewRoute::ReviewOnlyTypedHold,
            MigrationDisposition::SplitRequired,
            vec!["scope".into(), "time".into(), "determination-method".into(), "reference-transfer".into()],
            vec!["P3831".into(), "P459".into(), "P518".into(), "P580".into(), "P582".into()],
            vec!["P854".into()],
            vec!["follow:https://w.wiki/KR5d".into()],
            vec!["semantic-equivalence-unresolved".into()],
            "review split-heavy rows; do not widen blind direct rewrites".into(),
        );
        assert_eq!(packet.disposition, MigrationDisposition::SplitRequired);
        assert!(packet.split_plan_remains_execution_baseline);
        assert!(packet.unresolved_uncertainty_visible);
        assert!(!packet.owns_source_authority);
        assert!(!packet.owns_semantic_promotion);
    }

    #[test]
    fn observation_rejects_unowned_anchor() {
        let source = nat_source_unit();
        let result = ObservationClaim::from_source_unit(
            &source,
            "obs".into(),
            vec!["not-owned".into()],
            "Q10884".into(),
            "P5991".into(),
            "P14143".into(),
            vec![],
            vec![],
            CandidatePolarity::Unresolved,
            "extract".into(),
        );
        assert_eq!(result, Err(ObservationError::UnknownAnchor));
    }
}
