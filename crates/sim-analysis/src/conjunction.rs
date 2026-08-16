//! Phase 11 follow-up: a **descriptive census** of the plasticity
//! conjunction and of the learned state, over snapshots a campaign already
//! produced.
//!
//! # This is a census, not a hypothesis test
//!
//! Nothing here has a bar, a threshold, a null, a permutation, or a verdict,
//! and nothing here may acquire one. `plasticity.rs` decides C11.1 and C11.2
//! against pre-registered thresholds; this module counts what was in the
//! genomes and in the learned state of the same worlds, after the fact, to
//! say *which* of two very different things a null of those criteria was. It
//! is run on already-collected data with the outcome known, so it cannot be
//! a test of anything and every report it prints says so on its first line.
//!
//! # The conjunction
//!
//! `plasticity::step` writes a nonzero learned delta only when several
//! independent genes on the **same edge locus** are all off their founder
//! value at once. Reading `plasticity.rs` `step` and `structmut.rs`
//! `point_mutate`'s `Edge` arm together:
//!
//! | # | condition | mutation target | where it is enforced |
//! |---|---|---|---|
//! | 1 | `EDGE_FLAG_PLASTIC` set | target 1, a toggle | `controller2::compile_with_budget`, which only lists a flagged edge |
//! | 2 | `rule_id != RULE_STATIC` | target 2, a re-draw | `plasticity::step`'s first branch, a complete no-op |
//! | 3 | `eta > 0` | target 3, clamped at 0 from below | every rule multiplies by `eta` |
//! | 4 | a nonzero drive term | target 4 (coefficients) | `hebbian` returns 0 for all-zero coefficients |
//!
//! **Condition 4 is rule-dependent and that is the one correction to the
//! naive reading.** `RULE_OJA` computes `post * (pre - post * w_eff)` and
//! reads no coefficient at all, so an Oja edge satisfies condition 4
//! structurally and needs only three genes. Rules 1, 3 and 4 all route
//! through `hebbian(coefficients, pre, post)`, which is identically zero
//! when every coefficient is zero, so they need all four.
//!
//! **There is a fifth condition for the modulated rules**, and the naive
//! reading misses it in the other direction. Rules 3 and 4 multiply by the
//! modulator activation, and `world::learn_phase` hands `0.0` to any edge
//! whose `modulator` resolved to `NO_MODULATOR` - which happens when
//! `modulator_node` is 0 or names a node whose role is not `Modulatory`. So
//! a modulated edge with no usable modulator is inert whatever else it
//! carries. It is counted separately rather than folded into the four,
//! because the four are what the depth histogram is over and a five-deep
//! histogram would not be comparable to the four-target reading it is
//! checking.
//!
//! `eta > 0` and a nonzero coefficient are necessary but not sufficient even
//! together: the product still has to survive `to_q16`'s rounding, which
//! discards anything under half a Q16 unit. So every count here is an
//! **upper bound** on how many edges could have learned, and the learned
//! state measured beside it is the ground truth.
//!
//! # Why the learned state is counted and not averaged
//!
//! `LearnState::mean_abs_learned_milli` is a mean over every plastic edge
//! alive. A mean cannot distinguish "no edge learned" from "a few hundred
//! edges out of half a million learned and the mean washed them out", and
//! those are opposite findings. D-074 is the same shape: summing faults and
//! saturations into one anomaly count made a bug signal unreadable. So this
//! module reports the **count** of nonzero learned values, the maximum, and
//! quantiles over the nonzero ones only, and it recomputes the mean beside
//! them so a reporting defect in the mean would be visible as a
//! disagreement rather than inferred.

use sim_core::{
    EDGE_FLAG_PLASTIC, ExpressedEdge, Genome2, LearnSaveState, LocusKind, NO_MODULATOR,
    PlasticityBudget, PlasticityGenes, RULE_COUNT, RULE_OJA, RULE_STATIC,
    compile_network_with_budget, rule_is_modulated,
};
use std::fmt::Write as _;

pub const CONJUNCTION_CENSUS_VERSION: &str = "lifesim-conjunction-census-v1";

/// The four conditions a nonzero learned delta needs on one edge, as
/// booleans, so the conjunction and the depth are two views of one value
/// rather than two computations that could disagree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Conjunction {
    /// `EDGE_FLAG_PLASTIC`. Point-mutation target 1.
    pub flagged: bool,
    /// `rule_id != RULE_STATIC`. Point-mutation target 2.
    pub non_static: bool,
    /// `eta > 0`. Point-mutation target 3.
    pub eta_positive: bool,
    /// The rule form has a nonzero source term. Point-mutation target 4 for
    /// every rule except Oja, which reads no coefficients and is therefore
    /// satisfied structurally.
    pub drive: bool,
}

impl Conjunction {
    /// How many of the four this edge satisfies, `0..=4`.
    ///
    /// Destructured with no `..` (D-077): a fifth condition added to this
    /// struct fails to compile here until it is decided whether it counts
    /// toward the depth, rather than silently being left out of it.
    pub fn depth(&self) -> usize {
        let Self {
            flagged,
            non_static,
            eta_positive,
            drive,
        } = *self;
        usize::from(flagged)
            + usize::from(non_static)
            + usize::from(eta_positive)
            + usize::from(drive)
    }

    /// All four. **Necessary, not sufficient**: the resulting delta still has
    /// to reach half a Q16 unit to move the learned state, and a modulated
    /// rule still needs a resolvable modulator.
    pub fn complete(&self) -> bool {
        self.depth() == 4
    }
}

/// Whether a rule reads the coefficients at all.
///
/// The single fact the whole rule-dependence of condition 4 rests on, given
/// its own function so a test can pin it against `plasticity::step`'s match
/// arm rather than restating the arm's structure inline four times.
pub fn rule_reads_coefficients(rule_id: u8) -> bool {
    rule_id != RULE_OJA
}

/// The conjunction on one edge, from the values `plasticity::step` would
/// actually see.
///
/// Takes the four scalars rather than a `PlasticityGenes` or a
/// `PlasticityRule` so the allele census and the expressed census can share
/// one definition; a second copy of this predicate is exactly the shape that
/// would let the two disagree.
pub fn conjunction_of(flagged: bool, rule_id: u8, eta: f32, coefficients: [f32; 4]) -> Conjunction {
    let non_static = rule_id != RULE_STATIC;
    let drive = if rule_reads_coefficients(rule_id) {
        coefficients.iter().any(|value| *value != 0.0)
    } else {
        true
    };
    Conjunction {
        flagged,
        non_static,
        // Strictly positive. `eta` is clamped to `[0, 1]` from below by both
        // the mutation operator and `PlasticityGenes::normalized`, so
        // "nonzero" and "positive" coincide - but the condition the
        // arithmetic imposes is that the product `raw * eta` is nonzero, and
        // writing it as `> 0.0` rather than `!= 0.0` means a future signed
        // eta would fail this rather than passing it by accident.
        eta_positive: eta > 0.0,
        drive,
    }
}

/// Whether a rule's update is gated on a modulator activation **and** the
/// edge names one. Rules 1 and 2 are ungated, so they are trivially fine;
/// rules 3 and 4 are inert without one.
///
/// At the allele level `named` is `modulator_node != 0`, which is the
/// strongest statement a genome alone supports: whether the named node is
/// `Modulatory` is a property of the *expressed* network. At the expressed
/// level it is `modulator != NO_MODULATOR`, which is the resolved answer.
pub fn modulator_satisfied(rule_id: u8, named: bool) -> bool {
    !rule_is_modulated(rule_id) || named
}

/// Per-allele counts over every `Edge` locus on both haplotypes of every
/// genome in a world.
///
/// **Alleles, not expressed edges.** A diploid organism carries two alleles
/// at an edge locus and expression combines them - the flag is the OR, `eta`
/// is the dominance-weighted blend, and `rule_id` is taken from haplotype 0
/// alone (`genome2::blend_plasticity`). So an allele census answers "what is
/// in the gene pool" and the expressed census beside it answers "what ran".
/// Both are reported because a null could live in either.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AlleleConjunctionCensus {
    pub organisms: u64,
    pub edge_alleles: u64,
    /// Condition 1.
    pub flagged: u64,
    /// Condition 2.
    pub non_static: u64,
    /// Condition 3.
    pub eta_positive: u64,
    /// Condition 4, rule-aware: an Oja allele satisfies it with no
    /// coefficients at all.
    pub drive: u64,
    /// Condition 4 read literally, rule-blind: at least one coefficient off
    /// zero. Reported beside `drive` so the Oja exemption's size is visible
    /// rather than buried inside one number.
    pub coefficient_nonzero: u64,
    /// `decay > 0`. Not a condition - decay only pulls a learned value back
    /// toward zero - and counted because it is the fifth of the seven
    /// mutation targets and shows that the targets were being hit.
    pub decay_positive: u64,
    /// `modulator_node != 0`. Target 7.
    pub modulator_named: u64,
    /// All four conditions on one allele. **The decisive number.**
    pub full_conjunction: u64,
    /// All four, plus a named modulator where the rule needs one.
    pub full_conjunction_gated: u64,
    /// How many alleles satisfy 0, 1, 2, 3 or 4 of the conditions.
    pub depth: [u64; 5],
    /// Normalized `rule_id`, `0..RULE_COUNT`.
    pub rules: [u64; RULE_COUNT as usize],
    /// Stored `rule_id` outside the registry, which
    /// `PlasticityGenes::normalized` reduces modulo `RULE_COUNT` on the way
    /// to the phenotype. Expected to be zero: point mutation draws
    /// `% PLASTICITY_RULE_COUNT`.
    pub stored_rule_out_of_registry: u64,
    /// Every locus of every kind, both haplotypes. **Not a conjunction
    /// quantity.** It is the denominator of the waiting-time question:
    /// `structmut::point_mutate` picks a haplotype, then a chromosome, then
    /// one locus uniformly inside it, so one point mutation reaches a given
    /// edge locus with probability `1 / (2 * chromosomes * loci_in_that_
    /// chromosome)`. Measured here rather than assumed from the founder,
    /// because duplication grows the genome and the denominator with it.
    pub loci_total: u64,
    /// Summed `chromosome_count`; the mean is `chromosomes / organisms`.
    pub chromosomes: u64,
}

/// Count every edge allele in a population.
pub fn allele_conjunction_census(genomes: &[Genome2]) -> AlleleConjunctionCensus {
    let mut census = AlleleConjunctionCensus {
        organisms: genomes.len() as u64,
        ..AlleleConjunctionCensus::default()
    };
    for genome in genomes {
        census.chromosomes += genome.chromosome_count() as u64;
        for locus in genome.loci() {
            census.loci_total += 1;
            let LocusKind::Edge {
                flags, plasticity, ..
            } = locus.kind
            else {
                continue;
            };
            if plasticity.rule_id >= RULE_COUNT {
                census.stored_rule_out_of_registry += 1;
            }
            // The phenotype's values, not the stored ones: `normalized` is
            // what expression applies, and a census over stored values would
            // report a rule the organism never ran.
            let PlasticityGenes {
                rule_id,
                eta,
                coefficients,
                decay,
                modulator_node,
            } = plasticity.normalized();
            census.edge_alleles += 1;
            census.rules[usize::from(rule_id)] += 1;
            let named = modulator_node != 0;
            if decay > 0.0 {
                census.decay_positive += 1;
            }
            if named {
                census.modulator_named += 1;
            }
            if coefficients.iter().any(|value| *value != 0.0) {
                census.coefficient_nonzero += 1;
            }
            let conjunction =
                conjunction_of(flags & EDGE_FLAG_PLASTIC != 0, rule_id, eta, coefficients);
            let Conjunction {
                flagged,
                non_static,
                eta_positive,
                drive,
            } = conjunction;
            census.flagged += u64::from(flagged);
            census.non_static += u64::from(non_static);
            census.eta_positive += u64::from(eta_positive);
            census.drive += u64::from(drive);
            census.depth[conjunction.depth()] += 1;
            if conjunction.complete() {
                census.full_conjunction += 1;
                if modulator_satisfied(rule_id, named) {
                    census.full_conjunction_gated += 1;
                }
            }
        }
    }
    census
}

/// Counts over the plastic edges that were actually **compiled and run**.
///
/// Built by re-expressing and re-compiling each genome with the world's own
/// plasticity budget, which is a pure function of the genome and the config
/// (`controller2::compile_with_budget`), so it reproduces the plan the
/// organism was born with. `plastic_edges` must therefore equal the
/// `plastic_edges_total` metric; the caller checks that, and a disagreement
/// means this census is describing a different plan from the one that ran.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ExpressedConjunctionCensus {
    pub organisms: u64,
    /// Every expressed edge, plastic or not, disabled excluded - the
    /// denominator `plastic_edges` is a fraction of.
    pub live_edges: u64,
    /// Edges compiled plastic. Equals the number of learn-phase `step` calls
    /// per tick for this population.
    pub plastic_edges: u64,
    /// Edges flagged plastic that the per-organism budget refused, compiled
    /// as ordinary fixed edges instead.
    pub over_budget: u64,
    /// Of `plastic_edges`, how many carry a rule other than `RULE_STATIC`.
    /// **A plastic edge with rule 0 executes the learn phase as a total
    /// no-op**, so this is what says whether an update count means anything.
    pub non_static: u64,
    pub eta_positive: u64,
    pub drive: u64,
    /// Of `plastic_edges`, how many resolved a real `Modulatory` node.
    pub modulator_resolved: u64,
    /// Of `plastic_edges`, how many need a modulator and did not resolve one,
    /// so their update is multiplied by a hard 0.0 whatever else they carry.
    pub modulated_without_modulator: u64,
    pub full_conjunction: u64,
    pub full_conjunction_gated: u64,
    pub depth: [u64; 5],
    pub rules: [u64; RULE_COUNT as usize],
}

/// Re-express and re-compile every genome, and count the plastic edges the
/// learn phase actually visits.
///
/// A compile failure is returned as an error rather than skipped: a genome
/// that will not compile belongs to an organism that could not have been
/// alive, so it is a decode or restore defect and not a zero.
pub fn expressed_conjunction_census(
    genomes: &[Genome2],
    budget: PlasticityBudget,
) -> Result<ExpressedConjunctionCensus, String> {
    let mut census = ExpressedConjunctionCensus {
        organisms: genomes.len() as u64,
        ..ExpressedConjunctionCensus::default()
    };
    for (index, genome) in genomes.iter().enumerate() {
        let network = genome.express_network();
        census.live_edges += network
            .edges
            .iter()
            .filter(|edge: &&ExpressedEdge| !edge.disabled)
            .count() as u64;
        let compiled = compile_network_with_budget(&network, budget)
            .map_err(|error| format!("organism {index}: compile: {error:?}"))?;
        census.over_budget += u64::from(compiled.plastic_over_cap);
        for edge in &compiled.plastic_edges {
            census.plastic_edges += 1;
            let rule = edge.rule;
            if rule.rule_id >= RULE_COUNT {
                return Err(format!(
                    "organism {index}: compiled plastic edge {} carries rule {} outside the \
                     registry of {RULE_COUNT}, which expression normalizes away - the census is \
                     reading a value the learn phase could not have run",
                    edge.homology_id, rule.rule_id
                ));
            }
            census.rules[usize::from(rule.rule_id)] += 1;
            let resolved = edge.modulator != NO_MODULATOR;
            if resolved {
                census.modulator_resolved += 1;
            }
            if rule_is_modulated(rule.rule_id) && !resolved {
                census.modulated_without_modulator += 1;
            }
            // A compiled plastic edge is flagged by construction: the
            // compiler only lists flagged edges. Passing `true` is therefore
            // a statement about `compile_with_budget`, not an assumption -
            // and it is asserted directly by a test.
            let conjunction = conjunction_of(true, rule.rule_id, rule.eta, rule.coefficients);
            let Conjunction {
                flagged: _,
                non_static,
                eta_positive,
                drive,
            } = conjunction;
            census.non_static += u64::from(non_static);
            census.eta_positive += u64::from(eta_positive);
            census.drive += u64::from(drive);
            census.depth[conjunction.depth()] += 1;
            if conjunction.complete() {
                census.full_conjunction += 1;
                if modulator_satisfied(rule.rule_id, resolved) {
                    census.full_conjunction_gated += 1;
                }
            }
        }
    }
    Ok(census)
}

/// The learned state itself, counted rather than averaged.
///
/// Every field here exists because a mean cannot answer the question. The
/// mean is recomputed too - by the same formula
/// `LearnState::mean_abs_learned_milli` uses - so that a nonzero count
/// beside a zero mean is visible as a reporting property of the mean rather
/// than as a contradiction between two crates.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct LearnedStateCensus {
    pub organisms: u64,
    /// Plastic-edge rows, i.e. `LearnState::total_plastic_edges`.
    pub rows: u64,
    /// **The decisive number for "did anything learn".** Rows whose
    /// `learned_q16` is not exactly zero.
    pub nonzero_learned: u64,
    pub nonzero_trace: u64,
    pub organisms_with_nonzero_learned: u64,
    /// `|learned_q16|` at the maximum, in Q16. One Q16 unit is 1/65536.
    pub max_abs_learned_q16: u32,
    pub max_abs_trace_q16: u32,
    /// Quantiles of `|learned_q16|` **over the nonzero rows only**, in Q16.
    /// Zero when there are none.
    pub p50_nonzero_q16: u32,
    pub p90_nonzero_q16: u32,
    pub p99_nonzero_q16: u32,
    /// Recomputed with `LearnState::mean_abs_learned_milli`'s exact
    /// arithmetic: `sum(|q16|) * 1000 / 65536 / rows`, integer throughout.
    pub mean_abs_learned_milli: u64,
    /// The same sum before the division, so a mean of 0 over a nonzero sum is
    /// legible as truncation rather than as an absence.
    pub sum_abs_learned_q16: u128,
    pub faults: u64,
    /// The learn phase's own disposition counters, carried in the same
    /// section.
    ///
    /// **`plasticity_updates_total` in the metrics is
    /// `PlasticityCounters::total_evaluated`, which is applied + static +
    /// refused.** A "billion updates" headline therefore counts every
    /// rule-0 edge the learn phase visited and returned from without
    /// reading or writing anything (`plasticity::step`'s first branch). The
    /// split is what says how much of it was arithmetic.
    pub updates_applied: u64,
    pub updates_static: u64,
    pub updates_refused: u64,
    pub clamped: u64,
    pub trace_clamped: u64,
    /// Energy this world's population has paid for carrying plastic edges,
    /// in milli-EU, from `LearnSaveState::cost_milli`.
    ///
    /// The empirical answer to whether an intermediate on the path is
    /// neutral or deleterious. `world::learn_phase` charges **every** plastic
    /// edge, including a rule-0 edge that writes nothing, so a flagged edge
    /// with no working rule is not a neutral marker: it is a metabolic bill
    /// with no return. A number here rather than an argument.
    pub cost_milli: i128,
}

/// Count one world's learned state.
pub fn learned_state_census(learn: &LearnSaveState) -> LearnedStateCensus {
    let mut census = LearnedStateCensus {
        organisms: learn.edges.len() as u64,
        ..LearnedStateCensus::default()
    };
    census.faults = learn.faults.iter().map(|value| u64::from(*value)).sum();
    // Destructured with no `..` (D-077): a counter added to
    // `PlasticityCounters` fails to compile here until it is either carried
    // into this census or explicitly declined.
    let sim_core::PlasticityCounters {
        updates_applied,
        updates_static,
        updates_refused,
        faults: counter_faults,
        clamped,
        trace_clamped,
    } = learn.counters;
    census.updates_applied = updates_applied;
    census.updates_static = updates_static;
    census.updates_refused = updates_refused;
    census.clamped = clamped;
    census.trace_clamped = trace_clamped;
    census.cost_milli = learn.cost_milli;
    // The per-organism fault array and the counter are two independent
    // records of the same event; a disagreement would be a desync, so the
    // larger is kept and both are visible in the report through
    // `updates_refused` beside it.
    census.faults = census.faults.max(counter_faults);
    let mut nonzero: Vec<u32> = Vec::new();
    for row in &learn.edges {
        let mut any = false;
        for edge in row {
            census.rows += 1;
            let magnitude = edge.learned_q16.unsigned_abs();
            census.sum_abs_learned_q16 += u128::from(magnitude);
            census.max_abs_learned_q16 = census.max_abs_learned_q16.max(magnitude);
            census.max_abs_trace_q16 = census.max_abs_trace_q16.max(edge.trace_q16.unsigned_abs());
            if edge.learned_q16 != 0 {
                census.nonzero_learned += 1;
                nonzero.push(magnitude);
                any = true;
            }
            if edge.trace_q16 != 0 {
                census.nonzero_trace += 1;
            }
        }
        if any {
            census.organisms_with_nonzero_learned += 1;
        }
    }
    if census.rows > 0 {
        census.mean_abs_learned_milli =
            (census.sum_abs_learned_q16 * 1_000 / 65_536 / u128::from(census.rows)) as u64;
    }
    nonzero.sort_unstable();
    census.p50_nonzero_q16 = quantile(&nonzero, 50);
    census.p90_nonzero_q16 = quantile(&nonzero, 90);
    census.p99_nonzero_q16 = quantile(&nonzero, 99);
    census
}

/// The `percent`-th percentile of a sorted slice by the nearest-rank rule,
/// integer throughout. Zero for an empty slice, which is the honest answer
/// when the subpopulation the quantile is over does not exist.
fn quantile(sorted: &[u32], percent: u64) -> u32 {
    if sorted.is_empty() {
        return 0;
    }
    let length = sorted.len() as u64;
    // Nearest rank: ceil(p/100 * n), 1-based, clamped into the slice.
    let rank = (percent * length).div_ceil(100).max(1);
    sorted[(rank - 1) as usize % sorted.len()]
}

/// One world's census, with the manifest quantities it must be read against.
#[derive(Clone, Debug, PartialEq)]
pub struct WorldConjunction {
    pub condition: String,
    pub seed: u64,
    pub population: u64,
    pub extinct: bool,
    pub alleles: AlleleConjunctionCensus,
    pub expressed: ExpressedConjunctionCensus,
    /// `None` when the world carries no learned-state section at all, which
    /// is exactly the plasticity-disabled arms. Distinguished from a section
    /// full of zeros, because the two mean different things.
    pub learned: Option<LearnedStateCensus>,
    /// `World::metrics().plastic_edges_total`, the number the findings file's
    /// 48,119 came from.
    pub plastic_edges_total: u64,
    pub plasticity_updates_total: u64,
    /// `World::metrics().mean_abs_learned_milli`, the number the findings
    /// file's central claim rests on.
    pub reported_mean_abs_learned_milli: u64,
}

/// One arm's totals, summed over its worlds.
///
/// Sums rather than means of per-world means: this is a census of a
/// population of alleles, and the question "how many alleles in this arm
/// assembled the conjunction" is a count. C11.2's per-organism-then-per-world
/// averaging exists because a criterion must not let one prolific genome
/// outvote thirty others; a census has no criterion to protect.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArmConjunction {
    pub condition: String,
    pub worlds: usize,
    pub extinct: usize,
    pub alleles: AlleleConjunctionCensus,
    pub expressed: ExpressedConjunctionCensus,
    pub learned: LearnedStateCensus,
    pub worlds_with_learned_section: usize,
    pub worlds_with_any_nonzero_learned: usize,
    pub worlds_with_any_full_conjunction: usize,
    pub max_abs_learned_q16: u32,
    pub plastic_edges_total: u64,
    pub plasticity_updates_total: u64,
}

pub fn summarise(condition: &str, worlds: &[WorldConjunction]) -> ArmConjunction {
    let mut arm = ArmConjunction {
        condition: condition.to_owned(),
        worlds: worlds.len(),
        ..ArmConjunction::default()
    };
    for world in worlds {
        if world.extinct {
            arm.extinct += 1;
        }
        add_alleles(&mut arm.alleles, &world.alleles);
        add_expressed(&mut arm.expressed, &world.expressed);
        arm.plastic_edges_total += world.plastic_edges_total;
        arm.plasticity_updates_total += world.plasticity_updates_total;
        if world.alleles.full_conjunction > 0 {
            arm.worlds_with_any_full_conjunction += 1;
        }
        if let Some(learned) = world.learned {
            arm.worlds_with_learned_section += 1;
            add_learned(&mut arm.learned, &learned);
            arm.max_abs_learned_q16 = arm.max_abs_learned_q16.max(learned.max_abs_learned_q16);
            if learned.nonzero_learned > 0 {
                arm.worlds_with_any_nonzero_learned += 1;
            }
        }
    }
    if arm.learned.rows > 0 {
        arm.learned.mean_abs_learned_milli = (arm.learned.sum_abs_learned_q16 * 1_000
            / 65_536
            / u128::from(arm.learned.rows)) as u64;
    }
    arm
}

/// Add one world's allele counts into an accumulator.
///
/// **Destructured with no `..` (D-077).** A field added to
/// `AlleleConjunctionCensus` fails to compile here until it is given an
/// accumulation rule, which is what stops a new count from being reported
/// per world and silently zero in every arm total.
fn add_alleles(total: &mut AlleleConjunctionCensus, world: &AlleleConjunctionCensus) {
    let AlleleConjunctionCensus {
        organisms,
        edge_alleles,
        flagged,
        non_static,
        eta_positive,
        drive,
        coefficient_nonzero,
        decay_positive,
        modulator_named,
        full_conjunction,
        full_conjunction_gated,
        depth,
        rules,
        stored_rule_out_of_registry,
        loci_total,
        chromosomes,
    } = *world;
    total.organisms += organisms;
    total.edge_alleles += edge_alleles;
    total.flagged += flagged;
    total.non_static += non_static;
    total.eta_positive += eta_positive;
    total.drive += drive;
    total.coefficient_nonzero += coefficient_nonzero;
    total.decay_positive += decay_positive;
    total.modulator_named += modulator_named;
    total.full_conjunction += full_conjunction;
    total.full_conjunction_gated += full_conjunction_gated;
    for (slot, value) in depth.into_iter().enumerate() {
        total.depth[slot] += value;
    }
    for (slot, value) in rules.into_iter().enumerate() {
        total.rules[slot] += value;
    }
    total.stored_rule_out_of_registry += stored_rule_out_of_registry;
    total.loci_total += loci_total;
    total.chromosomes += chromosomes;
}

/// Destructured with no `..`, for `add_alleles`'s reason.
fn add_expressed(total: &mut ExpressedConjunctionCensus, world: &ExpressedConjunctionCensus) {
    let ExpressedConjunctionCensus {
        organisms,
        live_edges,
        plastic_edges,
        over_budget,
        non_static,
        eta_positive,
        drive,
        modulator_resolved,
        modulated_without_modulator,
        full_conjunction,
        full_conjunction_gated,
        depth,
        rules,
    } = *world;
    total.organisms += organisms;
    total.live_edges += live_edges;
    total.plastic_edges += plastic_edges;
    total.over_budget += over_budget;
    total.non_static += non_static;
    total.eta_positive += eta_positive;
    total.drive += drive;
    total.modulator_resolved += modulator_resolved;
    total.modulated_without_modulator += modulated_without_modulator;
    total.full_conjunction += full_conjunction;
    total.full_conjunction_gated += full_conjunction_gated;
    for (slot, value) in depth.into_iter().enumerate() {
        total.depth[slot] += value;
    }
    for (slot, value) in rules.into_iter().enumerate() {
        total.rules[slot] += value;
    }
}

/// Destructured with no `..`, for `add_alleles`'s reason. The quantile fields
/// are deliberately **not** summed - a quantile of a union is not the sum of
/// its quantiles - and are left at zero in an arm total, where the per-world
/// lines carry them.
fn add_learned(total: &mut LearnedStateCensus, world: &LearnedStateCensus) {
    let LearnedStateCensus {
        organisms,
        rows,
        nonzero_learned,
        nonzero_trace,
        organisms_with_nonzero_learned,
        max_abs_learned_q16,
        max_abs_trace_q16,
        p50_nonzero_q16: _,
        p90_nonzero_q16: _,
        p99_nonzero_q16: _,
        mean_abs_learned_milli: _,
        sum_abs_learned_q16,
        faults,
        updates_applied,
        updates_static,
        updates_refused,
        clamped,
        trace_clamped,
        cost_milli,
    } = *world;
    total.organisms += organisms;
    total.rows += rows;
    total.nonzero_learned += nonzero_learned;
    total.nonzero_trace += nonzero_trace;
    total.organisms_with_nonzero_learned += organisms_with_nonzero_learned;
    total.max_abs_learned_q16 = total.max_abs_learned_q16.max(max_abs_learned_q16);
    total.max_abs_trace_q16 = total.max_abs_trace_q16.max(max_abs_trace_q16);
    total.sum_abs_learned_q16 += sum_abs_learned_q16;
    total.faults += faults;
    total.updates_applied += updates_applied;
    total.updates_static += updates_static;
    total.updates_refused += updates_refused;
    total.clamped += clamped;
    total.trace_clamped += trace_clamped;
    total.cost_milli += cost_milli;
}

pub fn render(
    campaign_id: &str,
    per_world: &[(String, Vec<WorldConjunction>)],
    arms: &[ArmConjunction],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "conjunction-census 1 campaign {campaign_id}");
    let _ = writeln!(out, "analysis-version {CONJUNCTION_CENSUS_VERSION}");
    // Printed on every report, not only in the results file, because a table
    // of counts detached from this line reads exactly like a test result.
    let _ = writeln!(
        out,
        "kind DESCRIPTIVE CENSUS of already-collected snapshots. No threshold, no null, no \
         verdict. Not a hypothesis test and must not be reported as one."
    );
    let _ = writeln!(
        out,
        "conditions 1=EDGE_FLAG_PLASTIC 2=rule_id!=0 3=eta>0 4=nonzero-drive \
         (RULE_OJA reads no coefficients and satisfies 4 structurally); \
         gated=+resolvable modulator where rules 3,4 need one"
    );
    for (condition, worlds) in per_world {
        for world in worlds {
            let _ = writeln!(
                out,
                "world condition={condition} seed={:#018x} population={} extinct={} \
                 edge_alleles={} flagged={} non_static={} eta_positive={} drive={} \
                 coefficient_nonzero={} decay_positive={} modulator_named={} \
                 full_conjunction={} full_conjunction_gated={} \
                 depth0={} depth1={} depth2={} depth3={} depth4={} \
                 rule0={} rule1={} rule2={} rule3={} rule4={} out_of_registry={} \
                 organisms={} loci={} chromosomes={}",
                world.seed,
                world.population,
                world.extinct,
                world.alleles.edge_alleles,
                world.alleles.flagged,
                world.alleles.non_static,
                world.alleles.eta_positive,
                world.alleles.drive,
                world.alleles.coefficient_nonzero,
                world.alleles.decay_positive,
                world.alleles.modulator_named,
                world.alleles.full_conjunction,
                world.alleles.full_conjunction_gated,
                world.alleles.depth[0],
                world.alleles.depth[1],
                world.alleles.depth[2],
                world.alleles.depth[3],
                world.alleles.depth[4],
                world.alleles.rules[0],
                world.alleles.rules[1],
                world.alleles.rules[2],
                world.alleles.rules[3],
                world.alleles.rules[4],
                world.alleles.stored_rule_out_of_registry,
                world.alleles.organisms,
                world.alleles.loci_total,
                world.alleles.chromosomes,
            );
            let _ = writeln!(
                out,
                "expressed condition={condition} seed={:#018x} live_edges={} plastic_edges={} \
                 metric_plastic_edges={} over_budget={} non_static={} eta_positive={} drive={} \
                 modulator_resolved={} modulated_without_modulator={} full_conjunction={} \
                 full_conjunction_gated={} depth0={} depth1={} depth2={} depth3={} depth4={} \
                 rule0={} rule1={} rule2={} rule3={} rule4={}",
                world.seed,
                world.expressed.live_edges,
                world.expressed.plastic_edges,
                world.plastic_edges_total,
                world.expressed.over_budget,
                world.expressed.non_static,
                world.expressed.eta_positive,
                world.expressed.drive,
                world.expressed.modulator_resolved,
                world.expressed.modulated_without_modulator,
                world.expressed.full_conjunction,
                world.expressed.full_conjunction_gated,
                world.expressed.depth[0],
                world.expressed.depth[1],
                world.expressed.depth[2],
                world.expressed.depth[3],
                world.expressed.depth[4],
                world.expressed.rules[0],
                world.expressed.rules[1],
                world.expressed.rules[2],
                world.expressed.rules[3],
                world.expressed.rules[4],
            );
            match world.learned {
                Some(learned) => {
                    let _ = writeln!(
                        out,
                        "learned condition={condition} seed={:#018x} section=present rows={} \
                         nonzero_learned={} nonzero_trace={} organisms_with_nonzero={} \
                         max_abs_q16={} max_abs_trace_q16={} p50_nonzero_q16={} \
                         p90_nonzero_q16={} p99_nonzero_q16={} sum_abs_q16={} \
                         recomputed_mean_milli={} reported_mean_milli={} faults={} \
                         metric_updates={} applied={} static={} refused={} clamped={} \
                         trace_clamped={} cost_milli={}",
                        world.seed,
                        learned.rows,
                        learned.nonzero_learned,
                        learned.nonzero_trace,
                        learned.organisms_with_nonzero_learned,
                        learned.max_abs_learned_q16,
                        learned.max_abs_trace_q16,
                        learned.p50_nonzero_q16,
                        learned.p90_nonzero_q16,
                        learned.p99_nonzero_q16,
                        learned.sum_abs_learned_q16,
                        learned.mean_abs_learned_milli,
                        world.reported_mean_abs_learned_milli,
                        learned.faults,
                        world.plasticity_updates_total,
                        learned.updates_applied,
                        learned.updates_static,
                        learned.updates_refused,
                        learned.clamped,
                        learned.trace_clamped,
                        learned.cost_milli,
                    );
                }
                None => {
                    let _ = writeln!(
                        out,
                        "learned condition={condition} seed={:#018x} section=absent",
                        world.seed
                    );
                }
            }
        }
    }
    for arm in arms {
        let _ = writeln!(
            out,
            "arm {} worlds={} extinct={} edge_alleles={} flagged={} non_static={} \
             eta_positive={} drive={} coefficient_nonzero={} decay_positive={} \
             modulator_named={} full_conjunction={} full_conjunction_gated={} \
             depth0={} depth1={} depth2={} depth3={} depth4={} \
             rule0={} rule1={} rule2={} rule3={} rule4={} out_of_registry={} \
             organisms={} loci={} chromosomes={}",
            arm.condition,
            arm.worlds,
            arm.extinct,
            arm.alleles.edge_alleles,
            arm.alleles.flagged,
            arm.alleles.non_static,
            arm.alleles.eta_positive,
            arm.alleles.drive,
            arm.alleles.coefficient_nonzero,
            arm.alleles.decay_positive,
            arm.alleles.modulator_named,
            arm.alleles.full_conjunction,
            arm.alleles.full_conjunction_gated,
            arm.alleles.depth[0],
            arm.alleles.depth[1],
            arm.alleles.depth[2],
            arm.alleles.depth[3],
            arm.alleles.depth[4],
            arm.alleles.rules[0],
            arm.alleles.rules[1],
            arm.alleles.rules[2],
            arm.alleles.rules[3],
            arm.alleles.rules[4],
            arm.alleles.stored_rule_out_of_registry,
            arm.alleles.organisms,
            arm.alleles.loci_total,
            arm.alleles.chromosomes,
        );
        let _ = writeln!(
            out,
            "arm-expressed {} live_edges={} plastic_edges={} metric_plastic_edges={} \
             over_budget={} non_static={} eta_positive={} drive={} modulator_resolved={} \
             modulated_without_modulator={} full_conjunction={} full_conjunction_gated={} \
             depth0={} depth1={} depth2={} depth3={} depth4={} \
             rule0={} rule1={} rule2={} rule3={} rule4={}",
            arm.condition,
            arm.expressed.live_edges,
            arm.expressed.plastic_edges,
            arm.plastic_edges_total,
            arm.expressed.over_budget,
            arm.expressed.non_static,
            arm.expressed.eta_positive,
            arm.expressed.drive,
            arm.expressed.modulator_resolved,
            arm.expressed.modulated_without_modulator,
            arm.expressed.full_conjunction,
            arm.expressed.full_conjunction_gated,
            arm.expressed.depth[0],
            arm.expressed.depth[1],
            arm.expressed.depth[2],
            arm.expressed.depth[3],
            arm.expressed.depth[4],
            arm.expressed.rules[0],
            arm.expressed.rules[1],
            arm.expressed.rules[2],
            arm.expressed.rules[3],
            arm.expressed.rules[4],
        );
        let _ = writeln!(
            out,
            "arm-learned {} worlds_with_section={} rows={} nonzero_learned={} \
             nonzero_trace={} organisms_with_nonzero={} worlds_with_any_nonzero={} \
             worlds_with_any_full_conjunction={} max_abs_q16={} max_abs_trace_q16={} \
             sum_abs_q16={} recomputed_mean_milli={} faults={} metric_updates={} \
             applied={} static={} refused={} clamped={} trace_clamped={} cost_milli={}",
            arm.condition,
            arm.worlds_with_learned_section,
            arm.learned.rows,
            arm.learned.nonzero_learned,
            arm.learned.nonzero_trace,
            arm.learned.organisms_with_nonzero_learned,
            arm.worlds_with_any_nonzero_learned,
            arm.worlds_with_any_full_conjunction,
            arm.learned.max_abs_learned_q16,
            arm.learned.max_abs_trace_q16,
            arm.learned.sum_abs_learned_q16,
            arm.learned.mean_abs_learned_milli,
            arm.learned.faults,
            arm.plasticity_updates_total,
            arm.learned.updates_applied,
            arm.learned.updates_static,
            arm.learned.updates_refused,
            arm.learned.clamped,
            arm.learned.trace_clamped,
            arm.learned.cost_milli,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{
        Haplotype, LearnedEdgeSave, Locus, NodeRole, PlasticityCounters, RULE_ELIGIBILITY_TRACE,
        RULE_HEBBIAN, RULE_MODULATED_HEBBIAN,
    };

    fn genes(
        rule_id: u8,
        eta: f32,
        coefficients: [f32; 4],
        modulator_node: u32,
    ) -> PlasticityGenes {
        PlasticityGenes {
            rule_id,
            eta,
            coefficients,
            decay: 0.0,
            modulator_node,
        }
    }

    fn edge(homology_id: u32, flags: u8, plasticity: PlasticityGenes) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Edge {
                source: 1,
                target: 2,
                weight: 1.0,
                flags,
                plasticity,
            },
        }
    }

    /// One haplotype's loci mirrored onto both, which is what a founder
    /// genome looks like and makes every allele count exactly twice the
    /// locus count - the property that catches a census reading one
    /// haplotype.
    fn genome(loci: Vec<Locus>) -> Genome2 {
        let build = |loci: &Vec<Locus>| Haplotype {
            chromosomes: vec![loci.clone()],
        };
        Genome2 {
            haplotypes: [build(&loci), build(&loci)],
        }
    }

    /// The founder genotype `PlasticityGenes::inert()` produces: every field
    /// at zero and the flag clear.
    #[test]
    fn an_inert_founder_sits_at_depth_zero_and_assembles_nothing() {
        let founders: Vec<Genome2> = (0..10)
            .map(|_| genome(vec![edge(4_000, 0, PlasticityGenes::inert())]))
            .collect();
        let census = allele_conjunction_census(&founders);
        assert_eq!(census.edge_alleles, 20, "both haplotypes must be walked");
        assert_eq!(census.flagged, 0);
        assert_eq!(census.non_static, 0);
        assert_eq!(census.eta_positive, 0);
        assert_eq!(census.drive, 0);
        assert_eq!(census.full_conjunction, 0);
        assert_eq!(census.depth, [20, 0, 0, 0, 0]);
        assert_eq!(census.rules, [20, 0, 0, 0, 0]);
    }

    /// The decisive assertion: an allele that satisfies three conditions is
    /// **not** counted, and the fourth one alone flips it. Written as a sweep
    /// over which condition is withheld, so a conjunction that ignored any
    /// one of its four terms fails here.
    #[test]
    fn every_one_of_the_four_conditions_is_load_bearing_on_its_own() {
        let complete = genome(vec![edge(
            4_000,
            EDGE_FLAG_PLASTIC,
            genes(RULE_HEBBIAN, 0.5, [0.25, 0.0, 0.0, 0.0], 0),
        )]);
        let census = allele_conjunction_census(std::slice::from_ref(&complete));
        assert_eq!(census.full_conjunction, 2, "the complete case must count");
        assert_eq!(census.depth, [0, 0, 0, 0, 2]);
        // ...and it is gated too, because rule 1 needs no modulator.
        assert_eq!(census.full_conjunction_gated, 2);

        // Each of the four withheld in turn. Every one of these genomes
        // satisfies exactly three conditions, so a census that dropped any
        // single term from the conjunction would count them.
        let withheld: [(&str, u8, PlasticityGenes); 4] = [
            (
                "flag",
                0,
                genes(RULE_HEBBIAN, 0.5, [0.25, 0.0, 0.0, 0.0], 0),
            ),
            (
                "rule",
                EDGE_FLAG_PLASTIC,
                genes(RULE_STATIC, 0.5, [0.25, 0.0, 0.0, 0.0], 0),
            ),
            (
                "eta",
                EDGE_FLAG_PLASTIC,
                genes(RULE_HEBBIAN, 0.0, [0.25, 0.0, 0.0, 0.0], 0),
            ),
            (
                "coefficients",
                EDGE_FLAG_PLASTIC,
                genes(RULE_HEBBIAN, 0.5, [0.0; 4], 0),
            ),
        ];
        for (label, flags, plasticity) in withheld {
            let partial = genome(vec![edge(4_000, flags, plasticity)]);
            let census = allele_conjunction_census(std::slice::from_ref(&partial));
            assert_eq!(
                census.full_conjunction, 0,
                "an allele missing only {label} was counted as complete"
            );
            assert_eq!(
                census.depth,
                [0, 0, 0, 2, 0],
                "an allele missing only {label} must sit at depth 3"
            );
        }
    }

    /// The one correction to the four-target reading. Oja reads no
    /// coefficients - `plasticity.rs` `step` computes it as
    /// `post * (pre - post * w_eff)` - so an Oja edge with every coefficient
    /// at zero is complete on three genes, and the Hebbian control beside it,
    /// identical except for the rule id, is not.
    #[test]
    fn oja_completes_the_conjunction_with_no_coefficients_and_hebbian_does_not() {
        let oja = genome(vec![edge(
            4_000,
            EDGE_FLAG_PLASTIC,
            genes(RULE_OJA, 0.5, [0.0; 4], 0),
        )]);
        let census = allele_conjunction_census(std::slice::from_ref(&oja));
        assert_eq!(census.full_conjunction, 2);
        assert_eq!(census.drive, 2);
        // The literal, rule-blind reading disagrees, and both are reported:
        // no coefficient is off zero anywhere in this genome.
        assert_eq!(census.coefficient_nonzero, 0);

        let hebbian = genome(vec![edge(
            4_000,
            EDGE_FLAG_PLASTIC,
            genes(RULE_HEBBIAN, 0.5, [0.0; 4], 0),
        )]);
        let control = allele_conjunction_census(std::slice::from_ref(&hebbian));
        assert_eq!(
            control.full_conjunction, 0,
            "rule 1 with no coefficients computes 0 and must not count"
        );
        assert_eq!(control.drive, 0);

        // Rules 3 and 4 are the Hebbian form multiplied, so they inherit the
        // coefficient requirement. Asserting them keeps `rule_reads_
        // coefficients` from being special-cased to rule 1 alone.
        for rule_id in [RULE_MODULATED_HEBBIAN, RULE_ELIGIBILITY_TRACE] {
            assert!(rule_reads_coefficients(rule_id), "rule {rule_id}");
            let modulated = genome(vec![edge(
                4_000,
                EDGE_FLAG_PLASTIC,
                genes(rule_id, 0.5, [0.0; 4], 7),
            )]);
            let census = allele_conjunction_census(std::slice::from_ref(&modulated));
            assert_eq!(census.full_conjunction, 0, "rule {rule_id}");
        }
        assert!(!rule_reads_coefficients(RULE_OJA));
    }

    /// A modulated rule with no modulator is multiplied by a hard `0.0` in
    /// `world::learn_phase`, so the four-way conjunction over-counts it and
    /// the gated count is what says so. Both are reported and they must
    /// differ here, or the gated column is decoration.
    #[test]
    fn a_modulated_rule_without_a_modulator_is_complete_but_not_gated() {
        for rule_id in [RULE_MODULATED_HEBBIAN, RULE_ELIGIBILITY_TRACE] {
            let orphaned = genome(vec![edge(
                4_000,
                EDGE_FLAG_PLASTIC,
                genes(rule_id, 0.5, [0.25, 0.0, 0.0, 0.0], 0),
            )]);
            let census = allele_conjunction_census(std::slice::from_ref(&orphaned));
            assert_eq!(census.full_conjunction, 2, "rule {rule_id}");
            assert_eq!(
                census.full_conjunction_gated, 0,
                "rule {rule_id} with modulator_node=0 must not be gated-complete"
            );
            assert_eq!(census.modulator_named, 0);

            // The control: the same edge naming a node. Without this the
            // assertion above would pass on an implementation that always
            // returned zero.
            let gated = genome(vec![edge(
                4_000,
                EDGE_FLAG_PLASTIC,
                genes(rule_id, 0.5, [0.25, 0.0, 0.0, 0.0], 9),
            )]);
            let census = allele_conjunction_census(std::slice::from_ref(&gated));
            assert_eq!(census.full_conjunction_gated, 2, "rule {rule_id}");
            assert_eq!(census.modulator_named, 2);
        }
        // Rules 1 and 2 are ungated, so naming no modulator costs them
        // nothing - the asymmetry `rule_is_modulated` exists to express.
        for rule_id in [RULE_HEBBIAN, RULE_OJA] {
            assert!(modulator_satisfied(rule_id, false), "rule {rule_id}");
        }
        assert!(!modulator_satisfied(RULE_MODULATED_HEBBIAN, false));
        assert!(modulator_satisfied(RULE_MODULATED_HEBBIAN, true));
    }

    /// The depth histogram is what makes a plateau visible, so it has to
    /// place mass at the right rung and not merely sum to the allele count.
    #[test]
    fn the_depth_histogram_places_each_allele_at_its_own_rung() {
        let population = vec![genome(vec![
            // depth 0
            edge(1, 0, PlasticityGenes::inert()),
            // depth 1: flag only
            edge(2, EDGE_FLAG_PLASTIC, PlasticityGenes::inert()),
            // depth 2: flag + rule
            edge(3, EDGE_FLAG_PLASTIC, genes(RULE_HEBBIAN, 0.0, [0.0; 4], 0)),
            // depth 3: flag + rule + eta
            edge(4, EDGE_FLAG_PLASTIC, genes(RULE_HEBBIAN, 0.5, [0.0; 4], 0)),
            // depth 4
            edge(
                5,
                EDGE_FLAG_PLASTIC,
                genes(RULE_HEBBIAN, 0.5, [0.0, 0.0, 0.0, -1.0], 0),
            ),
            // depth 2 reached from the other side: rule + drive, no flag and
            // no eta. A histogram keyed on "how far along the flag-first
            // path" rather than on the count would put this somewhere else.
            edge(6, 0, genes(RULE_HEBBIAN, 0.0, [1.0, 0.0, 0.0, 0.0], 0)),
        ])];
        let census = allele_conjunction_census(&population);
        assert_eq!(census.edge_alleles, 12);
        assert_eq!(census.depth, [2, 2, 4, 2, 2]);
        assert_eq!(
            census.depth.iter().sum::<u64>(),
            census.edge_alleles,
            "every allele must land in exactly one rung"
        );
        assert_eq!(census.full_conjunction, census.depth[4]);
        assert_eq!(census.rules, [4, 8, 0, 0, 0]);
    }

    /// The locus and chromosome counts the waiting-time arithmetic divides
    /// by. Not a conjunction quantity, and asserted because a wrong
    /// denominator would make an expected waiting time wrong by exactly the
    /// factor nobody would notice.
    #[test]
    fn the_locus_denominator_counts_every_kind_on_both_haplotypes() {
        let population: Vec<Genome2> = (0..3)
            .map(|_| {
                genome(vec![
                    node(1, NodeRole::Input),
                    node(2, NodeRole::Output),
                    edge(3, 0, PlasticityGenes::inert()),
                    marker(4),
                ])
            })
            .collect();
        let census = allele_conjunction_census(&population);
        // Four loci per haplotype, two haplotypes, three organisms.
        assert_eq!(census.loci_total, 24);
        assert_eq!(census.edge_alleles, 6, "edges are a subset of the loci");
        assert_eq!(census.chromosomes, 3, "one chromosome each");
        assert_eq!(census.organisms, 3);
        // A mutation reaches one named edge locus with probability
        // 1 / (2 * 1 * 4): the haplotype pick, the chromosome pick, and the
        // uniform index inside it.
        let loci_per_chromosome = census.loci_total / (2 * census.chromosomes);
        assert_eq!(loci_per_chromosome, 4);

        // **A one-chromosome fixture cannot tell `chromosome_count()` from
        // the constant 1**, and the whole point of the field is that it is
        // the second factor in that probability. So the same census is taken
        // over a two-chromosome genome, where a hard-coded 1 is visible.
        let split: Vec<Genome2> = (0..3)
            .map(|_| {
                let build = || Haplotype {
                    chromosomes: vec![
                        vec![node(1, NodeRole::Input), node(2, NodeRole::Output)],
                        vec![edge(3, 0, PlasticityGenes::inert()), marker(4)],
                    ],
                };
                Genome2 {
                    haplotypes: [build(), build()],
                }
            })
            .collect();
        let census = allele_conjunction_census(&split);
        assert_eq!(
            census.loci_total, 24,
            "the same 24 loci, differently packed"
        );
        assert_eq!(census.edge_alleles, 6);
        assert_eq!(census.chromosomes, 6, "two chromosomes per genome");
        // Two loci per chromosome now, so one mutation reaches a named edge
        // locus with probability 1 / (2 * 2 * 2) - the same 1/8 the flat
        // packing gave at 1 / (2 * 1 * 4), which is why the denominator is
        // `loci_total` and not `chromosomes` alone.
        assert_eq!(census.loci_total / (2 * census.chromosomes), 2);
    }

    fn marker(homology_id: u32) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Marker {
                value: 0.0,
                flags: 0,
            },
        }
    }

    /// Stored rule ids outside the registry are reduced by
    /// `PlasticityGenes::normalized` on the way to the phenotype, so the
    /// histogram must show the rule the organism would have run and the
    /// out-of-registry count must show that it was stored differently.
    #[test]
    fn an_out_of_registry_rule_id_is_counted_as_the_rule_expression_would_run() {
        // 7 % 5 == 2, which is Oja - a rule that needs no coefficients, so
        // this also proves the normalization happens before the conjunction
        // rather than after it.
        let stored = genome(vec![edge(
            4_000,
            EDGE_FLAG_PLASTIC,
            genes(7, 0.5, [0.0; 4], 0),
        )]);
        let census = allele_conjunction_census(std::slice::from_ref(&stored));
        assert_eq!(census.stored_rule_out_of_registry, 2);
        assert_eq!(census.rules, [0, 0, 2, 0, 0]);
        assert_eq!(census.full_conjunction, 2);
    }

    fn learn_state(rows: Vec<Vec<i32>>) -> LearnSaveState {
        let organisms = rows.len();
        LearnSaveState {
            edges: rows
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .enumerate()
                        .map(|(slot, learned_q16)| LearnedEdgeSave {
                            edge_homology_id: slot as u32,
                            learned_q16,
                            trace_q16: 0,
                        })
                        .collect()
                })
                .collect(),
            faults: vec![0; organisms],
            cost_remainder: vec![0; organisms],
            counters: PlasticityCounters::default(),
            cost_milli: 4_242,
        }
    }

    /// **The reason this module exists on the learned-state side.** A
    /// population where one edge in ten thousand learned a full unit reports
    /// a mean of 0 milli and a count of 1, and the two must not be confused.
    #[test]
    fn a_single_learned_edge_survives_a_mean_that_rounds_to_zero() {
        let mut rows = vec![vec![0_i32; 100]; 100];
        rows[7][3] = 65_536; // exactly 1.0
        let census = learned_state_census(&learn_state(rows));
        assert_eq!(census.rows, 10_000);
        assert_eq!(census.nonzero_learned, 1, "the count is the whole point");
        assert_eq!(census.organisms_with_nonzero_learned, 1);
        assert_eq!(census.max_abs_learned_q16, 65_536);
        assert_eq!(census.sum_abs_learned_q16, 65_536);
        // 65536 * 1000 / 65536 / 10000 = 0. The mean the findings file quotes
        // reads zero over a population that contains a fully-learned edge.
        assert_eq!(census.mean_abs_learned_milli, 0);
        // Quantiles over the nonzero rows only: with one of them, all three
        // are that one value rather than zero.
        assert_eq!(census.p50_nonzero_q16, 65_536);
        assert_eq!(census.p99_nonzero_q16, 65_536);

        // The control: the same shape with nothing learned. Without it the
        // assertions above could pass on a census that reported constants.
        let empty = learned_state_census(&learn_state(vec![vec![0_i32; 100]; 100]));
        assert_eq!(empty.nonzero_learned, 0);
        assert_eq!(empty.max_abs_learned_q16, 0);
        assert_eq!(empty.p50_nonzero_q16, 0);
        assert_eq!(empty.mean_abs_learned_milli, 0);
        assert_eq!(
            empty.mean_abs_learned_milli, census.mean_abs_learned_milli,
            "the two populations are distinguishable only by the count"
        );
    }

    /// A negative learned value is learning too. A census that summed signed
    /// values would cancel a symmetric population to zero and report the
    /// same thing an inert one does.
    #[test]
    fn opposite_signed_learning_does_not_cancel() {
        let census = learned_state_census(&learn_state(vec![vec![65_536, -65_536]]));
        assert_eq!(census.nonzero_learned, 2);
        assert_eq!(census.sum_abs_learned_q16, 131_072);
        assert_eq!(census.mean_abs_learned_milli, 1_000);
        assert_eq!(census.max_abs_learned_q16, 65_536);
        // The energy an organism paid for carrying the edges is carried
        // through, because a flagged edge that learned nothing still cost
        // something and that is what makes it deleterious rather than
        // neutral.
        assert_eq!(census.cost_milli, 4_242);
    }

    /// Quantiles are over the nonzero rows only, so a population that is 99
    /// percent inert does not report a p90 of zero and hide its own tail.
    #[test]
    fn quantiles_ignore_the_inert_rows() {
        let mut row = vec![0_i32; 990];
        row.extend((1..=10).map(|value| value * 65_536));
        let census = learned_state_census(&learn_state(vec![row]));
        assert_eq!(census.rows, 1_000);
        assert_eq!(census.nonzero_learned, 10);
        // Nearest rank over ten values: p50 is the 5th, p90 the 9th, p99 the
        // 10th.
        assert_eq!(census.p50_nonzero_q16, 5 * 65_536);
        assert_eq!(census.p90_nonzero_q16, 9 * 65_536);
        assert_eq!(census.p99_nonzero_q16, 10 * 65_536);
        assert_eq!(census.max_abs_learned_q16, 10 * 65_536);
        // A quantile computed over all 1,000 rows would be 0 at p50 and p90.
        assert_ne!(census.p90_nonzero_q16, 0);
    }

    /// The expressed census must reproduce the plan the learn phase ran, and
    /// the budget is part of that plan: with the section disabled no edge is
    /// plastic however it is flagged.
    #[test]
    fn the_expressed_census_follows_the_budget_and_the_flag() {
        let network = genome(vec![
            node(1, NodeRole::Input),
            node(2, NodeRole::Output),
            edge_between(
                10,
                1,
                2,
                EDGE_FLAG_PLASTIC,
                genes(RULE_HEBBIAN, 0.5, [1.0, 0.0, 0.0, 0.0], 0),
            ),
        ]);
        let population = vec![network];

        let enabled = expressed_conjunction_census(&population, PlasticityBudget::edges(32))
            .expect("compiles");
        assert_eq!(enabled.plastic_edges, 1);
        assert_eq!(enabled.non_static, 1);
        assert_eq!(enabled.full_conjunction, 1);
        assert_eq!(enabled.depth, [0, 0, 0, 0, 1]);
        assert_eq!(enabled.live_edges, 1);

        // `None` is the disabled world, and it is **not** a budget of zero:
        // nothing is compiled plastic and nothing is counted as refused.
        let disabled = expressed_conjunction_census(&population, PlasticityBudget::disabled())
            .expect("compiles");
        assert_eq!(disabled.plastic_edges, 0);
        assert_eq!(disabled.over_budget, 0);
        assert_eq!(disabled.full_conjunction, 0);
        assert_eq!(disabled.live_edges, 1, "the edge is still expressed");

        // A budget of zero refuses it and says so, which is the third case
        // and the one that distinguishes "disabled" from "capped".
        let capped = expressed_conjunction_census(&population, PlasticityBudget::edges(0))
            .expect("compiles");
        assert_eq!(capped.plastic_edges, 0);
        assert_eq!(capped.over_budget, 1);
    }

    /// A modulated edge naming an ordinary node resolves to `NO_MODULATOR` -
    /// `compile_with_budget`'s role check - so the expressed gated count must
    /// fall while the ungated one does not.
    #[test]
    fn only_a_modulatory_node_gates_an_expressed_edge() {
        let build = |role: NodeRole| {
            vec![genome(vec![
                node(1, NodeRole::Input),
                node(2, NodeRole::Output),
                node(3, role),
                edge_between(
                    10,
                    1,
                    2,
                    EDGE_FLAG_PLASTIC,
                    genes(RULE_MODULATED_HEBBIAN, 0.5, [1.0, 0.0, 0.0, 0.0], 3),
                ),
            ])]
        };
        let wrong =
            expressed_conjunction_census(&build(NodeRole::Hidden), PlasticityBudget::edges(32))
                .unwrap();
        assert_eq!(wrong.plastic_edges, 1);
        assert_eq!(wrong.full_conjunction, 1);
        assert_eq!(wrong.modulator_resolved, 0);
        assert_eq!(wrong.modulated_without_modulator, 1);
        assert_eq!(wrong.full_conjunction_gated, 0);

        let right =
            expressed_conjunction_census(&build(NodeRole::Modulatory), PlasticityBudget::edges(32))
                .unwrap();
        assert_eq!(right.modulator_resolved, 1);
        assert_eq!(right.modulated_without_modulator, 0);
        assert_eq!(right.full_conjunction_gated, 1);
    }

    fn node(homology_id: u32, role: NodeRole) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Node {
                role,
                activation_id: 0,
                bias: 0.0,
                time_constant: 0,
            },
        }
    }

    fn edge_between(
        homology_id: u32,
        source: u32,
        target: u32,
        flags: u8,
        plasticity: PlasticityGenes,
    ) -> Locus {
        Locus {
            homology_id,
            gene_lineage_id: u64::from(homology_id),
            mutation_event_id: 0,
            kind: LocusKind::Edge {
                source,
                target,
                weight: 1.0,
                flags,
                plasticity,
            },
        }
    }

    /// Arm totals are sums, and the sum has to be over every field. A field
    /// left out of `add_alleles` would report per world and read zero here.
    #[test]
    fn arm_totals_sum_every_field_of_every_world() {
        let world = |seed: u64, alleles: AlleleConjunctionCensus| WorldConjunction {
            condition: "Avar".to_owned(),
            seed,
            population: 10,
            extinct: false,
            alleles,
            expressed: ExpressedConjunctionCensus::default(),
            learned: Some(LearnedStateCensus {
                rows: 4,
                nonzero_learned: 1,
                sum_abs_learned_q16: 65_536,
                max_abs_learned_q16: 65_536,
                organisms_with_nonzero_learned: 1,
                updates_applied: 10,
                updates_static: 90,
                ..LearnedStateCensus::default()
            }),
            plastic_edges_total: 4,
            plasticity_updates_total: 100,
            reported_mean_abs_learned_milli: 0,
        };
        let one = allele_conjunction_census(&[genome(vec![edge(
            1,
            EDGE_FLAG_PLASTIC,
            genes(RULE_HEBBIAN, 0.5, [1.0, 0.0, 0.0, 0.0], 4),
        )])]);
        let arm = summarise("Avar", &[world(1, one), world(2, one)]);
        assert_eq!(arm.worlds, 2);
        assert_eq!(arm.alleles.edge_alleles, 4);
        assert_eq!(arm.alleles.full_conjunction, 4);
        assert_eq!(arm.alleles.full_conjunction_gated, 4);
        assert_eq!(arm.alleles.modulator_named, 4);
        assert_eq!(arm.alleles.depth, [0, 0, 0, 0, 4]);
        assert_eq!(arm.alleles.rules, [0, 4, 0, 0, 0]);
        assert_eq!(arm.worlds_with_any_full_conjunction, 2);
        assert_eq!(arm.learned.rows, 8);
        assert_eq!(arm.learned.nonzero_learned, 2);
        assert_eq!(arm.worlds_with_any_nonzero_learned, 2);
        assert_eq!(arm.max_abs_learned_q16, 65_536);
        // 131072 * 1000 / 65536 / 8 = 250.
        assert_eq!(arm.learned.mean_abs_learned_milli, 250);
        assert_eq!(arm.plasticity_updates_total, 200);
        assert_eq!(arm.learned.updates_static, 180);
        assert_eq!(arm.learned.updates_applied, 20);
    }

    /// A world with no learned-state section is not a world of zeros. The B
    /// arms carry no section at all, and folding them into the same total
    /// would report a population of plastic edges that never existed.
    #[test]
    fn an_absent_learned_section_is_not_counted_as_zeros() {
        let absent = WorldConjunction {
            condition: "Bvar".to_owned(),
            seed: 1,
            population: 10,
            extinct: false,
            alleles: AlleleConjunctionCensus::default(),
            expressed: ExpressedConjunctionCensus::default(),
            learned: None,
            plastic_edges_total: 0,
            plasticity_updates_total: 0,
            reported_mean_abs_learned_milli: 0,
        };
        let arm = summarise("Bvar", std::slice::from_ref(&absent));
        assert_eq!(arm.worlds, 1);
        assert_eq!(arm.worlds_with_learned_section, 0);
        assert_eq!(arm.learned.organisms, 0);
        assert!(
            render("c", &[("Bvar".to_owned(), vec![absent])], &[arm]).contains("section=absent")
        );
    }

    /// Every report says what it is. A census reported as a test is the
    /// failure mode this whole module is one step away from.
    #[test]
    fn the_report_declares_itself_a_census_and_not_a_test() {
        let text = render("phase11-c111-confirmatory", &[], &[]);
        assert!(text.contains("DESCRIPTIVE CENSUS"));
        assert!(text.contains("Not a hypothesis test"));
        assert!(text.contains(CONJUNCTION_CENSUS_VERSION));
    }
}
