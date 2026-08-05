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

Phase 5 is complete (2026-08-04). Phase 6 is partially implemented: its
climate and biome half is built and verified, its origin-modes half has not
been started. Phases 7 to 16 are planned and none has started:

| Phase | Subject | Plan |
|---|---|---|
| 5 | Headless scale and multi-world experiments — **done** | `phase-5-headless-scale-and-experiments.md` |
| 6 | Biomes, climate drift, and world origin modes — **climate half done, origins not started** | `phase-6-biomes-climate-and-origins.md` |
| 7 | Territory, contest, and damage — **physics done, primary endpoint C7.1 unmeasured** | `phase-7-territory-and-conflict.md` |
| 8 | Evolvable genome: diploid genetics and variable topology | `phase-8-evolvable-genome.md` |
| 9 | Modular morphology and development | `phase-9-modular-morphology.md` |
| 10 | Lifetime learning | `phase-10-lifetime-learning.md` |
| 11 | Mutable world and artifacts | `phase-11-mutable-world-and-artifacts.md` |
| 12 | Social channel | `phase-12-social-channel.md` |
| 13 | Physiology, life history, and senescence | `phase-13-physiology-and-life-history.md` |
| 14 | Abiogenesis and the unicellular regime | `phase-14-abiogenesis-and-unicellular-regime.md` |
| 15 | The multicellularity transition | `phase-15-multicellularity-transition.md` |
| 16 | Offline era and tradition detection | `phase-16-era-and-tradition-detection.md` |

The former Phase 5 (performance optimization) and Phase 7 (advanced
ecosystems) plans are superseded and preserved unmodified under
`planning/superseded/`. Performance work is now a standing discipline in
every phase's Benchmark Impact section.

## Current Status

**Phase 7's physics is complete and verified; the phase is not.** Health,
damage, healing, death by depletion, the attack action and the three
reserved sensing channels, carcasses with an exact ledger, the canonical
pair key, RNG stream `Contest` (7), and event schema 3 are all in and
tested. C7.4 (exact accounting) and C7.5 (determinism) are met. **C7.1, the
designated primary endpoint, was not measured** — it needs a world-level
spatial-aggregation index that does not exist — and secondary criteria do
not rescue a failed primary. The behavioral campaign ran anyway and its
four-condition decomposition is recorded in
`planning/phase-7-territory-and-conflict.md` (D-052): perception alone costs
nothing, the attack action alone costs ~37 percent of the population at zero
damage, and damage costs a further ~31 percent. C7.2's tolerance clause and
C7.3 are recorded as **unmet rather than adjusted**.

**Phase 6 is complete** (2026-08-04), benchmark
`phase6-local-20260804T224418Z`.

### Prior: Phase 6 detail The
climate and biome half is built and verified: the seven-biome registry with
precedence-as-ID ordering, stateless drift, an exactly-conserving moisture
field that maintains its gradient, biome-dependent capacity with a ledgered
loss sink, degenerate-map rejection, world integration, and save/restore.
Both fixtures are preserved and the config hash of a climate-disabled world
is provably unchanged. **Not started**: origin modes, founder demes,
archetypes, biome-matched placement, and founder-population files — criteria
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

1. Review this planning change: the epistemic position, the phase order and
   its dependency argument, the determinism successors, and the six new
   proposed ADRs. Nothing below should start before that review.
2. Review the Phase 4 implementation, tests, and benchmark evidence.
3. If separately approved, run a read-only servernode3/monitoring/backup audit and record live facts.
4. Test physical target desktop, mobile, and kiosk browsers against the
   live server; do not treat viewport emulation as device evidence.
5. Resolve the remaining Phase 0 decision gates (deployment-shaped VM benchmark).
   This now also bounds the compute-cost risk, which is recorded as
   unresolved in `docs/20-risk-register.md`. Phase 5 measured the
   development host only; no supported campaign size is claimed.
6. Review the Phase 5 implementation, tests, and benchmark evidence
   (D-045, D-046).
7. Review the Phase 6 climate slice (D-047, D-048), in particular the two
   modelling defects and the biome-reachability measurement.
8. Finish Phase 6: `origin.mode` with `random` and `seeded`, founder demes,
   archetypes, biome-matched placement, and founder-population files. That
   is criteria C6.2, C6.3, C6.4, C6.5, C6.9, and C6.10, plus the Phase 6
   benchmark record (environment-phase cost per field, reclassification
   cost, and whether a dirty-cell strategy is needed). The temperature field
   `docs/05` has specified since Phase 1 now exists, so the
   thermal-preference gene inherited-but-inert since Phase 2 finally has
   something to be preferent about; wiring it is Phase 13's work.
9. Begin Phase 7 (requires separate approval): territory, contest, and
   damage. Its prerequisite is Phase 5, which is met.

## Deferred Backlog

Carried forward unchanged unless noted:

- Visual palette/sprite identity system.
- Secure observer/admin authentication mechanism.
- Exact disaster catalog.
- Genome topology/trait range exploration. Partly absorbed by Phase 8.
- GPU experiment design.
- ~~Independent-world scheduler.~~ **Delivered in Phase 5**
  (`crates/sim-experiment/src/scheduler.rs`).
- Parquet export (deferred Phase 4 item, still deferred).
- Profiling, SIMD, and parallelism work from the superseded Phase 5 plan.
  Each still requires its own profiler evidence and, for parallelism, its
  own deterministic-ordering evidence under ADR-0010.
- Controller batching strategy for variable topologies (Phase 8 makes
  grouping by topology ID obsolete; the replacement is a later performance
  slice).
- A phenotype-only kin-inference condition, as an alternative to providing
  computed genetic distance as a kinship input (Phase 7 follow-up).

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

## Completed Phase 5 Slice (2026-08-04)

- `crates/sim-persist/src/eventlog.rs`: ALEV format 1 append-only log —
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
