# ITIR-INV-1 — proof-directed investigation and acquisition

ITIR-INV-1 generalizes existing Digital-ESD adaptive screening and
Amy Eskridge proof-directed acquisition into a source-neutral ITIR capability.

It does **not** make acquisition priority a semantic authority.

The runtime loop is:

```
REL residual
  -> targeted acquisition obligation
  -> candidate lawful routes
  -> non-scalar Pareto frontier
  -> separately completed acquisition
  -> native source ingestion
  -> acquisition result receipt
  -> explicit dependency-closure reopening
  -> ordinary source -> observation -> PNF -> Compare_q
```

## Core invariants

- `not_lodated` / `known_absent` / `present` are distinct.
- A Pareto-optimal route may still be non-executable because access is
  unknown or authorization is missing.
- Pareto priority does not create truth, source quality, admission,
  reviewer priority, or access authority.
- Metadata duplicate, publication duplicate, report-family duplicate,
  same empirical study and common-source derivation are separate genealogy
  states.
- Source independence is separately evidenced. A new URL or publication ID
  is not an independent witness.
- New evidence selectively reopens only assessments connected by an explicit
  dependency path.
- Correlation/similarity does not create a dependency edge.

## Five acquisition axes

`AcquisitionRouteCandidate` carries five declared, separately inspectable
coordinates:

1. expected information/discrimination gain;
2. dependency-closure impact;
3. unpaid residual coverage;
4. provenance novelty / potential independence gain;
5. lawful acquisition / reviewer / resource cost.

The first four are maximized, the fifth minimized. The runtime Pareto function
uses no weighted sum and returns all non-dominated candidates.

Unknown/prohibited/authorization-pending routes remain visible on the frontier
but are excluded from `executable_frontier_route_refs`.

## Durable queue

`persist_acquisition_queue` requires an existing persisted
`DurableRelationalComparison` and an obligation that names one of that
comparison's actual residuals. The obligation must carry the exact two
native source revisions from that parent comparison.

The queue persists:

- immutable acquisition obligation;
- route candidates;
- recomputable Pareto priority receipt.

Reopening recomputes the frontier and reopens the parent REL comparison.

CLI:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example itir_inv1_queue -- \
  /path/to/acquisition-request.json
```

The command performs no network access.

## Acquisition outcome

`persist_acquisition_update` accepts only these transitions:

```
not_located -> present
not_located -> known_absent
```

For `present`, the acquired revision must already exist in the canonical
generic-source or chat source store. INV-1 therefore cannot manufacture a
source row merely to satisfy an obligation.

For `known_absent`, only the exact acquisition branch closes. No selective
reopening receipt is generated and no downstream hypothesis is refuted.

For `present`, the caller supplies an explicit dependency graph. The runtime
writes a `SelectiveReopeningReceipt` that distinguishes direct dependents,
transitive dependents, and unrelated assessment refs that remain closed.

CLI:

```bash
cargo run -p sensiblaw-pg-source-store \
  --example itir_inv1_update -- \
  /path/to/acquisition-result.json
```

This executable records an acquisition result; it does not perform the
acquisition.

## Operator view

`itir-dioxus` supports:

```bash
SENSIBLAW_MATTER_SCOPE=/path/to/matter-scope.json \
ITIR_INV_ACQUISITION_REF='acquisition:...' \
  cargo run --features desktop
```

The view requires all parent source revisions to remain visible under the
existing MatterContext. It shows:

- parent residual/comparison;
- target description and lawful-access constraint;
- each route's five Pareto axes;
- source genealogy / duplicate relation / independence state;
- frontier membership;
- executable versus access-blocked frontier routes;
- dependency targets.

It is read-only with respect to source acquisition and semantic admission.

## Formal owner

`DASHI/Core/ITIRInvestigationAcquisitionParetoExact.agda` reuses:

- `EvidenceAcquisitionSelectiveReopeningExact`;
- `AffectedDependencyClosureExact`;
- `AdmissibleConsumerMDLHyperfabricExact`;
- `NDimParetoHyperfabricExact`.

This is a general owner. Digital-ESD and Eskridge remain domain-specific
applications and evidence donors, not dependencies of the generic semantics.

## Acceptance still required

This tranche is source-written only until the new files are compiled at a
pinned head. Before INV-1 is runtime-accepted:

1. compile SLR and the new focused Agda owner;
2. persist one real REL residual;
3. create at least two non-dominated acquisition routes;
4. retain an access-blocked route without treating it as executable;
5. ingest a genuinely acquired source and verify direct/transitive selective
   reopening;
6. demonstrate `known_absent` closes only its own branch;
7. reopen the queue in Dioxus under MatterContext;
8. verify no source/claim/admission/access authority promotions.

REL-1C's three-domain corpus remains an independent acceptance gate.
INV-1 should help decide how to acquire missing evidence; it must not fabricate
that corpus.
