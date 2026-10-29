module DASHI.Education.DigitalESDAdaptiveScreeningExecutionExact where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Agda.Builtin.String using (String)
open import Data.Empty using (⊥)

------------------------------------------------------------------------
-- DIGITAL-ESD REAL ADAPTIVE SCREENING EXECUTION BOUNDARY
--
-- Schema only.  No observed runtime witness is manufactured here.
--
-- P0-A exact ERIC metadata universe / unresolved screening ledger
-- P0-B candidate-only title/abstract assessment
-- P0-C candidate duplicate/report-family hypotheses
-- P0-D stratified calibration selection
-- P0-E reviewed-subset calibration diagnostics
-- P0-F non-scalar Pareto reviewer work queue
-- P0-G authoritative include|probable -> real artifact SHA-256 gate
------------------------------------------------------------------------

expectedERICMetadataRecordCount : Nat
expectedERICMetadataRecordCount = 43996

data P0Stage : Set where
  p0A p0B p0C p0D p0E p0F p0G : P0Stage

data RuntimePaymentState : Set where
  sourceWritten
  observed
  awaitingReviewedDecisions
  awaitingFullTextArtifacts
  : RuntimePaymentState

record AdaptiveScreeningExecutionBoundary : Set where
  constructor adaptive-screening-execution-boundary
  field
    realERICMetadataRecordCount : Nat
    realERICMetadataRecordCountIsExpected :
      realERICMetadataRecordCount ≡ expectedERICMetadataRecordCount

    authoritativeLedgerBeginsUnresolved : Bool
    authoritativeLedgerBeginsUnresolvedIsTrue :
      authoritativeLedgerBeginsUnresolved ≡ true

    candidateAssessmentIsAdvisory : Bool
    candidateAssessmentIsAdvisoryIsTrue :
      candidateAssessmentIsAdvisory ≡ true

    studyFamilyHypothesisIsAdvisory : Bool
    studyFamilyHypothesisIsAdvisoryIsTrue :
      studyFamilyHypothesisIsAdvisory ≡ true

    calibrationNeedsReviewedDecisions : Bool
    calibrationNeedsReviewedDecisionsIsTrue :
      calibrationNeedsReviewedDecisions ≡ true

    paretoUsesScalarScreeningScore : Bool
    paretoUsesScalarScreeningScoreIsFalse :
      paretoUsesScalarScreeningScore ≡ false

    paretoCreatesScreeningDecision : Bool
    paretoCreatesScreeningDecisionIsFalse :
      paretoCreatesScreeningDecision ≡ false

    fullTextRequiresRetainedDecision : Bool
    fullTextRequiresRetainedDecisionIsTrue :
      fullTextRequiresRetainedDecision ≡ true

    fullTextRequiresRealArtifactDigest : Bool
    fullTextRequiresRealArtifactDigestIsTrue :
      fullTextRequiresRealArtifactDigest ≡ true

    allMetadataCountsAsVerifiedFullText : Bool
    allMetadataCountsAsVerifiedFullTextIsFalse :
      allMetadataCountsAsVerifiedFullText ≡ false

    fullTextVerificationCreatesSourceTruth : Bool
    fullTextVerificationCreatesSourceTruthIsFalse :
      fullTextVerificationCreatesSourceTruth ≡ false

    fullTextVerificationCreatesSourceAuditAdmission : Bool
    fullTextVerificationCreatesSourceAuditAdmissionIsFalse :
      fullTextVerificationCreatesSourceAuditAdmission ≡ false

open AdaptiveScreeningExecutionBoundary public

canonicalAdaptiveScreeningExecutionBoundary :
  AdaptiveScreeningExecutionBoundary
canonicalAdaptiveScreeningExecutionBoundary =
  adaptive-screening-execution-boundary
    43996 refl
    true refl
    true refl
    true refl
    true refl
    false refl
    false refl
    true refl
    true refl
    false refl
    false refl
    false refl

------------------------------------------------------------------------
-- Runtime receipt schemas.
------------------------------------------------------------------------

record AdaptiveScreeningRuntimeReceipt : Set where
  constructor adaptive-screening-runtime-receipt
  field
    runReference : String
    metadataCorpusHash : String
    screeningLedgerHash : String
    candidateAssessmentHash : String
    studyFamilyHypothesisHash : String
    calibrationSelectionHash : String
    calibrationEstimateHash : String
    paretoQueueHash : String

    metadataRecordCount : Nat
    unresolvedLedgerCount : Nat
    reviewedCalibrationPairCount : Nat

    p0AState : RuntimePaymentState
    p0BState : RuntimePaymentState
    p0CState : RuntimePaymentState
    p0DState : RuntimePaymentState
    p0EState : RuntimePaymentState
    p0FState : RuntimePaymentState

open AdaptiveScreeningRuntimeReceipt public

record FullTextGateRuntimeReceipt : Set where
  constructor full-text-gate-runtime-receipt
  field
    gateReference : String
    screeningLedgerHash : String
    retrievalManifestHash : String
    fullTextIndexHash : String
    eligibleCount : Nat
    retrievedCount : Nat
    verifiedCount : Nat
    failedCount : Nat
    pendingCount : Nat

open FullTextGateRuntimeReceipt public

------------------------------------------------------------------------
-- Firewalls.
------------------------------------------------------------------------

data CandidateAssessmentCreatesScreeningDecision : Set where
data CandidateAssessmentCreatesExclusion : Set where
data StudyFamilyHypothesisCreatesStudyIdentity : Set where
data ParetoPriorityCreatesScreeningDecision : Set where
data ParetoPriorityCreatesExclusion : Set where
data MetadataRecordCountsAsFullText : Set where
data MissingArtifactCountsAsVerifiedFullText : Set where
data FullTextVerificationCreatesSourceTruth : Set where
data FullTextVerificationCreatesSourceAuditAdmission : Set where

candidateAssessmentDoesNotCreateScreeningDecision :
  CandidateAssessmentCreatesScreeningDecision → ⊥
candidateAssessmentDoesNotCreateScreeningDecision ()

candidateAssessmentDoesNotCreateExclusion :
  CandidateAssessmentCreatesExclusion → ⊥
candidateAssessmentDoesNotCreateExclusion ()

studyFamilyHypothesisDoesNotCreateStudyIdentity :
  StudyFamilyHypothesisCreatesStudyIdentity → ⊥
studyFamilyHypothesisDoesNotCreateStudyIdentity ()

paretoPriorityDoesNotCreateScreeningDecision :
  ParetoPriorityCreatesScreeningDecision → ⊥
paretoPriorityDoesNotCreateScreeningDecision ()

paretoPriorityDoesNotCreateExclusion :
  ParetoPriorityCreatesExclusion → ⊥
paretoPriorityDoesNotCreateExclusion ()

metadataRecordDoesNotCountAsFullText :
  MetadataRecordCountsAsFullText → ⊥
metadataRecordDoesNotCountAsFullText ()

missingArtifactDoesNotCountAsVerifiedFullText :
  MissingArtifactCountsAsVerifiedFullText → ⊥
missingArtifactDoesNotCountAsVerifiedFullText ()

fullTextVerificationDoesNotCreateSourceTruth :
  FullTextVerificationCreatesSourceTruth → ⊥
fullTextVerificationDoesNotCreateSourceTruth ()

fullTextVerificationDoesNotCreateSourceAuditAdmission :
  FullTextVerificationCreatesSourceAuditAdmission → ⊥
fullTextVerificationDoesNotCreateSourceAuditAdmission ()
