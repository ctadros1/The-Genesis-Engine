# Performance Strategy

## Principle

Build the simplest correct simulation, measure it on the deployment-shaped VM, then remove measured bottlenecks. Performance work must improve an observable constraint without weakening determinism, safety, or inspectability.

## The Second Scale Axis

The open-ended-evolution goal adds a dimension that matters more than
organism count: **generations reached, multiplied by seeds, multiplied by
ablation conditions.** The Phase 2 long run reached 127 ancestry generations
in 200,000 ticks and 405.7 s. Every acceptance criterion from Phase 7 onward
is of the form "the effect occurs in N of 30 seeds under condition A and
fewer under condition B", so the compute requirement is the run length times
the seed count times the condition count.

This makes throughput a correctness prerequisite rather than a nicety: a
campaign that cannot reach enough generations produces an underpowered null,
which is a worse outcome than a negative result because it cannot be
distinguished from one.

Phase 5 addresses it with headless execution decoupled from observer pacing,
an independent-world scheduler, and asynchronous checkpointing. It also
measures the ceiling, which is currently unknown: no deployment-shaped VM
measurement exists and the compute-cost risk in `docs/20-risk-register.md`
is recorded as unresolved rather than mitigated.

Performance optimization is no longer a phase of its own. It is a standing
discipline carried by every phase's Benchmark Impact section. The superseded
plan is preserved at
`planning/superseded/phase-5-performance-optimization.md`; profiling, SIMD,
and GPU evaluation remain backlog items requiring their own evidence.

## Cost Growth The Later Phases Introduce

Each is measured in its phase, not estimated here. Recorded so the
interactions are visible in one place, because they stack:

| Phase | Cost added | Where it lands |
|---|---|---|
| 7 | Contest resolution, threat sensing, carcass entities | `sense`, `apply`, entity count |
| 9 | Variable per-organism evaluation; batching by topology ID no longer works; diploid genomes roughly double genome storage | `controllers`, snapshot size, memory |
| 11 | New `learn` phase scaling with plastic edge count; learned state in snapshots | `learn`, snapshot size, checkpoint stall |
| 13 | K-nearest gathering and sorting; signal field accumulation and decay | `sense`, `apply`, `finalize` |
| 12 | Objects in the spatial index; decay; object table and terrain deltas in snapshots | all phases, snapshot size, restore time |
| 8 | Allometry, thermoregulation, hazard draws per organism per tick | `environment`, `apply`, `lifecycle` |
| 10 | Developmental growth per birth; per-organism cost becomes module-count dependent | `lifecycle`, `controllers` |
| 14 | Incremental ontogeny per tick; disease load | `apply`, `lifecycle` |

The snapshot budget is the one to watch. The Phase 4 record already shows
size dominated by per-organism genome arrays at roughly 2.8 KB each, and
Phases 9, 11, and 12 each add a growth term on top. The checkpoint budget is
re-verified in each of those phases rather than assumed to carry forward.

## Staged Plan

1. Establish a single-threaded deterministic baseline at 500-2,000 organisms.
2. Record tick time by phase, allocations, RSS, organism count, neural evaluations, save cost, and stream bandwidth.
3. Profile CPU and memory under fixed seeds and representative map/resource settings.
4. Adopt SoA/dense loops and spatial buckets where traces show cost.
5. Batch neural inputs/outputs and eliminate per-organism allocation.
6. Parallelize only systems with deterministic ordering/reduction policy and equality tests. **Now specified rather than aspirational**: intra-world parallelism has ADR-0026 and Phase 18. Estimated serial fraction near 3.1 percent and a ceiling near 9x at 12 threads, but that estimate mixes the Phase 1 and Phase 2 records and is an orientation, not evidence; Phase 18 measures the real split. The serial part is `apply`, which Phases 7, 12, and 13 each grow.
7. Evaluate SIMD after data layout is stable.
8. Compare GPU inference against CPU batching with end-to-end timing, not kernel-only claims.
9. Run independent experiment worlds on spare capacity only after one world is stable. Promoted from a late optimization to Phase 5 enabling work, because multi-seed experiment design depends on it.

## Performance Budget

The prototype should aim for p95 tick duration below the configured tick interval at its supported baseline population. Memory must remain bounded over long-running tests; stream bandwidth per observer must remain under a documented budget. Exact thresholds are benchmark-derived and recorded by hardware profile rather than invented in this document.

## Likely Hot Paths

- Neighbor queries and local sensing.
- Resource-cell updates.
- Neural input gathering and dense evaluation.
- Intent conflict resolution.
- Entity compaction/cleanup.
- Serialization/keyframe construction under many observers.

## Not Premature Optimizations

- GPU passthrough.
- A general ECS framework before a concrete benchmark.
- A distributed tick across nodes.
- Full-world GPU buffers synchronized every tick.
- Columnar analytics database before simple exports prove inadequate.

## Benchmark Discipline

Every benchmark records git revision, hardware/VM allocation, build profile, config hash, seed, warm-up duration, run duration, population statistics, observer count, and summary percentiles. Keep raw samples outside source control unless intentionally curated.

## Phase 2 Evidence Boundary

The Phase 2 benchmark record (ID in `research/performance-notes.md`)
measures controller evaluation as its own tick phase, sensor gathering as
the sense phase, pairing/ancestry inside apply/lifecycle, allocation
counts, RSS, and the offline similarity-analysis runtime separately from
tick cost. Phase 2 numbers are comparable to Phase 1 only for the
fixed-population scenarios and only with the added work qualified: Phase 2
adds per-organism sensing and a 20-16-12-12 controller evaluation that
Phase 1 does not perform.

## Phase 5: Two Scale Axes, And Which One Is Measured Here

Phase 5 makes explicit that this project has two scale axes and that the
second now matters more. Organism count is the familiar one. The other is
**generations reached, multiplied by seeds, multiplied by ablation
conditions**, and it is what an open-ended-evolution question actually
consumes.

Headless execution and the independent-world scheduler exist to move the
second axis. Measured on the development host
(`phase5-local-20260804T210059Z`): 8,805 ticks/s per world at the 500 tier
and 1,653 ticks/s at the 2,000 tier; 16 independent worlds reach 3.67x
aggregate throughput at 4 workers with 8.4 percent per-world degradation,
and 4.96x at 8 workers with 38 percent degradation, on a 12-core machine
with 4 performance cores.

**No supported campaign size is claimed from those numbers.** They are one
host and one filesystem, and the deployment-VM measurement is still an open
Phase 0 gate. The plan deliberately contains no acceptance criterion of the
form "achieves X worlds at Y ticks per second", because declaring a target
before measuring is the unmeasured scale claim `AGENTS.md` forbids.

Two methodological notes worth carrying forward, both learned by getting
them wrong first:

- **A percentile can be blind to the thing you are measuring.** With a
  checkpoint every 200 ticks, checkpoint ticks are 0.5 percent of the
  sample, so p95 over all ticks cannot see a checkpoint stall at all. The
  affected ticks have to be measured as their own population.
- **Report the noise floor, not just the effect.** A single with-versus-
  without comparison of event-log cost produced a negative overhead. The
  measurement is now five alternating repetitions reporting the median of
  each side and the observed spread, and the honest conclusion is that the
  cost is below the noise floor rather than that it is one percent.

Intra-world parallelism remains closed. Nothing in Phase 5 opens it; it
stays gated on ADR-0010's ordering and reduction evidence.

## Current Evidence Boundary

The first local record is `phase0-local-20260804T030100Z`, summarized in
`research/performance-notes.md`. It establishes harness behavior and a
development-host baseline only. It does not set organism capacity, a VM budget,
a mobile performance budget, or a WebGPU preference. The next comparable run
must keep the seed/config/method fixed or explicitly explain every difference.
