# Legal semantic adequacy and admission frontier

This tranche begins after bounded expanded GWB v0.2 certification. Runtime feasibility and implementation parity are no longer the primary uncertainty. The live question is whether a parser-supported candidate is semantically adequate and sufficiently resolved/evidenced to become an admitted local normative delta.

## Gold conformance surface

`fixtures/legal_semantic_conformance_v0_1.tsv` is intentionally small and exact. Covered rows specify the complete expected consumer object for one sentence-local fibre: candidate kind, scope state, residual kind, alternative fibre, source span/address and head locality. `unresolved` is a legitimate expected result.

The same file keeps producer gaps explicit rather than pretending the carrier is complete. v0.1 currently names six gaps:

- Action/predicate
- Exception
- Jurisdiction
- Speaker
- Evidence
- Provenance

A passing gold test proves only conformance to these declared fixtures. It is not universal legal-semantic correctness.

## Candidate to admission

`sensiblaw-semantic-admission` requires an `AdmissionReceipt` matched to the exact stable candidate address, source span and semantic kind. The receipt also carries a non-parser authority class, a resolved-scope class, policy reference and resolver reference.

Scope resolution is exact:

- `SyntacticallyLocal` -> `LocalSyntactic`
- `ScopeUnresolved` -> `ScopeResolved`
- `AttachmentUnresolved` -> `AttachmentResolved`
- `ContextRequired` -> `ContextResolved`

No receipt means no admission. A bad receipt means no admission. In both cases the candidate remains retained and residual/alternative evidence is preserved.

There is deliberately no parser-authority variant, durable authority-id materialization, generation certification or publication API in this crate.

## Residual frontier

The expanded certification binary now emits one count for each typed residual kind. The v0.3 receipt requires:

- all eight residual kinds present;
- histogram sum equals the declared residual total;
- parity/direct-only passes have the same histogram;
- canonical parser-observation identity still matches;
- expanded direct/reference parity still holds;
- publication effects remain zero;
- direct-only performance still satisfies the 2x architectural gate.

Residual-frequency ranking is kept separate from semantic quality. `rank_residual_frontier` accepts external `legal_importance` and `resolvability` weights and computes a work-selection score only. High frequency does not imply high importance, correctness, confidence or authority.

## Static validation

```sh
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo build --release --workspace
python3 scripts/verify_source_contract.py
python3 -m py_compile python/gwb_expanded_certify.py
```

## Full residual-frontier run

Reuse the existing GWB v0.1 projection manifest:

```sh
python3 python/gwb_expanded_certify.py \
  --manifest /tmp/opencode/gwb-rust/source_projection.json \
  --rust-bin target/release/sensiblaw-expanded-cert \
  --model en_core_web_sm \
  --output /tmp/opencode/gwb-rust/expanded-semantic-certification-v03.json
```

Useful terminal lines include:

```text
GWB_EXPANDED_PARITY ...
GWB_EXPANDED_OBSERVATION ...
GWB_EXPANDED_RESIDUAL kind=... count=...
GWB_EXPANDED_DIRECT ...
GWB_EXPANDED_CERTIFICATION PASS ...
```

The residual histogram from this run is the input to the next producer-selection decision. Do not optimize for the smallest total residual number; choose the next semantic producer using measured frequency together with explicit legal importance, resolvability and gold-conformance evidence.
