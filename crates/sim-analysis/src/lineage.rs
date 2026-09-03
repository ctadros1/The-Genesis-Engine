//! Per-world lineage census of multi-module organisms (`lifesim-lineage-index-v1`).
//!
//! Phase 19 found that under "coupling v2" scratch worlds, bodies above one
//! module appear in 17 of 30 worlds but only one two-module organism at a
//! time, and none persists. The next question is whether those organisms
//! reproduce and whether their children keep the second module. This module
//! answers that from the same event log `world_demography` reads, and in the
//! same spirit: everything here is an arithmetic fact about recorded events,
//! never an estimate, and analysis observes and never instructs (ADR-0016) --
//! there is no threshold anywhere below.
//!
//! A "multi-module organism" is one whose module count - the maximum over
//! its `BodyComposition` sums and its `GrowthCompleted` counts - is two or
//! more. Historically: one with at least one `GrowthCompleted`
//! record whose `modules` is two or greater; an organism with several such
//! records is counted once, at its maximum. `Materialized` organisms
//! (Phase 16) always complete at one module, so in practice they never
//! qualify, but the census does not assume that -- it reads whatever the log
//! says.
//!
//! Lifespans follow `world_demography`'s rule exactly: a life starts at its
//! `Birth`, `PairedBirth`, or `Materialized` record and ends at its `Death`
//! record, a lifespan is `death_tick - start_tick`, and an organism with no
//! `Death` record is censored, counted, and never imputed. An organism whose
//! start record never appears in this log (a founder, admitted complete at
//! tick zero) has no defined start tick, so it is excluded from the lifespan
//! and censoring counts the same way -- it is neither completed nor
//! censored, because neither is a fact this log recorded. It is still
//! counted once in `multi_unknown` if it is a multi-module organism, because
//! that a multi-module organism started life outside this log is itself a
//! fact worth reporting.

use sim_core::{Event, EventKind};
use std::collections::{BTreeMap, BTreeSet};

pub const LINEAGE_INDEX_VERSION: &str = "lifesim-lineage-index-v1";

/// How an organism's life began, as recorded in the event log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartKind {
    /// A `Birth` or `PairedBirth` record.
    Born,
    /// A `Materialized` record (Phase 16, ADR-0032).
    Materialized,
}

/// One world's lineage summary: the multi-module organisms in its event
/// log, how they started, how long they lived, and whether they left
/// multi-module offspring.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldLineage {
    /// Organisms with at least one `GrowthCompleted` record whose `modules`
    /// is two or greater, counted once each at their maximum recorded
    /// `modules`.
    pub multi_total: u64,
    /// Of `multi_total`, the ones that started as a `Birth` or
    /// `PairedBirth` record.
    pub multi_born: u64,
    /// Of `multi_total`, the ones that started as a `Materialized` record.
    pub multi_materialized: u64,
    /// Of `multi_total`, the ones whose start record never appears in this
    /// log (for example a founder, admitted complete at tick zero).
    pub multi_unknown: u64,
    /// Multi-module organisms with a known start tick and a recorded
    /// `Death`: the count their lifespan statistic below is computed from.
    pub multi_completed_lifespans: u64,
    /// Lower median of `multi_completed_lifespans`' lifespans, in ticks.
    /// Zero when none completed -- read beside the count above, never as
    /// "instant".
    pub multi_median_lifespan_ticks: u64,
    /// Multi-module organisms with a known start tick that have no `Death`
    /// record in this log: alive at the end, censored, never imputed.
    pub multi_censored: u64,
    /// Lower median completed lifespan, in ticks, of the contemporaries the
    /// multi-module organisms are compared against: BORN organisms whose
    /// maximum recorded `GrowthCompleted.modules` is exactly one. Zero when
    /// none completed.
    pub one_module_born_median_lifespan_ticks: u64,
    /// The matched one-module cohort (ADR-0035): born one-module organisms
    /// admitted within `COHORT_WINDOW_TICKS` of any multi-module organism's
    /// admission in this world, so the comparison does not mix the second
    /// module with the epoch it appears in. Completed and censored are
    /// both counted; the median is over completed lifespans.
    pub cohort_completed: u64,
    pub cohort_censored: u64,
    pub cohort_median_lifespan_ticks: u64,
    /// Count of one-module-born organisms the median above is computed
    /// from (those with a recorded `Death`).
    pub one_module_born_completed: u64,
    /// `Birth`/`PairedBirth` records whose parent (`parent_id`, `parent_a`,
    /// or `parent_b`) is a multi-module organism. Each birth is counted
    /// once even when both parents of a `PairedBirth` qualify.
    pub multi_offspring_total: u64,
    /// Multi-module organisms that are a qualifying parent of at least one
    /// offspring counted in `multi_offspring_total`.
    pub multi_parents: u64,
    /// Born organisms that are themselves multi-module and have at least
    /// one multi-module parent.
    pub second_generation: u64,
    /// The largest `modules` value seen in any `GrowthCompleted` record in
    /// this world.
    pub max_modules: u64,
    /// Tick of the earliest `GrowthCompleted` record with `modules >= 2`.
    /// Zero when none.
    pub first_multi_tick: u64,
    /// Tick of the latest `Death` record of a multi-module organism. Zero
    /// when none died.
    pub last_multi_death_tick: u64,
    /// Composition multiset of multi-module bodies: each distinct count
    /// array (registry order, joined by '.') with its multiplicity, joined
    /// by ';' in lexical order. Empty when no record was present.
    /// Births with at least one BORN parent, and distinct born organisms
    /// that are parents - whether born organisms reproduce at all in this
    /// world (a materialized organism's children may all die before
    /// maturity, in which case the lineage question has a prior answer).
    pub births_with_born_parent: u64,
    pub born_parents: u64,
    pub multi_compositions: String,
    /// For every multi-module born child whose parent's record exists,
    /// the child-minus-parent counts against the parent with the larger
    /// module count - which type a duplication added - as a histogram of
    /// `type:count` over positive differences, ';'-joined. Empty when none.
    pub added_modules: String,
}

/// Half-width of the matched cohort's admission window, ticks.
pub const COHORT_WINDOW_TICKS: u64 = 2_000;

/// The lower median of a sorted, completed list; zero when nothing
/// completed, matching `demography::median_completed`.
fn median_completed(sorted: &[u64]) -> u64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[(sorted.len() - 1) / 2]
    }
}

/// Reduce one world's event log to its lineage summary.
fn note_modules(
    by_id: &mut BTreeMap<u64, u32>,
    overall: &mut u32,
    first_multi: &mut Option<u64>,
    id: u64,
    modules: u32,
    tick: u64,
) {
    let entry = by_id.entry(id).or_insert(0);
    if modules > *entry {
        *entry = modules;
    }
    if modules > *overall {
        *overall = modules;
    }
    if modules >= 2 {
        *first_multi = Some(first_multi.map_or(tick, |earliest| earliest.min(tick)));
    }
}

pub fn world_lineage(events: &[Event]) -> WorldLineage {
    // Pass 1: collect the raw facts the log carries, one map per fact,
    // keyed on organism id. Nothing here decides anything; it is the same
    // "read the log, keep what it says" discipline `world_demography` uses.
    let mut start: BTreeMap<u64, (u64, StartKind)> = BTreeMap::new();
    let mut parents_of: BTreeMap<u64, Vec<u64>> = BTreeMap::new();
    let mut max_modules_by_id: BTreeMap<u64, u32> = BTreeMap::new();
    let mut composition_by_id: BTreeMap<u64, [u16; sim_core::MODULE_TYPE_COUNT]> = BTreeMap::new();
    let mut death_tick: BTreeMap<u64, u64> = BTreeMap::new();
    let mut max_modules_overall: u32 = 0;
    let mut first_multi_tick: Option<u64> = None;

    for event in events {
        let tick = event.tick;
        match event.kind {
            EventKind::Birth { id, parent_id } => {
                start.entry(id).or_insert((tick, StartKind::Born));
                parents_of.entry(id).or_insert_with(|| vec![parent_id]);
            }
            EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                ..
            } => {
                start.entry(id).or_insert((tick, StartKind::Born));
                parents_of
                    .entry(id)
                    .or_insert_with(|| vec![parent_a, parent_b]);
            }
            EventKind::Materialized { id, .. } => {
                start.entry(id).or_insert((tick, StartKind::Materialized));
            }
            EventKind::GrowthCompleted { id, modules } => {
                note_modules(&mut max_modules_by_id, &mut max_modules_overall, &mut first_multi_tick, id, modules, tick);
            }
            EventKind::BodyComposition { id, counts } => {
                let modules: u32 = counts.iter().map(|&c| u32::from(c)).sum();
                note_modules(&mut max_modules_by_id, &mut max_modules_overall, &mut first_multi_tick, id, modules, tick);
                // Keep the record with the most modules (a grown body's
                // completion supersedes its admission record).
                let keep = composition_by_id
                    .get(&id)
                    .map_or(true, |old| old.iter().map(|&c| u32::from(c)).sum::<u32>() <= modules);
                if keep {
                    composition_by_id.insert(id, counts);
                }
            }
            EventKind::Death { id, .. } => {
                death_tick.insert(id, tick);
            }
            _ => {}
        }
    }

    // Pass 2: everything below is a pure function of the four maps above --
    // the multi-module set, the lifespan lists, and the parent/offspring
    // relations. No further reading of `events` is needed.
    let multi_set: BTreeSet<u64> = max_modules_by_id
        .iter()
        .filter(|&(_, &modules)| modules >= 2)
        .map(|(&id, _)| id)
        .collect();

    let mut summary = WorldLineage {
        multi_total: multi_set.len() as u64,
        max_modules: u64::from(max_modules_overall),
        first_multi_tick: first_multi_tick.unwrap_or(0),
        ..WorldLineage::default()
    };

    let mut multi_lifespans: Vec<u64> = Vec::new();
    let mut last_multi_death: Option<u64> = None;
    for &id in &multi_set {
        match start.get(&id) {
            Some(&(_, StartKind::Born)) => summary.multi_born += 1,
            Some(&(_, StartKind::Materialized)) => summary.multi_materialized += 1,
            None => summary.multi_unknown += 1,
        }
        let Some(&(start_tick, _)) = start.get(&id) else {
            continue;
        };
        match death_tick.get(&id) {
            Some(&dtick) => {
                multi_lifespans.push(dtick.saturating_sub(start_tick));
                last_multi_death = Some(match last_multi_death {
                    Some(latest) => latest.max(dtick),
                    None => dtick,
                });
            }
            None => summary.multi_censored += 1,
        }
    }
    multi_lifespans.sort_unstable();
    summary.multi_completed_lifespans = multi_lifespans.len() as u64;
    summary.multi_median_lifespan_ticks = median_completed(&multi_lifespans);
    summary.last_multi_death_tick = last_multi_death.unwrap_or(0);

    // The one-module contemporaries: BORN organisms whose maximum recorded
    // `GrowthCompleted.modules` is exactly one, restricted the same way to
    // the ones with a completed lifespan.
    let mut one_module_lifespans: Vec<u64> = Vec::new();
    for (&id, &(start_tick, kind)) in &start {
        if kind != StartKind::Born {
            continue;
        }
        if max_modules_by_id.get(&id) != Some(&1) {
            continue;
        }
        if let Some(&dtick) = death_tick.get(&id) {
            one_module_lifespans.push(dtick.saturating_sub(start_tick));
        }
    }
    one_module_lifespans.sort_unstable();
    summary.one_module_born_completed = one_module_lifespans.len() as u64;
    summary.one_module_born_median_lifespan_ticks = median_completed(&one_module_lifespans);

    // The matched cohort: the same one-module born organisms, restricted to
    // those admitted within the window of any multi-module admission.
    let multi_starts: Vec<u64> = multi_set
        .iter()
        .filter_map(|id| start.get(id).map(|&(tick, _)| tick))
        .collect();
    let mut cohort_lifespans: Vec<u64> = Vec::new();
    for (&id, &(start_tick, kind)) in &start {
        if kind != StartKind::Born || max_modules_by_id.get(&id) != Some(&1) {
            continue;
        }
        let matched = multi_starts
            .iter()
            .any(|&m| start_tick.abs_diff(m) <= COHORT_WINDOW_TICKS);
        if !matched {
            continue;
        }
        match death_tick.get(&id) {
            Some(&dtick) => cohort_lifespans.push(dtick.saturating_sub(start_tick)),
            None => summary.cohort_censored += 1,
        }
    }
    cohort_lifespans.sort_unstable();
    summary.cohort_completed = cohort_lifespans.len() as u64;
    summary.cohort_median_lifespan_ticks = median_completed(&cohort_lifespans);

    // Offspring and generational counts, read from `parents_of` against the
    // multi-module set computed above.
    let mut offspring_total: u64 = 0;
    let mut qualifying_parents: BTreeSet<u64> = BTreeSet::new();
    let mut second_generation: u64 = 0;
    for (&child_id, parent_ids) in &parents_of {
        let qualifying: Vec<u64> = parent_ids
            .iter()
            .copied()
            .filter(|parent_id| multi_set.contains(parent_id))
            .collect();
        if qualifying.is_empty() {
            continue;
        }
        offspring_total += 1;
        for parent_id in qualifying {
            qualifying_parents.insert(parent_id);
        }
        if multi_set.contains(&child_id) {
            // Inherited, not a fresh duplication inside a multi-module
            // lineage: the child's count may not exceed its best parent's.
            let child_modules = max_modules_by_id.get(&child_id).copied().unwrap_or(0);
            let parent_modules = parent_ids
                .iter()
                .filter_map(|p| max_modules_by_id.get(p).copied())
                .max()
                .unwrap_or(0);
            if child_modules <= parent_modules {
                second_generation += 1;
            }
        }
    }
    summary.multi_offspring_total = offspring_total;
    summary.multi_parents = qualifying_parents.len() as u64;
    summary.second_generation = second_generation;

    // Born parents: does reproduction ever run on a born organism?
    let mut born_parent_set: BTreeSet<u64> = BTreeSet::new();
    for parent_ids in parents_of.values() {
        let born: Vec<u64> = parent_ids
            .iter()
            .copied()
            .filter(|p| matches!(start.get(p), Some(&(_, StartKind::Born))))
            .collect();
        if !born.is_empty() {
            summary.births_with_born_parent += 1;
            born_parent_set.extend(born);
        }
    }
    summary.born_parents = born_parent_set.len() as u64;

    // Composition multiset over multi-module bodies.
    let mut compositions: BTreeMap<String, u64> = BTreeMap::new();
    for id in &multi_set {
        if let Some(counts) = composition_by_id.get(id) {
            let key = counts.iter().map(|c| c.to_string()).collect::<Vec<_>>().join(".");
            *compositions.entry(key).or_insert(0) += 1;
        }
    }
    summary.multi_compositions = compositions
        .iter()
        .map(|(key, n)| format!("{key}x{n}"))
        .collect::<Vec<_>>()
        .join(";");

    // What a duplication added: child minus the parent with more modules,
    // over multi-module born children whose parent record exists.
    const TYPE_NAMES: [&str; sim_core::MODULE_TYPE_COUNT] =
        ["structural", "sensory", "motor", "digestive", "storage", "reproductive", "neural"];
    let mut added: BTreeMap<usize, u64> = BTreeMap::new();
    for (&child_id, parent_ids) in &parents_of {
        if !multi_set.contains(&child_id) {
            continue;
        }
        let Some(child) = composition_by_id.get(&child_id) else {
            continue;
        };
        let parent = parent_ids
            .iter()
            .filter_map(|p| composition_by_id.get(p))
            .max_by_key(|c| c.iter().map(|&v| u32::from(v)).sum::<u32>());
        let Some(parent) = parent else {
            continue;
        };
        for slot in 0..sim_core::MODULE_TYPE_COUNT {
            let diff = i64::from(child[slot]) - i64::from(parent[slot]);
            if diff > 0 {
                *added.entry(slot).or_insert(0) += diff as u64;
            }
        }
    }
    summary.added_modules = added
        .iter()
        .map(|(slot, n)| format!("{}:{n}", TYPE_NAMES[*slot]))
        .collect::<Vec<_>>()
        .join(";");

    summary
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::DeathCause;

    fn scan(events: Vec<(u64, EventKind)>) -> Vec<Event> {
        events
            .into_iter()
            .map(|(tick, kind)| Event { tick, kind })
            .collect()
    }

    fn birth(id: u64, parent_id: u64) -> EventKind {
        EventKind::Birth { id, parent_id }
    }

    fn paired_birth(id: u64, parent_a: u64, parent_b: u64) -> EventKind {
        EventKind::PairedBirth {
            id,
            parent_a,
            parent_b,
            genome_hash: 0,
            invest_a_milli: 0,
            invest_b_milli: 0,
            mutated_trait_genes: 0,
            mutated_neural_genes: 0,
        }
    }

    fn materialized(id: u64) -> EventKind {
        EventKind::Materialized {
            id,
            cell: 0,
            class: 0,
            energy_milli: 4_000,
        }
    }

    fn growth(id: u64, modules: u32) -> EventKind {
        EventKind::GrowthCompleted { id, modules }
    }

    fn death(id: u64) -> EventKind {
        EventKind::Death {
            id,
            cause: DeathCause::Extrinsic,
        }
    }

    #[test]
    fn a_multi_module_organism_and_its_one_module_sibling_are_both_fully_accounted() {
        let summary = world_lineage(&scan(vec![
            (1, birth(10, 0)),
            (1, birth(11, 0)),
            (20, growth(11, 1)),
            (50, growth(10, 2)),
            (200, death(11)),
            (300, death(10)),
        ]));
        assert_eq!(summary.multi_total, 1);
        assert_eq!(summary.multi_born, 1);
        assert_eq!(summary.multi_materialized, 0);
        assert_eq!(summary.multi_unknown, 0);
        assert_eq!(summary.multi_completed_lifespans, 1);
        assert_eq!(summary.multi_median_lifespan_ticks, 299);
        assert_eq!(summary.multi_censored, 0);
        assert_eq!(summary.one_module_born_completed, 1);
        assert_eq!(summary.one_module_born_median_lifespan_ticks, 199);
        assert_eq!(summary.multi_offspring_total, 0);
        assert_eq!(summary.multi_parents, 0);
        assert_eq!(summary.second_generation, 0);
        assert_eq!(summary.max_modules, 2);
        assert_eq!(summary.first_multi_tick, 50);
        assert_eq!(summary.last_multi_death_tick, 300);
    }

    #[test]
    fn a_materialized_multi_module_organism_that_never_dies_is_censored_not_imputed() {
        let summary = world_lineage(&scan(vec![
            (5, materialized(20)),
            (100, growth(20, 3)),
            // Organism 20 never dies.
        ]));
        assert_eq!(summary.multi_total, 1);
        assert_eq!(summary.multi_born, 0);
        assert_eq!(summary.multi_materialized, 1);
        assert_eq!(summary.multi_unknown, 0);
        assert_eq!(summary.multi_completed_lifespans, 0);
        assert_eq!(summary.multi_median_lifespan_ticks, 0);
        assert_eq!(summary.multi_censored, 1);
        assert_eq!(summary.last_multi_death_tick, 0);
        assert_eq!(summary.max_modules, 3);
        assert_eq!(summary.first_multi_tick, 100);
    }

    #[test]
    fn a_second_generation_multi_module_child_is_counted_once_with_its_qualifying_parent() {
        let summary = world_lineage(&scan(vec![
            (1, birth(30, 0)),
            (10, growth(30, 2)),
            // Two children of the same multi-module parent: one stays
            // multi-module, one does not.
            (50, paired_birth(31, 30, 99)),
            (50, paired_birth(32, 30, 99)),
            (80, growth(31, 2)),
            (90, growth(32, 1)),
        ]));
        assert_eq!(summary.multi_total, 2); // 30 and 31
        assert_eq!(summary.multi_offspring_total, 2); // both 31 and 32
        assert_eq!(summary.multi_parents, 1); // only 30, counted once
        assert_eq!(summary.second_generation, 1); // only 31 is multi with a multi parent
    }

    #[test]
    fn a_world_with_no_multi_module_growth_reports_zeros_with_one_module_medians_computed() {
        let summary = world_lineage(&scan(vec![
            (1, birth(1, 0)),
            (1, birth(2, 0)),
            (10, growth(1, 1)),
            (10, growth(2, 1)),
            (50, death(1)),
            (150, death(2)),
        ]));
        assert_eq!(summary.multi_total, 0);
        assert_eq!(summary.multi_born, 0);
        assert_eq!(summary.multi_materialized, 0);
        assert_eq!(summary.multi_unknown, 0);
        assert_eq!(summary.multi_completed_lifespans, 0);
        assert_eq!(summary.multi_median_lifespan_ticks, 0);
        assert_eq!(summary.multi_censored, 0);
        assert_eq!(summary.multi_offspring_total, 0);
        assert_eq!(summary.multi_parents, 0);
        assert_eq!(summary.second_generation, 0);
        assert_eq!(summary.first_multi_tick, 0);
        assert_eq!(summary.last_multi_death_tick, 0);
        // The one-module comparison group is still fully computed: lifespans
        // 49 and 149, lower median 49.
        assert_eq!(summary.one_module_born_completed, 2);
        assert_eq!(summary.one_module_born_median_lifespan_ticks, 49);
        assert_eq!(summary.max_modules, 1);
    }

    fn composition(id: u64, counts: [u16; 7]) -> EventKind {
        EventKind::BodyComposition { id, counts }
    }

    #[test]
    fn the_composition_record_alone_makes_an_organism_multi_module() {
        // No GrowthCompleted anywhere (ontogeny off, as in every Phase 16
        // and 19 campaign): the record's sum is the module count.
        let summary = world_lineage(&scan(vec![
            (100, birth(1, 0)),
            (100, composition(1, [0, 0, 0, 2, 0, 0, 0])),
            (100, birth(2, 0)),
            (100, composition(2, [0, 0, 0, 1, 0, 0, 0])),
            (900, death(1)),
            (700, death(2)),
        ]));
        assert_eq!(summary.multi_total, 1);
        assert_eq!(summary.max_modules, 2);
        assert_eq!(summary.first_multi_tick, 100);
        assert_eq!(summary.multi_median_lifespan_ticks, 800);
        assert_eq!(summary.cohort_completed, 1);
        assert_eq!(summary.cohort_median_lifespan_ticks, 600);
        assert_eq!(summary.multi_compositions, "0.0.0.2.0.0.0x1");
    }

    #[test]
    fn a_child_with_more_modules_than_its_parent_is_a_new_duplication_not_a_second_generation() {
        let summary = world_lineage(&scan(vec![
            (100, birth(1, 0)),
            (100, composition(1, [0, 0, 0, 2, 0, 0, 0])),
            (200, paired_birth(2, 1, 9)),
            (200, composition(2, [0, 1, 0, 2, 0, 0, 0])), // three modules: added a sensor
            (300, paired_birth(3, 1, 9)),
            (300, composition(3, [0, 0, 0, 2, 0, 0, 0])), // inherited as is
            (400, paired_birth(4, 1, 9)),
            (400, composition(4, [0, 0, 0, 1, 0, 0, 0])), // lost it
        ]));
        assert_eq!(summary.multi_offspring_total, 3);
        assert_eq!(summary.multi_parents, 1);
        assert_eq!(summary.second_generation, 1, "only the child that carried the parent's count");
        assert_eq!(summary.added_modules, "sensory:1");
    }

    #[test]
    fn the_matched_cohort_excludes_one_module_organisms_born_far_from_any_multi_module_admission() {
        let summary = world_lineage(&scan(vec![
            (50_000, birth(1, 0)),
            (50_000, composition(1, [0, 0, 0, 2, 0, 0, 0])),
            (49_000, birth(2, 0)), // inside the window
            (49_000, composition(2, [0, 0, 0, 1, 0, 0, 0])),
            (10_000, birth(3, 0)), // far outside it
            (10_000, composition(3, [0, 0, 0, 1, 0, 0, 0])),
            (49_500, death(2)),
            (10_400, death(3)),
        ]));
        assert_eq!(summary.one_module_born_completed, 2, "the world-wide count keeps both");
        assert_eq!(summary.cohort_completed, 1);
        assert_eq!(summary.cohort_censored, 0);
        assert_eq!(summary.cohort_median_lifespan_ticks, 500);
        assert_eq!(summary.multi_censored, 1);
    }

    #[test]
    fn growth_completion_and_composition_agree_when_both_are_present() {
        let summary = world_lineage(&scan(vec![
            (100, birth(1, 0)),
            (100, composition(1, [0, 0, 0, 1, 0, 0, 0])),
            (100, EventKind::GrowthCompleted { id: 1, modules: 1 }),
            (600, EventKind::GrowthCompleted { id: 1, modules: 2 }),
            (600, composition(1, [0, 0, 1, 1, 0, 0, 0])),
        ]));
        assert_eq!(summary.multi_total, 1);
        assert_eq!(summary.first_multi_tick, 600);
        assert_eq!(summary.multi_compositions, "0.0.1.1.0.0.0x1", "the completed body supersedes the admission record");
    }
}
