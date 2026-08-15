# VM 120 deployment

This directory is the only supported initial deployment path for the
`genesis-engine` guest. It keeps the simulation server on loopback and makes
the VM, not a developer workstation, the durable owner of worlds and saves.

## Separation of duties

| Identity | Purpose | Privilege |
|---|---|---|
| `genesis` | systemd service and persistent world data | non-login service account |
| `genesis-dev` | Codex/Claude compilation, tests, and headless campaigns | SSH public key; no sudo |
| root | bootstrap, release promotion, secret file, service management | human-approved only |

The bootstrap accepts a public key only. It must be the 1Password-managed
Homelab AI public key while Codex Desktop/CLI and Claude Code share that
agent. Replace it later with separate per-client public keys if they move to
different machines. Never run the older `install-ai-access.sh` here: that
pattern grants passwordless sudo, which development agents do not need.

## First migration

From the guest console, after confirming its OS/update state and the desired
LAN address:

```sh
sudo /path/to/bootstrap-host.sh /path/to/homelab-ai.pub
sudo install -d -m 0750 -o root -g genesis /etc/genesis-engine
sudo install -m 0644 genesis-engine.service /etc/systemd/system/genesis-engine.service
sudo install -m 0640 -o root -g genesis runtime.env.example /etc/genesis-engine/runtime.env
# Replace both example token values from the approved secret manager.
sudo systemctl daemon-reload
sudo systemctl enable genesis-engine.service
sudo /path/to/install-release.sh git@github.com:ctadros1/The-Genesis-Engine.git <pinned-commit>
```

`install-release.sh` verifies the Git commit prefix, uses the repository's
Rust toolchain, verifies the pinned official Node archive checksum, builds
the server and observer, and promotes the immutable release only after both
builds succeed. It never deploys an uncommitted working tree.

Validate the local service before exposing a browser endpoint:

```sh
sudo systemctl status genesis-engine --no-pager
sudo -u genesis curl -fsS -H "Authorization: Bearer $LIFESIM_OBSERVER_TOKEN" http://127.0.0.1:8940/api/worlds
sudo journalctl -u genesis-engine -n 100 --no-pager
```

## Browser interface

Do not install `Caddyfile` until private DNS, firewall sources, and client
trust for Caddy's internal certificate authority are agreed. Substitute the
actual private DNS name; the file routes REST and the WebSocket while leaving
the Rust process bound to loopback. The production observer automatically
uses its own origin, while Vite development retains the loopback defaults.

## Recovery

Before creating a persistent named world, verify an application recovery set:
the snapshot, event log, config, and SQLite catalog must restore together in
an isolated location. Then verify that VM 120 is included in the current PBS
job for servernode3. The VM snapshot is supplementary, not a replacement for
the application-consistent recovery set.
