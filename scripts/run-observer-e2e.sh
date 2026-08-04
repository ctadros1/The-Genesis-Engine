#!/bin/sh
# Browser E2E: build lifesim-server, start it privately on test ports with
# fixed test tokens, run the observer Playwright suite, then shut down.
set -eu

phase3_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$phase3_repo/scripts/phase0-env.sh"
cd "$phase3_repo"

export LIFESIM_E2E_REST_PORT=${LIFESIM_E2E_REST_PORT:-8950}
export LIFESIM_E2E_WS_PORT=${LIFESIM_E2E_WS_PORT:-8951}
export LIFESIM_E2E_OBSERVER_TOKEN=${LIFESIM_E2E_OBSERVER_TOKEN:-e2e-observer}
export LIFESIM_E2E_ADMIN_TOKEN=${LIFESIM_E2E_ADMIN_TOKEN:-e2e-admin}

cargo build --release -p sim-server

LIFESIM_OBSERVER_TOKEN="$LIFESIM_E2E_OBSERVER_TOKEN" \
LIFESIM_ADMIN_TOKEN="$LIFESIM_E2E_ADMIN_TOKEN" \
target/release/lifesim-server \
  --rest-port "$LIFESIM_E2E_REST_PORT" \
  --ws-port "$LIFESIM_E2E_WS_PORT" \
  --organisms 300 \
  --speed 8 &
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

cd apps/observer
npm run test:e2e
