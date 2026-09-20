module DASHI.Interop.DigitalESD.ScholarlyFullTextCrossPollinationExact where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Agda.Builtin.String using (String)
open import Data.Empty using (⊥)

------------------------------------------------------------------------
-- DIGITAL-ESD SCHOLARLY FULL-TEXT CROSS-POLLINATION — EXACT
--
-- Schema only.  No observed runtime witness is manufactured here.
--
-- This module formally names the cross-pollination boundary where:
--
--   retained full-text artifact
--        ↓
--   ScholarlyFullTextCanonicalCarrier
--        ↓
--   EvidenceManifestation + EvidenceSourceRevision
--        ↓
--   ScholarlyDocumentStructureNode + exact EvidenceSpan
--        ↓
--   ScholarlyStudyFacetCandidate
--        ↓
--   canonical EvidenceObservation
--        ↓
--   ReviewedScholarlyStudyObservation
--        ↓
--   DigitalESDAuditProjectionReceipt
--        ↓
--   SituatedAuditObservation
--        ↓
--   independent SourceAuditAdmission
--
-- Candidate roles are generic scholarly concepts:
--   Study, Population, Sample, Intervention, Comparator, Outcome,
--   StudyDesign, Setting, TimePeriod, Method, Limitation, Funding,
--   Institution, ParticipantGroup, Measurement
--
-- Firewalls:
--   ERIC metadata != parsed study
--   full-text artifact != semantic observation
--   document structure != claim truth
--   document structure != study truth
--   study facet candidate != study truth
--   study facet candidate != screening decision
--   reviewed observation != claim truth
--   reviewed observation != SourceAuditAdmission
--   Digital-ESD projection != replacement canonical evidence
--   Digital-ESD projection != SourceAuditAdmission
--   parser success != review/payment
--   parser/model != source authority
--   StructuredCoordinate does not need a fake TextRange
------------------------------------------------------------------------

expectedRetainedFullTextArtifacts : Nat
expectedRetainedFullTextArtifacts = 0

data ScholarlyDocumentStructureNode : Set where
  heading : ScholarlyDocumentStructureNode
  paragraph : ScholarlyDocumentStructureNode
  table_cell : ScholarlyDocumentStructureNode
  list_item : ScholarlyDocumentStructureNode
  numbered_item : ScholarlyDocumentStructureNode

data StructuredCoordinate : Set where
  text_range : String → StructuredCoordinate
  structured_coordinate : String → StructuredCoordinate
  whole_revision : String → StructuredCoordinate

data CandidateStudyFacetRole : Set where
  study : CandidateStudyFacetRole
  population : CandidateStudyFacetRole
  sample : CandidateStudyFacetRole
  intervention : CandidateStudyFacetRole
  comparator : CandidateStudyFacetRole
  outcome : CandidateStudyFacetRole
  study_design : CandidateStudyFacetRole
  setting : CandidateStudyFacetRole
  time_period : CandidateStudyFacetRole
  method : CandidateStudyFacetRole
  limitation : CandidateStudyFacetRole
  funding : CandidateStudyFacetRole
  institution : CandidateStudyFacetRole
  participant_group : CandidateStudyFacetRole
  measurement : CandidateStudyFacetRole

record EvidenceObservation : Set where
  constructor evidence-observation
  field
    predicate : String
    value_ref : String
    document_node_anchor : String
    candidate_only : Bool
    candidate_onlyIsTrue : candidate_only ≡ true
    creates_study_truth : Bool
    creates_study_truthIsFalse : creates_study_truth ≡ false
    creates_source_audit_admission : Bool
    creates_source_audit_admissionIsFalse :
      creates_source_audit_admission ≡ false

record ScholarlyFullTextCanonicalCarrier : Set where
  constructor scholarly-full-text-canonical-carrier
  field
    full_text_cache_receipt : String
    evidence_manifestation : String
    evidence_source_revision : String
    manifestation_revision_matches_retained_artifact : Bool
    manifestation_revision_matches_retained_artifactIsTrue :
      manifestation_revision_matches_retained_artifact ≡ true
    manifestation_digest_matches_retained_sha256 : Bool
    manifestation_digest_matches_retained_sha256IsTrue :
      manifestation_digest_matches_retained_sha256 ≡ true
    canonical_revision_matches_manifestation_revision : Bool
    canonical_revision_matches_manifestation_revisionIsTrue :
      canonical_revision_matches_manifestation_revision ≡ true
    canonical_revision_manifestation_matches_manifestation_identity : Bool
    canonical_revision_manifestation_matches_manifestation_identityIsTrue :
      canonical_revision_manifestation_matches_manifestation_identity ≡ true
    canonical_revision_digest_matches_manifestation_digest : Bool
    canonical_revision_digest_matches_manifestation_digestIsTrue :
      canonical_revision_digest_matches_manifestation_digest ≡ true

record ScholarlyStudyFacetCandidate : Set where
  constructor scholarly-study-facet-candidate
  field
    document_node : ScholarlyDocumentStructureNode
    structured_coordinate : StructuredCoordinate
    facet_role : CandidateStudyFacetRole
    evidence_observation : EvidenceObservation
    candidate_only : Bool
    candidate_onlyIsTrue : candidate_only ≡ true
    creates_study_truth : Bool
    creates_study_truthIsFalse : creates_study_truth ≡ false
    creates_source_audit_admission : Bool
    creates_source_audit_admissionIsFalse :
      creates_source_audit_admission ≡ false

record DigitalESDAuditProjectionReceipt : Set where
  constructor digital-esd-audit-projection-receipt
  field
    projection_type : String
    canonical_evidence_substrate : String
    projection_does_not_replace_canonical_evidence : Bool
    projection_does_not_replace_canonical_evidenceIsTrue :
      projection_does_not_replace_canonical_evidence ≡ true
    projection_does_not_create_source_audit_admission : Bool
    projection_does_not_create_source_audit_admissionIsFalse :
      projection_does_not_create_source_audit_admission ≡ false

record ScholarlyFullTextCrossPollinationReceipt : Set where
  constructor scholarly-full-text-cross-pollination-receipt
  field
    schema : String
    prepared_request_count : Nat
    verified_bundle_count : Nat
    missing_request_count : Nat
    document_node_count : Nat
    study_facet_count : Nat
    source_identity_reconciled : Bool
    revision_reconciled : Bool
    digest_reconciled : Bool
    anchors_reconciled : Bool
    facet_observations_reconciled : Bool
    candidate_only_verified : Bool
    candidate_onlyVerifiedIsTrue : candidate_only_verified ≡ true
    partial_parse_explicit : Bool
    parser_success_not_review_payment : Bool
    parser_success_not_review_paymentIsTrue :
      parser_success_not_review_payment ≡ true
    verified_bundle_not_source_truth : Bool
    verified_bundle_not_source_truthIsTrue :
      verified_bundle_not_source_truth ≡ true
    verified_bundle_not_source_audit_admission : Bool
    verified_bundle_not_source_audit_admissionIsTrue :
      verified_bundle_not_source_audit_admission ≡ false
    canonical_carrier : ScholarlyFullTextCanonicalCarrier

defaultScholarlyFullTextCrossPollinationReceipt :
  ScholarlyFullTextCrossPollinationReceipt
defaultScholarlyFullTextCrossPollinationReceipt =
  scholarly-full-text-cross-pollination-receipt
    "sensiblaw.scholarly-fulltext-cross-pollination.v0_1"
    0 0 0 0 0 true true true true true true true true true
    true true true true
    canonical-carrier
  where
    canonical-carrier : ScholarlyFullTextCanonicalCarrier
    canonical-carrier =
      scholarly-full-text-canonical-carrier
        "" "" "" refl refl refl refl refl refl refl refl refl refl
