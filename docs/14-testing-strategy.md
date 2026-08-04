# Testing Strategy

## Testing Pyramid

The project tests the kernel first, persistence/protocol boundaries second, and the observer/deployment surface last. Tests are evidence for a declared behavior, not an excuse to freeze weak early rules.

| Layer | Purpose | Required Examples |
|---|---|---|
| Unit | Formula and state-transition correctness | energy, mutation bounds, death, encoding validation |
| Property | Broad invariant exploration | valid genomes stay valid; decoder never panics |
| Deterministic | Same seed/config/build behavior | tick checksum and event sequence fixtures |
| Integration | Cross-module contract | save/restore, API command, streaming resync |
| Fuzz | Hostile boundary data | genome, save, binary WebSocket frames |
| Long-run | Stability over many ticks | memory, entity count, non-finite state, population dynamics |
| Benchmark | Measured scale | tick phases, RSS, network fan-out, save cost |
| Browser/E2E | User-visible correctness | pan/zoom, selection, reconnect, mobile controls |
| Deployment | Operational proof | health, metric scrape, backup restore in isolated target |

## Required Invariants

- Energy changes only through documented sources/sinks.
- Organism counts equal the sum of live, terminally removed, and pending validated lifecycle events.
- Dead organisms cannot act after the removal phase.
- Invalid neural/genome/save/protocol input is rejected or neutralized without a process crash.
- A paused world advances zero ticks.
- A slow/disconnected observer cannot block the simulation.
- Save/restore preserves the documented state and replay provenance.
- Strict mode repeats a compatible run according to its declared checksum/tolerance policy.

## Experiment Discipline (Phases 5 To 16)

Behavioral phases produce claims about observed behavior, which need a
different kind of rigor than a unit test. These rules are as binding as the
testing pyramid above.

- **Every behavioral acceptance criterion states its control or ablation
  before the campaign runs.** A criterion of the form "the effect occurs" is
  incomplete; the form is "the effect occurs in N of M seeds under condition
  A and fewer under condition B, where B differs from A only in X".
- **"It looked interesting" is never an acceptance criterion.** Neither is a
  screenshot, a single remarkable run, or a chart a human found suggestive.
- **Thresholds are fixed before the data exists.** Weakening a criterion
  after seeing the result is a different experiment, and it must be labelled
  as one.
- **Nulls are results.** A phase whose behavioral criteria fail is complete
  if its ablations ran and its statistics are recorded. The ablation design
  is what makes a null answer *why* rather than merely *whether*.
- **Underpowered is not the same as negative.** If a condition could not be
  run long enough or across enough seeds for a null to be meaningful, it is
  reported as underpowered, and that distinction must survive into every
  downstream summary.
- **Confounds get their own condition.** The canonical case is Phase 11's
  condition D, where signals are delivered to a random unrelated receiver:
  it preserves the behavior, the energy cost, and the receipt, and destroys
  only the information. A difference against the do-nothing control without
  a difference against D is not transmission.
- **Cultural claims require a genetic control.** A behavior shared by close
  kin is a plausible inherited trait. Tradition findings carry a
  genotype-matched control statistic, its tolerance, and the cohort size, or
  they fail report validation.
- **Statistical criteria are automated tests**, with recorded tolerances,
  seeds, and sample sizes. A statistical criterion checked by a human
  reading a chart is not a test.

## Determinism Test Obligations For New Phases

Every phase that adds state or interaction adds all of these, per
`specifications/determinism-extensions.md`:

1. Disabled-section fixture equality against the previous phase's fixture.
2. Clean-process replay of the new phase's own fixture, two processes.
3. Storage-permutation equality over N ticks, which is what catches
   order-dependence in social learning and in per-node float summation.
4. Analysis-neutrality equality with analysis enabled and disabled.
5. Save, restore, continue: bit-identical trajectory with the new state
   present.
6. A seeded malformed-input harness over any new codec: zero panics, zero
   invalid admissions.

Phase 5 adds one more: scheduler concurrency equality, proving per-world
checksums are identical at concurrency 1, 2, and C and identical to
single-world execution.

## Test Data Discipline

Every fixture has a schema version, seed, and expected policy. Keep minimal valid examples, malformed examples, and prior-version saves. Do not replace a regression fixture merely because a model change made it inconvenient; version it and state why behavior changed.

## Execution Gates

Phase 1 requires unit, deterministic, and CLI integration tests. Phase 2 adds property/fuzz and long-run evolution tests. Phase 3 adds protocol/render/browser tests. Phase 4 adds restore and export tests.

Phase 5 adds scheduler-determinism and event-log tests and makes benchmark
regressions a release gate. Phases 7 through 13 each add the six determinism
obligations above plus their own behavioral campaign. Phase 16 adds
analysis-neutrality, synthetic-ground-truth, and null-control tests.

### Phase 4 Gate Status

Phase 4 adds: kernel save/restore tests (`sim-core/src/save.rs`:
round-trip checksum equality with bit-identical continued trajectories for
Phase 1 and Phase 2 worlds, terrain-checksum mismatch, tampered
energy/order/length/genome/section rejection); the `sim-persist` suite
(golden compressed and uncompressed snapshots, deterministic encoding,
truncation/magic/version/bit-flip/oversize rejection, a 2,000-case seeded
corruption sweep, migration-registry fail-closed behavior, atomic-write
crash simulation with last-valid-checkpoint survival, checkpoint pruning
that never touches named saves, forged-checksum detection, and the
restore-from-backup integration test that copies a full recovery set to an
isolated target, verifies provenance, continues the branched world
identically, and proves the source unmodified); and server tests for
audited manual saves, automatic checkpoints with retention, isolated
verify, and `--load-save` branching with a new world epoch. The
save/restore benchmark is an `#[ignore]` release test run by
`scripts/run-phase4-benchmarks.sh`.

### Phase 3 Gate Status

Phase 3 adds: protocol codec tests in `crates/sim-protocol` (round-trip
with and without checksum, golden big-endian bytes, malformed envelopes,
hostile counts rejected before allocation, unordered viewports, a
20,000-case seeded corruption sweep); server integration tests in
`crates/sim-server/tests/integration.rs` (WebSocket auth rejection,
welcome/subscribe/keyframe/delta/metrics flow with strictly increasing
sequences, subscription clamping, slow-client keyframe resync with
dropped-update counters, REST role/audit/idempotency/pause semantics,
bounded organism detail without controller matrices, malformed-frame
resilience); and browser E2E in `apps/observer/tests/e2e.spec.ts`
(connect/stream, pan/zoom, selection inspector plus overlay toggle, admin
pause/resume, role-gated controls, reconnect resync, mobile viewport).
The observer-fanout stream benchmark is an `#[ignore]` release test run by
`scripts/run-phase3-benchmarks.sh`, which also drives the gated browser
render sampling.

### Phase 2 Gate Status

Phase 2 adds: genome unit and regression tests (validation, codec,
hashing, founder determinism, recombination bounds and audit, phenotype
ranges); controller fixtures (all-zero, known directional, saturated,
extreme weights, non-finite neutralization, clamping, memory extraction,
BAM trigonometry); pairing/ancestry integration tests (mutual intent,
capacity, compatibility, cooldown spacing, no-pairing-after-death,
two-parent audit, event ordering); Phase 2 determinism tests (replay with
event equality, seed/config divergence, disabled-mode Phase 1 checksum
preservation, analysis/event-read neutrality, pause neutrality); a
deterministic malformed-input harness over the genome codec (seeded bit
flips, truncations, extensions, header scrambles, arbitrary buffers;
`LIFESIM_MALFORMED_ITERS` extends the sweep; explicitly not
coverage-guided fuzzing); similarity-analysis tests (determinism, sampling
bound, threshold extremes); an 8,000-tick default multi-generation test and
a 200,000-tick `#[ignore]` release scenario with health assertions only.
Clean-process replay runs through `scripts/verify-phase2-determinism.sh`.

### Phase 1 Gate Status

The Phase 1 suite lives in `crates/sim-core` (unit tests per module plus
`tests/determinism.rs` and `tests/longrun.rs`) and `crates/sim-cli/tests/cli.rs`.
It covers formula bounds, config validation and hashing, worldgen invariants,
lifecycle transitions, capacity rejection, extinction, exact energy/biomass
ledger conservation, pause purity, same-seed replay, seed/config divergence,
CLI summary/fixture/metrics behavior, and invalid-input rejection. Clean-process
determinism runs through `scripts/verify-phase1-determinism.sh` and the CLI
fixture test. The 24-hour-equivalent run (864,000 ticks) is an `#[ignore]`
release-mode test executed explicitly:
`cargo test --release -p sim-core --test longrun -- --ignored`.

## Restore-From-Backup

Phase 4 requires a repeatable isolated restore test. It must start from a completed backup set containing snapshot, event data, configuration, catalog, and checksum manifest; validate the set before loading; restore into a non-live destination; compare world ID, tick, configuration hash, and documented state checksum; and prove that the original world was unchanged. A VM snapshot alone does not satisfy this test.

## Failure Handling

A flaky deterministic test is a correctness failure, not a retry candidate. A performance regression requires a baseline comparison, profiler evidence, and an explicit decision to fix, accept, or revert. Fuzz failures become minimized corpus inputs before the issue closes.
