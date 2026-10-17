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

All four durable bridge layers are replay-strict:

```text
FormalProofDemand
FormalProofRouteReceiptInput
FormalEvidenceReceiptInput
FormalEvidencePayment
```

The PostgreSQL path may still use idempotent `ON CONFLICT DO NOTHING`, but it must immediately reopen the row inside the same transaction and compare the **complete immutable Rust value**. Reusing a stable ref with changed consumer, residual, proposition, statement language, prover, DA51 coordinate, source-revision set, requested result kind, dependent set, route coordinates, artifact/checker/certification coordinates, evidence dependencies, authority flags, or selective-reopening cone fails with `ExistingRowConflict` before commit.

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

The source includes unit/contract tests plus strict-replay regressions. Exact-head Cargo/PostgreSQL execution remains a separate verification payment whenever the execution environment cannot run those commands.

Focused commands:

```bash
cargo test -p sensiblaw-formal-proof-bridge
cargo test -p sensiblaw-pg-source-store --test formal_proof_bridge_contract
cargo test -p sensiblaw-pg-source-store
```
