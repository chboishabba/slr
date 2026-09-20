module DASHI.Everything where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDAdaptiveScreeningExecutionExact as AdaptiveScreening
import DASHI.Education.DigitalESDAdaptiveScreeningExecutionRegression as AdaptiveScreeningRegression
import DASHI.Education.DigitalESDERICStudyExecutionExact as ERICStudy
import DASHI.Education.DigitalESDERICStudyExecutionRegression as ERICStudyRegression
import DASHI.Education.DigitalESDSelectiveFullTextMaterialisationExact as FullText
import DASHI.Education.DigitalESDSelectiveFullTextMaterialisationRegression as FullTextRegression
import DASHI.EverythingDigitalESDReciprocalBraid as Braid

------------------------------------------------------------------------
-- DASHI EVERYTHING
--
-- Aggregate module naming all Digital-ESD formal owners.
-- No observed execution witness is imported here.
------------------------------------------------------------------------

canonicalERICStudyExecution : ERICStudy.RealERICStudyExecutionReceipt
canonicalERICStudyExecution = ERICStudy.defaultRealERICStudyExecutionReceipt

canonicalAdaptiveScreening : AdaptiveScreening.AdaptiveScreeningExecutionBoundary
canonicalAdaptiveScreening = AdaptiveScreening.canonicalAdaptiveScreeningExecutionBoundary

canonicalSelectiveFullText : FullText.SelectiveFullTextMaterialisationReceipt
canonicalSelectiveFullText = FullText.defaultSelectiveFullTextMaterialisationReceipt

canonicalBraid : Braid.DigitalESDReciprocalBraidBoundary
canonicalBraid = Braid.canonicalDigitalESDReciprocalBraidBoundary
