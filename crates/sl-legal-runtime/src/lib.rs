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
    pub manifestation_ref: Option<String>,
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
                manifestation_ref: reviewed.manifestation_ref.clone(),
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
    let mut applicability_unresolved = Vec::new();
    let mut premise_failed = false;
    let mut premise_contested = false;
    for premise in &rule.premise_refs {
        match proposition_status(context, premise) {
            PropositionStatus::Established => {}
            PropositionStatus::Failed => premise_failed = true,
            PropositionStatus::Contested => premise_contested = true,
            PropositionStatus::Unresolved => {
                unresolved.push(premise.clone());
                applicability_unresolved.push(premise.clone());
            },
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
            applicability_unresolved.push(reference.clone());
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
    } else if !applicability_unresolved.is_empty() {
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
    FormalRuleDerivation,
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
    for element in wrong_type.elements.iter().filter(|element| element.element.required) {
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
    if evaluation.applicability == ApplicabilityStatus::Contested {
        residuals.push(LegalResidual {
            residual_ref: format!("residual:{}:formal-rule-derivation", evaluation.rule_ref),
            kind: LegalResidualKind::FormalRuleDerivation,
            target_ref: evaluation.rule_ref.clone(),
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
        LegalResidualKind::FormalRuleDerivation => InformationActionKind::Think,
        LegalResidualKind::Element
        | LegalResidualKind::Burden
        | LegalResidualKind::Remedy => InformationActionKind::Review,
        LegalResidualKind::ExceptionOrDefeater
        | LegalResidualKind::JurisdictionOrTime => InformationActionKind::Think,
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
    fn residual_priority(kind: LegalResidualKind) -> u8 {
        match kind {
            LegalResidualKind::Source => 0,
            LegalResidualKind::FormalRuleDerivation => 1,
            LegalResidualKind::JurisdictionOrTime => 2,
            LegalResidualKind::ExceptionOrDefeater => 3,
            LegalResidualKind::Element => 4,
            LegalResidualKind::Burden => 5,
            LegalResidualKind::MatterEvidence => 6,
            LegalResidualKind::Remedy => 7,
            LegalResidualKind::ClosedForConsumer => u8::MAX,
        }
    }

    let selected_action = residuals
        .iter()
        .filter(|residual| residual.kind != LegalResidualKind::ClosedForConsumer)
        .min_by_key(|residual| residual_priority(residual.kind))
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
            .map(|action| action.action_ref.clone())
            .unwrap_or_default();
        vec![
            "SLR-M3.C".to_owned(),
            self.campaign_ref.clone(),
            self.iteration.to_string(),
            format!("{:?}", self.calibration),
            self.evaluation.rule_ref.clone(),
            format!("{:?}", self.evaluation.applicability),
            format!("{:?}", self.evaluation.violation),
            format!("{:?}", self.evaluation.liability),
            format!("{:?}", self.evaluation.remedy),
            residual_refs,
            selected,
            self.previous_receipt_head.clone().unwrap_or_default(),
            self.receipt_head.clone(),
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

    pub fn validate_persisted_payload(&self, payload: &str) -> Result<(), LegalRuntimeError> {
        let (campaign_ref, receipt_head, hops) = Self::replay_summary(payload)?;
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

    pub fn validate_restart_replay(&self) -> Result<(), LegalRuntimeError> {
        self.validate_persisted_payload(&self.encode())
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationCapstone {
    pub kind: AustralianCalibrationKind,
    pub issue: WrongTypeIssueState,
    pub rule: SourceRealisedLegalRule,
    pub context: LegalEvaluationContext,
    pub campaign: PersistedLegalCampaign,
}

fn calibration_reviewed_observation(
    kind: AustralianCalibrationKind,
    suffix: &str,
) -> Result<ReviewedCanonicalEvidence, LegalRuntimeError> {
    let case = match kind {
        AustralianCalibrationKind::Mabo => "mabo",
        AustralianCalibrationKind::Pabai => "pabai",
        AustralianCalibrationKind::CullenNswCla => "cullen",
        AustralianCalibrationKind::Glj => "glj",
    };
    let revision = format!("matter:{case}:revision:1");
    let observation = EvidenceObservation {
        observation_ref: format!("observation:{case}:{suffix}"),
        source_revision_ref: revision.clone(),
        span: EvidenceSpan::text(
            revision.clone(),
            format!("span:{case}:{suffix}"),
            0,
            32,
        )
        .map_err(|error| LegalRuntimeError::InvalidEvidence(format!("{error:?}")))?,
        predicate_ref: format!("predicate:{case}:{suffix}"),
        value_ref: format!("value:{case}:{suffix}"),
        candidate_only: true,
        creates_semantic_authority: false,
        applicability_promoted: false,
        claim_truth_promoted: false,
    };
    canonical_review(
        observation,
        &format!("manifestation:{revision}"),
        &format!("review:{case}:{suffix}"),
        &format!("payment:{case}:{suffix}"),
        EvidenceCoordinateKind::Mechanism,
    )
}

fn calibration_bundle(kind: AustralianCalibrationKind) -> WrongTypeRuleBundle {
    let case = match kind {
        AustralianCalibrationKind::Mabo => "mabo",
        AustralianCalibrationKind::Pabai => "pabai",
        AustralianCalibrationKind::CullenNswCla => "cullen",
        AustralianCalibrationKind::Glj => "glj",
    };
    WrongTypeRuleBundle {
        wrong_type_ref: format!("wrong:{case}:calibration"),
        elements: vec![
            WrongElementRequirement {
                element_ref: format!("element:{case}:primary"),
                kind: LegalElementKind::Statutory,
                proposition_ref: format!("prop:{case}:primary"),
                required: true,
            },
            WrongElementRequirement {
                element_ref: format!("element:{case}:secondary"),
                kind: LegalElementKind::Causation,
                proposition_ref: format!("prop:{case}:secondary"),
                required: true,
            },
            WrongElementRequirement {
                element_ref: format!("element:{case}:remedy"),
                kind: LegalElementKind::Remedy,
                proposition_ref: format!("prop:{case}:remedy"),
                required: false,
            },
        ],
        source_rule_refs: vec![format!("rule:{case}:calibration")],
    }
}

pub fn build_australian_calibration_capstone(
    kind: AustralianCalibrationKind,
) -> Result<CalibrationCapstone, LegalRuntimeError> {
    let case = match kind {
        AustralianCalibrationKind::Mabo => "mabo",
        AustralianCalibrationKind::Pabai => "pabai",
        AustralianCalibrationKind::CullenNswCla => "cullen",
        AustralianCalibrationKind::Glj => "glj",
    };
    let evidence = calibration_reviewed_observation(kind, "reviewed-world")?;
    let bundle = calibration_bundle(kind);
    let primary = format!("element:{case}:primary");
    let secondary = format!("element:{case}:secondary");
    let remedy = format!("element:{case}:remedy");

    let element_evidence = match kind {
        AustralianCalibrationKind::Mabo => vec![
            (&evidence, primary.as_str(), EvidenceDisposition::Supports),
            (&evidence, secondary.as_str(), EvidenceDisposition::Supports),
            (&evidence, remedy.as_str(), EvidenceDisposition::Supports),
        ],
        AustralianCalibrationKind::Pabai => vec![
            (&evidence, primary.as_str(), EvidenceDisposition::Supports),
            (&evidence, secondary.as_str(), EvidenceDisposition::Supports),
        ],
        AustralianCalibrationKind::CullenNswCla => vec![
            (&evidence, primary.as_str(), EvidenceDisposition::Supports),
        ],
        AustralianCalibrationKind::Glj => vec![
            (&evidence, primary.as_str(), EvidenceDisposition::Contests),
            (&evidence, secondary.as_str(), EvidenceDisposition::Supports),
        ],
    };
    let mut issue = project_reviewed_world_to_wrong_type(&bundle, &element_evidence)?;

    let mut rule = SourceRealisedLegalRule {
        rule_ref: format!("rule:{case}:calibration"),
        source_revision_ref: format!("authority:{case}:revision:1"),
        source_span_refs: if kind == AustralianCalibrationKind::Pabai {
            Vec::new()
        } else {
            calibration_refs(kind)
                .iter()
                .map(|reference| (*reference).to_owned())
                .collect()
        },
        conclusion_ref: format!("prop:{case}:conclusion"),
        premise_refs: vec![format!("prop:{case}:rule-enabled")],
        exception_refs: vec![format!("prop:{case}:exception")],
        defeater_refs: vec![format!("prop:{case}:defeater")],
        burden_refs: vec![format!("prop:{case}:burden")],
        jurisdiction_ref: if kind == AustralianCalibrationKind::CullenNswCla {
            "AU-NSW".into()
        } else {
            "AU".into()
        },
        valid_from: "1901-01-01".into(),
        valid_to: None,
        authority_role: AuthorityRole::Binding,
        source_realised: true,
        candidate_only: true,
    };

    let mut propositions = BTreeMap::new();
    for (reference, status) in [
        (
            format!("prop:{case}:rule-enabled"),
            PropositionStatus::Established,
        ),
        (
            format!("prop:{case}:exception"),
            PropositionStatus::Failed,
        ),
        (
            format!("prop:{case}:burden"),
            PropositionStatus::Established,
        ),
    ] {
        propositions.insert(
            reference.clone(),
            PropositionState {
                proposition_ref: reference,
                status,
                source_refs: rule.source_span_refs.clone(),
            },
        );
    }
    let defeater_status = match kind {
        AustralianCalibrationKind::Pabai => PropositionStatus::Established,
        AustralianCalibrationKind::Glj => PropositionStatus::Contested,
        AustralianCalibrationKind::Mabo | AustralianCalibrationKind::CullenNswCla => {
            PropositionStatus::Failed
        }
    };
    let defeater_ref = format!("prop:{case}:defeater");
    propositions.insert(
        defeater_ref.clone(),
        PropositionState {
            proposition_ref: defeater_ref,
            status: defeater_status,
            source_refs: rule.source_span_refs.clone(),
        },
    );

    let mut context = LegalEvaluationContext {
        jurisdiction_ref: rule.jurisdiction_ref.clone(),
        as_at: "2026-09-20".into(),
        propositions,
        wrong_type: issue.clone(),
    };

    let state = compile_legal_campaign_state(
        format!("campaign:{case}:legal-capstone"),
        kind,
        0,
        &rule,
        &context,
        None,
    )?;
    let mut campaign = PersistedLegalCampaign::new(state.campaign_ref.clone());
    campaign.append(state)?;

    match kind {
        AustralianCalibrationKind::Mabo => {
            // Positive doctrinal route is already closed for this bounded
            // calibration consumer.
        }
        AustralianCalibrationKind::Pabai => {
            // Hop 0 exposes the missing exact source/pinpoint surface and
            // therefore selects Look.  Acquisition fills that coordinate,
            // then the current issue is recomputed; the live defeater remains
            // legally decisive rather than being erased.
            rule.source_span_refs = calibration_refs(kind)
                .iter()
                .map(|reference| (*reference).to_owned())
                .collect();
            for proposition in context.propositions.values_mut() {
                proposition.source_refs = rule.source_span_refs.clone();
            }
            let previous = campaign.receipt_head.clone();
            let next = compile_legal_campaign_state(
                campaign.campaign_ref.clone(),
                kind,
                1,
                &rule,
                &context,
                Some(previous),
            )?;
            campaign.append(next)?;
        }
        AustralianCalibrationKind::CullenNswCla => {
            // Hop 0 leaves the second required element unresolved and selects
            // Review.  A reviewed follow-up observation pays that coordinate;
            // re-diagnosis then closes this bounded calibration.
            let followup = calibration_reviewed_observation(kind, "secondary-review")?;
            issue = project_reviewed_world_to_wrong_type(
                &bundle,
                &[
                    (&evidence, primary.as_str(), EvidenceDisposition::Supports),
                    (&followup, secondary.as_str(), EvidenceDisposition::Supports),
                    (&followup, remedy.as_str(), EvidenceDisposition::Supports),
                ],
            )?;
            context.wrong_type = issue.clone();
            let previous = campaign.receipt_head.clone();
            let next = compile_legal_campaign_state(
                campaign.campaign_ref.clone(),
                kind,
                1,
                &rule,
                &context,
                Some(previous),
            )?;
            campaign.append(next)?;
        }
        AustralianCalibrationKind::Glj => {
            // Hop 0 has a contested formal/legal route and selects Think.  The
            // bounded formal check resolves derivability of the defeater but
            // does not establish the contested matter element; re-diagnosis
            // therefore selects Review at hop 1.  Only reviewed evidence pays
            // that matter coordinate at hop 2.
            let defeater_ref = format!("prop:{case}:defeater");
            if let Some(defeater) = context.propositions.get_mut(&defeater_ref) {
                defeater.status = PropositionStatus::Failed;
            }
            let previous = campaign.receipt_head.clone();
            let after_think = compile_legal_campaign_state(
                campaign.campaign_ref.clone(),
                kind,
                1,
                &rule,
                &context,
                Some(previous),
            )?;
            campaign.append(after_think)?;

            let reviewed_resolution =
                calibration_reviewed_observation(kind, "reviewed-resolution")?;
            issue = project_reviewed_world_to_wrong_type(
                &bundle,
                &[
                    (
                        &reviewed_resolution,
                        primary.as_str(),
                        EvidenceDisposition::Supports,
                    ),
                    (
                        &evidence,
                        secondary.as_str(),
                        EvidenceDisposition::Supports,
                    ),
                    (
                        &reviewed_resolution,
                        remedy.as_str(),
                        EvidenceDisposition::Supports,
                    ),
                ],
            )?;
            context.wrong_type = issue.clone();
            let previous = campaign.receipt_head.clone();
            let after_review = compile_legal_campaign_state(
                campaign.campaign_ref.clone(),
                kind,
                2,
                &rule,
                &context,
                Some(previous),
            )?;
            campaign.append(after_review)?;
        }
    }

    campaign.validate_restart_replay()?;

    Ok(CalibrationCapstone {
        kind,
        issue,
        rule,
        context,
        campaign,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalRuntimeCapabilityReceipt {
    pub m2_5_mixed_family_replay: bool,
    pub m3_a_reviewed_world_to_wrong_type: bool,
    pub m3_b_source_realised_evaluator: bool,
    pub m3_c_all_calibrations_one_runner: bool,
    pub m3_c_restart_replay: bool,
    pub m4_a_matter_issue_projection: bool,
    pub m6_universal_explanation: bool,
    pub m6_reverse_material_impact: bool,
    pub m7_projection_fabric: bool,
    pub m7_same_identity_cross_projection: bool,
    pub s8_matter_runtime: bool,
    pub s8_shared_command_reducer: bool,
    pub s8_reader_command_weld: bool,
    pub s8_unseen_contract_matter: bool,
    pub s8_contract_follow_trace: bool,
    pub s11_visualisation_ir: bool,
    pub s11_research_flow_sankey: bool,
    pub s14_6_generic_campaign_kernel: bool,
    pub s14_7_consumer_adequacy_runtime: bool,
    pub s15_theorem_adequacy_bridge: bool,
    pub s15_exact_residual_compiler: bool,
    pub s15_admissible_research_frontier: bool,
    pub s15_stop_semantics: bool,
    pub s15_adequacy_witness_compiler: bool,
    pub s15_query_dependency_slice: bool,
    pub s16_source_realised_generic_legal_follow: bool,
    pub s18_first_class_legal_world: bool,
    pub s18_authority_validity: bool,
    pub s18_jurisdiction_scoped_coverage: bool,
    pub s18_revision_invalidation: bool,
    pub s18_affected_proof_cone: bool,
    pub s18_revision_consumer_reopening: bool,
    pub s18_query_scoped_world_impact: bool,
    pub s15_s18_query_world_run_controller: bool,
    pub s19_shared_world_consumer_join: bool,
    pub s19_shared_world_quotient_reuse: bool,
    pub s19_affected_consumer_recompute: bool,
    pub s20_adversarial_proof_search: bool,
    pub s20_defeat_counterdefeat_rerun: bool,
    pub s21_source_driven_case_battery: bool,
    pub candidate_only: bool,
    pub creates_semantic_authority: bool,
    pub receipt_digest: String,
}

pub fn compile_capability_receipt() -> Result<LegalRuntimeCapabilityReceipt, LegalRuntimeError> {
    let mixed = build_m2_5_mixed_family_campaign()?;
    mixed.validate_exact_replay(&MixedFamilyReplayReceipt::decode(&mixed.encode())?)?;

    let mut capstones = Vec::new();
    for kind in [
        AustralianCalibrationKind::Mabo,
        AustralianCalibrationKind::Pabai,
        AustralianCalibrationKind::CullenNswCla,
        AustralianCalibrationKind::Glj,
    ] {
        capstones.push(build_australian_calibration_capstone(kind)?);
    }
    for capstone in &capstones {
        capstone.campaign.validate_restart_replay()?;
        let last = capstone
            .campaign
            .hops
            .last()
            .ok_or_else(|| LegalRuntimeError::ReplayFormat("missing legal campaign hop".into()))?;
        let workspace =
            project_matter_issue_workspace(format!("matter:{:?}", capstone.kind), &capstone.issue, last);
        if !workspace.projection_only || workspace.creates_semantic_authority {
            return Err(LegalRuntimeError::Projection(
                "matter/issue workspace must remain projection-only".into(),
            ));
        }

        let first_evidence = capstone
            .issue
            .elements
            .iter()
            .flat_map(|element| element.evidence.iter())
            .next()
            .ok_or_else(|| LegalRuntimeError::Projection(
                "Sprint 6/7 capstone requires one source-addressable observation".into(),
            ))?;
        let anchor = first_evidence.observation_ref.clone();
        let time_ref = "2026-09-20T00:00:00+10:00".to_owned();
        let workbench = project_matter_issue_workbench(
            format!("matter:{:?}", capstone.kind),
            &capstone.issue,
            last,
            MatterWorkbenchSeed {
                events: vec![MatterEventProjection {
                    event_ref: format!("event:{:?}:m6-m7-capstone", capstone.kind),
                    label: format!("{:?} Sprint 6/7 capstone event", capstone.kind),
                    time_ref: time_ref.clone(),
                    observation_refs: vec![anchor.clone()],
                    entity_refs: Vec::new(),
                    candidate_only: true,
                }],
                observation_time_refs: BTreeMap::from([(anchor.clone(), time_ref.clone())]),
                ..MatterWorkbenchSeed::default()
            },
        )
        .map_err(LegalRuntimeError::Projection)?;

        let explanation = compile_explanation_index(&workbench, &capstone.issue, last)
            .map_err(LegalRuntimeError::Projection)?;
        let anchor_explanation = explanation.get(&anchor).ok_or_else(|| {
            LegalRuntimeError::Projection(
                "Sprint 6 explanation lost the canonical observation anchor".into(),
            )
        })?;
        if anchor_explanation.provenance.is_empty()
            || !explanation
                .material_consequences(&anchor)
                .contains(&format!("event:{:?}:m6-m7-capstone", capstone.kind))
        {
            return Err(LegalRuntimeError::Projection(
                "Sprint 6 backward/forward explanation invariant failed".into(),
            ));
        }

        let projection_context = ProjectionContext {
            temporal_refs: BTreeMap::from([(anchor.clone(), time_ref)]),
            jurisdiction_refs: BTreeMap::new(),
        };
        for projection_kind in [
            ProjectionKind::SourceView,
            ProjectionKind::Timeline,
            ProjectionKind::IssueProof,
            ProjectionKind::EntityRelationship,
            ProjectionKind::CitationAuthority,
            ProjectionKind::Flow,
            ProjectionKind::Comparative,
        ] {
            let mut query = ProjectionQuery::new(projection_kind);
            query.semantic_selection.insert(anchor.clone());
            let graph = compile_projection(&workbench, &explanation, &query, &projection_context)
                .map_err(LegalRuntimeError::Projection)?;
            let projected = graph
                .nodes
                .iter()
                .find(|node| node.semantic_ref == anchor)
                .ok_or_else(|| LegalRuntimeError::Projection(format!(
                    "Sprint 7 projection {projection_kind:?} lost canonical anchor"
                )))?;
            let expected_revisions = anchor_explanation
                .provenance
                .iter()
                .map(|provenance| provenance.source_revision_ref.clone())
                .collect::<BTreeSet<_>>();
            let projected_revisions = projected
                .source_revision_refs
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>();
            if expected_revisions != projected_revisions || graph.creates_semantic_authority {
                return Err(LegalRuntimeError::Projection(format!(
                    "Sprint 7 projection {projection_kind:?} rewrote canonical provenance"
                )));
            }

            let visual = visualisation_from_projection(&graph)
                .map_err(LegalRuntimeError::Projection)?;
            if !visual.projection_only
                || visual.creates_semantic_authority
                || visual.creates_legal_authority
            {
                return Err(LegalRuntimeError::Projection(
                    "Sprint 11 VisualisationIR crossed authority boundary".into(),
                ));
            }
            if projection_kind == ProjectionKind::Flow {
                let VisualisationIr::Sankey(sankey) = visual.ir else {
                    return Err(LegalRuntimeError::Projection(
                        "Sprint 11 flow projection did not lower to SankeyIR".into(),
                    ));
                };
                if sankey.weights_are_legal_importance {
                    return Err(LegalRuntimeError::Projection(
                        "Sprint 11 Sankey weight was promoted to legal importance".into(),
                    ));
                }
            }
        }

        // Runtime adequacy is intentionally weaker than the formal Agda
        // FactorsThrough witness.  Here source identity/provenance are paid by
        // the projection, while treatment remains a typed research demand.
        let mut adequacy_query = ProjectionQuery::new(ProjectionKind::IssueProof);
        adequacy_query.semantic_selection.insert(anchor.clone());
        let adequacy_graph = compile_projection(
            &workbench,
            &explanation,
            &adequacy_query,
            &projection_context,
        )
        .map_err(LegalRuntimeError::Projection)?;
        let adequacy = assess_consumer_adequacy(
            &ConsumerQueryDemand {
                query_ref: format!("query:{:?}:source-plus-treatment", capstone.kind),
                required_axes: BTreeSet::from([
                    ConsumerAxis::SemanticIdentity,
                    ConsumerAxis::SourceRevision,
                    ConsumerAxis::SourceSpan,
                    ConsumerAxis::Provenance,
                    ConsumerAxis::Treatment,
                ]),
                required_semantic_refs: BTreeSet::from([anchor.clone()]),
                candidate_only: true,
                creates_semantic_authority: false,
            },
            &adequacy_graph,
            &ConsumerCoverage::default(),
        )
        .map_err(LegalRuntimeError::Projection)?;
        if adequacy.disposition != ConsumerAdequacyDisposition::NeedsResearch
            || adequacy.research_demands.len() != 1
            || adequacy.research_demands[0].kind
                != ConsumerResearchDemandKind::ReviewTreatment
            || adequacy.factors_through_formally_proved
            || adequacy.creates_semantic_authority
            || adequacy.creates_claim_truth
        {
            return Err(LegalRuntimeError::Projection(
                "Sprint 14.7 consumer-adequacy routing invariant failed".into(),
            ));
        }

        let mut runtime = compile_matter_runtime(
            workbench.clone(),
            &capstone.issue,
            last,
            projection_context.clone(),
        )
        .map_err(LegalRuntimeError::Projection)?;
        let select = runtime
            .dispatch(MatterCommand::SelectObject(anchor.clone()))
            .map_err(LegalRuntimeError::Projection)?;
        if !select.projection.contains(&anchor)
            || select.creates_semantic_authority
            || !select.candidate_only
        {
            return Err(LegalRuntimeError::Projection(
                "Sprint 8 shared reducer failed selection/non-authority invariant".into(),
            ));
        }
        let reader_command = lower_reader_intent(
            sensiblaw_reader_model::ReaderIntent::OpenSource,
            Some(anchor.as_str()),
        )
        .map_err(LegalRuntimeError::Projection)?;
        let source = runtime
            .dispatch(reader_command)
            .map_err(LegalRuntimeError::Projection)?;
        if !matches!(source.effect, MatterRuntimeEffect::SourceOpened { .. })
            || source.creates_semantic_authority
        {
            return Err(LegalRuntimeError::Projection(
                "Sprint 8 reader command weld failed source-open invariant".into(),
            ));
        }
    }

    let unseen_contract = build_mann_unseen_matter_runtime()?;
    if unseen_contract.used_calibration_enum
        || unseen_contract.contract_specific_reducer_added
        || unseen_contract.creates_semantic_authority
        || !unseen_contract.trace.nodes.contains_key("matter:au:hca:2019:32")
    {
        return Err(LegalRuntimeError::Projection(
            "Sprint 8 unseen contract matter crossed generic-runtime boundary".into(),
        ));
    }

    generic_campaign_kernel_self_check()
        .map_err(LegalRuntimeError::Projection)?;

    source_realised_legal_campaign_self_check()
        .map_err(LegalRuntimeError::Projection)?;

    query_scoped_world_impact_self_check()
        .map_err(LegalRuntimeError::Projection)?;

    query_world_run_controller_self_check()
        .map_err(LegalRuntimeError::Projection)?;

    let research_flow = research_flow_sankey(&[
        ResearchFlowEvent {
            event_ref: "capability:frontier-to-demand".into(),
            from_stage: ResearchFlowStage::FrontierResidual,
            to_stage: ResearchFlowStage::SelectedDemand,
            count: 1,
            candidate_only: true,
        },
        ResearchFlowEvent {
            event_ref: "capability:demand-to-source".into(),
            from_stage: ResearchFlowStage::SelectedDemand,
            to_stage: ResearchFlowStage::SourceAcquired,
            count: 1,
            candidate_only: true,
        },
    ])
    .map_err(LegalRuntimeError::Projection)?;
    if research_flow.sankey.weights_are_legal_importance
        || research_flow.creates_semantic_authority
    {
        return Err(LegalRuntimeError::Projection(
            "Sprint 11 research-flow Sankey crossed semantic boundary".into(),
        ));
    }

    let receipt_digest = digest(
        std::iter::once(LEGAL_RUNTIME_VERSION)
            .chain(std::iter::once(mixed.receipt_head.as_str()))
            .chain(capstones.iter().map(|capstone| capstone.campaign.receipt_head.as_str()))
            .chain(std::iter::once("M6:universal-explanation"))
            .chain(std::iter::once("M7:projection-fabric"))
            .chain(std::iter::once("S8:matter-runtime"))
            .chain(std::iter::once("S8:shared-command-reducer"))
            .chain(std::iter::once("S8:unseen-contract-matter"))
            .chain(std::iter::once("S8:contract-follow-trace"))
            .chain(std::iter::once("S11:visualisation-ir"))
            .chain(std::iter::once("S11:research-flow-sankey"))
            .chain(std::iter::once("S14.6:generic-campaign-kernel"))
            .chain(std::iter::once("S14.7:consumer-adequacy-runtime"))
            .chain(std::iter::once("S15:theorem-adequacy-bridge"))
            .chain(std::iter::once("S15:exact-residual-compiler"))
            .chain(std::iter::once("S15:admissible-research-frontier"))
            .chain(std::iter::once("S15:stop-semantics"))
            .chain(std::iter::once("S15:adequacy-witness-compiler"))
            .chain(std::iter::once("S15:query-dependency-slice"))
            .chain(std::iter::once("S16:source-realised-generic-legal-follow"))
            .chain(std::iter::once("S18:first-class-legal-world"))
            .chain(std::iter::once("S18:authority-validity"))
            .chain(std::iter::once("S18:jurisdiction-scoped-coverage"))
            .chain(std::iter::once("S18:revision-invalidation"))
            .chain(std::iter::once("S18:affected-proof-cone"))
            .chain(std::iter::once("S18:revision-consumer-reopening"))
            .chain(std::iter::once("S18:query-scoped-world-impact"))
            .chain(std::iter::once("S15/S18:query-world-run-controller")),
    );

    Ok(LegalRuntimeCapabilityReceipt {
        m2_5_mixed_family_replay: true,
        m3_a_reviewed_world_to_wrong_type: true,
        m3_b_source_realised_evaluator: true,
        m3_c_all_calibrations_one_runner: true,
        m3_c_restart_replay: true,
        m4_a_matter_issue_projection: true,
        m6_universal_explanation: true,
        m6_reverse_material_impact: true,
        m7_projection_fabric: true,
        m7_same_identity_cross_projection: true,
        s8_matter_runtime: true,
        s8_shared_command_reducer: true,
        s8_reader_command_weld: true,
        s8_unseen_contract_matter: true,
        s8_contract_follow_trace: true,
        s11_visualisation_ir: true,
        s11_research_flow_sankey: true,
        s14_6_generic_campaign_kernel: true,
        s14_7_consumer_adequacy_runtime: true,
        s15_theorem_adequacy_bridge: true,
        s15_exact_residual_compiler: true,
        s15_admissible_research_frontier: true,
        s15_stop_semantics: true,
        s15_adequacy_witness_compiler: true,
        s15_query_dependency_slice: true,
        s16_source_realised_generic_legal_follow: true,
        s18_first_class_legal_world: true,
        s18_authority_validity: true,
        s18_jurisdiction_scoped_coverage: true,
        s18_revision_invalidation: true,
        s18_affected_proof_cone: true,
        s18_revision_consumer_reopening: true,
        s18_query_scoped_world_impact: true,
        s15_s18_query_world_run_controller: true,
        s19_shared_world_consumer_join: true,
        s19_shared_world_quotient_reuse: true,
        s19_affected_consumer_recompute: true,
        s20_adversarial_proof_search: true,
        s20_defeat_counterdefeat_rerun: true,
        s21_source_driven_case_battery: true,
        candidate_only: true,
        creates_semantic_authority: false,
        receipt_digest,
    })
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegalProjectionState {
    pub evaluation: SourceRealisedLegalEvaluation,
    pub residuals: Vec<LegalResidual>,
    pub selected_action: Option<InformationAction>,
}

impl From<&LegalCampaignState> for LegalProjectionState {
    fn from(campaign: &LegalCampaignState) -> Self {
        Self {
            evaluation: campaign.evaluation.clone(),
            residuals: campaign.residuals.clone(),
            selected_action: campaign.selected_action.clone(),
        }
    }
}

pub fn project_matter_issue_workspace_from_state(
    matter_ref: impl Into<String>,
    issue: &WrongTypeIssueState,
    state: &LegalProjectionState,
) -> MatterWorkspaceProjection {
    let issue_ref = format!("issue:{}", issue.wrong_type_ref);
    let mut nodes = Vec::new();

    nodes.push(SourceAddressableNode {
        node_ref: issue_ref.clone(),
        label: issue.wrong_type_ref.clone(),
        semantic_kind: "legal-issue".into(),
        source_revision_refs: vec![state.evaluation.source_revision_ref.clone()],
        span_refs: state.evaluation.source_span_refs.clone(),
        dependency_refs: issue
            .elements
            .iter()
            .map(|element| element.element.element_ref.clone())
            .collect(),
        downstream_refs: vec![state.evaluation.rule_ref.clone()],
        residual_refs: state
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
            residual_refs: state
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
        applicability: state.evaluation.applicability,
        violation: state.evaluation.violation,
        liability: state.evaluation.liability,
        remedy: state.evaluation.remedy,
        next_action: state.selected_action.clone(),
        projection_only: true,
        creates_semantic_authority: false,
    }
}

pub fn project_matter_issue_workspace(
    matter_ref: impl Into<String>,
    issue: &WrongTypeIssueState,
    campaign: &LegalCampaignState,
) -> MatterWorkspaceProjection {
    let state = LegalProjectionState::from(campaign);
    project_matter_issue_workspace_from_state(matter_ref, issue, &state)
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
    fn unresolved_burden_does_not_leak_backward_into_applicability() {
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
        for reference in ["prop:rule-enabled"] {
            propositions.insert(
                reference.into(),
                PropositionState {
                    proposition_ref: reference.into(),
                    status: PropositionStatus::Established,
                    source_refs: vec!["source:reviewed".into()],
                },
            );
        }
        for reference in ["prop:exception", "prop:defeater"] {
            propositions.insert(
                reference.into(),
                PropositionState {
                    proposition_ref: reference.into(),
                    status: PropositionStatus::Failed,
                    source_refs: vec!["source:reviewed".into()],
                },
            );
        }
        let context = LegalEvaluationContext {
            jurisdiction_ref: "AU-NSW".into(),
            as_at: "2026-09-20".into(),
            propositions,
            wrong_type: issue,
        };
        let evaluation = evaluate_source_realised_rule(&rule(), &context).unwrap();
        assert_eq!(evaluation.applicability, ApplicabilityStatus::Applicable);
        assert_eq!(evaluation.violation, ViolationStatus::Established);
        assert_eq!(evaluation.liability, LiabilityStatus::Unresolved);
        assert!(evaluation.unresolved_refs.contains(&"prop:burden-paid".into()));
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
            let capstone = build_australian_calibration_capstone(kind).unwrap();
            assert_eq!(capstone.kind, kind);
            let expected_hops = match kind {
                AustralianCalibrationKind::Mabo => 1,
                AustralianCalibrationKind::Pabai => 2,
                AustralianCalibrationKind::CullenNswCla => 2,
                AustralianCalibrationKind::Glj => 3,
            };
            assert_eq!(capstone.campaign.hops.len(), expected_hops);
            capstone.campaign.validate_restart_replay().unwrap();
            let last = capstone.campaign.hops.last().unwrap();
            let workspace =
                project_matter_issue_workspace(format!("matter:{kind:?}"), &capstone.issue, last);
            assert!(workspace.projection_only);
            assert!(!workspace.creates_semantic_authority);
        }
    }

    #[test]
    fn calibration_suite_exercises_look_think_review_and_rerun() {
        let pabai =
            build_australian_calibration_capstone(AustralianCalibrationKind::Pabai).unwrap();
        assert_eq!(
            pabai.campaign.hops[0].selected_action.as_ref().unwrap().kind,
            InformationActionKind::Look
        );
        assert!(pabai.campaign.hops[1].selected_action.is_none());

        let cullen =
            build_australian_calibration_capstone(AustralianCalibrationKind::CullenNswCla).unwrap();
        assert_eq!(
            cullen.campaign.hops[0].selected_action.as_ref().unwrap().kind,
            InformationActionKind::Review
        );
        assert!(cullen.campaign.hops[1].selected_action.is_none());

        let glj =
            build_australian_calibration_capstone(AustralianCalibrationKind::Glj).unwrap();
        assert_eq!(
            glj.campaign.hops[0].selected_action.as_ref().unwrap().kind,
            InformationActionKind::Think
        );
        assert_eq!(
            glj.campaign.hops[1].selected_action.as_ref().unwrap().kind,
            InformationActionKind::Review
        );
        assert!(glj.campaign.hops[2].selected_action.is_none());
    }

    #[test]
    fn glj_contested_formal_route_selects_think_without_authority() {
        let capstone =
            build_australian_calibration_capstone(AustralianCalibrationKind::Glj).unwrap();
        let hop = capstone.campaign.hops.first().unwrap();
        let action = hop.selected_action.as_ref().unwrap();
        assert_eq!(action.kind, InformationActionKind::Think);
        assert!(action.candidate_only);
        assert!(!action.creates_authority);
    }

    #[test]
    fn capability_receipt_closes_m2_5_through_sprint8_without_promotion() {
        let receipt = compile_capability_receipt().unwrap();
        assert!(receipt.m2_5_mixed_family_replay);
        assert!(receipt.m3_a_reviewed_world_to_wrong_type);
        assert!(receipt.m3_b_source_realised_evaluator);
        assert!(receipt.m3_c_all_calibrations_one_runner);
        assert!(receipt.m3_c_restart_replay);
        assert!(receipt.m4_a_matter_issue_projection);
        assert!(receipt.m6_universal_explanation);
        assert!(receipt.m6_reverse_material_impact);
        assert!(receipt.m7_projection_fabric);
        assert!(receipt.m7_same_identity_cross_projection);
        assert!(receipt.s8_matter_runtime);
        assert!(receipt.s8_shared_command_reducer);
        assert!(receipt.s8_reader_command_weld);
        assert!(receipt.s8_unseen_contract_matter);
        assert!(receipt.s8_contract_follow_trace);
        assert!(receipt.s15_query_dependency_slice);
        assert!(receipt.s16_source_realised_generic_legal_follow);
        assert!(receipt.s18_query_scoped_world_impact);
        assert!(receipt.s15_s18_query_world_run_controller);
        assert!(receipt.s19_shared_world_consumer_join);
        assert!(receipt.s19_shared_world_quotient_reuse);
        assert!(receipt.s19_affected_consumer_recompute);
        assert!(receipt.s20_adversarial_proof_search);
        assert!(receipt.s20_defeat_counterdefeat_rerun);
        assert!(receipt.s21_source_driven_case_battery);
        assert!(receipt.candidate_only);
        assert!(!receipt.creates_semantic_authority);
        assert!(receipt.receipt_digest.starts_with("sha256:"));
    }
}


pub mod workbench;

pub mod contract_specimens;
pub use contract_specimens::{
    build_mann_unseen_matter_runtime, project_waltons_reviewed_receipts_to_issue,
    waltons_estoppel_materialisation_specimen,
};
pub use workbench::*;


pub mod provenance;
pub use provenance::*;
pub mod projection_fabric;
pub use projection_fabric::*;


pub mod matter_runtime;
pub use matter_runtime::*;
pub mod consumer_adequacy;
pub use consumer_adequacy::*;
pub mod consumer_theorem_bridge;
pub use consumer_theorem_bridge::*;
pub mod consumer_research_planner;
pub use consumer_research_planner::*;
pub mod consumer_adequacy_compiler;
pub use consumer_adequacy_compiler::*;
pub mod visualisation_ir;
pub use visualisation_ir::*;
pub mod legal_follow_campaign;
pub use legal_follow_campaign::*;
pub mod source_realised_legal_follow;
pub use source_realised_legal_follow::*;
pub mod legal_world;
pub use legal_world::*;
pub mod shared_world;
pub use shared_world::*;
pub mod personal_world_handoff;
pub use personal_world_handoff::*;
pub mod personal_world_fact_review_adapter;
pub use personal_world_fact_review_adapter::*;
pub mod personal_world_scope_receipt;
pub use personal_world_scope_receipt::*;
pub mod wave5_professional_handoff_run;
pub use wave5_professional_handoff_run::*;
pub mod adversarial_proof_search;
pub use adversarial_proof_search::*;
pub mod finite_legal_cut;
pub use finite_legal_cut::*;
pub mod reviewed_treatment_compiler;
pub use reviewed_treatment_compiler::*;
pub mod legal_case_battery;
pub use legal_case_battery::*;
pub mod legal_case_battery_plan;
pub use legal_case_battery_plan::*;
pub mod yindjibarndi_empirical;
pub use yindjibarndi_empirical::*;
pub mod yindjibarndi_live_run;
pub use yindjibarndi_live_run::*;
pub mod yindjibarndi_finite_cut;
pub use yindjibarndi_finite_cut::*;
pub mod yindjibarndi_affected_consumers;
pub use yindjibarndi_affected_consumers::*;
pub mod munkara_tipakalippa_noncollapse;
pub use munkara_tipakalippa_noncollapse::*;
pub mod murujuga_open_discovery;
pub use murujuga_open_discovery::*;
pub mod pabai_golden_regression;
pub use pabai_golden_regression::*;
pub mod comparative_world;
pub use comparative_world::*;
pub mod pabai_comparative_world;
pub use pabai_comparative_world::*;
pub mod revision_consumer_bridge;
pub use revision_consumer_bridge::*;
pub mod query_revision_impact;
pub use query_revision_impact::*;
pub mod query_world_run_controller;
pub use query_world_run_controller::*;
pub mod personal_professional_comparison;
pub use personal_professional_comparison::*;

pub mod temporal_comparative_world;
pub use temporal_comparative_world::*;

pub mod adversarial_party_comparison;
pub use adversarial_party_comparison::*;

pub mod comparative_receipt;
pub use comparative_receipt::*;

pub mod change_locus;
pub use change_locus::*;

pub mod gravity_comparative_regressions;
pub use gravity_comparative_regressions::*;

pub mod effective_theory_comparison;
pub use effective_theory_comparison::*;

pub mod typed_answer_explanation;
pub use typed_answer_explanation::*;

pub mod comparative_empirical_battery;
pub use comparative_empirical_battery::*;

pub mod typed_comparative_receipt;
pub use typed_comparative_receipt::*;

pub mod comparative_change_adapters;
pub use comparative_change_adapters::*;
