#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

echo '== cargo test --workspace =='
cargo test --workspace

echo '== cargo clippy --workspace --all-targets -- -D warnings =='
cargo clippy --workspace --all-targets -- -D warnings

echo '== cargo build --release --workspace =='
cargo build --release --workspace

echo '== governed-online feature compile (Rust only; no network execution) =='
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_austlii_smoke
cargo check -p sensiblaw-governed-legal-provider --features live-network --example live_hca_judgment_docx_smoke
cargo check -p sensiblaw-proof-search-loop --example live_cullen_citation_review_queue
cargo check -p sensiblaw-proof-search-loop --example live_cullen_residual_review_shortlist
cargo check -p sensiblaw-proof-search-loop --example cullen_source_grounded_review_units

echo '== pre-existing source contracts =='
python3 scripts/verify_source_contract.py
python3 scripts/verify_semantic_status_contract.py
python3 scripts/verify_legal_counterfactual_contract.py
python3 scripts/verify_proof_search_scheduler_contract.py
python3 scripts/verify_proof_search_loop_contract.py
python3 scripts/verify_offline_research_engine_contract.py
python3 scripts/verify_online_readiness_contract.py

echo '== pre-existing Python syntax checks =='
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
  scripts/verify_online_readiness_contract.py

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
cargo run -p sensiblaw-proof-search-loop --example judgment_citation_candidates
cargo run -p sensiblaw-proof-search-loop --example reviewed_judgment_edge
cargo run -p sensiblaw-proof-search-loop --example official_acquisition_compounding
cargo run -p sensiblaw-proof-search-loop --example cullen_source_grounded_review_units

echo '== online readiness preflight =='
cargo run -p sensiblaw-proof-search-loop --example online_readiness_preflight

LIVE_DIR="${SENSIBLAW_LIVE_RECEIPT_DIR:-/tmp/sensiblaw-live-legal}"
LIVE_RECEIPT="${SENSIBLAW_HCA_JUDGMENT_RECEIPT:-$LIVE_DIR/governed-official-judgment-acquisition-v01.json}"
LIVE_DOCX="${SENSIBLAW_JUDGMENT_DOCX:-$LIVE_DIR/judgment.docx}"
if [[ -f "$LIVE_RECEIPT" && -f "$LIVE_DOCX" ]]; then
  echo '== retained Cullen artifacts (Rust-only, zero network) =='
  cargo run -p sensiblaw-proof-search-loop --example live_cullen_citation_review_queue -- \
    "$LIVE_RECEIPT" "$LIVE_DOCX" "$LIVE_DIR/cullen-citation-review-queue-v03.json"
  cargo run -p sensiblaw-proof-search-loop --example live_cullen_residual_review_shortlist -- \
    "$LIVE_RECEIPT" "$LIVE_DOCX" "$LIVE_DIR/cullen-positive-operational-act-shortlist-v01.json"
else
  echo '== retained Cullen artifacts unavailable; skipping optional zero-network replay =='
fi

echo 'NOTE: PR #13 governed-online/R7 additions are Rust-native.'
echo '      Python invoked above is pre-existing repository machinery from the PR base,'
echo '      not part of the governed-online runtime or its new validation semantics.'
echo '      Experimental online acquisition is already achieved; exact-current-head live'
echo '      execution and Agda/kernel certification remain separate production gates.'
echo 'LOCAL CI PASS'
