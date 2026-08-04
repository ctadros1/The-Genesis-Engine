#!/bin/sh
set -eu

phase0_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
phase0_id=${1:-phase0-local-$(date -u +%Y%m%dT%H%M%SZ)}
phase0_output="$phase0_repo/benchmarks/raw/$phase0_id"
phase0_renderer="$phase0_repo/spikes/renderer-spike"
mkdir -p "$phase0_output"

cd "$phase0_renderer"
npm run build
npm run dev -- --host 127.0.0.1 --port 4173 > "$phase0_output/vite.log" 2>&1 &
phase0_server_pid=$!
trap 'kill "$phase0_server_pid" 2>/dev/null || true' EXIT HUP INT TERM

phase0_attempt=0
until curl --silent --fail http://127.0.0.1:4173/ >/dev/null; do
  phase0_attempt=$((phase0_attempt + 1))
  if [ "$phase0_attempt" -ge 30 ]; then
    echo "renderer development server did not become ready" >&2
    exit 1
  fi
  sleep 1
done

npm run bench -- \
  --benchmark-id "$phase0_id" \
  --output "$phase0_output" \
  --url http://127.0.0.1:4173/ \
  --channel chrome \
  --warmup 30 \
  --frames 120

