# Phase 0 Benchmark Outputs

Reproduce the complete local benchmark set from the repository root:

~~~sh
scripts/bootstrap-phase0-toolchain.sh
cd spikes/renderer-spike && npm ci && cd ../..
scripts/run-all-phase0-benchmarks.sh
~~~

The scripts create `benchmarks/raw/<benchmark-id>/` containing:

- deterministic clean-process fixture output;
- Rust raw per-sample CSV and summary JSON for 500/2,000 organisms;
- PixiJS/Chrome summary JSON for desktop/mobile-sized WebGL and WebGPU runs;
- browser screenshots and the local Vite log.

Raw results are intentionally ignored. Curated results, provenance,
limitations, and decisions belong in `research/performance-notes.md` and the
relevant ADRs. Never compare two records without checking seed, config hash,
toolchain, hardware, warm-up, sample count, browser, viewport, and backend.

## Phase 1 Benchmarks

Reproduce the Phase 1 kernel benchmark set from the repository root:

~~~sh
scripts/bootstrap-phase0-toolchain.sh
scripts/run-phase1-benchmarks.sh
~~~

The script writes `benchmarks/raw/<benchmark-id>/` containing the
clean-process determinism fixture plus, for 500 and 2,000 organisms, an
ecology scenario (reproduction on, population follows the live trajectory)
and a `-fixedpop` scenario (reproduction off). Each summary JSON records
toolchain, hardware, build profile, config hash, seed, warm-up, sampling
method, per-phase p50/p95/p99 tick timings, allocations per tick, RSS at
completion, and the raw CSV path.

## Phase 2 Benchmarks

Reproduce the Phase 2 set with `scripts/run-phase2-benchmarks.sh` (same
scenario structure, controllers enabled, benchmark schema version 2). The
`sense` phase records sensor gathering, the `controllers` phase records
controller evaluation, and each summary's `phase2` section records policy
versions, pairing/fault counters, and the offline similarity-analysis
runtime measured separately from tick cost. Phase 2 timings are comparable
to Phase 1 only for fixed-population scenarios and only with the added
controller work qualified.

