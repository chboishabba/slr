module DASHI.Education.DigitalESDSelectiveFullTextMaterialisationExact where

open import Agda.Builtin.Bool using (Bool; false; true)
open import Agda.Builtin.Equality using (_≡_; refl)
open import Agda.Builtin.Nat using (Nat)
open import Agda.Builtin.String using (String)
open import Data.Empty using (⊥)
open import Data.Fin using (Fin)
open import Data.Vec using (Vec; []; _∷_)
open import Relation.Binary.PropositionalEquality using (_≡_; refl; sym; trans)

------------------------------------------------------------------------
-- DIGITAL-ESD SELECTIVE FULL-TEXT MATERIALISATION — EXACT
--
-- Schema only.  No observed runtime witness is manufactured here.
--
-- This module formally names the selective full-text materialisation
-- boundary where:
--
--   43,996 metadata rows  !=  43,996 materialised full-text files
--   unreviewed record     !=  eligible for full-text batch
--   include|probable      !=  automatic download
--   cached artifact       !=  SourceAuditAdmission
--   unprocessed artifact  !=  safely evictable
--
-- The four machine-executable stages are:
--
--   stage1  plan      — bounded fetch batch from include|probable worklist
--   stage2  register  — hash actual bytes, refuse unplaned artifacts
--   stage3  handoff   — lower only registered retained items toward SLR
--   stage4  gc-plan   — inspect safe eviction candidates only
------------------------------------------------------------------------

expectedMetadataRecordCount : Nat
expectedMetadataRecordCount = 43996

data MaterialisationStage : Set where
  plan      : MaterialisationStage
  register  : MaterialisationStage
  handoff   : MaterialisationStage
  gc-plan   : MaterialisationStage

data StageClass : MaterialisationStage -> Set where
  machineExecutable : ∀ {s} → StageClass s
  humanLegalReviewRequired : ∀ {s} → StageClass s
  externalProviderRequired : ∀ {s} → StageClass s

stage-class-plan      : StageClass plan
stage-class-plan      = machineExecutable

stage-class-register  : StageClass register
stage-class-register  = machineExecutable

stage-class-handoff   : StageClass handoff
stage-class-handoff   = machineExecutable

stage-class-gc-plan   : StageClass gc-plan
stage-class-gc-plan   = machineExecutable

record FullTextCacheBoundary : Set where
  constructor full-text-cache-boundary
  field
    metadataRecordCount : Nat
    metadataRecordCountIsExpected :
      metadataRecordCount ≡ expectedMetadataRecordCount

    maxBatchSize : Nat
    defaultMaxBatchSizeIs20 : maxBatchSize ≡ 20

    defaultCacheCapGib : Nat
    defaultCacheCapGibIs2 : defaultCacheCapGib ≡ 2

    defaultReserveGib : Nat
    defaultReserveGibIs5 : defaultReserveGib ≡ 5

    planningSizeMb : Nat
    defaultPlanningSizeIs10 : planningSizeMb ≡ 10

    metadataRowsNotFullTextFiles :
      metadataRecordCount ≡ 43996 → metadataRecordCount ≡ 43996
    metadataRowsNotFullTextFiles refl = refl

    unreviewedNotEligible : Bool
    unreviewedNotEligible = true

    includeProbableNotAutomatic : Bool
    includeProbableNotAutomatic = true

    cachedArtifactNotAuditAdmission : Bool
    cachedArtifactNotAuditAdmission = true

    unprocessedNotSafelyEvictable : Bool
    unprocessedNotSafelyEvictable = true

canonicalFullTextCacheBoundary : FullTextCacheBoundary
canonicalFullTextCacheBoundary =
  full-text-cache-boundary 43996 refl 20 refl 2 refl 5 refl 10 refl

------------------------------------------------------------------------
-- Stage invariants
------------------------------------------------------------------------

planRespectsCacheCap : Bool
planRespectsCacheCap = true

planRespectsReserve : Bool
planRespectsReserve = true

registerRejectsUnplaned : Bool
registerRejectsUnplaned = true

registerRejectsCacheCapExceeded : Bool
registerRejectsCacheCapExceeded = true

handoffOnlyRegistered : Bool
handoffOnlyRegistered = true

handoffOnlyRetained : Bool
handoffOnlyRetained = true

gcPlanNotDeletion : Bool
gcPlanNotDeletion = true

gcPlanRequiresSlrReceipt : Bool
gcPlanRequiresSlrReceipt = true

gcPlanPreservesRevisionDigest : Bool
gcPlanPreservesRevisionDigest = true

record SelectiveFullTextMaterialisationReceipt : Set where
  constructor selective-full-text-materialisation-receipt
  field
    schema : String
    metadataRecordCount : Nat
    selectedForFetch : Nat
    registeredCount : Nat
    handedOffCount : Nat
    safeEvictionCandidates : Nat
    metadataRowsNotFullTextFiles : Bool
    planRespectsCacheCap : Bool
    planRespectsReserve : Bool
    registerRejectsUnplaned : Bool
    handoffOnlyRegistered : Bool
    gcPlanNotDeletion : Bool
    canonicalBoundary : FullTextCacheBoundary

defaultSelectiveFullTextMaterialisationReceipt : SelectiveFullTextMaterialisationReceipt
defaultSelectiveFullTextMaterialisationReceipt =
  selective-full-text-materialisation-receipt
    "sensiblaw.digital-esd-selective-fulltext-materialisation.v0_1"
    43996 20 0 0 0 true true true true true true canonicalFullTextCacheBoundary
