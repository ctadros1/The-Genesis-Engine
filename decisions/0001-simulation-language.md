# ADR-0001: Simulation Language

Status: Proposed
Date: 2026-08-03
Author: Project planning

## Context
The kernel needs predictable memory/concurrency behavior for a 2,000-organism prototype and potentially larger workloads, while remaining maintainable by future coding agents.

## Options Considered
- Rust stable with explicit data-oriented kernel.
- C++.
- Go.
- Python with a native acceleration boundary.

## Proposed Decision
Propose Rust for sim-core/server. Run a Phase 0 deterministic tick and snapshot spike before accepting.

## Consequences
Rust adds toolchain/ownership learning cost but reduces memory-safety risk and permits a single-language performance path.

## Performance Implications
Hypothesis: Rust SoA loops meet prototype needs without GPU; validate p95 tick/RSS on target-shaped VM.

## Operational Implications
One static/native service can simplify deployment, but toolchain/image size and cross-platform replay need documentation.

## Revisit Conditions
Phase 0 shows unacceptable development/debug cost or a credible alternative meets deterministic/scale needs materially better.

## Evidence Required To Accept

- Phase-specific tests and benchmark evidence.
- Compatibility and rollback impact.
- Explicit review/approval when production infrastructure is affected.

## Phase 0 Local Evidence

Benchmark `phase0-local-20260804T030100Z` ran the dependency-free Rust 1.97.1
SoA spike at 500 and 2,000 organisms. Tick p95 was 55.091 us and 410.671 us
respectively on the local M3 Pro, and all determinism/snapshot tests and Clippy
passed. This supports retaining Rust for the next comparison but does not
validate the deployment VM or production ecology. Status remains Proposed.

## Phase 1 Local Evidence

The dependency-free Rust `sim-core` kernel and `lifesim` CLI implement the
full Phase 1 ecology (worldgen, resources, movement, feeding, crowding,
reproduction, death) with strict determinism; formatting, Clippy, unit,
deterministic, long-run, and CLI integration suites all pass. The local
Phase 1 benchmark keeps tick p95 far below the 100 ms budget at both
documented tiers (see `research/performance-notes.md`). Deployment-VM
behavior is still unmeasured. Status remains Proposed.
