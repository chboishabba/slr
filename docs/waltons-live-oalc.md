# Waltons live OALC S14 runbook

This is the operator path for the live Waltons calibration.  The native Rust
pipeline performs every deterministic acquisition/materialisation/compilation
step and stops only at four explicit gates:

1. human paragraph review;
2. sanctioned cited-by provider input;
3. human authority-identity review;
4. human citation-treatment review.

The pipeline is candidate-only throughout.  It does not create legal authority,
applicability, claim truth, liability, or a current-law conclusion.

## 0. Build the live-network CLI

```bash
cargo build -p sensiblaw-cli --features live-network --bin sensiblaw
```

Use one empty base directory for the campaign:

```bash
export WALTONS_BASE=/tmp/waltons-live
rm -rf "$WALTONS_BASE"
```

You can inspect the current gate at any time:

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live status
```

## 1. Live OALC source acquisition

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live prepare
```

This must acquire and retain Waltons Stores v Maher from OALC, validate the
immutable source receipt/text digest, materialise paragraph/citation candidates,
prepare the paragraph-review worksheet, and emit the cited-by traversal demand.

The command stops at:

```text
human_paragraph_review
```

Edit:

```text
$WALTONS_BASE/waltons-review-worksheet.json
```

For each reviewed row set `include=true` and populate the disposition,
`reviewer_ref`, and `review_evidence_refs`.  When the worksheet review is
actually finished, set the top-level:

```json
"review_complete": true
```

An untouched worksheet cannot advance the pipeline.

## 2. Compile reviewed Waltons propositions

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live paragraph-reviewed
```

This finalises the review, emits proposition receipts/payment bytes, recompiles
the WrongType frontier, compiles candidate S14 proposition hops, and reruns the
typed S14 trajectory.

The next gate is the external cited-by provider.  The pipeline deliberately does
not simulate reverse-citation traversal with full-text search.

Provider input schema:

```json
{
  "provider": "YOUR_PROVIDER",
  "operation": "CitedBy",
  "root_medium_neutral_citation": "[1988] HCA 7",
  "candidates": [
    {
      "medium_neutral_citation": "[2014] HCA 19",
      "explicit_reference": null,
      "provider_record_ref": "provider-specific-id"
    }
  ]
}
```

Provider candidates are discovery only; they are not treatment and are not
accepted as legal authority.

## 3. Re-acquire cited-by candidates through OALC

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live cited-by provider-results.json
```

Every cited-by candidate is independently re-acquired from OALC.  One failed
candidate does not abort successful candidates.  The acquisition report is:

```text
$WALTONS_BASE/later-authorities/oalc-acquisition-report.json
```

A source miss remains a source residual:

```text
missing source != negative legal evidence
```

If at least one authority resolves, the command produces:

```text
$WALTONS_BASE/waltons-authority-identity-review-worksheet.json
```

Review the proposed `document:oalc:<version-id>` to canonical
`case:<jurisdiction>:<court>:<year>:<number>` bindings.  Set `include=true`
for reviewed bindings, provide reviewer/evidence references and doctrine where
appropriate, then set:

```json
"review_complete": true
```

Raw OALC document identity never creates the canonical trace alias by itself.

## 4. Compile reviewed identities and prepare treatment review

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live identity-reviewed
```

This compiles reviewed identity hops/aliases, materialises citation occurrences
from each successfully reacquired judgment, merges the candidate treatment
queue, prepares the treatment worksheet, and reruns S14.

Edit:

```text
$WALTONS_BASE/waltons-treatment-review-worksheet.json
```

For each included review unit choose an owned anchor and populate the reviewed
citation-use/reasoning coordinates, reviewer reference and evidence references.
When complete set:

```json
"review_complete": true
```

A citation occurrence alone is not treatment.

## 5. Compile treatment, genealogy and final S14 trajectory

```bash
cargo run -p sensiblaw-cli --features live-network --bin sensiblaw -- \
  legal-follow waltons --base "$WALTONS_BASE" live treatment-reviewed
```

This emits the reviewed proposition-level treatment receipts, temporal
treatment genealogy, candidate S14 treatment edges and the final typed adaptive
trajectory.

The final state is:

```text
$WALTONS_BASE/waltons-live-pipeline-state.json
```

and the trajectory is:

```text
$WALTONS_BASE/waltons-s14-adaptive-trajectory.json
```

The final command validates every accepted hop:

```text
recompute_frontier_required = true
old_source_history_preserved = true
old_conclusions_frozen = false
creates_legal_authority = false
creates_current_law_conclusion = false
transport = typed_rust_in_process
json_is_semantic_command_transport = false
```

It also reports separate counts for bootstrap, reviewed identity, reviewed
proposition, and reviewed treatment hops.  Reviewed residuals remain in the
trajectory rather than being silently discarded.
