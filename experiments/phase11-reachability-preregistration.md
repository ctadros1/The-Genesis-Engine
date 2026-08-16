# The 2x2 reachability campaign: chain, moat, both, neither

**PRE-REGISTRATION.** Written and committed **before the campaign runs** and
before the moat half of the kernel exists. Sections 2 to 6 are the design;
section 7 is the decision rule and may not change after any number is seen.

**Disclosure.** Everything quantitative below is derived from the
already-published Phase 11 conjunction census
(`experiments/results/phase11-conjunction-census.txt`, committed `7769053`).
That census is prior data, not this campaign's data, and it is what makes the
power calculation in section 6 possible at all. No number from the 2x2 has
been generated.

---

## 1. What this campaign is for

D-107 established that Phase 11's null is about **reachability, not
selection**. A nonzero weight update needs several conditions to coincide on
one expressed edge, each reached by a different point-mutation target, and the
measured joint is 59 of 48,119 compiled plastic edges. The phase's own risk
table predicted "plasticity is selected to zero"; that is not what happened.
Plasticity was never assembled.

Two changes were proposed in response, and D-107 was explicit that they are
different claims that must not be conflated:

- **The chain** (ADR-0027, implemented): remove the dead value from the rule
  id space, so `rule_id` can no longer be spent on nothing. Shortens the
  conjunction by one condition.
- **The moat** (not yet built): charge the per-edge energy cost on the learned
  state *actually moving* rather than on a step being taken. Today
  `world.rs` debits `plan.plastic_edges.len() * cost_per_edge` with no
  reference to the rule, so ~95 percent of the campaign's charge bought rule-0
  no-ops, which is what makes the flagged half of the path deleterious while
  its interior is exactly neutral.

**"X but not Y" requires building the X-alone variant and measuring it.** Hence
a 2x2 rather than a single combined change.

## 2. Arms

Four arms, crossed, 12 seeds each, 48 worlds. Seeds are matched across arms.

| Arm | Chain | Moat | Meaning |
|---|---|---|---|
| **N** | off | off | today's engine; the condition the census describes |
| **C** | on | off | chain only |
| **M** | off | on | moat only |
| **B** | on | on | both |

The chain factor is `plasticity.live_rule_zero`. The moat factor is the field
the moat increment introduces; **this document fixes the design, and the
increment must adopt a field that expresses exactly it** - price on
`StepKind::Applied`, not on `plastic_edges.len()`. If the increment finds it
cannot, that is a change to the design and this pre-registration is superseded
rather than reinterpreted.

Everything else - seeds, ticks, map, mutation rates, patch schedule - is held
at the confirmatory campaign's values, so this campaign's N arm is comparable
to the published census.

## 3. Primary outcome: reachability, not payoff

`lifesim conjunction`'s **`full_conjunction`** count: edge alleles, and
separately expressed plastic edges, satisfying every condition a nonzero
weight update requires.

**Reachability rather than payoff, deliberately.** It counts genotypes, so it
is sound regardless of D-100 and of the second C11.1 confound - both of which
are about a *behavioural* statistic and neither of which touches a census of
which alleles exist. C11.1's own statistic is not used here and no claim about
within-lifetime behavioural shift is made by this campaign.

## 4. Reporting obligations, fixed in advance

- **The per-world distribution, never only the arm total.** In the published
  census 6 of 9 allele-level completions came from one world. An arm total
  hides that completely, and an arm whose total is carried by one world is a
  different result from one whose completions are spread across twelve.
  Every arm reports its twelve per-world counts.
- **The depth histogram in every arm**, at both the allele and expressed
  level. The chain moves alleles *between* depths; a total that moved without
  the histogram moving the way the mechanism predicts is a result that needs
  explaining, not reporting.
- **The plastic-flag frequency against `MARKER_FLAG_NEUTRAL` in every arm.**
  Under the moat the flag is free to drift: pricing on applied steps makes
  carrying a flagged rule-0 edge nearly free, which is the intended change and
  is also exactly how the flag stops being informative. The marker locus is
  the matched drift control (it toggles on the same mutation draw), so the
  reportable quantity is the flag's **excess over the marker**, and it is
  reported for M and B as well as N and C.
- **`plasticity_updates_applied` beside `plasticity_updates_total`** in every
  arm. The total counts edges visited; only the applied count says the
  arithmetic ran. Reporting the total alone is the D-098 error.
- **The compiled rule histogram, and `plasticity_saturations_total`, in every
  arm.** Added 2026-08-16 after implementing the chain, and the reason is a
  measured consequence rather than a precaution. The founder stores
  `rule_id = 0`, the remap sends it to `LIVE_RULE_BASE`, and 93 percent of
  compiled plastic edges still carry the founder's value - so under the chain
  the standing population is about 92 percent **plain Hebbian**, the rule the
  lifetime-learning review singles out for runaway weights. This engine's
  Hebbian is bounded (`LEARN_LIMIT_Q16`), so the clamp is the mitigation and
  the saturation counter measures how hard it is working. Without both
  numbers on the record, an arm that destabilised because nearly everything
  was Hebbian would read as "learning is harmful" rather than as "we made
  nearly everything Hebbian". See ADR-0027's implementation-consequence
  section.

  **The primary endpoint is unaffected**, and this is part of why reachability
  was the right choice: it counts completions and does not ask which rule
  completed. A payoff or stability endpoint would have been confounded by this
  and a reachability one is not.

## 5. Secondary outcomes

Stated now so they cannot be selected later:

- Allele-level `depth[3]` count per world - the direct precursor, and the
  quantity section 6's power calculation is built on.
- `learned_edges_nonzero` and `max_abs_learned_milli` per world: whether
  anything actually learned, as distinct from whether the genotype existed.
- Median final population per arm, as an ecological sanity check. The moat
  removes a metabolic charge, so M and B are expected to carry *more*
  standing biomass; a large shift there is a confound to report, not a result.

## 6. Power, computed before the run

**The naive estimate is wrong in the direction that matters, and getting it
right is what makes this campaign worth running.**

A uniform-draw model says `rule_id != 0` has probability 4/5, so removing the
dead value should multiply completions by 1.25 - far too small to detect with
12 seeds. That model is wrong. The measured marginal is:

```
compiled plastic edges          48,119
  of those, rule != 0            3,147   (6.54%)
  of those, rule 0              44,972  (93.46%)
```

Rule 0 dominates because the **founder** carries it and the mutation rate is
low, so the population sits near its founder value rather than near the
draw's uniform equilibrium. `rule != 0` is therefore the **binding**
condition at 6.54 percent, not a 4-in-5 lottery, and the chain sets it to 1.

Working from the expressed histogram, where every compiled plastic edge
already satisfies the flag and the remaining three conditions are
`rule != 0`, `eta > 0`, and nonzero drive:

```
expressed depth 1  35,192      eta > 0                4,272  (8.88%)
expressed depth 2  11,391      nonzero drive          7,103  (14.76%)
expressed depth 3   1,477
expressed depth 4      59
```

Under the chain, `rule != 0` is universal, so the new completion set is
exactly `{eta > 0} and {drive != 0}`. Under independence that is
`0.0888 x 0.1476 x 48,119 = 631` against the observed 59: **a predicted
~10.7x increase**, from ~2 completions per world to ~21.

The independence assumption is checkable against the same table and roughly
holds: it predicts depth 3 = 279 + 465 + 572 = 1,316 against an observed
1,477, a 12 percent underestimate.

**Consequence for the bar.** A per-world count moving from ~2 to ~21 makes
seed-paired ties rare, which is what the sign test needs. A bar of 10 of 12 is
adequately powered against this effect. Against the 1.25x the naive model
predicts, it would not have been, and the campaign would have been a waste -
which is why this section exists.

**What would make this prediction wrong**, stated now: if `eta > 0` and
nonzero drive are concentrated on the same alleles that already carry
`rule != 0`, the chain adds far less than the independent estimate, and the
observed depth-3 composition will show it. That is a reportable finding about
the structure of the conjunction, not a failed campaign.

**No power claim is made for the moat.** Its effect is on selection, not on
the mutational path, and there is no prior measurement to size it from. The
moat arms are exploratory on this endpoint and are reported as such.

## 7. Decision rule

Fixed before the run. **No threshold here may be weakened after any number is
seen**, and a threshold changed for any reason produces a different
experiment that must be reported as one.

- **Chain:** arm C's `full_conjunction` count exceeds arm N's in **at least 10
  of 12 seed-matched pairs.**
- **Moat:** the same bar for M against N.
- **Both:** the same bar for B against N.

Ties count as failures, which is the conservative direction and is stated
because with a near-zero base rate ties are the failure mode that matters.

Under a null of no effect, 10-or-more of 12 has probability
`(66 + 12 + 1) / 4096 = 0.0193`, so each bar is a one-sided sign test at
approximately alpha = 0.02. Three bars are tested; **no multiplicity
correction is applied and that is a limitation rather than an oversight** -
the three are the pre-specified factorial contrasts of one design, and each is
reported with its own count rather than as a family-wise verdict.

**An interaction is not tested.** With 12 seeds a 2x2 interaction on a count
this sparse has no useful power, and pretending otherwise would be the
"weakened threshold" failure in a different form. B is reported against N on
the same bar as the other two, and any statement about whether the chain and
the moat compose is descriptive only.

## 8. What a null means here

A null on all three bars would say the conjunction stays out of reach even
with one condition removed and the moat drained - which would make the
plateau-with-a-moat reading of Phase 11 stronger, not weaker, and would point
at the remaining conditions (`eta > 0` at 8.88 percent and nonzero drive at
14.76 percent) rather than at selection. It is a result and will be reported
as one.
