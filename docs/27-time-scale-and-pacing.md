# Time Scale And Pacing

## Why This Document Exists

Nothing in the project has ever stated how simulated time should relate to
wall-clock time. Pacing has been an emergent consequence of `dt` and the
speed multiplier rather than a designed property, which is fine for
campaigns (they run at maximum speed and the question is only how long they
take) and not fine for a flagship world that a person watches for months.

The stated intent: **a world that develops slowly enough to be worth
watching, with speed-up available, and no risk of reaching a ceiling in an
hour.** This document says what that means in numbers.

## The Measured Base Rate

From the Phase 2 long-run record: 200,000 ticks produced 127 ancestry
generations, in 405.7 s of wall clock at maximum headless speed.

    ticks per generation  ~ 1,575
    dt                    = 100 ms, so 1x = 10 ticks/second
    measured headless     = 493 ticks/second, about 49x realtime

Derived pacing, at Phase 2 complexity:

| Speed | Per hour | Per day | Per 30 days | Per year |
|---|---:|---:|---:|---:|
| 1x (10 Hz) | 23 gen | 549 gen | 16,500 gen | 200,000 gen |
| 8x | 183 gen | 4,390 gen | 132,000 gen | - |
| 49x (measured headless) | 1,128 gen | 27,000 gen | 812,000 gen | - |
| 64x (current speed cap) | 1,465 gen | 35,200 gen | 1,055,000 gen | - |

## What This Says About The Concern

**A world cannot reach a ceiling in an hour.** At 1x an hour buys 23
generations, which is not enough for anything. The real risk in this project
runs the other way: a world that produces nothing observable because it has
not been left running long enough.

At 1x, a month of continuous operation reaches roughly 16,500 generations.
That is a substantial evolutionary horizon and it arrives at a pace a person
can actually follow: a generation every two and a half minutes, a visible
population turnover over an afternoon, and structural change over weeks.

**1x is the right default for a flagship world.** Speed-up exists for
catching up, for skipping a barren stretch, and for campaigns.

## Caveats That Will Move These Numbers

Every one of these makes the table optimistic, and none is yet measured:

- **Later phases cost more per tick.** The 49x measurement is Phase 2
  complexity. Variable topology (8), morphology (9), plasticity (10),
  objects (11), perception (12), and physiology (13) each add per-organism
  work, and Phase 9's cost is a distribution rather than a constant. Ticks
  per second will fall, possibly by a large factor.
- **Generation length itself changes.** Phase 13 replaces the age threshold
  with an evolvable life history, so ticks per generation stops being
  roughly constant and becomes something the population determines. A
  population that evolves toward longer life slows its own generational
  clock.
- **The 5,000 entity ceiling was reached** in the run these numbers come
  from, so they describe a capped population.
- **Nothing here is measured on the deployment VM.** These are local M3 Pro
  figures.

The table is therefore a **design orientation, not a budget**. Any pacing
claim in a report cites a benchmark record, not this document.

## Design Position

- `dt` stays at 100 ms and stays a versioned config value. Changing it
  changes the config hash and starts a new replay lineage, so pacing is not
  something to tune casually mid-project.
- **Speed multiplication never changes results.** It changes how fast ticks
  are executed, never what a tick does. Phase 5's acceleration-neutrality
  criterion is the guarantee, and it is the reason speed-up is safe to give
  an operator.
- **Pacing is presentation, not physics.** A world that feels too slow is
  watched at a higher multiplier; it is not fixed by making organisms
  reproduce faster, which would be a behavioral policy change with a new
  lineage.
- The speed cap of 64 is a Phase 3 control clamp, not a physical limit.
  Headless has no cap. Whether the cap should rise for flagship use is an
  open question, not a decision.

## Implications For The Flagship Mode

ADR-0023 splits flagship worlds from campaign worlds. Pacing is one of the
places they differ concretely:

- A flagship world runs at 1x by default and is expected to be left alone.
  The operator's unit of attention is a **week**, not a session.
- Check-in reporting therefore matters more than live watching: over a week
  at 1x a world passes about 3,800 generations, which nobody watches
  continuously. Phase 16's segmentation over the event log is the right tool
  pointed at "what changed since Tuesday".
- A campaign world runs at maximum and is never watched at all.

## Open Questions

| Question | Deadline | Default |
|---|---|---|
| How far does ticks-per-second actually fall through Phases 8 to 13? | Measured per phase | Unknown. Each phase's Benchmark Impact section records it, and this table is restated when it does |
| Should the observer speed cap rise above 64 for flagship use? | Flagship mode adoption | Leave at 64; headless is uncapped and is the right tool for catching up |
| What is the right check-in cadence and report window? | With the first flagship run | One week, adjustable, derived from the event log |
| Does evolvable life history (Phase 13) destabilize the generational clock enough to make pacing unpredictable? | Phase 13 | Unknown, and it is a reportable finding either way |

## Related Documents

- Run modes: ADR-0023, `decisions/0023-flagship-and-campaign-worlds.md`
- Unattended operation prerequisites: `specifications/long-horizon-soak.md`
- Measured rates: `research/performance-notes.md`
- Epistemic position: `25-emergence-and-epistemic-position.md`
