# Implementation Roadmap

## Order Of Work

Phases 0 through 4 are complete. Their plan documents, acceptance evidence,
fixtures, and benchmark records are preserved unchanged.

1. **Phase 0** (done): validate hardware, rendering options, language/kernel assumptions, and benchmark harness.
2. **Phase 1** (done): deterministic headless minimum simulation with energy, food, movement, death, metrics, and CLI.
3. **Phase 2** (done): genome, neural controller, mutation, sexual reproduction, lineage, clustering, long-run stability.
4. **Phase 3** (done): private live observer with protocol, renderer, selection, charts, bounded controls.
5. **Phase 4** (done): snapshots, checkpoints, catalog, export, analytics, restore verification.
6. **Phase 5**: headless accelerated execution, independent-world scheduler, append-only event log, multi-seed experiment harness, asynchronous checkpointing.
7. **Phase 6**: territory, contest, and damage.
8. **Phase 7**: evolvable genome, diploid genetics and variable topology.
9. **Phase 8**: lifetime learning.
10. **Phase 9**: social channel.
11. **Phase 10**: mutable world and artifacts.
12. **Phase 11**: physiology, development, and life history.
13. **Phase 12**: offline era and tradition detection.

The former Phase 5 (performance optimization) and Phase 6 (advanced
ecosystems) plans are superseded and preserved unmodified under
`planning/superseded/`. Performance work is now a standing discipline
carried by every phase's Benchmark Impact section rather than a phase of its
own; profiling, SIMD, and GPU evaluation remain backlog items requiring
their own evidence.

## Why This Order

The ordering is a dependency argument, not a preference. Each claim below is
the reason a phase cannot usefully precede its predecessor.

**Phase 5 is first because every later acceptance criterion is a multi-seed
claim.** Criteria of the form "the effect occurs in 8 of 12 seeds under
condition A and 0 of 12 under condition B" are not measurable today: runs
are paced against observer time, there is no independent-world scheduler,
and there is no event log. Building any behavioral mechanism before the
instrument that measures it means measuring it later, worse, and with fewer
seeds.

**Phase 6 is second, ahead of the culture stack.** This deviates from the
ordering originally suggested when the goal changed, and the argument is:

- *It needs no schema change.* Topology 1 already reserves the channels.
  Health is input 2 reading a neutral 1.0, threat is input 10, recent damage
  is input 16, and attack is output 4, a documented no-op. Contest is the
  one behavioral goal implementable inside the existing frozen topology,
  which means it can validate the Phase 5 harness on a real behavioral
  question before the expensive genome successor lands.
- *It creates the information that makes a signal worth having.* A signal
  channel evolves into noise unless something worth signalling about
  exists. Threat is spatially structured, time-varying, and
  fitness-relevant, and alarm signalling is the best-attested natural
  signalling system precisely because threat information has high value to
  the receiver. Running Phase 9 in a world with contest runs it in the
  condition most likely to produce a result.
- *It de-risks the programme.* It is the highest-prior-probability positive
  result in the plan, and it does not depend on the culture stack at all.

  The cost is recorded: it burns a lineage break before the genome
  successor, so Phase 7 breaks lineage again. Every phase breaks lineage
  anyway; that is what versioned policies are for.

**Phase 7 before Phase 8** because plasticity genes live in the genome, and
a variable-topology genome is what makes modulatory nodes expressible at
all. Doing plasticity on schema 1 would mean designing the encoding twice.

**Phase 8 before Phase 9** because without within-lifetime change there is
nothing to transmit. A signalling channel in a world where behavior is fixed
at birth can carry information about state, but nothing an organism acquires
can be acquired by another, so the transmission question cannot be posed.

**Phase 9 before Phase 10** because building artifacts before a transmission
channel exists would produce another inherited trait and teach us nothing.
A world where organisms construct things but cannot learn from each other
tests whether construction is genetically encodable, which is a much less
interesting question than whether construction accumulates. Phase 10's
cumulative-dependency criterion depends entirely on transmission existing.

**Phase 11 after the culture stack** is the one placement where two good
arguments conflict, and the resolution is recorded in ADR-0017 rather than
glossed. Physiology changes the entire selection landscape, so introducing
it late means prior cross-condition results do not transfer across it.
Introducing it early makes every culture experiment more expensive per
organism and therefore reduces the generations each can reach. The split:
genetics realism goes in Phase 7 because the genome is being rewritten
anyway and doing it twice is strictly worse; physiology realism goes here,
with the explicit acknowledgement that the campaigns that matter are re-run
under `lifesim-physiology-v1` before their results become standing findings.

**Phase 12 last** because it reads what the earlier phases wrote. It needs
the event log (Phase 5), transmission (Phase 9), and artifacts (Phase 10)
for there to be anything to detect. It changes no world, so it carries the
least risk in the plan by construction.

## What Would Change This Order

- If Phase 8 shows plasticity is selected to zero under every environmental
  condition, Phase 9's prior drops sharply and reordering Phase 10 ahead of
  it becomes worth arguing, accepting that Phase 10 then answers the weaker
  question.
- If Phase 7's cost proves prohibitive, contest and artifacts are both
  implementable on schema 1 with reserved channels, and the genome successor
  could move later at the cost of a second schema bump.
- If compute cost makes 12-seed campaigns infeasible, the seed count is
  reduced with the loss of statistical power stated explicitly in every
  affected criterion, rather than the criteria being quietly weakened.

## Release Shape

The delivered system is a working prototype with a stable deterministic
loop, a live observer, and durable versioned worlds. Phases 5 through 12
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
| Genome successor | topology cost distribution, snapshot size, Mendelian and linkage validation | accept ADR-0013 |
| Learning | per-individual within-lifetime change, drift-controlled selection on learning genes | accept ADR-0014 |
| Transmission | A-versus-D control result and fidelity curve | claim transmission or report a measured null |
| Save format 2 | format 1 migration byte-identity, corruption sweep, restore-from-backup | accept ADR-0015 |
| Realism | Hardy-Weinberg, linkage decay, allometric exponent, lifespan versus extrinsic mortality | accept ADR-0017 |

## Definition Of Phase Completion

A phase completes when its plan document's deliverables, tests, benchmarks,
documentation updates, and rollback conditions are satisfied. A demo without
repeatable validation is not phase completion.

For Phases 5 onward there is one addition: **a phase whose behavioral
criteria return null results is still complete** if its ablations ran, its
statistics are recorded, and the negative finding is reported. Weakening a
criterion after seeing the data is not completion; it is a different
experiment.
