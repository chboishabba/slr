module DASHI.Education.DigitalESDAdaptiveScreeningExecutionRegression where

open import Agda.Builtin.Bool using (false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDAdaptiveScreeningExecutionExact as Exec

recordCountPinned :
  Exec.realERICMetadataRecordCount
    Exec.canonicalAdaptiveScreeningExecutionBoundary
  ≡ Exec.expectedERICMetadataRecordCount
recordCountPinned = refl

ledgerBeginsUnresolved :
  Exec.authoritativeLedgerBeginsUnresolved
    Exec.canonicalAdaptiveScreeningExecutionBoundary
  ≡ true
ledgerBeginsUnresolved = refl

paretoIsNotScalarScreening :
  Exec.paretoUsesScalarScreeningScore
    Exec.canonicalAdaptiveScreeningExecutionBoundary
  ≡ false
paretoIsNotScalarScreening = refl

metadataIsNotFullText :
  Exec.allMetadataCountsAsVerifiedFullText
    Exec.canonicalAdaptiveScreeningExecutionBoundary
  ≡ false
metadataIsNotFullText = refl

candidateCannotDecide :
  Exec.CandidateAssessmentCreatesScreeningDecision → ⊥
candidateCannotDecide =
  Exec.candidateAssessmentDoesNotCreateScreeningDecision

paretoCannotExclude :
  Exec.ParetoPriorityCreatesExclusion → ⊥
paretoCannotExclude =
  Exec.paretoPriorityDoesNotCreateExclusion

missingArtifactCannotVerify :
  Exec.MissingArtifactCountsAsVerifiedFullText → ⊥
missingArtifactCannotVerify =
  Exec.missingArtifactDoesNotCountAsVerifiedFullText

fullTextCannotAdmit :
  Exec.FullTextVerificationCreatesSourceAuditAdmission → ⊥
fullTextCannotAdmit =
  Exec.fullTextVerificationDoesNotCreateSourceAuditAdmission
