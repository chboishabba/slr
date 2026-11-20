# Original relation candidate row export

2026-11-20. Parent fa175b51 is operator-certified; this extension is source-written, not yet Cargo-tested.

Use the existing expanded-cert stdin token protocol with C parity=1 and --export-relations. No parser invocation is required. Stdout emits R TSV rows: revision ID, sentence ID, local ordinal, start char, end char, kind, head (root or ordinal), candidate_only, context_resolution_required. Default output and metrics remain unchanged. Consumers must reject nonzero exit, failed parity, missing or duplicated rows and mismatches to original source. Numeric revision is stream-local; bind it externally to the immutable source snapshot. No semantic admission or publication. Next: replay pinned single-article archived observations, validate all rows in Lean, inspect by/to edges; then governed identity/antecedent refinement. No CI changes.
