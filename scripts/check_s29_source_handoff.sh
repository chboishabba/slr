#!/usr/bin/env bash
# S29 opt-in source-bound PG acceptance; absence of a DB is NOT PASS.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"
python3 -m py_compile scripts/s29_source_review_postgres.py tests/test_s29_source_review_postgres.py
python3 -m unittest tests/test_s29_source_review_postgres.py -v
if [ "${SLR_S29_REQUIRE_REAL_PG:-0}" = "1" ]; then
  test -n "${SLR_S29_TEST_DATABASE_URL:-}" || {
    echo "BLOCKED: SLR_S29_TEST_DATABASE_URL required" >&2
    exit 3
  }
  # The test is in the same suite; it MUST have actually been enabled.
  python3 - <<'PY'
import psycopg
print("S29 actual PostgreSQL driver:", psycopg.__version__)
PY
  echo "S29 real PostgreSQL test requested: inspect enabled unittest output for PASS"
else
  echo "S29 offline source checks only; real PostgreSQL P5 gate OPEN unless test enabled."
fi
