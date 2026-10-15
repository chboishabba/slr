# SMITH-HCA37-INTERPRETATION acceptance

## Purpose

This acceptance gate instantiates the already-merged REAL-MATTER-CONTROVERSY-1 runtime for the statutory-interpretation dispute in *Smith v Chief Executive, Queensland Corrective Services* [2026] HCA 37 (judgment 14 October 2026).

It does **not** ingest the judgment, create a human review decision, infer evidence roles, assign normative orders, or fabricate legal authority. Every claim/evidence/order coordinate supplied to the live test must already exist in the canonical PostgreSQL stores and must have crossed the ordinary source/review gates.

## Primary-source attribution

The Smith specimen must retain opinion-level source identity.

- Gordon A-CJ and Beech-Jones J [4]: their reasons state that, properly construed, `assisted reproductive technology` includes only technology directly assisting conception or reproduction and does not include egg extraction/freezing that preserves fertility.
- Edelman J [32]-[36]: his reasons criticise the wide linguistic reading as `literalism` / `contextomy`, identify purpose/context (including the Human Rights Act context subject to procedural fairness), and state his direct/capable-by-itself construction.
- Edelman J [46]-[49]: his reasons distinguish conforming interpretation, severance and partial disapplication, with conforming interpretation concerned with meaning and the latter techniques constraining application after meaning is determined.

These propositions must not be flattened into a generic `HCA said` source coordinate where the proposition comes from separate reasons.

DASHI/SLR formal structures, representation tests, proof-search and acquisition machinery are repository constructions. Justice Lee, Two-Eyed Seeing/Kimmerer, intersectionality, Lacan/Irigaray, experimental-design and Pareto machinery retain their separate source/authority boundaries and are not legal authorities for Smith merely because their structural machinery is reused.

## Required persisted coordinates

Set the following environment variables to real existing refs:

```text
ITIR_SMITH_HCA37_MATTER_REF
ITIR_SMITH_HCA37_NARROW_CLAIM_REF
ITIR_SMITH_HCA37_NARROW_REVIEWED_EVIDENCE_REF
ITIR_SMITH_HCA37_APPLICANT_NORMATIVE_ORDER_REF
ITIR_SMITH_HCA37_WIDE_CLAIM_REF
ITIR_SMITH_HCA37_WIDE_REVIEWED_EVIDENCE_REF
ITIR_SMITH_HCA37_RESPONDENT_NORMATIVE_ORDER_REF
DATABASE_URL
```

The narrow and wide labels are fixture names only. They do not themselves prove that either construction is legally correct. The reviewed evidence refs must reopen the exact source/review ancestry used for those propositions.

## Live gate

Run:

```sh
cargo test -p sensiblaw-pg-source-store --test smith_hca37_real_matter_live -- --ignored --nocapture
```

The test invokes `materialize_real_matter_controversy_strict`, which reopens the persisted controversy children and rejects semantic mismatch.

The fixed controversy coordinates are:

```text
response mode:
  admit occurrence / dispute characterisation

disagreement kind:
  legal consequence

unresolved question:
  whether egg extraction/freezing falls within s 22 assisted reproductive technology on proper construction

discriminator:
  purpose-connected construction versus direct-assistance construction,
  retaining statutory purpose/context, rights context and opinion provenance

acquisition target:
  reviewed primary legal material capable of discriminating the constructions
  without treating acquisition/citation/precedent search as residual payment
```

## Acceptance requirements

A successful live receipt must:

1. reopen the exact supplied Matter, reviewed claim/evidence and normative-order coordinates;
2. persist/reopen the typed response, legal-consequence residual, discriminator obligation and reverse proof-search object;
3. remain `candidate_only=true`;
4. keep semantic authority, legal authority, applicability promotion and claim-truth promotion false;
5. preserve the applicant/respondent normative-order refs supplied by explicit human-reviewed upstream state;
6. make no claim that the two fixture constructions exhaust every semantically/logically compatible interpretation;
7. make no claim that precedent search generates the interpretation universe;
8. make no claim that the requested discriminator, investigative priority, or acquisition result itself decides statutory meaning.

## Companion max-cuts

- `dashi_agda` PR #1160: Smith PNF operative constellation, non-factorability, consumer-relative informativeness, experimental-design/Pareto/LegalFollow weld and no-collapse boundaries.
- `slr` PR #62: runtime consumer informativeness/dominance and discriminator→investigation→LegalFollow handoff receipts.

This acceptance gate deliberately remains independent of those draft PRs: it exercises the merged REAL-MATTER substrate and can run once the real Smith reviewed coordinates exist.
