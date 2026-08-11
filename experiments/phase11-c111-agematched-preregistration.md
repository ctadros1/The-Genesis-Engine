# C11.1's ruler, straightened: an age-stratified permutation null

**PRE-REGISTRATION.** The design in sections 2 to 5 was written before the
implementation and before any campaign number was recomputed, and is
unchanged from that draft except where marked.

Every `[VERIFY]` item from the draft has been checked against source and each
held: `control_tick = event_tick + relocate / 2` (`boundaries`, plasticity.rs),
`age_ticks` is carried per `.alac` record so no new kernel state is needed,
`permutation_p95_milli` shuffles the label and not the outcome, and
`PERMUTATIONS` is 199 with the p95 an exact order statistic.

**Disclosure, because it bears on how much this document can claim.** Between
writing the design and finalising it I ran the new statistic over the existing
campaign artifacts **once**, to establish that age strata overlap at all in
real data rather than only in the synthetic tests - the design is worthless if
every real world refuses. That run showed per-world `rho` and `shift=false`
for the Avar arm. No threshold, bar, stratum width or decision rule was
changed after it, and none of them may be; the decision rule is inherited
verbatim from the original pre-registration and is restated in section 5. The
run is disclosed here rather than in a footnote because a design document
written by someone who has seen the data is worth less than one written blind,
and a reader is entitled to discount this one accordingly.

---

## 1. The defect, stated as measured

D-100. C11.1 pairs, for each relocation tick `T` and each qualifying organism:

```
event pair    windows [T-500, T]        and [T, T+500]          boundary at T
control pair  windows [T+500, T+1000]   and [T+1000, T+1500]    boundary at T+1000
```

The control boundary sits **1,000 ticks later in the organism's life** than the
event boundary. It is matched on epoch phase and on nothing else.

Measured on a stationary rolling cohort in which the age offset is the only
difference and no event exists anywhere:

| construction | rho (milli) | null p95 | verdict |
|---|---|---|---|
| change decaying with age | **+158** | 30 | **PASSES** |
| change growing with age | −154 | 28 | refused (directed rule) |
| age-dependent, no curvature | **0 exactly** | — | ties == pairs |

Dose–response on the offset: 76 / 158 / 334 / 700 at 500 / 1,000 / 2,000 /
4,000 — about ×2.1 per doubling.

Two consequences worth being precise about:

- **The statistic reads curvature in the age trend**, not the event. The
  zero-curvature row is what identifies the mechanism: a pure linear age trend
  produces exactly nothing.
- **The campaign's 0-of-30 was luck, not protection.** This substrate's age
  trend runs the wrong way. The same pairing over a substrate with the opposite
  curvature turns 0-of-30 into a pass. The statistic is **not identified**, and
  a null from an unidentified statistic is not evidence of absence.

The permutation null cannot rescue it: every draw shuffles labels across the
same age-imbalanced pairs, so the artifact sits in the observed value and not
in the null it is compared against.

## 2. The fix: restrict the randomisation, do not touch the statistic

**The observed statistic is unchanged** — the signed rank correlation between
the event label and the window distance, pooled over all qualifying
observations.

**The null changes.** Labels are shuffled **only within strata defined by the
organism's age at the boundary tick.**

Why this is the right shape. A permutation null must preserve everything about
the data except the association being tested. The current one destroys the
event–distance link *and* the age balance, so it answers a question nobody
asked. Stratifying preserves the age structure, which puts the age artifact
into the null at the same strength it has in the observed value, where it
cancels.

### Why age-at-boundary is the correct stratifying variable

This is the load-bearing step. For an organism of age `a` at the boundary:

- event pair, boundary `T`, age `a`: windows cover ages `[a−500, a]`, `[a, a+500]`
- control pair, boundary `T+1000`, age `a`: windows cover ages `[a−500, a]`, `[a, a+500]`

**Identical age coverage.** So in a world where behaviour is any function of age
whatever, two observations in the same stratum have the same distance
regardless of label, the within-stratum association is exactly zero, and the
stratified null reproduces the pooled artifact exactly. The tripwire passes for
a structural reason rather than by tuning.

Note what this does *not* claim: it does not remove the age effect from the
data. It makes the null contain it.

### Degenerate strata must be dropped from BOTH sides

A stratum holding only one label contributes to the observed statistic but
cannot be shuffled, so leaving it in re-imports exactly the bias being removed.
Observations in single-label strata are **excluded from the observed statistic
and from the null alike**.

Reported per world, because a silent exclusion is how a design becomes a
different design: `strata_total`, `strata_informative`, `observations_kept`,
`observations_dropped`.

### Stratum width

`stratum = age_ticks / W` (integer division), `W` = the window width, 500.
[VERIFY] `age_ticks` is carried per record in `.alac`, so this needs no new
kernel state.

Stated in advance rather than tuned: narrower strata leak less age but
strand more observations in single-label strata; wider strata keep more
observations and leak more age. `W` is the one width with a reason behind it
rather than a preference — it is the resolution at which the windows
themselves are defined, so two observations in one stratum cannot differ by a
whole window.

**Sensitivity, declared now so it cannot be chosen later:** the analysis will
also report the statistic at `W/2` and `2W`. These are **reported, not
decisive**. The verdict reads the `W` row. If the three disagree in sign or
verdict that is a finding about fragility and is reported as one.

## 3. Alternatives weighed, and why they lose

**Age-matched between-organism pairing** — take the control from a different
organism at the same age. Rejected: it abandons the within-individual design,
which is the whole content of "within-lifetime". C11.1 would become a claim
about populations.

**Regress distance on age, correlate the residuals.** Rejected on the
measurement: the artifact is *curvature*, so a linear detrend removes nothing,
and any richer form is an authored functional choice about how behaviour
depends on age — exactly the kind of authoring ADR-0012 exists to refuse.
Stratification is nonparametric and commits to nothing.

**Symmetric bracketing control** at `T−1000` and `T+1000`, averaged. Rejected
on the same measurement: bracketing cancels a linear term, and the
zero-curvature row shows the linear term already contributes nothing. The
symmetric artifact survives symmetric bracketing.

## 4. The tripwire, and its indispensable second half

Permanent tests, both required:

1. **Negative.** The stationary rolling cohort — nothing happens anywhere —
   must not clear the bar, under all three curvature constructions (decaying,
   growing, none), at all four offsets (500/1,000/2,000/4,000). The
   `+158 vs 30` case is the one that currently passes and must stop passing.

2. **Positive.** A synthetic world with the same age structure in which the
   event genuinely does shift behaviour **must still clear the bar.**

Without (2), "scores zero on the fake world" is satisfiable by a statistic that
scores zero on everything, which is this project's trap 5 wearing a fix's
clothing. The positive control is what separates a straightened ruler from a
broken one. Its effect size is fixed **before** running: the planted shift must
be detectable at the smallest magnitude the old pairing could detect, so the
new ruler is not quietly less sensitive than the bent one.

## 5. What is decided in advance

- Decision rule **unchanged and not weakened**: signed rho must strictly exceed
  the p95 of |rho| under the stratified null; treatment ≥ 20 of 30 **and**
  control < 20 of 30. Two-sided count reported, decides nothing.
- `PERMUTATIONS` stays 199. `analysis_seed` stays `0x9e3779b97f4a7c15`.
- Policy version `lifesim-plasticity-analysis-v2` under Determinism Rule 9.
  v1 stays in the build and its report stays reproducible. [VERIFY] v1 is
  referenced by the committed campaign reports and must keep working.
- **The old result stays on the record**, labelled as produced with the
  unstratified null. It is not deleted and not restated.
- Seed count: this re-analysis runs over the **existing** 120 snapshots and
  `.alac` series, so it introduces no new seeds and no new worlds. It is a
  re-analysis of recorded data under a corrected statistic, and must be
  labelled as one everywhere — not as a new campaign.

## 6. Measured on implementation

The tripwire numbers, from `plasticity.rs`'s own tests. These are test
fixtures, not campaign results.

| construction | v1 rho / null | v2 rho / null |
|---|---|---|
| stationary cohort, change decays with age | **+158 / 30, PASSED** | **0 / 6, refused** |
| stationary cohort, change grows with age | −154 / 28 | 0 |
| stationary cohort, no curvature | 0 | 0 |
| dose–response at gaps 500/1k/2k/4k | 76 / 158 / 334 / 700 | **0 / 0 / 0 / 0** |
| n-scaling at 240/480/1,920/7,680 pairs | 163, 160, 158, 139 | **0 at every size** |
| **planted within-lifetime response** | passes | **+1000 / 140, passes** |

The last row is the one that stops this being a fix that breaks the
instrument. A statistic that scored zero on everything would satisfy every
other row here.

**One consequence that is a real cost, not a detail.** A birth-synchronised
population - everything alive from tick 1 - has no age stratum holding both
labels, because an organism's own two observations are always `relocate / 2`
apart in age. Such a world is now refused as `NoInformativeStrata` rather
than scored. That is the honest answer (C11.1 is unanswerable there, not
answered in the negative), but it means the statistic requires a demographic
precondition the old one did not. Real worlds satisfy it comfortably - the
campaign's Avar seed `0x1771` recorded 88,439 births against a founder cohort
of a few hundred, and no campaign world refused - and six synthetic tests had
to gain staggered births to remain evaluable.

## 7. What this does not fix

The instrument's ceiling is untouched. Three of seven action columns vary,
`turn_right` is ~94% of locomotion, `eat` and `mate` are saturated at organism
age in 100.0000% of 1,175,285 records, `rest` and `attack` are empty (D-101).
A straight ruler still cannot measure what the ruler is too coarse to see, and
a null under the corrected statistic inherits that limit. Say so in the report.

Nor does it touch reachability: the five-condition conjunction is queue item
[2] and is the reason there is little to detect in the first place.
