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
