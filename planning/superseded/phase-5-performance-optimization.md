> **Superseded 2026-08-04.** This plan predates the open-ended-evolution goal
> recorded in `docs/25-emergence-and-epistemic-position.md` and ADR-0012. It is
> preserved unmodified below for history. Do not execute it as written.
>
> Successors: see `docs/19-implementation-roadmap.md` for the current phase
> order. Performance work is now a standing discipline carried by every phase's
> Benchmark Impact section plus `planning/backlog.md`; the headless-throughput
> and independent-world parts moved into
> `planning/phase-5-headless-scale-and-experiments.md`. The advanced-ecosystem
> slices are now named phases 6 through 12.

# Phase 5: Performance Optimization

## Purpose
Increase supported scale only from measured bottlenecks while retaining deterministic correctness and operational simplicity.

## Scope
- Profiling, data layout refinement, spatial-index tuning, safe parallelism, batching, SIMD and optional GPU comparison.

## Non-Goals
- Unmeasured rewrite, distributed single-world tick, mandatory GPU passthrough.

## Dependencies
- Stable correctness suite, benchmark harness, deployment-shaped VM profile, Phase 4 save/restore.

## Deliverables
- Profile evidence, before/after benchmark reports, capacity tiers, regression thresholds.
- Accepted/rejected SIMD/GPU decisions with operational analysis.

## Technical Tasks
1. Profile representative scenarios before each optimization class.
2. Optimize only dominant phase costs and rerun deterministic tests.
3. Tune spatial buckets and dense views using density distributions.
4. Evaluate parallel systems with explicit ordering/reduction policy.
5. Run CPU SIMD and GPU end-to-end comparisons only when justified.

## Acceptance Criteria
- [ ] Every performance claim includes reproducible evidence.
- [ ] Correctness/replay policy is unchanged or explicitly versioned.
- [ ] No memory/bandwidth regression hides behind a throughput gain.
- [ ] Supported scale tiers are expressed as benchmarked ranges.

## Test Requirements
- Full deterministic suite and long-run stability after each kernel change.
- Slow-observer/save/restore regressions.
- GPU fallback and failure recovery if GPU path is evaluated.

## Benchmark Requirements
- Tick p50/p95/p99, RSS, allocation, CPU, bandwidth at tiered populations.
- Independent-world throughput and host contention.
- CPU versus GPU end-to-end cost.

## Documentation Updates
- Update performance strategy, GPU evaluation, capacity table, ADRs, benchmark notes.

## Risks
- Optimizations reduce reproducibility or code clarity.
- Host noise produces misleading benchmark gains.

## Rollback Strategy
Keep the last measured stable path available and use feature flags/config to disable risky optimization. Do not remove baseline tests.

## Suggested Codex Prompt
Use prompts/codex-performance-audit.md. Profile first, change one cause, then prove correctness and net benefit.
