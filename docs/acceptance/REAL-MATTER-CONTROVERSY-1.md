# REAL-MATTER-CONTROVERSY-1 Acceptance

## Product claim under test

A single persisted legal Matter can carry reviewed propositions, typed adversarial responses, authority/evidence ancestry, unresolved residuals and proof obligations, then be projected for client/community, solicitor/counsel and court users without changing the underlying Matter or automating adjudication.

The first empirical specimen is a genuine provider-backed Mabo matter. Historical Wikisource coordinates are not relabelled as OALC coordinates; a provider-backed run must use its own persisted source/review ancestry.

## User stories

### 1. Controversy decomposition

**As a legal operator, I want each disagreement classified by what is actually disputed so that opposition is not flattened into Boolean negation.**

Acceptance:
- occurrence denial is distinct from admitted occurrence / disputed characterisation;
- characterisation is distinct from causation;
- evidence reliability is distinct from occurrence;
- legal consequence and normative-order mismatch remain explicit residual kinds;
- replay returns the exact persisted response/disagreement kind.

### 2. Backward proof search

**As counsel, I want to start from a procedural goal and see open proof obligations and targeted evidence/authority queries.**

Acceptance:
- reverse search reads persisted open obligations and residuals;
- it returns the persisted requested discriminator and target evidence query;
- it does not create a verdict, credibility finding, ultimate fact, normative-weight assignment or final judgment;
- targeted residual search does not silently count retrieved material as admitted support.

### 3. Common-ground isolation

**As a judge/associate, I want common ground separated from unresolved controversy without turning admission into truth.**

Acceptance:
- a proposition fibre is common ground only when its persisted epistemic status is `Admitted`;
- `AdmitOccurrenceDisputeCharacterisation` does not mark the whole disputed proposition as admitted;
- common-ground projection leaves `claim_truth_promoted=false` and `applicability_promoted=false`.

### 4. Normative-order preservation

**As an affected-community member, client or legal reviewer, I want Indigenous normative claims and Crown municipal-law propositions to remain distinct and explicitly related.**

Acceptance:
- every legal proposition fibre carries a durable `normative_order_ref`;
- client and solicitor projections expose the distinct order refs;
- a `NormativeOrderMismatch` residual may remain open;
- recognition never creates the recognised normative order;
- display/persistence never infers cross-order priority.

### 5. Reviewed legal meaning

**As a reviewer, I want my exact source/candidate/evidence-role/normative-order decision preserved downstream.**

Acceptance:
- controversy materialisation consumes existing `reviewed_evidence_ref` coordinates;
- declared expected normative order must equal the persisted review decision;
- a provider-backed legal source manifestation must reopen for the same source revision/document;
- no runner creates or rewrites a human review decision.

### 6. Evidence-acquisition impact

**As a solicitor/investigator, I want to know which unresolved question a proposed acquisition would address before spending effort on it.**

Acceptance:
- each open controversy residual may carry a requested discriminator and target evidence/authority query;
- reverse search exposes those targets with the open obligation;
- future REL/INV routing starts from this residual rather than a generic request to find more documents;
- this tranche does not automatically choose or execute an acquisition route.

### 7. Judicial reconstruction

**As a judge/associate, I want a source-linked reconstruction of common ground, typed disputes and open questions without automated merits determination.**

Acceptance:
- court projection separates occurrence, characterisation/context, causation, evidence reliability, legal consequence and normative-order mismatch;
- source/review coordinates remain reopenable through the shared persisted Matter;
- `determines_credibility=false`;
- `determines_ultimate_fact=false`;
- `assigns_normative_weight=false`;
- `enters_final_judgment=false`.

## First empirical Mabo receipt

The first live receipt must establish a miniature controversy over real persisted coordinates:

```text
provider-backed reviewed proposition A
  + exact normative-order ref
  + source/review ancestry

respondent/other-side reviewed proposition B
  + explicit typed ResponseMode

reviewed source/authority support
  + retained exact source ancestry

open residual R
  + DisagreementKind
  + unresolved question

proof obligation O
  + requested discriminator
  + targeted evidence/authority query
```

Then the same `matter_ref` / `controversy_ref` must load through all three product projections:

```text
Client/community -> what is said / supported / disputed / still needed / which normative order
Solicitor/counsel -> typed responses / weaknesses / obligations / reverse-search target
Court             -> common ground / typed controversy / open questions / no merits automation
```

The applicant reviewed-evidence/source coordinate may constitute the first support/authority payment for proposition A; the system does not invent a third authority merely to satisfy a diagram. If a genuinely distinct authority C is required by the matter, it must be persisted/reviewed as its own source-backed proposition/support coordinate.

## Runtime runner

Normal empirical invocation is by persisted refs, not JSON:

```sh
cargo run -p sensiblaw-pg-source-store --example itir_real_matter_controversy -- \
  <matter-ref> \
  <applicant-claim-ref> <applicant-reviewed-evidence-ref> <applicant-normative-order-ref> \
  <respondent-claim-ref> <respondent-reviewed-evidence-ref> <respondent-normative-order-ref> \
  <response-mode> <disagreement-kind> \
  '<unresolved-question>' '<requested-discriminator>' '<target-evidence-query>' \
  [procedural-goal]
```

The runner must fail closed when the reviewed evidence, claim, provider-backed manifestation, or normative-order agreement is absent.

## Dioxus acceptance

With `ITIR_CONTROVERSY_REF` selected, `ITIR_MATTER_REF` is mandatory. The workbench must reject a persisted controversy belonging to another Matter rather than selecting a first/closest match.

Persona switching changes projection only; it does not create different Matter objects or mutate persisted legal state.

## Exit criterion for this tranche

`REAL-MATTER-CONTROVERSY-1` is runtime-complete when:

1. the SLR controversy contract and package tests pass;
2. Dioxus persona-projection tests pass against the exact SLR head;
3. Agda focused source/preflight reaches either kernel success or only the already-known unrelated `TypedDependencyCore` universe blocker;
4. one genuine provider-backed Mabo controversy has traversed the persisted human-review boundary and produced a real residual/query target.

Items 1–3 certify the implementation. Item 4 is the empirical acceptance receipt and remains the decisive product gate.
