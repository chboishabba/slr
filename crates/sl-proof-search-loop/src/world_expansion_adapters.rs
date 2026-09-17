//! Typed producer-artifact adapters for residual-driven world expansion.
//!
//! These functions perform no acquisition, parsing, review, persistence, or
//! semantic promotion. They only project already-governed producer artifacts
//! into `ExpansionCandidate` while retaining the exact triggering residual and
//! source-revision coordinates needed by the P7d controller.

use crate::frontier::{ProofResidual, ResidualStatus};
use crate::world_expansion::{
    ExpansionCandidate, KnowledgeObjectKind, ProducerLane, ResidualClass,
};
use sensiblaw_governed_legal_provider::OalcLookupReceipt;
use sensiblaw_route_executor::AcquiredSource;
use sensiblaw_route_selector::{RouteCandidate, RouteFamily};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpansionScoring {
    pub expected_residual_contraction: u64,
    pub provenance_quality: u64,
    pub same_object_confidence: u64,
    pub expected_new_world_value: u64,
    pub acquisition_cost: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpansionAdapterError {
    ResidualNotOpen,
    WrongWikidataRoute,
    WikipediaSourceNotCandidateOnly,
    WikipediaSourcePromoted,
}

fn require_open(residual: &ProofResidual) -> Result<(), ExpansionAdapterError> {
    if residual.status == ResidualStatus::Open {
        Ok(())
    } else {
        Err(ExpansionAdapterError::ResidualNotOpen)
    }
}

fn candidate(
    residual: &ProofResidual,
    residual_class: ResidualClass,
    candidate_ref: String,
    object_ref: String,
    object_kind: KnowledgeObjectKind,
    discovery_parent_ref: String,
    producer_lane: ProducerLane,
    source_revision_ref: Option<String>,
    scoring: ExpansionScoring,
) -> ExpansionCandidate {
    ExpansionCandidate {
        candidate_ref,
        object_ref,
        object_kind,
        discovery_parent_ref,
        triggering_residual_ref: residual.residual_ref.clone(),
        residual_class,
        producer_lane,
        source_revision_ref,
        expected_residual_contraction: scoring.expected_residual_contraction,
        provenance_quality: scoring.provenance_quality,
        same_object_confidence: scoring.same_object_confidence,
        expected_new_world_value: scoring.expected_new_world_value,
        acquisition_cost: scoring.acquisition_cost,
        admissible: true,
    }
}

pub fn from_oalc_lookup(
    residual: &ProofResidual,
    residual_class: ResidualClass,
    discovery_parent_ref: &str,
    receipt: &OalcLookupReceipt,
    scoring: ExpansionScoring,
) -> Result<ExpansionCandidate, ExpansionAdapterError> {
    require_open(residual)?;
    Ok(candidate(
        residual,
        residual_class,
        format!("oalc:{}", receipt.source_revision_ref),
        receipt.source_identity_ref.clone(),
        KnowledgeObjectKind::PrimaryLegalSource,
        discovery_parent_ref.to_owned(),
        ProducerLane::GovernedLegal,
        Some(receipt.source_revision_ref.clone()),
        scoring,
    ))
}

pub fn from_wikidata_route(
    residual: &ProofResidual,
    residual_class: ResidualClass,
    source_revision_ref: &str,
    route: &RouteCandidate,
    scoring: ExpansionScoring,
) -> Result<ExpansionCandidate, ExpansionAdapterError> {
    require_open(residual)?;
    if route.route_family != RouteFamily::WikidataProperty {
        return Err(ExpansionAdapterError::WrongWikidataRoute);
    }
    Ok(candidate(
        residual,
        residual_class,
        route.candidate_id.clone(),
        route.target_ref.clone(),
        KnowledgeObjectKind::Qid,
        route.source_ref.clone(),
        ProducerLane::WikidataIdentity,
        Some(source_revision_ref.to_owned()),
        scoring,
    ))
}

pub fn from_wikipedia_source(
    residual: &ProofResidual,
    residual_class: ResidualClass,
    source: &AcquiredSource,
    scoring: ExpansionScoring,
) -> Result<ExpansionCandidate, ExpansionAdapterError> {
    require_open(residual)?;
    if !source.candidate_only {
        return Err(ExpansionAdapterError::WikipediaSourceNotCandidateOnly);
    }
    if source.semantic_promotion {
        return Err(ExpansionAdapterError::WikipediaSourcePromoted);
    }
    Ok(candidate(
        residual,
        residual_class,
        source.document_ref.clone(),
        source.canonical_url.clone(),
        KnowledgeObjectKind::Article,
        source.source_ref.clone(),
        ProducerLane::WikipediaContext,
        Some(source.revision_ref.clone()),
        scoring,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frontier::{ProofResidual, ResidualStatus};
    use crate::world_expansion::{KnowledgeObjectKind, ProducerLane, ResidualClass};
    use sensiblaw_governed_legal_provider::OalcLookupReceipt;
    use sensiblaw_route_executor::{AcquiredSource, AcquiredSourceKind};
    use sensiblaw_route_selector::{ProducerFamily, RouteCandidate, RouteFamily};

    fn residual(status: ResidualStatus) -> ProofResidual {
        ProofResidual {
            residual_ref: "residual:mabo:source-follow".into(),
            proposition_ref: "mabo:proposition:radical-title-native-title".into(),
            producer_class_ref: "producer:world-expansion".into(),
            jurisdiction_ref: Some("AU".into()),
            authority_requirement_ref: Some("official-primary-case".into()),
            salience: 100,
            dependency_refs: vec![],
            status,
        }
    }

    fn scoring() -> ExpansionScoring {
        ExpansionScoring {
            expected_residual_contraction: 4,
            provenance_quality: 5,
            same_object_confidence: 5,
            expected_new_world_value: 4,
            acquisition_cost: 1,
        }
    }

    fn route(
        producer: ProducerFamily,
        route_family: RouteFamily,
        source_ref: &str,
        target_ref: &str,
        property_ref: &str,
    ) -> RouteCandidate {
        RouteCandidate {
            candidate_id: format!("route:{source_ref}:{target_ref}"),
            producer,
            route_family,
            source_ref: source_ref.into(),
            target_ref: target_ref.into(),
            property_ref: property_ref.into(),
            cross_language_gap_coverage: 0,
            source_surface_support: 1,
            root_qid_support: 1,
            typed_property_support: 1,
            route_specificity: 3,
            yield_history_observed: 0,
            prior_contracted_old_gaps: 0,
            prior_retired_obligations: 0,
            prior_new_gap_atoms: 0,
            prior_network_requests: 0,
        }
    }

    #[test]
    fn oalc_receipt_projects_to_legal_primary_source_candidate() {
        let receipt = OalcLookupReceipt {
            corpus_revision_ref: "oalc:rev:2026-09-17".into(),
            citation: "[1992] HCA 23".into(),
            source_identity_ref: "case:[1992]-HCA-23".into(),
            source_revision_ref: "oalc:[1992]-HCA-23:sha256:abc".into(),
            canonical_text_digest: "abc".into(),
            local_artifact_ref: "oalc://[1992]-HCA-23".into(),
            network_requests: 0,
            receipt_authority: "experimental_candidate_only",
        };

        let candidate = from_oalc_lookup(
            &residual(ResidualStatus::Open),
            ResidualClass::Legal,
            "Q1501525",
            &receipt,
            scoring(),
        )
        .unwrap();

        assert_eq!(candidate.object_ref, "case:[1992]-HCA-23");
        assert_eq!(candidate.object_kind, KnowledgeObjectKind::PrimaryLegalSource);
        assert_eq!(candidate.producer_lane, ProducerLane::GovernedLegal);
        assert_eq!(candidate.discovery_parent_ref, "Q1501525");
        assert_eq!(candidate.triggering_residual_ref, "residual:mabo:source-follow");
        assert_eq!(
            candidate.source_revision_ref.as_deref(),
            Some("oalc:[1992]-HCA-23:sha256:abc")
        );
    }

    #[test]
    fn wikidata_property_route_projects_target_qid_with_pinned_revision() {
        let route = route(
            ProducerFamily::IdentitySource,
            RouteFamily::WikidataProperty,
            "Q1501525",
            "Q975866",
            "P710",
        );
        let candidate = from_wikidata_route(
            &residual(ResidualStatus::Open),
            ResidualClass::Identity,
            "wikidata:Q1501525:oldid:2333409615",
            &route,
            scoring(),
        )
        .unwrap();

        assert_eq!(candidate.object_ref, "Q975866");
        assert_eq!(candidate.object_kind, KnowledgeObjectKind::Qid);
        assert_eq!(candidate.producer_lane, ProducerLane::WikidataIdentity);
        assert_eq!(candidate.discovery_parent_ref, "Q1501525");
        assert_eq!(
            candidate.source_revision_ref.as_deref(),
            Some("wikidata:Q1501525:oldid:2333409615")
        );
    }

    #[test]
    fn acquired_wikipedia_article_projects_canonical_article_not_revision_as_identity() {
        let source = AcquiredSource {
            kind: AcquiredSourceKind::WikipediaRenderedHtml,
            document_ref: "wikipedia:Q1501525:en:etag:123".into(),
            source_ref: "Q1501525".into(),
            language: "en".into(),
            revision_ref: "etag:123".into(),
            canonical_url: "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)".into(),
            source_sha256: [7; 32],
            text: "Mabo v Queensland (No 2) ...".into(),
            candidate_only: true,
            semantic_promotion: false,
        };

        let candidate = from_wikipedia_source(
            &residual(ResidualStatus::Open),
            ResidualClass::Context,
            &source,
            scoring(),
        )
        .unwrap();

        assert_eq!(
            candidate.object_ref,
            "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)"
        );
        assert_eq!(candidate.object_kind, KnowledgeObjectKind::Article);
        assert_eq!(candidate.producer_lane, ProducerLane::WikipediaContext);
        assert_eq!(candidate.discovery_parent_ref, "Q1501525");
        assert_eq!(candidate.source_revision_ref.as_deref(), Some("etag:123"));
    }

    #[test]
    fn adapters_fail_closed_on_closed_residual_or_wrong_route_kind() {
        let receipt = OalcLookupReceipt {
            corpus_revision_ref: "oalc:rev".into(),
            citation: "[1992] HCA 23".into(),
            source_identity_ref: "case:[1992]-HCA-23".into(),
            source_revision_ref: "oalc:source:rev".into(),
            canonical_text_digest: "abc".into(),
            local_artifact_ref: "oalc://mabo".into(),
            network_requests: 0,
            receipt_authority: "experimental_candidate_only",
        };
        assert_eq!(
            from_oalc_lookup(
                &residual(ResidualStatus::SatisfiedCandidate),
                ResidualClass::Legal,
                "Q1501525",
                &receipt,
                scoring(),
            ),
            Err(ExpansionAdapterError::ResidualNotOpen)
        );

        let wrong = route(
            ProducerFamily::ArticleSemantic,
            RouteFamily::WikipediaArticle,
            "Q1501525",
            "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)",
            "",
        );
        assert_eq!(
            from_wikidata_route(
                &residual(ResidualStatus::Open),
                ResidualClass::Identity,
                "wikidata:Q1501525:oldid:2333409615",
                &wrong,
                scoring(),
            ),
            Err(ExpansionAdapterError::WrongWikidataRoute)
        );
    }

    #[test]
    fn promoted_or_non_candidate_wikipedia_source_is_rejected() {
        let mut source = AcquiredSource {
            kind: AcquiredSourceKind::WikipediaRenderedHtml,
            document_ref: "wikipedia:Q1501525:en:rev".into(),
            source_ref: "Q1501525".into(),
            language: "en".into(),
            revision_ref: "rev".into(),
            canonical_url: "https://en.wikipedia.org/wiki/Mabo_v_Queensland_(No_2)".into(),
            source_sha256: [9; 32],
            text: "text".into(),
            candidate_only: false,
            semantic_promotion: false,
        };
        assert_eq!(
            from_wikipedia_source(
                &residual(ResidualStatus::Open),
                ResidualClass::Context,
                &source,
                scoring(),
            ),
            Err(ExpansionAdapterError::WikipediaSourceNotCandidateOnly)
        );
        source.candidate_only = true;
        source.semantic_promotion = true;
        assert_eq!(
            from_wikipedia_source(
                &residual(ResidualStatus::Open),
                ResidualClass::Context,
                &source,
                scoring(),
            ),
            Err(ExpansionAdapterError::WikipediaSourcePromoted)
        );
    }
}
