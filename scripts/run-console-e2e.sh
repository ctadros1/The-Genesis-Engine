#!/bin/sh
# Browser E2E: build lifesim-server, start it privately on test ports with
# fixed test tokens, run the console Playwright suite, then shut down.
# Mirrors scripts/run-observer-e2e.sh.
set -eu

console_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$console_repo/scripts/phase0-env.sh"
cd "$console_repo"

export LIFESIM_E2E_REST_PORT=${LIFESIM_E2E_REST_PORT:-8962}
export LIFESIM_E2E_WS_PORT=${LIFESIM_E2E_WS_PORT:-8963}
export LIFESIM_E2E_OBSERVER_TOKEN=${LIFESIM_E2E_OBSERVER_TOKEN:-e2e-observer}
export LIFESIM_E2E_ADMIN_TOKEN=${LIFESIM_E2E_ADMIN_TOKEN:-e2e-admin}

cargo build --release -p sim-server

# The Saves screen's Save now / Verify / Branch (C3 coverage) need a real
# snapshot store — without --data-dir the server 400s every save attempt
# ("no data dir configured"). A fresh temp directory per run keeps saves
# from one run invisible to the next.
console_data_dir=$(mktemp -d "${TMPDIR:-/tmp}/lifesim-console-e2e.XXXXXX")

LIFESIM_OBSERVER_TOKEN="$LIFESIM_E2E_OBSERVER_TOKEN" \
LIFESIM_ADMIN_TOKEN="$LIFESIM_E2E_ADMIN_TOKEN" \
target/release/lifesim-server \
  --rest-port "$LIFESIM_E2E_REST_PORT" \
  --ws-port "$LIFESIM_E2E_WS_PORT" \
  --data-dir "$console_data_dir" \
  --organisms 200 \
  --speed 4 \
  --max-worlds 10 &
# Most world-creating specs only stop what they create (so it stays
# inspectable) rather than delete it, so the registry only grows across one
# run of this suite; 10 comfortably covers every spec's creates with room
# to spare.
console_server_pid=$!
trap 'kill "$console_server_pid" 2>/dev/null || true; rm -rf "$console_data_dir"' EXIT HUP INT TERM

console_attempt=0
until curl --silent --fail "http://127.0.0.1:$LIFESIM_E2E_REST_PORT/api/health" >/dev/null; do
  console_attempt=$((console_attempt + 1))
  if [ "$console_attempt" -ge 30 ]; then
    echo "lifesim-server did not become ready" >&2
    exit 1
  fi
  sleep 1
done

cd apps/console
npm run test:e2e
