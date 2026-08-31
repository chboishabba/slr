# Proof and optimisation obligations

The Rust implementation follows the published DASHI/SensibLaw runtime contracts rather than treating them as informal design suggestions.

## Certified bounded baseline

The exact Rust head `60777f637732f28fed46458a30853d35b88a8a09` earned the canonical GWB v0.1 full-run receipt:

- 10 documents / 4,073,000 projected bytes;
- 41,044 sentences / 12,742 paragraphs;
- direct/reference parity 41,044 checked / 0 failed;
- parser publication effects 0;
- exact controller/Rust sentence and paragraph accounting;
- model cold load 701,777,110 ns;
- parser wall occupancy 127,919,406,353 ns;
- SensibLaw active work 1,135,911,693 ns;
- total semantic pipeline wall 136,058,451,205 ns;
- external controller wall 136,067,579,483 ns;
- post-parser tail 7,611,429 ns;
- parser-relative ratio 1.0636263494652243x (`production_1_2x` measured tier).

This closes G3 only for that exact corpus/model/observation surface. It does not establish a universal parity theorem or universal production cutover authority.

## Numeric projection boundary

The fine runtime carrier is the full timing vector, not the gate or tier label. `sensiblaw-metrics` retains model cold-load, parser occupancy, SensibLaw-active work, total pipeline wall, external-controller wall, and post-parser tail as independent coordinates. It derives the parser-relative ratio as an exact integer numerator/denominator and derives `2.0x`, `1.5x`, and `1.2x` policy tiers without floating-point classification.

A Boolean `PASS` or three-way passing tier is a declared consumer projection only:

```text
same gate/tier != same timing decomposition
```

The crate has an executable regression with two different timing vectors that map to the same `production_1_2x` projection. Changing a result codomain from Boolean to three-valued does not by itself reconstruct erased runtime detail or create semantic authority.

## Executable architecture

- spaCy/parser output is observation evidence only.
- missing dependency heads fail rather than becoming roots.
- parser capability absence is explicit.
- sentence ownership is closed before canonical head projection.
- sentence execution uses a packed local carrier.
- sentence-local DB crossings are structurally absent.
- parser-token surrogate persistence is not part of the production path.
- closed sentence interiors are not accepted by paragraph fusion.
- child-to-parent role-count transport has an executable commuting-square regression.
- candidate generations do not become visible authority without certification followed by publication.

## Post-certification semantic expansion

`sensiblaw-semantic-expansion` is a separate candidate-only lane. It adds richer semantic-role candidates, scoped-negation residuals, unresolved modality/temporal/conditional/reference attachments, and retained alternative fibres for ambiguous clauses. It has no publication API and is not covered by the prior GWB parity/performance receipt.

## Still open

- compile/test receipt for the new metrics and semantic-expansion crates;
- production-default cutover authority across representative SensibLaw workloads;
- fresh parity for the expanded semantic observation surface;
- fresh performance receipts after semantic expansion;
- representative multi-corpus certification of the long-term `T_post-parser <= 0.1*T_spaCy` constitution;
- cryptographic receipt signing / distributed consensus publication.

Those remain gates. Absence of their receipts is not interpreted as success. CI compilation is a software validation receipt only; it does not certify the still-open semantic or performance gates.
