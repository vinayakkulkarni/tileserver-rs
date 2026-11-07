#!/usr/bin/env bash
# Load the benchmark PostGIS fixtures (tables, functions) into $DATABASE_URL.
# tests/postgis_live.rs and the postgres_sources integration tests read them.
set -euo pipefail
for _ in $(seq 1 30); do
  pg_isready -d "$DATABASE_URL" >/dev/null 2>&1 && break
  sleep 2
done
for f in benchmarks/postgres/init.sql benchmarks/postgres/init-ogc.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -q -f "$f"
done
