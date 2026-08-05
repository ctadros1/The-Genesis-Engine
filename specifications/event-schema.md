# Event Schema

## Phase 5 Implementation Notes: The Event-Log File (ALEV format 1)

The append-only event-log *file* deferred under D-019 exists as of Phase 5
(`crates/sim-persist/src/eventlog.rs`). Snapshots carry a real
`event_log_offset` for the first time; it is the byte length of the log at
capture, so a restored world knows exactly where its recorded history stops.

Framing, all little-endian, matching the ALIF snapshot codec:

- A fixed 64-byte header: magic `ALEV`, format version, flags, world ID,
  seed, config hash, event schema version, the kernel's
  `MAX_EVENTS_PER_TICK`, the start tick, the build string length, and a
  CRC32 over the header and the build string that follows it.
- Then zero or more segments in strictly ascending tick order, one per tick
  that produced events or drops: magic `SEG1`, tick, event count, the number
  of events the bounded buffer dropped during that tick, body length, body,
  and a CRC32 over the whole frame.
- Each event in the body is a one-byte permanent type tag followed by a
  fixed payload. Tags are never reused or renumbered, exactly like an
  `RngSystem` value.

Why the header checksums itself, when the snapshot header does not: almost
everything it carries is provenance rather than framing. A corrupted length
fails structurally on its own, but a corrupted seed or config hash would
silently mislabel an experiment, and nothing downstream would notice.

A tick that produced no events and no drops writes nothing, so an idle world
costs no bytes.

Decoding rules, all of which are tested by a 20,000-case seeded corruption
sweep:

- Every declared length is capped before it reaches an allocation or a slice
  index; a segment claiming more events than this kernel can produce in one
  tick is rejected as impossible rather than trusted.
- The segment CRC is verified before any byte of the body is parsed.
- An unknown event type **fails closed**. Skipping it would silently corrupt
  every rate an analysis computes.
- Ticks must ascend strictly; a repeated or reordered tick is an error.
- A torn tail — the shape a crash between `write` and `sync` leaves — is
  reported by a separate prefix reader that returns the valid prefix and the
  typed error that ended it. That is a reporting path, not a repair path: it
  never rewrites the file, never invents a value, and never admits a
  partially decoded segment.

**Counter reconstruction.** Replaying a log reproduces every counter in
`sim_core::Counters` and `sim_core::Phase2Counters`. Two subtleties the
implementation has to get right: a `PairedBirth` advances `births_total` as
well as `paired_births_total`, and `ControllerFault` contributes its
neutralized *count*, not one per event. With zero drops the check is exact
equality on every counter. With drops it cannot be — a dropped event leaves
no record of which counter it would have advanced — so the check becomes the
two things that remain exactly checkable: the log's recorded drop total
equals the kernel's, and no reconstructed counter exceeds the world's.

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

`EVENT_SCHEMA_VERSION` becomes 3 across Phases 7 to 13. Version 2 payloads
are unchanged; version 3 is additive. A reader that encounters an unknown
event type fails closed rather than skipping it, because a silently skipped
event would corrupt any analysis that counts rates.

New bounded payloads, grouped by the phase that adds them:

| Phase | Event | Payload |
|---|---|---|
| 7 | `Damage` | attacker, target, raw and applied damage, resulting health |
| 7 | `DeathByDamage` | organism, attacker, final state summary |
| 7 | `CarcassCreated` / `CarcassConsumed` | source organism, object ID, transferable energy |
| 9 | `StructuralMutation` | child, operator class, locus count affected, new homology-ID range |
| 9 | `StructuralRejected` | child, cap class that rejected it |
| 11 | `PlasticityFault` | organism, neutralized non-finite count |
| 13 | `SignalEmitted` | emitter, channel mask, amplitude summary, energy cost |
| 13 | `PerceptionFault` | organism, neutralized count |
| 12 | `ObjectCreated` / `ObjectDestroyed` | object ID, material, creator, composition depth |
| 12 | `ObjectCombined` / `ObjectFractured` | inputs, output, joint quality or fragment count |
| 12 | `ObjectActionRejected` | actor, action, typed reason |
| 12 | `TerrainModified` | actor, layer, cell, old and new value |
| 8 | `DeathByHazard` / `DeathBySenescence` | organism, cause, age, accumulated hazard |
| 10 | `NonViableBody` | child, failed validation reason |
| 14 | `DiseaseTransmitted` | source, target, load |

Rules that do not change: events are append-only logical facts; they carry
no secrets, no unbounded activation history, and no arbitrary payload;
reading events never alters simulation state; the per-tick buffer stays
bounded with a deterministic dropped counter.

Two additions specific to the new goal:

- **Signal content is never an event label or a metric label.** It would be
  unbounded cardinality, and it would encourage reading meaning into
  channels that have none.
- **The event log is the substrate for Phase 17 analysis.** The append-only
  event-log *file* deferred under D-019 moved into Phase 5's scope, because
  every multi-seed experiment needs it long before era detection does. It is
  implemented; see the Phase 5 notes above. Snapshots taken without a log
  still record a zero reference, and that remains valid.

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


## Positions Are Not Events

Organism positions are deliberately absent from this schema and must stay
absent. An event records something that happened; a coordinate dump is
state, and admitting one would break the "one record, one occurrence"
reading that every counter reconstruction in `eventlog.rs` depends on, as
well as multiplying log size by three orders of magnitude.

Spatial analysis reads a separate artifact,
`specifications/spatial-sample-format.md` (ALSS 1), written by the
experiment harness through the kernel's read-only observer view. See D-060.
