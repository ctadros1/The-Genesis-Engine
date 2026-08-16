# Open Questions

## Must Resolve In Phase 0

- What vCPU/RAM/storage allocation can servernode3 safely dedicate after a live audit?
- Which CPU features and scheduling behavior are exposed to the proposed VM?
- What is the existing backup target and retention policy suitable for application-consistent saves?
- Which desktop, mobile, and kiosk browsers must support the observer, and do they expose WebGPU?
- Does the initial prototype need an authenticated observer role immediately or only an admin boundary behind WireGuard?

## Resolve Before The Referenced Phase

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Exact cell size/world dimensions | Phase 1 | Resolved as configurable: `SimConfig` defaults to 256x256 cells at 4 m, benchmarked locally; nothing is hard-coded |
| Genome schema and mutation ranges | Phase 2 | Resolved as versioned policy: `lifesim-genome-v1` (14 bounded scalar traits, fixed topology 1) with `uniform-bounded-v1` variation; all ranges in `docs/06-organism-model.md` and ADR-0011, changeable only through a new schema/policy version |
| Pixel-art visual system | Phase 3 | readable placeholder sprites with overlay |
| Snapshot codec library | Phase 4 | framed custom logical record plus zstd |
| Analytics columnar format | Phase 4 | CSV export |
| SIMD strategy | backlog, needs profiling evidence | scalar/SoA baseline |
| Disaster rules | not scheduled | disabled |
| Communication rules | Phase 13 | `specifications/social-signal-channel.md` |
| Disease rules | Phase 14, optional slice | disabled |

## Open Questions From The 2026-08-04 Goal Change

None of these blocks the next phase. Each is recorded so that a later
implementer does not have to rediscover that it was considered.

| Question | Deadline | Default If Unresolved |
|---|---|---|
| How many seeds and how many ticks make a null result meaningful, given the measured throughput? | Phase 5, from its own measurements | 30 seeds; run length set so the median seed reaches at least an order of magnitude more ancestry generations than Phase 2's 127 |
| Should controller activations move to fixed point along with the learned accumulator? | Phase 9 | Keep f32 activations (proven and benchmarked under ADR-0011) with a fixed-point learned accumulator. Full fixed-point evaluation stays the recorded fallback if cross-platform replay ever becomes a requirement |
| What is the right `propagation_passes_per_tick`? | **Resolved 2026-08-04, ADR-0022 A9.** Superseded by hybrid evaluation: zero-delay edges propagate within a tick in canonical topological order, delayed edges read prior-state buffers. The knob is gone | - |
| Duplication-only structural growth, or duplication plus explicit insertion? | Phase 9 | Measure both (C9.5). If duplication alone cannot produce structural change within the run budget, insertion becomes the default with the measurement recorded |
| Should a regulatory locus type (schema 2 type tag 5) exist? | not scheduled | Reserved and unallocated. Adding it needs its own ADR; `docs/26-biological-realism-policy.md` records why molecular-level regulation is excluded |
| Is providing computed genetic distance as a kinship input channel too generous? | **Resolved 2026-08-04, ADR-0022 A3.** No genotype-distance channel exists. Recognition must be solved from perceptible cues or reported unsolved | - |
| Should plasticity rule form 5 (observational) exist, or must imitation be discovered from generic plasticity plus perception? | Phase 13, by measurement | Run both as conditions P and S rather than deciding by assertion. See `specifications/social-signal-channel.md` |
| When should elevation become mutable? | not scheduled | Immutable. It feeds coastline derivation, drainage, and the lapse term, and the generator validates land fraction and connectivity against it |
| What density threshold selects the dense terrain-modification representation? | Phase 12 | Set from measurement, recorded as versioned config, never a magic constant |
| Do the Phase 7 to 13 campaigns need re-running under `lifesim-physiology-v2`? | Phase 14 | Yes for any result that is to become a standing finding. Stated in ADR-0017 |
| Can the deployment VM supply the compute a full campaign needs? | before any campaign | Unknown and unmeasured. This is a live risk, not a resolved default |

## Open Questions From The Research Reconciliation

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Can 30 to 50 worlds per condition actually be run on the available hardware? | Phase 5 | Unknown. If not, the number of claims is reduced rather than the seeds per claim, and every affected criterion says so |
| What is the smallest effect of interest for each primary endpoint? | Before each campaign | Must be fixed before data collection; there is no defensible default and picking one afterwards invalidates the null |
| Is recognition reachable from phenotype cues alone in this physics? | Phase 13 | Unknown, and the honest answer may be no. C13.7 reports either way |
| Does stigmergic transmission occur, given that Phase 13 was reordered to depend on it? | Phase 12 | Unknown. If not, Phase 13 loses its baseline and reverts to the weaker comparison |
| Does the developmental encoding survive its discontinuity gate? | Phase 10 | Unknown. Failure takes the parameterized fallback and forces a Phase 16 re-plan |
| Should the 2.5D height and support subset land in Phase 12 or later? | Phase 12 | Later. Until it lands the plan does not claim stacked construction |

## Open Questions From Intra-World Parallelism (ADR-0026)

| Question | Deadline | Default If Unresolved |
|---|---|---|
| Does thread-count invariance (Tier 1) hold, or does the fallback to per-thread-count determinism get taken? | Phase 18 | Unknown. C18.2 decides it. Tier 1 is proposed because the all-fixed-point state makes cross-partition reductions order-independent by construction |
| What is the real serial fraction, and does it survive Phases 7, 12, and 13? | Phase 18, re-measured after each | Unknown. The ~3.1 percent estimate mixes two benchmark records. `apply` is the serial part and those three phases each add conflict resolution to it |
| Below what population is parallelism a net loss? | Phase 18 | Unknown. Barrier overhead is roughly fixed per tick while parallel work scales with population, so a crossover exists. Default stays disabled until it is measured |
| Is 200,000 organisms in one world reachable? | Phase 18 | Plausible at current phase composition, marginal at full Phase 13 complexity. This is the question the phase exists to answer |

## Open Questions From Lifetime Learning (Phase 11, raised 2026-08-10)

| Question | Deadline | Default If Unresolved |
|---|---|---|
| **How is `E-variable` realized?** The plan says "patch locations shift on a configured schedule", but there is **no scripted-intervention machinery of any kind** - no command queue, no intervention list, no patch entity, no capacity override. `intervention` appears twice in `crates/`, both in prose. There is also no resource *patch*: resources are a per-cell scalar field regrown against an elevation-derived capacity, and nothing relocates. The alternative is to express `E-variable` as a **climate** configuration, which is implemented, deterministic, and a pure function of tick - but `season_period_ticks` (36,000) equals `max_age_ticks` (36,000), so an organism sees roughly *one* season per lifetime, which is between-generation variation and tests the opposite of what the criterion is about. Shortening it below the lifespan is a real ecological change needing its own stability control, and a smoothly moving field gives C11.1's per-individual before/after window no discrete event to align on | **Resolved 2026-08-11. The machinery was built and climate was not substituted**, which is what the recorded default asked for. `E-variable` is Phase 12's mutable-world half: a relocating capacity patch that is a pure function of `(seed, config, tick)`, moving on every multiple of `worldmod.relocate_interval_ticks` and giving C11.1's window pairs a discrete event to align on. Its control is a **zero-magnitude schedule** rather than a schedule-free world, because relocating a patch trims biomass into a ledgered loss sink. The treatment bit ecologically: median final population 5,964 against 2,846 in the A arms and 5,653 against 2,716 in the B arms at matched seeds. See D-105 | - |
| **Does a founder with non-inert plasticity genes change the result, and is that admissible?** The census (D-099) shows the founder sits at conjunction depth 0 and the phenotype needs four conditions on one locus, each behind a different one of seven point-mutation targets. A founder starting at **depth 3** - a live rule, a positive `eta` and a nonzero coefficient, with only `EDGE_FLAG_PLASTIC` left to evolve - converts a four-step conjunction into a one-step toggle and is the only change acting on all four conditions at once. Weaker variants: a rule reachable in one mutation, or a mutation regime in which one point mutation moves more than one plasticity field. **The admissibility question is not rhetorical.** Under ADR-0012 the test is whether you can name the outcome the mechanism makes more likely, and a founder pre-loaded with three of the four conditions for learning names it exactly. ADR-0018's bounded exception permits shaping the *environment and selective structure* toward a transition, not granting a trait, and this sits close to the line: it grants no behaviour, since a depth-3 edge with the flag clear still computes bit-identically to the founder, but it is a starting point chosen because of what it is one step away from. D-030's `seeded` origin mode makes the same argument for founder archetypes and was found admissible | Before any Phase 11 re-run | **Unknown, and it needs its own ADR before it is built.** The provisional reading is that it is admissible on D-030's reasoning - authoring where the search begins is not authoring the path it takes - provided the depth-0 founder runs as a matched control on the same seeds and the claim is reported as the weaker one (ADR-0018). Whatever is decided, the founder's conjunction depth becomes a reported property of every plasticity campaign |
| **Is the plateau-with-a-moat the real Phase 11 finding, and should the per-edge cost be charged only for live rules?** `learn_phase` charges `plastic_edges.len() * cost_per_edge` with no reference to `rule_id` (`world.rs:4008`), under a comment stating the reason: charging only edges that moved would make "turn the rule off" a free way to keep the flag. The measured consequence is that 95.4 percent of the Avar arm's 221,410,876 milli-EU bought rule-0 no-ops, so the flagged half of the path to plasticity is deleterious while its interior is exactly neutral - no monotone non-negative path from the founder to the phenotype exists. **Both horns are real.** Charge every flagged edge and the intermediate states are selected against, which is what the campaign measured. Charge only live rules and the flag becomes free, so it drifts up, and the cost stops being what makes "the amount of plasticity" a selected quantity - which is the whole point of the criterion the cost exists to serve. A third option is to charge in proportion to the work actually done (`applied` rather than `total_evaluated`), which prices the mechanism rather than the marker and leaves the flag cheap but not free | Before any Phase 11 re-run, and before any claim that Phase 11 measured the value of learning | **Unknown, and the honest position is that the current cost model is defensible and its consequence was not anticipated.** The finding stands as a landscape statement either way: a four-condition conjunction at one locus is unreachable in 64 generations whether or not the flag costs anything. Any change here is a new `lifesim-plasticity-v3` under Rule 9 and moves the Phase 11 fixture, so it must not be made incidentally |
| **RESOLVED 2026-08-15, split as the recorded default said.** `WorldMetrics` now carries `plasticity_updates_applied`, `plasticity_updates_static` and `plasticity_updates_refused` beside the total, which is kept rather than replaced - the same shape D-074 used when it split `plasticity_anomalies_total`. Three consequences worth recording. (1) The Phase 11 fixture schema moves 6 -> 7 and the three fields are inserted **before** `controller_faults_total`, because `verify-phase11-determinism.sh` greps `"controller_faults_total":0}` *with the closing brace* and so pins that field as last; appending would have failed the script with a message about faults. (2) No checksum moves: these are reporting fields, the counters themselves were already separate inside `PlasticityCounters`, and `hash_into`'s byte order is untouched. (3) The verify script's "the trace is not a control" list gains `"plasticity_updates_applied":0,` - a strengthening, not a weakening: a world whose every flagged edge carries rule 0 returns a large *total* having learned nothing, so the old list would pass a trace whose arithmetic had stopped running. Measured on the 200,000-tick single-organism trace: applied 400,000, static 0, refused 0, which is the pure case and is now pinned as such. Original question follows | | |
| **Should `plasticity_updates_total` separate applied from static?** It is `PlasticityCounters::total_evaluated` = applied + static + refused, and the confirmatory findings read its 1,109,373,897 as "the mechanism executed a billion times" when 95.43 percent were `StepKind::Static` - the early return for a flagged edge whose rule is 0, taken before any gene is read (D-098). The counters already exist separately inside `PlasticityCounters`; what is exported to the manifest and the report is the sum. This is D-093's split of `plasticity_anomalies_total` into faults and saturations, and D-074's split of `rejected_invalid`, arriving a third time: **one number that can only answer one question gets read as answering both** | Before the next campaign that reports a plasticity update count | **Split it**, on the same reasoning that carried both prior splits, but as its own change: the manifest column set is append-only and adding two columns beside the existing one is a reporting-surface change with a parser obligation. Until then, no report may cite `plasticity_updates_total` as evidence that the mechanism ran, and `learned_edges_nonzero` and `max_abs_learned_milli` (commit `3b52d98`) are the numbers that answer "did anything learn" |
| Should `eta_max` and `decay_max` be config, as `specifications/plasticity-and-learning.md` implies, or stay the literal `1.0` the genome validator has used since Phase 9? | Before the Phase 11 campaign | Stay at 1.0 and treat the spec's `eta_max` as a scale applied at update time. Tightening the genome bound retroactively would invalidate existing genomes |
| Should flag mutation reach `EDGE_FLAG_DISABLED` and `EDGE_FLAG_DELAYED`? Neither is reachable by any operator today, and `insert`'s comment claims "a later point mutation on its flags is the path to zero-delay" - a path that does not exist | Phase 11 or later | Leave both unreachable. Making them mutable changes non-plasticity structural evolution and would need its own control. The stale comment should be corrected either way |
| Condition A shows a higher non-viable-pairing rate than condition B for a reason unrelated to learning: with the gate open, crossover can separate an edge from its `modulator_node`, producing a dangling reference the birth path refuses. Does C11.1's matching need to account for it? | Before C11.1 is measured | Report the rate in both arms and check it is not large enough to confound. The refusal path already counts it |
| **What control boundary should C11.1 use?** The pre-registered control sits at `event + relocate / 2`, which is not age-matched: D-100 measures the offset alone producing `rho = +158` against a null of 30 in a world with no event in it, with a dose-response linear in the offset. Three candidates, none applied. **(a) Offset-balanced**: match each event at `T` to controls at both `T - relocate / 2` and `T + relocate / 2` and average their two distances into one control observation, so the per-pair age offsets are `-R/2` and `+R/2` and a linear age trend cancels exactly - and the measured dose-response is close enough to linear that this removes most of it, leaving the curvature. It changes no boundary set, only the matching, but requires presence over `[T - R/2 - W, T + R/2 + W]`, which narrows the eligible age band by `R/2` at each end and will raise the discard count; the new count must be reported. **(b) Age-stratified**: bin each observation by the organism's age at its boundary and compute the statistic within bins - and the permutation must then shuffle labels *within* bin, or the null still fails to carry the confound. **(c) Between-arm**: drop the within-world null for this criterion and decide on the seed-paired contrast of `rho` against the matched control arm, in which the age imbalance is common-mode. The report already computes it and it is null (+3 milli, [-1, +8], p = 0.707), so (c) needs no new machinery but does need the arms' age distributions reported side by side rather than assumed matched. | Before C11.1 is re-measured | **Do not re-measure C11.1 with the current pairing.** Whichever is chosen, the pre-registration must report the age distribution of the event and the control observations beside each other, so "matched" is a measured fact rather than a design assumption - which is the general lesson, and the one that would have caught this before the campaign ran |

## Open Question From ALIF Format 5 (raised 2026-08-15)

| Question | Deadline | Default If Unresolved |
|---|---|---|
| **The retained format-3 reader cannot read every format-3 file - only the ones this build's own writer produces.** Found while building format 5, by asking why that bump was necessary when Phase 11's config additions were not. It is because Phase 11 **grew the config block by seventeen bytes inside format 3**: four `plasticity` fields plus `genome2.mutation.plasticity_enabled`, appended to a positional, unconditional block without moving the version. A format-3 file written before Phase 11 therefore ends its config body seventeen bytes early and fails `TruncatedSection` in the current reader. The same is true of the Phase 10 morphology block and D-065's climate/contest/origin additions, at their own versions. `decode_snapshot_format3`'s doc says it exists so a real legacy file can be read; for pre-Phase-11 format-3 files that is not true, and the 3-to-current migration inherits the limitation. **Nothing is currently broken by it**: no `.alif` file exists in the repository, the 120 campaign artifacts are format 4, and every format-3 file the tests migrate is produced by `encode_snapshot_format3` in the same build. The question is whether the reader's contract should be narrowed to say so, or the reader widened to resolve a short config body by leaving the later sections at their defaults | Before any claim that a pre-Phase-11 save is readable, and before the next config-block growth | **Narrow the contract, do not widen the reader.** Resolving a short config body to defaults is exactly the "never alter meaning during load" rule broken - a file that cannot say whether climate was enabled would restore as though it said `false`, which is D-065's defect re-introduced deliberately. The honest fix is that `decode_snapshot_format3` documents itself as reading format-3 files **at the current config-block layout**, and that any future config-block growth takes a format bump, as format 5 did. That rule is what makes the limitation historical rather than ongoing |

## Open Questions From The Artifact Half (Phase 12, raised 2026-08-16)

| Question | Deadline | Default If Unresolved |
|---|---|---|
| **Should Phases 9-11 be re-run with `genome2.mutation.binding_q16` nonzero?** D-114 found that no operator could ever bind a channel the founder did not, so every schema-2 result to date - C9.1's fixation null, C10.3, C11.1/C11.2's reachability nulls - is a result about topology evolving over a two-channel interface (`energy_fraction` in, `turn` out) with every other action at its baseline. Whether a richer interface changes any of them is unmeasured | Before any of those results is cited as a general property of the engine rather than of that interface | Not re-run. Each result stands as stated, with the interface named beside it. A re-run is its own pre-registered campaign, not an amendment |
| **Should bond formation be passive, pressure-triggered, or the generic clamp `combine` is?** Review 19.2 asks for the three to be compared on discovery rate, solution diversity and recipe-like narrowing; ADR-0028 ships the clamp because a point-object world has no contact geometry to trigger the other two | Before `combine` is cited as evidence for or against cumulative dependency beyond C12.3's own wording | The clamp, recorded as a deviation. A C12.1 null diagnosed as "wrong action shape" is the trigger to run the comparison |
| **Which RNG stream owns a contested-acquisition lottery?** Rule 3 permits a `lifesim-pairkey-v1` lottery "if configuration enables one" and names no stream; the shipped resolution is (priority, distance squared, organism ID) with no lottery, so no draw is taken and no stream is consumed | Before a lottery is configured | None is built. Review 13.4 notes lowest-ID tie-breaking is a persistent fitness advantage; if a campaign shows organism ID predicting acquisition success, that is the trigger, and the stream must be a new appended value, not 13 or 14 |
| **Does C12.2's cross-sectional fitness comparison need an interventional arm?** Review 15.8 requires "measured function" to mean a causal outcome under structure removal or sham physics; the plan's C12.2 supplies function by matched-cell comparison. The branch-replay harness (restore at a tick, remove or relocate an object, run forward, diff) does not exist | Before C12.2 is claimed at review 10.2 Level 2 rather than as persistence plus correlation | Not built in this pass. The pre-registration states the claim level C12.2 licenses without it |

## Decision Protocol

An unanswered question is not permission to invent production behavior. If it affects persistent compatibility, security, budget, or meaning of an experiment, create a proposed ADR and seek the minimum needed evidence/approval.

## Phase 0 Local Evidence And Remaining Gaps

| Question | Local Evidence | Remaining Gap |
|---|---|---|
| Rust deterministic baseline | Two clean Rust processes matched at 500 organisms/500 ticks; 500 and 2,000 benchmarks completed | Deployment-shaped VM and cross-platform replay policy |
| Snapshot framing | Version/cap/checksum/negative decoding and uncompressed encode/decode measured | Compression library, migration fixtures, atomic filesystem write |
| Browser renderer | Chrome 150 on the Mac completed PixiJS 8.19 WebGL and WebGPU at desktop and mobile-sized viewports | Named supported browsers, physical mobile/kiosk hardware, non-headless WebGPU matrix |
| Proposed VM | No host or VM was accessed | Every VM acceptance checklist item remains open |
| Authentication boundary | Not exercised by these local spikes | Observer/admin decision remains open |

## A second confound in C11.1's pairing: DIAGNOSED (2026-08-11)

**Status: diagnosed, and it is structural rather than a bug. The per-world
count rule is not an identified estimate and cannot be made one by choosing a
better stratifying variable. The seed-paired between-arm contrast IS
identified, is already computed, and is null.**

### The diagnosis

Measured, not argued, by the same method that found D-100. A world was built
whose behaviour is a pure function of the **absolute window index** with no
age dependence at all - every organism alive in a given window behaves
identically whatever its age - and in which nothing happens at the event tick.
The age-stratified statistic reads **rho 47 against a null of 35** there: a
manufactured false positive, in the same sense D-100's +158 was.
(`a_calendar_trend_reaches_the_statistic_and_age_stratification_cannot_remove_it`.)

**Why no stratification can fix it.** For a fixed organism, age and calendar
time advance together. An event observation and a control observation that
share an age stratum came from organisms born `relocate / 2` apart and are
read `relocate / 2` apart in absolute time, so **matching age necessarily
unmatches calendar time**. Stratifying on absolute tick instead puts every
event and every control in disjoint strata, because event boundaries are
always at multiples of `relocate` and control boundaries always at the
midpoints - the two labels are perfectly separated by phase. There is no third
variable that separates them, because the label *is* the phase.

Direction matches the campaign: with the calendar trend decaying, `event_wins`
500 against `control_wins` 320; with it accelerating, 120 against 500 - the
campaign's own direction, `control_wins` above `event_wins` in all four arms.

**A trap found on the way, worth its own line:** the pooled rank correlation
and the paired win counts can disagree. The accelerating world's control side
wins four times as often while pooled rho is +25 - not significant, and not
the sign the paired view implies - because 1,300 of 1,920 pairs are tied.
Neither summary is broken; they answer different questions and must not be
read as a readout of each other.

### What follows

**The identified comparison already exists and is already null.** Avar and
Bvar share the seed, the terrain, the relocation schedule and therefore the
entire phase structure, differing only in plasticity. The calendar confound is
common to both arms and cancels in the seed-paired difference, which the
report computes: **+3 milli, 95 percent CI [-1, +8], 14 of 30 directed,
p = 0.707**. That is C11.1's honest answer on this data, and it is a null.

**Proposed for a future pre-registration, not applied here:** C11.1 should be
decided on the seed-paired between-arm contrast rather than on a per-world
count of worlds clearing a within-world null. The per-world rule was written
before the confound was known and cannot be repaired in place. Changing it is
a criterion change and needs its own pre-registration; it must NOT be swapped
in silently, and the existing 0-of-30 result stays on the record either way.

**The alternative, if the within-world statistic is wanted at all:** jitter the
relocation schedule so that "is this boundary a relocation" stops being a
deterministic function of the tick's phase. That is a kernel change to
`worldmod` scheduling, not an analysis change, and it is the only way to make
event and control boundaries comparable at the same phase.

### Superseded framing



D-100's age offset is fixed and the fix is proven on a synthetic tripwire:
where the age offset is the only difference and no event exists, the
statistic reads 0 against a null of 6, where before it read +158 against 30
and passed. The dose-response is flat at zero across gaps of 500 to 4,000 and
across 240 to 7,680 pooled pairs.

**But re-analysing the confirmatory campaign under the corrected statistic
leaves a strong negative association in all four arms, including Bstat** -
plasticity disabled, relocation at zero magnitude, nothing happening at the
event tick. Two-sided `associated` is 29-30 of 30 in every arm, median |rho|
46-61 against nulls of 9-14. An association that survives removing the event
was not caused by the event, which is the argument that exposed D-100 now
applying to what replaced it.

The lead, stated as a lead: `event_wins` runs below `control_wins`
consistently, and in the E-stationary arms the medians separate outright
(Astat event 8 / control 52; Bstat event 16 / control 64). Behaviour is
systematically *more stable* across a relocation boundary than across a
mid-epoch one, in arms where the relocation does nothing. Age is now matched,
so what still differs between the two pairs is epoch phase and absolute tick.

Candidates not yet distinguished: a sampling-phase artifact (the sample at the
relocation tick is taken after the relocation ran, so the event pre-window
carries one post-relocation tick out of 500); world-level temporal structure
at the relocation period that survives zero magnitude; or something about
the zero-inflated tie structure interacting with Spearman.

**Why this does not invalidate the current verdict.** The decision rule is
directed and requires a positive correlation; this association is negative,
so C11.1 refuses in every arm under both v1 and v2 and the 0-of-30 stands.
**Why it still matters:** a future positive result from this statistic could
not be trusted until the second confound is explained, so C11.1 must not be
re-run for a positive claim on the strength of the D-100 fix alone.

Answering it is cheap - the campaign artifacts are on disk and the statistic
is now instrumented per world with strata counts - and it should be answered
before any C11.1 re-run is pre-registered.
