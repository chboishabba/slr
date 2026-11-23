# C4.11: source-native antecedent review on original SLR S29

Date: 2026-11-23. Owner: original SLR/Rust/PostgreSQL/S29. Source of
coordinates: dashi_lean4 C4.10 same-source gate; not an identity authority.
Target: GWB corpus original sentence 22 Bush [4310,4314) and sentence
23 he [4534,4536).

## Reused owners

- Original SLR ingest.generic_source_revision and corpus.canonical_content:
  immutable revision and canonical UTF-8 document content.
- Original SLR ingest.long_document_region and corpus.source_statement:
  original region and statement source coordinates.
- Original SLR S29 review_workstation_store: review item, review commands,
  status, receipt, durable replay.
- Existing sl-semantic-status AntecedentResolutionReceipt and
  IdentityResolutionReceipt are conceptually distinct from workflow
  review and external Wikidata identity.
- SLR #78 source-native parser-evidence re-entry; this new adapter is
  attached to that branch rather than the old SensibLaw Python
  identity-prototype numeric tables.

## New limited adapter

`crates/sl-pg-source-store/src/native_antecedent_review.rs` consumes the
original C4.10 receipt. The Rust CLI accepts it with:

```sh
cargo run -p sensiblaw-cli -- \
  native-antecedent-review prepare \
  /path/to/dashi_lean4/.dashi/gwb-relation/native-he-antecedent-span-candidate-v1.json
```

The adapter rejects absent, foreign, shifted, promoted, malformed and
future-source candidates, checks the actual canonical document digest
and both original persisted sentences directly from PostgreSQL,
then creates an exact immutable candidate sidecar in
`semantic.native_antecedent_candidate_review` and queues a
`ReviewItemKind::Observation` in the **original S29** review store.
It includes the candidate's original context digest, exact
revision/document/statement/region and absolute Unicode character
coordinates. The S29 owner alone controls mutable review states.

S29 action `Accept` must **not** directly promote identity. It only
establishes that the proposed review item was accepted for review-workflow
purposes. To pay antecedent/coreference identity requires an independent,
governed resolution witness whose original source accessibility,
candidate set, exclusions, reviewer authority, exact premise/mention
references and decision receipt are validated. No such native identity
admission is implemented or claimed in this C4.11 packet.

## Operator status

Previous work (operator-reported and confirmed source tree):
- C4.09 native original source reopen passed (4 before, 2 after);
- P10 minimal Q207 JMD kernel marker `GWB_Q207_MINIMAL_ACCEPTED`;
- C4.10 source-span gate passed for Bush -> he geometry;
- C4.10 local tests 14/14 PASS, prior 40/40 PASS.

C4.11 code is **SOURCE-WRITTEN / NOT CARGO-COMPILED / NO LIVE S29
PERSISTENCE RECEIPT** in this connector environment. Run locally:

```sh
cargo test -p sensiblaw-pg-source-store native_antecedent_review
cargo test -p sensiblaw-cli native_antecedent_review
cargo run -p sensiblaw-cli -- native-antecedent-review prepare \
  /path/to/dashi_lean4/.dashi/gwb-relation/native-he-antecedent-span-candidate-v1.json
```

The positive output is a **queued S29 review proposal**, not
`antecedent_resolution=RESOLVED`. The original C4.10 local JSON file
was not checked into GitHub, so execution against that exact record
must occur in the original operator environment. No CI/TAR or source
reparse is required.

## P0 next gate

Inspect and independently reopen the six original statements and the
candidate alternatives; apply a governed S29 review command with a
named reviewer and evidence. Then, only if there is an independently
validated semantic anaphor-resolution contract with explicit
scope/competition evidence, materialize a separate local antecedent
identity witness (NOT an external QID). Wikidata linking, claim truth,
and article equivalence are separate obligations.
