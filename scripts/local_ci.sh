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
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_hca_judgment_docx_smoke

echo '== source contracts =='
python3 scripts/verify_source_contract.py
python3 scripts/verify_semantic_status_contract.py
python3 scripts/verify_legal_counterfactual_contract.py
python3 scripts/verify_proof_search_scheduler_contract.py
python3 scripts/verify_proof_search_loop_contract.py
python3 scripts/verify_offline_research_engine_contract.py
python3 scripts/verify_online_readiness_contract.py
python3 scripts/verify_governed_legal_provider_contract.py
python3 scripts/verify_official_acquisition_handoff_contract.py
python3 scripts/verify_hca_resource_discovery_contract.py
python3 scripts/verify_docx_text_materialization_contract.py
python3 scripts/verify_canonical_judgment_pnf_contract.py
python3 scripts/verify_residual_bound_acquisition_contract.py

echo '== live receipt lineage (no network) =='
python3 scripts/verify_live_receipt_lineage.py \
  --receipt-head 9c3007be97f7e4a1e9a8bc9c7c85b92368515935 \
  --validated-head bb6de859ca82700cba70d2784f11c39a2c4c1826 \
  --output /tmp/sensiblaw-live-legal/live-receipt-lineage-v01.json

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
  scripts/verify_official_acquisition_handoff_contract.py \
  scripts/verify_hca_resource_discovery_contract.py \
  scripts/verify_docx_text_materialization_contract.py \
  scripts/verify_canonical_judgment_pnf_contract.py \
  scripts/verify_residual_bound_acquisition_contract.py \
  scripts/verify_live_legal_receipt.py \
  scripts/verify_live_hca_judgment_receipt.py \
  scripts/verify_live_receipt_lineage.py

echo '== offline fixtures =='
cargo run -p sensiblaw-proof-search-scheduler --example offline_pabai
cargo run -p sensiblaw-proof-search-loop --example offline_pabai_loop
cargo run -p sensiblaw-proof-search-loop --example offline_research_engine_v01
cargo run -p sensiblaw-proof-search-loop --example offline_compounding_iteration_v02
cargo run -p sensiblaw-proof-search-loop --example offline_current_treatment_iteration_v03
cargo run -p sensiblaw-governed-legal-provider --example oalc_jsonl_index
cargo run -p sensiblaw-governed-legal-provider --example offline_replay_fixture
cargo run -p sensiblaw-governed-legal-provider --example hca_landing_resource_discovery
cargo run -p sensiblaw-governed-legal-provider --example docx_text_materialization
cargo run -p sensiblaw-proof-search-loop --example canonical_judgment_pnf_bridge
cargo run -p sensiblaw-proof-search-loop --example residual_bound_hca_acquisition
cargo run -p sensiblaw-proof-search-loop --example official_acquisition_compounding

echo '== online readiness preflight =='
cargo run -p sensiblaw-proof-search-loop --example online_readiness_preflight

echo 'NOTE: bounded official HCA landing acquisition has been validated experimentally.'
echo '      Resource discovery is zero-network and prefers the official DOCX for PNF.'
echo '      DOCX -> canonical text -> existing PNF bridge is validated offline before live use.'
echo '      Governed research acquisition must bind to the exact open residual/proposition/producer.'
echo '      The next opt-in step is scripts/run_live_hca_judgment_smoke.sh, which reuses'
echo '      the persisted landing page and spends at most one request on the DOCX.'
echo '      Exact-current-head live execution and Agda/kernel receipts remain separate.'
echo 'LOCAL CI PASS'
