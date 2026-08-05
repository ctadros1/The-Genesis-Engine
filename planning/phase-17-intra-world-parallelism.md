# Phase 17: Intra-World Parallelism

**Cross-cutting infrastructure, not a behavioral phase.** Numbered 17 to
avoid renumbering; it does not execute last. Recommended execution: **after
Phase 13a (demography), before Phase 12 (social)**  -  see Execution Position
below. Numbering is provisional; see Numbering below.

Status: planned, not started. Policy version `lifesim-parallel-v1`.
Decision: ADR-0026, which amends
`specifications/determinism-extensions.md` Rule 10.

## Problem

One world runs on one core. Phase 5's scheduler parallelizes across worlds,
which is right for campaigns and does nothing for a flagship world: on a
12-core Xeon E5-2680 v4, eleven cores are idle while the flagship runs.

That caps a single world at an estimated 10,000 to 30,000 organisms at 1x.
Population is the binding constraint on the project's central question, and
the constraint is quantitative rather than vague: real human populations
near 4,000 individuals lose technology rather than accumulate it. A world
that cannot exceed tens of thousands may sit below the threshold at which
cumulative culture is sustainable at all.

Rule 10 currently forbids this outright, and ADR-0026 amends it.

## Execution Position

This phase has no behavioral dependency, so its position is a judgment about
when the ceiling starts to bind rather than a dependency argument.

- **After Phase 13a (demography).** Raising the population ceiling before
  the population is demographically regulated would scale a world that is
  99.9 percent starvation mortality. More organisms starving is not more
  science. 13a also raises `max_entities` above the ecological equilibrium,
  which is what makes a higher ceiling meaningful.
- **Before Phase 12 (social).** Phase 12's criteria are the ones that need
  population most, and running them under a ceiling that is itself the
  suspected blocker would confound a null.
- **Interacts badly with Phases 7, 11, and 12**, which each add interaction
  resolution to `apply`, the serial part. That interaction is measured here
  and re-measured after each of them; see Risks.

Doing it earlier is defensible if the flagship world becomes the priority
sooner. Doing it later wastes the culture-stack campaigns.

## Scope

- A parallel execution mode for the tick, config-gated and disabled by
  default.
- Fixed partition count `P`, independent of thread count, in the config
  hash.
- Partition assignment as a pure function of stable object ID.
- Per-partition intent buffers; no worker writes world state.
- Canonical merge in ascending `(partition_index, object_id)` order.
- Fixed-topology reduction trees over `P` for any cross-partition
  accumulation.
- Cross-partition conflict resolution under the existing complete policies.
- Thread count recorded as an execution-class field in benchmarks and
  manifests.

## Non-Goals

- **No change to any behavioral rule.** This phase must be a pure
  performance change: with parallelism disabled, and with it enabled, the
  same seed and config produce the same world.
- **No optimistic parallel discrete-event simulation.** No speculative
  execution, no rollback, no anti-events. The methodology review's argument
  applies: rollback state and deterministic re-execution integrate badly
  with learning, mutation, and the event log, and a synchronous phased model
  is easier to verify and compatible with exact checkpointing.
- **No cross-world parallelism changes.** Phase 5's scheduler is unaffected
  and its A5.2 criterion continues to hold.
- **No GPU.** Separate decision, separate evidence.
- **No performance claim before measurement.** The ~9x Amdahl estimate in
  ADR-0026 is a hypothesis, and it mixes two benchmark records.
- No parallelism enabled for campaign worlds by default; they stay
  single-threaded and remain the basis for every claim (ADR-0023).

## Prerequisites

- **Phase 5**, for the benchmark harness, the manifest fields, and the
  comparison report that must learn to refuse across execution classes.
- **Phase 13a**, for the reasons in Execution Position.

## Determinism Notes

This phase is almost entirely a determinism problem, so these are
requirements rather than notes.

- **No new RNG stream.** Every draw remains keyed by
  `(seed, tick, system, subject, draw_index)` and is therefore already
  independent of evaluation order. A parallel worker computing a draw gets
  the same value as a serial one.
- **Rules 4 and 5 already have the required shape.** Perception and learning
  read frozen prior state and commit after; candidate sets are sorted and
  truncated before selection. This phase generalizes that shape to the whole
  tick rather than inventing it.
- **Rule 6 is unchanged and is the subtle killer.** Per-node float summation
  stays pinned to ascending `homology_id` order. A partitioned reduction is
  exactly where non-associativity bites, and the mitigation is that it never
  applies across organisms: per-organism float work lives entirely inside
  one partition.
- **Cross-partition reductions are integer.** The ledger is `i128` and every
  state accumulator is fixed point, so these reductions are order-independent
  by construction. This is the property that makes thread-count invariance
  reachable, and it is a payoff from ADR-0011.
- **Overflow behavior is explicit.** Integer reductions are only
  order-independent if overflow is defined; checked or wrapping arithmetic
  is specified, never build-profile-dependent.
- **Partition index is `f(object_id, P)`**, never an array slice and never a
  function of thread count.
- **The merge is canonical**, in ascending `(partition_index, object_id)`,
  never completion order.
- Checksum composition is unchanged. Parallelism adds no state.

## Acceptance Criteria

**Primary endpoint: C17.1.** Acceptance is conjunctive; a good speedup
number does not rescue a determinism failure. Seed floor 30 worlds for the
statistical criteria; the determinism criteria are exact and need no seed
count beyond the fixtures.

Conditions:

- **S**: parallelism disabled, single-threaded. The reference.
- **T1 / T4 / T12**: parallelism enabled at 1, 4, and 12 threads.

Criteria:

- [ ] **C17.1 Determinism per thread count (primary).** For each of T1, T4,
      and T12, two clean processes at that thread count produce identical
      final state checksums, over a run long enough to expose drift  -  at
      minimum the 864,000-tick release horizon, and at Soak-7 if that tier
      exists by then. A failure here fails the phase regardless of speedup.
- [ ] **C17.2 Thread-count invariance (the Tier 1 claim).** T1, T4, and T12
      produce identical final state checksums to each other **and to S**. If
      this passes, thread count is a scheduling detail and stays out of the
      config hash. If it fails, the phase falls back to Tier 2: thread count
      enters the config hash, a different thread count becomes a different
      replay lineage, and that degradation is recorded in the decision log
      rather than absorbed.
- [ ] **C17.3 Fixtures preserved.** With parallelism disabled,
      `0x1e3158a26afd3b39` and `0xff9dfcff5dffbf42` reproduce from clean
      processes.
- [ ] **C17.4 Partition-count invariance is explicitly *not* claimed.**
      Changing `P` may change results, because it changes reduction tree
      topology. `P` is therefore in the config hash and a different `P` is a
      different lineage. Verify that two different `P` values produce
      different config hashes, so this can never happen silently.
- [ ] **C17.5 The comparison report refuses across execution classes.**
      Under the Tier 2 fallback, runs at different thread counts are
      different experiments and the report refuses to aggregate them, using
      the same mechanism that already refuses two conditions that are
      secretly the same experiment (D-046). Under Tier 1 the report may
      aggregate across thread counts and must still record them.
- [ ] **C17.6 Serial fraction and speedup measured, not assumed.** Report
      the per-phase parallel and serial split at the supported tiers, the
      speedup curve across 1, 2, 4, 8, and 12 threads, and the population
      **crossover below which parallelism is a net loss**. A speedup number
      without the crossover is incomplete, because the default must be
      "disabled" and the operator needs to know when to turn it on.
- [ ] **C17.7 No behavioral change.** Beyond checksum equality, event
      streams from S and T12 are compared and must be identical in content
      and order. Checksums could in principle agree while event ordering
      differed, and the event log is what Phase 16 analysis reads.
- [ ] **C17.8 Conflict resolution is order-independent.** Contested
      acquisition, contested pairing, and simultaneous damage produce
      identical outcomes under S and T12, and under a deliberately perturbed
      thread schedule. This is tested directly rather than inferred from
      aggregate checksums, because a rare contention path may not appear in
      a fixture run.
- [ ] **C17.9 Memory and peak RSS bounded.** Per-partition intent buffers
      add peak memory; report it, and confirm it does not break the
      checkpoint budget at the supported tier.

## Test Plan

- **Determinism**: C17.1, C17.2, C17.3 as automated clean-process tests, not
  manual procedures. These are the phase.
- **Adversarial scheduling**: run with a deliberately perturbed or
  randomized thread schedule and require identical results. A test that only
  ever sees a well-behaved schedule proves little.
- **Storage permutation**: permute stored organism order and require
  identical results at every thread count, extending the existing Rule 4
  obligation.
- **Contention**: scripted scenarios that force contested pickup, contested
  pairing, and simultaneous damage at high density, compared S versus T12.
- **Property**: partition assignment is total, stable, and independent of
  thread count for arbitrary ID sets.
- **Overflow**: integer reduction paths tested at saturation boundaries.
- **Long run**: the release-horizon run at T12 with invariants checked at
  the existing cadence.
- **Regression**: the full workspace suite passes at both S and T12.

## Benchmark Impact

New record `phase17-local-<timestamp>`. This phase exists to move a
performance number, so the benchmark *is* the deliverable alongside the
determinism proof.

Record: per-phase parallel and serial split at 2,000, 20,000, and the
highest reachable population; speedup at 1, 2, 4, 8, 12 threads; the
crossover population; peak RSS and intent-buffer memory; barrier and merge
overhead as a share of tick; memory-bandwidth saturation evidence for the
environment phase specifically, since it is a full-grid scan and is the most
likely phase to stop scaling.

Also record the **serial fraction attributable to `apply`**, separately,
because that is the number that later phases will grow and it is the leading
indicator for how much of this phase's benefit survives Phases 11 and 12.

Benchmark schema increments; earlier records stay valid and are comparable
only within their own schema version.

## Documentation Updates

`specifications/determinism-extensions.md` Rule 10 (amended by this work),
`docs/13-performance-strategy.md`, `docs/03-system-architecture.md`,
`specifications/simulation-tick.md`, `specifications/experiment-config-schema.md`
(partition count, thread count as execution class),
`specifications/metrics-schema.md`, `docs/27-time-scale-and-pacing.md`
(the pacing table's ticks-per-second assumption changes),
`research/performance-notes.md`, decision log, ADR-0026.

## Risks

| Risk | Mitigation |
|---|---|
| **Tier 1 fails and the fallback costs a lineage variable.** Thread count in the config hash means hardware changes break a months-long flagship world's lineage | C17.2 determines this before anything depends on it. The fallback is defined rather than improvised, and its cost is recorded in the decision log. Tier 3 is explicitly not available (ADR-0026) |
| **A rare contention path is order-dependent and no fixture exercises it** | C17.8 tests contention directly with scripted high-density scenarios rather than inferring safety from checksum equality on a typical run |
| **The serial fraction grows until the speedup does not justify the complexity.** Phases 7, 11, and 12 all add conflict resolution to `apply`  -  the phases this work enables are the ones that erode it | Measure `apply` separately (Benchmark Impact) and re-measure after each. If the ceiling falls below a stated threshold, the honest response is to disable parallelism rather than defend it |
| Memory bandwidth binds before core count, so environment does not scale | Measured explicitly; if confirmed, environment stays serial or gets a dirty-cell strategy and the achievable speedup is restated |
| **Parallelism is a net loss at small populations** and gets enabled by default anyway | Default is disabled; C17.6 requires the crossover to be reported, not just the peak speedup |
| A future phase adds a cross-organism float reduction and silently degrades Tier 1 | Recorded as a standing obligation in the amended Rule 10: any new cross-organism reduction is integer or uses a fixed-topology tree |
| Event ordering differs while checksums agree, corrupting Phase 16 analysis | C17.7 compares event streams directly, not just checksums |
| The complexity introduces a defect in the single-threaded path | Parallelism is config-gated and inert when disabled; C17.3 proves the disabled path is byte-identical to today |

## Rollback

One config section. Disabled, the kernel takes the existing single-threaded
path and both fixtures reproduce exactly. The parallel path can be removed
entirely without touching any behavioral rule, because by C17.7 it produces
identical results  -  that is the property that makes rollback safe and is
worth preserving deliberately rather than as a side effect.
