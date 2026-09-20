//! OALC judgment -> source-span/citation-candidate materialisation.
//!
//! This adapter is deliberately domain-neutral.  It turns one already pinned,
//! immutable OALC decision receipt plus its canonical text into source-located
//! paragraph observations and the existing judgment citation candidates.  It
//! does not infer holdings, estoppel elements, citation treatment, current
//! authority, applicability, or proof payment.

use sensiblaw_governed_legal_provider::{
    CitationTreatmentIntent, KnownAuthorityDemand, OalcResolvedSourceReceipt,
    PropositionUseIntent,
};
use sha2::{Digest, Sha256};

use crate::judgment_candidates::{
    extract_judgment_citation_candidates, CitationOccurrenceCandidate,
};
use crate::residual_review_shortlist::{
    shortlist_anchored_citations_for_residual, ResidualAnchorCriterion,
    ResidualCitationReviewDemand, ResidualReviewShortlistError, ResidualShortlistedCitation,
};
use crate::review_units::{cluster_shortlisted_citations, CitationReviewUnit};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgmentParagraphCandidate {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub paragraph_ordinal: u64,
    pub paragraph_locator_ref: String,
    pub reported_paragraph_label: Option<String>,
    pub paragraph_text: String,
    pub matched_research_criterion_refs: Vec<String>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParagraphResearchCriterion {
    pub criterion_ref: String,
    pub needles: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OalcJudgmentMaterialisation {
    pub document_ref: String,
    pub source_revision_ref: String,
    pub canonical_text_sha256: String,
    pub paragraph_candidates: Vec<JudgmentParagraphCandidate>,
    pub citation_candidates: Vec<CitationOccurrenceCandidate>,
    pub candidate_only: bool,
    pub creates_legal_authority: bool,
    pub creates_claim_truth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OalcJudgmentMaterialisationError {
    NonCandidateReceipt,
    AuthorityPromotedReceipt,
    ClaimTruthPromotedReceipt,
    WrongDocumentType,
    EmptyVersion,
    EmptyText,
    DigestMismatch,
}

fn reported_paragraph_label(paragraph: &str) -> Option<String> {
    let trimmed = paragraph.trim_start();
    let rest = trimmed.strip_prefix('[')?;
    let end = rest.find(']')?;
    let digits = &rest[..end];
    (!digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit()))
        .then(|| format!("[{digits}]"))
}

fn sha256(text: &str) -> String {
    format!("sha256:{:x}", Sha256::digest(text.as_bytes()))
}

fn research_matches(
    paragraph: &str,
    criteria: &[ParagraphResearchCriterion],
) -> Vec<String> {
    let lower = paragraph.to_ascii_lowercase();
    let mut matched = criteria
        .iter()
        .filter(|criterion| {
            criterion.needles.iter().any(|needle| {
                let needle = needle.trim().to_ascii_lowercase();
                !needle.is_empty() && lower.contains(&needle)
            })
        })
        .map(|criterion| criterion.criterion_ref.clone())
        .collect::<Vec<_>>();
    matched.sort();
    matched.dedup();
    matched
}

pub fn materialize_oalc_judgment(
    receipt: &OalcResolvedSourceReceipt,
    canonical_text: &str,
    criteria: &[ParagraphResearchCriterion],
) -> Result<OalcJudgmentMaterialisation, OalcJudgmentMaterialisationError> {
    if !receipt.candidate_only {
        return Err(OalcJudgmentMaterialisationError::NonCandidateReceipt);
    }
    if receipt.creates_legal_authority {
        return Err(OalcJudgmentMaterialisationError::AuthorityPromotedReceipt);
    }
    if receipt.creates_claim_truth {
        return Err(OalcJudgmentMaterialisationError::ClaimTruthPromotedReceipt);
    }
    if receipt.document_type != "decision" {
        return Err(OalcJudgmentMaterialisationError::WrongDocumentType);
    }
    if receipt.version_id.trim().is_empty() {
        return Err(OalcJudgmentMaterialisationError::EmptyVersion);
    }
    if canonical_text.trim().is_empty() {
        return Err(OalcJudgmentMaterialisationError::EmptyText);
    }

    let digest = sha256(canonical_text);
    if digest != receipt.canonical_text_digest {
        return Err(OalcJudgmentMaterialisationError::DigestMismatch);
    }

    let document_ref = format!("document:oalc:{}", receipt.version_id);
    let source_revision_ref = format!(
        "{}:{}",
        receipt.corpus_revision_ref, receipt.version_id
    );

    let mut paragraph_candidates = Vec::new();
    for (index, paragraph) in canonical_text.lines().enumerate() {
        let paragraph = paragraph.trim();
        if paragraph.is_empty() {
            continue;
        }
        let ordinal = index as u64 + 1;
        let locator = format!("{document_ref}#paragraph-{ordinal}");
        paragraph_candidates.push(JudgmentParagraphCandidate {
            document_ref: document_ref.clone(),
            source_revision_ref: source_revision_ref.clone(),
            canonical_text_sha256: digest.clone(),
            paragraph_ordinal: ordinal,
            paragraph_locator_ref: locator,
            reported_paragraph_label: reported_paragraph_label(paragraph),
            paragraph_text: paragraph.to_string(),
            matched_research_criterion_refs: research_matches(paragraph, criteria),
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        });
    }

    let citation_candidates = extract_judgment_citation_candidates(
        &document_ref,
        &source_revision_ref,
        &digest,
        canonical_text,
    );

    Ok(OalcJudgmentMaterialisation {
        document_ref,
        source_revision_ref,
        canonical_text_sha256: digest,
        paragraph_candidates,
        citation_candidates,
        candidate_only: true,
        creates_legal_authority: false,
        creates_claim_truth: false,
    })
}

pub fn shortlist_materialized_citations_for_requirement(
    materialization: &OalcJudgmentMaterialisation,
    residual_ref: &str,
    proposition_ref: &str,
    criterion_ref: &str,
    required_anchor_phrases: Vec<String>,
) -> Result<Vec<ResidualShortlistedCitation>, ResidualReviewShortlistError> {
    shortlist_anchored_citations_for_residual(
        &materialization.citation_candidates,
        &ResidualCitationReviewDemand {
            residual_ref: residual_ref.to_string(),
            proposition_ref: proposition_ref.to_string(),
            criteria: vec![ResidualAnchorCriterion {
                criterion_ref: criterion_ref.to_string(),
                residual_ref: residual_ref.to_string(),
                proposition_ref: proposition_ref.to_string(),
                required_anchor_phrases,
            }],
        },
    )
}

pub fn review_units_for_materialized_requirement(
    materialization: &OalcJudgmentMaterialisation,
    residual_ref: &str,
    proposition_ref: &str,
    criterion_ref: &str,
    required_anchor_phrases: Vec<String>,
) -> Result<Vec<CitationReviewUnit>, ResidualReviewShortlistError> {
    let shortlisted = shortlist_materialized_citations_for_requirement(
        materialization,
        residual_ref,
        proposition_ref,
        criterion_ref,
        required_anchor_phrases,
    )?;
    Ok(cluster_shortlisted_citations(&shortlisted))
}

pub fn exact_mnc_candidate_follow_demand(
    candidate: &CitationOccurrenceCandidate,
    jurisdiction_ref: &str,
) -> Option<KnownAuthorityDemand> {
    let citation = candidate.citation_text.trim();
    if !(citation.starts_with('[')
        && citation.contains("] ")
        && citation.split_whitespace().count() == 3)
    {
        return None;
    }
    Some(KnownAuthorityDemand {
        demand_ref: format!(
            "citation-follow:{}:{}",
            candidate.source_revision_ref, candidate.paragraph_locator_ref
        ),
        jurisdiction_ref: jurisdiction_ref.to_string(),
        source_identity_ref: format!("authority:{citation}"),
        medium_neutral_citation: Some(citation.to_string()),
        explicit_austlii_ref: None,
        proposition_ref: None,
        use_intent: PropositionUseIntent::CitationTreatment,
        treatment_intent: CitationTreatmentIntent::None,
    })
}

pub fn paragraph_candidate_is_legal_proposition(
    _candidate: &JudgmentParagraphCandidate,
) -> bool {
    false
}

pub fn research_match_is_estoppel_element_payment(
    _candidate: &JudgmentParagraphCandidate,
) -> bool {
    false
}

pub fn citation_follow_demand_classifies_treatment(
    _demand: &KnownAuthorityDemand,
) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use sensiblaw_governed_legal_provider::{
        OalcTemporalCoverage, OALC_RECEIPT_AUTHORITY,
    };
    use std::path::PathBuf;

    fn receipt(text: &str) -> OalcResolvedSourceReceipt {
        OalcResolvedSourceReceipt {
            demand_ref: "contract-follow:waltons:hca7".into(),
            origin_ref: "doctrine:au:contract:estoppel".into(),
            citation: "[1988] HCA 7".into(),
            version_id: "high_court_of_australia:1988-hca-7".into(),
            corpus_revision_ref: "isaacus/open-australian-legal-corpus@deadbeef".into(),
            source: "high_court_of_australia".into(),
            jurisdiction: "commonwealth".into(),
            document_type: "decision".into(),
            court: Some("court:HCA".into()),
            date: Some("1988-02-19".into()),
            canonical_url: None,
            when_scraped: None,
            canonical_text_digest: sha256(text),
            local_artifact_ref: PathBuf::from("waltons.txt"),
            temporal_coverage: OalcTemporalCoverage::DecisionDateAnchored,
            resolution_path: "fixture".into(),
            network_requests: 0,
            receipt_authority: OALC_RECEIPT_AUTHORITY,
            candidate_only: true,
            creates_legal_authority: false,
            creates_claim_truth: false,
        }
    }

    fn estoppel_criteria() -> Vec<ParagraphResearchCriterion> {
        vec![
            ParagraphResearchCriterion {
                criterion_ref: "requirement:estoppel:assumption".into(),
                needles: vec!["assumption".into(), "expectation".into()],
            },
            ParagraphResearchCriterion {
                criterion_ref: "requirement:estoppel:reliance".into(),
                needles: vec!["reliance".into(), "relied".into()],
            },
            ParagraphResearchCriterion {
                criterion_ref: "requirement:estoppel:detriment".into(),
                needles: vec!["detriment".into()],
            },
            ParagraphResearchCriterion {
                criterion_ref: "requirement:estoppel:unconscionability".into(),
                needles: vec!["unconscionable".into(), "unconscionability".into()],
            },
        ]
    }

    #[test]
    fn pinned_oalc_text_becomes_source_located_paragraphs_without_proposition_promotion() {
        let text = "[1] The assumed expectation was relevant to the controversy.\n[2] Reliance and detriment were disputed.\n";
        let result = materialize_oalc_judgment(&receipt(text), text, &estoppel_criteria()).unwrap();
        assert_eq!(result.paragraph_candidates.len(), 2);
        assert_eq!(
            result.paragraph_candidates[0].reported_paragraph_label.as_deref(),
            Some("[1]")
        );
        assert!(result.paragraph_candidates[0]
            .matched_research_criterion_refs
            .contains(&"requirement:estoppel:assumption".to_string()));
        assert!(result.paragraph_candidates[1]
            .matched_research_criterion_refs
            .contains(&"requirement:estoppel:detriment".to_string()));
        assert!(!paragraph_candidate_is_legal_proposition(
            &result.paragraph_candidates[0]
        ));
        assert!(!research_match_is_estoppel_element_payment(
            &result.paragraph_candidates[0]
        ));
    }

    #[test]
    fn materialized_citations_reuse_existing_residual_shortlist_and_review_units() {
        let text = "[55] In considering reliance and detriment the reasons referred to Sidhu v Van Dyke [2014] HCA 19.\n";
        let result = materialize_oalc_judgment(&receipt(text), text, &estoppel_criteria()).unwrap();
        let shortlisted = shortlist_materialized_citations_for_requirement(
            &result,
            "residual:estoppel:reliance",
            "prop:estoppel:reliance",
            "criterion:estoppel:reliance-and-detriment",
            vec!["reliance".into(), "detriment".into()],
        )
        .unwrap();
        assert_eq!(shortlisted.len(), 1);
        let units = review_units_for_materialized_requirement(
            &result,
            "residual:estoppel:reliance",
            "prop:estoppel:reliance",
            "criterion:estoppel:reliance-and-detriment",
            vec!["reliance".into(), "detriment".into()],
        )
        .unwrap();
        assert_eq!(units.len(), 1);
        assert!(units[0].candidate_only);
        assert!(!crate::review_units::review_unit_is_citation_treatment(&units[0]));
        assert!(!crate::review_units::review_unit_is_semantic_payment(&units[0]));
    }

    #[test]
    fn citation_candidates_feed_exact_authority_follow_without_auto_treatment() {
        let text = "[55] Later discussion referred to Sidhu v Van Dyke [2014] HCA 19.\n";
        let result = materialize_oalc_judgment(&receipt(text), text, &[]).unwrap();
        let candidate = result.citation_candidates.first().unwrap();
        let demand = exact_mnc_candidate_follow_demand(candidate, "AU").unwrap();
        assert_eq!(demand.medium_neutral_citation.as_deref(), Some("[2014] HCA 19"));
        assert_eq!(demand.use_intent, PropositionUseIntent::CitationTreatment);
        assert_eq!(demand.treatment_intent, CitationTreatmentIntent::None);
        assert!(!citation_follow_demand_classifies_treatment(&demand));
    }

    #[test]
    fn text_digest_must_match_pinned_oalc_receipt() {
        let result = materialize_oalc_judgment(
            &receipt("[1] original\n"),
            "[1] changed\n",
            &[],
        );
        assert_eq!(result, Err(OalcJudgmentMaterialisationError::DigestMismatch));
    }
}
