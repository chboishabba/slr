# Mabo residual-stage commutativity

Date: 2026-10-17

## Live defect

The real Mabo REAL-MATTER replay reached the strict runtime and failed with `PersistedMeaningMismatch`. The stable controversy contains more than one immutable residual narrative in the same typed dispute lane. The historical aggregate loader correctly retains both, but reverse proof search previously selected the first residual target, allowing historical meaning to leak into the current operator projection.

## Max-cut

1. Preserve every existing residual row and obligation for audit/history.
2. Introduce an immutable, human-reviewed residual supersession receipt.
3. Derive one unique current residual head per typed lane; ambiguous unsuperseded heads fail closed.
4. Make reverse proof search and strict REAL-MATTER replay consume the derived current projection.
5. Provide a narrow operator to reconcile already-persisted Mabo residuals using an existing S29 review receipt.
6. Re-run the exact Mabo live gate; the failure that discovered the defect is the acceptance regression.

## Non-goals

- no destructive overwrite;
- no insertion-time or lexical "latest" rule;
- no new Matter/controversy ontology;
- no semantic/legal authority or claim-truth promotion;
- no automatic choice between the two Mabo narratives.

## Exact execution

```sh
cargo test -p sensiblaw-pg-source-store \
  --test legal_controversy_residual_history_contract
cargo test -p sensiblaw-pg-source-store \
  --test real_matter_controversy_contract
cargo test -p sensiblaw-pg-source-store

cargo run -p sensiblaw-pg-source-store \
  --example reconcile_legal_controversy_residual_head -- \
  <controversy_ref> <successor_residual_ref> <review_command_ref> \
  <historical_residual_ref> [historical_residual_ref ...]

# Then re-run the same ignored/live Mabo REAL-MATTER gate that exposed
# PersistedMeaningMismatch.
```

The supersession command requires a genuine persisted review receipt whose `semantic_ref` is the chosen successor residual. If no such review exists, the correct state remains ambiguous/unpaid.
