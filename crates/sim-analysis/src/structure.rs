//! Phase 9 structural evolution: C9.1, C9.2, and C9.5.
//!
//! Reduces a campaign manifest to one record per world and applies the
//! criteria's decision rules. Everything here observes; nothing here can
//! reach the kernel (ADR-0016).
//!
//! ## Why two statistics, and why they are allowed to disagree
//!
//! C9.1 names the **median** expressed node and edge count. That is a
//! deliberately hard bar: founders are minimal, so the median moves only
//! once half the living population has diverged from the founding topology
//! -- which for a neutral variant means something close to fixation. The
//! **mean** moves as soon as any organism carries a duplicate at all.
//!
//! Both are reported. A world where the mean has moved and the median has
//! not is a world with standing structural variation that has not spread,
//! which is a different finding from "nothing happened" and the report has
//! to be able to say which one it is looking at.
//!
//! ## What C9.1 does and does not establish
//!
//! Given a per-birth duplication rate and a generation count, *some*
//! structural change is arithmetic. What is not arithmetic is whether the
//! resulting structures survive: a duplicate can be rejected by a cap, orphan
//! an edge, produce a network that will not compile, or simply be purged.
//! C9.1 at a stated rate therefore tests whether structural variation is
//! **viable and spreads**, not whether mutation occurs. The rate is reported
//! alongside every result so that claim can be read at its true strength.

use crate::demography::world_demography;
use crate::paired::{Direction, Pair, PairedResult, compare, median_milli};
use sim_core::MutationCounters;
use sim_experiment::{Manifest, RunResult};
use std::fmt::Write as _;

pub const STRUCTURE_ANALYSIS_VERSION: &str = "lifesim-structure-analysis-v1";

/// The analysis plan. Every field is echoed into the report so a reader can
/// check it against the campaign source rather than trusting a summary.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StructurePlan {
    /// Founder expressed node and edge count. Minimal by construction and
    /// asserted at tick 0 by `phase9_world::structure_evolves_and_stays_
    /// inside_its_caps`, so this is a restatement rather than an assumption.
    pub founder_nodes: u64,
    pub founder_edges: u64,
    /// Smallest median shift that counts, in whole nodes or edges. One is
    /// the minimum a median of integers can express.
    pub median_shift_min: u64,
    /// C9.1's two bars, out of the campaign's seed count.
    pub median_bar: usize,
    pub diversity_bar: usize,
    /// C9.2's equivalence bound on the relative scale, milli-units, and its
    /// bar. A world passes if it is inside the bound **or better**, which is
    /// the criterion's own wording: a treatment that raises population is
    /// not destabilizing it.
    pub stability_tolerance_milli: i64,
    pub stability_bar: usize,
    pub analysis_seed: u64,
}

impl Default for StructurePlan {
    fn default() -> Self {
        Self {
            founder_nodes: 3,
            founder_edges: 2,
            median_shift_min: 1,
            median_bar: 20,
            diversity_bar: 25,
            stability_tolerance_milli: 250,
            stability_bar: 20,
            analysis_seed: 0x9e3779b97f4a7c15,
        }
    }
}

/// One world's structural outcome.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WorldStructure {
    pub seed: u64,
    pub population: u64,
    pub extinct: bool,
    pub births: u64,
    pub generations: u32,
    pub median_nodes: u64,
    pub median_edges: u64,
    pub mean_nodes_milli: u64,
    pub mean_edges_milli: u64,
    pub distinct_structures: u64,
    pub applied: u64,
    pub rejected: u64,
    pub nonviable_recombinants: u64,
    /// Median completed lifespan, ticks. Zero when no event log was read.
    pub median_lifespan_ticks: u64,
    /// Structural-mutation outcomes by operator and by rejection reason.
    ///
    /// C9.8 restates the caps from a measurement, and the restatement is
    /// only meaningful if the campaign can say whether the caps ever bound.
    /// The `structmut_rejected` total cannot answer that: a cap rejection and
    /// a self-loop draw both land in it. The manifest now carries the
    /// thirteen per-class columns, so this reads from the run record and
    /// falls back to re-opening the world's snapshot only for manifests
    /// written before those columns existed - which is the only remaining
    /// reason a campaign has to keep snapshots on to answer the question.
    ///
    /// `None` when neither source had them, and reported as absent rather
    /// than as zero: "not recorded" and "never happened" are opposite
    /// conclusions, and every count below is summed only over the worlds
    /// that had it.
    pub counters: Option<MutationCounters>,
}

impl WorldStructure {
    /// `snapshot_counters` is the pre-column fallback and is used **only**
    /// when the run record carries none. A manifest written by this build
    /// always wins, so a campaign run with `output snapshots off` answers
    /// the cap question and a re-analysis of an archived campaign still
    /// answers it from the `.alif` files beside the manifest.
    fn from_run(
        run: &RunResult,
        median_lifespan_ticks: u64,
        snapshot_counters: Option<MutationCounters>,
    ) -> Self {
        Self {
            seed: run.seed,
            population: run.population,
            extinct: run.extinct,
            births: run.counters.births_total,
            generations: run.max_ancestry_depth,
            median_nodes: run.median_nodes,
            median_edges: run.median_edges,
            mean_nodes_milli: run.mean_nodes_milli,
            mean_edges_milli: run.mean_edges_milli,
            distinct_structures: run.distinct_structures,
            applied: run.structural_mutations_applied,
            rejected: run.structural_mutations_rejected,
            nonviable_recombinants: run
                .phase2
                .map_or(0, |phase2| phase2.pair_rejected_nonviable_total),
            median_lifespan_ticks,
            counters: run.mutation.or(snapshot_counters),
        }
    }

    /// Cap refusals, `None` when this world carried no per-class counters.
    pub fn rejected_cap(&self) -> Option<u64> {
        self.counters.map(|counters| counters.rejected_cap)
    }

    pub fn rejected_inapplicable(&self) -> Option<u64> {
        self.counters.map(|counters| counters.rejected_inapplicable)
    }

    pub fn rejected_invalid(&self) -> Option<u64> {
        self.counters.map(|counters| counters.rejected_invalid)
    }

    /// Did structure reach the population median, in either quantity?
    ///
    /// Either, not both: duplication grows node count and insertion grows
    /// edge count, so requiring both would make the criterion a test of
    /// which operator was enabled rather than of whether structure evolved.
    fn median_shifted(&self, plan: &StructurePlan) -> bool {
        self.median_nodes >= plan.founder_nodes + plan.median_shift_min
            || self.median_edges >= plan.founder_edges + plan.median_shift_min
    }

    fn diversified(&self) -> bool {
        self.distinct_structures > 1
    }

    /// Structurally frozen: the control's expected state, and checked
    /// exactly rather than approximately, because under condition B it holds
    /// by construction and anything else is a defect in the gating.
    fn invariant(&self, plan: &StructurePlan) -> bool {
        self.median_nodes == plan.founder_nodes
            && self.median_edges == plan.founder_edges
            && self.mean_nodes_milli == plan.founder_nodes * 1_000
            && self.mean_edges_milli == plan.founder_edges * 1_000
            && self.distinct_structures <= 1
    }
}

/// C9.1's outcome for one condition.
#[derive(Clone, Debug, PartialEq)]
pub struct StructureOutcome {
    pub condition: String,
    pub worlds: usize,
    pub extinct: usize,
    pub median_shifted: usize,
    pub diversified: usize,
    pub invariant: usize,
    pub median_generations: i64,
    pub median_births: i64,
    pub median_population: i64,
    pub median_nodes: i64,
    pub median_edges: i64,
    pub median_mean_nodes_milli: i64,
    pub median_mean_edges_milli: i64,
    pub median_distinct: i64,
    pub total_applied: u64,
    pub total_rejected: u64,
    pub total_nonviable: u64,
    /// Per-class totals, summed across the worlds that carried them, with
    /// the count of those worlds beside it so a zero can be told apart from
    /// an absence. All thirteen classes rather than the three this used to
    /// carry: the five applied counters answer "did duplication ever fire",
    /// which is the other half of the question a bare total cannot.
    pub total_counters: MutationCounters,
    pub worlds_with_counters: usize,
}

/// Field-by-field sum, **`..`-free on both operands** (D-077).
///
/// A counter added to `MutationCounters` fails to compile here rather than
/// being quietly left out of the condition totals and the report - which is
/// the same failure the manifest columns exist to prevent, one layer up.
fn accumulate(total: MutationCounters, add: MutationCounters) -> MutationCounters {
    let MutationCounters {
        point_applied,
        duplication_applied,
        deletion_applied,
        insertion_applied,
        transposition_applied,
        binding_applied,
        rejected_homology_collision,
        rejected_orphaned,
        rejected_min_nodes,
        rejected_no_bindings,
        rejected_cap,
        rejected_inapplicable,
        rejected_cycle,
        rejected_invalid,
    } = total;
    let MutationCounters {
        point_applied: more_point_applied,
        duplication_applied: more_duplication_applied,
        deletion_applied: more_deletion_applied,
        insertion_applied: more_insertion_applied,
        transposition_applied: more_transposition_applied,
        binding_applied: more_binding_applied,
        rejected_homology_collision: more_rejected_homology_collision,
        rejected_orphaned: more_rejected_orphaned,
        rejected_min_nodes: more_rejected_min_nodes,
        rejected_no_bindings: more_rejected_no_bindings,
        rejected_cap: more_rejected_cap,
        rejected_inapplicable: more_rejected_inapplicable,
        rejected_cycle: more_rejected_cycle,
        rejected_invalid: more_rejected_invalid,
    } = add;
    MutationCounters {
        point_applied: point_applied + more_point_applied,
        duplication_applied: duplication_applied + more_duplication_applied,
        deletion_applied: deletion_applied + more_deletion_applied,
        insertion_applied: insertion_applied + more_insertion_applied,
        transposition_applied: transposition_applied + more_transposition_applied,
        binding_applied: binding_applied + more_binding_applied,
        rejected_homology_collision: rejected_homology_collision + more_rejected_homology_collision,
        rejected_orphaned: rejected_orphaned + more_rejected_orphaned,
        rejected_min_nodes: rejected_min_nodes + more_rejected_min_nodes,
        rejected_no_bindings: rejected_no_bindings + more_rejected_no_bindings,
        rejected_cap: rejected_cap + more_rejected_cap,
        rejected_inapplicable: rejected_inapplicable + more_rejected_inapplicable,
        rejected_cycle: rejected_cycle + more_rejected_cycle,
        rejected_invalid: rejected_invalid + more_rejected_invalid,
    }
}

pub fn summarise(
    condition: &str,
    worlds: &[WorldStructure],
    plan: &StructurePlan,
) -> StructureOutcome {
    let pick = |values: Vec<i64>| median_milli(&values);
    StructureOutcome {
        condition: condition.to_owned(),
        worlds: worlds.len(),
        extinct: worlds.iter().filter(|world| world.extinct).count(),
        median_shifted: worlds.iter().filter(|w| w.median_shifted(plan)).count(),
        diversified: worlds.iter().filter(|w| w.diversified()).count(),
        invariant: worlds.iter().filter(|w| w.invariant(plan)).count(),
        median_generations: pick(worlds.iter().map(|w| i64::from(w.generations)).collect()),
        median_births: pick(worlds.iter().map(|w| w.births as i64).collect()),
        median_population: pick(worlds.iter().map(|w| w.population as i64).collect()),
        median_nodes: pick(worlds.iter().map(|w| w.median_nodes as i64).collect()),
        median_edges: pick(worlds.iter().map(|w| w.median_edges as i64).collect()),
        median_mean_nodes_milli: pick(worlds.iter().map(|w| w.mean_nodes_milli as i64).collect()),
        median_mean_edges_milli: pick(worlds.iter().map(|w| w.mean_edges_milli as i64).collect()),
        median_distinct: pick(
            worlds
                .iter()
                .map(|w| w.distinct_structures as i64)
                .collect(),
        ),
        total_applied: worlds.iter().map(|w| w.applied).sum(),
        total_rejected: worlds.iter().map(|w| w.rejected).sum(),
        total_nonviable: worlds.iter().map(|w| w.nonviable_recombinants).sum(),
        total_counters: worlds
            .iter()
            .filter_map(|w| w.counters)
            .fold(MutationCounters::default(), accumulate),
        worlds_with_counters: worlds.iter().filter(|w| w.counters.is_some()).count(),
    }
}

/// Seed-matched pairs of a world-level quantity, treatment against control.
///
/// Worlds present in only one condition are dropped rather than compared
/// against nothing, and the count of dropped seeds is the caller's to
/// report.
pub fn pairs_of(
    treatment: &[WorldStructure],
    control: &[WorldStructure],
    quantity: impl Fn(&WorldStructure) -> i64,
) -> Vec<Pair> {
    let mut pairs = Vec::new();
    for world in treatment {
        if let Some(peer) = control.iter().find(|peer| peer.seed == world.seed) {
            pairs.push(Pair {
                seed: world.seed,
                treatment_milli: quantity(world),
                control_milli: quantity(peer),
            });
        }
    }
    pairs.sort_by_key(|pair| pair.seed);
    pairs
}

/// C9.2: worlds inside the tolerance **or better**.
///
/// "Or better" is the criterion's own wording and it matters: a treatment
/// that raises population has not destabilized the ecology, so counting it
/// as a failure for being outside a two-sided band would be wrong.
pub fn stability_count(pairs: &[Pair], tolerance_milli: i64) -> usize {
    pairs
        .iter()
        .filter(|pair| match pair.relative_milli() {
            Some(relative) => relative >= -tolerance_milli,
            // A control of zero is a dead world; the treatment cannot be
            // "within tolerance" of it in any meaningful sense.
            None => false,
        })
        .count()
}

#[derive(Clone, Debug, PartialEq)]
pub struct StabilityReport {
    pub treatment: String,
    pub control: String,
    pub quantity: String,
    pub pairs: usize,
    pub within_or_better: usize,
    pub paired: PairedResult,
}

pub fn stability(
    treatment_name: &str,
    control_name: &str,
    quantity: &str,
    pairs: &[Pair],
    plan: &StructurePlan,
) -> StabilityReport {
    StabilityReport {
        treatment: treatment_name.to_owned(),
        control: control_name.to_owned(),
        quantity: quantity.to_owned(),
        pairs: pairs.len(),
        within_or_better: stability_count(pairs, plan.stability_tolerance_milli),
        // Null rate 500: under the null the treatment is as likely to sit
        // above the control as below, so a per-world "reaches the bound"
        // event has probability one half. The p-value is not the decision
        // rule for C9.2 - the count against `stability_bar` is - but it is
        // reported so the count can be read against chance.
        paired: compare(
            pairs,
            plan.stability_tolerance_milli,
            500,
            Direction::Either,
            plan.analysis_seed,
        ),
    }
}

/// Read every world in a condition, attaching lifespan from the event log
/// when one is present.
///
/// A missing log yields a zero lifespan and is reported as such by the
/// caller; it is never silently treated as "no difference".
///
/// The per-class mutation counters come from the manifest. Re-opening the
/// world's snapshot - decompressing it and rebuilding a whole `World` to
/// read thirteen integers - is now the fallback for manifests written
/// before the columns existed, not the primary path, and it is skipped
/// entirely when the run record carries them.
pub fn worlds_for(
    manifest: &Manifest,
    directory: &std::path::Path,
    condition: &str,
) -> Vec<WorldStructure> {
    manifest
        .runs_for(condition)
        .into_iter()
        .map(|run| {
            let stem = sim_experiment::run_stem(&run.condition, run.seed);
            let lifespan = std::fs::read(directory.join(format!("{stem}.alev")))
                .ok()
                .and_then(|bytes| sim_persist::decode_log_events(&bytes).ok())
                .map(|(_, events)| world_demography(&events).median_lifespan_ticks)
                .unwrap_or(0);
            let snapshot_counters = run.mutation.is_none().then(|| {
                std::fs::read(directory.join(format!("{stem}.alif")))
                    .ok()
                    .and_then(|bytes| sim_persist::decode_snapshot(&bytes).ok())
                    .and_then(|(_, state)| sim_core::World::from_state(state).ok())
                    .and_then(|world| world.mutation_counters())
            });
            WorldStructure::from_run(run, lifespan, snapshot_counters.flatten())
        })
        .collect()
}

/// Render the flat report. One line per world, then one per condition, then
/// the criterion decisions, each with the counts that produced it.
pub fn render(
    campaign_id: &str,
    plan: &StructurePlan,
    per_world: &[(String, Vec<WorldStructure>)],
    outcomes: &[StructureOutcome],
    stabilities: &[StabilityReport],
) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "structure-report 1 campaign {campaign_id}");
    let _ = writeln!(out, "analysis-version {STRUCTURE_ANALYSIS_VERSION}");
    let _ = writeln!(
        out,
        "plan founder_nodes={} founder_edges={} median_shift_min={} median_bar={} \
         diversity_bar={} stability_tolerance_milli={} stability_bar={} analysis_seed={:#018x}",
        plan.founder_nodes,
        plan.founder_edges,
        plan.median_shift_min,
        plan.median_bar,
        plan.diversity_bar,
        plan.stability_tolerance_milli,
        plan.stability_bar,
        plan.analysis_seed,
    );
    for (condition, worlds) in per_world {
        for world in worlds {
            let _ = writeln!(
                out,
                "world condition={condition} seed={:#018x} population={} extinct={} births={} \
                 generations={} median_nodes={} median_edges={} mean_nodes_milli={} \
                 mean_edges_milli={} distinct={} applied={} rejected={} nonviable={} \
                 median_lifespan={}",
                world.seed,
                world.population,
                world.extinct,
                world.births,
                world.generations,
                world.median_nodes,
                world.median_edges,
                world.mean_nodes_milli,
                world.mean_edges_milli,
                world.distinct_structures,
                world.applied,
                world.rejected,
                world.nonviable_recombinants,
                world.median_lifespan_ticks,
            );
        }
    }
    for outcome in outcomes {
        // Destructured with no `..` so a class added to `MutationCounters`
        // has to be given a column here rather than disappearing from the
        // only place a human reads these.
        let MutationCounters {
            point_applied,
            duplication_applied,
            deletion_applied,
            insertion_applied,
            transposition_applied,
            binding_applied,
            rejected_homology_collision,
            rejected_orphaned,
            rejected_min_nodes,
            rejected_no_bindings,
            rejected_cap,
            rejected_inapplicable,
            rejected_cycle,
            rejected_invalid,
        } = outcome.total_counters;
        let _ = writeln!(
            out,
            "condition {} worlds={} extinct={} median_shifted={} diversified={} invariant={} \
             med_generations={} med_births={} med_population={} med_nodes={} med_edges={} \
             med_mean_nodes_milli={} med_mean_edges_milli={} med_distinct={} \
             applied={} rejected={} nonviable={} point_applied={point_applied} \
             duplication_applied={duplication_applied} deletion_applied={deletion_applied} \
             insertion_applied={insertion_applied} \
             transposition_applied={transposition_applied} \
             binding_applied={binding_applied} \
             rejected_homology_collision={rejected_homology_collision} \
             rejected_orphaned={rejected_orphaned} rejected_min_nodes={rejected_min_nodes} \
             rejected_no_bindings={rejected_no_bindings} rejected_cap={rejected_cap} \
             rejected_inapplicable={rejected_inapplicable} rejected_cycle={rejected_cycle} \
             rejected_invalid={rejected_invalid} worlds_with_counters={}",
            outcome.condition,
            outcome.worlds,
            outcome.extinct,
            outcome.median_shifted,
            outcome.diversified,
            outcome.invariant,
            outcome.median_generations,
            outcome.median_births,
            outcome.median_population,
            outcome.median_nodes,
            outcome.median_edges,
            outcome.median_mean_nodes_milli,
            outcome.median_mean_edges_milli,
            outcome.median_distinct,
            outcome.total_applied,
            outcome.total_rejected,
            outcome.total_nonviable,
            outcome.worlds_with_counters,
        );
    }
    for report in stabilities {
        let _ = writeln!(
            out,
            "stability treatment={} control={} quantity={} pairs={} within_or_better={} \
             mean_relative_milli={} relative_ci_low_milli={} relative_ci_high_milli={} \
             equivalent={} median_difference_milli={}",
            report.treatment,
            report.control,
            report.quantity,
            report.pairs,
            report.within_or_better,
            report.paired.mean_relative_milli,
            report.paired.relative_ci_low_milli,
            report.paired.relative_ci_high_milli,
            report.paired.equivalent,
            report.paired.median_difference_milli,
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn world(seed: u64, median_nodes: u64, median_edges: u64, distinct: u64) -> WorldStructure {
        WorldStructure {
            seed,
            population: 1_000,
            extinct: false,
            births: 30_000,
            generations: 60,
            median_nodes,
            median_edges,
            mean_nodes_milli: median_nodes * 1_000,
            mean_edges_milli: median_edges * 1_000,
            distinct_structures: distinct,
            applied: 100,
            rejected: 10,
            nonviable_recombinants: 0,
            median_lifespan_ticks: 1_000,
            counters: Some(MutationCounters::default()),
        }
    }

    #[test]
    fn the_median_bar_needs_a_whole_node_or_edge_not_a_fraction() {
        let plan = StructurePlan::default();
        // Founder-identical: no shift, whatever the diversity.
        assert!(!world(1, 3, 2, 9).median_shifted(&plan));
        // One more node is a shift; so is one more edge on its own, because
        // duplication grows nodes and insertion grows edges and the
        // criterion must not become a test of which operator was enabled.
        assert!(world(1, 4, 2, 2).median_shifted(&plan));
        assert!(world(1, 3, 3, 2).median_shifted(&plan));
    }

    #[test]
    fn a_frozen_control_is_recognised_only_when_every_quantity_is_frozen() {
        let plan = StructurePlan::default();
        assert!(world(1, 3, 2, 1).invariant(&plan));
        // A moved mean with an unmoved median is *not* invariant. This is
        // the case the control exists to rule out: structural variation
        // that exists but has not spread would otherwise read as "the
        // structural mutation gating worked".
        let mut drifted = world(1, 3, 2, 1);
        drifted.mean_nodes_milli = 3_044;
        assert!(!drifted.invariant(&plan));
        assert!(!world(1, 3, 2, 4).invariant(&plan));
    }

    #[test]
    fn stability_counts_better_as_passing_rather_than_as_a_two_sided_failure() {
        let plan = StructurePlan::default();
        let pairs = vec![
            // 20% below control: inside a 25% tolerance.
            Pair {
                seed: 1,
                treatment_milli: 800,
                control_milli: 1_000,
            },
            // 30% below: outside.
            Pair {
                seed: 2,
                treatment_milli: 700,
                control_milli: 1_000,
            },
            // Twice the control: "or better", so it passes.
            Pair {
                seed: 3,
                treatment_milli: 2_000,
                control_milli: 1_000,
            },
        ];
        assert_eq!(stability_count(&pairs, plan.stability_tolerance_milli), 2);
    }

    #[test]
    fn an_unmatched_seed_is_dropped_rather_than_compared_against_nothing() {
        let treatment = vec![world(1, 4, 2, 5), world(2, 4, 2, 5), world(3, 4, 2, 5)];
        let control = vec![world(1, 3, 2, 1), world(3, 3, 2, 1)];
        let pairs = pairs_of(&treatment, &control, |world| world.population as i64);
        assert_eq!(pairs.len(), 2);
        assert_eq!(
            pairs.iter().map(|pair| pair.seed).collect::<Vec<_>>(),
            vec![1, 3]
        );
    }
}
