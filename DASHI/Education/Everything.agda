module DASHI.Education.Everything where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDAdaptiveScreeningExecutionExact as AdaptiveScreeningExact
import DASHI.Education.DigitalESDAdaptiveScreeningExecutionRegression as AdaptiveScreeningRegression
import DASHI.Education.DigitalESDERICStudyExecutionExact as ERICStudyExact
import DASHI.Education.DigitalESDERICStudyExecutionRegression as ERICStudyRegression
import DASHI.Education.DigitalESDSelectiveFullTextMaterialisationExact as FullTextExact
import DASHI.Education.DigitalESDSelectiveFullTextMaterialisationRegression as FullTextRegression

------------------------------------------------------------------------
-- DIGITAL-ESD EDUCATION MODULE COMPOSITION
--
-- This module re-exports all Education Agda formal owners.
-- No observed execution witness is imported here.
------------------------------------------------------------------------

adaptiveScreeningExact : AdaptiveScreeningExact.AdaptiveScreeningExecutionBoundary
adaptiveScreeningExact = AdaptiveScreeningExact.canonicalAdaptiveScreeningExecutionBoundary

ericStudyExact : ERICStudyExact.RealERICStudyExecutionReceipt
ericStudyExact = ERICStudyExact.defaultRealERICStudyExecutionReceipt

fullTextExact : FullTextExact.SelectiveFullTextMaterialisationReceipt
fullTextExact = FullTextExact.defaultSelectiveFullTextMaterialisationReceipt
