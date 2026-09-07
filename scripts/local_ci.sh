#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo '== cargo test --workspace =='
cargo test --workspace

echo '== cargo clippy --workspace --all-targets -- -D warnings =='
cargo clippy --workspace --all-targets -- -D warnings

echo '== cargo build --release --workspace =='
cargo build --release --workspace

echo '== live-provider feature compile (no network execution) =='
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_austlii_smoke

echo '== source contracts =='
python3 scripts/verify_source_contract.py
python3 scripts/verify_semantic_status_contract.py
python3 scripts/verify_legal_counterfactual_contract.py
python3 scripts/verify_proof_search_scheduler_contract.py
python3 scripts/verify_proof_search_loop_contract.py
python3 scripts/verify_offline_research_engine_contract.py
python3 scripts/verify_online_readiness_contract.py
python3 scripts/verify_governed_legal_provider_contract.py

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
  scripts/verify_offline_research_engine_contract.py \
  scripts/verify_online_readiness_contract.py \
  scripts/verify_governed_legal_provider_contract.py \
  scripts/verify_live_legal_receipt.py

echo '== offline fixtures =='
cargo run -p sensiblaw-proof-search-scheduler --example offline_pabai
cargo run -p sensiblaw-proof-search-loop --example offline_pabai_loop
cargo run -p sensiblaw-proof-search-loop --example offline_research_engine_v01
cargo run -p sensiblaw-proof-search-loop --example offline_compounding_iteration_v02
cargo run -p sensiblaw-proof-search-loop --example offline_current_treatment_iteration_v03
cargo run -p sensiblaw-governed-legal-provider --example offline_replay_fixture

echo '== online readiness preflight =='
cargo run -p sensiblaw-proof-search-loop --example online_readiness_preflight

echo 'NOTE: live legal acquisition is opt-in and is not executed by CI.'
echo '      Run scripts/run_live_legal_smoke.sh explicitly when ready.'
echo 'LOCAL CI PASS'
