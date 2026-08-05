# Backlog

## Goal Change, 2026-08-04

The project's long-term ambition changed: organisms should be able to evolve
toward tool use, persistent structures, transmitted knowledge, technological
accumulation, territoriality, and organized inter-group conflict, none of it
scripted as stages. The governing philosophy is **author physics, never
progress** (ADR-0012), and the honest epistemic position is in
`docs/25-emergence-and-epistemic-position.md`. Biology and genetics are to be
modelled as realistically as determinism and budget allow (ADR-0017).

This was a documentation, specification, and planning change only. **No code
was modified.** Phases 0 to 4 records, fixtures (`0x1e3158a26afd3b39`,
`0xff9dfcff5dffbf42`), and every benchmark ID are preserved exactly. All
ADRs remain Proposed.

Phases 5, 6, and 7 are complete. Phases 8 to 18 are planned and none has
started:

| Phase | Subject | Plan |
|---|---|---|
| 5 | Headless scale and multi-world experiments - **done** | `phase-5-headless-scale-and-experiments.md` |
| 6 | Biomes, climate drift, and world origin modes - **done** | `phase-6-biomes-climate-and-origins.md` |
| 7 | Territory, contest, and damage - **done** | `phase-7-territory-and-conflict.md` |
| 8 | Demography and life history | `phase-8-demography-and-life-history.md` |
| 9 | Evolvable genome: diploid genetics and variable topology | `phase-9-evolvable-genome.md` |
| 10 | Modular morphology and development | `phase-10-modular-morphology.md` |
| 11 | Lifetime learning | `phase-11-lifetime-learning.md` |
| 12 | Mutable world and artifacts | `phase-12-mutable-world-and-artifacts.md` |
| 13 | Social channel | `phase-13-social-channel.md` |
| 14 | Ontogeny and sexual selection | `phase-14-ontogeny-and-sexual-selection.md` |
| 15 | Abiogenesis and the unicellular regime | `phase-15-abiogenesis-and-unicellular-regime.md` |
| 16 | The multicellularity transition | `phase-16-multicellularity-transition.md` |
| 17 | Offline era and tradition detection | `phase-17-era-and-tradition-detection.md` |
| 18 | Intra-world parallelism - **cross-cutting; after 8, before 13** | `phase-18-intra-world-parallelism.md` |

The former Phase 5 (performance optimization) and Phase 7 (advanced
ecosystems) plans are superseded and preserved unmodified under
`planning/superseded/`. Performance work is now a standing discipline in
every phase's Benchmark Impact section.

## Current Status

**Phase 7 is complete** (2026-08-05), benchmark
`phase7-local-20260805T025643Z`. The primary endpoint C7.1 is measured and
met: 52 of 60 worlds on the aggregation index and 44 of 60 on the encounter
index, against a prespecified bar of 40, conjunctively, with zero worlds
excluded. The index it needed is new (`crates/sim-analysis`,
`lifesim-spatial-index-v1`), computed offline from a new versioned
position-sample artifact, **with no kernel change** - both fixtures are
untouched.

What the phase established is narrower than the headline. The aggregation
half of C7.1 is confounded with population and is reproduced by an
ecological control with no contest in it; the encounter half runs *against*
its confound and is the load-bearing result: **damage reduces short-range
co-occurrence, conditional on position, by about a quarter.** Damage rather
than the attack action does it - condition C fires attacks at thirteen times
the rate with damage set to zero and moves neither index. C7.2's tolerance
clause and C7.3 remain **unmet rather than adjusted**; C7.3's failure has
moved from its saturation clause to its contention-correlation clause, and
the cost-structure question is settled as **no change**
(D-060, D-061, D-062).

**Phase 6 is complete** (2026-08-04), benchmark
`phase6-local-20260804T224418Z`.

### Prior: Phase 6 detail The
climate and biome half is built and verified: the seven-biome registry with
precedence-as-ID ordering, stateless drift, an exactly-conserving moisture
field that maintains its gradient, biome-dependent capacity with a ledgered
loss sink, degenerate-map rejection, world integration, and save/restore.
Both fixtures are preserved and the config hash of a climate-disabled world
is provably unchanged. **Not started**: origin modes, founder demes,
archetypes, biome-matched placement, and founder-population files - criteria
C6.2, C6.3, C6.4, C6.5, C6.9, C6.10, and the Phase 6 benchmark record. See
`planning/phase-6-biomes-climate-and-origins.md` for the status detail and
the two modelling defects found while building it (D-048).

### The Phase 5 Experiment Instrument

The Phase 5 experiment instrument is implemented: the ALEV append-only event
log with a self-checking header and fail-closed decode, snapshots carrying a
real `event_log_offset` (closing D-019), the `sim-experiment` crate
(conditions, campaigns, the independent-world scheduler, manifests,
comparison reports that refuse rather than aggregate), asynchronous
checkpointing with a capacity-one refuse-and-count queue, and server
`--pacing headless` / `--run-ticks`. All seven acceptance criteria are met
with automated evidence; benchmark record `phase5-local-20260804T210059Z`.
Both fixtures reproduce from clean processes under every new execution path.
Workspace suite: 177 passed, 0 failed, 9 ignored (the release long-runs and
the Phase 5 benchmarks).

Deferred within Phase 5: Parquet export (carried from Phase 4) and any live
monitoring integration. Deployment, TLS, physical-device, and VM gates
remain unresolved; all ADRs remain proposed.

### Prior: The Phase 4 Local Persistence Slice

The Phase 4 local persistence slice is implemented: `sim_core::SaveState`
capture/restore, `crates/sim-persist` (ALIF format 1 with zstd, atomic
writes, SQLite catalog, recovery scan, isolated restore verifier,
fail-closed migration registry), CLI save/load/verify/CSV/compare
commands, and server checkpoints with audited save endpoints and
`--load-save` branching. All prior fixtures are preserved; a restored
world continues bit-identically. Benchmark IDs for all phases are curated
in `research/performance-notes.md` (`phase4-local-20260804T141013Z` is
current, including the zstd-versus-uncompressed comparison the ADR-0007
gate required). Deferred within Phase 4: the append-only event-log file,
Parquet export, and any live monitoring integration (proposal only).
Deployment, TLS, physical-device, and VM gates remain unresolved; all
ADRs remain proposed.

## Ordered Next Work

Phases 5, 6, and 7 are complete.

1. Review the Phase 5, 6, and 7 implementations and their benchmark
   evidence (D-045 through D-052), in particular the two Phase 6 modelling
   defects recorded in D-048 and Phase 7's four-condition decomposition in
   D-052.
2. Review the planning changes that landed alongside them, all Proposed and
   none settled: the flagship and campaign run-mode split (D-053), the
   time-scale position (D-054), the soak tiers (D-055), 3D voxel rendering
   (D-056), the Phase 13 split moving demography before the culture stack
   (D-057), and intra-world parallelism (D-058).
3. ~~Build the world-level spatial-aggregation index C7.1 needs, then
   measure C7.1.~~ **Done** (D-060 to D-062).
4. **Phase 8, demography and life history.** Now the next behavioral
   phase, ahead of the genome successor, because the culture stack cannot be
   measured in a world where 99.9 percent of deaths are starvation
   (ADR-0025, D-057). Includes raising `max_entities` above the ecological
   equilibrium so ecology binds rather than a memory guard.
5. Resolve the remaining Phase 0 decision gates (deployment-shaped VM
   benchmark). This bounds the compute-cost risk recorded as unresolved in
   `docs/20-risk-register.md`. Phase 5 measured the development host only;
   no supported campaign size is claimed.
6. If separately approved, run a read-only servernode3/monitoring/backup
   audit and record live facts.
7. Test physical target desktop, mobile, and kiosk browsers against the live
   server; do not treat viewport emulation as device evidence. Note that
   ADR-0024's voxel path reopens this gate with a different rendering
   technique, so sprite-path device evidence will not transfer.
8. Phase 18, intra-world parallelism, when the single-world population
   ceiling starts to bind. Scheduled after 8 and before Phase 13.

## Deferred Backlog

Carried forward unchanged unless noted:

- Visual palette/sprite identity system.
- Secure observer/admin authentication mechanism.
- Exact disaster catalog.
- Genome topology/trait range exploration. Partly absorbed by Phase 9.
- GPU experiment design.
- ~~Independent-world scheduler.~~ **Delivered in Phase 5**
  (`crates/sim-experiment/src/scheduler.rs`).
- Parquet export (deferred Phase 4 item, still deferred).
- Profiling and SIMD work from the superseded Phase 5 plan. Each still
  requires its own profiler evidence. **Intra-world parallelism is no
  longer a backlog item**: it has ADR-0026 and Phase 18.
- **Raise `max_entities` above the ecological equilibrium** so ecology binds
  instead of a memory guard. Population sat on the guard for the entire
  Phase 2 long run, which means the guard was the carrying capacity and
  every dynamic measured under it is an artifact. Config change, gated on
  the Phase 8 benchmark.
- Re-measure the `apply` serial fraction after Phases 7, 12, and 13. Each
  adds conflict resolution to exactly the part that caps parallel speedup
  (ADR-0026).
- Implement the voxel observer. Reuses protocol handling, selection,
  overlay, charts, and reconnect; replaces the render layer. Gated on
  ADR-0024's benchmark evidence.
- Close the **composite geometry gap** in
  `specifications/artifact-and-material-ontology.md`: composites have no
  spatial arrangement, so a depth-2 composite has no shape to render.
  Required before composites render as structures.
- Build the **check-in report** for flagship worlds: what changed since the
  last observation, derived from the event log.
- Define the **flagship retention and backup policy**, distinct from
  campaign pruning. A flagship checkpoint chain is irreplaceable once the
  world has received any intervention.
- Raise or confirm the observer speed cap (clamped to 64) for flagship
  catch-up. Headless is uncapped, so this may not be needed.
- Controller batching strategy for variable topologies (Phase 9 makes
  grouping by topology ID obsolete; the replacement is a later performance
  slice).
- A phenotype-only kin-inference condition, as an alternative to providing
  computed genetic distance as a kinship input (Phase 7 follow-up).
- **Exercise the `max_carcasses` cap.** The Phase 7 benchmark never filled
  it - the table peaked at 51 against a cap of 64 - so the eviction path and
  its decay-ledgered loss are unmeasured. Needs a world that generates
  carcasses faster than the standard tier does.
- **A cleaner density control than condition E.** E thins the population by
  cutting carrying capacity, so it changes food availability as well as
  density and is reported as a supporting comparison rather than a control.
  Phase 8's non-food extrinsic mortality is the natural instrument for a
  real one, and re-running the C7.1 contrast against it would sharpen the
  aggregation half of the result.

## Repository Hygiene

- **`crates/sim-core/src/config 2.rs` deleted, 2026-08-04.** It was a
  filesystem sync artifact: a 403-line pre-Phase-2 snapshot of `config.rs`,
  verified before removal to be a strict subset (0 unique lines; 190 lines
  absent, including `PHASE2_BEHAVIOR_POLICY_VERSION`, the whole
  `Phase2Config` struct, the `phase2` field, and the conditional Phase 2
  contribution to the config hash). It was never compiled: `lib.rs` declares
  only `mod config;`, and a filename containing a space is not a valid Rust
  module path. Nothing referenced it anywhere under `crates/`, `apps/`,
  `spikes/`, or `scripts/`.

  Re-verified after removal: `cargo build --workspace` clean; full workspace
  suite 90 passed / 0 failed / 3 ignored (the release long-runs);
  `verify-phase1-determinism.sh` PASS reproducing config
  `0x918a381c77559236` and state `0x1e3158a26afd3b39`;
  `verify-phase2-determinism.sh` PASS reproducing config
  `0xf83d3981bf7dd189` and state `0xff9dfcff5dffbf42`; `cargo fmt --check`
  and `cargo clippy --workspace --all-targets` clean.

  No behavior, schema, protocol, or replay semantics changed, so no ADR and
  no new replay lineage. Dead-file removal only.

- `.DS_Store` files are present in the working tree at several levels and
  are excluded from `FILE_MANIFEST.md`.
- **Non-ASCII punctuation** (em-dashes, curly quotes) remains in eight
  project documents: `docs/04`, `docs/12`, `docs/15`,
  `planning/phase-6-biomes-climate-and-origins.md`,
  `planning/phase-7-territory-and-conflict.md`,
  `specifications/event-schema.md`,
  `specifications/experiment-config-schema.md`,
  `specifications/metrics-schema.md`, and `research/performance-notes.md`.
  `AGENTS.md` requires ASCII. Purely typographic and meaning-preserving to
  fix; deliberately not swept during the merge to avoid churning another
  session's diffs for cosmetics. The vendored research reviews under
  `.agents/skills/*/references/` are **exempt**: they are verbatim source
  documents and must not be reformatted.

## Completed Phase 5 Slice (2026-08-04)

- `crates/sim-persist/src/eventlog.rs`: ALEV format 1 append-only log  - 
  self-checksumming header (provenance is not framing, so nothing else
  would catch a corrupted seed or config hash), per-segment CRC, all
  declared lengths capped before allocation, unknown event types fail
  closed, strictly ascending ticks, exact counter reconstruction, and a
  separate prefix reader that reports a torn tail without repairing it.
- `crates/sim-persist/src/checkpoint.rs`: asynchronous checkpoint writer.
  Capture on the tick thread, write on another; capacity-one queue that
  refuses and counts rather than queueing without bound; the Phase 4
  durability ordering untouched.
- `crates/sim-experiment`: config-field registry, conditions as named
  deltas with their own hashes, hand-parsed campaign files, the
  independent-world scheduler, preflight, manifests with verbatim embedded
  campaign source, and a comparison report with five typed refusals.
- CLI: `batch`, `report`, `fields`, `verify-events`, and `--event-log` on
  `run`. Server: `--pacing headless|realtime`, `--run-ticks`,
  `--checkpoint-mode sync|async`, plus two new checkpoint metrics.
- Suites: 65 new tests (177 passed / 0 failed / 9 ignored workspace-wide),
  including a 20,000-case event-log corruption sweep, a 10^6-tick
  completeness run, scheduler equality at six worker counts plus solo runs,
  scheduler failure isolation, an asynchronous crash simulation, and
  `scripts/verify-phase5-determinism.sh`.

## Completed Phase 4 Local Slice (2026-08-04)

- `sim_core::SaveState`: validated logical capture and fail-closed restore
  (terrain regenerated and checksum-verified; derived state recomputed;
  full invariant re-verification; bit-identical continued trajectories).
- `crates/sim-persist`: ALIF format 1 codec (sectioned, checksummed,
  bounded, zstd), atomic temp+fsync+rename writes with
  catalog-after-durability ordering, SQLite catalog, recovery scan,
  checkpoint pruning, isolated restore verifier, migration registry.
- CLI: `--save-path`, `--load-save`, `verify-save`, versioned CSV export,
  `compare`; server: `--data-dir`, checkpoint scheduler, audited
  save/list/verify endpoints, `--load-save` branching (epoch 2), save
  metrics.
- Suites: 11 kernel/persistence save tests plus server persistence
  integration; crash-simulation and restore-from-backup evidence.

## Completed Phase 3 Local Slice (2026-08-04)

- `crates/sim-protocol`: pure `ALSP` 1.0 codec, bounded fail-closed decode,
  golden/negative/endian/corruption tests.
- `crates/sim-server`: tick thread with real-time pacing and speed control,
  loopback REST (world info, bounded organism detail, analysis, metrics,
  audit, controls with roles/idempotency/rate limits), WebSocket sessions
  (Hello auth, clamped subscriptions, keyframe/delta/metrics streams, ack
  tracking, keyframe-collapse backpressure), 14 integration tests plus an
  ignored observer-fanout benchmark.
- `apps/observer`: TypeScript/PixiJS observer (terrain texture, pooled
  culled sprites, pan/zoom/pinch, selection inspector, scientific overlay,
  population chart with text alternative, admin controls, reconnect with
  keyframe resync, reduced-motion support), 7 Playwright E2E tests plus a
  gated render benchmark.
- Kernel additions: read-only `render_entities_in`, `biomass_cells`,
  `organism_detail` views (fixtures unchanged).

## Completed Phase 2 Local Slice (2026-08-04)

- Genome schema 1: 14 bounded trait genes plus 696 neural genes, bounded
  fail-closed codec, canonical hash, deterministic founder/recombination/
  variation through named streams, phenotype derivation.
- Controller topology 1 (20-16-12-12, 4 memory values) with rational tanh,
  clamps, non-finite neutralization, zero per-tick allocation.
- Config-gated `phase2-behavior-v1` tick: controller sensing/intents,
  heading/throttle movement with integer trigonometry, gated feeding,
  greedy stable-ID pairing with typed rejections, ancestry state, event
  schema 2, similarity analysis, CLI (`--phase2`, `analyze`), scripts, and
  benchmark instrumentation (controllers phase, similarity runtime).
- Suites: 88 workspace tests (86 default, 2 ignored release long-runs)
  including a deterministic malformed-input harness; clean-process Phase 2
  fixture script.

## Completed Phase 1 Local Slice (2026-08-03)

- Pure `sim-core` crate: versioned fixed-point config with canonical hash,
  `lifesim-rng-v1` named streams, `lifesim-worldgen-v1` continent generation,
  logistic food regrowth, spatial buckets, policy-v1 movement/feeding/
  crowding/reproduction/death, exact energy/biomass ledger, bounded events,
  on-demand state checksums, and invariant checks.
- `lifesim` CLI: run/fixture/inspect/benchmark, pause verification,
  Prometheus text metrics, per-phase timing, allocation counting, provenance.
- Suites at Phase 1 completion (recounted during the 2026-08-03 Phase 1
  audit): 46 workspace tests total, of which 45 run by default and 1 is the
  ignored 864,000-tick 24-hour-equivalent release test. Breakdown: 6 Phase 0
  spike tests; 40 Phase 1 tests (22 sim-core unit, 5 determinism, 3 long-run
  including the ignored one, 3 property, 7 CLI). Plus the two-clean-process
  fixture script. An earlier count of "42" predated the property tests and
  excluded the ignored test.

## Completed Phase 0 Local Slice

- Stable-ID fixed-point Rust tick with named deterministic RNG streams.
- Versioned little-endian snapshot frame with size caps and CRC/state checks.
- PixiJS WebGL/WebGPU renderer with per-entity viewport culling.
- Reproducible local harnesses, desktop/mobile browser smoke tests, and raw records.

## Entry Template

Each implementation item needs: phase, problem, scope, non-goals, prerequisites, owner, acceptance criteria, test plan, benchmark impact, documentation updates, risk, and rollback.

From Phase 5 onward, a behavioral item additionally needs: the ablation or
control condition, the metric that must differ between conditions, the seed
count, and the threshold, all fixed before the campaign runs. "It looked
interesting" is never an acceptance criterion, and a threshold weakened
after seeing the data is a different experiment.
