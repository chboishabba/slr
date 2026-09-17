# Mabo P7d re-entry / recurrence / lineage delta — 2026-09-17

This addendum continues the accepted residual-driven P7 roadmap after P7d.1 was
operator-certified Cargo GREEN on SLR PR #22 at `0351844`.

## Current status

```text
P7d.0 one exact residual -> reviewed admission step          PAID @ 68dcab7
P7d.1 producer artifact -> ExpansionCandidate               PAID @ 0351844
P7d.2 post-acquisition PNF/world re-entry                    SOURCE-WRITTEN
P7d.3 canonical ProofFrontier recurrence                     SOURCE-WRITTEN
P7d.4 durable discovery lineage + atomic session             SOURCE-WRITTEN
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

## P7d.4 lineage + transaction boundary

P7d.0's step receipt was strengthened to retain two coordinates that durable
lineage actually needs:

```text
selected_discovery_parent_ref
selected_source_revision_ref
```

The proof-search `DiscoveryLineageReceipt` retains:

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

`WorldExpansionSession` stages admission on a cloned ledger. It commits the
ledger, recomputed frontier, and lineage together only after post-acquisition
re-entry succeeds. A failed re-entry therefore cannot advance the 100-object
counter, frontier state, or lineage history.

### Provider-neutral PostgreSQL boundary

The PostgreSQL store deliberately does **not** depend on `sl-proof-search-loop`.
The semantic/runtime receipt is projected at the boundary into the primitive
`DiscoveryLineageInput`, and `sl-pg-source-store` owns only durable storage.
This preserves the dependency direction:

```text
proof-search semantic receipt
-> thin projection at integration boundary
-> provider-neutral DiscoveryLineageInput
-> context.discovery_lineage_receipt
```

The append-only table uses a deterministic SHA-256 receipt over the stored
coordinates. Persistence cannot create authority, applicability or truth.

Runnable materialiser:

```text
cargo run -p sensiblaw-pg-source-store \
  --example materialize_mabo_discovery_lineage -- \
  <object_ref> <parent_ref> <residual_ref> <candidate_ref> <producer_lane> \
  <source_revision_ref> <pnf_world_ref> <expected> <observed> [new_residual ...]
```

It uses the existing `DATABASE_URL` / `.env` configuration loader and constructs
the provider-neutral storage input directly.

## Actual blocker

The remaining blocker is no longer a missing Rust type, planner, recurrence,
transaction, or persistence seam. To advance P7d.5 we now need actual execution
to produce and feed:

```text
live open Mabo ProofFrontier
+ real governed producer artifact
+ post-acquisition parse/PNF/world diagnosis
+ explicit review/disambiguation
+ PostgreSQL materialisation
```

and repeat the recurrence until either:

```text
total_new_world_objects >= 100
```

or the live frontier/provider world exposes a genuine acquisition/access/data
blocker.

No source-written status in this addendum is an execution-GREEN claim. The new
Rust and PostgreSQL code still require fresh Cargo/clippy/workspace execution,
and the new PostgreSQL table/materialiser requires a live DB receipt.
