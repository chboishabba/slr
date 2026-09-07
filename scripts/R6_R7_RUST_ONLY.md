# R6/R7 Rust-only execution rule

The governed-online and online-research-compounding implementation introduced by PR #13 is Rust-native.

- Rust owns provider policy, acquisition binding, artifact parsing, citation observation, residual shortlisting, review gating, receipt serialization, and validation.
- Shell may sequence commands but carries no research semantics.
- Python files that predate PR #13 remain legacy/unrelated machinery and are not part of the R6/R7 runtime path.
- PR #13 must not add Python source files for these capabilities.
