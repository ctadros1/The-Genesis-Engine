# Phase 5: Headless Scale And Multi-World Experiments

Status: planned, not started. Supersedes the headless-throughput and
independent-world parts of `planning/superseded/phase-5-performance-optimization.md`.

## Problem

Every acceptance criterion from Phase 7 onward is a claim of the form "the
effect occurs in N of M seeds under condition A and fewer under condition
B". None of that is measurable today.

Three concrete blockers:

- The server paces ticks in real time against `dt / speed`, so a run is
  bound to observer time. The Phase 2 long run took 405.7 s of wall clock
  for 200,000 ticks and reached 127 ancestry generations. Cultural ratchets
  plausibly need orders of magnitude more generations than that, and every
  ablation multiplies the requirement by the number of conditions and seeds.
- There is no independent-world scheduler. It has sat in the deferred
  backlog since Phase 0 and every multi-seed design needs it.
- There is no append-only event log file. Snapshots carry a zero event-log
  reference (D-019). Every later analysis, and all of Phase 16, reads that
  file.

This phase builds the instrument. It adds no organism behavior at all.

## Scope

- Headless accelerated execution fully decoupled from observer pacing, with
  a documented and tested guarantee that acceleration cannot change results.
- An independent-world scheduler running N worlds concurrently on one host,
  sharing no mutable state.
- An experiment harness: a declarative multi-seed, multi-condition run
  definition; per-run provenance; per-seed checksums; result aggregation.
- The append-only event log segment, and wiring the snapshot event-log
  reference that has been zero since Phase 4.
- Asynchronous or double-buffered checkpointing, so a checkpoint no longer
  stalls the tick thread synchronously.
- An ablation-condition mechanism in the config schema: a condition is a
  named config delta with its own hash, so a control and a treatment are
  never confused for the same experiment.

## Non-Goals

- No new organism behavior, sensing, action, or genome change.
- No intra-world parallelism. Distributing one tick across threads or hosts
  remains gated on ADR-0010's ordering and reduction evidence and is not
  opened here.
- No GPU work.
- No distributed multi-host scheduling. One host, N processes or threads.
- No claim of a supported world count or throughput before it is measured.
- No deployment, VM, or infrastructure change.

## Prerequisites

- Phase 4 persistence, complete.
- Nothing else. This phase is the prerequisite for everything after it.

## Deliverables

- `lifesim batch` or equivalent: run a declared experiment across seeds and
  conditions, writing per-run snapshots, event logs, metrics, and a manifest.
- Independent-world scheduler with a configured worker count.
- Event-log file format, writer, reader, and bounded fail-closed decode.
- Asynchronous checkpoint path with the tick-thread stall measured before
  and after.
- Experiment manifest schema and a comparison report across conditions.

## Acceptance Criteria

- [ ] **A5.1 Acceleration is result-neutral.** For a fixed seed and config,
      the final state checksum after T ticks is identical when run at 1x
      pacing, at maximum headless speed, with an observer attached, and with
      an observer attached and then detached mid-run. Four executions, one
      checksum.
- [ ] **A5.2 Scheduling is result-neutral.** For a fixed set of 30 seeds and
      one config, per-world final state checksums are identical at scheduler
      concurrency 1, 2, and C (the configured maximum), and identical to
      running each world alone. Any work-stealing, thread-count, or
      completion-order dependency shows up here as a checksum difference.
      This is the determinism criterion that makes every later multi-seed
      claim trustworthy.
- [ ] **A5.3 The event log is complete and replayable.** For a run of at
      least 10^6 ticks, the event log contains every event the kernel
      emitted with zero drops, or, if the bounded per-tick buffer dropped
      events, the recorded drop counter matches the gap exactly. Reading the
      log back reconstructs the counters in the final snapshot exactly.
- [ ] **A5.4 The event log decoder is hostile-input safe.** A seeded
      corruption sweep of at least 20,000 cases produces zero panics and
      typed rejections, matching the discipline of the existing protocol and
      snapshot sweeps.
- [ ] **A5.5 Checkpoints no longer stall the tick thread beyond budget.**
      Measured tick p95 during checkpointing is within the configured tick
      interval at both supported tiers, with before and after numbers
      recorded against the Phase 4 record `phase4-local-20260804T141013Z`.
- [ ] **A5.6 Conditions are distinguishable by construction.** Two
      conditions of the same experiment produce different config hashes, and
      the comparison report refuses to aggregate runs whose hashes differ in
      any field the report does not explicitly name as the varied field.
- [ ] **A5.7 Fixtures preserved.** `0x1e3158a26afd3b39` and
      `0xff9dfcff5dffbf42` reproduce from clean processes under the new
      execution paths.

Note on what is deliberately absent: there is no acceptance criterion of the
form "achieves X worlds at Y ticks per second". Throughput is measured and
recorded, and the supported tier is stated from the measurement. Declaring a
target first would be exactly the unmeasured scale claim AGENTS.md forbids.

## Test Plan

- Determinism: A5.1 and A5.2 as automated tests, not manual procedures.
- Event log: round-trip equality, drop-counter consistency, corruption
  sweep, bounded decode with all lengths capped before allocation.
- Checkpoint: crash simulation during an asynchronous checkpoint leaves the
  last valid checkpoint authoritative, extending the existing Phase 4
  crash-simulation test to the new write path.
- Scheduler: worker crash isolation; one world failing does not corrupt or
  stall another; a failed world is reported, not silently dropped.
- Manifest: a comparison across conditions with mismatched non-varied fields
  is rejected with an actionable error.

## Benchmark Impact

New record `phase5-local-<timestamp>` covering: headless ticks per second
per world at both tiers; aggregate throughput at worker counts 1, 2, 4, 8;
host contention (per-world throughput degradation as workers increase);
event-log write cost as a share of tick time and its file growth rate per
10^6 ticks; asynchronous checkpoint cost and tick p95 during checkpointing
versus the Phase 4 synchronous baseline.

Benchmark schema increments to 3. Earlier records stay valid and unmodified;
they are comparable within their own schema version only.

## Documentation Updates

`docs/13-performance-strategy.md`, `docs/12-data-storage-and-saves.md`,
`docs/03-system-architecture.md` (experiment runner), `docs/14-testing-strategy.md`
(ablation discipline), `specifications/experiment-config-schema.md`,
`specifications/event-schema.md`, `specifications/metrics-schema.md`,
`research/performance-notes.md`, decision log.

## Risks

| Risk | Mitigation |
|---|---|
| Asynchronous checkpointing introduces a race that corrupts a snapshot | Snapshot from an immutable captured state, never from live arrays; extend the existing crash-simulation test to the new path; keep the synchronous path available behind config |
| Event-log write cost eats the tick budget | Measure first; buffer and write off the tick thread with the same durability ordering the snapshot writer uses |
| Scheduler introduces a shared-state dependency nobody notices | A5.2 is designed to catch exactly this and is the phase's most important test |
| Multi-world runs exhaust host memory or disk | Configured worker cap, per-run disk budget, and a measured growth rate before any long campaign |
| Compute cost of the experiment campaigns exceeds what the homelab can supply | This is a real and unresolved risk. See `docs/20-risk-register.md`; it is not solved by this phase, only measured by it |

## Rollback

Every addition is behind config: pacing mode, worker count, event-log
enable, checkpoint mode. Disabling all of them returns the exact Phase 4
execution path, verified by A5.7. The event-log file is additive; snapshots
without a log reference remain valid.
