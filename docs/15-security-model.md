# Security Model

## Phase 3 Implementation Notes

The Phase 3 server realizes the private boundary as follows: it binds
127.0.0.1 only (LAN/WireGuard exposure is a later, separately approved
deployment step); every REST request and WebSocket session requires a
bearer token resolved server-side to the observer or admin role; tokens
come from `LIFESIM_OBSERVER_TOKEN`/`LIFESIM_ADMIN_TOKEN` or are generated
from OS entropy and printed once at startup — nothing is persisted or
committed. Controls validate role, action, bounds, idempotency key, and
rate limit, and append audit records. WebSocket frames are validated
fail-closed before allocation; malformed frames get a structured error
and cannot influence world state. The observer UI keeps tokens in input
fields; query-parameter tokens exist for test/kiosk automation on private
networks only and are documented as such. TLS and reverse-proxy choices
remain deployment decisions after a live network audit.

## Default Posture

The initial system is private. It is reachable only from the trusted LAN and/or WireGuard paths approved for the homelab. It has no public DNS dependency, public registration flow, or unauthenticated public control endpoint.

## Roles

| Role | Allowed | Not Allowed |
|---|---|---|
| Observer | Read world state, analytics, selected organism details | Pause, mutate world, restore, export protected data |
| Administrator | World lifecycle, sandbox interventions, save/restore, experiments | Bypass validation/audit |
| Service | Tick, persistence, metrics | Interactive admin actions |

Roles are authorization claims validated server-side; a hidden browser button is never an authorization boundary.

## Control Safety

Every state-changing command validates identity, role, world state, bounds, idempotency key, and input schema. It records actor, request, accepted effect, tick, config hash, and correlation ID. Destructive world operations require an explicit server-side confirmation token or branch-first flow. Rate-limit controls and cap payload/viewport sizes.

## Secrets And Dependencies

Use the existing approved secret manager for generated credentials. Do not commit environment files, API keys, WireGuard material, backup credentials, or database secrets. Services use a dedicated non-login account and least-privilege filesystem paths. Pin and review dependencies; maintain a dependency-update procedure before any Internet exposure.

## Input And File Safety

Treat all save files, genomes, configs, WebSocket frames, and exports as hostile. Validate magic/version/length/checksum before allocation or decompression; impose strict decoded-size and nesting limits; reject unknown incompatible schema. Do not deserialize executable code or accept arbitrary paths in export/restore APIs.

## Transport

Use TLS when access leaves a locally trusted tunnel or when authentication cookies/tokens require it. Exact reverse-proxy and certificate choices remain a deployment decision after a live network audit. Do not weaken host firewall, WireGuard, or proxy policy to simplify first deployment.

## Incident Expectations

Security events include failed auth, denied controls, malformed frames, invalid saves, audit-log integrity failures, and repeated resource-limit violations. Alerts must not leak secrets or full organism/world payloads.
