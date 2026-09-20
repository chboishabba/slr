module DASHI.Education.DigitalESDERICStudyExecutionExact where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Agda.Builtin.String using (String)
open import Data.Empty using (⊥)

expectedRawQueryOccurrenceCount : Nat
expectedRawQueryOccurrenceCount = 46597

expectedUniqueERICRecordCount : Nat
expectedUniqueERICRecordCount = 43996

record RealERICStudyExecutionReceipt : Set where
  constructor real-eric-study-execution-receipt
  field
    runId : String
    startedAt : String
    completedAt : String

    expectedRawQueryOccurrenceCount : Nat
    observedRawQueryOccurrenceCount : Nat
    rawCountMatchesExpected :
      observedRawQueryOccurrenceCount ≡ expectedRawQueryOccurrenceCount

    expectedUniqueERICRecordCount : Nat
    observedUniqueERICRecordCount : Nat
    uniqueCountMatchesExpected :
      observedUniqueERICRecordCount ≡ expectedUniqueERICRecordCount

    realERIC : Bool
    realERICIsTrue : realERIC ≡ true

    stopsBeforeFullTextAcquisition : Bool
    stopsBeforeFullTextAcquisitionIsTrue :
      stopsBeforeFullTextAcquisition ≡ true

    screeningLedgerAllUnresolved : Bool
    screeningLedgerAllUnresolvedIsTrue :
      screeningLedgerAllUnresolved ≡ true

    scalarScreeningScoreUsed : Bool
    scalarScreeningScoreUsedIsFalse :
      scalarScreeningScoreUsed ≡ false

    createsScreeningDecision : Bool
    createsScreeningDecisionIsFalse :
      createsScreeningDecision ≡ false

    createsSourceTruth : Bool
    createsSourceTruthIsFalse :
      createsSourceTruth ≡ false

    createsSourceAuditAdmission : Bool
    createsSourceAuditAdmissionIsFalse :
      createsSourceAuditAdmission ≡ false

    parsedMetadataCorpusPath : String
    parsedMetadataCorpusHash : String
    parserManifestPath : String
    parserManifestHash : String
    screeningLedgerPath : String
    screeningLedgerHash : String
    candidateAssessmentPath : String
    candidateAssessmentHash : String
    studyFamilyHypothesesPath : String
    studyFamilyHypothesesHash : String
    calibrationSelectionPath : String
    calibrationSelectionHash : String
    calibrationEstimatePath : String
    calibrationEstimateHash : String
    paretoQueuePath : String
    paretoQueueHash : String

open RealERICStudyExecutionReceipt public

record ERICExecutionBoundary : Set where
  constructor eric-execution-boundary
  field
    expectedOccurrenceCount : Nat
    expectedOccurrenceCountIs46597 :
      expectedOccurrenceCount ≡ 46597

    expectedUniqueCount : Nat
    expectedUniqueCountIs43996 :
      expectedUniqueCount ≡ 43996

    metadataParsingCountsAsFullTextParsing : Bool
    metadataParsingCountsAsFullTextParsingIsFalse :
      metadataParsingCountsAsFullTextParsing ≡ false

    queryOverlapCreatesStudyIdentity : Bool
    queryOverlapCreatesStudyIdentityIsFalse :
      queryOverlapCreatesStudyIdentity ≡ false

    candidateAssessmentCreatesReviewedDecision : Bool
    candidateAssessmentCreatesReviewedDecisionIsFalse :
      candidateAssessmentCreatesReviewedDecision ≡ false

    paretoQueueCreatesReviewedDecision : Bool
    paretoQueueCreatesReviewedDecisionIsFalse :
      paretoQueueCreatesReviewedDecision ≡ false

open ERICExecutionBoundary public

canonicalERICExecutionBoundary : ERICExecutionBoundary
canonicalERICExecutionBoundary =
  eric-execution-boundary
    46597 refl
    43996 refl
    false refl
    false refl
    false refl
    false refl

data ParsedMetadataCreatesFullText : Set where
data QueryOverlapCreatesSameStudy : Set where
data CandidateAssessmentCreatesReviewedDecision : Set where
data ParetoQueueCreatesReviewedDecision : Set where
data WrapperSuccessCreatesScreeningDecision : Set where
data WrapperSuccessCreatesSourceAuditAdmission : Set where

parsedMetadataDoesNotCreateFullText :
  ParsedMetadataCreatesFullText → ⊥
parsedMetadataDoesNotCreateFullText ()

queryOverlapDoesNotCreateSameStudy :
  QueryOverlapCreatesSameStudy → ⊥
queryOverlapDoesNotCreateSameStudy ()

candidateAssessmentDoesNotCreateReviewedDecision :
  CandidateAssessmentCreatesReviewedDecision → ⊥
candidateAssessmentDoesNotCreateReviewedDecision ()

paretoQueueDoesNotCreateReviewedDecision :
  ParetoQueueCreatesReviewedDecision → ⊥
paretoQueueDoesNotCreateReviewedDecision ()

wrapperSuccessDoesNotCreateScreeningDecision :
  WrapperSuccessCreatesScreeningDecision → ⊥
wrapperSuccessDoesNotCreateScreeningDecision ()

wrapperSuccessDoesNotCreateSourceAuditAdmission :
  WrapperSuccessCreatesSourceAuditAdmission → ⊥
wrapperSuccessDoesNotCreateSourceAuditAdmission ()
