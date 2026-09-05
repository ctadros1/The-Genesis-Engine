#!/bin/sh
# Developer instance (D1): builds the server and the console, starts both
# against a persistent local data directory, and prints how to reach them
# through the SSH alias's local port-forward. Stays up until stopped
# (Ctrl-C or a signal), then tears both processes down. Mirrors
# scripts/run-observer-e2e.sh's shape (POSIX sh, trap-based cleanup).
set -eu

console_repo=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
. "$console_repo/scripts/phase0-env.sh"
cd "$console_repo"

data_dir="$HOME/console-dev-data"
tokens_file="$data_dir/tokens.env"
mkdir -p "$data_dir"

rest_port=${CONSOLE_REST_PORT:-8960}
ws_port=${CONSOLE_WS_PORT:-8961}
ui_port=${CONSOLE_UI_PORT:-5280}
# Loopback by default (docs/28: Caddy is the browser-facing boundary). Set
# CONSOLE_BIND to 0.0.0.0 or a LAN address to reach the developer instance
# without the SSH tunnel; the bearer tokens then travel in the clear on the
# private LAN, which is the documented posture for a developer instance only.
bind=${CONSOLE_BIND:-127.0.0.1}
case "$bind" in
  0.0.0.0) advertise=$(hostname -I 2>/dev/null | awk '{print $1}'); [ -n "$advertise" ] || advertise=127.0.0.1 ;;
  *) advertise=$bind ;;
esac

# Tokens: an operator who supplies both env vars keeps full control and sees
# no output about them here (they already know their own values). Otherwise
# reuse whatever a previous run generated and saved, or generate fresh ones
# now and save them for next time.
tokens_generated_or_loaded=0
if [ -z "${LIFESIM_OBSERVER_TOKEN:-}" ] || [ -z "${LIFESIM_ADMIN_TOKEN:-}" ]; then
  if [ -f "$tokens_file" ]; then
    . "$tokens_file"
  fi
  if [ -z "${LIFESIM_OBSERVER_TOKEN:-}" ]; then
    LIFESIM_OBSERVER_TOKEN=$(openssl rand -hex 24)
  fi
  if [ -z "${LIFESIM_ADMIN_TOKEN:-}" ]; then
    LIFESIM_ADMIN_TOKEN=$(openssl rand -hex 24)
  fi
  {
    echo "LIFESIM_OBSERVER_TOKEN=$LIFESIM_OBSERVER_TOKEN"
    echo "LIFESIM_ADMIN_TOKEN=$LIFESIM_ADMIN_TOKEN"
  } > "$tokens_file"
  chmod 600 "$tokens_file"
  tokens_generated_or_loaded=1
fi
export LIFESIM_OBSERVER_TOKEN
export LIFESIM_ADMIN_TOKEN

echo "Building lifesim-server..."
cargo build --release -p sim-server

echo "Building the console..."
(cd apps/console && npm ci && npm run build)

target/release/lifesim-server \
  --bind "$bind" \
  --rest-port "$rest_port" \
  --ws-port "$ws_port" \
  --data-dir "$data_dir" \
  --max-worlds 8 \
  --organisms 300 \
  --speed 4 &
server_pid=$!

# Run the local vite binary directly (not via `npx`/`npm exec`, which
# spawns vite as an unmanaged grandchild that a plain `kill` of the wrapper
# doesn't reach); `exec` replaces the subshell itself with vite's process,
# so $ui_pid names the real server and a signal to it always stops it.
(cd apps/console && exec ./node_modules/.bin/vite preview --host "$bind" --port "$ui_port") &
ui_pid=$!

cleanup() {
  kill "$server_pid" 2>/dev/null || true
  kill "$ui_pid" 2>/dev/null || true
}
trap cleanup EXIT HUP INT TERM

dev_attempt=0
until curl --silent --fail "http://$advertise:$rest_port/api/health" >/dev/null; do
  dev_attempt=$((dev_attempt + 1))
  if [ "$dev_attempt" -ge 30 ]; then
    echo "lifesim-server did not become ready" >&2
    exit 1
  fi
  sleep 1
done

if [ "$tokens_generated_or_loaded" -eq 1 ]; then
  echo "Developer tokens (saved to $tokens_file, mode 600):"
  echo "  LIFESIM_OBSERVER_TOKEN=$LIFESIM_OBSERVER_TOKEN"
  echo "  LIFESIM_ADMIN_TOKEN=$LIFESIM_ADMIN_TOKEN"
fi

echo "Server:  http://$advertise:$rest_port  (WS ws://$advertise:$ws_port)"
echo "Console: http://$advertise:$ui_port"
if [ "$bind" = "127.0.0.1" ]; then
  echo "Bound to loopback; set CONSOLE_BIND=0.0.0.0 to serve the private LAN directly, or"
fi
echo "Reach both from elsewhere with:"
echo "  ssh -L $ui_port:127.0.0.1:$ui_port -L $rest_port:127.0.0.1:$rest_port -L $ws_port:127.0.0.1:$ws_port genesis-engine"

wait
