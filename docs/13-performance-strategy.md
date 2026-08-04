# Performance Strategy

## Principle

Build the simplest correct simulation, measure it on the deployment-shaped VM, then remove measured bottlenecks. Performance work must improve an observable constraint without weakening determinism, safety, or inspectability.

## The Second Scale Axis

The open-ended-evolution goal adds a dimension that matters more than
organism count: **generations reached, multiplied by seeds, multiplied by
ablation conditions.** The Phase 2 long run reached 127 ancestry generations
in 200,000 ticks and 405.7 s. Every acceptance criterion from Phase 7 onward
is of the form "the effect occurs in N of 12 seeds under condition A and
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
| 8 | Variable per-organism evaluation; batching by topology ID no longer works; diploid genomes roughly double genome storage | `controllers`, snapshot size, memory |
| 10 | New `learn` phase scaling with plastic edge count; learned state in snapshots | `learn`, snapshot size, checkpoint stall |
| 11 | K-nearest gathering and sorting; signal field accumulation and decay | `sense`, `apply`, `finalize` |
| 12 | Objects in the spatial index; decay; object table and terrain deltas in snapshots | all phases, snapshot size, restore time |
| 13 | Allometry, thermoregulation, growth, hazard draws per organism per tick | `environment`, `apply`, `lifecycle` |

The snapshot budget is the one to watch. The Phase 4 record already shows
size dominated by per-organism genome arrays at roughly 2.8 KB each, and
Phases 8, 8, and 10 each add a growth term on top. The checkpoint budget is
re-verified in each of those phases rather than assumed to carry forward.

## Staged Plan

1. Establish a single-threaded deterministic baseline at 500-2,000 organisms.
2. Record tick time by phase, allocations, RSS, organism count, neural evaluations, save cost, and stream bandwidth.
3. Profile CPU and memory under fixed seeds and representative map/resource settings.
4. Adopt SoA/dense loops and spatial buckets where traces show cost.
5. Batch neural inputs/outputs and eliminate per-organism allocation.
6. Parallelize only systems with deterministic ordering/reduction policy and equality tests.
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

## Current Evidence Boundary

The first local record is `phase0-local-20260804T030100Z`, summarized in
`research/performance-notes.md`. It establishes harness behavior and a
development-host baseline only. It does not set organism capacity, a VM budget,
a mobile performance budget, or a WebGPU preference. The next comparable run
must keep the seed/config/method fixed or explicitly explain every difference.
