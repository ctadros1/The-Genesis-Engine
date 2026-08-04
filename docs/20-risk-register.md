# Risk Register

## Existing Risks

| Risk | Probability | Impact | Mitigation | Trigger/Owner |
|---|---|---|---|---|
| Target scale exceeds CPU/RAM budget | Medium | High | benchmark staged populations; profile before optimization | Phase 0/5 performance owner |
| Floating-point/nondeterminism invalidates replay | Medium | High | fixed tick, named RNG streams, ordering rules, fixture checksums | kernel owner |
| Ecological rules collapse populations or explode | High | Medium | adjustable config, long-run scenarios, explicit capacity event | simulation owner |
| Browser stream overloads clients | Medium | High | viewport culling, deltas, LOD, drop superseded frames | observer owner |
| Saves cannot migrate safely | Medium | High | framed schemas, negative tests, migration registry, rejection policy | persistence owner |
| GPU adds complexity without benefit | Medium | Medium | CPU-first end-to-end comparison | performance owner |
| Existing homelab service is disrupted | Low | High | isolated VM, explicit approval, read-only audit, rollback plan | operations owner |
| Security controls are too weak for sandbox actions | Medium | High | private network, roles, audit trail, rate limits | security owner |
| Rule changes invalidate historical interpretation | High | Medium | config/version ledger and branch-first intervention policy | research owner |
| Project overbuilds before core proof | High | High | phase gates and non-goals enforced by AGENTS.md | project owner |

## Risks Added By The Open-Ended-Evolution Goal (2026-08-04)

| Risk | Probability | Impact | Mitigation | Trigger/Owner |
|---|---|---|---|---|
| **Null result across the culture stack.** Phases 8, 9, and 10 all return no transmission, no traditions, and no cumulative dependency | **High** | Medium | Each phase's ablation design makes the null informative rather than merely disappointing: conditions B, C, D, and S in Phase 9 answer *why* not just *whether*. A measured null is a reportable finding and an acceptable phase outcome per `19-implementation-roadmap.md`. The project's value does not depend on reaching the ceiling | research owner, at each phase gate |
| **Unbounded research scope.** "Open-ended evolution" has no completion condition, so the project can expand indefinitely without a decision point | **High** | High | Every phase has falsifiable acceptance criteria fixed before its campaign runs, and a stated non-goal list. ADR-0012's "can you name the outcome it makes more likely" test rejects additions that are really authored progress. Phase gates remain the stopping mechanism | project owner, at each phase gate |
| **Compute cost exceeds what the homelab can supply.** Generations x seeds x conditions grows multiplicatively; the Phase 2 long run reached 127 generations in 405.7 s, and a cultural ratchet plausibly needs far more | **High** | High | Phase 5 measures throughput and worker scaling before any campaign; seed counts and run lengths are stated per phase and reduced explicitly with the loss of statistical power recorded, never silently. Unresolved: no measurement of what the deployment VM can supply exists yet | performance owner, Phase 5 |
| **Determinism regression from learning.** Learned weights accumulate over 10^5+ ticks, amplifying float reassociation differences that are currently harmless | Medium | High | Rule 7: anything accumulating over a lifetime is fixed point. Per-tick delta is f32 (covered by ADR-0011), accumulation is integer. Bit-identical 10^6-tick trace is a Phase 8 acceptance criterion | kernel owner, Phase 8 |
| **Determinism regression from social mechanisms.** Order-dependence through "A learns from B" is the classic failure and is invisible until a compaction changes storage layout | Medium | High | Rule 4 (read-prior, commit-after), Rule 5 (sorted candidate sets), Rule 1 (canonical pair key). Storage-permutation equality is a required test in every affected phase | kernel owner, Phases 6 and 9 |
| **Determinism regression from variable topology.** Per-node edge summation in storage order is a latent replay bug that only appears after compaction | Medium | High | Rule 6 pins summation to ascending edge innovation-ID order; a compaction test proves independence from layout | kernel owner, Phase 7 |
| **Save-format migration risk.** A subtle difference between the format 1 to format 2 migrated path and the native path corrupts historical worlds silently | Medium | **High** | Byte-identity requirement (C10.5): a migrated format 1 save must produce a world byte-identical to a format 1 load. Format 1 readers and tests stay in the build permanently. Migration is registered and fail-closed, never inferred | persistence owner, Phase 10 |
| **Snapshot and checkpoint budget breaks.** Schema 2 diploid genomes, learned state, object tables, and terrain deltas each add growth to a snapshot already dominated by per-organism genome arrays at ~2.8 KB each | Medium | High | Asynchronous checkpointing is a Phase 5 prerequisite; sparse representations for learned state and terrain deltas; structural caps set from measurement, not guessed; the budget is re-verified in Phases 7, 8, and 10 rather than assumed to carry forward | persistence owner, Phases 7/8/10 |
| **Genome bloat.** Duplication grows genomes until memory and snapshots become unmanageable | Medium | High | Hard caps with deterministic rejection and counting; non-negligible deletion rate; genome size as a reported metric | kernel owner, Phase 7 |
| **Loss of controller batching.** Variable topology invalidates grouping by topology ID and may lose the zero-per-tick-allocation property | Medium | Medium | Measured, not assumed; if severe, group by structural signature in a later performance slice; any loss of the allocation property is recorded rather than absorbed | performance owner, Phase 7 |
| **Plasticity is selected to zero** because the environment is too stationary for learning to pay | **High** | Medium | The environmental-variability sweep is mandatory in Phase 8, not optional. A zero result under variability is a real finding and a strong predictor for Phase 9 | simulation owner, Phase 8 |
| **Condition S is underpowered rather than negative.** The strict no-observational-rule condition may need more compute than exists to produce a meaningful null | Medium | Medium | Report as underpowered rather than negative, and carry that distinction into every downstream summary. Unresolved | research owner, Phase 9 |
| **Analysis leaks into simulation.** A "small hook" from era or tradition detection into the kernel, most likely when a phase is returning nulls | Medium | **High** | Structural: crate dependency direction makes it a compile error; checksum equality with analysis on and off is an acceptance criterion in every analysis phase | kernel owner, ADR-0016 |
| **Detector invents narrative from noise.** Era detection finds segments in worlds where nothing can happen | Medium | Medium | Null control (C12.5) on ablated worlds; synthetic ground-truth validation (C12.4); segments are never named after historical periods | research owner, Phase 12 |
| **Tradition claims made without the genetic control**, so an inherited trait is reported as culture | Medium | **High** | The genotype-matched control is a required report field; a finding without one fails validation. Enforced by the report format, not by reviewer diligence | research owner, Phases 9 and 12 |
| **Realism cost reduces reachable generations** below what the culture phases need | Medium | Medium | ADR-0017's precedence order says the realism increment yields, not the science. Phase 11 reports the throughput delta as a headline number. Each physiology item is independently config-gated | performance owner, Phase 11 |
| **Ambition is overstated in communication** even though the documents are careful, because "evolving civilizations" is a much better story than the honest position | Medium | Medium | `25-emergence-and-epistemic-position.md` is the single authority and separates possible, hoped, and predicted. Reports may not use era or civilization vocabulary. Signal channels are never called language | project owner, ongoing |

## Review

Review this register at every phase transition. Add a dated decision-log
entry when a risk is accepted, reduced, transferred, or newly discovered.

Two entries above are flagged as unresolved rather than mitigated: the
compute-cost ceiling (no deployment-VM measurement exists) and the
statistical power of Phase 9's condition S. Neither is solved by a
mitigation currently in the plan.
