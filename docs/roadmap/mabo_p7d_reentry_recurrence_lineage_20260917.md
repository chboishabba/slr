# Mabo P7d re-entry / recurrence / lineage delta — 2026-09-17

This addendum continues the accepted residual-driven P7 roadmap after P7d.1 was
operator-certified Cargo GREEN on SLR PR #22 at `0351844`.

## Current status

```text
P7d.0 one exact residual -> reviewed admission step          PAID @ 68dcab7
P7d.1 producer artifact -> ExpansionCandidate               PAID @ 0351844
P7d.2 post-acquisition PNF/world re-entry                    SOURCE-WRITTEN
P7d.3 canonical ProofFrontier recurrence                     SOURCE-WRITTEN
P7d.4 durable discovery lineage                              SOURCE-WRITTEN
P7d.5 live Mabo run >= 100 novel admitted objects            BLOCKED ON ACTUAL RUN
```

## P7d.2 observed world delta

Predicted candidate contraction remains a scheduling coordinate only. After an
object is actually acquired/admitted, the caller supplies an explicit
`PostAcquisitionWorldObservation` containing:

```text
observation identity
exact source revision
exact triggering residual
ResidualAssessmentKind
observed residual contraction
explicitly diagnosed new open residuals
PNF/world disambiguation receipt
candidate-only observation authority
```

The re-entry code never infers new residuals from identifiers, source names,
Wikipedia adjacency, QIDs, or citation links.

## P7d.3 recurrence

`world_expansion_reentry::reenter_after_acquisition(...)` delegates the original
residual update to the canonical `transition::apply_assessments(...)` surface.
Only explicitly diagnosed new residuals are then appended, and each must be new
and `ResidualStatus::Open`.

Therefore recurrence is:

```text
actual acquired object
-> explicit parse/PNF/world diagnosis
-> observed ResidualAssessment
-> canonical apply_assessments
-> append explicitly diagnosed new open residuals
-> recomputed ProofFrontier / ResearchTermination
-> existing Ibrahim/frontier machinery chooses next work
```

No second scheduler or planner was introduced.

## P7d.4 lineage ABI repair

P7d.0's step receipt was strengthened to retain two coordinates that durable
lineage actually needs:

```text
selected_discovery_parent_ref
selected_source_revision_ref
```

The durable `DiscoveryLineageReceipt` now retains:

```text
object_ref
discovery_parent_ref
triggering_residual_ref
selected_candidate_ref
producer_lane
source_revision_ref
pnf_world_disambiguation_ref
expected_residual_contraction
observed_residual_contraction
new_residual_refs
candidate_only = true
creates_semantic_authority = false
applicability_promoted = false
claim_truth_promoted = false
```

`sl-pg-source-store` adds append-only
`context.discovery_lineage_receipt`, with a deterministic SHA-256 receipt over
these coordinates. Persistence cannot create authority, applicability or truth.

Runnable materialiser:

```text
cargo run -p sensiblaw-pg-source-store \
  --example materialize_mabo_discovery_lineage -- \
  <object_ref> <parent_ref> <residual_ref> <candidate_ref> <producer_lane> \
  <source_revision_ref> <pnf_world_ref> <expected> <observed> [new_residual ...]
```

It uses the existing `DATABASE_URL` / `.env` configuration loader.

## Actual blocker

The remaining blocker is no longer a missing Rust type or orchestration seam.
To advance P7d.5 we need a real execution environment to produce and feed:

```text
live open Mabo ProofFrontier
+ real governed producer artifact
+ post-acquisition parse/PNF/world diagnosis
+ explicit review/disambiguation
+ PostgreSQL materialisation
```

and then repeat the recurrence until either:

```text
total_new_world_objects >= 100
```

or the live frontier/provider world exposes a genuine acquisition/access/data
blocker.

No source-written status in this addendum is an execution-GREEN claim. The new
Rust and PostgreSQL code still require fresh Cargo/clippy/workspace execution,
and the new PostgreSQL table/materialiser requires a live DB receipt.
