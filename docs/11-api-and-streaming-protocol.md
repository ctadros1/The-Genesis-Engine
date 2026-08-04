# API And Streaming Protocol

## Phase 3 Implementation Notes

`sim-server` implements the Phase 3 subset, loopback-only by default:
REST `GET /api/health` (unauthenticated), `GET /api/worlds`,
`GET /api/worlds/1`, `GET /api/worlds/1/organisms/{id}` (bounded detail),
`GET /api/worlds/1/analysis`, `GET /metrics` (Prometheus text), and
`GET /api/benchmarks/ticks` for the observer role; `GET /api/audit` and
`POST /api/worlds/1/control?action=pause|resume|speed&multiplier=N` for
the admin role with Idempotency-Key replay, a 100 ms control rate limit,
input clamps, and audit records (actor role, action, acceptance, tick,
key). Live state flows only over the binary WebSocket
(`specifications/websocket-protocol.md`). Worlds/saves/exports/experiment
groups and mid-run interventions beyond pause/resume/speed belong to
later phases. Measured stream behavior is recorded in
`research/performance-notes.md` (Phase 3 record).

## Separation

Use REST/HTTP for durable metadata, world lifecycle, controls, exports, and save operations. Use a binary WebSocket for live world state. Keep control commands request/response and state stream one-way after subscription. Do not make REST polling the live rendering path.

## HTTP Resource Groups

| Group | Example Operations | Authorization |
|---|---|---|
| Worlds | List, create from config, branch, pause, resume | Observer for read; admin for mutation |
| Organisms | Lookup, lineage, trait/controller detail | Observer |
| Analytics | Query downsampled metrics, export run | Observer/export policy |
| Saves | List, create, restore request, validate | Admin |
| Controls | Spawn, food/weather change, protect, disaster | Admin plus audit |
| Experiments | Launch/compare independent seeded worlds | Admin |

All mutation endpoints use idempotency keys, schema validation, world lifecycle checks, and audit events.

## WebSocket Lifecycle

1. Client authenticates and declares protocol versions/capabilities.
2. Client sends a bounded viewport subscription with layers and max update rate.
3. Server replies with world metadata and a keyframe.
4. Server sends ordered delta frames; client acknowledges applied sequence periodically.
5. On sequence gap, world epoch change, or excessive lag, server sends a new keyframe.
6. Client unsubscribe/disconnect releases the subscription without affecting the world.

## Frame Types

| Type | Direction | Purpose |
|---|---|---|
| Hello/Welcome | Both | Version and capability negotiation |
| Subscribe/Unsubscribe | Client to server | Bounded viewport/layer selection |
| Keyframe | Server to client | Self-contained visible-state baseline |
| Delta | Server to client | Adds/updates/removes since sequence |
| MetricsSample | Server to client | Downsampled chart information |
| Ack | Client to server | Backpressure/resync hint |
| Error | Both | Structured non-secret failure |

The binary framing layout is defined in specifications/websocket-protocol.md. Use explicit integer widths, bounds, protocol version, world epoch, state sequence, and payload length. Reject malformed lengths before allocation.

## Bandwidth Policy

The server culls by viewport, collapses superseded updates for slow clients, sends terrain separately at a lower cadence, and aggregates far-zoom organisms. Update budget, maximum entities/frame, and resync behavior are config-backed and measured. Never serialize entire genome/controller matrices in movement deltas; inspect them through bounded detail endpoints.

## Compatibility

The protocol has a semantic version plus frame schema version. Additive compatible fields may negotiate by capability; breaking layout changes require a new version and a migration/rollout plan. WebSocket clients cannot control tick state through undocumented messages.
