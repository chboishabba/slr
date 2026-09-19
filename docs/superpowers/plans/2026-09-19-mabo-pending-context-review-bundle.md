# Mabo pending context review bundle implementation plan

**Goal:** make each adaptive `ContextReviewRequired` stop emit a stable, auditable review bundle without weakening the explicit-review firewall.

## Constraints

- Do not auto-review.
- Do not convert graph adjacency into semantics.
- Do not change the bounded Wikidata property surface.
- Do not change campaign scheduling or target accounting.
- Preserve the existing context review manifest ABI:
  `source_revision_ref<TAB>candidate_set_sha256<TAB>review_ref`.

## Implementation

1. Add a pure formatter in `adaptive_context_review.rs` that:
   - validates the exact source revision/QID binding;
   - recomputes the deterministic bounded candidate-set digest;
   - sorts and deduplicates candidate rows;
   - emits only commented lines;
   - includes a commented manifest template.

2. Add a regression proving:
   - bundle output is deterministic under input permutation;
   - the review parser returns zero assignments for the pending bundle;
   - digest/revision/candidate coordinates are present.

3. On `ContextReviewRequired`, write the pending bundle to:
   `artifacts/mabo/context-reviews/pending/<source_revision>.pending.tsv`
   while retaining the existing stdout review template.

## Verification

Run locally on the branch:

```sh
cargo test -p sensiblaw-world-expansion-runtime --test mabo_context_review_manifest
cargo test -p sensiblaw-world-expansion-runtime
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p sensiblaw-world-expansion-runtime --example mabo_100hop_recurrent_campaign -- fixtures/mabo_reviewed_identity_manifest.tsv
```

Expected stop remains `ContextReviewRequired`; the new observable is `context_review_bundle_path=...`.

No GREEN claim is made until those commands run at the exact branch head.
