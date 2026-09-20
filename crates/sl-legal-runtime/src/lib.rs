//! Production legal-runtime capstone.
//!
//! This crate deliberately composes the already-paid SLR evidence substrate
//! with the legal semantics represented in DASHI.  It is not a second world
//! model, scheduler, evidence ontology, or UI ontology.
//!
//! Capability gates owned here:
//! - M2.5 mixed-family persisted evidence replay;
//! - M3.A reviewed world -> WrongType/element issue state;
//! - M3.B source-realised legal evaluator;
//! - M3.C adaptive Australian legal capstone;
//! - M4.A read-only matter + issue workspace projection.

use std::collections::{BTreeMap, BTreeSet};

use sensiblaw_consumer_residual::EvidenceCoordinateKind;
use sensiblaw_core::canonical_evidence::{
    EvidenceManifestation, EvidenceManifestationFamily, EvidenceObservation,
    EvidenceSourceRevision, EvidenceSpan,
};
use sensiblaw_reviewed_evidence_payment::{
    reduce_reviewed_canonical_evidence, CanonicalEvidenceProjection,
    ProjectionDisposition, ProjectionFamily, ReviewedCanonicalEvidence,
    ReviewedEvidenceCoordinate, SharedEvidenceReductionReceipt,
};
use sha2::{Digest, Sha256};

pub const LEGAL_RUNTIME_VERSION: &str = "sl-legal-runtime:v1";

fn digest(parts: impl IntoIterator<Item = impl AsRef<str>>) -> String {
    let mut hasher = Sha256::new();
    for part in parts {
        let bytes = part.as_ref().as_bytes();
        hasher.update((bytes.len() as u64).to_le_bytes());
        hasher.update(bytes);
    }
    format!("sha256:{:x}", hasher.finalize())
}

fn non_empty(label: &'static str, value: &str) -> Result<(), LegalRuntimeError> {
    if value.trim().is_empty() {
        Err(LegalRuntimeError::EmptyCoordinate(label))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegalRuntimeError {
    EmptyCoordinate(&'static str),
    InvalidEvidence(String),
    ReplayFormat(String),
    ReplayDigestMismatch,
    ReplayIdentityMismatch,
    MissingRule(String),
    MissingElement(String),
    RuleSourceUnrealised(String),
    RuleOutOfScope(String),
    Projection(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FixedProjection {
    family: ProjectionFamily,
    disposition: ProjectionDisposition,
}

impl CanonicalEvidenceProjection for FixedProjection {
    fn family(&self) -> ProjectionFamily {
        self.family
    }

    fn project(
        &self,
        _evidence: &ReviewedCanonicalEvidence,
    ) -> Result<ProjectionDisposition, String> {
        Ok(self.disposition.clone())
    }
}

// -------------------------------------------------------------------------
// M2.5 — mixed-family persisted replay.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReplayEvidenceFamily {
    StructuredWorld,
    LegalAuthority,
    MatterDocument,
}

impl ReplayEvidenceFamily {
    fn tag(self) -> &'static str {
        match self {
            Self::StructuredWorld => "structured-world",
            Self::LegalAuthority => "legal-authority",
            Self::MatterDocument => "matter-document",
        }
    }

    fn parse(value: &str) -> Result<Self, LegalRuntimeError> {
        match value {
            "structured-world" => Ok(Self::StructuredWorld),
            "legal-authority" => Ok(Self::LegalAuthority),
            "matter-document" => Ok(Self::MatterDocument),
            other => Err(LegalRuntimeError::ReplayFormat(format!(
                "unknown evidence family {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedCanonicalEvidenceEntry {
    pub family: ReplayEvidenceFamily,
    pub manifestation_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub observation_ref: String,
    pub review_ref: String,
    pub payment_ref: String,
    pub projection_summary: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
    pub entry_digest: String,
}

impl PersistedCanonicalEvidenceEntry {
    fn from_runtime(
        family: ReplayEvidenceFamily,
        manifestation: &EvidenceManifestation,
        reviewed: &ReviewedCanonicalEvidence,
        reduction: &SharedEvidenceReductionReceipt,
    ) -> Self {
        let mut projection_parts = reduction
            .projections
            .iter()
            .map(|projection| {
                let outcome = match &projection.disposition {
                    ProjectionDisposition::Produced { delta_ref } => {
                        format!("produced:{delta_ref}")
                    }
                    ProjectionDisposition::Abstained { reason_ref } => {
                        format!("abstained:{reason_ref}")
                    }
                };
                format!("{:?}:{outcome}", projection.family)
            })
            .collect::<Vec<_>>();
        projection_parts.sort();
        let projection_summary = projection_parts.join(",");

        let mut entry = Self {
            family,
            manifestation_ref: manifestation.manifestation_ref.clone(),
            source_revision_ref: reviewed.observation.source_revision_ref.clone(),
            span_ref: reviewed.observation.span.span_ref.clone(),
            observation_ref: reviewed.observation.observation_ref.clone(),
            review_ref: reviewed.review_ref.clone(),
            payment_ref: reviewed.payment_ref.clone(),
            projection_summary,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            entry_digest: String::new(),
        };
        entry.entry_digest = entry.compute_digest();
        entry
    }

    fn compute_digest(&self) -> String {
        digest([
            LEGAL_RUNTIME_VERSION,
            self.family.tag(),
            self.manifestation_ref.as_str(),
            self.source_revision_ref.as_str(),
            self.span_ref.as_str(),
            self.observation_ref.as_str(),
            self.review_ref.as_str(),
            self.payment_ref.as_str(),
            self.projection_summary.as_str(),
            if self.candidate_only { "1" } else { "0" },
            if self.creates_semantic_authority { "1" } else { "0" },
            if self.applicability_promoted { "1" } else { "0" },
            if self.claim_truth_promoted { "1" } else { "0" },
        ])
    }

    fn validate(&self) -> Result<(), LegalRuntimeError> {
        for (label, value) in [
            ("manifestation_ref", self.manifestation_ref.as_str()),
            ("source_revision_ref", self.source_revision_ref.as_str()),
            ("span_ref", self.span_ref.as_str()),
            ("observation_ref", self.observation_ref.as_str()),
            ("review_ref", self.review_ref.as_str()),
            ("payment_ref", self.payment_ref.as_str()),
        ] {
            non_empty(label, value)?;
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(LegalRuntimeError::InvalidEvidence(
                "persisted canonical evidence must remain candidate-only and non-promoting"
                    .into(),
            ));
        }
        if self.entry_digest != self.compute_digest() {
            return Err(LegalRuntimeError::ReplayDigestMismatch);
        }
        Ok(())
    }

    fn encode_line(&self) -> String {
        [
            self.family.tag().to_owned(),
            self.manifestation_ref.clone(),
            self.source_revision_ref.clone(),
            self.span_ref.clone(),
            self.observation_ref.clone(),
            self.review_ref.clone(),
            self.payment_ref.clone(),
            self.projection_summary.clone(),
            self.entry_digest.clone(),
        ]
        .join("\t")
    }

    fn decode_line(line: &str) -> Result<Self, LegalRuntimeError> {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 9 {
            return Err(LegalRuntimeError::ReplayFormat(format!(
                "expected 9 fields, got {}",
                fields.len()
            )));
        }
        let mut entry = Self {
            family: ReplayEvidenceFamily::parse(fields[0])?,
            manifestation_ref: fields[1].into(),
            source_revision_ref: fields[2].into(),
            span_ref: fields[3].into(),
            observation_ref: fields[4].into(),
            review_ref: fields[5].into(),
            payment_ref: fields[6].into(),
            projection_summary: fields[7].into(),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
            entry_digest: fields[8].into(),
        };
        entry.validate()?;
        // Reassert fixed non-promotion coordinates after decode.
        entry.candidate_only = true;
        Ok(entry)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MixedFamilyReplayReceipt {
    pub entries: Vec<PersistedCanonicalEvidenceEntry>,
    pub receipt_head: String,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub applicability_promoted: bool,
    pub claim_truth_promoted: bool,
}

impl MixedFamilyReplayReceipt {
    pub fn encode(&self) -> String {
        let mut lines = vec![format!("SLR-M2.5\t{}", self.receipt_head)];
        lines.extend(self.entries.iter().map(PersistedCanonicalEvidenceEntry::encode_line));
        lines.join("\n")
    }

    pub fn decode(payload: &str) -> Result<Self, LegalRuntimeError> {
        let mut lines = payload.lines();
        let header = lines
            .next()
            .ok_or_else(|| LegalRuntimeError::ReplayFormat("missing header".into()))?;
        let header_fields = header.split('\t').collect::<Vec<_>>();
        if header_fields.len() != 2 || header_fields[0] != "SLR-M2.5" {
            return Err(LegalRuntimeError::ReplayFormat("bad M2.5 header".into()));
        }
        let entries = lines
            .filter(|line| !line.trim().is_empty())
            .map(PersistedCanonicalEvidenceEntry::decode_line)
            .collect::<Result<Vec<_>, _>>()?;
        let receipt = Self::from_entries(entries);
        if receipt.receipt_head != header_fields[1] {
            return Err(LegalRuntimeError::ReplayDigestMismatch);
        }
        Ok(receipt)
    }

    fn from_entries(entries: Vec<PersistedCanonicalEvidenceEntry>) -> Self {
        let receipt_head = digest(
            std::iter::once(LEGAL_RUNTIME_VERSION)
                .chain(std::iter::once("M2.5"))
                .chain(entries.iter().map(|entry| entry.entry_digest.as_str())),
        );
        Self {
            entries,
            receipt_head,
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        }
    }

    pub fn validate_exact_replay(&self, reloaded: &Self) -> Result<(), LegalRuntimeError> {
        if self != reloaded {
            return Err(LegalRuntimeError::ReplayIdentityMismatch);
        }
        if !self.candidate_only
            || self.creates_semantic_authority
            || self.applicability_promoted
            || self.claim_truth_promoted
        {
            return Err(LegalRuntimeError::InvalidEvidence(
                "replay promoted semantic state".into(),
            ));
        }
        Ok(())
    }
}

fn manifestation(
    family: EvidenceManifestationFamily,
    source: &str,
    revision: &str,
    digest_ref: &str,
) -> EvidenceManifestation {
    EvidenceManifestation {
        manifestation_ref: format!("manifestation:{revision}"),
        family,
        source_ref: source.into(),
        source_revision_ref: revision.into(),
        content_digest_ref: digest_ref.into(),
        acquisition_receipt_ref: format!("acquisition:{revision}"),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    }
}

fn canonical_review(
    observation: EvidenceObservation,
    source_ref: &str,
    review_ref: &str,
    payment_ref: &str,
    coordinate: EvidenceCoordinateKind,
) -> Result<ReviewedCanonicalEvidence, LegalRuntimeError> {
    let review = ReviewedEvidenceCoordinate {
        review_ref: review_ref.into(),
        consumer_id: "consumer:sl-legal-runtime".into(),
        requirement_id: format!("requirement:{}", observation.observation_ref),
        coordinate,
        source_ref: Some(source_ref.into()),
        evidence_ref: observation.observation_ref.clone(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    ReviewedCanonicalEvidence::from_reviewed_coordinate(&review, observation, payment_ref)
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))
}

fn reduction(
    reviewed: &ReviewedCanonicalEvidence,
    families: &[ProjectionFamily],
) -> Result<SharedEvidenceReductionReceipt, LegalRuntimeError> {
    let mut world = None;
    let mut matter = None;
    let mut legal = None;
    let mut projectors = Vec::new();
    for family in families {
        projectors.push(FixedProjection {
            family: *family,
            disposition: ProjectionDisposition::Produced {
                delta_ref: format!(
                    "delta:{:?}:{}",
                    family, reviewed.observation.observation_ref
                ),
            },
        });
    }
    for projector in &projectors {
        match projector.family {
            ProjectionFamily::World => world = Some(projector as &dyn CanonicalEvidenceProjection),
            ProjectionFamily::Matter => {
                matter = Some(projector as &dyn CanonicalEvidenceProjection)
            }
            ProjectionFamily::Legal => legal = Some(projector as &dyn CanonicalEvidenceProjection),
        }
    }
    reduce_reviewed_canonical_evidence(reviewed, world, matter, legal)
        .map_err(|error| LegalRuntimeError::Projection(format!("{error:?}")))
}

/// Construct the Sprint-2 closure campaign over three materially different
/// source/anchor families and return the persisted replay receipt.
pub fn build_m2_5_mixed_family_campaign() -> Result<MixedFamilyReplayReceipt, LegalRuntimeError> {
    let wd = manifestation(
        EvidenceManifestationFamily::Wikidata,
        "wikidata:Q1501525",
        "wikidata:Q1501525:oldid:2333409615",
        "sha256:wikidata-fixture",
    );
    wd.validate()
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let wd_revision = EvidenceSourceRevision::from_manifestation(
        &wd,
        "revision-receipt:wikidata:Q1501525:2333409615",
    )
    .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let wd_observation = EvidenceObservation {
        observation_ref: "observation:m2.5:wikidata:p710".into(),
        source_revision_ref: wd_revision.source_revision_ref.clone(),
        span: EvidenceSpan::structured(
            wd_revision.source_revision_ref.clone(),
            "span:m2.5:wikidata:p710",
            "wikidata:Q1501525:P710:Q975866",
        )
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?,
        predicate_ref: "wikidata:P710".into(),
        value_ref: "wikidata:Q975866".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let wd_review = canonical_review(
        wd_observation,
        &wd.manifestation_ref,
        "review:m2.5:wikidata",
        "payment:m2.5:wikidata",
        EvidenceCoordinateKind::Classification,
    )?;
    let wd_reduction = reduction(
        &wd_review,
        &[ProjectionFamily::World, ProjectionFamily::Matter],
    )?;

    let authority = manifestation(
        EvidenceManifestationFamily::LegalAuthority,
        "authority:hca:fixture",
        "authority:hca:fixture:revision:1",
        "sha256:authority-fixture",
    );
    authority
        .validate()
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let authority_revision = EvidenceSourceRevision::from_manifestation(
        &authority,
        "revision-receipt:authority:hca:fixture",
    )
    .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let authority_observation = EvidenceObservation {
        observation_ref: "observation:m2.5:authority:paragraph".into(),
        source_revision_ref: authority_revision.source_revision_ref.clone(),
        span: EvidenceSpan::text(
            authority_revision.source_revision_ref.clone(),
            "span:m2.5:authority:paragraph",
            100,
            220,
        )
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?,
        predicate_ref: "legal:reported-proposition".into(),
        value_ref: "proposition:m2.5:authority".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let authority_review = canonical_review(
        authority_observation,
        &authority.manifestation_ref,
        "review:m2.5:authority",
        "payment:m2.5:authority",
        EvidenceCoordinateKind::Authority,
    )?;
    let authority_reduction = reduction(
        &authority_review,
        &[ProjectionFamily::Matter, ProjectionFamily::Legal],
    )?;

    let matter = manifestation(
        EvidenceManifestationFamily::Transcript,
        "matter:transcript:fixture",
        "matter:transcript:fixture:revision:1",
        "sha256:matter-fixture",
    );
    matter
        .validate()
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let matter_revision = EvidenceSourceRevision::from_manifestation(
        &matter,
        "revision-receipt:matter:transcript:fixture",
    )
    .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
    let matter_observation = EvidenceObservation {
        observation_ref: "observation:m2.5:matter:utterance".into(),
        source_revision_ref: matter_revision.source_revision_ref.clone(),
        span: EvidenceSpan::text(
            matter_revision.source_revision_ref.clone(),
            "span:m2.5:matter:utterance",
            20,
            94,
        )
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?,
        predicate_ref: "matter:asserted-event".into(),
        value_ref: "event-candidate:m2.5".into(),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    let matter_review = canonical_review(
        matter_observation,
        &matter.manifestation_ref,
        "review:m2.5:matter",
        "payment:m2.5:matter",
        EvidenceCoordinateKind::Mechanism,
    )?;
    let matter_reduction = reduction(
        &matter_review,
        &[ProjectionFamily::World, ProjectionFamily::Matter],
    )?;

    Ok(MixedFamilyReplayReceipt::from_entries(vec![
        PersistedCanonicalEvidenceEntry::from_runtime(
            ReplayEvidenceFamily::StructuredWorld,
            &wd,
            &wd_review,
            &wd_reduction,
        ),
        PersistedCanonicalEvidenceEntry::from_runtime(
            ReplayEvidenceFamily::LegalAuthority,
            &authority,
            &authority_review,
            &authority_reduction,
        ),
        PersistedCanonicalEvidenceEntry::from_runtime(
            ReplayEvidenceFamily::MatterDocument,
            &matter,
            &matter_review,
            &matter_reduction,
        ),
    ]))
}

// -------------------------------------------------------------------------
// M3.A — reviewed world -> WrongType / element issue state.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegalElementKind {
    Duty,
    Breach,
    Causation,
    Damage,
    MentalState,
    Statutory,
    Jurisdiction,
    Standing,
    Limitation,
    Defence,
    Remedy,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ElementDisposition {
    Satisfied,
    Unsatisfied,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceDisposition {
    Supports,
    Contradicts,
    Contests,
    DoesNotAddress,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongElementRequirement {
    pub element_ref: String,
    pub kind: LegalElementKind,
    pub proposition_ref: String,
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongTypeRuleBundle {
    pub wrong_type_ref: String,
    pub elements: Vec<WrongElementRequirement>,
    pub source_rule_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ElementEvidenceLink {
    pub element_ref: String,
    pub reviewed_evidence_ref: String,
    pub observation_ref: String,
    pub source_revision_ref: String,
    pub span_ref: String,
    pub disposition: EvidenceDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongElementEvaluation {
    pub element: WrongElementRequirement,
    pub disposition: ElementDisposition,
    pub evidence: Vec<ElementEvidenceLink>,
    pub candidate_only: bool,
    pub creates_liability: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrongTypeIssueState {
    pub wrong_type_ref: String,
    pub elements: Vec<WrongElementEvaluation>,
    pub candidate_only: bool,
    pub applicability_promoted: bool,
    pub violation_promoted: bool,
    pub liability_promoted: bool,
}

impl WrongTypeIssueState {
    pub fn unresolved_element_refs(&self) -> Vec<String> {
        self.elements
            .iter()
            .filter(|element| {
                matches!(
                    element.disposition,
                    ElementDisposition::Contested | ElementDisposition::Unresolved
                )
            })
            .map(|element| element.element.element_ref.clone())
            .collect()
    }
}

fn disposition_from_links(links: &[ElementEvidenceLink]) -> ElementDisposition {
    let support = links
        .iter()
        .any(|link| link.disposition == EvidenceDisposition::Supports);
    let contradict = links
        .iter()
        .any(|link| link.disposition == EvidenceDisposition::Contradicts);
    let contested = links
        .iter()
        .any(|link| link.disposition == EvidenceDisposition::Contests);
    match (support, contradict, contested) {
        (true, false, false) => ElementDisposition::Satisfied,
        (false, true, false) => ElementDisposition::Unsatisfied,
        (true, true, _) | (_, _, true) => ElementDisposition::Contested,
        _ => ElementDisposition::Unresolved,
    }
}

pub fn project_reviewed_world_to_wrong_type(
    bundle: &WrongTypeRuleBundle,
    evidence: &[(&ReviewedCanonicalEvidence, &str, EvidenceDisposition)],
) -> Result<WrongTypeIssueState, LegalRuntimeError> {
    non_empty("wrong_type_ref", &bundle.wrong_type_ref)?;
    let known_elements = bundle
        .elements
        .iter()
        .map(|element| element.element_ref.as_str())
        .collect::<BTreeSet<_>>();

    let mut links_by_element: BTreeMap<String, Vec<ElementEvidenceLink>> = BTreeMap::new();
    for (reviewed, element_ref, disposition) in evidence {
        reviewed
            .validate()
            .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?;
        if !known_elements.contains(element_ref) {
            return Err(LegalRuntimeError::MissingElement((*element_ref).into()));
        }
        links_by_element
            .entry((*element_ref).into())
            .or_default()
            .push(ElementEvidenceLink {
                element_ref: (*element_ref).into(),
                reviewed_evidence_ref: reviewed.reviewed_evidence_ref.clone(),
                observation_ref: reviewed.observation.observation_ref.clone(),
                source_revision_ref: reviewed.observation.source_revision_ref.clone(),
                span_ref: reviewed.observation.span.span_ref.clone(),
                disposition: *disposition,
            });
    }

    let elements = bundle
        .elements
        .iter()
        .cloned()
        .map(|element| {
            let links = links_by_element
                .remove(&element.element_ref)
                .unwrap_or_default();
            WrongElementEvaluation {
                disposition: disposition_from_links(&links),
                evidence: links,
                element,
                candidate_only: true,
                creates_liability: false,
            }
        })
        .collect();

    Ok(WrongTypeIssueState {
        wrong_type_ref: bundle.wrong_type_ref.clone(),
        elements,
        candidate_only: true,
        applicability_promoted: false,
        violation_promoted: false,
        liability_promoted: false,
    })
}

// -------------------------------------------------------------------------
// M3.B — source-realised legal evaluator.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthorityRole {
    Binding,
    Persuasive,
    Historical,
    Background,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PropositionStatus {
    Established,
    Failed,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropositionState {
    pub proposition_ref: String,
    pub status: PropositionStatus,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRealisedLegalRule {
    pub rule_ref: String,
    pub source_revision_ref: String,
    pub source_span_refs: Vec<String>,
    pub conclusion_ref: String,
    pub premise_refs: Vec<String>,
    pub exception_refs: Vec<String>,
    pub defeater_refs: Vec<String>,
    pub burden_refs: Vec<String>,
    pub jurisdiction_ref: String,
    pub valid_from: String,
    pub valid_to: Option<String>,
    pub authority_role: AuthorityRole,
    pub source_realised: bool,
    pub candidate_only: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ApplicabilityStatus {
    Applicable,
    NotApplicable,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ViolationStatus {
    Established,
    NotEstablished,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LiabilityStatus {
    Established,
    NotEstablished,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemedyStatus {
    Eligible,
    NotEligible,
    Contested,
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalEvaluationContext {
    pub jurisdiction_ref: String,
    pub as_at: String,
    pub propositions: BTreeMap<String, PropositionState>,
    pub wrong_type: WrongTypeIssueState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRealisedLegalEvaluation {
    pub rule_ref: String,
    pub applicability: ApplicabilityStatus,
    pub violation: ViolationStatus,
    pub liability: LiabilityStatus,
    pub remedy: RemedyStatus,
    pub live_exception_refs: Vec<String>,
    pub live_defeater_refs: Vec<String>,
    pub unresolved_refs: Vec<String>,
    pub source_revision_ref: String,
    pub source_span_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

fn proposition_status(context: &LegalEvaluationContext, reference: &str) -> PropositionStatus {
    context
        .propositions
        .get(reference)
        .map(|state| state.status)
        .unwrap_or(PropositionStatus::Unresolved)
}

fn in_temporal_scope(rule: &SourceRealisedLegalRule, as_at: &str) -> bool {
    if as_at < rule.valid_from.as_str() {
        return false;
    }
    match &rule.valid_to {
        Some(end) => as_at <= end.as_str(),
        None => true,
    }
}

pub fn evaluate_source_realised_rule(
    rule: &SourceRealisedLegalRule,
    context: &LegalEvaluationContext,
) -> Result<SourceRealisedLegalEvaluation, LegalRuntimeError> {
    if !rule.source_realised {
        return Err(LegalRuntimeError::RuleSourceUnrealised(rule.rule_ref.clone()));
    }
    for (label, value) in [
        ("rule_ref", rule.rule_ref.as_str()),
        ("source_revision_ref", rule.source_revision_ref.as_str()),
        ("conclusion_ref", rule.conclusion_ref.as_str()),
        ("jurisdiction_ref", rule.jurisdiction_ref.as_str()),
        ("valid_from", rule.valid_from.as_str()),
    ] {
        non_empty(label, value)?;
    }
    if rule.jurisdiction_ref != context.jurisdiction_ref || !in_temporal_scope(rule, &context.as_at) {
        return Ok(SourceRealisedLegalEvaluation {
            rule_ref: rule.rule_ref.clone(),
            applicability: ApplicabilityStatus::NotApplicable,
            violation: ViolationStatus::NotEstablished,
            liability: LiabilityStatus::NotEstablished,
            remedy: RemedyStatus::NotEligible,
            live_exception_refs: Vec::new(),
            live_defeater_refs: Vec::new(),
            unresolved_refs: Vec::new(),
            source_revision_ref: rule.source_revision_ref.clone(),
            source_span_refs: rule.source_span_refs.clone(),
            candidate_only: true,
            creates_claim_truth: false,
        });
    }

    let mut unresolved = Vec::new();
    let mut premise_failed = false;
    let mut premise_contested = false;
    for premise in &rule.premise_refs {
        match proposition_status(context, premise) {
            PropositionStatus::Established => {}
            PropositionStatus::Failed => premise_failed = true,
            PropositionStatus::Contested => premise_contested = true,
            PropositionStatus::Unresolved => unresolved.push(premise.clone()),
        }
    }

    let live_exceptions = rule
        .exception_refs
        .iter()
        .filter(|reference| proposition_status(context, reference) == PropositionStatus::Established)
        .cloned()
        .collect::<Vec<_>>();
    let live_defeaters = rule
        .defeater_refs
        .iter()
        .filter(|reference| proposition_status(context, reference) == PropositionStatus::Established)
        .cloned()
        .collect::<Vec<_>>();

    let exception_contested = rule.exception_refs.iter().any(|reference| {
        proposition_status(context, reference) == PropositionStatus::Contested
    });
    let defeater_contested = rule.defeater_refs.iter().any(|reference| {
        proposition_status(context, reference) == PropositionStatus::Contested
    });
    for reference in rule.exception_refs.iter().chain(rule.defeater_refs.iter()) {
        if proposition_status(context, reference) == PropositionStatus::Unresolved {
            unresolved.push(reference.clone());
        }
    }
    for burden in &rule.burden_refs {
        if proposition_status(context, burden) == PropositionStatus::Unresolved {
            unresolved.push(burden.clone());
        }
    }

    let applicability = if premise_failed || !live_exceptions.is_empty() || !live_defeaters.is_empty() {
        ApplicabilityStatus::NotApplicable
    } else if premise_contested || exception_contested || defeater_contested {
        ApplicabilityStatus::Contested
    } else if !unresolved.is_empty() {
        ApplicabilityStatus::Unresolved
    } else {
        ApplicabilityStatus::Applicable
    };

    let required_elements = context
        .wrong_type
        .elements
        .iter()
        .filter(|element| element.element.required)
        .collect::<Vec<_>>();
    let element_contested = required_elements.iter().any(|element| {
        element.disposition == ElementDisposition::Contested
    });
    let element_unresolved = required_elements.iter().any(|element| {
        element.disposition == ElementDisposition::Unresolved
    });
    let element_unsatisfied = required_elements.iter().any(|element| {
        element.disposition == ElementDisposition::Unsatisfied
    });
    let all_required_satisfied = !required_elements.is_empty()
        && required_elements
            .iter()
            .all(|element| element.disposition == ElementDisposition::Satisfied);

    let violation = match applicability {
        ApplicabilityStatus::Applicable if all_required_satisfied => ViolationStatus::Established,
        ApplicabilityStatus::Applicable if element_unsatisfied => ViolationStatus::NotEstablished,
        ApplicabilityStatus::Applicable if element_contested => ViolationStatus::Contested,
        ApplicabilityStatus::Applicable if element_unresolved => ViolationStatus::Unresolved,
        ApplicabilityStatus::Contested => ViolationStatus::Contested,
        ApplicabilityStatus::Unresolved => ViolationStatus::Unresolved,
        ApplicabilityStatus::NotApplicable => ViolationStatus::NotEstablished,
        ApplicabilityStatus::Applicable => ViolationStatus::Unresolved,
    };

    // Liability remains separately burdened: violation alone never manufactures it.
    let burden_failed = rule
        .burden_refs
        .iter()
        .any(|burden| proposition_status(context, burden) == PropositionStatus::Failed);
    let burden_contested = rule
        .burden_refs
        .iter()
        .any(|burden| proposition_status(context, burden) == PropositionStatus::Contested);
    let burden_unresolved = rule
        .burden_refs
        .iter()
        .any(|burden| proposition_status(context, burden) == PropositionStatus::Unresolved);
    let burdens_paid = rule
        .burden_refs
        .iter()
        .all(|burden| proposition_status(context, burden) == PropositionStatus::Established);

    let liability = match violation {
        ViolationStatus::Established if burdens_paid => LiabilityStatus::Established,
        ViolationStatus::Established if burden_failed => LiabilityStatus::NotEstablished,
        ViolationStatus::Established if burden_contested => LiabilityStatus::Contested,
        ViolationStatus::Established if burden_unresolved => LiabilityStatus::Unresolved,
        ViolationStatus::Contested => LiabilityStatus::Contested,
        ViolationStatus::NotEstablished => LiabilityStatus::NotEstablished,
        ViolationStatus::Unresolved => LiabilityStatus::Unresolved,
        ViolationStatus::Established => LiabilityStatus::Unresolved,
    };

    let remedy_element = context
        .wrong_type
        .elements
        .iter()
        .find(|element| element.element.kind == LegalElementKind::Remedy);
    let remedy = match (liability, remedy_element.map(|element| element.disposition)) {
        (LiabilityStatus::Established, Some(ElementDisposition::Satisfied)) => {
            RemedyStatus::Eligible
        }
        (LiabilityStatus::Established, Some(ElementDisposition::Unsatisfied)) => {
            RemedyStatus::NotEligible
        }
        (LiabilityStatus::Established, Some(ElementDisposition::Contested)) => {
            RemedyStatus::Contested
        }
        (LiabilityStatus::Established, _) => RemedyStatus::Unresolved,
        (LiabilityStatus::Contested, _) => RemedyStatus::Contested,
        (LiabilityStatus::NotEstablished, _) => RemedyStatus::NotEligible,
        (LiabilityStatus::Unresolved, _) => RemedyStatus::Unresolved,
    };

    Ok(SourceRealisedLegalEvaluation {
        rule_ref: rule.rule_ref.clone(),
        applicability,
        violation,
        liability,
        remedy,
        live_exception_refs: live_exceptions,
        live_defeater_refs: live_defeaters,
        unresolved_refs: unresolved,
        source_revision_ref: rule.source_revision_ref.clone(),
        source_span_refs: rule.source_span_refs.clone(),
        candidate_only: true,
        creates_claim_truth: false,
    })
}

// -------------------------------------------------------------------------
// M3.C — adaptive persisted Australian legal capstone.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AustralianCalibrationKind {
    Mabo,
    Pabai,
    CullenNswCla,
    Glj,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LegalResidualKind {
    Source,
    MatterEvidence,
    Element,
    ExceptionOrDefeater,
    Burden,
    JurisdictionOrTime,
    Remedy,
    ClosedForConsumer,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalResidual {
    pub residual_ref: String,
    pub kind: LegalResidualKind,
    pub target_ref: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InformationActionKind {
    Look,
    Think,
    Review,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InformationAction {
    pub action_ref: String,
    pub kind: InformationActionKind,
    pub residual_ref: String,
    pub target_ref: String,
    pub candidate_only: bool,
    pub creates_authority: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalCampaignState {
    pub campaign_ref: String,
    pub calibration: AustralianCalibrationKind,
    pub iteration: u64,
    pub evaluation: SourceRealisedLegalEvaluation,
    pub residuals: Vec<LegalResidual>,
    pub selected_action: Option<InformationAction>,
    pub source_refs: Vec<String>,
    pub previous_receipt_head: Option<String>,
    pub receipt_head: String,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

fn residuals_for_evaluation(
    evaluation: &SourceRealisedLegalEvaluation,
    wrong_type: &WrongTypeIssueState,
) -> Vec<LegalResidual> {
    let mut residuals = Vec::new();
    if evaluation.source_revision_ref.is_empty() || evaluation.source_span_refs.is_empty() {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:source", evaluation.rule_ref),
            kind: LegalResidualKind::Source,
            target_ref: evaluation.rule_ref.clone(),
            source_refs: Vec::new(),
        });
    }
    for element in &wrong_type.elements {
        if matches!(
            element.disposition,
            ElementDisposition::Contested | ElementDisposition::Unresolved
        ) {
            residuals.push(LegalResidual {
                residual_ref: format!("residual:{}:{}", evaluation.rule_ref, element.element.element_ref),
                kind: LegalResidualKind::Element,
                target_ref: element.element.element_ref.clone(),
                source_refs: element
                    .evidence
                    .iter()
                    .map(|evidence| evidence.source_revision_ref.clone())
                    .collect(),
            });
        }
    }
    for reference in &evaluation.unresolved_refs {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:{reference}", evaluation.rule_ref),
            kind: LegalResidualKind::ExceptionOrDefeater,
            target_ref: reference.clone(),
            source_refs: evaluation.source_span_refs.clone(),
        });
    }
    if evaluation.applicability == ApplicabilityStatus::Unresolved {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:applicability", evaluation.rule_ref),
            kind: LegalResidualKind::JurisdictionOrTime,
            target_ref: evaluation.rule_ref.clone(),
            source_refs: evaluation.source_span_refs.clone(),
        });
    }
    if evaluation.liability == LiabilityStatus::Unresolved
        && !evaluation.unresolved_refs.is_empty()
    {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:burden", evaluation.rule_ref),
            kind: LegalResidualKind::Burden,
            target_ref: evaluation.rule_ref.clone(),
            source_refs: evaluation.source_span_refs.clone(),
        });
    }
    if evaluation.liability == LiabilityStatus::Established
        && evaluation.remedy == RemedyStatus::Unresolved
    {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:remedy", evaluation.rule_ref),
            kind: LegalResidualKind::Remedy,
            target_ref: evaluation.rule_ref.clone(),
            source_refs: evaluation.source_span_refs.clone(),
        });
    }
    if residuals.is_empty() {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:closed", evaluation.rule_ref),
            kind: LegalResidualKind::ClosedForConsumer,
            target_ref: evaluation.rule_ref.clone(),
            source_refs: evaluation.source_span_refs.clone(),
        });
    }
    residuals
}

fn action_for_residual(residual: &LegalResidual) -> Option<InformationAction> {
    let kind = match residual.kind {
        LegalResidualKind::Source | LegalResidualKind::MatterEvidence => InformationActionKind::Look,
        LegalResidualKind::Element
        | LegalResidualKind::ExceptionOrDefeater
        | LegalResidualKind::Burden
        | LegalResidualKind::JurisdictionOrTime
        | LegalResidualKind::Remedy => InformationActionKind::Review,
        LegalResidualKind::ClosedForConsumer => return None,
    };
    Some(InformationAction {
        action_ref: format!("action:{:?}:{}", kind, residual.residual_ref),
        kind,
        residual_ref: residual.residual_ref.clone(),
        target_ref: residual.target_ref.clone(),
        candidate_only: true,
        creates_authority: false,
    })
}

fn state_receipt_head(
    campaign_ref: &str,
    iteration: u64,
    evaluation: &SourceRealisedLegalEvaluation,
    residuals: &[LegalResidual],
    selected: Option<&InformationAction>,
    previous: Option<&str>,
) -> String {
    let mut parts = vec![
        LEGAL_RUNTIME_VERSION.to_owned(),
        campaign_ref.to_owned(),
        iteration.to_string(),
        evaluation.rule_ref.clone(),
        format!("{:?}", evaluation.applicability),
        format!("{:?}", evaluation.violation),
        format!("{:?}", evaluation.liability),
        format!("{:?}", evaluation.remedy),
        previous.unwrap_or("").to_owned(),
    ];
    parts.extend(residuals.iter().map(|residual| residual.residual_ref.clone()));
    if let Some(selected) = selected {
        parts.push(selected.action_ref.clone());
    }
    digest(parts)
}

pub fn compile_legal_campaign_state(
    campaign_ref: impl Into<String>,
    calibration: AustralianCalibrationKind,
    iteration: u64,
    rule: &SourceRealisedLegalRule,
    context: &LegalEvaluationContext,
    previous_receipt_head: Option<String>,
) -> Result<LegalCampaignState, LegalRuntimeError> {
    let campaign_ref = campaign_ref.into();
    let evaluation = evaluate_source_realised_rule(rule, context)?;
    let residuals = residuals_for_evaluation(&evaluation, &context.wrong_type);
    let selected_action = residuals
        .iter()
        .find(|residual| residual.kind != LegalResidualKind::ClosedForConsumer)
        .and_then(action_for_residual);
    let receipt_head = state_receipt_head(
        &campaign_ref,
        iteration,
        &evaluation,
        &residuals,
        selected_action.as_ref(),
        previous_receipt_head.as_deref(),
    );

    Ok(LegalCampaignState {
        campaign_ref,
        calibration,
        iteration,
        evaluation,
        residuals,
        selected_action,
        source_refs: rule.source_span_refs.clone(),
        previous_receipt_head,
        receipt_head,
        candidate_only: true,
        creates_claim_truth: false,
    })
}

impl LegalCampaignState {
    pub fn encode(&self) -> String {
        let residual_refs = self
            .residuals
            .iter()
            .map(|residual| residual.residual_ref.as_str())
            .collect::<Vec<_>>()
            .join("|");
        let selected = self
            .selected_action
            .as_ref()
            .map(|action| action.action_ref.as_str())
            .unwrap_or("");
        [
            "SLR-M3.C",
            &self.campaign_ref,
            &self.iteration.to_string(),
            &format!("{:?}", self.calibration),
            &self.evaluation.rule_ref,
            &format!("{:?}", self.evaluation.applicability),
            &format!("{:?}", self.evaluation.violation),
            &format!("{:?}", self.evaluation.liability),
            &format!("{:?}", self.evaluation.remedy),
            &residual_refs,
            selected,
            self.previous_receipt_head.as_deref().unwrap_or(""),
            &self.receipt_head,
        ]
        .join("\t")
    }

    pub fn replay_identity(&self) -> String {
        let iteration = self.iteration.to_string();
        digest([
            self.campaign_ref.as_str(),
            iteration.as_str(),
            self.evaluation.rule_ref.as_str(),
            self.receipt_head.as_str(),
        ])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedLegalCampaign {
    pub campaign_ref: String,
    pub hops: Vec<LegalCampaignState>,
    pub receipt_head: String,
    pub candidate_only: bool,
    pub creates_claim_truth: bool,
}

impl PersistedLegalCampaign {
    pub fn new(campaign_ref: impl Into<String>) -> Self {
        let campaign_ref = campaign_ref.into();
        let receipt_head = digest([
            LEGAL_RUNTIME_VERSION,
            "M3.C-campaign",
            campaign_ref.as_str(),
        ]);
        Self {
            campaign_ref,
            hops: Vec::new(),
            receipt_head,
            candidate_only: true,
            creates_claim_truth: false,
        }
    }

    pub fn append(&mut self, state: LegalCampaignState) -> Result<(), LegalRuntimeError> {
        if state.campaign_ref != self.campaign_ref {
            return Err(LegalRuntimeError::ReplayIdentityMismatch);
        }
        if state.iteration != self.hops.len() as u64 {
            return Err(LegalRuntimeError::ReplayFormat(
                "campaign hop indexes must be consecutive".into(),
            ));
        }
        let expected_previous = self.hops.last().map(|hop| hop.receipt_head.as_str());
        if state.previous_receipt_head.as_deref() != expected_previous {
            return Err(LegalRuntimeError::ReplayIdentityMismatch);
        }
        if !state.candidate_only || state.creates_claim_truth {
            return Err(LegalRuntimeError::InvalidEvidence(
                "legal campaign hop promoted claim truth".into(),
            ));
        }
        self.receipt_head = state.receipt_head.clone();
        self.hops.push(state);
        Ok(())
    }

    pub fn encode(&self) -> String {
        let mut lines = vec![format!(
            "SLR-M3.C-LEDGER\t{}\t{}",
            self.campaign_ref, self.receipt_head
        )];
        lines.extend(self.hops.iter().map(LegalCampaignState::encode));
        lines.join("\n")
    }

    pub fn replay_summary(payload: &str) -> Result<(String, String, Vec<String>), LegalRuntimeError> {
        let mut lines = payload.lines();
        let header = lines
            .next()
            .ok_or_else(|| LegalRuntimeError::ReplayFormat("missing campaign header".into()))?;
        let header_fields = header.split('\t').collect::<Vec<_>>();
        if header_fields.len() != 3 || header_fields[0] != "SLR-M3.C-LEDGER" {
            return Err(LegalRuntimeError::ReplayFormat("bad campaign header".into()));
        }
        let hop_lines = lines
            .filter(|line| !line.trim().is_empty())
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        if let Some(last) = hop_lines.last() {
            let fields = last.split('\t').collect::<Vec<_>>();
            if fields.len() != 13 || fields[0] != "SLR-M3.C" {
                return Err(LegalRuntimeError::ReplayFormat(
                    "bad persisted legal hop".into(),
                ));
            }
            if fields[12] != header_fields[2] {
                return Err(LegalRuntimeError::ReplayDigestMismatch);
            }
        }
        Ok((
            header_fields[1].to_owned(),
            header_fields[2].to_owned(),
            hop_lines,
        ))
    }

    pub fn validate_restart_replay(&self) -> Result<(), LegalRuntimeError> {
        let encoded = self.encode();
        let (campaign_ref, receipt_head, hops) = Self::replay_summary(&encoded)?;
        if campaign_ref != self.campaign_ref
            || receipt_head != self.receipt_head
            || hops.len() != self.hops.len()
        {
            return Err(LegalRuntimeError::ReplayIdentityMismatch);
        }
        for (persisted, live) in hops.iter().zip(&self.hops) {
            if persisted != &live.encode() {
                return Err(LegalRuntimeError::ReplayIdentityMismatch);
            }
        }
        Ok(())
    }
}

/// The calibration names are explicit so one generic runner can be demonstrated
/// against distinct legal shapes without turning any fixture into an authority.
pub fn calibration_refs(kind: AustralianCalibrationKind) -> &'static [&'static str] {
    match kind {
        AustralianCalibrationKind::Mabo => &[
            "calibration:mabo:positive-doctrinal-route",
            "calibration:mabo:source-lineage",
        ],
        AustralianCalibrationKind::Pabai => &[
            "calibration:pabai:defeater",
            "calibration:pabai:reformulation-candidate",
        ],
        AustralianCalibrationKind::CullenNswCla => &[
            "calibration:cullen:duty",
            "calibration:cullen:nsw-cla-s5b",
            "calibration:cullen:distinction",
        ],
        AustralianCalibrationKind::Glj => &[
            "calibration:glj:procedural-posture",
            "calibration:glj:majority-dissent",
            "calibration:glj:source-correction",
        ],
    }
}

// -------------------------------------------------------------------------
// M4.A — read-only matter + issue workspace.
// -------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceAddressableNode {
    pub node_ref: String,
    pub label: String,
    pub semantic_kind: String,
    pub source_revision_refs: Vec<String>,
    pub span_refs: Vec<String>,
    pub dependency_refs: Vec<String>,
    pub downstream_refs: Vec<String>,
    pub residual_refs: Vec<String>,
    pub candidate_only: bool,
    pub projection_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatterWorkspaceProjection {
    pub matter_ref: String,
    pub nodes: Vec<SourceAddressableNode>,
    pub issue_ref: String,
    pub wrong_type_ref: String,
    pub applicability: ApplicabilityStatus,
    pub violation: ViolationStatus,
    pub liability: LiabilityStatus,
    pub remedy: RemedyStatus,
    pub next_action: Option<InformationAction>,
    pub projection_only: bool,
    pub creates_semantic_authority: bool,
}

pub fn project_matter_issue_workspace(
    matter_ref: impl Into<String>,
    issue: &WrongTypeIssueState,
    campaign: &LegalCampaignState,
) -> MatterWorkspaceProjection {
    let issue_ref = format!("issue:{}", issue.wrong_type_ref);
    let mut nodes = Vec::new();

    nodes.push(SourceAddressableNode {
        node_ref: issue_ref.clone(),
        label: issue.wrong_type_ref.clone(),
        semantic_kind: "legal-issue".into(),
        source_revision_refs: vec![campaign.evaluation.source_revision_ref.clone()],
        span_refs: campaign.evaluation.source_span_refs.clone(),
        dependency_refs: issue
            .elements
            .iter()
            .map(|element| element.element.element_ref.clone())
            .collect(),
        downstream_refs: vec![campaign.evaluation.rule_ref.clone()],
        residual_refs: campaign
            .residuals
            .iter()
            .map(|residual| residual.residual_ref.clone())
            .collect(),
        candidate_only: true,
        projection_only: true,
    });

    for element in &issue.elements {
        nodes.push(SourceAddressableNode {
            node_ref: element.element.element_ref.clone(),
            label: element.element.proposition_ref.clone(),
            semantic_kind: format!("legal-element:{:?}", element.element.kind),
            source_revision_refs: element
                .evidence
                .iter()
                .map(|evidence| evidence.source_revision_ref.clone())
                .collect(),
            span_refs: element
                .evidence
                .iter()
                .map(|evidence| evidence.span_ref.clone())
                .collect(),
            dependency_refs: element
                .evidence
                .iter()
                .map(|evidence| evidence.reviewed_evidence_ref.clone())
                .collect(),
            downstream_refs: vec![issue_ref.clone()],
            residual_refs: campaign
                .residuals
                .iter()
                .filter(|residual| residual.target_ref == element.element.element_ref)
                .map(|residual| residual.residual_ref.clone())
                .collect(),
            candidate_only: true,
            projection_only: true,
        });
    }

    MatterWorkspaceProjection {
        matter_ref: matter_ref.into(),
        nodes,
        issue_ref,
        wrong_type_ref: issue.wrong_type_ref.clone(),
        applicability: campaign.evaluation.applicability,
        violation: campaign.evaluation.violation,
        liability: campaign.evaluation.liability,
        remedy: campaign.evaluation.remedy,
        next_action: campaign.selected_action.clone(),
        projection_only: true,
        creates_semantic_authority: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reviewed_text(
        observation_ref: &str,
        source_revision_ref: &str,
        span_ref: &str,
    ) -> ReviewedCanonicalEvidence {
        let observation = EvidenceObservation {
            observation_ref: observation_ref.into(),
            source_revision_ref: source_revision_ref.into(),
            span: EvidenceSpan::text(source_revision_ref, span_ref, 0, 10).unwrap(),
            predicate_ref: format!("predicate:{observation_ref}"),
            value_ref: format!("value:{observation_ref}"),
            candidate_only: true,
            creates_semantic_authority: false,
            applicability_promoted: false,
            claim_truth_promoted: false,
        };
        canonical_review(
            observation,
            &format!("manifestation:{source_revision_ref}"),
            &format!("review:{observation_ref}"),
            &format!("payment:{observation_ref}"),
            EvidenceCoordinateKind::Mechanism,
        )
        .unwrap()
    }

    fn negligence_bundle() -> WrongTypeRuleBundle {
        WrongTypeRuleBundle {
            wrong_type_ref: "wrong:negligence:fixture".into(),
            elements: vec![
                WrongElementRequirement {
                    element_ref: "element:duty".into(),
                    kind: LegalElementKind::Duty,
                    proposition_ref: "prop:duty".into(),
                    required: true,
                },
                WrongElementRequirement {
                    element_ref: "element:breach".into(),
                    kind: LegalElementKind::Breach,
                    proposition_ref: "prop:breach".into(),
                    required: true,
                },
                WrongElementRequirement {
                    element_ref: "element:causation".into(),
                    kind: LegalElementKind::Causation,
                    proposition_ref: "prop:causation".into(),
                    required: true,
                },
                WrongElementRequirement {
                    element_ref: "element:damage".into(),
                    kind: LegalElementKind::Damage,
                    proposition_ref: "prop:damage".into(),
                    required: true,
                },
                WrongElementRequirement {
                    element_ref: "element:remedy".into(),
                    kind: LegalElementKind::Remedy,
                    proposition_ref: "prop:remedy-eligibility".into(),
                    required: false,
                },
            ],
            source_rule_refs: vec!["rule:fixture".into()],
        }
    }

    fn rule() -> SourceRealisedLegalRule {
        SourceRealisedLegalRule {
            rule_ref: "rule:fixture".into(),
            source_revision_ref: "authority:fixture:rev1".into(),
            source_span_refs: vec!["authority:fixture:[1]-[10]".into()],
            conclusion_ref: "prop:liability-candidate".into(),
            premise_refs: vec!["prop:rule-enabled".into()],
            exception_refs: vec!["prop:exception".into()],
            defeater_refs: vec!["prop:defeater".into()],
            burden_refs: vec!["prop:burden-paid".into()],
            jurisdiction_ref: "AU-NSW".into(),
            valid_from: "2020-01-01".into(),
            valid_to: None,
            authority_role: AuthorityRole::Binding,
            source_realised: true,
            candidate_only: true,
        }
    }

    #[test]
    fn m2_5_roundtrip_replays_exact_three_family_identity() {
        let original = build_m2_5_mixed_family_campaign().unwrap();
        assert_eq!(original.entries.len(), 3);
        assert_eq!(
            original
                .entries
                .iter()
                .map(|entry| entry.family)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from([
                ReplayEvidenceFamily::StructuredWorld,
                ReplayEvidenceFamily::LegalAuthority,
                ReplayEvidenceFamily::MatterDocument,
            ])
        );
        let persisted = original.encode();
        let reloaded = MixedFamilyReplayReceipt::decode(&persisted).unwrap();
        original.validate_exact_replay(&reloaded).unwrap();
        assert!(!reloaded.creates_semantic_authority);
        assert!(!reloaded.applicability_promoted);
        assert!(!reloaded.claim_truth_promoted);
    }

    #[test]
    fn m3_a_reviewed_evidence_projects_to_four_way_element_disposition() {
        let duty = reviewed_text("obs:duty", "matter:rev1", "span:duty");
        let breach_support = reviewed_text("obs:breach:support", "matter:rev1", "span:breach:1");
        let breach_against = reviewed_text("obs:breach:against", "matter:rev2", "span:breach:2");
        let damage = reviewed_text("obs:damage", "matter:rev3", "span:damage");

        let issue = project_reviewed_world_to_wrong_type(
            &negligence_bundle(),
            &[
                (&duty, "element:duty", EvidenceDisposition::Supports),
                (&breach_support, "element:breach", EvidenceDisposition::Supports),
                (&breach_against, "element:breach", EvidenceDisposition::Contradicts),
                (&damage, "element:damage", EvidenceDisposition::Contradicts),
            ],
        )
        .unwrap();

        let map = issue
            .elements
            .iter()
            .map(|element| (element.element.element_ref.as_str(), element.disposition))
            .collect::<BTreeMap<_, _>>();
        assert_eq!(map["element:duty"], ElementDisposition::Satisfied);
        assert_eq!(map["element:breach"], ElementDisposition::Contested);
        assert_eq!(map["element:causation"], ElementDisposition::Unresolved);
        assert_eq!(map["element:damage"], ElementDisposition::Unsatisfied);
        assert!(!issue.liability_promoted);
    }

    #[test]
    fn m3_b_live_exception_reopens_previously_applicable_rule() {
        let evidence = reviewed_text("obs:generic", "matter:rev1", "span:generic");
        let issue = project_reviewed_world_to_wrong_type(
            &negligence_bundle(),
            &[
                (&evidence, "element:duty", EvidenceDisposition::Supports),
                (&evidence, "element:breach", EvidenceDisposition::Supports),
                (&evidence, "element:causation", EvidenceDisposition::Supports),
                (&evidence, "element:damage", EvidenceDisposition::Supports),
                (&evidence, "element:remedy", EvidenceDisposition::Supports),
            ],
        )
        .unwrap();

        let mut propositions = BTreeMap::new();
        for reference in ["prop:rule-enabled", "prop:burden-paid"] {
            propositions.insert(
                reference.into(),
                PropositionState {
                    proposition_ref: reference.into(),
                    status: PropositionStatus::Established,
                    source_refs: vec!["source:reviewed".into()],
                },
            );
        }
        propositions.insert(
            "prop:exception".into(),
            PropositionState {
                proposition_ref: "prop:exception".into(),
                status: PropositionStatus::Failed,
                source_refs: vec!["source:reviewed".into()],
            },
        );
        propositions.insert(
            "prop:defeater".into(),
            PropositionState {
                proposition_ref: "prop:defeater".into(),
                status: PropositionStatus::Failed,
                source_refs: vec!["source:reviewed".into()],
            },
        );

        let mut context = LegalEvaluationContext {
            jurisdiction_ref: "AU-NSW".into(),
            as_at: "2026-09-20".into(),
            propositions,
            wrong_type: issue,
        };

        let first = evaluate_source_realised_rule(&rule(), &context).unwrap();
        assert_eq!(first.applicability, ApplicabilityStatus::Applicable);
        assert_eq!(first.violation, ViolationStatus::Established);
        assert_eq!(first.liability, LiabilityStatus::Established);
        assert_eq!(first.remedy, RemedyStatus::Eligible);

        context
            .propositions
            .get_mut("prop:exception")
            .unwrap()
            .status = PropositionStatus::Established;
        let reopened = evaluate_source_realised_rule(&rule(), &context).unwrap();
        assert_eq!(reopened.applicability, ApplicabilityStatus::NotApplicable);
        assert_eq!(reopened.violation, ViolationStatus::NotEstablished);
        assert_eq!(reopened.liability, LiabilityStatus::NotEstablished);
        assert_eq!(reopened.remedy, RemedyStatus::NotEligible);
        assert_eq!(reopened.live_exception_refs, vec!["prop:exception"]);
    }

    #[test]
    fn all_elements_do_not_erase_unresolved_defeater_or_burden() {
        let evidence = reviewed_text("obs:generic", "matter:rev1", "span:generic");
        let issue = project_reviewed_world_to_wrong_type(
            &negligence_bundle(),
            &[
                (&evidence, "element:duty", EvidenceDisposition::Supports),
                (&evidence, "element:breach", EvidenceDisposition::Supports),
                (&evidence, "element:causation", EvidenceDisposition::Supports),
                (&evidence, "element:damage", EvidenceDisposition::Supports),
            ],
        )
        .unwrap();
        let mut propositions = BTreeMap::new();
        propositions.insert(
            "prop:rule-enabled".into(),
            PropositionState {
                proposition_ref: "prop:rule-enabled".into(),
                status: PropositionStatus::Established,
                source_refs: vec!["source:reviewed".into()],
            },
        );
        let context = LegalEvaluationContext {
            jurisdiction_ref: "AU-NSW".into(),
            as_at: "2026-09-20".into(),
            propositions,
            wrong_type: issue,
        };
        let evaluation = evaluate_source_realised_rule(&rule(), &context).unwrap();
        assert_eq!(evaluation.applicability, ApplicabilityStatus::Unresolved);
        assert_eq!(evaluation.liability, LiabilityStatus::Unresolved);
        assert!(evaluation.unresolved_refs.contains(&"prop:defeater".into()));
        assert!(evaluation.unresolved_refs.contains(&"prop:burden-paid".into()));
    }

    #[test]
    fn m3_c_and_m4_a_one_runner_retains_residual_and_source_drilldown() {
        let evidence = reviewed_text("obs:generic", "matter:rev1", "span:generic");
        let issue = project_reviewed_world_to_wrong_type(
            &negligence_bundle(),
            &[
                (&evidence, "element:duty", EvidenceDisposition::Supports),
                (&evidence, "element:breach", EvidenceDisposition::Supports),
                (&evidence, "element:damage", EvidenceDisposition::Supports),
            ],
        )
        .unwrap();

        let mut propositions = BTreeMap::new();
        propositions.insert(
            "prop:rule-enabled".into(),
            PropositionState {
                proposition_ref: "prop:rule-enabled".into(),
                status: PropositionStatus::Established,
                source_refs: vec!["source:reviewed".into()],
            },
        );
        let context = LegalEvaluationContext {
            jurisdiction_ref: "AU-NSW".into(),
            as_at: "2026-09-20".into(),
            propositions,
            wrong_type: issue.clone(),
        };

        let state = compile_legal_campaign_state(
            "campaign:cullen-capstone",
            AustralianCalibrationKind::CullenNswCla,
            0,
            &rule(),
            &context,
            None,
        )
        .unwrap();
        assert!(state.selected_action.is_some());
        assert!(state
            .residuals
            .iter()
            .any(|residual| residual.target_ref == "element:causation"));

        let workspace = project_matter_issue_workspace("matter:cullen", &issue, &state);
        assert!(workspace.projection_only);
        assert!(!workspace.creates_semantic_authority);
        let causation = workspace
            .nodes
            .iter()
            .find(|node| node.node_ref == "element:causation")
            .unwrap();
        assert!(!causation.residual_refs.is_empty());
        assert_eq!(workspace.next_action, state.selected_action);
    }

    #[test]
    fn all_four_australian_calibrations_share_one_runner_contract() {
        for kind in [
            AustralianCalibrationKind::Mabo,
            AustralianCalibrationKind::Pabai,
            AustralianCalibrationKind::CullenNswCla,
            AustralianCalibrationKind::Glj,
        ] {
            assert!(!calibration_refs(kind).is_empty());
        }
    }
}
