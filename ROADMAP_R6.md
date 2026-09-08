# SensibLaw Rust R6/R7 governed Australian research tranche

Rust remains the primary implementation track. Agda mirrors the runtime contracts and observed receipts; it does not expand the execution surface independently.

## Achieved experimentally

- Governed official HCA acquisition with explicit operator opt-in.
- First-run official landing fetch = 1 request; persisted replay = 0.
- Official HCA DOCX resource discovery from persisted landing state = 0 requests.
- Full Cullen DOCX acquisition = 1 request; persisted replay = 0.
- Deterministic DOCX -> canonical body text materialization with immutable carrier/text digests.
- Residual-bound acquisition permit for `residual:cullen-positive-operational-act` / `prop:cullen-positive-operational-duty`.
- Body-only citation observer v0.1 inadequacy observed: one self-citation candidate.
- Body + material-footnote citation observer v0.2 observed at Rust head `00bb9957ace2d3c2ea7d6104ad5cab100dd29369`:
  - network requests = 0;
  - material footnotes = 163;
  - citation candidates = 190;
  - footnote candidates = 189;
  - Mallonland `(2024) 98 ALJR 956` and `418 ALR 639` recovered at `#footnote-90`;
  - every occurrence remains unreviewed and `experimental_candidate_only`.

## Active introspective refinement: anchored citation observer v0.3

The v0.2 observer solved citation identity but exposed the next missing coordinate:

```text
footnote citation identity != body proposition that invoked that footnote
```

The v0.3 source path therefore preserves the exact WordprocessingML relation:

```text
body paragraph
  -> w:footnoteReference/@w:id
  -> footnote body
  -> citation occurrence
```

Each footnote citation candidate can now retain:

- exact footnote locator;
- anchor body-paragraph locator(s);
- anchor body-paragraph text(s);
- immutable source revision and refined observer digest.

This is observation provenance only:

```text
anchor != residual payment
anchor != proposition correspondence
anchor != CitationUse
anchor != current authority
```

The calibration target is source-grounded Cullen material around the positive-act/omission distinction. The v0.3 verifier requires Robinson `[2018] AC 736` and Modbury `(2000) 205 CLR 254` to be anchored to body context containing `positive acts in creating risk` before residual-specific review is scheduled.

## Next deterministic gate

Run:

```bash
scripts/local_ci.sh
python3 scripts/run_local_cullen_review_queue.py
```

The second command is zero-network and writes:

```text
/tmp/sensiblaw-live-legal/cullen-citation-review-queue-v03.json
```

Do not broaden citation extraction merely because more authorities exist. After v0.3 is observed, the next task is to shortlist only anchored occurrences that bear on the exact live residual and send those through the existing explicit locator-bound review gate.

## Production boundary

Experimental online acquisition is achieved. Production promotion still requires exact-current-head execution receipts where required plus matching Agda/kernel certification. Local Rust CI and runtime observation remain distinct from Agda kernel proof.
