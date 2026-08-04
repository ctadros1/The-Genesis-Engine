#!/bin/sh
# Phase 3 benchmark set: (1) tick percentiles with 0/1/4 synthetic
# observers plus per-client bandwidth/backpressure, (2) browser render
# frame-interval sampling against a live server. Writes raw records with
# provenance under an ignored benchmarks/raw/<benchmark-id>/ directory.
set -eu

phase3_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase3_repo/scripts/phase0-env.sh"
cd "$phase3_repo"

phase3_id=${1:-phase3-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase3_output="$phase3_repo/benchmarks/raw/$phase3_id"
mkdir -p "$phase3_output"

# Provenance shared by both parts.
{
  printf '{\n'
  printf '  "benchmark_id": "%s",\n' "$phase3_id"
  printf '  "revision": "%s",\n' "$(git rev-parse HEAD 2>/dev/null || echo unborn-main)"
  printf '  "toolchain": "%s",\n' "$(rustc --version)"
  printf '  "node": "%s",\n' "$(node --version)"
  printf '  "os": "%s %s",\n' "$(sw_vers -productName)" "$(sw_vers -productVersion)"
  printf '  "cpu": "%s",\n' "$(sysctl -n machdep.cpu.brand_string)"
  printf '  "build_profile": "release-lto-thin"\n'
  printf '}\n'
} > "$phase3_output/provenance.json"

# Part 1: stream/tick benchmark (release).
LIFESIM_BENCH_OUTPUT="$phase3_output" \
  cargo test --release -p sim-server --test bench_observers -- --ignored --nocapture \
  > "$phase3_output/stream-bench.log" 2>&1

# Part 2: browser render sampling against a live server.
export LIFESIM_E2E_REST_PORT=8950
export LIFESIM_E2E_WS_PORT=8951
export LIFESIM_E2E_OBSERVER_TOKEN=bench-observer
export LIFESIM_E2E_ADMIN_TOKEN=bench-admin

LIFESIM_OBSERVER_TOKEN="$LIFESIM_E2E_OBSERVER_TOKEN" \
LIFESIM_ADMIN_TOKEN="$LIFESIM_E2E_ADMIN_TOKEN" \
target/release/lifesim-server \
  --rest-port "$LIFESIM_E2E_REST_PORT" \
  --ws-port "$LIFESIM_E2E_WS_PORT" \
  --organisms 500 \
  --speed 16 &
phase3_server_pid=$!
trap 'kill "$phase3_server_pid" 2>/dev/null || true' EXIT HUP INT TERM

phase3_attempt=0
until curl --silent --fail "http://127.0.0.1:$LIFESIM_E2E_REST_PORT/api/health" >/dev/null; do
  phase3_attempt=$((phase3_attempt + 1))
  if [ "$phase3_attempt" -ge 30 ]; then
    echo "lifesim-server did not become ready" >&2
    exit 1
  fi
  sleep 1
done
sleep 5

cd apps/observer
LIFESIM_BENCH=1 npm run test:e2e -- tests/bench.spec.ts > "$phase3_output/render-bench.log" 2>&1 || true
grep -o 'RENDER_BENCH .*' "$phase3_output/render-bench.log" \
  | sed 's/RENDER_BENCH //' > "$phase3_output/phase3-render-summary.json" || true

cd "$phase3_repo"
printf '%s\n' "$phase3_output"
