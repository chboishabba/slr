module DASHI.Education.DigitalESDERICStudyExecutionRegression where

open import Agda.Builtin.Bool using (false)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDERICStudyExecutionExact as Exec

rawCountPinned :
  Exec.expectedOccurrenceCount Exec.canonicalERICExecutionBoundary
  ≡ 46597
rawCountPinned = refl

uniqueCountPinned :
  Exec.expectedUniqueCount Exec.canonicalERICExecutionBoundary
  ≡ 43996
uniqueCountPinned = refl

metadataDoesNotCountAsFullText :
  Exec.metadataParsingCountsAsFullTextParsing
    Exec.canonicalERICExecutionBoundary
  ≡ false
metadataDoesNotCountAsFullText = refl

queryOverlapDoesNotCreateStudy :
  Exec.QueryOverlapCreatesSameStudy → ⊥
queryOverlapDoesNotCreateStudy =
  Exec.queryOverlapDoesNotCreateSameStudy

candidateAssessmentDoesNotDecide :
  Exec.CandidateAssessmentCreatesReviewedDecision → ⊥
candidateAssessmentDoesNotDecide =
  Exec.candidateAssessmentDoesNotCreateReviewedDecision

paretoDoesNotDecide :
  Exec.ParetoQueueCreatesReviewedDecision → ⊥
paretoDoesNotDecide =
  Exec.paretoQueueDoesNotCreateReviewedDecision

wrapperCannotAdmit :
  Exec.WrapperSuccessCreatesSourceAuditAdmission → ⊥
wrapperCannotAdmit =
  Exec.wrapperSuccessDoesNotCreateSourceAuditAdmission
