# File Manifest

This is the tracked project file inventory after the Phase 0 local bootstrap
and the 2026-08-04 goal-change documentation pass. It excludes Git internals,
local macOS metadata, dependencies, build output, workspace-local toolchains,
and generated raw benchmark artifacts.

`crates/sim-core/src/config 2.rs` was deleted on 2026-08-04. It was a
filesystem sync artifact: a pre-Phase-2 snapshot of `config.rs`, verified to
be a strict subset (0 unique lines, 190 lines missing including the entire
`Phase2Config` section), not declared in `lib.rs`, and never compiled
because its name is not a valid Rust module path. Build, the full test
suite, both determinism fixtures, fmt, and clippy were re-verified after
removal.

Stray `.DS_Store` files remain in the working tree and are excluded from
this manifest.

## Root

- .gitignore
- AGENTS.md
- CLAUDE.md
- CODEX.md
- FILE_MANIFEST.md
- README.md
- Cargo.lock
- Cargo.toml
- rust-toolchain.toml

## Benchmark Harness

- benchmarks/README.md
- scripts/bootstrap-phase0-toolchain.sh
- scripts/phase0-env.sh
- scripts/run-all-phase0-benchmarks.sh
- scripts/run-phase0-benchmarks.sh
- scripts/run-observer-e2e.sh
- scripts/run-phase1-benchmarks.sh
- scripts/run-phase2-benchmarks.sh
- scripts/run-phase3-benchmarks.sh
- scripts/run-phase4-benchmarks.sh
- scripts/run-renderer-benchmarks.sh
- scripts/verify-determinism.sh
- scripts/verify-phase1-determinism.sh
- scripts/verify-phase2-determinism.sh

## Simulation Kernel (Phases 1 And 2)

- crates/sim-core/Cargo.toml
- crates/sim-core/src/lib.rs
- crates/sim-core/src/checksum.rs
- crates/sim-core/src/config.rs
- crates/sim-core/src/controller.rs
- crates/sim-core/src/genome.rs
- crates/sim-core/src/phase2.rs
- crates/sim-core/src/rng.rs
- crates/sim-core/src/similarity.rs
- crates/sim-core/src/world.rs
- crates/sim-core/src/worldgen.rs
- crates/sim-core/tests/determinism.rs
- crates/sim-core/tests/genome_malformed_harness.rs
- crates/sim-core/tests/longrun.rs
- crates/sim-core/tests/phase2_determinism.rs
- crates/sim-core/tests/phase2_longrun.rs
- crates/sim-core/tests/phase2_pairing.rs
- crates/sim-core/tests/phase2_similarity.rs
- crates/sim-core/tests/property.rs
- crates/sim-cli/Cargo.toml
- crates/sim-cli/src/main.rs
- crates/sim-cli/tests/cli.rs
- crates/sim-core/src/save.rs

## Persistence (Phase 4)

- crates/sim-persist/Cargo.toml
- crates/sim-persist/src/lib.rs
- crates/sim-persist/src/codec.rs
- crates/sim-persist/src/store.rs
- crates/sim-persist/tests/bench_saves.rs
- crates/sim-persist/tests/persistence.rs

## Observer Protocol And Server (Phase 3)

- crates/sim-protocol/Cargo.toml
- crates/sim-protocol/src/lib.rs
- crates/sim-server/Cargo.toml
- crates/sim-server/src/main.rs
- crates/sim-server/src/state.rs
- crates/sim-server/src/stream.rs
- crates/sim-server/tests/bench_observers.rs
- crates/sim-server/tests/integration.rs

## Observer App (Phase 3)

- apps/observer/index.html
- apps/observer/package.json
- apps/observer/package-lock.json
- apps/observer/playwright.config.ts
- apps/observer/tsconfig.json
- apps/observer/src/main.ts
- apps/observer/src/protocol.ts
- apps/observer/src/render.ts
- apps/observer/src/style.css
- apps/observer/src/vite-env.d.ts
- apps/observer/tests/bench.spec.ts
- apps/observer/tests/e2e.spec.ts

## Phase 0 Spikes

- spikes/sim-spike/Cargo.toml
- spikes/sim-spike/README.md
- spikes/sim-spike/src/lib.rs
- spikes/sim-spike/src/main.rs
- spikes/renderer-spike/index.html
- spikes/renderer-spike/package-lock.json
- spikes/renderer-spike/package.json
- spikes/renderer-spike/playwright.config.ts
- spikes/renderer-spike/README.md
- spikes/renderer-spike/scripts/run-benchmark.mjs
- spikes/renderer-spike/src/main.ts
- spikes/renderer-spike/src/style.css
- spikes/renderer-spike/src/vite-env.d.ts
- spikes/renderer-spike/tests/smoke.spec.ts
- spikes/renderer-spike/tsconfig.json

## Documentation

- docs/00-project-vision.md
- docs/01-user-requirements.md
- docs/02-scope-and-non-goals.md
- docs/03-system-architecture.md
- docs/04-simulation-model.md
- docs/05-world-model.md
- docs/06-organism-model.md
- docs/07-neural-network-design.md
- docs/08-genetics-and-evolution.md
- docs/09-species-and-lineage.md
- docs/10-observer-interface.md
- docs/11-api-and-streaming-protocol.md
- docs/12-data-storage-and-saves.md
- docs/13-performance-strategy.md
- docs/14-testing-strategy.md
- docs/15-security-model.md
- docs/16-observability.md
- docs/17-proxmox-deployment.md
- docs/18-development-environment.md
- docs/19-implementation-roadmap.md
- docs/20-risk-register.md
- docs/21-open-questions.md
- docs/22-decision-log.md
- docs/23-glossary.md
- docs/24-codex-execution-playbook.md
- docs/25-emergence-and-epistemic-position.md
- docs/26-biological-realism-policy.md

## Planning

- planning/backlog.md
- planning/phase-0-discovery.md
- planning/phase-1-minimum-simulation.md
- planning/phase-2-evolution.md
- planning/phase-3-live-observer.md
- planning/phase-4-persistence-and-analytics.md
- planning/phase-5-headless-scale-and-experiments.md
- planning/phase-6-territory-and-conflict.md
- planning/phase-7-evolvable-genome.md
- planning/phase-8-lifetime-learning.md
- planning/phase-9-social-channel.md
- planning/phase-10-mutable-world-and-artifacts.md
- planning/phase-11-physiology-and-life-history.md
- planning/phase-12-era-and-tradition-detection.md
- planning/superseded/phase-5-performance-optimization.md
- planning/superseded/phase-6-advanced-ecosystems.md

## Infrastructure

- infrastructure/README.md
- infrastructure/backup-and-recovery.md
- infrastructure/deployment-plan.md
- infrastructure/gpu-evaluation.md
- infrastructure/monitoring-plan.md
- infrastructure/network-plan.md
- infrastructure/storage-plan.md
- infrastructure/vm-requirements.md

## Specifications

- specifications/artifact-and-material-ontology.md
- specifications/determinism-extensions.md
- specifications/entity-component-model.md
- specifications/era-and-tradition-detection.md
- specifications/event-schema.md
- specifications/experiment-config-schema.md
- specifications/genome-schema-2.md
- specifications/metrics-schema.md
- specifications/mutable-world-state.md
- specifications/neural-network-schema.md
- specifications/organism-genome.md
- specifications/plasticity-and-learning.md
- specifications/simulation-tick.md
- specifications/social-signal-channel.md
- specifications/websocket-protocol.md
- specifications/world-save-format.md

## Prompts

- prompts/codex-architecture-review.md
- prompts/codex-bootstrap.md
- prompts/codex-deployment.md
- prompts/codex-long-running-stability.md
- prompts/codex-performance-audit.md
- prompts/codex-persistence.md
- prompts/codex-phase-1.md
- prompts/codex-phase-2.md
- prompts/codex-release-readiness.md
- prompts/codex-review.md
- prompts/codex-save-format-migration.md
- prompts/codex-security-review.md
- prompts/codex-ui.md

## Decisions

- decisions/0001-simulation-language.md
- decisions/0002-world-coordinate-model.md
- decisions/0003-entity-storage-model.md
- decisions/0004-neural-network-implementation.md
- decisions/0005-frontend-rendering.md
- decisions/0006-streaming-protocol.md
- decisions/0007-persistence-format.md
- decisions/0008-primary-deployment-node.md
- decisions/0009-gpu-usage.md
- decisions/0010-determinism-policy.md
- decisions/0011-phase2-numeric-policy.md
- decisions/0012-emergence-in-authored-possibility-space.md
- decisions/0013-variable-topology-encoding.md
- decisions/0014-lifetime-learning-model.md
- decisions/0015-mutable-world-state-and-save-format-2.md
- decisions/0016-analysis-observes-never-instructs.md
- decisions/0017-biological-realism-policy.md
- decisions/README.md
- decisions/adr-template.md

## Research

- research/neural-network-options.md
- research/performance-notes.md
- research/related-projects.md
- research/rendering-options.md
- research/simulation-engine-options.md
