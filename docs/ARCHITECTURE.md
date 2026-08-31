# SensibLaw Rust rebuild — foundation contract

This tree is a from-scratch Rust foundation derived from the current SensibLaw/DASHI contracts.
It deliberately does **not** port the legacy Python module graph one-for-one.

## Trusted split

1. **Parser worker (replaceable, currently spaCy/Python)**
   - Emits sentence-framed token observations.
   - Never writes canonical state.
   - Never promotes facts.
2. **Rust deterministic core (canonical)**
   - Owns revision-scoped `TextSpan` identity.
   - Interns parser strings into numeric symbols.
   - Resolves dependency heads only after a whole sentence is received.
   - Emits projection failures rather than repairing missing heads into self-loops.
   - Maps dependency observations to **candidate-only** semantic fragments.
3. **Admission/promotion layer**
   - Candidate -> promoted requires an explicit policy/reviewer receipt.
   - Ambiguity is retained as a fibre/set of alternatives.
   - Learned/model output cannot silently mutate token boundaries or canonical facts.

## Streaming

The worker parses independently-owned paragraph blocks sequentially and emits each complete sentence immediately (`S`, repeated `T`, then `E`) before parsing later blocks. Rust therefore processes closed deltas concurrently with later spaCy work. Canonical head projection commits only on `E`, so chunk/sentence ownership is never guessed.

## Performance invariant

Published budget for the rebuild:

`T_total <= 2 * T_spaCy_parse`

where:
- `T_spaCy_parse`: measured parser wall occupancy over the same owned text blocks.
- `T_total`: walltime from the parser-ready document marker to the final closed-delta commit, with parsing and Rust processing overlapped.
- `T_SL_active`: retained diagnostic for active Rust handler walltime; it is not a substitute for the total-time gate.

`scripts/bench_stream.py` enforces the published architectural gate. Milestones after this are 1.5x then 1.2x total; the long-term constitution is `T_post_parser <= 0.1*T_spaCy` on representative large corpora.

## Crypto-compute deployment posture

- deterministic integer IDs and first-occurrence symbol assignment in the canonical core;
- no Python objects or opaque model state in receipts;
- parser capability absence remains explicit;
- append-only receipt/state encoding is the target persistence model;
- canonical logic must be reproducible from source revision + parser observation stream + policy version;
- cryptographic digest/signature work belongs at the receipt boundary, not inside semantic inference;
- GPU/accelerator use may speed learned enrichment, but cannot acquire promotion authority.
