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
  --frontier /tmp/waltons-live/recursive/outbound-frontier.json \
  --output-dir /tmp/waltons-live/recursive/giumelli
```

The selected source is independently reacquired through the governed OALC case path.

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
  --trajectory /tmp/waltons-live/waltons-s14-adaptive-trajectory.json \
  --worksheet /tmp/waltons-live/recursive/giumelli-identity-review.json \
  --decisions /tmp/waltons-live/recursive/giumelli-identity-decisions.json \
  --output /tmp/waltons-live/recursive/after-giumelli-identity.json
```

The output is another resumable typed campaign receipt containing its own `final_trace`.

## Generic Doueihi -> Giumelli treatment gate

Prepare exact citation review units from the retained Doueihi primary source:

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
  --queue /tmp/waltons-live/recursive/doueihi-giumelli-treatment-queue.json \
  --worksheet /tmp/waltons-live/recursive/doueihi-giumelli-treatment-review.json \
  --decisions /tmp/waltons-live/recursive/doueihi-giumelli-treatment-decisions.json \
  --output /tmp/waltons-live/recursive/after-doueihi-giumelli-treatment.json
```

The compiled treatment is whatever the reviewed citation-use receipt supports. It is not predetermined by the controller.

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
