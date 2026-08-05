# Simulation Tick Specification

## Phase 1 Implementation Notes

`sim-core::World::step_with_observer` implements the canonical order below as
explicit phases `commands, environment, spatial_index, sense, controllers,
apply, lifecycle, finalize`. Phase 1 defines no in-tick commands (pause/resume
act between ticks); with Phase 2 disabled the `controllers` phase is an empty
boundary kept for timing comparability. With Phase 2 enabled, `sense` gathers
the bounded normalized inputs in stable entity-ID order, `controllers`
evaluates every genome controller into bounded intents, and `apply` resolves
memory commit, movement, costs, feeding, pairing, and aging; conflict
resolution remains stable entity-ID iteration. A host-supplied `TickObserver`
receives phase boundaries so wall-clock timing stays outside the kernel. The
per-tick event buffer is bounded (4,096) with a deterministic dropped-event
counter, and the state checksum is computed on demand, never inside the tick.

## Planned Successor Phases (6 To 11)

The canonical order below is extended, not reordered. Each addition is empty
when its config section is disabled, so simulation results for existing
worlds are unchanged; only the per-phase benchmark shape changes, which
increments the benchmark schema version.

    commands, environment, spatial_index, sense, controllers, apply,
    learn, lifecycle, finalize

- `sense` gains perception of neighbours' last committed actions and of the
  committed signal field (Phase 13), and object perception (Phase 12). All of
  it reads the **previous tick's committed state only**.
- `controllers` evaluates variable-topology networks synchronously (Phase 9).
- `apply` gains contest resolution (Phase 7), object actions (Phase 12),
  signal emission accumulation (Phase 13), and growth and thermoregulation
  (Phase 13).
- `learn` is new (Phase 11): plastic edges update in ascending edge
  `homology_id` order, reading only values committed earlier in the same
  tick and writing only learned state.
- `lifecycle` gains carcass creation (Phase 7), innovation-ID allocation for
  new children (Phase 9), object decay and terrain-modification application
  in ascending `(layer_id, cell_index)` order (Phase 12), and hazard draws
  (Phase 13).
- `finalize` commits the signal field (Phase 13).

The ordering rules below are extended by
`specifications/determinism-extensions.md`, which is normative. The four
additions most easily missed:

1. Pairwise random draws use the canonical pair key `lifesim-pairkey-v1`, so
   an outcome never depends on which participant the tick visited first.
2. Every candidate set (nearest neighbours, reachable objects, imitation
   models) is materialized, sorted by `(distance_squared, object_id)`, and
   truncated before any selection. Spatial bucket scan order never reaches a
   decision.
3. Roles in an asymmetric interaction are assigned by ID comparison, never
   by traversal.
4. Per-node float summation is pinned to ascending edge `homology_id` order.

## Contract

A tick advances one world from state S(t) to S(t + dt) through fixed phases. The default dt is 0.1 simulated seconds; it is a versioned config value. Headless acceleration processes more ticks per wall-clock second without changing dt.

## Canonical Order

1. Apply validated queued commands at the tick boundary.
2. Advance clock and environmental fields.
3. Build/update spatial index.
4. Gather bounded sensors in stable entity-ID order.
5. Evaluate controllers into bounded intents.
6. Resolve movement, feeding, attack, mating, and reproduction conflicts deterministically.
7. Apply energy/health/aging/death/carcass transitions.
8. Produce events, metrics, deterministic checksum, and scheduled persistence view.

## Ordering

Each phase sees the documented prior or current snapshot only. Multiple contenders use a deterministic tie key: action priority, stable entity ID, then deterministic action-local random value if a config enables a tie lottery. Never depend on hash-map order or task completion order.

## Random Streams

derive_rng(world_seed, tick, system_id, subject_id, draw_index) returns a deterministic local stream. Systems cannot share a mutable global RNG. Adding an unrelated random draw must not perturb another system's stream.

## Failures

Invalid state triggers a typed invariant failure in test/strict mode and safe neutralization/rejection policy in production mode as documented by component. The tick must not partially advance without an event/error record. Persistence snapshots only occur at completed tick boundaries.

## Tests

- Same starting snapshot/config/seed yields same event/checksum policy.
- Reordering container insertion does not change outcome.
- Paused world produces no state delta.
- Invalid command/frame/genome does not advance malformed state.
