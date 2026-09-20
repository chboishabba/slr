# S14.5 Generic Recursive LegalFollow Campaign Runtime

Phase IV removes the case-specific orchestration shell from recursive Australian-contract reconstruction.

The completed Waltons -> Doueihi campaign remains a valid bootstrap/live receipt. After that hop, recursive continuation is owned by:

```text
ContractFollowCampaign
  -> persisted typed final trace
  -> fresh frontier
  -> outbound citation residuals
  -> bounded selector
  -> governed OALC acquisition
  -> generic authority identity review
  -> generic exact-citation treatment review
  -> reviewed-hop compiler
  -> typed delta
  -> recompute
```

No Sidhu/Giumelli/Verwayen-specific runtime module is required.

## Important semantic boundaries

- Research priority is not legal truth or authority ranking.
- A citation occurrence is a discovery residual, not citation treatment.
- OALC acquisition is candidate-only.
- Raw OALC identity does not create a canonical authority alias.
- Identity and treatment remain explicit review gates.
- Missing source is not negative legal evidence.
- JSON remains persistence/review output, not semantic command transport.
- Every accepted delta recomputes the frontier and preserves prior source history.
- Campaign budgets bound accepted hops, source acquisitions and network requests.

## Existing completed Waltons archive

The original completed Waltons archive predates persisted `final_trace`. On the updated branch, rerun only the deterministic typed sync once:

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base /tmp/waltons-live s14-sync
```

This upgrades `waltons-s14-adaptive-trajectory.json` with a resumable `final_trace`.
It does not perform a new legal review or acquire a new authority. Future campaigns produced by S14.5 already persist the trace and need no migration.

After this one compatibility migration, do not invoke Waltons-specific orchestration for recursive continuation.

## Recursive discovery from Doueihi

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign discover \
  --trajectory /tmp/waltons-live/waltons-s14-adaptive-trajectory.json \
  --source-receipt /tmp/waltons-live/later-authorities/2016-nswca-105/oalc-source-receipt.json \
  --source-semantic-ref case:nsw:nswca:2016:105 \
  --output /tmp/waltons-live/recursive/outbound-frontier.json
```

The selector filters authorities already represented by the typed trace. In the retained Doueihi source, Sidhu `[2014] HCA 19` is already represented by the old Waltons trace. The S14.5 regression therefore uses the genuinely fresh HCA authority:

```text
Giumelli v Giumelli [1999] HCA 10
```

as the recursive capstone candidate.

The selector is deterministic research scheduling only:

```text
Australian MNC
  -> court-class research priority
  -> first observed source ordinal
  -> citation lexical tie-break

priority_is_legal_truth_rank = false
```

## Bounded governed acquisition

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign acquire-next \
  --trajectory /tmp/waltons-live/waltons-s14-adaptive-trajectory.json \
  --frontier /tmp/waltons-live/recursive/outbound-frontier.json \
  --output-dir /tmp/waltons-live/recursive/giumelli
```

The selected source is independently reacquired through the governed OALC case path.

The recursive acquisition mode is now `IndexedThenPinnedStream`.  Hugging
Face Dataset Server search is an accelerator rather than the source boundary:

```text
dataset metadata / pinned revision
        ↓
bounded datasets-server /search
        ├─ exact MNC found → retain
        └─ incomplete / absent / HTTP 5xx
                        ↓
             revision-pinned corpus.jsonl stream
                        ↓
             first exact terminal-MNC row
                        ↓
                     retain
```

The pinned stream is still candidate-only.  It records
`stream_rows_examined`, `stream_bytes_read`,
`stream_terminated_after_match=true`, and
`stream_uniqueness_exhaustively_verified=false`.  The latter is deliberate:
stopping at the first exact terminal-MNC match does not pretend to prove that
no duplicate row exists later in the corpus.  Authority identity review remains
the admission gate.

A transport interruption while streaming is not source absence and is not
negative legal evidence.  Only a clean EOF with no exact row is represented as
a source residual for that pinned corpus revision.
Before the network call the controller reserves OALC's three-request worst-case
budget.  The actual request count is then charged to the cumulative campaign
budget.  A successful acquisition writes:

```text
giumelli/oalc-source-receipt.json
giumelli/authority-identity-review-worksheet.json
giumelli/campaign-after-source-acquisition.json
```

The last file is the next resumable campaign receipt.  Acquisition does not add
a legal graph hop; it advances source/network counters and sets the explicit
`AuthorityIdentityReview` operator gate.

## Generic identity gate

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign identity-prepare \
  --receipt /tmp/waltons-live/recursive/giumelli/oalc-source-receipt.json \
  --output /tmp/waltons-live/recursive/giumelli-identity-review.json
```

Review the worksheet and mark `review_complete=true`. Then:

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign identity-reviewed \
  --trajectory /tmp/waltons-live/recursive/giumelli/campaign-after-source-acquisition.json \
  --worksheet /tmp/waltons-live/recursive/giumelli/authority-identity-review-worksheet.json \
  --decisions /tmp/waltons-live/recursive/giumelli-identity-decisions.json \
  --output /tmp/waltons-live/recursive/after-giumelli-identity.json
```

The output is another resumable typed campaign receipt containing its own
`final_trace`.  The controller also uses the pending recursive coordinate to
prepare, automatically:

```text
recursive-treatment-queue.json
recursive-treatment-review-worksheet.json
```

and sets the explicit `AuthorityTreatmentReview` operator gate.

## Generic Doueihi -> Giumelli treatment gate

The manual `treatment-prepare` command remains available as a compatibility
surface, but the normal recursive path no longer needs it.  The identity stage
has already prepared exact citation review units from the retained Doueihi
primary source.

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign treatment-prepare \
  --source-receipt /tmp/waltons-live/later-authorities/2016-nswca-105/oalc-source-receipt.json \
  --source-semantic-ref case:nsw:nswca:2016:105 \
  --target-mnc '[1999] HCA 10' \
  --target-semantic-ref case:au:hca:1999:10 \
  --queue /tmp/waltons-live/recursive/doueihi-giumelli-treatment-queue.json \
  --worksheet /tmp/waltons-live/recursive/doueihi-giumelli-treatment-review.json
```

After explicit review:

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow contracts campaign treatment-reviewed \
  --trajectory /tmp/waltons-live/recursive/after-giumelli-identity.json \
  --decisions /tmp/waltons-live/recursive/doueihi-giumelli-treatment-decisions.json \
  --output /tmp/waltons-live/recursive/after-doueihi-giumelli-treatment.json
```

The compiled treatment is whatever the reviewed citation-use receipt supports.
It is not predetermined by the controller.  After the reviewed treatment delta
is accepted, the controller materialises the newly acquired target source,
recomputes its outbound citation residuals against the updated typed trace, and
writes `next-outbound-frontier.json`.  If a fresh candidate exists, the next
operator gate is `OutboundCitationAcquisition`; otherwise it is `None`.
This is the recursive loop rather than a one-shot Giumelli path.

Campaign counters and the configured budget are inherited from the parent
receipt at every continuation.  Writing a new receipt cannot reset the accepted
hop, source acquisition, or network-request budgets.

## Phase-IV acceptance criterion

Source implementation is complete when:

```text
completed typed campaign
 -> persisted final trace
 -> fresh outbound residual
 -> generic selector
 -> governed source acquisition
 -> identity review
 -> candidate authority delta
 -> fresh recompute
 -> exact source->target treatment review
 -> candidate treatment delta
 -> fresh recompute
```

and every step remains candidate-only, typed Rust in-process, bounded, and case-module independent.

The first live recursive capstone is complete only after the Giumelli source and treatment review have actually been run. The Agda capstone contract intentionally records that this live receipt is not fabricated by the source formalisation.