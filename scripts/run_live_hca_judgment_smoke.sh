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

if [[ ! -f "$SENSIBLAW_HCA_LANDING_HTML" ]]; then
  echo "missing persisted HCA landing page: $SENSIBLAW_HCA_LANDING_HTML" >&2
  exit 2
fi

cargo run \
  -p sensiblaw-governed-legal-provider \
  --features live-network \
  --example live_hca_judgment_docx_smoke

python3 scripts/verify_live_hca_judgment_receipt.py \
  "$SENSIBLAW_LIVE_RECEIPT_DIR/governed-official-judgment-acquisition-v01.json"

echo "LIVE HCA JUDGMENT SMOKE PASS head=$SENSIBLAW_RUNTIME_HEAD receipt=$SENSIBLAW_LIVE_RECEIPT_DIR/governed-official-judgment-acquisition-v01.json"
