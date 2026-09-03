//! Per-world census of the born cohort's life (`lifesim-cohort-index-v1`,
//! Phase 21, ADR-0036).
//!
//! Phase 20 found the term that limits every multi-module lineage: a born
//! organism lives a fraction of a materialized one's life, and the
//! candidate reasons are the ground it is born on, the order it eats in,
//! and the company it keeps. `BirthSite` (event schema 13) records the
//! cell, occupant count, own trait-derived maturity, and field masses a
//! newcomer finds at admission -- for births and materializations alike --
//! and this module reduces that record, together with `Birth`,
//! `PairedBirth`, `Materialized`, and `Death`, into the numbers the plan's
//! "The census, exactly" section names. Nothing here decides anything: no
//! threshold, no verdict (ADR-0016).
//!
//! FOOD, throughout, is `substrate[S_PRIMORDIAL] + substrate[S_MONOMER] +
//! biomass_milli` -- what an organism can actually eat. Waste, polymer,
//! and microbial density are read from the same record but are never
//! summed into FOOD; they are reported as separate columns because a cell
//! grazing produces waste and consumes microbial mass, and conflating
//! either with food would make heavily grazed ground look starved.
//!
//! Lifespans and censoring follow the same rule `lineage::world_lineage`
//! and `demography::world_demography` use: a life starts at its `Birth`,
//! `PairedBirth`, or `Materialized` record and ends at its `Death`
//! record; `lifespan = death_tick - start_tick`; an organism with no
//! `Death` record is censored, counted, and never imputed. A born or
//! materialized organism whose `BirthSite` record is missing from the log
//! (should not happen -- the kernel emits one per admission -- but the
//! census does not assume it) is excluded from every site-derived
//! statistic (food, occupants, waste, polymer, microbial, maturity,
//! reproduction-by-bucket) exactly as an organism with no site record has
//! no site to report; it is still counted in the plain lifespan/censoring
//! totals, because that much the log does say.

use crate::demography::spearman_milli;
use sim_core::{Event, EventKind, S_MONOMER, S_POLYMER, S_PRIMORDIAL, S_WASTE};
use std::collections::{BTreeMap, BTreeSet};

pub const COHORT_INDEX_VERSION: &str = "lifesim-cohort-index-v1";

/// Width of the fixed admission-tick blocks C21.1's block-stratified rank
/// correlations are computed within, so a run's own drift from empty
/// ungrazed ground to a grazed, crowded steady state cannot manufacture
/// the sign of an association that block stratification is meant to rule
/// out (ADR-0036, "The census, exactly").
const ADMISSION_BLOCK_TICKS: u64 = 20_000;

/// A block must hold at least this many born organisms with a completed
/// lifespan and a site record before its rank correlations are computed;
/// fewer and the block is skipped and counted (ADR-0022's floor of 30,
/// applied per block here rather than per world).
const MIN_BLOCK_ORGANISMS: usize = 30;

/// How an organism's life began, as recorded in the event log.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StartKind {
    /// A `Birth` or `PairedBirth` record.
    Born,
    /// A `Materialized` record (Phase 16, ADR-0032).
    Materialized,
}

/// The fields of one organism's `BirthSite` record this census needs.
/// `cell` is not kept -- nothing here reads which cell, only what was in
/// it.
#[derive(Clone, Copy, Debug)]
struct Site {
    occupants: u16,
    maturity_ticks: u32,
    substrate_milli: [i64; sim_core::SUBSTRATE_COUNT],
    microbial_milli: i64,
    biomass_milli: i64,
}

impl Site {
    /// FOOD: primordial + monomer + biomass. Waste, polymer, and
    /// microbial density are read from the same record but never enter
    /// this sum (see the module doc comment).
    fn food_milli(&self) -> i64 {
        self.substrate_milli[S_PRIMORDIAL] + self.substrate_milli[S_MONOMER] + self.biomass_milli
    }
}

/// One world's census of the born cohort's life, with the materialized
/// cohort as a magnitude reference (ADR-0036). Every rho is milli-units;
/// every median is a lower median (`sorted[(len - 1) / 2]`), matching
/// `lineage` and `demography`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldCohort {
    /// Born organisms with a recorded `Death`: the count the born median
    /// lifespan below is computed from.
    pub born_completed: u64,
    /// Born organisms with no `Death` record in this log: alive at the
    /// end, censored, never imputed.
    pub born_censored: u64,
    /// Lower median of `born_completed`'s lifespans, in ticks. Zero when
    /// none completed -- read beside the count, never as "instant".
    pub born_median_lifespan_ticks: u64,
    /// Lower median of FOOD at admission over every born organism with a
    /// `BirthSite` record, completed and censored alike -- a site
    /// statistic, not a lifespan statistic, so censoring does not exclude
    /// a member from it.
    pub born_site_food_median: i64,
    /// Lower median of occupant count at admission, same population as
    /// `born_site_food_median`.
    pub born_occupants_median: u64,
    /// Materialized organisms with a recorded `Death`.
    pub mat_completed: u64,
    /// Materialized organisms with no `Death` record: censored.
    pub mat_censored: u64,
    /// Lower median of `mat_completed`'s lifespans, in ticks.
    pub mat_median_lifespan_ticks: u64,
    /// Lower median of FOOD at admission over every materialized organism
    /// with a `BirthSite` record, completed and censored alike. A
    /// magnitude reference, not a control: materialization selects the
    /// densest cells by construction (ADR-0032's trigger).
    pub mat_site_food_median: i64,
    /// Lower median of occupant count at admission, same population as
    /// `mat_site_food_median`.
    pub mat_occupants_median: u64,
    /// `born_site_food_median * 1000 / mat_site_food_median`; 0 when the
    /// denominator is 0.
    pub food_ratio_milli: i64,
    /// Median, over qualifying 20,000-tick admission blocks, of the
    /// block's Spearman rho (milli, ties averaged) between a born
    /// organism's completed lifespan and its birth-site FOOD. 0 when
    /// `blocks_used` is 0.
    pub rho_food_milli: i64,
    /// Median, over qualifying blocks, of the block's Spearman rho
    /// between completed lifespan and occupants at birth. 0 when
    /// `blocks_used` is 0.
    pub rho_occupants_milli: i64,
    /// Admission blocks with at least `MIN_BLOCK_ORGANISMS` born
    /// organisms that have both a completed lifespan and a site record --
    /// the blocks `rho_food_milli` and `rho_occupants_milli` are the
    /// median over.
    pub blocks_used: u64,
    /// Admission blocks with at least one such organism but fewer than
    /// `MIN_BLOCK_ORGANISMS`: skipped, and counted as skipped rather than
    /// silently absent.
    pub blocks_skipped: u64,
    /// The same lifespan/FOOD Spearman rho pooled over the whole run,
    /// with no block stratification -- reported for the record beside the
    /// block median, never used in place of it, because a pooled rho over
    /// a run that moves from empty ground to a grazed steady state would
    /// carry that trend as if it were place.
    pub pooled_rho_food_milli: i64,
    /// The same pooled rho for lifespan and occupants.
    pub pooled_rho_occupants_milli: i64,
    /// Median, over qualifying blocks, of the **rank partial** of
    /// lifespan and FOOD controlling for occupants: rank lifespan, FOOD,
    /// and occupants within the block (ties averaged); regress the
    /// lifespan-rank and FOOD-rank on the occupants-rank by ordinary
    /// least squares; take the residuals; and compute the Spearman
    /// correlation of the residual pairs (itself a fresh ranking and
    /// Pearson correlation of that ranking). This is a rank partial, not
    /// a regression coefficient -- no slope or intercept from the OLS
    /// step is reported anywhere.
    pub partial_food_milli: i64,
    /// The same rank partial of lifespan and occupants, controlling for
    /// FOOD instead.
    pub partial_occupants_milli: i64,
    /// Born organisms with a `BirthSite` record whose completed lifespan
    /// is at least the record's own `maturity_ticks`, or who are censored
    /// with `horizon - start >= maturity_ticks` (`horizon` = the last
    /// event tick in this log) -- reaching maturity while still alive at
    /// the end counts as reaching it.
    pub born_reached_maturity: u64,
    /// Born organisms with a `BirthSite` record whose id appears as
    /// `parent_id`, `parent_a`, or `parent_b` on some `Birth` or
    /// `PairedBirth` record in this log.
    pub born_reproduced: u64,
    /// `born_reached_maturity`, split by the born cohort's own FOOD
    /// quartile (bounds from the born cohort's FOOD distribution, lower
    /// median-style cuts at ranks `(n-1)/4`, `(n-1)/2`, `3(n-1)/4`):
    /// `[q1, q2, q3, q4]`.
    pub reached_maturity_food_quartile: [u64; 4],
    /// `born_reproduced`, split the same way.
    pub reproduced_food_quartile: [u64; 4],
    /// `born_reached_maturity`, split by occupant count at birth: `[0, 1,
    /// 2, 3+]`.
    pub reached_maturity_occupants: [u64; 4],
    /// `born_reproduced`, split the same way.
    pub reproduced_occupants: [u64; 4],
    /// Lower median of waste (`S_WASTE`) at born admission sites. Never
    /// part of FOOD.
    pub waste_median: i64,
    /// Lower median of polymer (`S_POLYMER`) at born admission sites.
    /// Never part of FOOD.
    pub polymer_median: i64,
    /// Lower median of microbial density at born admission sites. Never
    /// part of FOOD.
    pub microbial_median: i64,
}

/// The lower median of a sorted list, u64; zero when empty.
fn median_u64(sorted: &[u64]) -> u64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[(sorted.len() - 1) / 2]
    }
}

/// The lower median of a sorted list, i64; zero when empty. Sign-agnostic:
/// the index rule is the same regardless of whether the values are
/// negative, so a food or rho column reduces exactly like a tick count.
fn median_i64(sorted: &[i64]) -> i64 {
    if sorted.is_empty() {
        0
    } else {
        sorted[(sorted.len() - 1) / 2]
    }
}

/// Tie-averaged ranks of an `f64` slice. Same construction as
/// `demography::spearman_milli`'s internal rank helper, generalized to
/// floating-point input so it can rank OLS residuals inside the
/// within-block partials, which are not integers even though the values
/// they are computed from are.
fn rank_of_f64(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].partial_cmp(&values[b]).expect("finite input"));
    let mut ranks = vec![0.0_f64; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }
        let average = (start + end - 1) as f64 / 2.0;
        for &index in &order[start..end] {
            ranks[index] = average;
        }
        start = end;
    }
    ranks
}

/// Spearman correlation (milli) between two equal-length rank-space
/// series: rank each with tie averaging and take the Pearson correlation
/// of the ranks -- the same construction as `demography::spearman_milli`,
/// but over `f64` input so it applies to OLS residuals (see
/// `WorldCohort::partial_food_milli`'s doc comment: a rank partial, not a
/// regression coefficient).
fn spearman_of_f64(xs: &[f64], ys: &[f64]) -> i64 {
    let n = xs.len();
    if n < 3 {
        return 0;
    }
    let rx = rank_of_f64(xs);
    let ry = rank_of_f64(ys);
    let mean = (n - 1) as f64 / 2.0;
    let mut numerator = 0.0;
    let mut sx = 0.0;
    let mut sy = 0.0;
    for index in 0..n {
        let dx = rx[index] - mean;
        let dy = ry[index] - mean;
        numerator += dx * dy;
        sx += dx * dx;
        sy += dy * dy;
    }
    if sx <= 0.0 || sy <= 0.0 {
        return 0;
    }
    ((numerator / (sx * sy).sqrt()) * 1_000.0) as i64
}

/// Ordinary-least-squares residuals of `y` regressed on `x`: `y - (a +
/// b*x)` for the least-squares line through the block's points. Used only
/// as the "control for occupants" (or "control for FOOD") step inside a
/// within-block rank partial; the slope and intercept are discarded once
/// the residuals are taken.
fn residuals_of(x: &[f64], y: &[f64]) -> Vec<f64> {
    let n = x.len() as f64;
    let mean_x = x.iter().sum::<f64>() / n;
    let mean_y = y.iter().sum::<f64>() / n;
    let mut covariance = 0.0;
    let mut variance_x = 0.0;
    for index in 0..x.len() {
        covariance += (x[index] - mean_x) * (y[index] - mean_y);
        variance_x += (x[index] - mean_x) * (x[index] - mean_x);
    }
    let slope = if variance_x > 0.0 { covariance / variance_x } else { 0.0 };
    let intercept = mean_y - slope * mean_x;
    x.iter()
        .zip(y.iter())
        .map(|(&xi, &yi)| yi - (intercept + slope * xi))
        .collect()
}

/// One qualifying born organism: a completed lifespan and a site record,
/// carrying exactly what the block-stratified rank correlations need.
struct BornPoint {
    block: u64,
    lifespan_milli: i64,
    food_milli: i64,
    occupants_milli: i64,
}

/// FOOD quartile bucket (0-indexed) from cut points at the lower-median
/// style ranks `(n-1)/4`, `(n-1)/2`, `3(n-1)/4` of the born cohort's own
/// FOOD distribution.
fn quartile_of(food_milli: i64, cut1: i64, cut2: i64, cut3: i64) -> usize {
    if food_milli <= cut1 {
        0
    } else if food_milli <= cut2 {
        1
    } else if food_milli <= cut3 {
        2
    } else {
        3
    }
}

/// Occupant-count bucket: `0, 1, 2, 3+`.
fn occupant_bucket_of(occupants: u16) -> usize {
    match occupants {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 3,
    }
}

/// Reduce one world's event log to its born-cohort census.
pub fn world_cohort(events: &[Event]) -> WorldCohort {
    // Pass 1: read the log into id-keyed maps. Nothing here decides
    // anything; it is the same discipline `world_lineage` and
    // `world_demography` use.
    let mut start: BTreeMap<u64, (u64, StartKind)> = BTreeMap::new();
    let mut death_tick: BTreeMap<u64, u64> = BTreeMap::new();
    let mut sites: BTreeMap<u64, Site> = BTreeMap::new();
    let mut reproduced_ids: BTreeSet<u64> = BTreeSet::new();
    let mut horizon: u64 = 0;

    for event in events {
        let tick = event.tick;
        if tick > horizon {
            horizon = tick;
        }
        match event.kind {
            EventKind::Birth { id, parent_id } => {
                start.entry(id).or_insert((tick, StartKind::Born));
                reproduced_ids.insert(parent_id);
            }
            EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                ..
            } => {
                start.entry(id).or_insert((tick, StartKind::Born));
                reproduced_ids.insert(parent_a);
                reproduced_ids.insert(parent_b);
            }
            EventKind::Materialized { id, .. } => {
                start.entry(id).or_insert((tick, StartKind::Materialized));
            }
            EventKind::BirthSite {
                id,
                occupants,
                maturity_ticks,
                substrate_milli,
                microbial_milli,
                biomass_milli,
                ..
            } => {
                sites.entry(id).or_insert(Site {
                    occupants,
                    maturity_ticks,
                    substrate_milli,
                    microbial_milli,
                    biomass_milli,
                });
            }
            EventKind::Death { id, .. } => {
                death_tick.insert(id, tick);
            }
            _ => {}
        }
    }

    // Pass 2: everything below is a pure function of the maps above.
    let born_ids: Vec<u64> = start
        .iter()
        .filter(|&(_, &(_, kind))| kind == StartKind::Born)
        .map(|(&id, _)| id)
        .collect();
    let mat_ids: Vec<u64> = start
        .iter()
        .filter(|&(_, &(_, kind))| kind == StartKind::Materialized)
        .map(|(&id, _)| id)
        .collect();

    let born_summary = cohort_medians(&born_ids, &start, &death_tick, &sites);
    let mat_summary = cohort_medians(&mat_ids, &start, &death_tick, &sites);

    let mut summary = WorldCohort {
        born_completed: born_summary.0,
        born_censored: born_summary.1,
        born_median_lifespan_ticks: born_summary.2,
        born_site_food_median: born_summary.3,
        born_occupants_median: born_summary.4,
        mat_completed: mat_summary.0,
        mat_censored: mat_summary.1,
        mat_median_lifespan_ticks: mat_summary.2,
        mat_site_food_median: mat_summary.3,
        mat_occupants_median: mat_summary.4,
        food_ratio_milli: if mat_summary.3 == 0 {
            0
        } else {
            (i128::from(born_summary.3) * 1_000 / i128::from(mat_summary.3)) as i64
        },
        ..WorldCohort::default()
    };

    // Waste/polymer/microbial medians at born admission sites, never
    // folded into FOOD.
    let mut waste: Vec<i64> = Vec::new();
    let mut polymer: Vec<i64> = Vec::new();
    let mut microbial: Vec<i64> = Vec::new();
    for &id in &born_ids {
        if let Some(site) = sites.get(&id) {
            waste.push(site.substrate_milli[S_WASTE]);
            polymer.push(site.substrate_milli[S_POLYMER]);
            microbial.push(site.microbial_milli);
        }
    }
    waste.sort_unstable();
    polymer.sort_unstable();
    microbial.sort_unstable();
    summary.waste_median = median_i64(&waste);
    summary.polymer_median = median_i64(&polymer);
    summary.microbial_median = median_i64(&microbial);

    // Born organisms with a completed lifespan and a site record: the
    // population the block-stratified rank correlations and their
    // pooled/partial companions are computed over.
    let mut born_points: Vec<BornPoint> = Vec::new();
    for &id in &born_ids {
        let &(start_tick, _) = start.get(&id).expect("born id has a start");
        let Some(&dtick) = death_tick.get(&id) else {
            continue;
        };
        let Some(site) = sites.get(&id) else {
            continue;
        };
        born_points.push(BornPoint {
            block: start_tick / ADMISSION_BLOCK_TICKS,
            lifespan_milli: dtick.saturating_sub(start_tick) as i64,
            food_milli: site.food_milli(),
            occupants_milli: i64::from(site.occupants),
        });
    }

    let mut by_block: BTreeMap<u64, Vec<usize>> = BTreeMap::new();
    for (index, point) in born_points.iter().enumerate() {
        by_block.entry(point.block).or_default().push(index);
    }

    let mut block_food_rhos: Vec<i64> = Vec::new();
    let mut block_occupant_rhos: Vec<i64> = Vec::new();
    let mut block_partial_food: Vec<i64> = Vec::new();
    let mut block_partial_occupants: Vec<i64> = Vec::new();
    let mut blocks_used: u64 = 0;
    let mut blocks_skipped: u64 = 0;

    for indices in by_block.values() {
        if indices.len() < MIN_BLOCK_ORGANISMS {
            blocks_skipped += 1;
            continue;
        }
        blocks_used += 1;

        let lifespans: Vec<i64> = indices.iter().map(|&i| born_points[i].lifespan_milli).collect();
        let foods: Vec<i64> = indices.iter().map(|&i| born_points[i].food_milli).collect();
        let occupants: Vec<i64> = indices.iter().map(|&i| born_points[i].occupants_milli).collect();

        let food_points: Vec<(i64, i64)> =
            lifespans.iter().copied().zip(foods.iter().copied()).collect();
        let occupant_points: Vec<(i64, i64)> =
            lifespans.iter().copied().zip(occupants.iter().copied()).collect();
        block_food_rhos.push(spearman_milli(&food_points));
        block_occupant_rhos.push(spearman_milli(&occupant_points));

        let lifespan_f: Vec<f64> = lifespans.iter().map(|&v| v as f64).collect();
        let food_f: Vec<f64> = foods.iter().map(|&v| v as f64).collect();
        let occupants_f: Vec<f64> = occupants.iter().map(|&v| v as f64).collect();

        let rank_lifespan = rank_of_f64(&lifespan_f);
        let rank_food = rank_of_f64(&food_f);
        let rank_occupants = rank_of_f64(&occupants_f);

        // Partial of lifespan and FOOD, controlling for occupants.
        let resid_lifespan_on_occupants = residuals_of(&rank_occupants, &rank_lifespan);
        let resid_food_on_occupants = residuals_of(&rank_occupants, &rank_food);
        block_partial_food.push(spearman_of_f64(
            &resid_lifespan_on_occupants,
            &resid_food_on_occupants,
        ));

        // Partial of lifespan and occupants, controlling for FOOD.
        let resid_lifespan_on_food = residuals_of(&rank_food, &rank_lifespan);
        let resid_occupants_on_food = residuals_of(&rank_food, &rank_occupants);
        block_partial_occupants.push(spearman_of_f64(
            &resid_lifespan_on_food,
            &resid_occupants_on_food,
        ));
    }

    block_food_rhos.sort_unstable();
    block_occupant_rhos.sort_unstable();
    block_partial_food.sort_unstable();
    block_partial_occupants.sort_unstable();

    summary.blocks_used = blocks_used;
    summary.blocks_skipped = blocks_skipped;
    summary.rho_food_milli = median_i64(&block_food_rhos);
    summary.rho_occupants_milli = median_i64(&block_occupant_rhos);
    summary.partial_food_milli = median_i64(&block_partial_food);
    summary.partial_occupants_milli = median_i64(&block_partial_occupants);

    let pooled_food_points: Vec<(i64, i64)> = born_points
        .iter()
        .map(|point| (point.lifespan_milli, point.food_milli))
        .collect();
    let pooled_occupant_points: Vec<(i64, i64)> = born_points
        .iter()
        .map(|point| (point.lifespan_milli, point.occupants_milli))
        .collect();
    summary.pooled_rho_food_milli = spearman_milli(&pooled_food_points);
    summary.pooled_rho_occupants_milli = spearman_milli(&pooled_occupant_points);

    // Maturity and reproduction, by born organism, over every born id with
    // a site record (completed and censored alike -- reaching maturity
    // does not require having died).
    struct BornSite {
        food_milli: i64,
        occupants: u16,
        reached_maturity: bool,
        reproduced: bool,
    }
    let mut born_sites: Vec<BornSite> = Vec::new();
    for &id in &born_ids {
        let Some(site) = sites.get(&id) else {
            continue;
        };
        let &(start_tick, _) = start.get(&id).expect("born id has a start");
        let reached_maturity = match death_tick.get(&id) {
            Some(&dtick) => dtick.saturating_sub(start_tick) >= u64::from(site.maturity_ticks),
            None => horizon.saturating_sub(start_tick) >= u64::from(site.maturity_ticks),
        };
        born_sites.push(BornSite {
            food_milli: site.food_milli(),
            occupants: site.occupants,
            reached_maturity,
            reproduced: reproduced_ids.contains(&id),
        });
    }

    summary.born_reached_maturity = born_sites.iter().filter(|s| s.reached_maturity).count() as u64;
    summary.born_reproduced = born_sites.iter().filter(|s| s.reproduced).count() as u64;

    let mut sorted_food: Vec<i64> = born_sites.iter().map(|s| s.food_milli).collect();
    sorted_food.sort_unstable();
    let (cut1, cut2, cut3) = if sorted_food.is_empty() {
        (0, 0, 0)
    } else {
        let n = sorted_food.len();
        (
            sorted_food[(n - 1) / 4],
            sorted_food[(n - 1) / 2],
            sorted_food[3 * (n - 1) / 4],
        )
    };

    for site in &born_sites {
        let food_bucket = quartile_of(site.food_milli, cut1, cut2, cut3);
        let occupant_bucket = occupant_bucket_of(site.occupants);
        if site.reached_maturity {
            summary.reached_maturity_food_quartile[food_bucket] += 1;
            summary.reached_maturity_occupants[occupant_bucket] += 1;
        }
        if site.reproduced {
            summary.reproduced_food_quartile[food_bucket] += 1;
            summary.reproduced_occupants[occupant_bucket] += 1;
        }
    }

    summary
}

/// `(completed, censored, median_lifespan_ticks, site_food_median,
/// occupants_median)` for one cohort's ids.
fn cohort_medians(
    ids: &[u64],
    start: &BTreeMap<u64, (u64, StartKind)>,
    death_tick: &BTreeMap<u64, u64>,
    sites: &BTreeMap<u64, Site>,
) -> (u64, u64, u64, i64, u64) {
    let mut lifespans: Vec<u64> = Vec::new();
    let mut censored: u64 = 0;
    let mut foods: Vec<i64> = Vec::new();
    let mut occupants: Vec<u64> = Vec::new();

    for &id in ids {
        let &(start_tick, _) = start.get(&id).expect("cohort id has a start");
        match death_tick.get(&id) {
            Some(&dtick) => lifespans.push(dtick.saturating_sub(start_tick)),
            None => censored += 1,
        }
        if let Some(site) = sites.get(&id) {
            foods.push(site.food_milli());
            occupants.push(u64::from(site.occupants));
        }
    }

    lifespans.sort_unstable();
    foods.sort_unstable();
    occupants.sort_unstable();

    (
        lifespans.len() as u64,
        censored,
        median_u64(&lifespans),
        median_i64(&foods),
        median_u64(&occupants),
    )
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

    fn death(id: u64) -> EventKind {
        EventKind::Death {
            id,
            cause: DeathCause::Extrinsic,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn birth_site(
        id: u64,
        occupants: u16,
        maturity_ticks: u32,
        primordial: i64,
        monomer: i64,
        polymer: i64,
        waste: i64,
        microbial: i64,
        biomass: i64,
    ) -> EventKind {
        EventKind::BirthSite {
            id,
            cell: 0,
            occupants,
            maturity_ticks,
            substrate_milli: [primordial, monomer, polymer, waste],
            microbial_milli: microbial,
            biomass_milli: biomass,
        }
    }

    #[test]
    fn every_median_and_the_food_ratio_are_computed_from_two_known_cohorts() {
        let summary = world_cohort(&scan(vec![
            // Born: foods 1000, 2000, 3000; occupants 1, 2, 3; lifespans
            // 100, 300, 500 -- all completed.
            (0, birth(1, 0)),
            (0, birth_site(1, 1, 50, 100, 900, 0, 0, 0, 0)), // food 1000
            (100, death(1)),
            (0, birth(2, 0)),
            (0, birth_site(2, 2, 50, 200, 1_800, 0, 0, 0, 0)), // food 2000
            (300, death(2)),
            (0, birth(3, 0)),
            (0, birth_site(3, 3, 50, 300, 2_700, 0, 0, 0, 0)), // food 3000
            (500, death(3)),
            // Materialized: foods 4000, 5000, 6000; occupants 0, 1, 2;
            // lifespans 1000, 2000, 4000 -- all completed.
            (0, materialized(11)),
            (0, birth_site(11, 0, 50, 4_000, 0, 0, 0, 0, 0)),
            (1_000, death(11)),
            (0, materialized(12)),
            (0, birth_site(12, 1, 50, 5_000, 0, 0, 0, 0, 0)),
            (2_000, death(12)),
            (0, materialized(13)),
            (0, birth_site(13, 2, 50, 6_000, 0, 0, 0, 0, 0)),
            (4_000, death(13)),
        ]));

        assert_eq!(summary.born_completed, 3);
        assert_eq!(summary.born_censored, 0);
        assert_eq!(summary.born_median_lifespan_ticks, 300);
        assert_eq!(summary.born_site_food_median, 2_000);
        assert_eq!(summary.born_occupants_median, 2);

        assert_eq!(summary.mat_completed, 3);
        assert_eq!(summary.mat_censored, 0);
        assert_eq!(summary.mat_median_lifespan_ticks, 2_000);
        assert_eq!(summary.mat_site_food_median, 5_000);
        assert_eq!(summary.mat_occupants_median, 1);

        // 2000 * 1000 / 5000 = 400.
        assert_eq!(summary.food_ratio_milli, 400);
    }

    #[test]
    fn a_world_with_no_materialized_admissions_reports_a_zero_ratio() {
        let summary = world_cohort(&scan(vec![
            (0, birth(1, 0)),
            (0, birth_site(1, 1, 50, 100, 0, 0, 0, 0, 0)),
            (100, death(1)),
        ]));
        assert_eq!(summary.mat_site_food_median, 0);
        assert_eq!(summary.food_ratio_milli, 0);
    }

    /// Builds a block of `n` born organisms starting at `block_start_tick`
    /// (all inside the same 20,000-tick admission block when `n` ticks
    /// apart is smaller than the block width), with lifespan and food
    /// both increasing in `k` (so the food/lifespan relation is a clean
    /// monotone increase) and occupants decreasing in `k` (so the
    /// occupants/lifespan relation is a clean monotone decrease).
    fn monotone_block(block_start_tick: u64, n: u64) -> Vec<(u64, EventKind)> {
        let mut events = Vec::new();
        for k in 0..n {
            let id = 1_000 + block_start_tick + k;
            let start_tick = block_start_tick + k; // stays inside the block
            let lifespan = 100 + k * 10;
            let food = 1_000 + (k as i64) * 100;
            let occupants = (n - k) as u16; // decreasing as k (and lifespan) rise
            events.push((start_tick, birth(id, 0)));
            events.push((start_tick, birth_site(id, occupants, 0, food, 0, 0, 0, 0, 0)));
            events.push((start_tick + lifespan, death(id)));
        }
        events
    }

    #[test]
    fn a_block_under_the_floor_is_skipped_and_a_full_block_yields_signed_rhos() {
        let mut events = monotone_block(0, 10); // block 0: 10 < 30, skipped
        events.extend(monotone_block(20_000, 30)); // block 1: exactly 30, used
        let summary = world_cohort(&scan(events));

        assert_eq!(summary.blocks_skipped, 1);
        assert_eq!(summary.blocks_used, 1);
        assert!(
            (summary.rho_food_milli - 1_000).abs() <= 1,
            "expected ~1000, got {}",
            summary.rho_food_milli
        );
        assert!(
            (summary.rho_occupants_milli + 1_000).abs() <= 1,
            "expected ~-1000, got {}",
            summary.rho_occupants_milli
        );
    }

    #[test]
    fn maturity_and_reproduction_are_read_from_the_record_and_from_paired_birth_parents() {
        // Horizon is the last event tick in the log: 300 (the paired
        // birth below).
        let mut events = vec![
            // A: completed, lifespan 150 >= maturity 100 -> reached.
            (0, birth(1, 0)),
            (0, birth_site(1, 0, 100, 1_000, 0, 0, 0, 0, 0)), // occupants 0
            (150, death(1)),
            // B: completed, lifespan 50 < maturity 100 -> not reached.
            (0, birth(2, 0)),
            (0, birth_site(2, 1, 100, 1_000, 0, 0, 0, 0, 0)), // occupants 1
            (50, death(2)),
            // C: censored, horizon(300) - start(0) = 300 >= maturity 100
            // -> reached (censored past maturity is reached).
            (0, birth(3, 0)),
            (0, birth_site(3, 2, 100, 1_000, 0, 0, 0, 0, 0)), // occupants 2
            // D: censored, horizon(300) - start(0) = 300 < maturity
            // 1,000,000 -> not reached.
            (0, birth(4, 0)),
            (0, birth_site(4, 5, 1_000_000, 1_000, 0, 0, 0, 0, 0)), // occupants 5 -> bucket 3+
        ];
        // A reproduces: appears as parent_a of a later PairedBirth. The
        // child (id 500) has no BirthSite of its own, so it does not
        // enter any site-derived statistic.
        events.push((300, paired_birth(500, 1, 999)));
        let summary = world_cohort(&scan(events));

        assert_eq!(summary.born_reached_maturity, 2, "A and C");
        assert_eq!(summary.born_reproduced, 1, "only A");

        // All four sites share FOOD 1000, so every cut lands at 1000 and
        // every organism falls in quartile 1 (index 0).
        assert_eq!(summary.reached_maturity_food_quartile, [2, 0, 0, 0]);
        assert_eq!(summary.reproduced_food_quartile, [1, 0, 0, 0]);

        // Occupants: A=0 (bucket 0, reached+reproduced), B=1 (bucket 1,
        // neither), C=2 (bucket 2, reached only), D=5 (bucket 3, neither).
        assert_eq!(summary.reached_maturity_occupants, [1, 0, 1, 0]);
        assert_eq!(summary.reproduced_occupants, [1, 0, 0, 0]);
    }

    #[test]
    fn food_excludes_waste_polymer_and_microbial_and_a_huge_waste_site_sorts_as_zero_food() {
        let summary = world_cohort(&scan(vec![
            (0, birth(1, 0)),
            // primordial 0, monomer 0, polymer 555, waste 999_999_999,
            // microbial 777, biomass 0 -> FOOD = 0 despite the huge waste.
            (0, birth_site(1, 1, 10, 0, 0, 555, 999_999_999, 777, 0)),
            (50, death(1)),
        ]));
        assert_eq!(summary.born_site_food_median, 0);
        assert_eq!(summary.waste_median, 999_999_999);
        assert_eq!(summary.polymer_median, 555);
        assert_eq!(summary.microbial_median, 777);
    }
}
