# Implementation Roadmap

## Order Of Work

Phases 0 through 4 are complete. Their plan documents, acceptance evidence,
fixtures, and benchmark records are preserved unchanged.

| Phase | Subject | Status |
|---:|---|---|
| 0 | Discovery spikes and benchmark harness | done |
| 1 | Deterministic fixed-point kernel and headless CLI | done |
| 2 | Genome, neural controller, paired reproduction | done |
| 3 | Binary protocol, private server, PixiJS observer | done |
| 4 | Versioned persistence and verified restore | done |
| 5 | Headless scale and multi-world experiments | planned |
| 6 | Biomes, climate drift, and world origin modes | planned |
| 7 | Territory, contest, and damage | planned |
| 8 | Evolvable genome: diploid genetics and variable topology | planned |
| 9 | Modular morphology and development | planned |
| 10 | Lifetime learning | planned |
| 11 | Mutable world and artifacts | planned |
| 12 | Social channel, and organized conflict | planned |
| 13 | Physiology, life history, and senescence | planned |
| 14 | Abiogenesis and the unicellular regime | planned |
| 15 | The multicellularity transition | planned |
| 16 | Offline era and tradition detection | planned |

The former Phase 5 (performance optimization) and Phase 6 (advanced
ecosystems) plans are superseded and preserved unmodified under
`planning/superseded/`. Performance work is now a standing discipline
carried by every phase's Benchmark Impact section; profiling, SIMD, and GPU
evaluation remain backlog items requiring their own evidence.

## Why This Order

The ordering is a dependency argument, not a preference. Each claim below is
the reason a phase cannot usefully precede its predecessor.

**5 is first because every later acceptance criterion is a multi-seed
claim.** Criteria of the form "the effect occurs in 20 of 30 seeds under
condition A and 0 of 30 under condition B" are not measurable today: runs
are paced against observer time, there is no independent-world scheduler,
and there is no event log. Building any mechanism before the instrument that
measures it means measuring it later, worse, and with fewer seeds.

**6 before 7 for two reasons.** Biome structure makes territory meaningful:
contested space over a homogeneous map is a weaker question than contested
space over heterogeneous resource. And Phase 7's kin-biased grouping
criteria are far sharper from several spatially separated founder demes than
from one well-mixed pool, which is exactly what origin modes deliver. Phase 6
also finally implements the temperature field that `docs/05` has specified
since Phase 1 and that the thermal-preference gene, inherited-but-inert
since Phase 2, was written for.

**7 before the genome successor, with reduced scope.** Contest is the one
behavioral mechanism implementable inside the frozen topology, because
topology 1 already reserves its channels: health is input 2 reading a
neutral 1.0, threat is input 10, recent damage is input 16, and attack is
output 4, a documented no-op. That lets it validate the Phase 5 harness on a
real question before the expensive genome work, and it makes threat
information real before a signal channel exists.

**What changed here.** The earlier justification also claimed organized
violence "tends to fall out of scarcity plus kin-biased grouping". The
commissioned social-organization review rejects that shortcut outright and
lists roughly eleven further dependencies, most of which are Phase 10 to 12
machinery (ADR-0022 A1). Phase 7 therefore keeps its position but delivers
only the *physics* of damage and contest; kin recognition, directed
inter-group aggression, and coalition formation move to Phase 12 where
recognition, memory, and transmission exist.

**8 before 9** because morphology's developmental encoding is a genome locus
type, and schema 3 extends schema 2's locus machinery, innovation IDs, and
meiosis. Doing morphology first would mean designing the encoding twice.

**9 before 10** is soft rather than hard. Morphology changes which sensory
and motor channels exist and introduces a controller node budget derived
from neural modules, so doing learning afterwards avoids redoing channel
bindings and lets plasticity cost interact with a body cost that already
exists.

**10 before 11** because without within-lifetime change there is nothing to
transmit. A signalling channel in a world where behavior is fixed at birth
can carry information about state, but nothing an organism acquires can be
acquired by another, so the transmission question cannot be posed.

**11 before 12: artifacts precede signalling.** This reverses the earlier
order, and the reversal is triple-sourced (ADR-0022 A2). The earlier
argument was that building artifacts before a transmission channel exists
would produce another inherited trait. That assumed transmission means
signalling, and it does not: **an artifact left behind is a transmission
channel that requires no perception of conspecifics at all.**
`cumulative_culture` section 1.2 lists persistent generic artifacts inside the
*minimum viable* transmission system; `artifacts` section 1.7 puts social
transmission at step 10 of 12, after carrying, reuse, caching, structures,
and stigmergy; `social_organization` section 1.1 puts stigmergic cooperation before
communication.

So Phase 11 delivers objects and the first transmission mechanism
(stigmergy), and Phase 12 asks whether a *second*, faster channel adds
anything on top. That is a sharper question than the original ordering could
have asked, because it has a baseline to beat.

**13 after the culture stack** is the one placement where two good arguments
conflict, and the resolution is in ADR-0017 rather than glossed. Physiology
changes the entire selection landscape, so introducing it late means prior
cross-condition results do not transfer across it. Introducing it early
makes every culture experiment more expensive per organism and reduces the
generations each can reach. The split: genetics realism goes in Phase 8
because the genome is being rewritten anyway and doing it twice is strictly
worse; physiology realism goes here, with the explicit acknowledgement that
the campaigns that matter are re-run under `lifesim-physiology-v1` before
their results become standing findings.

**14 and 15 are last, and this is the most counterintuitive decision here**,
because "from scratch" sounds like it should be first. Narrative order is not
dependency order.

- Nothing depends on them. Phases 6 through 13 all work from existing
  organisms, so if the origin-of-life work never lands, everything else
  still stands.
- Phase 15 depends on Phase 9: the transition materializes one-module
  organisms, which needs the morphology representation to exist.
- They are the least tractable work in the programme and the most likely to
  return null. Placing the riskiest, least-depended-on work last is what
  keeps a null there from consuming the budget of everything else.

**16 last** because it reads what the earlier phases wrote. It needs the
event log (Phase 5), artifacts (Phase 11), and transmission (Phase 12) for
there to be anything to detect. It changes no world, so it carries the least
risk in the plan by construction.

## The Three Origin Modes Are Not Three Phases

`origin.mode` selects how a world begins
(`specifications/world-origin-modes.md`). The modes land at different
depths because they need different machinery, not because they are ordered
against each other:

| Mode | Delivered | Needs |
|---|---|---|
| `random` | Phase 6 (already the current behavior) | Nothing beyond the founder framework |
| `seeded` | Phase 6 | Biomes, archetypes, deme placement |
| `scratch` | Phase 15 | Field regime (14) and morphology (9) |

Every mode is a starting condition. Authoring where the search begins is not
authoring the path it takes, which is what makes all three admissible under
ADR-0012. See ADR-0021.

## What Would Change This Order

- If Phase 10 shows plasticity is selected to zero under every environmental
  condition, Phase 12's prior drops sharply and reordering Phase 11 ahead of
  it becomes worth arguing, accepting that Phase 11 then answers the weaker
  question.
- If Phase 8 or Phase 9 cost proves prohibitive, contest and artifacts are
  both implementable on schema 1 with reserved channels, and the genome and
  morphology work could move later at the cost of a second schema bump.
- If Phase 9's genotype-phenotype discontinuity measurement shows selection
  cannot act on the developmental encoding, the parameterized body-plan
  fallback returns and Phase 15 needs a different unicellular
  representation.
- If compute cost makes 12-seed campaigns infeasible, the seed count is
  reduced with the loss of statistical power stated explicitly in every
  affected criterion, rather than the criteria being quietly weakened.

## Release Shape

The delivered system is a working prototype with a stable deterministic
loop, a live observer, and durable versioned worlds. Phases 5 through 16
turn it into an instrument for a research question. Success at each phase is
a measured result with its control, including a measured null. It is not a
claim of 50,000 organisms, and it is not a claim that any particular
behavior emerged.

## Decision Gates

| Gate | Required Evidence | Decision |
|---|---|---|
| Language/kernel | microbenchmark, save prototype, developer ergonomics | accept/revise Rust baseline |
| Renderer | desktop/mobile spike, culling/bandwidth measurement | accept/revise PixiJS baseline |
| VM | live capacity audit, guest benchmark, backup feasibility | approve target allocation |
| Persistence | save/restore/negative tests | accept snapshot/catalog formats |
| Scale | p50/p95 tick, RSS, bandwidth, correctness suite | claim supported population tier |
| GPU | end-to-end CPU vs GPU test | adopt only if net operational win |
| Experiment throughput | measured ticks/s/world, worker scaling, host contention | claim a supported campaign size |
| Origin modes | fixture preservation under `random`, archetype inertness, biome-matched placement | accept ADR-0021 |
| Genome successor | topology cost distribution, snapshot size, Mendelian and linkage validation | accept ADR-0013 |
| Morphology | development purity, discontinuity measurement, non-viability rate, cost distribution | accept ADR-0019 |
| Learning | per-individual within-lifetime change, drift-controlled selection on learning genes | accept ADR-0014 |
| Transmission | A-versus-D control result and fidelity curve | claim transmission or report a measured null |
| Save format 2 | format 1 migration byte-identity, corruption sweep, restore-from-backup | accept ADR-0015 |
| Realism | Hardy-Weinberg, linkage decay, allometric exponent, lifespan versus extrinsic mortality | accept ADR-0017 |
| Two-regime engine | exact conservation across the boundary, field cost independent of population, transition neutrality | accept ADR-0020 |
| Scaffolding | unscaffolded control run and reported, naming test passed under review | accept ADR-0018 |

## Definition Of Phase Completion

A phase completes when its plan document's deliverables, tests, benchmarks,
documentation updates, and rollback conditions are satisfied. A demo without
repeatable validation is not phase completion.

For Phases 5 onward there is one addition: **a phase whose behavioral
criteria return null results is still complete** if its ablations ran, its
statistics are recorded, and the negative finding is reported. Weakening a
criterion after seeing the data is not completion; it is a different
experiment.

For the scaffolded phases (14 and 15) there is a second addition: a result
is not complete until its **unscaffolded control** has run on the same seeds
and both numbers are reported together. See ADR-0018.
