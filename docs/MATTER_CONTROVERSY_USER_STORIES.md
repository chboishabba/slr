# Matter controversy user stories

These stories refine the existing Mary/SensibLaw operator stories after REAL-MATTER-1 exposed the durable legal-review and adversarial-controversy boundaries. They are acceptance requirements over the existing persisted Matter substrate, not mandates for a second backend.

## SL-US-15 — Controversy decomposition

**As a legal operator, I want each disagreement classified by what is actually contested, so that occurrence, characterisation, evidence reliability, causation, relation and legal consequence are not flattened into Boolean opposition.**

Acceptance:
- every response has an explicit typed response mode;
- response target and responding proposition belong to the same Matter;
- occurrence denial is distinguishable from admitted occurrence/disputed characterisation, causation dispute and reliability challenge;
- synthesis does not imply party admission or proved fact.

## SL-US-16 — Backward proof search

**As counsel, I want to work backwards from a procedural goal or proposition to open proof obligations, unresolved residuals and a targeted evidence/authority query.**

Acceptance:
- reverse search is over the same persisted controversy object;
- common ground is not returned as an open obligation;
- unresolved residuals expose a requested discriminator and target query where present;
- reverse search does not itself create access authority, an acquisition decision, actual reopening, applicability or truth.

## SL-US-17 — Common-ground isolation

**As a judge or associate, I want admitted/common-ground propositions separated from unresolved controversy so that the remaining dispute can be reconstructed precisely.**

Acceptance:
- admitted propositions are inspectable as common ground;
- disputed occurrence, characterisation, causation, evidence and legal-consequence lanes remain distinct;
- source/review provenance remains visible;
- the projection does not determine credibility, ultimate fact, normative weight or judgment.

## SL-US-18 — Normative-order preservation

**As an affected community member, client or legal reviewer, I want Indigenous normative claims and Crown/municipal-law propositions to remain distinct and explicitly related, so that recognition is not silently treated as creation and one order is not silently ranked over another.**

Acceptance:
- normative-order coordinates survive persistence and every persona projection;
- cross-order propositions may be related without coordinate collapse;
- a judicial or municipal-law non-recognition/non-justiciability state is not represented as non-existence of another normative order;
- no projection assigns normative weight.

## SL-US-19 — Reviewed legal meaning

**As a legal reviewer, I want my review to durably bind the exact source observation, candidate interpretation, evidence role, consumer requirement, proposition and normative order, so downstream legal materialisation cannot reinterpret what I reviewed without another attributable review action.**

Acceptance:
- the same review receipt cannot pay a different source/candidate review item;
- evidence role and normative order cannot be supplied later on the strict production path;
- source manifestation and consumer/requirement cannot be swapped;
- accept and qualify remain distinguishable;
- review creates no semantic/legal authority, applicability or truth.

## SL-US-20 — Evidence acquisition impact

**As a solicitor or investigator, I want to know which unresolved propositions a prospective source could affect before acquisition effort is spent.**

Acceptance:
- impact is derived from a persisted controversy residual/proof obligation;
- potential reopening is clearly distinct from actual reopening;
- route priority does not create access/acquisition authority;
- Present/KnownAbsent lifecycle and selective reopening remain owned by INV.

## SL-US-21 — Judicial reconstruction

**As a judicial officer or associate, I want a source-linked reconstruction of competing propositions, typed responses, evidentiary bases and unresolved legal/evidentiary questions without automated merits determination.**

Acceptance:
- common ground and each dispute kind are independently inspectable;
- authorities/evidence remain linked to persisted source/review coordinates;
- changes between procedural/evidence stages can be represented without rewriting the historical state;
- the system has no winner, credibility score, ultimate-fact field, normative-weight assignment or final-judgment operation.

## Shared-Matter acceptance

Client/affected-community, solicitor/counsel and court projections must all carry the same `matter_ref` and `controversy_ref`. Persona-specific presentation may hide or group coordinates according to the active MatterContext, but `hidden != false`, `unshared != absent`, and a projection never mutates the canonical Matter.