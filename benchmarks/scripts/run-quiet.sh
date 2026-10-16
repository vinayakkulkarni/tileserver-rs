#!/usr/bin/env bash
# Run the benchmark suite only when the host is quiet.
#
# Before each benchmark group the script waits until, for one full sample
# window, ALL of these hold:
#   - non-Docker processes use less than MAX_HOST_CPU % CPU in total
#   - the benchmark containers are idle (less than MAX_CONTAINER_CPU % total)
#   - macOS reports at least MIN_FREE_MEM % memory free
# It records the measured load before and after each group, so a result can
# be traced back to the machine state it was measured in.
#
# Usage: benchmarks/scripts/run-quiet.sh [type ...]
#   default types: pmtiles mbtiles postgres postgres_function cog dem raster ogc stac compression
#   BENCH_ARGS="--mode grid" runs viewport-grid mode for the given types
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# 120 = 10% of a 12-core machine. An idle macOS desktop (WindowServer,
# terminal, kernel_task) already sums to 60-90%, so a lower default never
# passes; a build or tile job (several hundred %) still blocks the run.
MAX_HOST_CPU="${MAX_HOST_CPU:-120}"         # percent of one core, summed
MAX_CONTAINER_CPU="${MAX_CONTAINER_CPU:-20}"
MIN_FREE_MEM="${MIN_FREE_MEM:-25}"
WINDOW="${WINDOW:-15}"                      # seconds per CPU sample
MAX_WAIT="${MAX_WAIT:-7200}"                # give up after this many seconds

TYPES=("$@")
[ ${#TYPES[@]} -eq 0 ] && TYPES=(pmtiles mbtiles postgres postgres_function cog dem raster ogc stac compression)

STAMP="$(date +%Y-%m-%dT%H%M%S)"
OUT="$ROOT/results/$STAMP"
mkdir -p "$OUT"
LOG="$OUT/stability.log"

host_cpu() {
  # Second `top` sample is the accurate one; first is since-boot average.
  top -l 2 -s "$WINDOW" -stats cpu,command -o cpu -n 40 \
    | awk 'BEGIN{n=0} /^%CPU/{n++; next} n==2 && NF>=2 {
        cmd=$0; sub(/^[ ]*[0-9.]+[ ]+/, "", cmd); sub(/[ ]+$/, "", cmd)
        # top truncates names to 16 chars ("com.apple.Virtua"). The Docker VM
        # runs the benchmark servers, so it is measured by container_cpu.
        if (cmd ~ /Virtua|com\.docker|^top$/) next
        s+=$1; if ($1>5) top=top sprintf("%s=%s%% ", cmd, $1)
      } END{printf "%.0f|%s\n", s, top}'
}

container_cpu() {
  docker stats --no-stream --format '{{.Name}} {{.CPUPerc}}' \
    | awk '$1 ~ /^benchmarks-/ { gsub(/%/, "", $2); s+=$2 } END{printf "%.0f\n", s}'
}

free_mem() {
  memory_pressure 2>/dev/null | awk -F': ' '/free percentage/{gsub(/%/, "", $2); print int($2)}'
}

snapshot() { echo "load=$(sysctl -n vm.loadavg | tr -d '{}' | xargs) host_cpu=$1 containers=$2 free_mem=$3"; }

wait_for_quiet() {
  local label="$1" start now h hc c m
  start=$(date +%s)
  while :; do
    h="$(host_cpu)"; hc="${h%%|*}"; c="$(container_cpu)"; m="$(free_mem)"
    if [ "$hc" -lt "$MAX_HOST_CPU" ] && [ "$c" -lt "$MAX_CONTAINER_CPU" ] && [ "${m:-0}" -ge "$MIN_FREE_MEM" ]; then
      echo "$(date +%T) QUIET  before $label: $(snapshot "$hc" "$c" "$m")" | tee -a "$LOG"
      return 0
    fi
    echo "$(date +%T) NOISY  before $label: $(snapshot "$hc" "$c" "$m") busy: ${h#*|}" | tee -a "$LOG"
    now=$(date +%s)
    if [ $((now - start)) -ge "$MAX_WAIT" ]; then
      echo "$(date +%T) GAVE UP waiting for a quiet host before $label" | tee -a "$LOG"
      return 1
    fi
  done
}

for t in "${TYPES[@]}"; do
  wait_for_quiet "$t" || exit 1
  rc=0
  # BENCH_ARGS passes extra harness flags, e.g. BENCH_ARGS="--mode grid".
  # shellcheck disable=SC2086
  ( cd "$ROOT" && node run-benchmarks.js --type "$t" ${BENCH_ARGS:-} ) > "$OUT/$t.log" 2>&1 || rc=$?
  h="$(host_cpu)"
  echo "$(date +%T) DONE   $t (exit=$rc): $(snapshot "${h%%|*}" "-" "$(free_mem)") busy: ${h#*|}" | tee -a "$LOG"
done
echo "results: $OUT"
