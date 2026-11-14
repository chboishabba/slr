# Source-unit / Wikidata bundle handoff

This crate is the Rust consumer surface for the formal DASHI.Wikimedia / mature SensibLaw boundary-artifact contracts.

Canonical flow:

```text
ExternalSource
  -> SourceUnit
  -> ObservationClaim
  -> StatementBundleReview
  -> SplitPlan / ReviewPacket
  -> Verification
  -> separately governed publication/promotion
```

The crate intentionally contains no HTTP client, database integration, parser execution, or publication API.

## Source units

`SourceUnit` mirrors the reusable `sl.source_unit.v1` shape closely enough for the Rust semantic runtime:

- textual or integer revision IDs;
- revision timestamp and retrieval method;
- nullable URL/title;
- source/content type;
- revision-scoped text;
- exact anchors;
- metadata/source receipt references.

A source unit is a provenance-bearing input artifact, not source authority.

## Native Wikidata reference roles

The bounded reference-property classifier follows DASHI's native reference semantics:

- `P248` (`stated in`) -> source candidate;
- `P854` (`reference URL`) -> source candidate;
- `P143` (`imported from Wikimedia project`) -> provenance only;
- other properties -> unresolved role.

The role classification does not verify URL content, truth, admissibility, or authority.

## Statement bundle is the review unit

Migration review uses the full statement bundle rather than a naked triple:

```text
MainSnak + Qualifiers + References + Rank + Provenance
```

`StatementBundleReview` keeps those as separately addressable coordinates. It is therefore possible to reopen only reference transfer, only qualifier interpretation, only rank treatment, etc.

## Sparse reverse reopening

Bundle changes compile into the existing `sensiblaw-evidential-reopen` reverse dependency engine.

A changed bundle coordinate wakes only consumer fibres that declared dependency on that exact coordinate. No registered dependency means zero work, not negative evidence.

For example, a changed `P854` reference may wake a `reference-transfer` consumer while leaving a mainsnak semantic-equivalence consumer asleep unless that consumer also declared a dependency on the changed reference coordinate.

## Nat / Climate calibration

The tests use the existing `P5991 -> P14143` Nat/Climate lane as a bounded calibration:

- source unit identity remains revision-scoped;
- expected qualifiers include `P3831`, `P459`, `P518`, `P580`, `P582`;
- bounded reference surface includes `P854`;
- the sample statement bundle remains `SplitRequired`;
- observation/review/bundle carriers never set world-truth, source-authority, or semantic-promotion authority.

## Formal counterpart

DASHI PR #814 contains the formal counterparts:

- `DASHI/Wikimedia/NativeReferenceSemanticsExact.agda`
- `DASHI/Wikimedia/SensibLawSourceUnitReviewHandoffExact.agda`
- `DASHI/Wikimedia/SensibLawBoundaryArtifactMorphismExact.agda`
- `DASHI/Wikimedia/SensibLawNatClimateReviewHandoffExact.agda`
- `DASHI/Wikimedia/SLRWikimediaHandoffABIExact.agda`

The Rust implementation does not inherit Agda proof authority merely by implementing compatible carriers.
