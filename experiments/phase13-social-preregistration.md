# Phase 13 Confirmatory Campaign Pre-Registration

**Status: committed before the campaign runs.** The companion campaign is
`experiments/phase13-social-confirmatory.campaign` (and the ledger soak,
`experiments/phase13-c1311-ledger-soak.campaign`); this document locks
every decision rule, constant, and claim ceiling first, per the staged
discipline ADR-0029 records (methodology review 6.1) and ADR-0022 A7
(secondary criteria do not rescue a failed primary).

Stage A closed 2026-09-01: the arrival detector
(`lifesim-arrival-detector-v1`), the reachability census, the fidelity
and tradition detectors (`lifesim-fidelity-v1`, `lifesim-tradition-v1`),
and the `lifesim social` / `lifesim social-contrast` commands, each with
Gate E synthetic ground-truth validation committed beside it.

Stage B closed 2026-09-01 with the pilot
(`experiments/phase13-social-pilot.campaign`, seeds 13901..13908,
disjoint from every range below; raw run in the session scratchpad,
findings summarized here). The pilot decided, and this document locks:

- **The tier stands.** 16 worlds of 60,000 ticks at the Phase 12
  population tier with full instrumentation ran in 33.3 minutes on 8
  workers (480 aggregate ticks/second, populations 2,000-7,600). The
  240-world campaign below projects to ~8.3 hours. D-121's terms hold:
  worlds are single-threaded; no parallelism touches any evidence base.
- **The epoch set is E = {10, 15, 20, 25}.** Every pilot world had a
  nonzero naive cohort at all four epochs (hundreds per world-epoch;
  epoch 5 was thinner and founder-heavy and is excluded). Arrival
  fractions were non-degenerate in both arms (world means 15-123 milli),
  so the instrument can see its factor on evolved data (D-120's check).
- **Reachability is not the bottleneck.** Pilot horizon censuses:
  hearers 446-2,854 and speakers 301-1,788 per world, emit-and-in
  conjunctions 60-755, and 3.2-10.9 million priced emissions per A
  world. A C13.1 null here will not be a reachability null, and the
  claim wording below still distinguishes the two cases because the
  confirmatory populations are their own worlds.
- **The pilot's own A-versus-C contrast was null** (8 pairs, mean paired
  difference -5 milli, 95% CI [-14, +1], 2/8 positive): consistent with
  the expected pre-ladder null. The direction below is fixed from
  theory, not tuned to this.

## Arms

Base: the pilot's base verbatim (Phase 12 confirmatory base; Phase 11
relocating patch interval 2,000 / radius 32 / capacity 4x; Phase 11
plasticity block with the chain gate ON - `plasticity.live_rule_zero
true`, citing D-120; social on with observational common-mode). 30 seeds
per arm, 13001..13030, matched across arms.

  A    perception on, signal on                  (the full channel)
  B    signal_enabled false                      (perception only)
  C    perception_enabled false, signal off      (neither)
  D    scramble_delivery true                    (signal, delivery severed)
  S    observational_enabled false               (rule 5 withheld)
  A8k / A16k / A32k                              (A at corruption 8192 / 16384 / 32768)

Every arm differs from A in exactly its named variable(s) (the D-118
lesson). Validation enforces scramble-requires-signal and
observational-requires-plasticity.

## Primary endpoint: C13.1, decided by A-versus-C AND A-versus-D

Per world: `lifesim social-contrast` semantics, fixed here. For each
epoch e in E, the arrival census over the window [2000e, 2000e+2000)
with the naive cut at 2000e (born strictly after; first observed outside
the patch; 50-tick sample resolution; censoring counted never imputed).
The world statistic F is the mean of `arrival_fraction_milli` over
epochs with `naive_total > 0` (ADR-0022 A5); a world with none is
unusable and reported by seed.

Contrast: seed-paired, same epochs, between arms - the one form D-100
calls age-free; no within-arm before/after comparison exists anywhere in
this campaign's analysis. For seed s, D_AC(s) = F_A(s) - F_C(s) and
D_AD(s) = F_A(s) - F_D(s).

**Decision rule (locked):**
- Direction: increase (A arrives faster). Fixed from theory.
- SESOI: **20 milli absolute** on the paired difference - two
  percentage points of naive arrivals, roughly half the pilot's
  between-seed spread (pilot |D_AC| ranged 2-31 milli). The ABSOLUTE
  form is the decision count (`reaching_absolute_directed`): the
  relative form is undefined at a zero control fraction and a count
  that silently drops those pairs is blind to its own factor (D-120;
  recorded on `PairedResult`).
- Bar: **at least 20 of 30 seeds** with D_AC(s) >= +20 milli, AND at
  least 20 of 30 with D_AD(s) >= +20 milli. The exact binomial tail at
  null rate 500 is reported beside each count (20/30 at the null is
  p = 0.049, so the bar is itself a valid one-sided test).
- A-versus-C passing without A-versus-D passing is **not transmission**
  and is reported as a negative result, per the criterion's own wording.
- Analysis seed: 0x0000000000001373. Invocation:
  `lifesim social-contrast --manifest M --treatment A --baseline C
  --epochs 10,15,20,25 --sesoi 20 --analysis-seed 0x1373` and the same
  with `--baseline D`.

**Power, stated before the data:** with the pilot's between-seed spread
(SD ~13 milli of paired differences), the 20-of-30 bar at SESOI 20 has
>= 80% power for true effects of about +28 milli and above - a
two-thirds relative increase over the pilot's ~40 milli base rate. A
true effect smaller than that is expected to report as not-passing with
the 95% CI localizing it; that is a deliberate ceiling, not an
oversight: "measurably faster" is claimed only for an effect of that
size, and anything smaller lands in estimation, not decision.

**Expected outcome, stated in advance:** null. The ladder (D-120's
levers) has not yet produced the prerequisites transmission needs; the
value of the campaign is the controlled why (B, C, D, S and the F-curve
answer which link fails), plus the artifacts every later lever reuses.

## Secondary criteria (none rescues the primary)

- **C13.2**: F-curve across {0, 8192, 16384, 32768} corruption from
  `lifesim-fidelity-v1` world reductions of arm A and the sweep arms
  (exposure radius = the kernel's 8 m perception radius in fp; kinship
  bins as compiled; every bin table reported). Threshold: none borrowed;
  the curve is reported against the persistence>1 line computed from
  the same run's transmission-event counts. Unmatched-exposed counts
  are part of the report (control power is visible).
- **C13.3**: `lifesim-tradition-v1` over arms A (30 seeds) and C (12
  seeds per the criterion) with quadrat 32 cells (the patch's own
  scale), cluster threshold 100 milli, concentration factor 1500,
  minimum neighbourhood 8, kinship tolerance 1/8 in Q32 relatedness.
  The bar is the plan's (15/30 under A, 0/12 under C); every finding
  carries its genotype-matched control or does not exist. Expected
  null.
- **C13.4**: C13.1 and C13.3 re-read for S versus A. Claim ceiling
  (recorded in ADR-0029 from neuroevolution review 5.6/8.10): P-vs-S
  licenses "the rule's availability mattered", never "observational
  learning did it" - the recurrence confound needs state-reset
  interventions that are out of scope.
- **C13.5**: undefined until two identified variants exist (the plan's
  own wording); reported as such if no variant pair emerges.
- **C13.6**: from the fidelity machinery's per-individual records;
  aggregated per world; reported descriptively against the plan's
  wording.
- **C13.7-C13.10**: classifiers not yet built; they will be built with
  Gate E validation and their own pre-registered addendum BEFORE being
  run on this campaign's artifacts, and C13.9/C13.10 need 50 seeds if
  run to decision (a follow-up seed block, not this campaign). C13.10's
  null is expected and stated now.
- **C13.11**: the soak campaign (1 world, 10^6 ticks, arm A config,
  seed 13201, `check_interval` continuous): the ledger stays exact to
  the milli-unit with signalling costs flowing. Pass/fail by the
  kernel's own invariant.
- **C13.12 / C13.13**: carried by the committed suite (fixture replay,
  storage permutation over the social arrays, clean-process verify
  scripts on both hosts, and the mid-phase mutation test).

## Reading notes, locked with the rules

- `never_observed` in a windowed census counts every organism born
  after the cut that no window sample covered - dominated by
  post-window births; it is bookkeeping, not signal.
- `rule5_alleles` counts stored ids in the raw space and is blind under
  the chain's live-space renumbering (stored 4 names rule 5 when the
  chain and the gate are both on); `rule5_expressed_edges` - compiled
  under the world's own budget - is the interpretive number. The pilot
  showed exactly this: zero stored-5 alleles beside up to 42 expressed
  observational edges.
- A null with these censuses attached is a transmission null only if
  hearers and speakers are present at the horizon in that world's own
  census; otherwise it is a reachability null and says so.
- The S arm's recurrence ceiling and the D arm's role (the only
  delivery-severed control; A-vs-C alone is never transmission) survive
  into every downstream summary.
