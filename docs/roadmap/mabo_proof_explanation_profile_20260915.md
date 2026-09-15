# SLR roadmap: thin Mabo proof/explanation profile

Date: 2026-09-15

## Purpose

Define the next legal flagship without turning SLR into a second legal-semantics engine.

The profile is deliberately thin:

```text
SensibLaw Mabo proof request
-> typed SLR consumer/residual requirements
-> existing bounded SLR recurrence
-> acquired/reviewed source refs + residual/payment deltas
-> SensibLaw Mabo proof specimen
```

SLR owns cheap structural parsing, consumer-relative residual production, producer selection, bounded acquisition, recurrence, and provenance-preserving candidate evidence.

SensibLaw owns legal proposition identity, authority/application semantics, support/defeater/comparator roles, source-role review, admissibility, payment/promotion, and the human/legal explanation.

## Why Mabo remains the flagship

The point is not to globally "prove Mabo". The first complete specimen should take one concrete proposition chain already represented in the Mabo work and close it end to end:

```text
literal argument
-> authority candidate
-> applicability
-> support + defeater + comparator
-> residual
-> selective/full-source reacquisition
-> review/payment
-> human-readable explanation
```

That is a stronger architecture test than a large undirected corpus sweep because it proves that the algebra, parser, acquisition recurrence, source review, and explanation projection all refer to the same typed object.

## Legal/public-interest user story

The same profile should support more than lawyers.

A concerned citizen, activist, CLC client, or self-represented litigant should be able to bring a more complete and interpretable argument to a lawyer/CLC rather than only a prose bundle. A public-interest/political-accountability user should be able to expose:

```text
literal claim
+ source
+ applicable authority/rule
+ factual predicate events
+ supporting material
+ contrary/limiting material
+ unresolved residuals
+ available next questions/acquisition paths
```

This is not automatic legal correctness or advice. It is a typed, provenance-preserving argument object that makes disagreement and incompleteness inspectable.

## Production boundary

Do not call a legacy Python Mabo legal-semantic runner from production SLR.

Do not port Mabo legal semantics into Rust/SLR.

The old SensibLaw Python machinery may remain a golden/reference fixture for regression and corpus-specific comparison.

Production should eventually expose a thin runner such as:

```text
run_slr_mabo_proof_graph.sh
```

whose only legal-specific responsibility is compiling an admitted SensibLaw proof request into existing SLR consumer/source-role constraints and returning typed evidence/provenance deltas.

## Strict legal payment

Search results, encyclopedia prose, headnotes, snippets, and ontology candidates may route acquisition but cannot pay strict primary-authority obligations.

Required rule:

```text
primary-authority payment eligibility
-> verified full source
+ exact source span
+ admitted source role
+ review/payment decision
```

Firewalls:

```text
AcquiredAuthorityCandidate != ApplicableAuthority
AcquiredSource != EvidencePayment
SourceAgreement != IndependentAncestry
SLRRouteSelection != LegalConclusion
Defeater != Comparator != Support
ExactCommonGround -> zero further acquisition for that exact residual
```

## UI projection contract

The runner must not emit a separate simplified UI graph. It returns typed research/proof deltas to SensibLaw. A single canonical Mabo proof specimen is then projected by the UI:

```text
SensibLaw Mabo proof specimen
-> Explain
-> Inspect
-> Source
-> Graph
```

The default UI policy is:

```text
argument-first
-> graph-second
-> provenance-on-demand
```

Progressive disclosure is an observation policy, not epistemic deletion.

```text
hidden from current view
!= discarded
!= unavailable
!= unsupported
```

Every explanation-bearing claim must have a reversible path back to its proof/source coordinates.

## Legal-term/context drill-in

A selected legal term such as `estoppel` should support progressive context rather than a single global definition:

```text
term
-> quick lexical definition (for orientation)
-> Wikipedia/Wikidata context (navigation/identity only)
-> Australian legal construction in the selected corpus
-> situation-specific applicability/contingent arguments
```

The UI may show a short dictionary definition or encyclopedia lead first, but those do not become legal authority.

A `Show contingent arguments` action may expose the corpus-relative derivation/applicability graph only on demand.

## Wiki/Wikidata/source inspection

Wiki/Wikidata are research/context surfaces, not default proof authority.

The source trail should remain explicit:

```text
Wikipedia/Wikidata navigation
-> cited/follow candidate
-> acquired source
-> independent source-role review
-> possible payment
```

Keep the distinctions:

```text
QID identity
!= source identity
!= same-object evidence
!= semantic equivalence
!= authority
```

## Materialisation compatibility

This profile inherits the consumer-adequate materialisation roadmap.

The default explanation need not retain every authority/source byte locally. A skeletal corpus may support navigation and proof-graph structure while exact quotation/primary-authority audit triggers verified source reacquisition.

```text
Q_layExplanation may FactorsThrough explanation projection
Q_primaryAuthorityAudit does not FactorsThrough explanation-only projection
```

## High-alpha implementation order

Complete next:

1. formal UI/projection parity owner: progressive disclosure is reversible and query-indexed;
2. canonical Mabo proof-specimen contract in SensibLaw;
3. thin SLR Mabo consumer-profile adapter over the existing recurrence;
4. exact-source inspector coordinates and Wiki/Wikidata context drill-in;
5. one flagship proposition chain with support/defeater/comparator/residual/explanation;
6. Svelte `Explain <-> Inspect` projection over that same specimen.

Defer:

- a separate Mabo-only UI data model;
- graph-first default presentation;
- broad legal corpus crawling merely to make the graph look dense;
- legacy Streamlit as the destination frontend;
- Rust reimplementation of SensibLaw legal semantics.

## Acceptance criterion

A lay user should be able to answer:

```text
What changed?
Why could the court reach that result?
What authority/source supports this step?
What limits or contradicts it?
What is still unresolved?
```

without seeing the full proof graph unless requested.

An expert should be able to reopen the same explanation into exact spans, provenance, authority/application coordinates, alternative readings, support/defeater/comparator edges, and residual/acquisition history.
