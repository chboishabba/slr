# SensibLaw Rust R6 governed-online tranche

Rust remains the primary implementation track. Agda mirrors the runtime contracts and does not expand the execution surface independently.

## Implemented

- R6a known-authority resolver: persisted/local -> explicit AustLII -> JADE exact MNC -> deterministic MNC-to-AustLII -> bounded AustLII search -> unresolved.
- R6b deterministic AustLII SINO lowering and explicit-document fetch boundary.
- R6c JADE exact/search/citation-neighbourhood planning boundary.
- R6d bounded citation-follow governance: 4-second minimum legal-host pacing, burst 1, max depth 1, max new documents 5, explicit network budget, cache/persisted-first, no scheduler HTTP.
- R6e code path for first governed fetch -> local ingestion -> immutable source revision -> same-demand persisted replay with zero network.
- Live HTTP is feature-gated behind `live-network` and additionally requires explicit `SENSIBLAW_LIVE_LEGAL_OPT_IN=1`.
- Local CI compiles the live feature but never executes network.

## Deterministic live acceptance receipt

The opt-in smoke writes `sl.governed_legal_acquisition.v0_1` containing:

- exact runtime git head;
- provider/source/proposition/MNC/explicit-reference identity;
- first-run network request count (=1);
- SHA256 identity of acquired bytes;
- immutable local source revision;
- local-ingestion receipt;
- same-demand replay network request count (=0);
- candidate-only authority boundary;
- explicit false claims for semantic payment and legal authority.

Run only after local CI passes:

```bash
export SENSIBLAW_LIVE_LEGAL_OPT_IN=1
scripts/run_live_legal_smoke.sh
```

## Current wall

Before the opt-in smoke is actually executed and its JSON receipt retained, `LiveProviderFixtureMissing` remains true. Production governed-online status additionally requires the matching exact-head Agda/kernel receipt.

Thus the expected readiness transition is:

```text
OfflineOnly
  -- successful bounded live receipt --> ExperimentalLiveAcquisitionReady
  -- exact-head Agda/kernel receipt --> ProductionGovernedOnlineReady
```
