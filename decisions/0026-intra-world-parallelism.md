# ADR-0026: Intra-World Parallelism

Status: Proposed
Date: 2026-08-04
Author: Scale revision

**Amends ADR-0010** (determinism policy) and
`specifications/determinism-extensions.md` Rule 10, which currently states
that nothing in Phases 5 through 18 authorizes intra-world parallelism.
ADR-0010 keeps its status; its requirement that parallelism prove an
ordering and reduction policy plus equality tests is satisfied rather than
waived.

## Context

One world runs on one core. Phase 5's scheduler parallelizes *across*
worlds, which is correct for campaigns and useless for a single flagship
world: on a 12-core Xeon E5-2680 v4, eleven cores sit idle while the
flagship runs.

That caps a single world at an estimated 10,000 to 30,000 organisms at 1x,
and population is the binding constraint on the project's central question.
Real human populations near 4,000 individuals **lose** technology; the
Tasmanian case is the documented example. A world that cannot exceed tens of
thousands is arguably below the threshold at which cumulative culture is
sustainable at all, which makes raising the single-world ceiling plausibly
the highest-value engineering item remaining.

The question this ADR opens is whether 200,000 or more is reachable.

## The Determinism Target, And Why It Is Higher Than Requested

The instruction that produced this ADR set **Tier 2** as the target:
determinism per thread count, with thread count folded into the config hash
so that a different thread count is a different replay lineage. Tier 1,
thread-count invariance, was described as a bonus.

**This ADR proposes Tier 1**, and the reason is that two facts make it
reachable by design rather than by luck.

**First**, the commissioned methodology review recommends partitioning work
"by a stable function of entity/region ID, **not worker count when
possible**" (section 11.9), and requires that parallel reductions "have a
fixed topology independent of worker count" if thread invariance is claimed
(section 10.8). Both are design choices available here, not properties to be
discovered.

**Second, and decisively, this project's state is all fixed point.**
Integer addition is associative, so a reduction over state-critical
quantities is order-independent *by construction*. The ledger is `i128`
throughout; energy, biomass, and every accumulator are integers; and a
direct check of the kernel finds no float accumulated across organisms. The
only float is per-organism controller evaluation, which lives inside a
single partition and is untouched by how organisms are divided.

That is a direct payoff from ADR-0011's earlier discipline. In a
float-state simulator Tier 1 would be a research problem. Here it is
mostly an engineering constraint to hold.

### The proposed design

Partition count `P` is a **config constant independent of thread count**,
folded into the config hash. Threads claim partitions dynamically; the
partition structure never changes.

1. **Freeze read state** at the tick boundary. Rules 4 and 5 already
   require perception and learning to read only prior committed state, so
   this shape exists.
2. **Partition by stable ID**: partition index is `f(object_id, P)`, a pure
   function. Never by thread count, never by array slice.
3. **Threads produce intents** into per-partition buffers. No thread writes
   world state.
4. **Canonical merge** in ascending `(partition_index, object_id)` order,
   never completion order.
5. **Resolve cross-partition conflicts** under the existing complete
   policies: priority key, then stable ID, then the `lifesim-pairkey-v1`
   lottery where configured.
6. **Commit once**, single-threaded.
7. **Reductions use a fixed tree over `P`**, independent of thread count.

Under this design thread count affects only *scheduling*. Results are
identical at 1, 4, and 12 threads.

### What each tier costs, if Tier 1 fails

| Tier | Guarantee | Cost if adopted |
|---|---|---|
| **1 (proposed)** | Same seed, any thread count, bit-identical | Fixed `P` adds merge overhead and risks load imbalance |
| **2 (fallback)** | Same seed and same thread count, bit-identical | Thread count enters the config hash; a different thread count is a different replay lineage; hardware changes break lineage continuity; a 12-thread bug cannot be reproduced on 1 thread |
| **3 (last resort)** | Statistical similarity only | **Not proposed.** See below |

Tier 3 would be scoped to flagship worlds only and never to campaign worlds,
and the loss is severe enough to state plainly: no replay of an interesting
moment, no reproduction of a defect at tick 30 million, and **no fail-closed
checkpoint verification**. Restore would degrade from "reproduces the
recorded checksum exactly or errors" to "looks statistically similar,"
which means a corrupted restore can pass silently  -  on precisely the one
world whose checkpoint chain is irreplaceable because it can never be
re-run. That trade is bad enough that abandoning intra-world parallelism is
preferable, and this ADR says so rather than leaving it as an option.

## Options Considered

- **Keep the prohibition.** Zero risk, and it leaves the flagship ceiling at
  10k to 30k and eleven cores idle.
- **Optimistic parallel discrete-event simulation (Time Warp).** Rejected
  for the reasons the methodology review gives: it needs rollback state,
  anti-events, fossil collection, deterministic re-execution, and awkward
  integration with learning, mutation, and the event log. A synchronous
  phased model is simpler, easier to verify, and compatible with exact
  checkpointing.
- **Tier 2, partition by thread count.** Simpler, and it gives up thread
  invariance for no gain this project needs.
- **Tier 1, fixed partition count with canonical merge.** Proposed.

## Consequences

Positive: the flagship ceiling rises; thread count stops being a lineage
variable, so hardware can change without breaking a months-long world; a
defect seen at 12 threads reproduces at 1 thread, which is the difference
between a debuggable system and an unfalsifiable one.

Negative and accepted:

- **The achievable speedup is bounded well below core count.** See below.
- Fixed `P` costs merge overhead and can leave threads idle under load
  imbalance. `P` must exceed the maximum supported thread count by enough to
  smooth this, and `P` is in the config hash so it is not freely tunable
  after a world starts.
- **Parallelism is a net loss below some population.** Barrier and merge
  overhead is roughly constant per tick while parallel work scales with
  population, so there is a crossover below which single-threaded is faster.
  That crossover must be measured and the default must be
  parallelism-disabled.
- Intent buffers are new memory traffic and new peak memory.
- Every future phase inherits an obligation: any new cross-organism
  reduction must be integer or use a fixed-topology tree, or Tier 1 silently
  degrades to Tier 2.

Compatibility: config-gated and inert when disabled, following D-014. With
parallelism disabled the kernel takes the existing single-threaded path and
fixtures `0x1e3158a26afd3b39` and `0xff9dfcff5dffbf42` reproduce exactly.

## Performance Implications

**No speedup is claimed. The following is a hypothesis to be measured.**

An Amdahl estimate from the recorded benchmarks, at the 2,000-organism
fixed-population tier:

| Tick phase | p95 (us) | Parallelizable |
|---|---:|---|
| environment | ~202.5 | Yes, per cell |
| spatial index | ~12.2 | Partly |
| sense | 174.3 | Yes, per organism |
| controllers | 473.3 | Yes, per organism |
| apply, lifecycle, finalize | ~27 (residual) | **No, canonical order** |
| **total** | **877.2** | |

That gives a serial fraction near **3.1 percent**, and therefore a ceiling
around **9x at 12 threads** and about 32x with unlimited threads.

Four caveats, and they matter more than the headline:

1. **The table mixes benchmark records.** `sense` and `controllers` come
   from the Phase 2 record; `environment` and `spatial` from Phase 1 at the
   same tier. The project's own discipline forbids comparing records without
   qualification, so this is an orientation for design, not evidence. Phase 18 measures the real split.
2. **The serial fraction grows with every interaction phase.** `apply` is
   currently tiny because Phase 2 has few interactions. Phase 7 contest,
   Phase 12 artifacts, and Phase 13 social all add conflict resolution to
   exactly the serial part. If `apply` reaches 20 percent of tick, the
   ceiling falls to about 3.7x. **The phases this work exists to enable are
   the same phases that erode its benefit.**
3. **Memory bandwidth may bind before core count.** The environment phase is
   a full-grid scan and is memory-bound; a Xeon E5-2680 v4 has four DDR4
   channels, and twelve cores scanning may saturate them. Controllers are
   compute-bound and should scale better.
4. **Barrier overhead is fixed per tick**, so short ticks at small
   populations lose. This is the crossover noted above.

Applying the estimate to the motivating question: at 200,000 organisms and
current phase composition, extrapolating linearly and adjusting for the
Xeon being roughly 2.5x slower per core than the M3 Pro that produced these
numbers, a single tick is on the order of 200 ms single-threaded and roughly
25 ms at 9x. That fits a 100 ms budget at 1x. At full Phase 13 complexity,
with per-organism cost 3 to 10 times higher, the same extrapolation lands
between 70 ms and well over budget.

So **200,000 looks plausible at current complexity and marginal at full
complexity**, which is a useful answer and not a promise.

## Operational Implications

Thread count becomes a recorded execution-class field in every benchmark and
manifest, per the methodology review's requirement that a project state
which machines, thread counts, and toolchains are covered. Under Tier 1 it
is not in the config hash; under the Tier 2 fallback it would be.

## Interaction With ADR-0023

Campaign worlds stay single-threaded and remain the basis for every claim.
Parallelism is a flagship capability.

Under Tier 1 this distinction costs nothing, since results are identical
either way. Under the Tier 2 fallback it would matter a great deal, and the
reason it is still acceptable is that ADR-0023 already bars a flagship world
from supporting a claim on n=1 grounds. A distinct replay lineage on a world
that was never evidence costs nothing scientifically.

## Revisit Conditions

- Fixed-partition overhead or load imbalance makes Tier 1 slower than Tier
  2 by a margin that matters; the fallback is then taken with its lineage
  cost recorded.
- A future phase introduces a genuine cross-organism float reduction that
  cannot be given a fixed-topology tree, which would break Tier 1.
- The measured serial fraction is high enough that the achievable speedup
  does not justify the complexity, in which case the prohibition returns.
- Population turns out not to be the binding constraint on cumulative
  culture after all, which would remove the motivation entirely.

## Evidence Required To Accept

- Phase 18's primary endpoint: two clean processes at the same thread count
  producing identical final state checksums over a run long enough to expose
  drift.
- The Tier 1 claim specifically: identical checksums across 1, 4, and 12
  threads.
- Measured serial fraction and speedup curve, with the crossover population
  below which parallelism is a net loss.
- Fixtures reproduced with parallelism disabled.
- Explicit approval, since this amends a determinism rule.
