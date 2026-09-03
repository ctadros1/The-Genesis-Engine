# Phase 22 lineages-under-the-other-order pre-registration (C22.1)

**STATUS: LOCKED 2026-09-03**, before any confirmatory world ran. The
pilot is the Phase 21 confirmatory's lineage census - an observation
that campaign was not pre-registered to decide, archived and disjoint
in seeds from this one - and every rule, bar and expectation below was
fixed from it and from the sign-test arithmetic before this lock; the
design was reviewed by a three-lens adversarial workflow (23 findings,
6 confirmed and applied) before it entered the tree. No threshold is
weakened after the data; a different rule is a different phase.

## Question

Is the order in which co-located organisms eat what limits multi-
module lineages in this ecology? Phase 20 found lineages once per six
worlds under the shipped order; Phase 21 found that the shipped order
starves the born cohort - the only cohort a second module appears in -
before it breeds, and that youngest-first lifts its life tenfold.

## The pilot (an observation, D-135)

`runs/phase21-born-cohort-confirmatory-gate-0xca1805044815a9f2`,
`lifesim lineage` over both arms, seeds 21002..21031: shipped order 61
multi-module organisms, 3 parents, 2 second-generation in 1 of 30
worlds; youngest-first 313, 214 parents in 23 worlds, 244 second-
generation in 16 of 30 (largest per-world count 77). Not pre-registered for
this endpoint; it sets the bar and the expectation below and decides
nothing.

## World and arms

Phase 21's confirmatory base (the shipped coupled scratch world with
the composition and birth-site records on), 100,000 ticks, events on;
O1 the shipped intake order (control) against O2 youngest-first
(`physiology.intake_order descending`, the only difference), on 50
matched seeds 22001..22052 without 22010 and 22028 (refused at
preflight; every other seed probed generable on 2026-09-03).

## Primary endpoint (C22.1)

Per world, from `lifesim lineage` (index v1, ADR-0035's definitions):
second-generation multi-module organisms - born, two or more modules,
a parent with two or more, a module count not above that parent's.
Two rules, both fixed here, both required (acceptance is conjunctive):

1. The seed-paired directed count of worlds where O2 has at least one
   and O1 has none, **at least 15 of 50**. Its false-positive rate is
   an unconditional binomial tail that depends on the shipped arm's
   true worlds-with-any rate: 0.002 at 0.16 (Phase 20's rate), ~0 at
   Phase 21's 1 of 30, and the reduction restates it at the upper end
   of the O1 arm's observed rate. Power 0.90 at a treatment rate of
   0.45 and 0.99 at the pilot's 0.53; only ~0.5 at 0.35, so a null at
   this bar reads as "fewer than nine worlds in twenty under the probe
   hold a lineage the shipped order lacks", never as no effect.
2. The exact conditional sign test on the discordant pairs, which
   assumes nothing about either arm's rate: among pairs where exactly
   one arm has a second-generation organism, the one-sided binomial
   probability (p = 0.5) of at least the observed O2-only count is
   **below 0.01**. The pilot gives 16 O2-only against 1 O1-only (p =
   1.3e-4).

The birth-normalized rate (second-generation per 10,000 births) is
reported beside both in both arms and decides nothing.

**Expected, stated in advance**: both rules met.

## The reduction, pinned before the lock

`experiments/results/phase22-lineages-reduction.py` is new code and
is checked the way the censuses were: run on the archived Phase 21
confirmatory (arms O1/O2, seeds 21002..21031) it reproduces the
observation exactly - O1: 1 of 30 worlds with any, 2 pooled; O2: 16 of
30, 244 pooled, largest per-world count 77; 16 directed pairs, 1
reverse - and refuses a manifest with one world's census line removed.

## Reported beside it

C22.2 the lineage's shape per arm (counts per world, the largest
lineage, the parents' fraction against the born cohort's, the
compositions); C22.3 the born cohort's median lifespan per arm from
`lifesim cohort`; the free-lunch gate (no world at the entity cap).

## Hard gates

Both identities exact in-run (`check-interval 10000`); every event log
and manifest present; the reductions refuse anything missing; no
world at the entity cap or reported as such.
