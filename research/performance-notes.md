# Performance Notes

## Status

A local Phase 0 spike benchmark now exists. It is development-host evidence only;
the capacity values in project documents remain staged targets until a
deployment-shaped VM and target devices are measured.

## Phase 0 Local Record

Benchmark ID: `phase0-local-20260804T030100Z` (2026-08-03 EDT / 2026-08-04 UTC).

Provenance: unborn Git branch with dirty/untracked planning and spike files;
macOS 26.5 arm64; Apple M3 Pro, 12 logical CPUs, 18 GiB host memory; Rust 1.97.1
release profile with thin LTO; Node 25.6.0; PixiJS 8.19.0; headless Chrome
150.0.7871.187. Seed `0x5eedcafef00dbeef`; tick config hashes
`0x2e72cb9844dff3fb` at 500 organisms and `0xc94d302b40ae5d15` at 2,000 organisms.

Method: 100 Rust warm-up ticks, 50 samples of 10 ticks, 50 snapshot samples;
30 browser warm-up frames and 120 sampled frames. The browser world is two
viewports wide and high with deterministic square markers and per-entity PixiJS
culling. RSS is sampled at completion, not peak. Browser render time is CPU
submission time, not GPU completion.

| Scenario | Tick p50/p95/p99 | RSS at completion | Snapshot bytes | Encode p95 | Decode p95 |
|---|---:|---:|---:|---:|---:|
| Rust, 500 organisms | 46.479 / 55.091 / 55.983 us | 1,900,544 B | 8,072 | 61.000 us | 63.000 us |
| Rust, 2,000 organisms | 385.758 / 410.671 / 428.613 us | 2,031,616 B | 32,072 | 274.709 us | 266.417 us |

The 2,000-organism p95 tick phase split was: spatial index 2.975 us,
sense/controller 364.233 us, apply 0.517 us, and checksum 42.096 us. The
500-organism split was 0.762, 42.075, 0.125, and 9.913 us respectively. These
figures describe this synthetic fixed-point spike, not organism capacity for the
planned ecology.

| Renderer scenario | Actual backend | Frame interval p95 | Update/cull CPU p95 | Render-submit CPU p95 | Draw calls p95 | Visible p50 |
|---|---|---:|---:|---:|---:|---:|
| Desktop 500 | WebGL | 10.1 ms | 0.4 ms | 0.5 ms | 1 | 139 |
| Desktop 2,000 | WebGL | 10.1 ms | 1.1 ms | 1.3 ms | 1 | 529 |
| Mobile viewport 500 | WebGL | 9.8 ms | 0.4 ms | 0.5 ms | 1 | 145 |
| Mobile viewport 2,000 | WebGL | 10.0 ms | 1.2 ms | 1.2 ms | 1 | 555 |
| Desktop 2,000 | WebGPU | 9.9 ms | 1.1 ms | 1.1 ms | 1 | 529 |
| Mobile viewport 2,000 | WebGPU | 9.7 ms | 1.2 ms | 1.3 ms | 1 | 555 |

Two clean processes produced identical 500-organism/500-tick output:
state checksum `0xf263056bcafcfdc0`, snapshot CRC32 `0xfdf3f46f`, and 8,072
snapshot bytes. Six focused Rust tests, two WebGL browser smoke tests, TypeScript
type checking, Vite build, Rust formatting, and Clippy passed.

Raw samples, summaries, screenshots, logs, and the deterministic fixture are in
`benchmarks/raw/phase0-local-20260804T030100Z/`. That directory is intentionally
ignored so repeated machine-local measurements do not rewrite curated evidence.

Additional recorded limitation (2026-08-03 audit): the browser scenarios were
served by the Vite development server, not the built `dist/` bundle. The
sampled per-frame update/render CPU times exercise the same pre-bundled PixiJS
code either way, so the recorded frame costs are considered representative, but
any future load-time or bundle-size claim must use a production build.

Interpretation: the local results support retaining Rust, fixed deterministic
ordering, bounded snapshot framing, PixiJS, and CPU-first evaluation as proposed
baselines. They do not select WebGPU over WebGL, validate zstd, establish a
supported population, validate physical mobile/kiosk devices, or validate the
proposed VM. ADR-0001, ADR-0005, ADR-0007, and ADR-0010 therefore remain proposed.

## Phase 1 Local Record

Benchmark ID: `phase1-local-20260804T034607Z` (2026-08-03 EDT / 2026-08-04 UTC).

Provenance: unborn Git branch with a dirty worktree; macOS 26.5 arm64; Apple
M3 Pro, 18 GiB host memory; Rust 1.97.1 release profile with thin LTO; seed
`0x5eedcafef00dbeef`; behavior policy `phase1-behavior-v1`; default 256x256
cells at 4 m, `max_entities` 5,000. Config hashes per scenario: 500 organisms
`0x918a381c77559236`, 500 fixed-population `0xe904f202d29d2dbf`, 2,000
`0x704fb663db7e3e6c`, 2,000 fixed-population `0x72cf39b86380a505`.

Method: 200 warm-up ticks, then 50 samples of 10 ticks each per scenario.
"Ecology" scenarios leave reproduction on, so the live population grows
during sampling (500 -> 997 and 2,000 -> 3,870); "fixed-population"
scenarios disable reproduction so per-tick cost maps to a near-constant
population. The state checksum is computed on demand and excluded from tick
timing. Allocation counts come from a counting global allocator in the CLI;
RSS is sampled at completion, not peak.

| Scenario | Tick p50/p95/p99 (us) | Environment p95 | Spatial p95 | Sense p95 | Allocs/tick p50 | RSS |
|---|---:|---:|---:|---:|---:|---:|
| 500 ecology | 221.2 / 271.4 / 273.9 | 192.7 | 9.7 | 65.7 | 5.4 | 4,571,136 B |
| 500 fixed | 215.8 / 228.6 / 237.4 | 193.3 | 9.6 | 25.9 | 5.3 | 4,456,448 B |
| 2,000 ecology | 357.6 / 663.4 / 748.4 | 208.7 | 17.0 | 399.4 | 5.1 | 4,947,968 B |
| 2,000 fixed | 352.0 / 364.0 / 380.2 | 202.5 | 12.2 | 141.2 | 4.1 | 4,653,056 B |

Two clean `lifesim fixture` processes produced identical 500-organism/500-tick
output: state checksum `0x1e3158a26afd3b39`, terrain checksum
`0x60049f78e1881044`, config hash `0x918a381c77559236`. The
24-hour-equivalent long-run test (864,000 ticks, default 500-organism world)
passed in 315 s with invariants checked every 10,000 ticks: final population
1,062; 1,957,803 births; 1,957,241 starvation deaths; 0 old-age deaths;
2,246 capacity rejections; maximum population 5,000 (the ceiling, touched
during the initial boom); final state checksum `0x0590aa6e069f8f3d`.

Recorded correction (2026-08-03 Phase 1 audit): the two `-fixedpop` summary
JSON files in this record carry the generator's unconditional limitation line
"population varies across the run", which is wrong for those scenarios (no
births occur; population moved only 500 -> 499 and 2,000 -> 1,954 through
deaths). The generator now emits a scenario-appropriate limitation. The
historical raw files are preserved unmodified; their `population` fields
(`at_start`/`after_warmup`/`at_end`) are correct and authoritative.

Interpretation: the Phase 1 kernel meets the declared 500-2,000 prototype
tier on the local host with p95 tick under 0.4% of the 100 ms budget at a
fixed 2,000 population. The dominant fixed cost is the environment phase
(about 200 us: the full 65,536-cell logistic regrowth scan), which is the
first optimization hypothesis for Phase 5 (dirty-cell or region tracking);
the sense phase scales with population and local density. Limitations: local
development host only, not the deployment VM; ecology-scenario timings track
a changing population; RSS is completion-sampled; no cross-platform claim.
Raw samples are under `benchmarks/raw/phase1-local-20260804T034607Z/`
(intentionally ignored).

## Phase 2 Local Record

Benchmark ID: `phase2-local-20260804T044516Z` (2026-08-04 UTC).

Provenance: unborn Git branch, dirty worktree; macOS 26.5 arm64; Apple M3
Pro, 18 GiB host memory; Rust 1.97.1 release with thin LTO; seed
`0x5eedcafef00dbeef`; policies `phase2-behavior-v1`, `lifesim-genome-v1`
(schema 1, topology 1), `lifesim-controller-v1`, `lifesim-similarity-v1`;
default 256x256-cell world, `max_entities` 5,000; benchmark schema 2.
Config hashes: 500 `0xf83d3981bf7dd189`, 500-fixedpop `0xd7f03ad19e7ed6fa`,
2,000 `0x299b4468b6724647`, 2,000-fixedpop `0xe91e9c0216f0a768`. Method as
in Phase 1 (200 warm-up ticks, 50 samples of 10 ticks; checksum excluded
from tick timing; counting allocator; RSS at completion).

| Scenario | Tick p50/p95/p99 (us) | Sense p95 | Controllers p95 | Allocs/tick p50 | RSS |
|---|---:|---:|---:|---:|---:|
| 500 ecology | 322.2 / 341.9 / 346.5 | 28.6 | 117.9 | 3.5 | 5,718,016 B |
| 500 fixedpop | 321.7 / 346.3 / 352.2 | 26.9 | 119.9 | 2.4 | 5,668,864 B |
| 2,000 ecology | 812.8 / 928.7 / 986.2 | 180.3 | 501.1 | 3.8 | 11,304,960 B |
| 2,000 fixedpop | 815.3 / 877.2 / 1,425.8 | 174.3 | 473.3 | 2.8 | 11,255,808 B |

Offline similarity analysis (bounded 2,048 sample, trait distance only):
466-474 us at ~370 sampled and 7.45-7.47 ms at ~1,485 sampled, measured
separately from tick cost. Controller faults and invalid records admitted:
0 in every scenario.

Important qualification: even the "fixedpop" scenarios lose population to
starvation during the founder bottleneck (500 -> ~368, 2,000 -> ~1,485
across warm-up plus 500 sampled ticks), so per-tick cost reflects a
declining population; controller cost is roughly 0.32-0.33 us per organism
per tick. Phase 2 is comparable to Phase 1 only with the added work
qualified: Phase 2 spends its extra time in the controllers phase (about
35 percent of tick at the 500 tier, 55 percent at the 2,000 tier) plus
genome-driven sensing; the environment phase is unchanged.

Clean-process Phase 2 fixture (two separate processes, byte-identical):
500 organisms, 500 ticks, config hash `0xf83d3981bf7dd189`, terrain
checksum `0x60049f78e1881044`, state checksum `0xff9dfcff5dffbf42`.

Multi-generation long-run (release, 200,000 ticks, default 500-organism
Phase 2 world, invariants checked every 10,000 ticks, 405.7 s wall):
final population 4,706 (max 5,000, the configured ceiling); 204,257 paired
births; ancestry depth 127; 199,871 starvation and 180 old-age deaths;
pair rejections capacity/placement/energy 72/1/0; zero controller faults;
mean pairwise trait diversity 0.0863 (founders start near 0.167 by
construction, so selection narrowed the trait distribution); one cluster
at the default 0.2 threshold; final state checksum `0x20e7ca5c421537bf`.
These are observations of this configuration, not biological claims.
Malformed-input harness: 100,000 seeded corruptions, 0 panics, 99,993
typed rejections, 7 accepts (identity truncations), all accepts
re-validated and round-tripped.

Limitations: local development host only; no deployment-VM, cross-platform,
physical-device, or GPU claim; ecology and fixedpop populations both change
during sampling; RSS is completion-sampled, not peak. Raw samples are under
`benchmarks/raw/phase2-local-20260804T044516Z/` (intentionally ignored).
Phase 0 and Phase 1 records above are preserved unchanged.

## Phase 3 Local Record

Benchmark ID: `phase3-local-20260804T051859Z` (2026-08-04 UTC). Provenance:
unborn Git branch, dirty worktree; macOS 26.5 arm64, Apple M3 Pro; Rust
1.97.1 release with thin LTO; Node 25.6.0; protocol `ALSP` 1.0. Scenario:
500-organism Phase 2 world at 16x speed, full-world subscription, all
layers, 20 Hz cap, loopback only.

Tick percentiles with synthetic healthy observers (server-measured,
4,096-sample window over 15 s per scenario):

| Observers | Tick p50 / p95 / p99 (us) | Per-client bandwidth | Dropped |
|---:|---|---:|---:|
| 0 | 197.4 / 354.6 / 547.2 | - | - |
| 1 | 248.5 / 420.2 / 552.5 | 194.9 KB/s | 0 |
| 4 | 234.0 / 428.8 / 551.9 | 186.8 KB/s each | 0 |

Interpretation: streaming adds roughly 65-75 us to p95 tick (viewport
extraction and per-client diffing on the tick thread) and the increment is
nearly flat from one to four observers. Healthy clients drop nothing; the
slow-client path (collapse to keyframe resync, dropped-update counters) is
proven by the integration suite rather than this record. Browser render
sampling at the same scenario (Chrome, dense default viewport, 175 live
organisms, 196 deltas applied during the window): frame interval p50
8.3 ms, p95 9.0 ms, p99 9.3 ms.

Limitations: loopback only, not a deployment network; synthetic Rust
observers for the fan-out scenarios; a single live browser for render
sampling; population follows its live trajectory during measurement; no
WebGPU, physical-device, kiosk, or TLS measurements. Raw records are under
`benchmarks/raw/phase3-local-20260804T051859Z/` (intentionally ignored).

## Phase 4 Local Record

Benchmark ID: `phase4-local-20260804T141013Z` (2026-08-04 UTC). Provenance:
unborn Git branch, dirty worktree; macOS 26.5 arm64, Apple M3 Pro; Rust
1.97.1 release with thin LTO; snapshot format ALIF 1; 20 samples per
variant; worlds are the Phase 2 defaults after 2,000 warm-up ticks
(population 196 at the 500 tier, 1,024 at the 2,000 tier).

| Tier | Codec | Bytes | Encode p50 (ms) | Decode p50 (ms) | Rebuild p50 (ms) |
|---|---|---:|---:|---:|---:|
| 500 | uncompressed | 1,101,629 | 14.28 | 14.18 | 8.49 |
| 500 | zstd-1 | 570,979 | 11.92 | 11.35 | 8.46 |
| 500 | zstd-3 | 572,247 | 12.35 | 11.37 | 8.47 |
| 500 | zstd-9 | 560,405 | 16.02 | 11.39 | 8.49 |
| 2,000 | uncompressed | 3,537,605 | 45.98 | 45.16 | 33.01 |
| 2,000 | zstd-1 | 2,706,088 | 45.01 | 43.09 | 33.55 |
| 2,000 | zstd-3 | 2,489,792 | 45.88 | 41.98 | 33.55 |
| 2,000 | zstd-9 | 2,363,801 | 60.77 | 43.57 | 34.59 |

Interpretation (the bounded compressed-codec comparison ADR-0007 required):
zstd levels 1-3 reduce snapshot size 30-48 percent at equal or better
encode/decode time than the uncompressed codec, so compression costs
nothing here; level 9 buys little further size for materially slower
encodes. The server's synchronous checkpoint stall equals the encode plus
fsync time — under one 100 ms tick interval at both tiers, so continuous
operation with periodic checkpoints does not skip ticks at 1x speed.
Snapshot size is dominated by per-organism genome arrays (~2.8 KB of f32
parameters each). Restore (decode plus world rebuild with terrain
regeneration and full invariant checks) is ~20 ms / ~77 ms at the two
tiers. Limitations: local development host and filesystem effects excluded
from the tabled numbers (in-memory encode/decode; fsync cost is
workload-dependent); populations reflect the live trajectory at tick
2,000. Raw records under `benchmarks/raw/phase4-local-20260804T141013Z/`
(intentionally ignored).

## Required Record Format

| Field | Required Value |
|---|---|
| Benchmark ID/date | unique immutable identifier |
| Revision/build | git revision, compiler/runtime, profile |
| Hardware/VM | CPU/RAM/storage/guest allocation and relevant flags |
| Scenario | config hash, seed, map, organism count, observers |
| Method | warmup, duration, sampling cadence, deterministic mode |
| Result | p50/p95/p99 tick, RSS, allocation, save, stream data |
| Interpretation | bottleneck hypothesis and limitations |
| Decision | retain/revise/defer plus linked ADR |

## First Five Experiments

1. Single-threaded deterministic tick at 500 and 2,000 synthetic organisms.
2. Spatial bucket size/density sweep with identical world config.
3. Compact neural evaluation share of tick at Phase 2 topology.
4. PixiJS dense viewport FPS/draw calls with WebGPU and WebGL fallback.
5. Snapshot encode/decode/restore size and duration at supported tiers.

A later GPU test must compare full ticks, not a matrix multiplication microbenchmark.
