#!/usr/bin/env bash
# Read-only reopening of the published endpoint's actual persisted relation
# packet; no source repair, direct staging, parser, identity or authority work.
set -euo pipefail
if [[ $# -lt 1 || $# -gt 2 ]]; then
  echo "usage: $0 PUBLISHED_ENDPOINT_JSON [INDEPENDENT_REFERENCE_JSON]" >&2
  exit 2
fi
endpoint=$(realpath "$1")
reference=""
if [[ $# == 2 ]]; then reference=$(realpath "$2"); fi
cd "$(git rev-parse --show-toplevel)"
command -v jq >/dev/null
command -v timeout >/dev/null
command -v sha256sum >/dev/null
: "${SENSIBLAW_DATABASE_URL:?original numeric database required}"
checkpoint="${SLR_NATIVE_RELATION_REOPEN_CHECKPOINT:-.native-v8-relation-reopen}"
mkdir -p "$checkpoint"
bounded() { timeout --kill-after=5s "${SLR_NATIVE_STEP_TIMEOUT:-90s}" "$@"; }
bounded cargo test -p sensiblaw-pg-source-store --lib numeric_pnf >&2
bounded cargo test -p sensiblaw-cli numeric_pnf_native >&2
bounded cargo build -p sensiblaw-cli >&2
cli=(target/debug/sensiblaw numeric-pnf-native)
run=$(jq -er '.binding_passes[-1].numeric_run_ref' "$endpoint")
doc=$(jq -er '.source_local_answer.source_document_ref' "$endpoint")
snapshot=$(jq -er '.binding_passes[-1].native_binding_bridge.snapshot_ref' "$endpoint")
digest=$(jq -er '.binding_passes[-1].native_binding_bridge.original_relation_packet.packet_sha256' "$endpoint")
head=$(git rev-parse HEAD)
binary=$(sha256sum target/debug/sensiblaw | cut -d' ' -f1)
endpoint_sha=$(sha256sum "$endpoint" | cut -d' ' -f1)
reference_sha=absent
if [[ -n "$reference" ]]; then reference_sha=$(sha256sum "$reference" | cut -d' ' -f1); fi
manifest=$(jq -cn --arg head "$head" --arg binary "$binary" --arg endpoint "$endpoint_sha" --arg reference "$reference_sha" --arg run "$run" --arg doc "$doc" --arg snapshot "$snapshot" --arg digest "$digest"  '{head:$head,binary_sha256:$binary,endpoint_sha256:$endpoint,reference_sha256:$reference,numeric_run_ref:$run,document_ref:$doc,snapshot_ref:$snapshot,packet_sha256:$digest}')
if [[ -f "$checkpoint/manifest.json" ]]; then
  jq -e --argjson m "$manifest" '.==$m' "$checkpoint/manifest.json" >/dev/null
else
  printf '%s\n' "$manifest" > "$checkpoint/manifest.json.tmp"
  mv "$checkpoint/manifest.json.tmp" "$checkpoint/manifest.json"
fi
bounded "${cli[@]}" relation-reopen "$run" "$doc" "$snapshot" "$digest" > "$checkpoint/reopened.json.tmp"
jq -e --arg digest "$digest" --arg snapshot "$snapshot" '
  .schema=="slr-reopened-original-relation-packet-v1" and
  .packet_sha256==$digest and .snapshot_ref==$snapshot and
  .exact_persisted_packet_readback==true and .writes_database==false and
  .current_snapshot_certified==false and .typed_coverage_paid==false and
  .governed_identity_paid==false and .whole_document_parity_paid==false
' "$checkpoint/reopened.json.tmp" >/dev/null
mv "$checkpoint/reopened.json.tmp" "$checkpoint/reopened.json"
if [[ -n "$reference" ]]; then
  bounded "${cli[@]}" relation-compare "$checkpoint/reopened.json" "$reference" > "$checkpoint/comparison.json.tmp"
  jq -e '.schema=="slr-native-original-relation-comparison-v1" and
    .independent_producer_authenticated==false and .writes_database==false and
    .typed_coverage_paid==false and .governed_identity_paid==false' "$checkpoint/comparison.json.tmp" >/dev/null
  [[ "$(sha256sum "$reference" | cut -d' ' -f1)" == "$reference_sha" ]]
  mv "$checkpoint/comparison.json.tmp" "$checkpoint/comparison.json"
fi
[[ "$(sha256sum "$endpoint" | cut -d' ' -f1)" == "$endpoint_sha" ]]
[[ "$(sha256sum target/debug/sensiblaw | cut -d' ' -f1)" == "$binary" ]]
echo "Reopened exact persisted packet at $checkpoint/reopened.json"
if [[ -n "$reference" ]]; then echo "Comparison receipt: $checkpoint/comparison.json"; fi
