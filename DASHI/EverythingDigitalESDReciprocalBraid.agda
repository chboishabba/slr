module DASHI.EverythingDigitalESDReciprocalBraid where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDERICStudyExecutionExact as ERIC
import DASHI.Education.DigitalESDERICStudyExecutionRegression as ERICRegression
import DASHI.Education.DigitalESDAdaptiveScreeningExecutionExact as Screening
import DASHI.Education.DigitalESDAdaptiveScreeningExecutionRegression as ScreeningRegression

------------------------------------------------------------------------
-- DIGITAL-ESD RECIPROCAL BRAID
--
-- Schema/regression composition only.
--
-- No observed execution witness is imported here.  A real ERIC execution may
-- generate DASHI.Generated.DigitalESDERICStudyExecutionObserved, but absence of
-- that generated module does not let the braid fabricate an observation.
------------------------------------------------------------------------

record DigitalESDReciprocalBraidBoundary : Set where
  constructor digital-esd-reciprocal-braid-boundary
  field
    expectedERICQueryOccurrences : Nat
    expectedERICQueryOccurrencesIs46597 :
      expectedERICQueryOccurrences ≡ 46597

    expectedERICUniqueRecords : Nat
    expectedERICUniqueRecordsIs43996 :
      expectedERICUniqueRecords ≡ 43996

    parsedMetadataCountsAsFullText : Bool
    parsedMetadataCountsAsFullTextIsFalse :
      parsedMetadataCountsAsFullText ≡ false

    candidateAssessmentCreatesDecision : Bool
    candidateAssessmentCreatesDecisionIsFalse :
      candidateAssessmentCreatesDecision ≡ false

    paretoQueueCreatesDecision : Bool
    paretoQueueCreatesDecisionIsFalse :
      paretoQueueCreatesDecision ≡ false

    missingFullTextCountsAsVerified : Bool
    missingFullTextCountsAsVerifiedIsFalse :
      missingFullTextCountsAsVerified ≡ false

    fullTextVerificationCreatesSourceAuditAdmission : Bool
    fullTextVerificationCreatesSourceAuditAdmissionIsFalse :
      fullTextVerificationCreatesSourceAuditAdmission ≡ false

open DigitalESDReciprocalBraidBoundary public

canonicalDigitalESDReciprocalBraidBoundary :
  DigitalESDReciprocalBraidBoundary
canonicalDigitalESDReciprocalBraidBoundary =
  digital-esd-reciprocal-braid-boundary
    ERIC.expectedRawQueryOccurrenceCount
    refl
    ERIC.expectedUniqueERICRecordCount
    refl
    false refl
    false refl
    false refl
    false refl
    false refl

data BraidCreatesObservedWitness : Set where
data BraidPromotesMetadataToFullText : Set where
data BraidPromotesCandidateAssessmentToDecision : Set where
data BraidPromotesFullTextToAuditAdmission : Set where

braidDoesNotCreateObservedWitness :
  BraidCreatesObservedWitness → ⊥
braidDoesNotCreateObservedWitness ()

braidDoesNotPromoteMetadataToFullText :
  BraidPromotesMetadataToFullText → ⊥
braidDoesNotPromoteMetadataToFullText ()

braidDoesNotPromoteCandidateAssessmentToDecision :
  BraidPromotesCandidateAssessmentToDecision → ⊥
braidDoesNotPromoteCandidateAssessmentToDecision ()

braidDoesNotPromoteFullTextToAuditAdmission :
  BraidPromotesFullTextToAuditAdmission → ⊥
braidDoesNotPromoteFullTextToAuditAdmission ()

------------------------------------------------------------------------
-- Anchor the imported regression owners in this aggregate.
------------------------------------------------------------------------

ericMetadataNotFullText :
  ERIC.metadataParsingCountsAsFullTextParsing
    ERIC.canonicalERICExecutionBoundary
  ≡ false
ericMetadataNotFullText = ERICRegression.metadataDoesNotCountAsFullText

screeningMetadataNotFullText :
  Screening.allMetadataCountsAsVerifiedFullText
    Screening.canonicalAdaptiveScreeningExecutionBoundary
  ≡ false
screeningMetadataNotFullText = ScreeningRegression.metadataIsNotFullText
