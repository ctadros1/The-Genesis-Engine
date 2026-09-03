# ADR-0033: Offline Era Detection's Concrete Design (Phase 17)

Status: accepted 2026-09-02. The design authority for Phase 17 remains
`specifications/era-and-tradition-detection.md` (ADR-0016) and
`planning/phase-17-era-and-tradition-detection.md`; this record pins the
concrete choices the specification left open - the feature vector as the
event log can actually supply it, the exact segmenter, the threshold's
meaning, the validation fixtures and the report - so the implementation
cannot pick them silently. Where this record and the specification
disagree, the disagreement is a defect in this record.

## The hard rule, restated as structure

`lifesim-era-v1` lives in `sim-analysis`, reads a decoded event log and a
run's config, holds no world handle, draws from no `RngSystem` stream,
and is a pure function of its inputs: the same log and parameters give
byte-identical reports in two processes (C17.3). `sim-core` does not
depend on `sim-analysis`, and Phase 17 makes that a test that parses the
crate manifests rather than a convention (C17.2). The analysis version
is recorded in every report and is deliberately outside the config hash.

A segment is a segment. No report, doc, or test names one after a human
period.

## Feature extraction: what the log can supply, and what it cannot

The event log is partitioned into fixed windows of `window_ticks`
(parameter, recorded). Each window yields one feature vector of
**twenty-two integer features in milli-units**, all rates per 1,000
organism-ticks (a count divided by the window's integrated population,
so a crowded world and a sparse one are comparable) except the two level
features, which are fractions of `max_entities`:

| Group | Features (rate per 1,000 organism-ticks unless noted) |
|---|---|
| Demography | births (paired + asexual + materialized), deaths by starvation, by old age, by damage, by hazard (senescence + extrinsic); mean population (level, milli of `max_entities`); pairing rejections |
| Conflict | damage events, carcasses consumed |
| Objects | created, destroyed, picked up, placed (`ObjectReleased{placed}`), struck, terrain struck, combined, consumed, actions refused |
| Social | signals emitted; signal energy (milli per emission, level) |
| Development and transition | growth completions, mate choices, materializations |
| Genetics | structural mutation rejections |

Population per window is reconstructed from the log itself: the config's
`initial_organisms`, plus every birth and materialization, minus every
death, integrated tick by tick inside the window. A log that yields a
negative population at any tick is refused (a torn or foreign log, not a
world).

**Recorded deviations from the specification's feature table**, each
because the quantity is not in the log as built:

- Genetics: applied structural mutations, node and edge counts, and
  genetic diversity are snapshot quantities, not events; only rejections
  are evented (C9.6). v1 carries the rejection rate and says so.
- Plasticity: the fraction of plastic edges and the learned-delta
  magnitude are snapshot quantities; `PlasticityFault` is a bug signal
  (D-085's family) and is deliberately **not** a feature - a detector
  that segmented on a fault rate would be segmenting on defects.
- Social: mean perceived neighbours is not evented (reception is
  unbounded per tick by design, ADR-0029).
- Conflict: within- versus between-cluster damage needs the communities
  analysis's proximity-matched labels; v1 carries the raw damage rate.

A feature the log cannot supply is reported as absent, never as zero.

## Segmentation: exact optimal partitioning, integer arithmetic

Over the window sequence `x_1..x_T` (after a `burn_in_ticks` prefix is
dropped and recorded), the segmenter minimizes

    sum over segments s of SSD(s) + penalty * (segments - 1)

where `SSD(s)` is the within-segment sum of squared deviations from the
segment mean, summed over features, computed exactly in `i128` as
`sum(x^2) - floor(sum(x)^2 / n)` per feature (the floor is the only
rounding and it is stated), and `penalty` is the parameter that says
what a boundary must buy to exist. Dynamic programming over the prefix
(`best[j]` = the cheapest partition of the first `j` windows), `O(T^2)`,
bounded by `max_segments` (parameter): a partition with more segments
than the bound is never considered. Ties are broken toward fewer
segments, then toward the earlier boundary - stated, so two builds agree.

**The threshold's meaning.** "A segment above threshold" is a boundary
the penalty admits: the SSD reduction it buys exceeds `penalty`. A world
whose best partition is one segment reports no boundaries, which is
C17.7's explicit negative result, printed. There is no second
significance test on top; the penalty is the threshold, and it is
calibrated once, on the null control and the synthetic fixtures, and
then locked in the pre-registration before the confirmatory reads it.

## Validation, the part that makes it falsifiable

- **Synthetic ground truth (C17.4).** A versioned generator in the test
  suite (`SYNTHETIC_FIXTURE_VERSION` 1) builds event logs with
  piecewise-constant rates and known boundaries at known windows, from a
  keyed hash (no RNG stream; deterministic by construction). Precision
  and recall at `+/- 1` window are reported at the locked penalty; the
  fixture set covers a single boundary, three boundaries, a boundary in
  one feature group only, and no boundary at all (the synthetic null).
- **Null control (C17.5).** A campaign of 30 worlds with artifacts,
  signalling and contest all disabled (a schema-2 world with the field
  and transition off), events on, 60,000 ticks, analyzed after the
  burn-in: the detector reports no boundary in at least 25 of 30. The
  burn-in exists because a founder population's first thousands of
  ticks are a real regime (settlement from the founders' endowment), and
  a detector that found it would be right, not noisy; the pre-registered
  burn-in is what makes "nothing can happen" true of the analyzed range.
- **Determinism (C17.3).** The CLI command's report for one world is
  produced twice in two processes and compared byte for byte.
- **Neutrality (C17.1).** A world is stepped, analyzed from its own
  collected events at several cadences, and stepped on; its checksum
  trajectory equals an identical world never analyzed. Trivially true of
  a pure function over a copy - which is exactly what the test records.

## Report format (`era-report 1`)

Header: `era-report 1 campaign <id> detector lifesim-era-v1 window
<ticks> penalty <milli> max_segments <k> burn_in <ticks>`. Per world:
`world condition=<c> seed=<hex> config=<hex> schema=<n> windows=<T>
segments=<k> cost=<i128>` then either `no segments above threshold` or
one `boundary tick=<t> window=<i>` line per boundary followed by
`delta <feature>=<milli>` lines for the five largest absolute feature
deltas across it (every feature is available in the machine-readable
tail: `features window=<i> f0=.. f21=..` when `--features` is passed).
Absent features print `absent`. Every parameter is echoed verbatim.

## Traditions (`lifesim-tradition-v1`) in this phase

The tradition detector Phase 13 built already carries the mandatory
genotype-matched control in every finding (C17.6) and prints its
rejection counters (C17.7). Phase 13's campaign convicted it on its own
control arm (D-126: findings uniform across all eight arms, including
no-channel arms), so Phase 17 does **not** re-run its verdict; it
records the conviction as the reading of C17.6 - the format enforces the
control, and the control found the detector's findings uninformative -
and names the sharper control as follow-on work (a within-endpoint
label-permutation null on the concentration statistic, D-100's shape).
Adding that null is a `lifesim-tradition-v2` with its own pre-registration,
not a change slipped into this phase's report.

## Cost (C17.8)

`lifesim era` over the largest event log in the archive (a Phase 13
confirmatory world) is timed and its peak resident memory recorded,
separately from tick cost, in
`experiments/results/phase17-benchmark-measurements.txt`. The decoder
loads a whole log today (`decode_log_events`); if the largest log does
not fit the budget, windowing over the decoded event vector is still
bounded by the vector, and that bound is what the record states.

## Consequences

- No world, save, protocol or config hash changes. Rollback is deleting
  a module and a command.
- The feature vector is narrower than the specification's table and the
  report says which groups are absent; widening it needs snapshot-side
  features and is a `lifesim-era-v2`.
- Every number the detector prints is an integer with a stated unit.

## As built

(Amended at the end of the phase with every divergence.)
