# Event Schema

## Phase 2 Implementation Notes (event schema version 2)

The in-kernel event stream (`sim_core::EventKind`, versioned by
`EVENT_SCHEMA_VERSION = 2`) adds bounded Phase 2 payloads:

- `PairedBirth`: child ID, both immutable parent IDs, genome hash,
  per-parent energy investment, and mutated trait/neural gene counts (the
  numeric-variation audit summary).
- `PairRejected`: both parent IDs and a typed reason (capacity, placement,
  energy). Absence of an eligible partner is not an event.
- `ControllerFault`: organism ID and the neutralized non-finite count for
  that tick.

Malformed parameter records are rejected with typed errors at the codec
boundary before anything reaches world state, so no world event exists for
them; hosts count rejections at their own boundary. The per-tick buffer
stays bounded (4,096 with a deterministic dropped counter), and reading
events never alters simulation state. The full envelope (event_id,
world_id, config hash) remains a Phase 3+ transport concern; in-kernel
events carry tick and typed payload only.

## Planned Successor: Event Schema Version 3

`EVENT_SCHEMA_VERSION` becomes 3 across Phases 6 to 11. Version 2 payloads
are unchanged; version 3 is additive. A reader that encounters an unknown
event type fails closed rather than skipping it, because a silently skipped
event would corrupt any analysis that counts rates.

New bounded payloads, grouped by the phase that adds them:

| Phase | Event | Payload |
|---|---|---|
| 6 | `Damage` | attacker, target, raw and applied damage, resulting health |
| 6 | `DeathByDamage` | organism, attacker, final state summary |
| 6 | `CarcassCreated` / `CarcassConsumed` | source organism, object ID, transferable energy |
| 7 | `StructuralMutation` | child, operator class, locus count affected, new innovation ID range |
| 7 | `StructuralRejected` | child, cap class that rejected it |
| 8 | `PlasticityFault` | organism, neutralized non-finite count |
| 9 | `SignalEmitted` | emitter, channel mask, amplitude summary, energy cost |
| 9 | `PerceptionFault` | organism, neutralized count |
| 10 | `ObjectCreated` / `ObjectDestroyed` | object ID, material, creator, composition depth |
| 10 | `ObjectCombined` / `ObjectFractured` | inputs, output, joint quality or fragment count |
| 10 | `ObjectActionRejected` | actor, action, typed reason |
| 10 | `TerrainModified` | actor, layer, cell, old and new value |
| 11 | `DiseaseTransmitted` | source, target, load |

Rules that do not change: events are append-only logical facts; they carry
no secrets, no unbounded activation history, and no arbitrary payload;
reading events never alters simulation state; the per-tick buffer stays
bounded with a deterministic dropped counter.

Two additions specific to the new goal:

- **Signal content is never an event label or a metric label.** It would be
  unbounded cardinality, and it would encourage reading meaning into
  channels that have none.
- **The event log is the substrate for Phase 12 analysis.** The append-only
  event-log *file* deferred under D-019 moves into Phase 5's scope, because
  every multi-seed experiment needs it long before era detection does.
  Snapshots carry a zero event-log reference until it exists.

## Envelope

Every event has event_id, world_id, parent_world_id if applicable, tick, sequence_in_tick, event_schema_version, simulation_version, config_hash, event_type, and typed payload. Event IDs are monotonic within a world event stream or otherwise canonicalized for deterministic replay.

## Event Types

| Type | Required Payload |
|---|---|
| organism_birth | child, parent IDs, genome hash, energy investment, location |
| organism_death | organism ID, cause, final state summary |
| feeding/attack | actor/target/cell, attempted/accepted transfer, reason |
| intervention | actor role/id pseudonym, requested/accepted effect, idempotency key |
| world_state | pause/resume/speed/branch/restore state |
| save | save ID/path-safe reference, checksum, duration, result |
| capacity | requested allocation, limit, deterministic outcome |
| disaster | type, seed, extent, intensity, start/end |

## Rules

Events are append-only logical facts. They do not contain secrets, unbounded neural activation history, or arbitrary user payload. Schema additions are versioned and tests prove exporter/replay compatibility.
