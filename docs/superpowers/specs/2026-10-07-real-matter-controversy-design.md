# REAL-MATTER-CONTROVERSY-1 Design

This document is the repo-local execution mirror of the approved 2026-10-07 max-cut supplied in the working conversation. It does not introduce a new product direction.

## Goal

Build one persisted legal controversy over a shared Matter such that client, solicitor/counsel, and court-facing projections all consume the same source/review/provenance-bearing substrate without adjudicating it.

The unit of value is whether a user can understand the controversy, find what matters, test it, and hand it to the next legal actor without losing provenance or changing meaning.

## Canonical semantic donor

`DASHI/Reasoning/JusticeLeeSensibLawAdversarialProofGraphBidiExact.agda` is the canonical donor for:

- proposition-oriented legal fibres;
- typed adversarial response modes;
- controversy residuals;
- proof obligations;
- forward compilation and reverse proof search;
- the machine/adjudication authority firewall.

Existing two-order / reviewed-evidence Agda owners remain authoritative for normative-order non-collapse and durable review meaning.

## Runtime architecture

Reuse the existing generic `chronology_contestation` proposition/claim ancestry rather than replacing it. Add a legal controversy layer above it that binds:

- Matter + root proposition;
- claim -> legal proposition-fibre metadata (party, legal role, epistemic status, evidence kind, normative order, reviewed-evidence/source ancestry);
- typed response edges;
- typed controversy residuals;
- proof obligations;
- reverse-search receipts.

The legal layer is persisted, candidate/challengeable, and cannot create semantic authority, legal authority, applicability, credibility findings, ultimate facts, normative weight, or final judgment.

## Typed response modes

At minimum preserve the Agda distinctions:

- deny occurrence;
- admit occurrence / dispute characterisation;
- admit conduct / add context;
- dispute causation;
- challenge evidence reliability;
- offer alternative event;
- admit proposition.

They must not collapse to Boolean negation.

## Residuals and proof search

Preserve the Agda disagreement kinds and explicitly allow a normative-order mismatch residual at the runtime/product boundary, sourced from the existing two-order formalism. A residual carries an unresolved question and may point to a requested discriminator / target evidence query.

Reverse search starts from a procedural goal and returns existing open obligations/residuals. It proposes what to investigate; it does not decide admissibility, credibility, merits, or truth.

## Empirical REAL-MATTER path

Mabo is the first acceptance specimen, but no historical Wikisource source coordinate may be relabelled as OALC. The strict runner consumes already-persisted provider-backed review decisions/reviewed evidence and fails closed when they are absent.

A minimal accepted controversy contains:

1. Proposition A with exact reviewed source role/order;
2. Response B with an explicit typed disagreement;
3. authority/support C;
4. genuinely unresolved residual R;
5. a targeted evidence/authority query derived from R.

No human review decision may be invented by the runner.

## Persona projections

All persona views are projections over the same persisted Matter.

### Client / affected community

Show what is said, supporting evidence, disputes, other-side responses, outstanding needs, and reviewed-vs-candidate state. Indigenous normative claims and Crown municipal-law propositions remain separate and explicitly related.

### Solicitor / counsel

Show proposition -> evidence/authority -> typed responses -> weaknesses/open obligations -> residuals. Provide reverse proof search from a requested order/proposition to open obligations and targeted evidence/authority queries.

### Judge / associate

Show common ground, typed disputed occurrence/characterisation/causation/evidence/legal consequence, source-linked support, and open questions. Never recommend who should win or manufacture credibility/ultimate-fact/normative-weight/final-judgment determinations.

## Dioxus boundary

The workbench consumes persisted controversy state. It does not duplicate persistence, review logic, proof admission, or adjudication logic. The existing Matter workspace gains controversy projections rather than a second backend.

## Acceptance boundaries

- same persisted Matter underlies all persona projections;
- response modes remain typed;
- common ground is isolated without converting admission into truth;
- normative orders remain explicit and non-collapsed;
- every displayed proposition/response/residual can reopen provenance coordinates;
- reverse proof search returns obligations/residuals/query targets but no merits outcome;
- UI contains no operation that determines credibility, ultimate fact, normative weight, or final judgment;
- Mabo empirical construction requires genuine persisted provider-backed review coordinates.

## Out of scope for this tranche

- automated merits determination;
- a new source/materialisation subsystem;
- full document/work-product generation;
- automatic selection of a second REL observation or consumer;
- full INV Present/KnownAbsent reopening. Those follow from the first genuine controversy residual.
