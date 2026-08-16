# File Manifest

This is the tracked project file inventory after the Phase 0 local bootstrap
and the 2026-08-04 goal-change documentation pass. It excludes Git internals,
local macOS metadata, dependencies, build output, workspace-local toolchains,
and generated raw benchmark artifacts.

**Reconciled against `git ls-files` on 2026-08-11.** Every tracked file
under `crates/`, `scripts/` and `experiments/` is now listed; the previous
revision had fallen roughly four phases behind, omitting the entire
`sim-analysis` crate, the schema-2 genome, morphology, plasticity and
mutable-world modules, and every campaign and result from Phases 8 to 11.
An inventory that lists half a tree is worse than no inventory, because it
reads as complete.

`crates/sim-core/src/config 2.rs` was deleted on 2026-08-04. It was a
filesystem sync artifact: a pre-Phase-2 snapshot of `config.rs`, verified to
be a strict subset (0 unique lines, 190 lines missing including the entire
`Phase2Config` section), not declared in `lib.rs`, and never compiled
because its name is not a valid Rust module path. Build, the full test
suite, both determinism fixtures, fmt, and clippy were re-verified after
removal.

`crates/sim-analysis/src/plasticity 2.rs` was deleted on 2026-08-11 for the
same reason and after the same checks. It was an iCloud sync conflict copy
created while two sessions edited `plasticity.rs` concurrently, verified
**byte-identical to `plasticity.rs` at commit `aaa260e`** (so strictly an
older snapshot), not declared in `crates/sim-analysis/src/lib.rs`, and never
compiled because its name is not a valid Rust module path. It had already
started appearing in source greps beside the live file, which is the hazard.
The full suite and all five fixture scripts were re-run after removal.

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

## Benchmark And Verification Scripts

- benchmarks/README.md
- scripts/bootstrap-phase0-toolchain.sh
- scripts/phase0-env.sh
- scripts/run-all-phase0-benchmarks.sh
- scripts/run-observer-e2e.sh
- scripts/run-phase0-benchmarks.sh
- scripts/run-phase1-benchmarks.sh
- scripts/run-phase2-benchmarks.sh
- scripts/run-phase3-benchmarks.sh
- scripts/run-phase4-benchmarks.sh
- scripts/run-phase5-benchmarks.sh
- scripts/run-phase6-benchmarks.sh
- scripts/run-phase7-benchmarks.sh
- scripts/run-phase8-benchmarks.sh
- scripts/run-phase9-benchmarks.sh
- scripts/run-phase10-benchmarks.sh
- scripts/run-phase11-benchmarks.sh
- scripts/run-renderer-benchmarks.sh

The five fixture scripts every change must keep passing, plus the Phase 0
clean-process check that predates them:

- scripts/verify-determinism.sh (Phase 0 spike harness)
- scripts/verify-phase1-determinism.sh (`0x1e3158a26afd3b39`)
- scripts/verify-phase2-determinism.sh (`0xff9dfcff5dffbf42`)
- scripts/verify-phase5-determinism.sh (scheduler equality and manifest identity)
- scripts/verify-phase9-determinism.sh (config `0x9abc0cd47914127f`, state `0x5f0c4e95e4f5170f`)
- scripts/verify-phase11-determinism.sh (config `0xae34cd2b6f7a3e13`, state `0x53b354bd94e82bcf`)

## Simulation Kernel (`sim-core`)

Phases 1 and 2:

- crates/sim-core/Cargo.toml
- crates/sim-core/src/lib.rs
- crates/sim-core/src/checksum.rs
- crates/sim-core/src/config.rs
- crates/sim-core/src/controller.rs
- crates/sim-core/src/genome.rs
- crates/sim-core/src/phase2.rs
- crates/sim-core/src/rng.rs
- crates/sim-core/src/save.rs
- crates/sim-core/src/similarity.rs
- crates/sim-core/src/world.rs
- crates/sim-core/src/worldgen.rs

Phase 6 (biomes, climate, origin modes), Phase 7 (contest), Phase 8
(physiology):

- crates/sim-core/src/climate.rs
- crates/sim-core/src/origin.rs
- crates/sim-core/src/contest.rs
- crates/sim-core/src/physiology.rs

Phase 9 (schema-2 genome), Phase 10 (morphology), Phase 11 (learning and
the measurement probe), Phase 12 (mutable world):

- crates/sim-core/src/genome2.rs
- crates/sim-core/src/schema2.rs
- crates/sim-core/src/meiosis.rs
- crates/sim-core/src/structmut.rs
- crates/sim-core/src/controller2.rs
- crates/sim-core/src/registry.rs
- crates/sim-core/src/morphology.rs
- crates/sim-core/src/morphstate.rs
- crates/sim-core/src/develop.rs
- crates/sim-core/src/plasticity.rs
- crates/sim-core/src/learnstate.rs
- crates/sim-core/src/actioncensus.rs
- crates/sim-core/src/terrainmod.rs

Kernel tests and in-tree benchmarks:

- crates/sim-core/tests/determinism.rs
- crates/sim-core/tests/genome_malformed_harness.rs
- crates/sim-core/tests/longrun.rs
- crates/sim-core/tests/property.rs
- crates/sim-core/tests/phase2_determinism.rs
- crates/sim-core/tests/phase2_longrun.rs
- crates/sim-core/tests/phase2_pairing.rs
- crates/sim-core/tests/phase2_similarity.rs
- crates/sim-core/tests/phase6_origin.rs
- crates/sim-core/tests/phase7_contest.rs
- crates/sim-core/tests/phase8_demography.rs
- crates/sim-core/tests/phase9_determinism.rs
- crates/sim-core/tests/phase9_genetics.rs
- crates/sim-core/tests/phase9_genome2.rs
- crates/sim-core/tests/phase9_world.rs
- crates/sim-core/tests/phase10_development.rs
- crates/sim-core/tests/phase10_world.rs
- crates/sim-core/tests/phase11_learning.rs
- crates/sim-core/tests/phase11_probe.rs
- crates/sim-core/tests/phase12_worldmod.rs
- crates/sim-core/tests/bench_phase6.rs
- crates/sim-core/tests/bench_phase7.rs
- crates/sim-core/tests/bench_phase8.rs
- crates/sim-core/tests/bench_phase9.rs
- crates/sim-core/tests/bench_phase10.rs
- crates/sim-core/tests/bench_phase11.rs
- crates/sim-core/tests/bench_phase12.rs

## CLI (`sim-cli`)

- crates/sim-cli/Cargo.toml
- crates/sim-cli/src/main.rs
- crates/sim-cli/tests/cli.rs
- crates/sim-cli/tests/phase5_cli.rs
- crates/sim-cli/tests/phase11_plasticity_cli.rs
- crates/sim-cli/tests/phase11_conjunction_cli.rs

## Persistence And Artifacts (`sim-persist`)

Snapshot codec (ALIF, format 5, with retained format 3 and format 4 readers
and writers), the event log (ALEV 1), the spatial sample
log (ALSS 1), and the per-individual action sample log (ALAC 1):

- crates/sim-persist/Cargo.toml
- crates/sim-persist/src/lib.rs
- crates/sim-persist/src/codec.rs
- crates/sim-persist/src/store.rs
- crates/sim-persist/src/checkpoint.rs
- crates/sim-persist/src/eventlog.rs
- crates/sim-persist/src/founders.rs
- crates/sim-persist/src/spatial.rs
- crates/sim-persist/src/actionlog.rs
- crates/sim-persist/tests/persistence.rs
- crates/sim-persist/tests/eventlog.rs
- crates/sim-persist/tests/founders.rs
- crates/sim-persist/tests/config_round_trip.rs
- crates/sim-persist/tests/phase11_action_section.rs
- crates/sim-persist/tests/phase12_format4.rs
- crates/sim-persist/tests/format5.rs
- crates/sim-persist/tests/bench_saves.rs
- crates/sim-persist/tests/bench_phase9_snapshot.rs
- crates/sim-persist/tests/bench_phase11_snapshot.rs

## Experiment Harness (`sim-experiment`)

- crates/sim-experiment/Cargo.toml
- crates/sim-experiment/src/lib.rs
- crates/sim-experiment/src/fields.rs
- crates/sim-experiment/src/campaign.rs
- crates/sim-experiment/src/scheduler.rs
- crates/sim-experiment/src/manifest.rs
- crates/sim-experiment/src/report.rs
- crates/sim-experiment/tests/phase5_determinism.rs
- crates/sim-experiment/tests/config_field_coverage.rs
- crates/sim-experiment/tests/spatial_sampling.rs
- crates/sim-experiment/tests/action_sampling.rs
- crates/sim-experiment/tests/bench_phase5.rs

## Offline Analysis (`sim-analysis`)

Analysis observes and never instructs (ADR-0016): `sim-core` does not depend
on this crate, it draws from no RNG stream, and its version strings are
excluded from the config hash.

- crates/sim-analysis/Cargo.toml
- crates/sim-analysis/src/lib.rs
- crates/sim-analysis/src/paired.rs
- crates/sim-analysis/src/power.rs
- crates/sim-analysis/src/spatial.rs
- crates/sim-analysis/src/demography.rs
- crates/sim-analysis/src/structure.rs
- crates/sim-analysis/src/morph.rs
- crates/sim-analysis/src/plasticity.rs
- crates/sim-analysis/src/conjunction.rs

## Observer Protocol And Server (Phase 3)

- crates/sim-protocol/Cargo.toml
- crates/sim-protocol/src/lib.rs
- crates/sim-server/Cargo.toml
- crates/sim-server/src/main.rs
- crates/sim-server/src/state.rs
- crates/sim-server/src/stream.rs
- crates/sim-server/tests/integration.rs
- crates/sim-server/tests/bench_observers.rs
- crates/sim-server/tests/phase5_acceleration.rs

## Campaigns And Recorded Results

Campaign sources are pre-registration artifacts: each is committed before
its run, embedded verbatim in the manifest it produces, and never edited
afterwards.

- experiments/phase7-c71-pilot.campaign
- experiments/phase7-c71-confirmatory.campaign
- experiments/phase8-extrinsic-sweep.campaign
- experiments/phase8-c81-confirmatory.campaign
- experiments/phase9-climate-probe.campaign
- experiments/phase9-ecology-sweep.campaign
- experiments/phase9-c91-confirmatory.campaign
- experiments/phase10-c103-confirmatory.campaign
- experiments/phase11-c111-confirmatory.campaign
- experiments/phase11-c111-agematched-preregistration.md
- experiments/phase11-reachability-preregistration.md

- experiments/results/phase7-c71-pilot-manifest.txt
- experiments/results/phase7-c71-pilot-spatial.txt
- experiments/results/phase7-c71-confirmatory-manifest.txt
- experiments/results/phase7-c71-confirmatory-spatial.txt
- experiments/results/phase8-extrinsic-sweep-manifest.txt
- experiments/results/phase8-c81-confirmatory-manifest.txt
- experiments/results/phase8-c81-confirmatory-demography.txt
- experiments/results/phase9-climate-probe-manifest.txt
- experiments/results/phase9-ecology-sweep-manifest.txt
- experiments/results/phase9-c91-confirmatory-manifest.txt
- experiments/results/phase9-c91-confirmatory-structure.txt
- experiments/results/phase10-c103-confirmatory-manifest.txt
- experiments/results/phase10-c103-confirmatory-morph.txt
- experiments/results/phase10-benchmark-measurements.txt
- experiments/results/phase11-c111-confirmatory-manifest.txt
- experiments/results/phase11-c111-confirmatory-plasticity.txt
- experiments/results/phase11-c111-confirmatory-findings.txt
- experiments/results/phase11-c111-reanalysis-v2-plasticity.txt
- experiments/results/phase11-conjunction-census.txt
- experiments/results/phase11-conjunction-census-raw.txt
- experiments/results/phase11-benchmark-measurements.txt

The Phase 11 campaign's own `.alac` and `.alif` artifacts (1.8 GB) are
**not** tracked. That is a recorded limitation rather than an oversight:
D-100 could not measure the age trend in the campaign's own worlds because
the `.alac` files were not retained, and a future campaign whose statistic
may need re-deriving should keep them.

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
- docs/27-time-scale-and-pacing.md

## Planning

- planning/backlog.md
- planning/phase-0-discovery.md
- planning/phase-1-minimum-simulation.md
- planning/phase-2-evolution.md
- planning/phase-3-live-observer.md
- planning/phase-4-persistence-and-analytics.md
- planning/phase-5-headless-scale-and-experiments.md
- planning/phase-6-biomes-climate-and-origins.md
- planning/phase-7-territory-and-conflict.md
- planning/phase-8-demography-and-life-history.md
- planning/phase-9-evolvable-genome.md
- planning/phase-10-modular-morphology.md
- planning/phase-11-lifetime-learning.md
- planning/phase-12-mutable-world-and-artifacts.md
- planning/phase-13-social-channel.md
- planning/phase-14-ontogeny-and-sexual-selection.md
- planning/phase-15-abiogenesis-and-unicellular-regime.md
- planning/phase-16-multicellularity-transition.md
- planning/phase-17-era-and-tradition-detection.md
- planning/phase-18-intra-world-parallelism.md
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

- specifications/appearance-derivation.md
- specifications/artifact-and-material-ontology.md
- specifications/biome-and-climate.md
- specifications/determinism-extensions.md
- specifications/entity-component-model.md
- specifications/era-and-tradition-detection.md
- specifications/event-schema.md
- specifications/experiment-config-schema.md
- specifications/genome-schema-2.md
- specifications/long-horizon-soak.md
- specifications/metrics-schema.md
- specifications/morphology-and-development.md
- specifications/mutable-world-state.md
- specifications/neural-network-schema.md
- specifications/organism-genome.md
- specifications/plasticity-and-learning.md
- specifications/simulation-tick.md
- specifications/social-signal-channel.md
- specifications/spatial-sample-format.md
- specifications/unicellular-regime.md
- specifications/websocket-protocol.md
- specifications/world-origin-modes.md
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
- decisions/0018-scaffolded-possibility-space.md
- decisions/0019-modular-morphology-encoding.md
- decisions/0020-two-regime-simulation.md
- decisions/0021-world-origin-modes.md
- decisions/0022-research-findings-adopted-and-declined.md
- decisions/0023-flagship-and-campaign-worlds.md
- decisions/0024-voxel-rendering-and-derived-appearance.md
- decisions/0025-demography-before-culture.md
- decisions/0026-intra-world-parallelism.md
- decisions/0027-live-rule-zero-and-the-rule-id-space.md
- decisions/README.md
- decisions/adr-template.md

## Project Skills And Commissioned Research

The six `genesis-*` skills are tracked because they carry the commissioned
research reviews that govern engine design (ADR-0022, indexed in
`research/deep-research-index.md`). Generic third-party skills remain
untracked local tooling.

- .agents/skills/genesis-artificial-genetics/ (SKILL.md + references/)
- .agents/skills/genesis-cumulative-culture/
- .agents/skills/genesis-experimental-methodology/
- .agents/skills/genesis-mutable-world-tool-use/
- .agents/skills/genesis-neuroevolution/
- .agents/skills/genesis-social-organization-territory-conflict/
- .claude/skills/genesis-* (symlinks into .agents/skills/)

## Research

- research/deep-research-index.md
- research/neural-network-options.md
- research/performance-notes.md
- research/related-projects.md
- research/rendering-options.md
- research/simulation-engine-options.md
