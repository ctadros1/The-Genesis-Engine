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

Phases 5, 6, and 7 are complete; Phase 8 is implemented with its primary
endpoint met and three secondary criteria unmet. Phases 9 to 18 are planned
and none has started:

| Phase | Subject | Plan |
|---|---|---|
| 5 | Headless scale and multi-world experiments - **done** | `phase-5-headless-scale-and-experiments.md` |
| 6 | Biomes, climate drift, and world origin modes - **done** | `phase-6-biomes-climate-and-origins.md` |
| 7 | Territory, contest, and damage - **done** | `phase-7-territory-and-conflict.md` |
| 8 | Demography and life history - **implemented; C8.1 met, C8.5/C8.6/C8.7 unmet** | `phase-8-demography-and-life-history.md` |
| 9 | Evolvable genome: diploid genetics and variable topology | `phase-9-evolvable-genome.md` |
| 10 | Modular morphology and development | `phase-10-modular-morphology.md` |
| 11 | Lifetime learning - **COMPLETE 2026-08-16: every criterion decided; C11.3-C11.8 met, C11.1/C11.2 unmet as measured nulls with controls** | `phase-11-lifetime-learning.md` |
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

**Phase 12's mutable-world half is built and verified; the artifact half is
NOT STARTED** (2026-08-10). Met: **C12.5** (ALIF format 4, every clause,
each mutation-verified) and **C12.8** (all four fixtures preserved).
Partial: C12.4 (modification-set ordering and identity done; no objects, no
Phase 12 fixture), C12.6 (capacity trim ledgered exactly; **no 10^6-tick
run**, and mass does not exist because objects do not), C12.7 (terrain-write
caps typed and counted; object caps unbuilt). **Not built:** C12.1, C12.2,
C12.3 - all three need the object system.

**Three corrections landed before any code**, each recorded rather than
worked around: the plan argued the pre-reversal ordering and named a Phase
13 fixture that does not exist (D-096); the save format successor is **4**,
not the "format 2" every spec said, and the format-1-to-2 migration those
specs demand **was never implementable** - a format-1 file cannot say what
its climate settings were; and `migration_for` had no transform type at all,
so a "registered migration" was not expressible.

**The measurement worth carrying: the cost is the seam, not the data.**
Enabling the section costs ~20 percent of tick time with *zero* overrides
(253.6 -> 305.4 us at 65,536 cells), because every terrain read goes through
a composed accessor; going from zero to 6,187 overrides costs another 3
percent. The plan's risk table anticipated the opposite shape.

**The composed checksum is a full recompute, not incremental.** FNV-1a
cannot be updated for a value changed mid-stream, and a recompute costs
~1 ms at 65,536 cells - about four ticks - so it runs on a cadence. The spec
asks for incremental cross-checked against full; only full exists, and the
reason is recorded rather than the gap being papered over.

**A guard defended by nothing, found by mutation** (D-097): sortedness and
uniqueness are decode-time invariants, and deleting either guard left every
test green, because a disordered set also fails the composed checksum and
the tamper test accepted *either* error. Two tests now pin the near guard by
its diagnostic. The general pattern - a guard whose only test accepts a set
of errors rather than the one it should produce - is worth grepping for.

**PHASE 12 IS COMPLETE (2026-08-16): all eight criteria decided**, both
halves. Met: C12.4-C12.8 (own fixture `0x853d257398a2718c`, format 7,
10^6-tick ledger soak clean, caps driven, five fixtures preserved).
**C12.1-C12.3 UNMET as measured nulls with controls** (D-117) - not a
reachability null: the bind operator (D-114) put every object action into
every world's living population; firing them did not pay at this reach,
cost and horizon. Follow-ups recorded, each needing its own
pre-registration: the D-118 inert-arm fix under `lifesim-artifact-v2`
first (**landed 2026-09-01, D-119**: inert skips exactly the five verbs,
fixture re-pinned `0x21405a5c0591ceeb` / `0x24defb6052eb9d42`), then
reach / hold cap / costs / bind rate as the knobs, and ALSP 1.1 if
objects should reach the observer.

## The Ladder To Learning And Culture (from the Phase 11 and 12 nulls)

Recorded 2026-08-16, consolidating what D-099, D-105, D-117 and D-118
measured into the four levers a follow-up moves. None of these is a new
criterion or a change to any decided one; each is a candidate campaign
that must carry its own pre-registration before it runs, and the D-118
inert-arm fix under `lifesim-artifact-v2` lands before any of them
(**landed 2026-09-01, D-119**).

1. **Make the payoff bigger and closer.** Both nulls have the same shape:
   the mechanism was reachable and firing, and it did not pay. The
   Phase 12 refusal ledger names the specific knobs - `reach_m` (14.1M
   NoTarget refusals at 2 m), `max_held_objects` (4.1M HeldCap at 1),
   the action costs, and object value itself (placed objects currently
   confer nothing; a shelter or storage effect would be a new ADR under
   `lifesim-artifact-v2`, not a tuning change).
2. **Make the world demand learning within a lifetime.** C11.1's
   instrument existed and its nulls were real: a static-enough world
   rewards instinct, and plasticity was selected to zero. The relocating
   patch (`worldmod.patch_*`) is the built mechanism for within-lifetime
   change; the unexplored region is change fast enough and consequential
   enough that a fixed genome cannot track it - swept as its own
   pre-registered campaign, with the Phase 11 age-stratified analysis
   reused as the ruler.
3. **Longer horizons.** Every behavioral campaign so far ran 60,000 ticks
   (~60 generations at the confirmatory base). The kernel sustains 10^6
   ticks (C11.6, C12.6) and the scheduler is embarrassingly parallel
   across worlds; a 10x horizon at 30 seeds is compute, not design. The
   honest caveat from D-105 stands: a null at 60k ticks licensed "not at
   this horizon", so a longer-horizon arm is the direct test of that
   caveat, not a fishing trip.
4. **Then culture (Phase 13), in order.** Culture is learned behavior
   transmitted between individuals; the social channel is the built path
   for it, and Phase 17's era/tradition detection is the ruler. The
   ordering constraint is stated here so it is not rediscovered: imitation
   has nothing to copy until levers 1-3 make some within-lifetime
   behavior worth copying, so Phase 13 should ship its mechanism and its
   reachability census expecting the transmission criteria to read null
   until a learning campaign under levers 1-3 succeeds.

**PHASE 11 IS COMPLETE (2026-08-16): all eight criteria decided**, benchmark
schema 7, record `phase11-local-20260816T063000Z`. Met: C11.3, C11.4, C11.5,
C11.6 (10^6 ticks, 18,971,594 plasticity updates, ledger exact), **C11.7**,
C11.8. **C11.1 and C11.2 are UNMET, as measured nulls with controls** - they
stood at NOT MEASURED until the four missing pieces were built, and a
measured null is a result rather than a gap.

**C11.7 closed 2026-08-16** (D-113). The checkpoint stall is now measured
through `AsyncCheckpointer` rather than substituted by synchronous encode
time: against a 100 ms budget, async checkpoint-tick p50 is 3.41 ms at tier
500 and 13.94 ms at tier 2000 against synchronous 21.22 and 52.07, worst
async tick 15 percent of budget against synchronous 62 percent, and
`async_refused` zero everywhere. **Plasticity does not measurably move the
stall** - seeded against off is +2.5 percent at one tier and -2.4 percent at
the other, a sign change, so it is noise. Learn-path allocation is zero:
3,125,484 applied updates added none.

**Two harness defects were found producing that record**, both fixed. The
benchmark script never invoked the kernel-side target and said it "is not
written yet" long after it existed, so every record had no `learn` lines. And
with `--test-threads 1`, which the timed benchmarks require, cargo prints
`test <name> ... ` without a newline, so a `grep '^PHASE11-BENCH'` dropped
each test's **first** measurement - the zero-plastic-edge baseline. The
script now uses `grep -o` and fails if the record is not exactly 25 lines.

**Both behavioural criteria returned a null, pre-registered in `4b160fe`
before the run.** 120 worlds, 4 arms x 30 seeds, 60,000 ticks, 0 failed.
C11.1 Avar **0 of 30** against a bar of 20 with the Bvar control at 0; C11.2
Avar **8 of 30** against a bar of 20 with Bvar at 0, all eight on the
plastic-flag scale and none on `eta`. No threshold was changed after the
data. The seed-paired between-arm contrast of C11.1's statistic is null as
well: +3 milli, [-1, +8], p = 0.707.

**The null is a null at ten times the shipped point-mutation rate**
(`point_q16 = 65535` against 6554) and at a patch of radius 32 / scale 4.0
against defaults of 15 / 2.0. Both were raised in pilot calibration ruled by
controls rather than outcomes - the marker had to actually drift, and the
schedule had to actually change the world - and both bound how far the null
generalizes. At the shipped rate the phenotype is further out of reach, not
closer.

**The census overturns what the null is about, and it matters for three
revisit conditions.** The phase's Risks table names "plasticity is selected
to zero" as its most likely failure. A null happened; **that mechanism did
not**. Plasticity was never assembled. A nonzero learned delta needs four
conditions on one edge locus, each behind a different one of seven
point-mutation targets, plus a fifth for the two modulated rules. **9 of
684,370 Avar edge alleles carry all four** - 13 per million, from at most
four independent assembly events in 30 worlds - and 25 of 48,119 plastic
edges ended with a nonzero learned weight in 14 worlds. Every incomplete
state computes bit-identically to the founder, so selection cannot see it,
while the learn phase charges every flagged edge whatever its rule: a
plateau with a moat, not a gradient. The revisit conditions on D-023, D-025
and D-035 are phrased on "selected to zero" and are **not** triggered
(D-099, D-105).

**Three readings in the confirmatory findings file were corrected rather
than rewritten** (the file is a committed artifact; the correction is
appended and marked). `mean_abs_learned_milli = 0` was read as "nothing
learned" and is a truncation - 139,116 Q16 over 48,119 rows (D-098).
"1,109,373,897 updates" is 95.43 percent `StepKind::Static` no-ops. And the
explanation named two genes where there are four conditions plus a fifth.

**C11.1's matched control is not age-matched, and the offset alone can
produce a pass** (D-100). Measured in a stationary rolling cohort with no
event in it: rho = +158 against a null of 30, with a dose-response of
76/158/334/700 at offsets of 500/1,000/2,000/4,000 ticks. The sign follows
the substrate's age trend, so the directed statistic is not identified and
this campaign's 0 of 30 depends on which way that trend runs. Nothing was
changed to accommodate it; **C11.1 must not be re-measured with the current
pairing** until a corrected boundary is pre-registered.

**And the instrument could resolve one number** (D-101). Reproduced
independently on fresh worlds with a from-scratch `.alac` reader: `eat` and
`mate` equal the organism's age in 100.0000 percent of 1,175,285 records and
`rest` and `attack` are 0 in 100.0000 percent. Only the three heading bands
vary - a partition, so two degrees of freedom, one of which carries 0.59
percent.

**What is not claimed.** The phase built the mechanism, measured its cost,
safety and persistence, and has now measured both behavioural criteria. It
has *not* shown that anything learns anything useful, that plasticity is
selected for, or that behaviour changes within a lifetime in a way selection
did not produce. It has also **not** shown that lifetime learning is
worthless here: the census measures reachability, not value.

**Verification found no defect in the kernel or the codec, and 41 real gaps
in the tests defending them.** Three independent adversarial passes ran 103
mutations of their own design against the C11.1 decision path, the C11.2
comparison with the marker's four matched-control properties, and the kernel
measurement substrate with the `.alac` codec: 41 survived, 34 were real gaps
now closed, and 7 are tautologies named in D-100, D-101 so nobody writes a
test for them. Every mutated line was correct as shipped. Two prior reports
of "28 injected, all caught" and "29 mutations all caught" did not survive
contact, for one reason worth carrying: **a mutation set chosen by the
author of the tests preferentially hits what the tests already cover.**

**Three defects worth carrying.** The snapshot loader **failed open on
hostile input in five sections since Phase 6**: every allocation bound read
`count.checked_mul(size) > Some(body_len)`, and `None > Some(_)` is false, so
overflowing counts reached `Vec::with_capacity` (D-091). The plasticity genes
were a reserved schema slot rather than a mechanism - nothing wrote
`EDGE_FLAG_PLASTIC` at all, so the phase's most likely predicted failure was
mechanically guaranteed. And the shipped per-tick plasticity cost is **zero**:
`2 milli/s` at `dt_ms 100` truncates to 0, so "plasticity is under selection"
is false as configured (D-092).

**Phase 10 is complete except C10.3** (2026-08-06), benchmark schema 6.
C10.1, C10.2, C10.4, C10.5, C10.6, C10.7, C10.8, C10.9, C10.10 and C10.11
are met.

**Morphology has consequence and does not spread.** Body size predicts
reproductive success in 26 of 30 worlds against a within-world permutation
null (median rho +132 against +47), and under a frozen control the same
measurement is undefined in 30 of 30. But the median body is the founder's
three modules in every world while the mean is 4.65 and the median world
carries 33 distinct bodies, so C10.3's divergence clause fails 0 of 30 and
the conjunctive criterion is **not met**. At 89 percent silent morphological
mutations, nothing reaches half a population in 60 generations - the
fixation-scale problem D-079 recorded for C9.1, on a different mechanism
(D-080, D-087).

**The developmental encoding survived its own gate.** ADR-0022 D1 made C10.4
pass/fail so a discontinuous genotype-phenotype map would take the
parameterized fallback. It passes at a median of 0 against a bar of 500, and
narrowly: 89.0 percent silent against a 90 percent vacuity ceiling.

Four defects were found by measurement rather than review, and one of them
killed 29 of 30 worlds on the first campaign run: an uncalibrated derived
phenotype (D-085), the morphology config missing from the snapshot codec,
a fixed-morphology control that was not fixed, and an analysis that swallowed
restore failures (D-086).


**Phase 9 is complete** (2026-08-10). All eight criteria are met. C9.6
closed by emitting `EventKind::StructuralMutationRejected` (log tag 12,
event schema 4) and reconciling it against the checksummed counters class
by class (D-088). C9.7 closed with the **Phase 9 fixture** - config
`0x9abc0cd47914127f`, state `0x5f0c4e95e4f5170f`, 8,000 ticks - the
compaction tests, and a stated substitute for the storage-permutation
clause, which is **unfalsifiable as written**: a complete joint permutation
followed by the id-sort `from_state` requires is the identity on the record
(D-089).

Two results carry forward, and both were found by measurement rather than by
inspection.

**At the shipped default mutation rates, structural evolution never reaches
the population median - 0 of 30 worlds.** Duplication and deletion ship at
the *same* rate, so genome size sits at mutation-deletion equilibrium with no
expected growth. It takes ten times the duplication rate (29 of 30) or
insertion enabled at its default rate (30 of 30). Standing structural
variation is real at every rate - nine distinct structures per world even at
the defaults, against exactly one under both controls - it simply never
fixes. The median is a fixation-scale statistic and C9.1 asks a
fixation-scale question. C9.2 passes at every rate on both population and
lifespan, by TOST equivalence as well as by count, with zero extinctions in
210 worlds; the cost of structural freedom is monotone and small, about 5
percent of population at the highest rate (D-079).

**The structural caps were mutually inconsistent and three of the four could
never bind.** `max_genome_bytes` ran out at ~341 loci while `max_nodes` and
`max_edges` would have needed 58 KB. Restated from the C9.8 measurement to
160/160/160/32 with the byte cap unchanged, under the rule that every cap
must be individually reachable within the byte budget (D-078). Two related
findings invert the plan's stated risks: schema-2 snapshots are **smaller**
than schema-1's (1.7-1.9 KB per organism against ~2.8 KB) and the tick is
**1.4x to 1.6x faster** - both because evolved topologies are tiny next to a
fixed 20-16-12-12 network, which is a statement about how little structure
evolved rather than about the evaluator.

Four defects were found along the way, all in paths the campaign would have
exercised: a merged-network zero-delay cycle that validation could not see
and that crashed the sense phase (D-074), meiotic recombinants that were
never validated, a snapshot section that made schema-2 worlds
**uncheckpointable** while its round-trip test passed by never touching the
codec (D-076), and a counter list maintained by hand that silently changed
restored checksums (D-077).

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

Phases 5 through 10 are done, **Phase 9 entirely** (2026-08-10) and Phase 10
bar C10.3. The Phase 9 debt that protected everything after it - C9.6's
event, C9.7's fixture and compaction tests, and the manifest's per-class
rejection counters - is closed (D-088, D-089, D-090).

**The next implementation phase is 11, lifetime learning**, which has not
been started.

**Correct a claim this file used to make.** It said `PlasticityGenes` had
been "carried, inherited, and validated since Phase 9 precisely so that
enabling it is a flag rather than a schema change." The genes are indeed
carried, inherited and validated - but they are a **reserved schema slot,
not a working mechanism**, and enabling a flag would measure nothing.
Verified against the shipped code on 2026-08-10:

- `PlasticityGenes` is **discarded during diploid expression** - the gather
  destructures `LocusKind::Edge { source, target, weight, flags, .. }` and
  the `..` drops it, and `ExpressedEdge` has no plasticity field at all.
- **No production path anywhere sets `EDGE_FLAG_PLASTIC`.** It is defined,
  included in the flag mask, read during expression and exported, and
  written only in one test. `insert` writes `EDGE_FLAG_DELAYED`,
  `minimal_founder` writes `flags: 0`, `duplicate` copies its source. **No
  edge can become plastic.**
- `point_mutate`'s only Edge arm touches `weight`, so **`eta` can never
  leave zero**.
- `NodeRole` is never mutated, so `Modulatory` is unreachable by evolution
  and rule forms 3 and 4 are dead on arrival.

This matters more than a scoping correction. Phase 11's own risk table names
"plasticity is selected to zero and the phase returns a null result" as
**the single most likely failure of the phase**. On today's code that null
is guaranteed for a purely mechanical reason, and it would have been read as
biology. Expression and mutability are therefore Phase 11 scope, and the
`plasticity_enabled` control must consume its draws either way for the same
reason `regulatory_enabled` does (D-086).

Phase 10 inherits a warning from Phase 9 worth stating here rather than
rediscovering: **a criterion phrased on a population median is a
fixation-scale question**, and at realistic mutation rates most variants
never fix. Phase 10's acceptance criteria should say which scale they mean
and set the rate accordingly, or they will measure the mutation rate.

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
4. ~~**Phase 8, demography and life history.**~~ **Implemented** (D-063 to
   D-065). Starvation share 494/1000 against a baseline of 1000/1000, food
   field at 99 percent of capacity, and every world far below the guard, so
   the culture stack is now measurable. Three secondary criteria are unmet
   and stay unmet; C8.7's control failure in particular is worth revisiting
   before any claim about thermoregulation.
5. ~~**Phase 9, evolvable genome, diploid genetics, variable topology.**~~
   **Complete** (D-066 to D-079, D-088, D-089). Campaigns measured across 210
   worlds; caps restated from measurement; four defects found and fixed in
   the schema-2 birth and persistence paths.
6. ~~**Phase 10, modular morphology and development.**~~ **Complete bar
   C10.3** (D-080, D-081, D-085, D-086, D-087).
7. ~~**Phase 11, lifetime learning.**~~ **Measured** (D-092 to D-105).
   Seven of eight criteria decided; C11.1 and C11.2 are unmet measured nulls
   and C11.7 is partial. Two things are owed before the phase is closed or
   re-run: the **age-matched control boundary** C11.1 needs, which must be
   pre-registered rather than chosen after the fact, and the
   **checkpoint-stall measurement through `AsyncCheckpointer`** that C11.7
   still substitutes synchronous encode time for.
8. Resolve the remaining Phase 0 decision gates (deployment-shaped VM
   benchmark). This bounds the compute-cost risk recorded as unresolved in
   `docs/20-risk-register.md`. Phase 5 measured the development host only;
   no supported campaign size is claimed.
9. If separately approved, run a read-only servernode3/monitoring/backup
   audit and record live facts.
10. Test physical target desktop, mobile, and kiosk browsers against the live
   server; do not treat viewport emulation as device evidence. Note that
   ADR-0024's voxel path reopens this gate with a different rendering
   technique, so sprite-path device evidence will not transfer.
11. Phase 18, intra-world parallelism, when the single-world population
   ceiling starts to bind. Scheduled after 8 and before Phase 13.

## Deferred Backlog

Carried forward unchanged unless noted:

- **Revisit C10.3's divergence clause before any phase reuses it.** It was
  operationalized as a median shift, and the median is a fixation-scale
  statistic that at realistic mutation rates never moves. A distributional
  statistic - 33 distinct bodies against the founder's one - answers a
  better-posed question. The rule was not revised after the run and should
  not be; what should change is how the *next* such criterion is written.
- **Sweep the mating compatibility weights.** Phase 9's risk table listed
  reproductive isolation as a risk and the campaign did not test it: at a
  median of three nodes there is almost no structural distance for the
  metric to act on. It becomes testable once evolved structures diverge
  further, and it should be swept before any phase relies on it.
- **Audit every remaining hand-maintained field list in a codec.** D-075 and
  D-077 are the same defect twice: a decode length check that encoded a
  field count, and a serializer that listed fields by hand. Both were found
  by accident. Destructuring makes the compiler enforce the second; the
  first needs each exact-equality length check replaced by a bound.

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
- **Cache the allometric multiplier per organism.** It triples the `apply`
  phase - two Newton-iteration integer square roots per organism per tick -
  and body scale never changes during a life. Profiler evidence is in the
  Phase 8 record; the change is a pure memoization with no behavioral effect.
- **The moisture-exchange cadence now costs more than two phases of work
  combined.** The Phase 8 record prices it: 1,492 microseconds per 1,000
  organisms in `environment` against 24.5 in `apply`. D-050 opened this;
  it is now the single largest throughput item in the engine.
- **A control that separates food distribution from temperature**, so C8.7
  becomes answerable. Biome capacity correlates with temperature, so
  organisms follow food into thermally non-random cells whether or not they
  can perceive temperature, and the phase's inert-gene control inherits that
  correlation.
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

## Next: ALIF format 5, and why item [2] needs it first (2026-08-11)

**Blocking the chain half of D-107's 2x2.** Discovered by building it and
hitting the wall, not by review.

The chain change has to be a **runtime setting**, not a build constant: the
2x2 must run chain-on and chain-off arms against the same seeds in one
campaign and on one build, and a compile-time registry change cannot be an
arm. Making it a setting also keeps every existing fixture intact while it is
off, which a constant change would not.

But a new field on `PlasticityConfig` **must** be encoded into the snapshot.
Not encoding it is D-065's exact defect - a restored world would silently run
with the flag false and no error anywhere. And `encode_config` is positional,
so a new field shifts every later field and makes existing format-4 files
undecodable. The 120 `.alif` campaign artifacts are format 4 and are still
being read for re-analysis, so silently breaking them is not acceptable.

**So the order is: format 5 first, then the flag.**

Increment A - ALIF format 5: **DONE 2026-08-15.** See D-108 and the Phase 11
notes in `specifications/world-save-format.md`. What shipped, against what was
planned:

- `FORMAT_VERSION` 4 -> 5, `FORMAT_VERSION_4` retained, plus a retained
  format-4 **writer** (`encode_snapshot_format4`) that was not in the plan and
  is required: there is no `.alif` file in the repository, so the only way to
  produce a real legacy file to migrate is to write one.
- The reader is a real reader - the version is threaded through
  `decode_payload` to `decode_config`, which is the single place the format
  difference is expressed, so it cannot drift from what a format-4 file holds.
- `FORMAT4_TO_CURRENT` in `migration_for`. Named for its **source**, not its
  target: `FORMAT3_TO_FORMAT4`'s `to_format` was already `FORMAT_VERSION`, so
  its name went stale the moment the constant moved. Both transforms land on
  the current format in one hop; `decode_snapshot_migrating` applies one.
- Standing rule 2 applied to the config section's declared length at seven
  adversarial values with the payload CRC resealed each time, plus every
  truncated-body prefix - including the one-byte-short case, which is exactly
  a format-4 body and the only one an implementation could want to accept.
- Byte-identity clause discharged at all three levels: `SaveState`, world
  equality, and 200 further ticks.

Three things the increment turned up that were not anticipated, all one class -
**a comparison or a helper written against "the current format" while meaning
"format 4"**:

1. `SECTION_WORLDMOD` and `SECTION_ACTION_CENSUS` were guarded by
   `format < FORMAT_VERSION`. At format 5 that refuses both sections in every
   format-4 file on disk. Now `FORMAT_VERSION_4`.
2. `a_format_3_file_carrying_a_format_4_section_is_refused` built its subject
   with the current-format writer, so it began failing on the new config byte
   before reaching the section it exists to test - still red, but for the
   wrong reason, and it would have been green again with the guard defect in
   place.
3. `a_disabled_world_encodes_the_payload_format_3_wrote` compared a format-5
   file against a format-3 one and asserted they differ in two bytes.

Also recorded as an open question (2026-08-15): the retained format-3 reader
can only read the format-3 files this build's own writer produces, because
Phase 11 grew the config block seventeen bytes *inside* format 3. Nothing is
broken by it today - no pre-Phase-11 format-3 file exists - but it is why
format 5 is a bump rather than an append.

**The field is encoded, not behavioural.** `validate` refuses
`live_rule_zero == true` and the flag is not in the config hash, both of which
come out in increment B. No fixture moved.

Increment B - `plasticity.live_rule_zero` becomes behavioural. NEXT.
- The WIP diff referred to here is **gone** - it lived in a scratchpad that did
  not survive the session. Rebuild from this description; it is complete.
- `PlasticityBudget` (today a bare `Option<u32>` type alias in
  `controller2.rs`) becomes a struct carrying `max_plastic_edges` and the
  flag, with `disabled()` / `edges(n)` / `with_live_rule_zero()`. The rule
  remap is done **once** in `compile_with_budget` so `plasticity::step` stays
  a pure function of the rule it is handed.
- Delete the `validate` refusal added in increment A, in the same commit that
  gives the flag effect - and re-check every test whose world might now reach
  a live rule 0, because a test whose two sides both become the same error
  passes with the defect present.
- Hashed **only when true** (D-014 at field granularity), so the Phase 11
  fixture does not move for a setting nobody switched on. Increment A
  deliberately left it out of the hash: a hash difference claims a
  replay-lineage split, and there was not one while the flag did nothing.
- Already registered in `FIELD_NAMES` and in the codec by increment A, so
  `config_field_coverage.rs` has been defending it from the start - which is
  standing rule 3 working, one increment earlier than it had to.
- Do **not** redo this as a constants change. `RULE_COUNT` 5 -> 4 plus a
  renumber builds sim-core, breaks sim-analysis, moves fixtures for no reason,
  and cannot be an arm of the 2x2 anyway: both arms must run on one build.

Increment C - the moat, then the 2x2 campaign, pre-registered as in D-107.
**Moat DONE 2026-08-16** (D-111, ALIF format 6 / D-112). Campaign
pre-registered and committed; **not yet run**.

The moat's price basis is **state movement**, not `StepKind::Applied`, and the
pre-registration was corrected before the code was written rather than after
the campaign. `Applied` means "the rule ran and the state was rewritten,
possibly to the same value", and the chain removes the only other kind - so
under the chain every edge is `Applied` and an Applied-priced moat charges the
flat price. Measured by applying the mistake as a mutation: "Charged 191172
against a flat 191172", the moat a complete no-op, arm B collapsed into arm C.
D-107 predicted exactly this in writing.

- The **counter split is DONE 2026-08-15** and was the stated prerequisite.
  `WorldMetrics` carries `plasticity_updates_applied`, `_static` and
  `_refused` beside the total; fixture schema 6 -> 7; the verify script's
  "not a control" list gained `"plasticity_updates_applied":0,`. No checksum
  moved - the counters were already separate inside `PlasticityCounters` and
  only the export collapsed them.
- The moat itself is **not** done. `world.rs` still debits
  `plan.plastic_edges.len() * cost_per_edge_thousandths` with no reference to
  the rule. With `plasticity_updates_applied` now exported, the moat arm can
  be priced against it and the difference reported, which was not possible
  before.

## RESOLVED 2026-08-15 by ADR-0027 - was blocking increment B

**The handoff says "remap the rule ONCE in `compile_with_budget`". That
cannot, on its own, produce what D-107 asked for**, and the gap is worth
settling before code rather than after.

`structmut.rs:633` draws `rule_id` uniformly over `PLASTICITY_RULE_COUNT`
(5) and `genome2.rs:257` reduces a stored id mod 5 at expression time. D-107
A3 is "remove the dead value from the rule id space so every `rule_id` names
a live rule". A compile-time remap sees a 5-way draw and has four live rules
to map it onto, so **some rule gets twice the probability of the others** -
and choosing which is exactly the "authors which learning form evolution
starts adjacent to" objection that D-107 raised to reject option A1.

Getting a uniform four-way draw means the flag reaches
`PLASTICITY_RULE_COUNT` as well - the draw and the mod - not only
`compile_with_budget`. That has a consequence to state rather than discover:
**genome validity becomes config-dependent across the 2x2's arms.** A genome
carrying `rule_id = 4` is meaningful in the flag-off arm and out of range in
the flag-on arm, so the arms no longer share a genome-validity predicate.

**Chosen: (a).** ADR-0027 and D-110. The deciding evidence is the
lifetime-learning review section 6.3, not a preference: the rule that option
(b) doubles is plain Hebbian, which the review rates "strongly supported as a
baseline; unsupported as the sole production rule". An arm biased toward the
least stable rule could produce or destroy its own effect.

One thing the check turned up that the obvious worry gets backwards: genome
validity does **not** become config-dependent, because `normalized` reduces
rather than rejects and every bit pattern still names some rule. What does
change is that the arms no longer share a genotype-to-phenotype map, so a
genome may not be transplanted between arms - which the 2x2 does not do.

Options considered:
- **(a) Flag reaches the draw and the reduction.** Uniform over four live
  rules, which is what "remove the dead value" means. Costs a
  config-dependent genome validity and a bigger blast radius than a compile
  remap.
- **(b) Compile-time remap `r -> (r % 4) + 1`.** Narrow, but doubles rule 1's
  share and is A1's objection wearing a different hat.
- **(c) Keep the 5-way draw and treat id 0 as a fifth live rule that
  duplicates one of the four.** Same objection as (b), stated more honestly.

Implementation shape, for whoever picks this up: the effective rule count
must reach `structmut.rs:633`'s draw and all eight `normalized()` call sites
across `structmut.rs`, `genome2.rs` and `develop.rs`; `compile_with_budget`
maps `r` to live rule `r + 1`; `plasticity::step` still receives a rule and
stays pure. ADR-0027's "Evidence Required To Accept" is the test list.
