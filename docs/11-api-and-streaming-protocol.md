# API And Streaming Protocol

## Phase 3 Implementation Notes

`sim-server` implemented the Phase 3 subset, loopback-only by default:
REST `GET /api/health` (unauthenticated), `GET /api/worlds`,
`GET /api/worlds/1`, `GET /api/worlds/1/organisms/{id}` (bounded detail),
`GET /api/worlds/1/analysis`, `GET /metrics` (Prometheus text), and
`GET /api/benchmarks/ticks` for the observer role; `GET /api/audit` and
`POST /api/worlds/1/control?action=pause|resume|speed&multiplier=N` for
the admin role with Idempotency-Key replay, a 100 ms control rate limit,
input clamps, and audit records (actor role, action, acceptance, tick,
key). Live state flows only over the binary WebSocket
(`specifications/websocket-protocol.md`). Measured stream behavior is
recorded in `research/performance-notes.md` (Phase 3 record).

ADR-0039 widened that surface to many worlds without changing any of it:
every route above still exists, still means what it meant, and world 1 is
still the world the command-line flags built. The section below is the
current surface.

## Multi-World Surface (ADR-0039)

One process hosts a registry of worlds, each with its own `World`, control
state, subscribers and tick thread. World ids are process-assigned
integers; world 1 comes from the command-line flags. `--max-worlds N`
bounds the registry (default 8, counting world 1); a creation at the bound
is refused with 409 rather than growing the process. The registry is not
durable: a restart boots world 1 only, and a world outlives a restart by
being branched from one of its saves.

Authorization is unchanged: bearer token on everything but
`/api/health`, observer role reads, admin role mutates. Every mutation is
audited with its world id, keyed mutations replay through the
Idempotency-Key cache, and controls keep the 100 ms per-world rate limit.

| Method | Route | Role | Meaning |
|---|---|---|---|
| GET | `/api/health` | none | liveness |
| GET | `/api/schema` | observer | presets (`phase1`, `phase2`, with a description each), every settable field (`name`, `type` in u32/u64/i32/i64/bool/choice, `choices` when the type is choice, `defaults` per preset), and `limits` (`max_worlds`, `max_cells_x`, `max_cells_y`) |
| POST | `/api/schema/preview` | observer | body `{preset, seed?, settings:{name:value}}` -> `{preset, seed, config_hash, valid, errors:[...]}`; creates nothing, so an impossible world answers 200 with `valid` false rather than an error status |
| GET | `/api/worlds` | observer | every hosted world's summary |
| POST | `/api/worlds` | admin | body `{name, preset, seed?, settings:{}, paused?, speed?}` -> 201 summary; 400 on an unknown field, an out-of-range value or a validation failure (the message names the field); 409 at the `--max-worlds` bound |
| GET | `/api/worlds/{id}` | observer | one world's summary (below) |
| POST | `/api/worlds/{id}/control?action=pause,resume,speed,stop` | admin | per world; `speed` takes `&multiplier=N`; `stop` ends the tick thread after a final checkpoint when a store exists, and a stopped world stays readable and saveable |
| DELETE | `/api/worlds/{id}` | admin | removes a stopped world from the registry; 409 while running; its saves stay on disk |
| GET | `/api/worlds/{id}/saves` | observer | the save rows carrying this world id |
| POST | `/api/worlds/{id}/saves` | admin | create a save of this world |
| POST | `/api/worlds/{id}/saves/{save_id}/verify` | admin | rebuild that save in isolation and compare checksums |
| POST | `/api/worlds/{id}/branch?save_id=N&name=` | admin | 201 a new world loaded from that save (`world_epoch` 2, `parent_world_id` = id); 404 for an unknown save |
| GET | `/api/worlds/{id}/organisms/{oid}` | observer | bounded organism detail |
| GET | `/api/worlds/{id}/analysis` | observer | species clustering; 409 on a world without phase 2 |
| GET | `/api/benchmarks/ticks?world=N` | observer | that world's tick series (`world` defaults to 1) |
| GET | `/metrics` | observer | Prometheus text, one line per world per family; world 1 keeps its pre-ADR-0039 `world="server"` label alone, and every other world adds `world_id="N"` beside it |
| GET | `/api/audit` | admin | the last 100 records, each with its `world_id` |

A world summary carries `world_id`, `name`, `status`
(`running`/`paused`/`stopped`), `created_unix_ms`, `parent_world_id`
(0 when it was not branched), `world_epoch`, `preset`, `seed`,
`config_hash`, `cells_x`, `cells_y`, `cell_size_m`, `dt_ms`, `tick`,
`population`, `births_total`, the `deaths_*` counters, `extinct`,
`phase2`, `paused`, `speed_multiplier`, `tick_mean_us`,
`ticks_per_second`, `total_biomass_milli` and `total_energy_milli`.
Every field world 1 reported before ADR-0039 keeps its name and shape.

**Settings are set at creation and never after.** A world is created from
a preset plus named settings drawn from the campaign field registry
(`sim-experiment::fields`), validated by `SimConfig::validate`, and
reported with its config hash. The seed is a creation input, not a
setting. There is no route that mutates a running world's config; the
control surface is pause, resume, speed and stop. This is ADR-0039's
first decision: a running world's settings are hashed into every result
it produces, so a live edit would fork its identity.

### Stream World Selection

The WebSocket upgrade path selects the world. `/worlds/{id}` names a
world; `/` and `/worlds/1` are world 1. `ALSP` stays at 1.0 - no frame
changed and no new frame was added; the Welcome's existing `world_id`
field reports the selection. A path that names no world (`/worlds`,
`/worlds/two`, `/worlds/2/extra`) and a path naming a world the registry
does not hold both answer Error 404 and close; a world stopped
mid-session answers Error 410 and closes. Selection is reported only
after the token is checked, so an unauthenticated client learns nothing
about which worlds exist.

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
