# Formal Proof Bridge

Date: 2026-10-17

SLR remains the production owner for source/PNF/proposition/residual/proof-obligation/payment state. The formal-proof bridge is an additive interoperability layer for obligations whose next producer is a formal prover or formalisation backend.

```text
source/span
-> candidate PNF
-> explicit review
-> reviewed PNF
-> proposition support / LegalIR when applicable
-> ProofResidual / ProofObligation
-> FormalProofDemand
-> dashi-repo discovery/dedupe/history/routing
-> reuse | agda2lean | Aristotle | Lean/Agda formalisation
-> independent checker receipt
-> FormalEvidenceReceipt
-> exact SLR demand/obligation/digest weld
-> formal evidence SourceCoordinate
-> existing sparse_wake reverse-dependency engine
-> ordinary SLR recurrence/payment
```

Upstream typing remains separate. DASHI PR #1222 supplies the formal PNF/consumer-WrongType -> explicitly witnessed Lee residual adapter; SLR PR #66 supplies same-Matter consumer-informativeness receipts. Neither authorizes this formal route by itself: only an existing **open proof obligation** and reviewed PNF revision enter this bridge.

## Parser evidence reconstruction for language-as-algebra consumers

The M12 statement compiler remains the source-bound candidate PNF owner. Typed
PostgreSQL stores surface, lemma, POS, morphology, dependency heads, labels,
entity spans, and optionally full artifact bytes/locators. CandidatePnfFactor
is a coarse candidate projection, not the complete parser representation.

- load_parser_evidence_region reopens one successful candidate-only run/region
  with original tokens, entity annotations and parser artifact coordinates.
- compile_db_statement_pnf_checked(config, snapshot, statement) is the
  canonical one-call DB handoff. It loads exact parser observations, checks
  the canonical receipt db-parser:<run>:<region>, and reuses ordinary M12.
- compile_statement_pnf_with_parser_evidence also accepts a preloaded,
  sealed ParserEvidenceRegion and requires the original source literal
  to match SLR's existing parser_product_key (region text digest plus
  parser family/version/model/configuration). Missing product keys fail closed.
- ParserEvidenceRegion::matches_observation_coverage verifies complete
  non-whitespace token coverage, source-relative token surfaces,
  dependency head ordinals and named-entity spans. This is a bounded
  region-local check, not document-wide re-materialization.
- audit_candidate_pnf_evidence rejects mismatched run-derived candidate IDs,
  source revisions, persisted region boundaries, surface, lemma, dependency,
  role, missing tokens and claimed authority payment.
  Success certifies parser projection parity ONLY, not semantic adequacy.

This strict path expects a parser job with a product identity from SLR's
existing content-reuse enqueue path. Older rows with a NULL parser_product_key
are unresolved here; the broader legacy adapter remains unchanged.

For a consumer question Q_c, the distinct factorisation obligation is:

    forall x y. pi(x) = pi(y) => Q_c(x) = Q_c(y)

Suppressed syntax must be explicitly reconstructed or marked unresolved.
Do not conflate dependency roles with legal scope, identity or proposition truth.

## Lean -> SLR -> Lean and the global proof world

Forward: Lean/Agda declaration or discovered artifact -> canonical statement
and dependency fingerprint -> exact repository commit/source coordinates ->
SLR source and reviewed PNF/proof obligation. Reverse: FormalProofDemand ->
exact formal statement and source/dependency set -> dashi-repo reuse or
Agda-to-Lean translation candidate -> independent Lean/Agda kernel check ->
SLR FormalEvidenceReceipt -> exact payment -> selective dependency reopening.
Structural shape and round-trip translation alone do not certify equality.
Source code stays at its original repository owner; avoid full copies in SLR.

## Marchenko-Pastur (PNF MP) spectral candidate gate

Reuse the existing owners rather than building a second classifier:

- dashi_agda/DASHI/Information/PNFMarchenkoPasturAdmissionExact.agda
- dashi_agda/scripts/pnf_marchenko_pastur_numeric.py
- ITIR-suite/itir-mcp/src/itir_mcp/pnf_spectral_packet.py

MP describes a numerical feature-panel null. Spectral eigen-motifs may rank
candidate proof-world joins and guide retrieval; they do not establish
declaration equivalence, legal applicability, Wikidata truth or proof soundness.
The Agda mpCandidateCannotPromoteNumericABI explicitly maintains this boundary.
No MP result pays SLR proof obligations without exact statement/dependency
evidence and independent kernel-certified checker receipts.

The formal bridge's certification_state = "kernel-certified" value is
currently a supplied receipt field: validating this string and matching
statement digests does not independently execute a Lean/Agda kernel or
authenticate the checker. That trust gate remains open, regardless of the
stronger source/parser parity achieved above.

### Remaining acceptance gates

- Cargo/PostgreSQL tests for the new reopened evidence path (not yet executed).
- Live provider revision -> parser run -> evidence-backed M12 candidate ->
  reviewed proposition -> formal demand -> independent kernel receipt.
- Consumer-specific MP/PNF factorisation and missing-feature residual checks.
- Agda/Lean round-trip equivalence of statement and dependencies.
- End-to-end Lean -> SLR -> Lean proof-world reuse and invalidation acceptance.

## Storage boundary

`formal_bridge.*` stores only demand/route/evidence/payment joins. It does not copy PNF graphs, proposition payloads, controversy state, or proof-obligation bodies. `FormalProofDemand` must reopen an existing open `legal_controversy.proof_obligation` and an existing `algebra.factor_revision` before it can be persisted.

## Exact-payment boundary

Formal evidence pays only the exact formal premise named by its demand when all of these agree:

- `demand_ref`;
- `obligation_ref`;
- `statement_digest`;
- required `result_kind`;
- DA51 address when the demand requires one;
- an independently named checker/revision with `certification_state = kernel-certified`.

A changed formal statement requires rerouting or an explicit equivalence/transport receipt upstream. Symbol-name reuse, DA51 reuse, or artifact reuse is insufficient. Source/dependency array entries are validated as non-empty coordinates rather than accepting blank placeholders.

## Immutable replay

The max-cut source audit found that idempotent inserts were not sufficient by themselves: a reused stable ref could preserve an older row while the caller supplied changed coordinates. The bridge therefore treats all four durable layers as replay-strict:

```text
FormalProofDemand
FormalProofRouteReceiptInput
FormalEvidenceReceiptInput
FormalEvidencePayment
```

The PostgreSQL path may still use idempotent `ON CONFLICT DO NOTHING`, but it immediately reopens the row inside the same transaction and compares the **complete immutable Rust value**. Reusing a stable ref with changed consumer, residual, proposition, statement language, prover, DA51 coordinate, source-revision set, requested result kind, dependent set, route coordinates, artifact/checker/certification coordinates, evidence dependencies, authority flags, or selective-reopening cone fails with `ExistingRowConflict` before commit.

This is deliberately stronger than matching only the obligation and statement digest. A lookalike receipt cannot inherit a previous demand, independently certified evidence record, or payment.

## Selective reopening

`sensiblaw-formal-proof-reopen` owns no dependency graph. After exact formal payment, it projects the evidence ref to the existing:

```text
SourceCoordinate {
  kind = Evidence,
  stable_ref = formal evidence ref
}
```

and delegates wake selection to `sensiblaw-evidential-reopen::sparse_wake`.

Therefore:

```text
explicit reverse dependency -> matching consumer fibre may reopen
missing dependency -> zero work
unrelated dependency -> stays closed
```

The older `reopen_refs` field on the payment remains an explicit producer-side hint/audit coordinate; it is not allowed to bypass the canonical reverse-dependency graph. Its persisted value is nevertheless immutable and replay-checked, so a reused evidence/payment ref cannot silently widen the recorded reopening cone.

## Authority firewalls

```text
FormalProofDemand != proof
FormalProofRouteReceipt != proof
DA51 address/status != proof
agda2lean capability != proof
Aristotle candidate != certification
artifact/CID/URI != proof
formal proof != legal applicability
formal proof != claim truth
formal evidence wake != semantic/legal promotion
```

Every demand is candidate-only and non-promoting. Every payment and reopen plan hard-code no legal authority, applicability, or claim-truth promotion. Formal proof can discharge only the formal premise it was requested for; legal and factual promotion remain on their existing explicit SLR review/payment paths.

## Cross-repo ownership

- DASHI/Agda: golden PNF/proof/debt/search contracts.
- SLR/Rust: production PNF, residual, proof obligation, payment and recurrence.
- dashi-repo: repository discovery/dedupe/history and routing receipts.
- dashi_lean4: DA51 ABI and Lean proof-bearing boundary.
- agda2lean: translation capability only.
- Aristotle: adaptive reasoning/formalisation backend, never certification authority.
- ITIR/ERDFA/Kant: optional evidence artifact/shard packaging.
- IPFS/HF/file: sink/materialisation refs only.

## Verification boundary

The source includes unit/contract tests plus strict-replay regressions. This connector environment does not expose Cargo/PostgreSQL execution, so no exact-head GREEN claim is made from this session.

Focused execution payment:

```bash
cargo test -p sensiblaw-formal-proof-bridge
cargo test -p sensiblaw-pg-source-store --test formal_proof_bridge_contract
cargo test -p sensiblaw-pg-source-store
```
