# Server Development Workflow

`genesis-engine` (VM 120 on `servernode3`) is the authoritative development
and simulation host. Do not run multi-minute simulations, release builds, or
full workspace tests on a developer laptop when the VM is reachable.

## Access

On the configured development Mac, use the 1Password-managed Homelab AI
identity through the SSH alias:

```sh
ssh genesis-engine
```

This logs into the deliberately restricted `genesis-dev` account. It has a
read-only GitHub deploy key, no sudo, no password login, no agent forwarding,
and local-only port forwarding. Codex and Claude working from this Mac use
this same account; an agent on another machine needs its own public key added
to `genesis-dev` by the infrastructure owner.

The checked-out developer workspace is:

```text
/home/genesis-dev/src/The-Genesis-Engine
```

Use normal Git commands there. The configured deploy key makes `git fetch`,
`git pull`, and read-only clones work without exporting GitHub credentials.

## Development commands

```sh
ssh genesis-engine
cd ~/src/The-Genesis-Engine
git pull --ff-only
cargo test -p sim-server
cd apps/observer && npm ci && npm run build
```

For long simulations, redirect output to a named file under `/home/genesis-dev`
and monitor the process. Run one expensive WorldMod or benchmark campaign at a
time; the full workspace suite contains intentionally CPU-intensive long-run
tests and must not overlap the production simulation service.

## Console developer instance

`apps/console` (ADR-0039) is served by a **second** `lifesim-server`
instance on developer ports, with its own data directory and its own
tokens. It never touches the production process, its data, or its tokens.

```sh
ssh genesis-engine
cd ~/src/The-Genesis-Engine
git pull --ff-only
scripts/run-console-dev.sh
```

The script builds `lifesim-server` and the console, starts the server on
REST 8960 and WebSocket 8961 against `$HOME/console-dev-data` with
`--max-worlds 8`, serves the built console on 127.0.0.1:5280, waits for
`/api/health`, prints the URLs, and stays up until Ctrl-C, which tears both
processes down. Override the ports with `CONSOLE_REST_PORT`,
`CONSOLE_WS_PORT` and `CONSOLE_UI_PORT`.

**Tokens.** Export `LIFESIM_OBSERVER_TOKEN` and `LIFESIM_ADMIN_TOKEN` to
supply your own, and the script prints nothing about them. Otherwise it
reuses whatever a previous run saved in `$HOME/console-dev-data/tokens.env`
(mode 600) or generates a fresh pair, saves them there, and prints them
once. Do not commit them, paste them into an agent prompt, or reuse the
production tokens from `/etc/genesis-engine/runtime.env` here.

The data directory persists, so a second run reuses the saves and
checkpoints the first one wrote. Note what ADR-0039 does not promise: the
**world registry** is not durable. A restart boots world 1 only; every
other world comes back by branching one of its saves.

Everything is loopback-only on the VM, so reach it from the development Mac
through the SSH alias's local port-forward, then open
`http://127.0.0.1:5280`:

```sh
ssh -L 5280:127.0.0.1:5280 -L 8960:127.0.0.1:8960 -L 8961:127.0.0.1:8961 genesis-engine
```

In the console's Server screen, point the profile at
`http://127.0.0.1:8960` and `ws://127.0.0.1:8961`, paste the two tokens,
and press Test before Connect.

Playwright specs for the console run against a real server through
`scripts/run-console-e2e.sh`, which is separate from this one: its own
ports (8962/8963), fixed test tokens, and no `--data-dir` at all, so it
cannot touch the developer instance's worlds or saves.

## Runtime and release boundary

The production process is a separate, non-login `genesis` account managed by
`genesis-engine.service`. It owns persistent world data in
`/var/lib/genesis-engine`; `genesis-dev` must not modify it.

Install a reviewed, pushed commit with the root-only deployment script. The
installer compiles an immutable release under `/opt/genesis-engine/releases`,
switches `/opt/genesis-engine/current`, and restarts the service. It never
uses a developer checkout as production runtime.

The Observer is served privately at `https://genesisengine.local`. The Rust
server remains loopback-only on ports 8940 (REST) and 8941 (WebSocket); Caddy
is the only browser-facing boundary. Paste the observer token into the
Observer connection panel. Keep API tokens in `/etc/genesis-engine/runtime.env`
and never commit, print, or paste them into agent prompts.

The VM advertises this name with mDNS. If a client LAN blocks mDNS discovery,
the infrastructure owner must provide internal DNS or a host mapping for
`192.168.75.186 genesisengine.local`; do not replace the HTTPS name with a raw
IP address, because the certificate and browser-origin policy are intentional.
