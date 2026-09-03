# ADR-0034: Chemistry As Food - Coupling v2's Concrete Design (Phase 19)

Status: accepted 2026-09-02. The design authority remains
`specifications/unicellular-regime.md` ("Reverse coupling") and
`planning/phase-19-chemistry-as-food.md`; this record pins the concrete
choices those documents leave open, so the implementation cannot pick
them silently. It discharges the revisit conditions ADR-0031 (coupling
v1's deferral of consumption) and ADR-0032 (neutrality under a coupled
field) both wrote down. Where this record and the specification
disagree, the disagreement is a defect in this record.

## What the increment is, and is not

One new flow: an organism may take substrate from the chemistry field in
its own cell and assimilate it as energy, through the ledger. The flow
serves every organism identically and reads nothing but the digestive
capability every body already has (Phase 10's `intake_milli`). Nothing
here names a unicell, a module count, or an outcome; the plan's
Non-Goals are the review clause.

## The feeding pass, exactly

Inside the existing feeding pass, after biomass feeding and in the same
ascending-entity-ID order, for every living organism whose cell holds
substrate:

1. `appetite = intake_capability_milli * consumption_fraction_q16 >> 16
   - biomass_eaten_this_tick`, floored at zero. One gut, one intake rate:
   substrate fills what biomass left of the organism's per-tick
   capability, never adds to it.
2. `room = energy_capacity - energy`, so an organism never exceeds its
   capacity by eating substrate (the invariant `check_invariants` already
   enforces).
3. `gross = min(appetite, room * Q16_ONE / yield, S_MONOMER + S_PRIMORDIAL
   in the cell)`, taken from S_MONOMER first (ADR-0031's "richer input")
   and then S_PRIMORDIAL, exact integers.
4. `gained = gross * consumption_yield_q16 >> 16` credited to the
   organism and to `ledger.assimilated_milli` (it is food); `gross -
   gained` deposited as S_WASTE in the same cell through the existing
   counted `deposit_field` path (so it lands in `deposited_milli`).
5. `chemistry.consumed_milli += gross`.

Two organisms in one cell take in ID order from what remains; the
second may find less. Stated, not incidental.

## Identities

- Organism energy: unchanged in form - substrate energy enters through
  `assimilated_milli` exactly as biomass energy does.
- Field: `produced + deposited - materialized - consumed == chemistry +
  microbial`, where `consumed` is the gross substrate removed and the
  metabolic loss returns through `deposited`. `check_invariants` gains
  the term; the Phase 15 and 16 reductions gain a column.

## Config, hash, format

- `chemistry.consumption_fraction_q16` (0 = off, the default; up to
  Q16_ONE) and `chemistry.consumption_yield_q16` (default 39,322, the
  microbial growth yield, 0.6 - one number for "substrate to energy" in
  both regimes; validated at or below Q16_ONE). Hashed under
  `lifesim-chemistry-consumption` only when the fraction is nonzero
  (D-014); both fields join `FIELD_NAMES`.
- ALIF format 15: the two fields appended to the config body
  (`FORMAT15_CONFIG_BYTES` = 8) and `consumed_milli` (i128) appended to
  `SECTION_CHEMISTRY`'s body when the format is 15 or above - the Phase 12
  precedent of extending a section under a format guard (format 7 added
  a counter word to the schema-2 section). Retained format-14 writer and
  reader, `refuse_format15_state` (a nonzero fraction, a non-default
  yield, or a nonzero `consumed_milli` is refused by every pre-15
  writer), `FORMAT14_TO_CURRENT` resolving both defaults and the term to
  zero - no build that wrote format 14 could consume.
- No new stream, no new event, no new tick phase (benchmark schema 10
  unchanged). Metrics gain `chemistry_consumed_milli`; the field series
  becomes `field-series 3` with `consumed_milli=` appended; the Phase 16
  reduction's regex accepts the trailing column.

## Neutrality, re-proven under the coupled field

The Phase 16 neutrality tests (founder-path twin, no provenance, the
relabelled twin's shared future) are re-run with excretion, remains and
consumption all nonzero. The relabelled twin now shares a future in
which the field feeds it back, which is exactly the case ADR-0032 said
its clause did not cover. A divergence is authored progress.

## Campaign shape (pre-registered before any world runs)

Scratch worlds, the Phase 16 confirmatory's base, two arms on 30 matched
seeds (19001..19030, probed generable first): **v1** (fraction 0 - the
Phase 16 world) and **v2** (fraction Q16_ONE: substrate may fill the
whole digestive capability). Events on (the lifespan endpoint reads
`Materialized` and `Death` records). Primary endpoint C19.3: median
completed lifespan of materialized organisms, v2 minus v1, seed-paired
directed count with a SESOI and bar locked from a four-seed pilot on
disjoint seeds (19901..). C19.4 births and C19.5 module counts reported
beside it with their own stated expectations; the influx cap and the
pairing threshold stay at Phase 16's values so the only difference is
the mouth. The pre-registration states the arithmetic before the run:
basal cost 200 milli against intake capability 1,000 milli at yield
0.6 gives a fed unicell a positive budget of up to ~400 milli per tick
when substrate is present, so lifespan is expected to rise from ~200
ticks toward the trait-derived maturity, and whether it crosses 800 is
the measurement.

## Consequences

- One config pair, one ledger term, one format bump. Fraction zero is
  the Phase 16 world byte for byte (C19.7).
- The unicell ecology becomes a question about physics rather than
  about a missing mouth, and C16.6's null gets re-asked on those terms.
- The scaffold effect on births Phase 16 observed (D-131) is a named
  hypothesis for this campaign's exploratory reading, not a decision.

## As built

(Amended at the end of the phase with every divergence.)
