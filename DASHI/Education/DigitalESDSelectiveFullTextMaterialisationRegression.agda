module DASHI.Education.DigitalESDSelectiveFullTextMaterialisationRegression where

open import Agda.Builtin.Bool using (false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Data.Empty using (⊥)

import DASHI.Education.DigitalESDSelectiveFullTextMaterialisationExact as Exact

------------------------------------------------------------------------
-- DIGITAL-ESD SELECTIVE FULL-TEXT MATERIALISATION — REGRESSION
--
-- Regression invariants for the selective full-text materialisation
-- boundary.  Each theorem states that a tempting shortcut is false.
------------------------------------------------------------------------

metadataRowsAreNotFullTextFiles :
  Exact.metadataRecordCount
    Exact.canonicalFullTextCacheBoundary
  ≡ 43997
metadataRowsAreNotFullTextFiles = ⊥-elim (Exact.metadataRowsNotFullTextFiles refl)

unreviewedRecordNotEligible :
  Exact.unreviewedNotEligible
    Exact.canonicalFullTextCacheBoundary
  ≡ true
unreviewedRecordNotEligible = refl

includeProbableNotAutomaticDownload :
  Exact.includeProbableNotAutomatic
    Exact.canonicalFullTextCacheBoundary
  ≡ true
includeProbableNotAutomaticDownload = refl

cachedArtifactNotAuditAdmission :
  Exact.cachedArtifactNotAuditAdmission
    Exact.canonicalFullTextCacheBoundary
  ≡ true
cachedArtifactNotAuditAdmission = refl

unprocessedNotSafelyEvictable :
  Exact.unprocessedNotSafelyEvictable
    Exact.canonicalFullTextCacheBoundary
  ≡ true
unprocessedNotSafelyEvictable = refl

planRespectsCacheCap :
  Exact.planRespectsCacheCap
    Exact.canonicalFullTextCacheBoundary
  ≡ true
planRespectsCacheCap = refl

planRespectsReserve :
  Exact.planRespectsReserve
    Exact.canonicalFullTextCacheBoundary
  ≡ true
planRespectsReserve = refl

registerRejectsUnplaned :
  Exact.registerRejectsUnplaned
    Exact.canonicalFullTextCacheBoundary
  ≡ true
registerRejectsUnplaned = refl

handoffOnlyRegistered :
  Exact.handoffOnlyRegistered
    Exact.canonicalFullTextCacheBoundary
  ≡ true
handoffOnlyRegistered = refl

gcPlanNotDeletion :
  Exact.gcPlanNotDeletion
    Exact.canonicalFullTextCacheBoundary
  ≡ true
gcPlanNotDeletion = refl

gcPlanPreservesRevisionDigest :
  Exact.gcPlanPreservesRevisionDigest
    Exact.canonicalFullTextCacheBoundary
  ≡ true
gcPlanPreservesRevisionDigest = refl

maxBatchIs20 :
  Exact.maxBatchSize
    Exact.canonicalFullTextCacheBoundary
  ≡ 20
maxBatchIs20 = refl

cacheCapIs2GiB :
  Exact.defaultCacheCapGib
    Exact.canonicalFullTextCacheBoundary
  ≡ 2
cacheCapIs2GiB = refl

reserveIs5GiB :
  Exact.defaultReserveGib
    Exact.canonicalFullTextCacheBoundary
  ≡ 5
reserveIs5GiB = refl

planningSizeIs10MiB :
  Exact.planningSizeMb
    Exact.canonicalFullTextCacheBoundary
  ≡ 10
planningSizeIs10MiB = refl

metadataRowsNotFullTextFilesTheorem :
  Exact.metadataRecordCount
    Exact.canonicalFullTextCacheBoundary
  ≡ 43996
metadataRowsNotFullTextFilesTheorem = refl
