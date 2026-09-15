#!/usr/bin/env bash
set -euo pipefail

cd "$(dirname "$0")/.."

if [[ "${SENSIBLAW_LIVE_LEGAL_OPT_IN:-}" != "1" ]]; then
  echo "refusing live HCA judgment acquisition: export SENSIBLAW_LIVE_LEGAL_OPT_IN=1" >&2
  exit 2
fi

export SENSIBLAW_RUNTIME_HEAD="$(git rev-parse HEAD)"
export SENSIBLAW_LIVE_RECEIPT_DIR="${SENSIBLAW_LIVE_RECEIPT_DIR:-/tmp/sensiblaw-live-legal}"
export SENSIBLAW_HCA_LANDING_HTML="${SENSIBLAW_HCA_LANDING_HTML:-$SENSIBLAW_LIVE_RECEIPT_DIR/authority.html}"
export SENSIBLAW_BOUND_ACQUISITION_PLAN="${SENSIBLAW_BOUND_ACQUISITION_PLAN:-$SENSIBLAW_LIVE_RECEIPT_DIR/residual-bound-acquisition-v01.json}"

if [[ ! -f "$SENSIBLAW_HCA_LANDING_HTML" ]]; then
  echo "missing persisted HCA landing page: $SENSIBLAW_HCA_LANDING_HTML" >&2
  exit 2
fi

# The proof-search engine must first prove that this source acquisition is the
# scheduled repair for the exact live residual.  The provider runner may consume
# this permit but cannot mint it for itself.
cargo run \
  -p sensiblaw-proof-search-loop \
  --example residual_bound_hca_acquisition

if [[ ! -f "$SENSIBLAW_BOUND_ACQUISITION_PLAN" ]]; then
  echo "missing residual-bound acquisition permit: $SENSIBLAW_BOUND_ACQUISITION_PLAN" >&2
  exit 2
fi

cargo run \
  -p sensiblaw-governed-legal-provider \
  --features live-network \
  --example live_hca_judgment_docx_smoke

python3 scripts/verify_live_hca_judgment_receipt.py \
  "$SENSIBLAW_LIVE_RECEIPT_DIR/governed-official-judgment-acquisition-v01.json"

echo "LIVE HCA JUDGMENT SMOKE PASS head=$SENSIBLAW_RUNTIME_HEAD permit=$SENSIBLAW_BOUND_ACQUISITION_PLAN receipt=$SENSIBLAW_LIVE_RECEIPT_DIR/governed-official-judgment-acquisition-v01.json"
