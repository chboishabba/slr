//! S28.GWB heterogeneous-corpus chronology capstone.
//!
//! This adapter consumes an explicitly reviewed bounded manifest over the
//! already-retained GWB corpus. It does not mine events automatically from
//! sentence adjacency, QIDs, person/date overlap, or source-family identity.
//! Every persisted statement is checked against the exact source span already
//! present in Postgres via the generic M12 statement store.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use sensiblaw_core::{
    chronology_contestation::{
        ClaimLeaf, ClaimLeafKind, ClaimReviewState, ContestationRelation,
        ContestationRelationKind, PropositionRoot, TemporalAssertion, TemporalForm,
    },
    review_workstation::{
        ReviewAction, ReviewItem, ReviewItemKind, ReviewStatus,
    },
};
use serde::Deserialize;
use thiserror::Error;

use crate::{
    canonical_observation_event_link_ref, canonical_statement_observation_link_ref,
    canonical_statement_ref, install_chronology_contestation_schema,
    install_review_workstation_schema, install_statement_trace_schema,
    persist_claim_leaf, persist_contestation_relation, persist_event_claim_link,
    persist_event_temporal_link, persist_observation_event_link,
    persist_proposition_root, persist_review_item, persist_source_statement,
    persist_statement_observation_link, persist_temporal_assertion, DatabaseConfig,
    EventClaimLink, EventTemporalLink, ExactSourceSpan, ObservationEventLink,
    SourceStatementEnvelope, StatementObservationDisposition, StatementObservationLink,
    StatementOrigin,
};

pub const GWB_CAPSTONE_SCHEMA: &str =
    "sensiblaw.gwb-heterogeneous-chronology-capstone.v0_1";

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbChronologyCapstoneManifest {
    pub schema: String,
    pub matter_ref: String,
    pub handoff_ref: String,
    pub max_selected_statements: usize,
    pub statements: Vec<GwbReviewedStatement>,
    pub event_joins: Vec<GwbReviewedEventJoin>,
    pub temporal_assertions: Vec<GwbTemporalAssertion>,
    pub proposition_roots: Vec<GwbPropositionRoot>,
    pub claim_leaves: Vec<GwbClaimLeaf>,
    pub contestation_relations: Vec<GwbContestationRelation>,
    #[serde(default)]
    pub review_items: Vec<GwbReviewItem>,
    pub candidate_only: bool,
    pub semantic_promotion: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbReviewedStatement {
    pub statement_key: String,
    pub document_ordinal: u32,
    pub document_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub start_char: u32,
    pub end_char: u32,
    pub literal_text: String,
    pub source_family_ref: String,
    pub source_role_ref: String,
    pub candidate_pnf_ref: String,
    pub observation_ref: String,
    pub parser_receipt_ref: String,
    pub parse_review_ref: Option<String>,
    pub admission_receipt_ref: Option<String>,
    pub disposition: GwbStatementDisposition,
    pub qualification_ref: Option<String>,
    pub reviewed: bool,
    pub origin: GwbStatementOrigin,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbStatementOrigin {
    InitialIntake,
    ResearchReentry,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbStatementDisposition {
    Candidate,
    ParseReviewed,
    SemanticallyAdmitted,
    Rejected,
    Abstained,
    Qualified,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbReviewedEventJoin {
    pub event_ref: String,
    pub observation_refs: Vec<String>,
    pub assembly_receipt_ref: String,
    pub join_basis: String,
    pub reviewed: bool,
    pub automatic_join: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbTemporalAssertion {
    pub temporal_ref: String,
    pub event_ref: String,
    pub form: GwbTemporalForm,
    #[serde(default)]
    pub statement_keys: Vec<String>,
    #[serde(default)]
    pub observation_refs: Vec<String>,
    pub review_ref: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GwbTemporalForm {
    ExactInstant { instant_ref: String },
    ExactDate { date_ref: String },
    Interval { start_ref: String, end_ref: String },
    Approximate { label: String },
    RelativeBefore { event_ref: String },
    RelativeAfter { event_ref: String },
    Contemporaneous { event_ref: String },
    Undated,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbPropositionRoot {
    pub proposition_ref: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbClaimLeaf {
    pub claim_ref: String,
    pub proposition_ref: String,
    pub kind: GwbClaimLeafKind,
    pub speaker_ref: Option<String>,
    pub statement_keys: Vec<String>,
    #[serde(default)]
    pub observation_refs: Vec<String>,
    #[serde(default)]
    pub temporal_refs: Vec<String>,
    #[serde(default)]
    pub scope_refs: Vec<String>,
    pub review_state: GwbClaimReviewState,
    pub review_ref: Option<String>,
    pub event_ref: Option<String>,
    pub event_assembly_receipt_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbClaimLeafKind {
    Affirmation,
    Denial,
    Qualification,
    AlternativeAccount,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbClaimReviewState {
    Unreviewed,
    Accepted,
    Rejected,
    Abstained,
    Qualified,
    Superseded,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbContestationRelation {
    pub relation_ref: String,
    pub from_claim_ref: String,
    pub to_claim_ref: String,
    pub kind: GwbContestationRelationKind,
    #[serde(default)]
    pub statement_keys: Vec<String>,
    #[serde(default)]
    pub observation_refs: Vec<String>,
    pub review_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbContestationRelationKind {
    Supports,
    Qualifies,
    Denies,
    Contradicts,
    Adjacent,
    Supersedes,
    SameIncidentDifferentAccount,
    UnresolvedRelation,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct GwbReviewItem {
    pub review_item_ref: String,
    pub semantic_ref: String,
    pub item_kind: GwbReviewItemKind,
    pub reason: String,
    #[serde(default)]
    pub provenance_refs: Vec<String>,
    #[serde(default)]
    pub source_statement_keys: Vec<String>,
    pub current_status: GwbReviewStatus,
    pub available_actions: Vec<GwbReviewAction>,
    #[serde(default)]
    pub affected_consumer_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbReviewItemKind {
    PnfParse,
    Observation,
    ClaimContestation,
    EventAssembly,
    ChronologyAmbiguity,
    AuthorityFollow,
    ResearchAcquisition,
    LegalTreatment,
    ScopeHandoff,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbReviewStatus {
    Pending,
    Accepted,
    Rejected,
    Abstained,
    Qualified,
    Superseded,
    NeedsEvidence,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GwbReviewAction {
    Accept,
    Reject,
    Abstain,
    Qualify,
    Supersede,
    RequestEvidence,
    OpenSource,
    FollowAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GwbChronologyCapstoneReceipt {
    pub matter_ref: String,
    pub handoff_ref: String,
    pub statement_count: usize,
    pub source_family_count: usize,
    pub event_count: usize,
    pub temporal_assertion_count: usize,
    pub proposition_root_count: usize,
    pub claim_leaf_count: usize,
    pub contestation_relation_count: usize,
    pub review_item_count: usize,
    pub event_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

#[derive(Debug, Error)]
pub enum GwbChronologyCapstoneError {
    #[error("manifest schema mismatch")]
    SchemaMismatch,
    #[error("required capstone coordinate is empty: {0}")]
    EmptyCoordinate(&'static str),
    #[error("capstone must remain candidate-only and non-promoting")]
    PromotionNotAllowed,
    #[error("capstone selected-statement bound is invalid")]
    InvalidSelectionBound,
    #[error("capstone must contain at least one reviewed statement")]
    EmptyStatementSelection,
    #[error("duplicate statement key: {0}")]
    DuplicateStatementKey(String),
    #[error("duplicate observation ref: {0}")]
    DuplicateObservationRef(String),
    #[error("statement was not explicitly reviewed: {0}")]
    UnreviewedStatement(String),
    #[error("invalid statement span: {0}")]
    InvalidStatementSpan(String),
    #[error("semantic admission is missing an admission receipt: {0}")]
    MissingAdmissionReceipt(String),
    #[error("qualified statement is missing a qualification ref: {0}")]
    MissingQualification(String),
    #[error("event join was not explicitly reviewed: {0}")]
    UnreviewedEventJoin(String),
    #[error("automatic event join is forbidden: {0}")]
    AutomaticEventJoin(String),
    #[error("event join has no observations: {0}")]
    EmptyEventJoin(String),
    #[error("event join references unknown observation {observation_ref} for event {event_ref}")]
    UnknownObservationInEvent {
        event_ref: String,
        observation_ref: String,
    },
    #[error("duplicate event ref: {0}")]
    DuplicateEventRef(String),
    #[error("temporal assertion references unknown event: {0}")]
    UnknownTemporalEvent(String),
    #[error("manifest references unknown statement key: {0}")]
    UnknownStatementKey(String),
    #[error("claim references unknown proposition root: {0}")]
    UnknownPropositionRoot(String),
    #[error("claim event link requires an assembly receipt: {0}")]
    MissingClaimEventReceipt(String),
    #[error("contestation relation references unknown claim: {0}")]
    UnknownClaim(String),
    #[error("manifest JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("manifest IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("statement trace store error: {0}")]
    StatementTrace(#[from] crate::StatementTraceStoreError),
    #[error("chronology/contestation store error: {0}")]
    Chronology(#[from] crate::ChronologyContestationStoreError),
    #[error("review workstation store error: {0}")]
    Review(#[from] crate::ReviewWorkstationStoreError),
}

impl GwbChronologyCapstoneManifest {
    pub fn validate(&self) -> Result<(), GwbChronologyCapstoneError> {
        if self.schema != GWB_CAPSTONE_SCHEMA {
            return Err(GwbChronologyCapstoneError::SchemaMismatch);
        }
        require("matter_ref", &self.matter_ref)?;
        require("handoff_ref", &self.handoff_ref)?;
        if !self.candidate_only || self.semantic_promotion {
            return Err(GwbChronologyCapstoneError::PromotionNotAllowed);
        }
        if self.max_selected_statements == 0
            || self.statements.len() > self.max_selected_statements
        {
            return Err(GwbChronologyCapstoneError::InvalidSelectionBound);
        }
        if self.statements.is_empty() {
            return Err(GwbChronologyCapstoneError::EmptyStatementSelection);
        }

        let mut statement_keys = BTreeSet::new();
        let mut observation_refs = BTreeSet::new();
        for statement in &self.statements {
            for (name, value) in [
                ("statement_key", statement.statement_key.as_str()),
                ("document_ref", statement.document_ref.as_str()),
                ("source_revision_ref", statement.source_revision_ref.as_str()),
                ("span_ref", statement.span_ref.as_str()),
                ("literal_text", statement.literal_text.as_str()),
                ("source_family_ref", statement.source_family_ref.as_str()),
                ("source_role_ref", statement.source_role_ref.as_str()),
                ("candidate_pnf_ref", statement.candidate_pnf_ref.as_str()),
                ("observation_ref", statement.observation_ref.as_str()),
                ("parser_receipt_ref", statement.parser_receipt_ref.as_str()),
            ] {
                require(name, value)?;
            }
            if statement.document_ordinal == 0
                || statement.start_char >= statement.end_char
            {
                return Err(GwbChronologyCapstoneError::InvalidStatementSpan(
                    statement.statement_key.clone(),
                ));
            }
            if !statement.reviewed {
                return Err(GwbChronologyCapstoneError::UnreviewedStatement(
                    statement.statement_key.clone(),
                ));
            }
            if !statement_keys.insert(statement.statement_key.clone()) {
                return Err(GwbChronologyCapstoneError::DuplicateStatementKey(
                    statement.statement_key.clone(),
                ));
            }
            if !observation_refs.insert(statement.observation_ref.clone()) {
                return Err(GwbChronologyCapstoneError::DuplicateObservationRef(
                    statement.observation_ref.clone(),
                ));
            }
            if matches!(
                statement.disposition,
                GwbStatementDisposition::SemanticallyAdmitted
            ) && statement
                .admission_receipt_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
            {
                return Err(GwbChronologyCapstoneError::MissingAdmissionReceipt(
                    statement.statement_key.clone(),
                ));
            }
            if matches!(statement.disposition, GwbStatementDisposition::Qualified)
                && statement
                    .qualification_ref
                    .as_deref()
                    .map_or(true, |value| value.trim().is_empty())
            {
                return Err(GwbChronologyCapstoneError::MissingQualification(
                    statement.statement_key.clone(),
                ));
            }
        }

        let mut event_refs = BTreeSet::new();
        for event in &self.event_joins {
            require("event_ref", &event.event_ref)?;
            require("assembly_receipt_ref", &event.assembly_receipt_ref)?;
            require("join_basis", &event.join_basis)?;
            if !event.reviewed {
                return Err(GwbChronologyCapstoneError::UnreviewedEventJoin(
                    event.event_ref.clone(),
                ));
            }
            if event.automatic_join {
                return Err(GwbChronologyCapstoneError::AutomaticEventJoin(
                    event.event_ref.clone(),
                ));
            }
            if event.observation_refs.is_empty() {
                return Err(GwbChronologyCapstoneError::EmptyEventJoin(
                    event.event_ref.clone(),
                ));
            }
            if !event_refs.insert(event.event_ref.clone()) {
                return Err(GwbChronologyCapstoneError::DuplicateEventRef(
                    event.event_ref.clone(),
                ));
            }
            for observation_ref in &event.observation_refs {
                if !observation_refs.contains(observation_ref) {
                    return Err(GwbChronologyCapstoneError::UnknownObservationInEvent {
                        event_ref: event.event_ref.clone(),
                        observation_ref: observation_ref.clone(),
                    });
                }
            }
        }

        for temporal in &self.temporal_assertions {
            require("temporal_ref", &temporal.temporal_ref)?;
            if !event_refs.contains(&temporal.event_ref) {
                return Err(GwbChronologyCapstoneError::UnknownTemporalEvent(
                    temporal.event_ref.clone(),
                ));
            }
            validate_statement_keys(&statement_keys, &temporal.statement_keys)?;
            for observation_ref in &temporal.observation_refs {
                if !observation_refs.contains(observation_ref) {
                    return Err(GwbChronologyCapstoneError::UnknownObservationInEvent {
                        event_ref: temporal.event_ref.clone(),
                        observation_ref: observation_ref.clone(),
                    });
                }
            }
        }

        let proposition_refs = self
            .proposition_roots
            .iter()
            .map(|root| root.proposition_ref.clone())
            .collect::<BTreeSet<_>>();
        for root in &self.proposition_roots {
            require("proposition_ref", &root.proposition_ref)?;
            require("proposition_label", &root.label)?;
        }

        let mut claim_refs = BTreeSet::new();
        for claim in &self.claim_leaves {
            require("claim_ref", &claim.claim_ref)?;
            if !claim_refs.insert(claim.claim_ref.clone()) {
                return Err(GwbChronologyCapstoneError::UnknownClaim(
                    claim.claim_ref.clone(),
                ));
            }
            if !proposition_refs.contains(&claim.proposition_ref) {
                return Err(GwbChronologyCapstoneError::UnknownPropositionRoot(
                    claim.proposition_ref.clone(),
                ));
            }
            validate_statement_keys(&statement_keys, &claim.statement_keys)?;
            if let Some(event_ref) = &claim.event_ref {
                if !event_refs.contains(event_ref) {
                    return Err(GwbChronologyCapstoneError::UnknownTemporalEvent(
                        event_ref.clone(),
                    ));
                }
                if claim
                    .event_assembly_receipt_ref
                    .as_deref()
                    .map_or(true, |value| value.trim().is_empty())
                {
                    return Err(GwbChronologyCapstoneError::MissingClaimEventReceipt(
                        claim.claim_ref.clone(),
                    ));
                }
            }
        }

        for relation in &self.contestation_relations {
            require("relation_ref", &relation.relation_ref)?;
            if !claim_refs.contains(&relation.from_claim_ref) {
                return Err(GwbChronologyCapstoneError::UnknownClaim(
                    relation.from_claim_ref.clone(),
                ));
            }
            if !claim_refs.contains(&relation.to_claim_ref) {
                return Err(GwbChronologyCapstoneError::UnknownClaim(
                    relation.to_claim_ref.clone(),
                ));
            }
            validate_statement_keys(&statement_keys, &relation.statement_keys)?;
        }

        for review in &self.review_items {
            require("review_item_ref", &review.review_item_ref)?;
            require("semantic_ref", &review.semantic_ref)?;
            require("reason", &review.reason)?;
            validate_statement_keys(&statement_keys, &review.source_statement_keys)?;
        }

        Ok(())
    }
}

pub fn load_gwb_chronology_capstone_manifest(
    path: impl AsRef<Path>,
) -> Result<GwbChronologyCapstoneManifest, GwbChronologyCapstoneError> {
    let raw = fs::read_to_string(path)?;
    let manifest = serde_json::from_str::<GwbChronologyCapstoneManifest>(&raw)?;
    manifest.validate()?;
    Ok(manifest)
}

pub fn materialize_gwb_chronology_capstone(
    config: &DatabaseConfig,
    manifest: &GwbChronologyCapstoneManifest,
) -> Result<GwbChronologyCapstoneReceipt, GwbChronologyCapstoneError> {
    manifest.validate()?;

    install_statement_trace_schema(config)?;
    install_chronology_contestation_schema(config)?;
    install_review_workstation_schema(config)?;

    let mut statement_refs = BTreeMap::new();
    let mut source_families = BTreeSet::new();

    for input in &manifest.statements {
        let mut statement = SourceStatementEnvelope {
            statement_ref: String::new(),
            document_ref: input.document_ref.clone(),
            source_revision_ref: input.source_revision_ref.clone(),
            span: ExactSourceSpan {
                span_ref: input.span_ref.clone(),
                start_char: input.start_char,
                end_char: input.end_char,
            },
            literal_text: input.literal_text.clone(),
            origin: match input.origin {
                GwbStatementOrigin::InitialIntake => StatementOrigin::InitialIntake,
                GwbStatementOrigin::ResearchReentry => StatementOrigin::ResearchReentry,
            },
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        statement.statement_ref = canonical_statement_ref(&statement);
        persist_source_statement(config, &statement)?;
        statement_refs.insert(input.statement_key.clone(), statement.statement_ref.clone());
        source_families.insert(input.source_family_ref.clone());

        let link = StatementObservationLink {
            link_ref: canonical_statement_observation_link_ref(
                &statement.statement_ref,
                &input.candidate_pnf_ref,
                &input.observation_ref,
            ),
            statement_ref: statement.statement_ref,
            candidate_pnf_ref: input.candidate_pnf_ref.clone(),
            observation_ref: input.observation_ref.clone(),
            parser_receipt_ref: Some(input.parser_receipt_ref.clone()),
            parse_review_ref: input.parse_review_ref.clone(),
            admission_receipt_ref: input.admission_receipt_ref.clone(),
            disposition: statement_disposition(input.disposition),
            qualification_ref: input.qualification_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_statement_observation_link(config, &link)?;
    }

    for event in &manifest.event_joins {
        for observation_ref in &event.observation_refs {
            persist_observation_event_link(
                config,
                &ObservationEventLink {
                    link_ref: canonical_observation_event_link_ref(
                        observation_ref,
                        &event.event_ref,
                    ),
                    observation_ref: observation_ref.clone(),
                    event_ref: event.event_ref.clone(),
                    assembly_receipt_ref: event.assembly_receipt_ref.clone(),
                    candidate_only: true,
                    creates_semantic_authority: false,
                    applicability_promoted: false,
                    claim_truth_promoted: false,
                },
            )?;
        }
    }

    for temporal in &manifest.temporal_assertions {
        let assertion = TemporalAssertion {
            temporal_ref: temporal.temporal_ref.clone(),
            form: temporal_form(&temporal.form),
            statement_refs: resolve_statement_keys(&statement_refs, &temporal.statement_keys)?,
            observation_refs: temporal.observation_refs.clone(),
            review_ref: temporal.review_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_temporal_assertion(config, &assertion)?;
        let event = manifest
            .event_joins
            .iter()
            .find(|event| event.event_ref == temporal.event_ref)
            .expect("manifest validation established temporal event");
        persist_event_temporal_link(
            config,
            &EventTemporalLink {
                event_ref: temporal.event_ref.clone(),
                temporal_ref: temporal.temporal_ref.clone(),
                assembly_receipt_ref: event.assembly_receipt_ref.clone(),
            },
        )?;
    }

    for root in &manifest.proposition_roots {
        persist_proposition_root(
            config,
            &PropositionRoot {
                proposition_ref: root.proposition_ref.clone(),
                label: root.label.clone(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
        )?;
    }

    for claim in &manifest.claim_leaves {
        let value = ClaimLeaf {
            claim_ref: claim.claim_ref.clone(),
            proposition_ref: claim.proposition_ref.clone(),
            kind: claim_kind(claim.kind),
            speaker_ref: claim.speaker_ref.clone(),
            statement_refs: resolve_statement_keys(&statement_refs, &claim.statement_keys)?,
            observation_refs: claim.observation_refs.clone(),
            temporal_refs: claim.temporal_refs.clone(),
            scope_refs: claim.scope_refs.clone(),
            review_state: claim_review_state(claim.review_state),
            review_ref: claim.review_ref.clone(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        persist_claim_leaf(config, &value)?;

        if let Some(event_ref) = &claim.event_ref {
            persist_event_claim_link(
                config,
                &EventClaimLink {
                    event_ref: event_ref.clone(),
                    claim_ref: claim.claim_ref.clone(),
                    assembly_receipt_ref: claim
                        .event_assembly_receipt_ref
                        .clone()
                        .expect("manifest validation established claim-event receipt"),
                },
            )?;
        }
    }

    for relation in &manifest.contestation_relations {
        persist_contestation_relation(
            config,
            &ContestationRelation {
                relation_ref: relation.relation_ref.clone(),
                from_claim_ref: relation.from_claim_ref.clone(),
                to_claim_ref: relation.to_claim_ref.clone(),
                kind: relation_kind(relation.kind),
                statement_refs: resolve_statement_keys(
                    &statement_refs,
                    &relation.statement_keys,
                )?,
                observation_refs: relation.observation_refs.clone(),
                review_ref: relation.review_ref.clone(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
        )?;
    }

    for review in &manifest.review_items {
        persist_review_item(
            config,
            &ReviewItem {
                review_item_ref: review.review_item_ref.clone(),
                semantic_ref: review.semantic_ref.clone(),
                item_kind: review_item_kind(review.item_kind),
                reason: review.reason.clone(),
                provenance_refs: review.provenance_refs.clone(),
                source_refs: resolve_statement_keys(
                    &statement_refs,
                    &review.source_statement_keys,
                )?,
                current_status: review_status(review.current_status),
                available_actions: review
                    .available_actions
                    .iter()
                    .copied()
                    .map(review_action)
                    .collect(),
                affected_consumer_refs: review.affected_consumer_refs.clone(),
                candidate_only: true,
                creates_semantic_authority: false,
                applicability_promoted: false,
                claim_truth_promoted: false,
            },
        )?;
    }

    let mut event_refs = manifest
        .event_joins
        .iter()
        .map(|event| event.event_ref.clone())
        .collect::<Vec<_>>();
    event_refs.sort();

    Ok(GwbChronologyCapstoneReceipt {
        matter_ref: manifest.matter_ref.clone(),
        handoff_ref: manifest.handoff_ref.clone(),
        statement_count: manifest.statements.len(),
        source_family_count: source_families.len(),
        event_count: manifest.event_joins.len(),
        temporal_assertion_count: manifest.temporal_assertions.len(),
        proposition_root_count: manifest.proposition_roots.len(),
        claim_leaf_count: manifest.claim_leaves.len(),
        contestation_relation_count: manifest.contestation_relations.len(),
        review_item_count: manifest.review_items.len(),
        event_refs,
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    })
}

fn require(
    name: &'static str,
    value: &str,
) -> Result<(), GwbChronologyCapstoneError> {
    if value.trim().is_empty() {
        Err(GwbChronologyCapstoneError::EmptyCoordinate(name))
    } else {
        Ok(())
    }
}

fn validate_statement_keys(
    available: &BTreeSet<String>,
    keys: &[String],
) -> Result<(), GwbChronologyCapstoneError> {
    for key in keys {
        if !available.contains(key) {
            return Err(GwbChronologyCapstoneError::UnknownStatementKey(
                key.clone(),
            ));
        }
    }
    Ok(())
}

fn resolve_statement_keys(
    statement_refs: &BTreeMap<String, String>,
    keys: &[String],
) -> Result<Vec<String>, GwbChronologyCapstoneError> {
    keys.iter()
        .map(|key| {
            statement_refs
                .get(key)
                .cloned()
                .ok_or_else(|| GwbChronologyCapstoneError::UnknownStatementKey(key.clone()))
        })
        .collect()
}

fn statement_disposition(
    value: GwbStatementDisposition,
) -> StatementObservationDisposition {
    match value {
        GwbStatementDisposition::Candidate => StatementObservationDisposition::Candidate,
        GwbStatementDisposition::ParseReviewed => {
            StatementObservationDisposition::ParseReviewed
        }
        GwbStatementDisposition::SemanticallyAdmitted => {
            StatementObservationDisposition::SemanticallyAdmitted
        }
        GwbStatementDisposition::Rejected => StatementObservationDisposition::Rejected,
        GwbStatementDisposition::Abstained => StatementObservationDisposition::Abstained,
        GwbStatementDisposition::Qualified => StatementObservationDisposition::Qualified,
    }
}

fn temporal_form(value: &GwbTemporalForm) -> TemporalForm {
    match value {
        GwbTemporalForm::ExactInstant { instant_ref } => TemporalForm::ExactInstant {
            instant_ref: instant_ref.clone(),
        },
        GwbTemporalForm::ExactDate { date_ref } => TemporalForm::ExactDate {
            date_ref: date_ref.clone(),
        },
        GwbTemporalForm::Interval { start_ref, end_ref } => TemporalForm::Interval {
            start_ref: start_ref.clone(),
            end_ref: end_ref.clone(),
        },
        GwbTemporalForm::Approximate { label } => TemporalForm::Approximate {
            label: label.clone(),
        },
        GwbTemporalForm::RelativeBefore { event_ref } => TemporalForm::RelativeBefore {
            event_ref: event_ref.clone(),
        },
        GwbTemporalForm::RelativeAfter { event_ref } => TemporalForm::RelativeAfter {
            event_ref: event_ref.clone(),
        },
        GwbTemporalForm::Contemporaneous { event_ref } => {
            TemporalForm::Contemporaneous {
                event_ref: event_ref.clone(),
            }
        }
        GwbTemporalForm::Undated => TemporalForm::Undated,
        GwbTemporalForm::Unknown => TemporalForm::Unknown,
    }
}

fn claim_kind(value: GwbClaimLeafKind) -> ClaimLeafKind {
    match value {
        GwbClaimLeafKind::Affirmation => ClaimLeafKind::Affirmation,
        GwbClaimLeafKind::Denial => ClaimLeafKind::Denial,
        GwbClaimLeafKind::Qualification => ClaimLeafKind::Qualification,
        GwbClaimLeafKind::AlternativeAccount => ClaimLeafKind::AlternativeAccount,
    }
}

fn claim_review_state(value: GwbClaimReviewState) -> ClaimReviewState {
    match value {
        GwbClaimReviewState::Unreviewed => ClaimReviewState::Unreviewed,
        GwbClaimReviewState::Accepted => ClaimReviewState::Accepted,
        GwbClaimReviewState::Rejected => ClaimReviewState::Rejected,
        GwbClaimReviewState::Abstained => ClaimReviewState::Abstained,
        GwbClaimReviewState::Qualified => ClaimReviewState::Qualified,
        GwbClaimReviewState::Superseded => ClaimReviewState::Superseded,
    }
}

fn relation_kind(value: GwbContestationRelationKind) -> ContestationRelationKind {
    match value {
        GwbContestationRelationKind::Supports => ContestationRelationKind::Supports,
        GwbContestationRelationKind::Qualifies => ContestationRelationKind::Qualifies,
        GwbContestationRelationKind::Denies => ContestationRelationKind::Denies,
        GwbContestationRelationKind::Contradicts => {
            ContestationRelationKind::Contradicts
        }
        GwbContestationRelationKind::Adjacent => ContestationRelationKind::Adjacent,
        GwbContestationRelationKind::Supersedes => ContestationRelationKind::Supersedes,
        GwbContestationRelationKind::SameIncidentDifferentAccount => {
            ContestationRelationKind::SameIncidentDifferentAccount
        }
        GwbContestationRelationKind::UnresolvedRelation => {
            ContestationRelationKind::UnresolvedRelation
        }
    }
}

fn review_item_kind(value: GwbReviewItemKind) -> ReviewItemKind {
    match value {
        GwbReviewItemKind::PnfParse => ReviewItemKind::PnfParse,
        GwbReviewItemKind::Observation => ReviewItemKind::Observation,
        GwbReviewItemKind::ClaimContestation => ReviewItemKind::ClaimContestation,
        GwbReviewItemKind::EventAssembly => ReviewItemKind::EventAssembly,
        GwbReviewItemKind::ChronologyAmbiguity => ReviewItemKind::ChronologyAmbiguity,
        GwbReviewItemKind::AuthorityFollow => ReviewItemKind::AuthorityFollow,
        GwbReviewItemKind::ResearchAcquisition => ReviewItemKind::ResearchAcquisition,
        GwbReviewItemKind::LegalTreatment => ReviewItemKind::LegalTreatment,
        GwbReviewItemKind::ScopeHandoff => ReviewItemKind::ScopeHandoff,
    }
}

fn review_status(value: GwbReviewStatus) -> ReviewStatus {
    match value {
        GwbReviewStatus::Pending => ReviewStatus::Pending,
        GwbReviewStatus::Accepted => ReviewStatus::Accepted,
        GwbReviewStatus::Rejected => ReviewStatus::Rejected,
        GwbReviewStatus::Abstained => ReviewStatus::Abstained,
        GwbReviewStatus::Qualified => ReviewStatus::Qualified,
        GwbReviewStatus::Superseded => ReviewStatus::Superseded,
        GwbReviewStatus::NeedsEvidence => ReviewStatus::NeedsEvidence,
    }
}

fn review_action(value: GwbReviewAction) -> ReviewAction {
    match value {
        GwbReviewAction::Accept => ReviewAction::Accept,
        GwbReviewAction::Reject => ReviewAction::Reject,
        GwbReviewAction::Abstain => ReviewAction::Abstain,
        GwbReviewAction::Qualify => ReviewAction::Qualify,
        GwbReviewAction::Supersede => ReviewAction::Supersede,
        GwbReviewAction::RequestEvidence => ReviewAction::RequestEvidence,
        GwbReviewAction::OpenSource => ReviewAction::OpenSource,
        GwbReviewAction::FollowAuthority => ReviewAction::FollowAuthority,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> GwbChronologyCapstoneManifest {
        GwbChronologyCapstoneManifest {
            schema: GWB_CAPSTONE_SCHEMA.into(),
            matter_ref: "matter:gwb:capstone".into(),
            handoff_ref: "gwb-world-handoff:v2:sha256:fixture".into(),
            max_selected_statements: 20,
            statements: vec![
                GwbReviewedStatement {
                    statement_key: "memoir:a".into(),
                    document_ordinal: 10,
                    document_ref: "document:gwb:10".into(),
                    source_revision_ref: "revision:gwb:10:1".into(),
                    span_ref: "span:gwb:10:100-120".into(),
                    start_char: 100,
                    end_char: 120,
                    literal_text: "A reviewed statement.".into(),
                    source_family_ref: "book".into(),
                    source_role_ref: "first-person-presidential-memoir".into(),
                    candidate_pnf_ref: "candidate:gwb:10:a".into(),
                    observation_ref: "observation:gwb:a".into(),
                    parser_receipt_ref: "parser:gwb:10:1".into(),
                    parse_review_ref: Some("review:parse:gwb:a".into()),
                    admission_receipt_ref: Some("admission:gwb:a".into()),
                    disposition: GwbStatementDisposition::SemanticallyAdmitted,
                    qualification_ref: None,
                    reviewed: true,
                    origin: GwbStatementOrigin::ResearchReentry,
                },
                GwbReviewedStatement {
                    statement_key: "wiki:b".into(),
                    document_ordinal: 2,
                    document_ref: "document:gwb:2".into(),
                    source_revision_ref: "revision:gwb:wiki:2".into(),
                    span_ref: "span:gwb:2:20-40".into(),
                    start_char: 20,
                    end_char: 40,
                    literal_text: "Another statement.".into(),
                    source_family_ref: "wikipedia".into(),
                    source_role_ref: "secondary-encyclopedia-biography".into(),
                    candidate_pnf_ref: "candidate:gwb:2:b".into(),
                    observation_ref: "observation:gwb:b".into(),
                    parser_receipt_ref: "parser:gwb:2:1".into(),
                    parse_review_ref: Some("review:parse:gwb:b".into()),
                    admission_receipt_ref: None,
                    disposition: GwbStatementDisposition::ParseReviewed,
                    qualification_ref: None,
                    reviewed: true,
                    origin: GwbStatementOrigin::ResearchReentry,
                },
            ],
            event_joins: vec![GwbReviewedEventJoin {
                event_ref: "event:gwb:1".into(),
                observation_refs: vec![
                    "observation:gwb:a".into(),
                    "observation:gwb:b".into(),
                ],
                assembly_receipt_ref: "review:event-assembly:gwb:1".into(),
                join_basis: "explicit operator-reviewed same incident".into(),
                reviewed: true,
                automatic_join: false,
            }],
            temporal_assertions: vec![
                GwbTemporalAssertion {
                    temporal_ref: "temporal:gwb:exact".into(),
                    event_ref: "event:gwb:1".into(),
                    form: GwbTemporalForm::ExactDate {
                        date_ref: "date:2001-01-20".into(),
                    },
                    statement_keys: vec!["wiki:b".into()],
                    observation_refs: vec!["observation:gwb:b".into()],
                    review_ref: Some("review:temporal:gwb:exact".into()),
                },
                GwbTemporalAssertion {
                    temporal_ref: "temporal:gwb:relative".into(),
                    event_ref: "event:gwb:1".into(),
                    form: GwbTemporalForm::RelativeAfter {
                        event_ref: "event:gwb:prior".into(),
                    },
                    statement_keys: vec!["memoir:a".into()],
                    observation_refs: vec!["observation:gwb:a".into()],
                    review_ref: Some("review:temporal:gwb:relative".into()),
                },
            ],
            proposition_roots: vec![GwbPropositionRoot {
                proposition_ref: "proposition:gwb:p1".into(),
                label: "Reviewed proposition".into(),
            }],
            claim_leaves: vec![
                GwbClaimLeaf {
                    claim_ref: "claim:gwb:a".into(),
                    proposition_ref: "proposition:gwb:p1".into(),
                    kind: GwbClaimLeafKind::Affirmation,
                    speaker_ref: Some("Q207".into()),
                    statement_keys: vec!["memoir:a".into()],
                    observation_refs: vec!["observation:gwb:a".into()],
                    temporal_refs: vec!["temporal:gwb:relative".into()],
                    scope_refs: vec!["matter:gwb:capstone".into()],
                    review_state: GwbClaimReviewState::Accepted,
                    review_ref: Some("review:claim:gwb:a".into()),
                    event_ref: Some("event:gwb:1".into()),
                    event_assembly_receipt_ref: Some(
                        "review:event-assembly:gwb:1".into(),
                    ),
                },
                GwbClaimLeaf {
                    claim_ref: "claim:gwb:b".into(),
                    proposition_ref: "proposition:gwb:p1".into(),
                    kind: GwbClaimLeafKind::Qualification,
                    speaker_ref: None,
                    statement_keys: vec!["wiki:b".into()],
                    observation_refs: vec!["observation:gwb:b".into()],
                    temporal_refs: vec!["temporal:gwb:exact".into()],
                    scope_refs: vec!["matter:gwb:capstone".into()],
                    review_state: GwbClaimReviewState::Qualified,
                    review_ref: Some("review:claim:gwb:b".into()),
                    event_ref: Some("event:gwb:1".into()),
                    event_assembly_receipt_ref: Some(
                        "review:event-assembly:gwb:1".into(),
                    ),
                },
            ],
            contestation_relations: vec![GwbContestationRelation {
                relation_ref: "relation:gwb:a:b".into(),
                from_claim_ref: "claim:gwb:a".into(),
                to_claim_ref: "claim:gwb:b".into(),
                kind: GwbContestationRelationKind::Qualifies,
                statement_keys: vec!["memoir:a".into(), "wiki:b".into()],
                observation_refs: vec![
                    "observation:gwb:a".into(),
                    "observation:gwb:b".into(),
                ],
                review_ref: Some("review:relation:gwb:a:b".into()),
            }],
            review_items: vec![GwbReviewItem {
                review_item_ref: "review-item:gwb:chronology:1".into(),
                semantic_ref: "event:gwb:1".into(),
                item_kind: GwbReviewItemKind::ChronologyAmbiguity,
                reason: "exact date and relative account coexist".into(),
                provenance_refs: vec!["relation:gwb:a:b".into()],
                source_statement_keys: vec!["memoir:a".into(), "wiki:b".into()],
                current_status: GwbReviewStatus::Pending,
                available_actions: vec![
                    GwbReviewAction::OpenSource,
                    GwbReviewAction::Qualify,
                    GwbReviewAction::RequestEvidence,
                ],
                affected_consumer_refs: vec!["timeline:matter:gwb:capstone".into()],
            }],
            candidate_only: true,
            semantic_promotion: false,
        }
    }

    #[test]
    fn reviewed_multi_source_fixture_validates_without_merging_accounts() {
        let manifest = fixture();
        manifest.validate().unwrap();
        assert_eq!(manifest.event_joins.len(), 1);
        assert_eq!(manifest.claim_leaves.len(), 2);
        assert_eq!(manifest.proposition_roots.len(), 1);
        assert_eq!(manifest.temporal_assertions.len(), 2);
    }

    #[test]
    fn automatic_same_event_join_fails_closed() {
        let mut manifest = fixture();
        manifest.event_joins[0].automatic_join = true;
        assert_eq!(
            manifest.validate(),
            Err(GwbChronologyCapstoneError::AutomaticEventJoin(
                "event:gwb:1".into()
            ))
        );
    }

    #[test]
    fn sentence_id_without_exact_source_span_is_not_enough() {
        let mut manifest = fixture();
        manifest.statements[0].literal_text.clear();
        assert_eq!(
            manifest.validate(),
            Err(GwbChronologyCapstoneError::EmptyCoordinate("literal_text"))
        );
    }

    #[test]
    fn admitted_statement_requires_explicit_admission_receipt() {
        let mut manifest = fixture();
        manifest.statements[0].admission_receipt_ref = None;
        assert_eq!(
            manifest.validate(),
            Err(GwbChronologyCapstoneError::MissingAdmissionReceipt(
                "memoir:a".into()
            ))
        );
    }

    #[test]
    fn temporal_assertions_can_disagree_in_precision_without_duplicate_event() {
        let manifest = fixture();
        let exact = &manifest.temporal_assertions[0];
        let relative = &manifest.temporal_assertions[1];
        assert_eq!(exact.event_ref, relative.event_ref);
        assert_ne!(exact.form, relative.form);
        assert_eq!(manifest.event_joins.len(), 1);
    }
}
