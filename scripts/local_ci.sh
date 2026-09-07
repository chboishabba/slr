#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo '== cargo test --workspace =='
cargo test --workspace

echo '== cargo clippy --workspace --all-targets -- -D warnings =='
cargo clippy --workspace --all-targets -- -D warnings

echo '== cargo build --release --workspace =='
cargo build --release --workspace

echo '== live/provider-derived feature compile (no network execution) =='
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_austlii_smoke
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_hca_judgment_docx_smoke
cargo check -p sensiblaw-proof-search-loop --example live_cullen_citation_review_queue
cargo check -p sensiblaw-proof-search-loop --example live_cullen_residual_review_shortlist

echo '== source contracts =='
python3 scripts/verify_source_contract.py
python3 scripts/verify_semantic_status_contract.py
python3 scripts/verify_legal_counterfactual_contract.py
python3 scripts/verify_proof_search_scheduler_contract.py
python3 scripts/verify_proof_search_loop_contract.py
python3 scripts/verify_offline_research_engine_contract.py
python3 scripts/verify_online_readiness_contract.py
python3 scripts/verify_governed_legal_provider_contract.py
python3 scripts/verify_provider_access_policy_contract.py
python3 scripts/verify_official_acquisition_handoff_contract.py
python3 scripts/verify_hca_resource_discovery_contract.py
python3 scripts/verify_docx_text_materialization_contract.py
python3 scripts/verify_canonical_judgment_pnf_contract.py
python3 scripts/verify_residual_bound_acquisition_contract.py
python3 scripts/verify_judgment_candidate_extraction_contract.py
python3 scripts/verify_judgment_review_gate_contract.py
python3 scripts/verify_live_cullen_review_queue_contract.py
python3 scripts/verify_live_cullen_residual_shortlist_contract.py

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
  scripts/verify_provider_access_policy_contract.py \
  scripts/verify_official_acquisition_handoff_contract.py \
  scripts/verify_hca_resource_discovery_contract.py \
  scripts/verify_docx_text_materialization_contract.py \
  scripts/verify_canonical_judgment_pnf_contract.py \
  scripts/verify_residual_bound_acquisition_contract.py \
  scripts/verify_residual_bound_acquisition_permit.py \
  scripts/verify_judgment_candidate_extraction_contract.py \
  scripts/verify_judgment_review_gate_contract.py \
  scripts/verify_live_legal_receipt.py \
  scripts/verify_live_hca_judgment_receipt.py \
  scripts/verify_live_receipt_lineage.py \
  scripts/run_local_cullen_review_queue.py \
  scripts/verify_live_cullen_review_queue.py \
  scripts/verify_live_cullen_review_queue_contract.py \
  scripts/run_local_cullen_residual_shortlist.py \
  scripts/verify_live_cullen_residual_shortlist.py \
  scripts/verify_live_cullen_residual_shortlist_contract.py

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
python3 scripts/verify_residual_bound_acquisition_permit.py \
  /tmp/sensiblaw-live-legal/residual-bound-acquisition-v01.json
cargo run -p sensiblaw-proof-search-loop --example judgment_citation_candidates
cargo run -p sensiblaw-proof-search-loop --example reviewed_judgment_edge
cargo run -p sensiblaw-proof-search-loop --example official_acquisition_compounding

echo '== online readiness preflight =='
cargo run -p sensiblaw-proof-search-loop --example online_readiness_preflight

echo 'NOTE: experimental online acquisition is already achieved.'
echo '      Legal-host pacing remains self-capped at 1 request / 4 seconds, burst 1 unless'
echo '      a provider publishes a stricter requirement. Missing published numeric limits'
echo '      never mean unlimited access. HCA/FCA remain bounded/cache-first; OALC is'
echo '      bulk-snapshot/local-first; AustLII/JADE are not default live lanes.'
echo '      Anchored Cullen queue v0.3 is locally validated at fc5aeec...; the next semantic'
echo '      step is the zero-network residual review shortlist, then explicit reviewed edges.'
echo '      Exact-current-head live execution and Agda/kernel receipts remain separate.'
echo 'LOCAL CI PASS'
