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
-> dependency-scoped reopen
-> ordinary SLR recurrence/payment
```

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

A changed formal statement requires rerouting or an explicit equivalence/transport receipt upstream. Symbol-name reuse, DA51 reuse, or artifact reuse is insufficient.

The resulting payment only returns the demand's declared dependency-scoped `reopen_refs`; unrelated fibres stay closed.

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
```

Every demand is candidate-only and non-promoting. Every payment hard-codes `creates_legal_authority = false`, `applicability_promoted = false`, and `claim_truth_promoted = false`. Formal proof can discharge only the formal premise it was requested for; legal and factual promotion remain on their existing explicit SLR review/payment paths.

## Cross-repo ownership

- DASHI/Agda: golden PNF/proof/debt/search contracts.
- SLR/Rust: production PNF, residual, proof obligation, payment and recurrence.
- dashi-repo: repository discovery/dedupe/history and routing receipts.
- dashi_lean4: DA51 ABI and Lean proof-bearing boundary.
- agda2lean: translation capability only.
- Aristotle: adaptive reasoning/formalisation backend, never certification authority.
- ITIR/ERDFA/Kant: optional evidence artifact/shard packaging.
- IPFS/HF/file: sink/materialisation refs only.
