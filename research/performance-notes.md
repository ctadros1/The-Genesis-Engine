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

## Phase 5 Local Record

Benchmark ID: `phase5-local-20260804T210059Z` (2026-08-04 UTC), benchmark
schema 3, baseline `phase4-local-20260804T141013Z`. Provenance: revision
`67b6b074`; macOS 26.5 arm64, Apple M3 Pro, 12 logical cores; Rust 1.97.1
release with thin LTO. Worlds are the Phase 2 defaults after 2,000 warm-up
ticks. Schema 3 records are per-phase comparable only within schema 3;
earlier records remain valid and unmodified.

### Headless throughput per world

| Tier | Population | ticks/s | Tick p50 (ms) | Tick p95 (ms) | Tick p99 (ms) |
|---|---:|---:|---:|---:|---:|
| 500 | 135 | 8,805 | 0.103 | 0.195 | 0.301 |
| 2,000 | 1,051 | 1,653 | 0.520 | 1.194 | 1.623 |

### Scheduler scaling and host contention

16 independent worlds, 3,000 ticks each, 128x128 map, 200 founders.

| Workers | Wall (s) | Aggregate ticks/s | Speedup | Per-world degradation |
|---:|---:|---:|---:|---:|
| 1 | 3.511 | 13,669 | 1.00x | 0% |
| 2 | 1.939 | 24,750 | 1.81x | 9.5% |
| 4 | 0.958 | 50,106 | 3.67x | 8.4% |
| 8 | 0.708 | 67,790 | 4.96x | 38.0% |

### Event-log write cost and growth

Five alternating repetitions per tier; the median of each side is reported
with the observed spread, because a single comparison on this host produced
a *negative* overhead.

| Tier | Median without (s) | Median with (s) | Median overhead | Spread (without/with) | Bytes / 10^6 ticks |
|---|---:|---:|---:|---:|---:|
| 500 | 0.5533 | 0.5237 | -5.3% | 18.9% / 50.1% | 0.99 MB |
| 2,000 | 2.2517 | 2.2736 | +1.0% | 3.0% / 5.1% | 20.7 MB |

### Checkpoint stall on the tick thread (A5.5)

Checkpoint every 200 ticks over 2,000 ticks; tick budget is the configured
tick interval, 100 ms at `dt_ms = 100`.

| Tier | Mode | Tick p95 (ms) | Checkpoint-tick p50 (ms) | Checkpoint-tick max (ms) |
|---|---|---:|---:|---:|
| 500 | synchronous | 0.282 | 26.22 | 29.04 |
| 500 | asynchronous | 0.154 | 1.44 | 1.63 |
| 2,000 | synchronous | 1.405 | 67.99 | 75.59 |
| 2,000 | asynchronous | 1.139 | 4.69 | 6.68 |

Interpretation. **The stall on the tick thread falls by 14 to 18 times**:
from 26 ms to 1.4 ms at the 500 tier and from 68 ms to 4.7 ms at the 2,000
tier. The Phase 4 note recorded that the synchronous stall stayed under one
tick interval at both tiers, which was true, but the 2,000-tier figure of
75.6 ms was 76 percent of the budget with no headroom for a slower disk, a
larger population, or the extra state later phases add. Asynchronous
checkpointing leaves roughly fifteen times the margin. The write itself did
not get cheaper — it peaked at 86.3 ms at the 2,000 tier — it simply
stopped happening on the tick thread.

Two caveats stated rather than buried. First, **p95 over all ticks cannot
see this effect at all**: with a checkpoint every 200 ticks only 0.5 percent
of ticks carry one, so the stall lives above p99.5 and a p95-only report
would show nothing. The checkpoint ticks are therefore measured separately
and that is the column that matters. Second, the asynchronous writer
competes with the tick thread for memory bandwidth, so the general tick
distribution is not uniformly better; at the 2,000 tier p95 improved, but
the effect is small relative to run-to-run variance and is not claimed as a
result.

At the 500 tier one checkpoint in ten was **refused** because the previous
write had not finished (200 ticks at 0.1 ms is 20 ms, shorter than a 24 ms
write). Refusals are counted and exported as
`lifesim_checkpoints_skipped_total`; a nonzero value means the configured
interval is shorter than a checkpoint takes, which is a configuration fact
worth surfacing rather than hiding behind an unbounded queue.

The event-log write cost is **not resolvable above run-to-run variance** at
either tier: the 2,000-tier median overhead of 1.0 percent sits inside a
3-5 percent spread, and the 500-tier median is negative. The honest
statement is that the cost is below this host's noise floor, not that it is
1 percent. Growth is the firmer number: 20.7 MB per 10^6 ticks at the 2,000
tier, 0.99 MB at the 500 tier, at 48-58 bytes per event.

Scheduler scaling is close to linear to 4 workers (3.67x with 8.4 percent
per-world degradation) and falls off at 8 (4.96x, 38 percent), which is the
expected shape for 12 logical cores where 4 are performance cores. **No
supported campaign size is claimed from this**; it is one host, and the
deployment-VM measurement remains an open Phase 0 gate.

Limitations: local development host; a single filesystem; populations
reflect the live trajectory at tick 2,000; the scheduler measurement uses
small worlds so that 16 of them fit in a short run. Raw records under
`benchmarks/raw/phase5-local-20260804T210059Z/` (intentionally ignored).

## Phase 6 Local Record

Benchmark ID: `phase6-local-20260804T224418Z`, benchmark schema 4, baseline
`phase1-local-20260804T034607Z`. macOS 26.5 arm64, Apple M3 Pro, Rust 1.97.1
release with thin LTO. 256x256 (65,536 cells), 500 founders, 3,000 measured
ticks after 500 warm-up.

### Environment phase, by field

| Configuration | Environment p50 (us) | Environment p95 (us) | Ticks/s |
|---|---:|---:|---:|
| Climate off (Phase 1/2 regrowth only) | 50.8 | 155.1 | 7,296 |
| Moisture exchange only | 481.0 | 584.8 | 1,797 |
| Moisture + reclassify every 100 ticks | 473.0 | 575.9 | 1,793 |
| Moisture + reclassify every tick | 1,121.3 | 1,203.6 | 832 |

### Founder generation, one-time at tick 0

| Origin | `World::new` p50 (ms) | p95 (ms) |
|---|---:|---:|
| `random`, one deme | 5.98 | 6.09 |
| `random`, four demes | 6.22 | 6.35 |
| `seeded`, two archetypes | 7.26 | 7.64 |

Interpretation, and it answers the question the plan asked. **The
reclassification cadence does not need a dirty-cell strategy; the moisture
exchange does.** Reclassifying every 100 ticks is free — 473 versus 481
microseconds is inside the run-to-run spread, so the amortization works as
designed. Reclassifying every tick more than doubles the phase, which is
exactly the cost the cadence exists to avoid and confirms the cadence is
load-bearing rather than decorative.

The per-tick moisture exchange is the real cost: it adds roughly 430
microseconds to a phase that was 51, and drops whole-tick throughput from
7,296 to 1,793 ticks per second, a **4x slowdown**. That is a large price
for a field that changes slowly, and it is the obvious first optimization
target — the exchange visits every neighbouring pair every tick to move
quantities that are near their fixed point most of the time. A cadence for
moisture, or a dirty-region scheme, is now a measured backlog item rather
than a guess.

Founder generation is one-time and small: four demes cost 4 percent more
than one, and `seeded` costs 21 percent more than `random`, against a
`World::new` that is dominated by terrain generation in every case.

One comparison to make carefully rather than loosely: the Phase 1 record
described the environment phase as "about 200 microseconds for the 65,536-cell
logistic regrowth scan", and the climate-off row here reads 50.8 microseconds
p50 for what should be the same work. The two are not directly comparable —
different host state, different measurement harness, and a p50 against a
figure that was not stated as a percentile — so this is recorded as a note,
not as a claimed improvement. Benchmark schema 4 records are per-phase
comparable only within schema 4.

Limitations: local development host; one map size; the environment-phase
timer is a `TickObserver` hook, so it includes observer dispatch. Raw records
under `benchmarks/raw/phase6-local-20260804T224418Z/` (intentionally ignored).

## Phase 6 Partial Measurement: Biome Reachability By Map Size

Not a benchmark record; a generation-validity measurement taken while
calibrating the biome thresholds, recorded because it constrains how Phase 6
campaigns must be configured.

C6.7 rejects a world in which any biome is empty. How often that fires
depends almost entirely on map size, because `Arid` requires an interior far
enough from water to be dry and a small continent does not have one.

| Map | Worlds generated (of 30 seeds) | Rejected for land fraction | Rejected for an empty biome |
|---|---:|---:|---:|
| 96x96 | 8 | 1 | 21 |
| 160x160 | 14 | 0 | 16 |
| 256x256 | 22 | 0 | 8 |

`Arid` is the only missing biome at 160x160 and above. This is a physical
result, not a threshold artifact: the thresholds were calibrated against
measured field distributions across seven seeds (land elevation spans
roughly 17,000 to 35,500-53,300 Q16; inland moisture spans 18,500-37,000 to
111,500-115,600 milli-units), and every threshold sits inside the range every
seed reaches.

The operational consequence is that a Phase 6 campaign must either use the
256x256 default map or expect roughly a quarter of its seeds to be refused.
Phase 5's preflight makes that visible before compute is spent rather than
after, which is exactly what it exists for.

## Phase 7 Local Record

Benchmark ID: `phase7-local-20260805T025643Z`, benchmark schema 4, baseline
`phase5-local-20260804T210059Z`. macOS 26.5 arm64, Apple M3 Pro, Rust 1.97.1
release with thin LTO. 256x256, 4,000 measured ticks after 500 warm-up.

The plan's Benchmark Impact section predicted contest would land on exactly
two phases: `apply` gains intent resolution against neighbours and `sense`
gains threat estimation. It does, and nothing else moves.

**Contest changes the population, so raw microseconds per phase are not a
like-for-like comparison** - contest-enabled at the 2,000 tier shows a
*faster* whole tick (221.9 us against 301.7) purely because it is running
531 organisms instead of 781. Everything below is therefore normalized per
1,000 organisms, and the zero-damage condition is included because it does
contest's work at close to the baseline's population.

### Per-phase cost, microseconds per 1,000 organisms, 500 tier

| Phase | Contest off | Contest on | Zero damage | Change |
|---|---:|---:|---:|---|
| spatial_index | 11.67 | 12.58 | 12.16 | flat |
| **sense** | **44.29** | **60.28** | **62.75** | **+36% to +42%** |
| controllers | 209.05 | 206.76 | 216.27 | flat |
| **apply** | **21.43** | **27.78** | **28.77** | **+30% to +34%** |
| lifecycle | 1.67 | 1.84 | 1.74 | flat |

`environment` is a per-cell scan, not per-organism, so its per-1,000 figure
moves with population and carries no information here; its absolute p50 is
49.0, 49.2, and 49.5 us across the three conditions, which is flat.

### Carcass entity-count effect, measured separately

| Decay setting | Carcasses held | Whole tick p50 (us) |
|---|---:|---:|
| Fast (0.5/s) | 1 | 96.04 |
| Default (0.05/s) | 1 | 98.62 |
| None | 51 | 100.50 |

Roughly 1.9 us for 50 extra carcasses, about 0.04 us each, against a 98 us
tick. Carcasses are not a cost worth managing at this scale. **The
`max_carcasses` cap is untested by this benchmark**: the table peaked at 51
entries, so the small-cap condition (64) never bound and measured the same
thing as the no-decay condition. Exercising the cap needs a world that
generates carcasses faster than this one does.

Raw records under `benchmarks/raw/phase7-local-20260805T025643Z/`
(intentionally ignored).

## Phase 7 Campaign Cost

Not a per-phase benchmark; the wall-clock cost of the C7.1 confirmatory
campaign, recorded because it sets what a Phase 8 campaign can afford.
300 worlds (5 conditions x 60 seeds) at 20,000 ticks with spatial sampling
every 50 ticks: **123.5 s wall at 8 workers, 48,601 aggregate ticks/s**,
producing 198 MB of `.alss` sample files. The same campaign without spatial
sampling ran at 69,102 aggregate ticks/s, so sampling costs roughly 30
percent of campaign throughput at this cadence and population.

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
