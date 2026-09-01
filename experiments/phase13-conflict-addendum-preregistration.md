# Phase 13 Conflict Addendum Pre-Registration (C13.7 - C13.10)

**Status: committed before any of these classifiers are read against the
confirmatory campaign's artifacts**, per the main pre-registration's own
deferral (`experiments/phase13-social-preregistration.md`) and ADR-0022
A7. Every parameter below was locked from the Stage B pilot
(seeds 13901..13908, disjoint from all decision seed ranges) and from
Gate E scripted fixtures; nothing here was chosen after seeing a
confirmatory world.

Classifiers: `lifesim-communities-v1` (C13.8-C13.10) and
`lifesim-recognition-v1` (C13.7), read through `lifesim communities` and
`lifesim recognition`. Analysis seed everywhere: 0x0000000000001373.

## Instrument constants (locked)

Communities: association radius = the run's own perception radius (8 m in
fp); window = 40 spatial samples (2,000 ticks at the 50-tick cadence -
one relocation epoch); edge at >= 20 co-occurrences (half the window);
community >= 4 members; chain link = Jaccard >= 500 milli inclusive;
persistent chain >= 3 windows; proximity-matched null = 50 within-window
identity permutations among home-quadrat (32-cell) peers, p95 reported.
Recognition: candidate radius = the perception radius; 200
scale-permutation shuffles; founders excluded by construction (no
phenotype record).

## The pilot facts these rules answer to

- Co-present opportunity denominators are the only readable ones: raw
  membership-pair rates read within >> between purely through proximity
  (recorded with the instrument, commit b02cc35).
- **The co-present between/within factor is ~2x in BOTH pilot arms** -
  A (median ~2.0, range 0.95-3.40) and C, which has no perception at all
  (median ~2.0, range 1.14-3.02). The elevated between-rate is therefore
  community-boundary structure, not cue-directed aggression, and a
  within-arm factor can never evidence recognition. This is the
  criterion's own control clause doing its work at the pilot stage.
- Persistent chains are sporadic and arm-symmetric in the pilot (0-2 per
  world in both arms against a null p95 of 0).

## Decision rules

- **C13.9 primary: the seed-paired A-versus-C contrast of the co-present
  factor.** Per world, factor_milli = copresent_between_rate * 1000 /
  copresent_within_rate (worlds with a zero co-present-within denominator
  are unusable and reported by seed). D_f(s) = factor_A(s) - factor_C(s),
  direction increase, SESOI 500 milli absolute
  (`reaching_absolute_directed`, the main pre-registration's form), bar
  20/30, exact binomial tail at null rate 500 beside it. **Expected
  null** (the pilot medians are equal). The within-arm factor >= 1500
  milli count is reported as descriptive structure only, with the pilot's
  cross-arm symmetry stated beside it; it decides nothing.
- **C13.8**: a world shows persistent communities when
  `persistent_chains > null_p95_chains`. Reported for A and C; the
  criterion's "not explained by spatial proximity alone" is the null
  comparison itself. Count of 30 per arm reported against the sibling
  bar (20/30 under A); **expected marginal-to-null**, and the A-versus-C
  symmetry is reported beside it (pilot: 3/8 A, 4/8 C - community
  persistence in this physics does not appear cue-dependent, which is
  itself a finding about what "community" means here).
- **C13.10**: descriptive, expected null, per the plan's own wording:
  the advantage/disadvantage/even ledger and coalition_targets /
  attacked_targets, per arm. No threshold; any claim stronger than "the
  numbers are reported" needs its own future pre-registration.
- **C13.7**: needs event schema 8 artifacts (tag 26), which the
  confirmatory (launched at schema 7) does not carry. Its campaign:
  **seeds 13101..13130, arms A and C only, the confirmatory base
  verbatim**, run after the confirmatory completes. A world shows
  recognition when |mean_delta_milli| > null_p95_abs_milli. Pass
  requires >= 20/30 under A AND the same statistic NOT clearing its
  null in >= 20/30 under C - C (perception off) is the closest as-built
  realization of the criterion's cue-ablated control, and that reading
  is recorded here as a deviation: no cue-scramble config exists
  (ADR-0029 shipped delivery-scramble only). **Expected null.**
- Unusable worlds (zero denominators, empty censuses) are reported by
  seed and never imputed, everywhere.

## Claim ceilings

A passing C13.9 contrast licenses "aggression is directed by something
the social channel carries", never "kin recognition" or "coalitionary
warfare"; a passing C13.8 licenses "repeated-association structure
exceeds home-range structure", never "social groups" in any richer
sense. C13.7's ceiling: discrimination correlated with cue-visible
phenotype, never mechanism. Every null inherits the main
pre-registration's reachability-versus-transmission wording where it
applies.
