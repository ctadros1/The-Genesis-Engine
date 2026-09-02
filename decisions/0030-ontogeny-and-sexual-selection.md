# ADR-0030: Ontogeny and Sexual Selection (Phase 14, `lifesim-physiology-v2`)

Status: accepted 2026-09-02. Design record for Phase 14
(`planning/phase-14-ontogeny-and-sexual-selection.md`). Four commissioned
reviews were consulted before this design was written - the extractions
were run per review by sonnet agents and the constraint lists below cite
the reviews' own section numbers, following ADR-0029's practice.
`docs/26-biological-realism-policy.md` governs throughout.

## Context

Phase 8 delivered the demographic half of physiology (allometry,
thermoregulation, senescence, extrinsic and juvenile hazard); Phase 10
delivered module bodies grown by a bounded deterministic development
program at birth; Phase 13 delivered cue-based perception of the K
nearest conspecifics. Phase 14 is the slice that could not move earlier:
development of the body *over a lifetime*, mate choice conditioned on
*perceived* phenotype, and (optionally) contact-structured disease.

Pairing today (`World::resolve_pairs`) is mutual controller intent +
maturity + energy + cooldown + range + a genome-distance compatibility
threshold, resolved to the **nearest** eligible candidate with a
deterministic `(distance^2, id)` tie-break. There is no choice in it:
proximity decides. The compatibility threshold is a Phase 9 mechanism
and is **not** what this phase replaces - mate choice and compatibility
are separate mechanisms and remain separately gated (genetics §5.1, the
nine-mechanism separation; §5.16).

Phase 13's confirmatory (D-126) matters here in one specific way: the
perception channel is decisively populated at campaign densities
(hearers in the hundreds per world minimum), so a mate-choice mechanism
that reads perceived cues is reachable by construction, and the
speaker-depletion lesson (priced emission selects against emitters)
warns that any new priced behavior needs its cost measured against the
selection pressure it creates.

## The review constraints this design answers to

Neuroevolution: controller I/O and topology are fixed for an organism's
lifetime - "developmental growth" and "lifetime topology mutation" are
first-policy non-goals (§10.3); evolved scalar parameters read by
engine-level rules are the established pattern for organism traits that
are not per-tick decisions (§10.5, §10.7, the plasticity-parameter
precedent); every added network input is a permanent per-tick cost
across the population (§15.1); engine-rule tiers are recommended before
evolved-output tiers for new signal pathways (§7.5).

Genetics: mate choice, pairing, parentage, contribution, assortment and
crossover must remain nine separable mechanisms (§5.1); a minimal
sexual-selection implementation needs observable heritable traits, real
costs, controller access to observations, choice actions, causally tied
outcomes, and exact parentage records, and signals must never be labeled
"fitness" or "quality" (§5.15); controller-mediated choice from
observable cues and physical locality is preferred over any world-level
genetic-distance mechanism (§5.14); allele choice stays symmetric under
parent swap (§5.3); "crossover may appear beneficial because mate choice
differs" becomes a standing confound note for every later inheritance
experiment (§14.4, §17.10).

Social organization: the only legal causal path is genotype ->
perceptible phenotype -> perception -> behavior; no genotype distance,
pedigree, or observer label may reach behavior (§1.1, §4.2); cue
production genes and response genes must be separable and recombining
(§4.4, §11.5); mate selection reads a specified committed snapshot and
its tie-breaks draw from their own named keyed RNG namespace (§15.13,
§15.2); no hard-coded sexes or sex-specific behavior (§17.23); and §11.5
states the falsifiable prediction this phase's primary is built on:
**cue-dependent assortment must collapse under cue permutation**.

Methodology: an ablation is specific, matched on cost and sensory input,
versioned, interpretable, and *checked* - verified via events that the
mechanism was actually inactive (§6.4); negative controls preserve
exposure while destroying information (§6.7); seed lists lock before
outcomes and no "boring" world is ever dropped (§6.2); power is
simulated at world level against the SESOI, never closed-form (§7.12);
fixation-driven outcomes are reported world-by-world with
probability-of-outcome separated from conditional magnitude (§7.7); the
transferable null for pairing structure is opportunity-matched
permutation (§3.3.D, §7.2).

## Decision 1 - Ontogeny is a paid unfolding of the developed body

Development (`develop.rs`) still runs **at birth**, deterministically,
producing the organism's **adult body** and, with it, the module
emission order that provenance already records. Ontogeny does not re-run
development over a lifetime; it *reveals* the developed body in emission
order, one module at a time, each activation paid for through the
ledger.

- New per-organism state (fixed point, Rule 7, in the
  `lifesim-physiology-state-v1` checksum section): `grown_modules: u32`
  (a prefix length of the emission order) and a `growth_paid_milli`
  accumulator toward the next module's cost.
- Config (`PhysiologyConfig`, gated by a new `ontogeny_enabled`,
  policy `lifesim-physiology-v2`): `birth_modules_min` (the grown prefix
  at birth; at least 1 - the origin module), `growth_cost_milli_per_mass_milli`
  (energy per unit of module mass), and `growth_rate_milli_per_s`
  (the maximum ledger flow into growth per second, so growth is a
  metered expense, not a lump sum).
- Derived attributes are computed **from the grown prefix only**. A
  juvenile with two of nine modules grown has the thrust, sensor range,
  carry capacity, storage, intake, and invest capacity of those two
  modules. The plan's juvenile constraints (lower speed, carry, sensor
  range) are consequences of the partial body, not authored multipliers
  - which is exactly the distinction C14.1 draws against Phase 8's
  scalar penalty.
- **The controller is the recorded exception.** It is compiled at birth
  from the adult body's neural budget and does not grow, because the
  neuroevolution review's first-policy non-goals exclude lifetime
  topology change (§10.3) and the genetics review's determinism
  contract wants development total at birth (§12.14). The brain is
  adult-sized in a juvenile body; recorded as a deliberate realism
  deviation with its reason, per the realism policy's shortcut rule.
- Growth is **deterministic**: energy-gated, no draws. No new RNG
  stream is allocated for it. The plan's determinism note ("new streams
  Development (15), Mortality (16)") is stale and recorded so here:
  15 is `TerrainMod` (Phase 12), 16 is `Mortality` (Phase 8, already
  carrying senescence draw 0 and extrinsic draw 1). `develop.rs`'s
  module doc claims a `Morphogenesis` stream exists; it does not, and
  the comment is corrected in this phase rather than an unused stream
  being allocated to make a stale comment true.
- Maturity: pairing eligibility becomes `age >= maturity_ticks` AND
  (ontogeny disabled OR fully grown). Growth stalled by energy shortage
  delays reproduction - a real life-history consequence, not a bug.
  Phase 8's `juvenile_hazard_multiplier_q16` continues to apply before
  maturity; C14.1's juvenile-vs-adult mortality comparison reads the
  *realized* rates, to which ontogeny adds the emergent contribution of
  small bodies (less storage, shorter reach) on top of the authored
  multiplier.
- Perception honesty: the Phase 13 cue that carries visible phenotype
  (cue 7, body scale) and the contest health scale must read the
  **grown** body, not the adult phenotype, wherever ontogeny is
  enabled. A juvenile is perceived as small because it *is* small.
  This is what makes "perceived phenotype" under ontogeny a real
  developmental signal rather than a birth-static label.

## Decision 2 - Mate choice is an evolved preference over perceived cues

The act stays with the controller; the taste becomes genome; the engine
applies the taste to what the chooser can perceive. Three parts:

- **Act**: the existing `intent_mate` controller output, unchanged. No
  new controller inputs or outputs are added (neuroevolution §10.3,
  §15.1 - K candidate-cue input vectors on every controller would be a
  permanent per-tick cost paid by every organism for a decision made at
  pairing frequency).
- **Taste**: a fixed-size heritable preference block - nine signed Q16
  weights, one per perception cue channel - carried in the genome
  alongside (and recombining independently of) the loci that produce
  the phenotype, satisfying the production/response separability
  constraint (social-org §4.4, §11.5). The plasticity parameter block
  is the architectural precedent (neuroevolution §10.5/§10.7). Weights
  are traits; nothing in config or code names them "quality" or
  "fitness" (genetics §5.15).
- **Application**: `resolve_pairs` keeps its eligibility gates
  (intent, maturity, energy, cooldown, range, compatibility) and its
  canonical ascending-ID iteration, and replaces *nearest* with
  *highest preference score*: for each eligible candidate the engine
  computes the same nine cue values `social_tick` would compute for
  that neighbour - from the previous tick's committed state (social-org
  §15.13; Phase 13's Rule 4 discipline) - and scores
  `dot(preference_q16, cues)` in fixed point. Highest score wins;
  ties break by the existing `(distance^2, id)` key, so a
  zero-preference genome reproduces today's proximity pairing exactly.
  Choice is symmetric by construction: both parties must intend, both
  score, and a pair forms greedily in canonical order exactly as today
  - no sexes, no roles (social-org §17.23; genetics §5.3's semantic
  roles stay out of parent order).

**The P-scramble arm** (`mate_choice_scramble`, a config gate): the cue
vectors entering the chooser's scoring are permuted among the
candidates actually under consideration - candidate identities keep
their eligibility, distance, and cost; which cue vector belongs to
which candidate is destroyed. Draws are keyed on the `Perception`
stream (12) - reserved by ADR-0029 section 6 for exactly "a later
preference gene" - subject the chooser, draw index the candidate
ordinal, so the permutation is deterministic per (world, tick,
chooser). A `scrambled_choices_total` counter lands in the metrics and
the census so the arm is *checked*, never merely configured
(methodology §6.4; the Phase 13 D-arm precedent). Permuting among the
real candidate set preserves the cue distribution (matched, §6.7) while
destroying the cue-candidate pairing - the exact information-destroying
form §11.5's falsifiable prediction names.

**Observation for C14.2**: a new event (schema 9) records, per formed
pair, the chooser, the chosen, the candidate count, the chosen
candidate's true cue vector, and the candidate-set true cue sums - so
the world-level assortment statistic (how far the chosen deviates from
the opportunity set on each cue dimension) is computable offline
without re-simulation and without the analysis touching genotype. The
per-world statistic and its opportunity-matched permutation null get
their exact form in the Phase 14 pre-registration, not here; the design
commitment is only that the event carries the opportunity set summary,
because a pairing log without its opportunity denominator is the bias
the reviews warn about twice (social-org §12.1; methodology §7.7).

**C14.3 (costly display)** adds no display trait. Body scale is already
perceivable (cue 7), heritable, and genuinely costly (mass drives
allometric upkeep); if preference drives scale beyond its survival
optimum, that is measurable from existing censuses plus the schema-8
phenotype-at-birth record, and the expected null is stated in the plan.
A dedicated display channel would be authored ornament - declined.

## Decision 3 - Disease is deferred out of this increment

The plan makes disease optional and droppable without ceremony. Neither
commissioned review carries any treatment of disease, pathogens, or
transmission (both extractions: "not covered"), so a disease design
would rest on general epidemiological knowledge without a
project-evidence base, while ontogeny and mate choice both answer to
specific review constraints. Disease is deferred to its own increment
with its own ADR and pre-registration, to be taken up only after C14.1
and C14.2 are measured. What is recorded now, for that future design:
transmission must be contact-structured with opportunity denominators
logged (social-org §12.1), pair-symmetric via `lifesim-pairkey-v1`, and
its load state fixed point under Rule 7.

## Versioning, schema, and compatibility

- Policy version `lifesim-physiology-v2` covers both gates
  (`ontogeny_enabled`, `mate_choice_enabled`, plus
  `mate_choice_scramble`); config fields hash only when their gate is
  on, per the established pattern. `mate_choice_scramble` requires
  `mate_choice_enabled`; `ontogeny_enabled` requires
  `morphology.enabled` (there is no body to grow otherwise) and
  `physiology.enabled` (it is a physiology term).
- Event schema bumps to 9 with the mate-choice observation tag; the
  decoder keeps accepting older schemas (the D-125-era lesson).
- ALIF gains the physiology-v2 per-organism growth state in the
  `lifesim-physiology-state-v1` checksum section; format bump with the
  prior reader retained and a migration registered, per D-065's
  perturb-every-field round-trip discipline.
- All gates off reproduces the Phase 13 fixture exactly - the phase's
  C14.5 requirement and the standing rollback rule. The preference
  block exists in the genome only when mate choice is enabled at world
  creation; enabling it is a config-hash change (a different world), so
  no genome migration question arises inside a running world.
- Benchmarks: schema stays 10 (Phase 13's); the physiology delta is
  recorded per the plan's benchmark section - the honest price in
  ticks per second and generations per unit compute, headline not
  footnote.

## Campaign shape (to be pre-registered before running, per ADR-0022 A7)

Arms A (ontogeny + choice), B (Phase 8 baseline), P-scramble (choice
with permuted cues). Primary C14.2 at 50 seeds pending simulation-based
world-level power against the pre-registered SESOI (methodology §7.12 -
the plan's 50 is a floor assumption, not a locked number). C14.2
decides on the directed-count form over seed-paired contrasts
(A-vs-P-scramble AND A-vs-B, with the A-vs-P-scramble contrast the one
that separates "choice is informed" from "choice happens"), because
fixation-driven outcomes want probability-of-outcome counting, not mean
effects (§7.7). Stage A (Gate E detectors) and Stage B (pilot on
disjoint seeds) precede any confirmatory world, as in Phase 13.

## Consequences

- Juvenile life becomes a real developmental stage with an energy
  budget, and every constraint it imposes is derivable from the body,
  not from a multiplier. C14.1 becomes measurable.
- Pairing becomes selective without a single new controller channel,
  and the selectivity is heritable, recombining, and cue-mediated - the
  only causal path the reviews permit.
- The scramble arm gives Phase 14 the same controlled-why structure
  Phase 13's D arm gave C13.1: if assortment survives scrambling it is
  not cue-driven, and the phase reports a negative result by its own
  pre-committed wording.
- Two stale documentation claims are corrected in passing: the phase
  plan's stream numbers, and `develop.rs`'s Morphogenesis-stream
  comment.
- The compatibility-threshold mechanism is untouched, and the genetics
  review's structured-compatibility recommendation (§5.16, §9.2)
  remains open as a recorded non-adoption for a future phase - this
  phase adds choice on top of the existing gate rather than redesigning
  the gate.
