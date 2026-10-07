# Shared Matter Controversy Design

## Goal

Make one persisted legal Matter carry propositions, evidence, typed responses, authority, review, residuals and provenance, then project that same object for client/affected-community, solicitor/counsel and court users without changing its meaning or making adjudicative decisions.

## Binding reference

This spec implements the 2026-10-07 REAL-MATTER max-cut supplied in conversation and reuses `DASHI.Reasoning.JusticeLeeSensibLawAdversarialProofGraphBidiExact` as the formal type/reference owner. Justice Lee is design motivation only; the proof-graph/response/residual machinery remains DASHI/SensibLaw construction.

## Persisted controversy

A controversy belongs to one `matter_ref`. It contains persisted proposition fibres, typed response edges, controversy residuals and proof obligations. Every proposition preserves party role, legal role, epistemic status, source/review coordinates, evidence kind, time/relation and normative-order context. Response modes are typed and must not collapse to Boolean negation.

The first runtime response modes are the formal owner's existing modes: deny occurrence; admit occurrence/dispute characterisation; admit conduct/add context; dispute causation; challenge evidence reliability; offer alternative event; admit proposition.

Residual kinds include node/relation/evidence/characterisation/causal/legal-consequence disagreement and preserve an unresolved question plus the underlying persisted REL residual/obligation when one exists.

## Reverse proof search and acquisition impact

A persisted procedural goal projects the same controversy backwards to open obligations, candidate residuals, a requested discriminator and a target evidence query. It may identify which unresolved proposition/residual an acquisition could affect, but it does not rank credibility, prove relevance, create access authority, applicability or truth. Existing REL and INV owners remain authoritative for comparison, obligation and acquisition lifecycle.

## Persona projections

All personas consume the same persisted controversy.

- Client/affected-community: what is said, supporting evidence, what is disputed, the other side's position, what remains needed, reviewed vs candidate state, and normative-order context. Indigenous normative claims and Crown/municipal propositions remain distinct and explicitly related.
- Solicitor/counsel: controversy map, support/contrary material, typed response mode, admitted/common-ground propositions, source/review coordinates, residuals, reverse proof obligations and acquisition impact.
- Court/associate: common ground, typed disputed occurrence/characterisation/causation/evidence/legal consequence, supporting sources/authorities and unresolved evidentiary/legal obligations. No credibility, ultimate-fact, normative-weight or merits determination.

Projection is read-only: hidden != false, unshared != absent, synthesis != admission, and no projection mutates the canonical Matter.

## First empirical specimen

Mabo is the first target, but the old Wikisource page/span must not be relabelled as OALC. The real run must use a new provider-backed exact source and human-reviewed role/order decision. A miniature controversy must include at least one reviewed proposition, one typed opposing response, one support/authority coordinate and one genuine unresolved residual that can produce a targeted evidence/authority query.

## Non-goals

No second source/review/REL/INV backend. No automated legal merits, credibility, ultimate fact, normative weight or final judgment. No generic source-materialisation redesign. No silent cross-order collapse. No synthetic REL second observation or consumer invented merely to complete the demo.