#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo '== cargo test --workspace =='
cargo test --workspace

echo '== cargo clippy --workspace --all-targets -- -D warnings =='
cargo clippy --workspace --all-targets -- -D warnings

echo '== cargo build --release --workspace =='
cargo build --release --workspace

echo '== source contracts =='
python3 scripts/verify_source_contract.py
python3 scripts/verify_semantic_status_contract.py
python3 scripts/verify_legal_counterfactual_contract.py
python3 scripts/verify_proof_search_scheduler_contract.py
python3 scripts/verify_proof_search_loop_contract.py
python3 scripts/verify_offline_research_engine_contract.py

echo '== Python syntax checks =='
python3 -m py_compile \
  python/spacy_stream.py \
  python/gwb_tranche.py \
  python/gwb_prepare.py \
  python/gwb_full_run.py \
  python/gwb_certify.py \
  python/gwb_expanded_certify.py \
  scripts/bench_stream.py \
  scripts/verify_source_contract.py \
  scripts/verify_semantic_status_contract.py \
  scripts/verify_legal_counterfactual_contract.py \
  scripts/verify_proof_search_scheduler_contract.py \
  scripts/verify_proof_search_loop_contract.py \
  scripts/verify_offline_research_engine_contract.py

echo '== offline fixtures =='
cargo run -p sensiblaw-proof-search-scheduler --example offline_pabai
cargo run -p sensiblaw-proof-search-loop --example offline_pabai_loop
cargo run -p sensiblaw-proof-search-loop --example offline_research_engine_v01
cargo run -p sensiblaw-proof-search-loop --example offline_compounding_iteration_v02
cargo run -p sensiblaw-proof-search-loop --example offline_current_treatment_iteration_v03

echo '== online readiness preflight =='
cargo run -p sensiblaw-proof-search-loop --example online_readiness_preflight

echo 'LOCAL CI PASS'
