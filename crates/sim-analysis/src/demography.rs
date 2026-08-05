//! World-level demography statistics (`lifesim-demography-index-v1`).
//!
//! Phase 8's campaign criteria are claims about a *distribution* over
//! organisms -- the mix of death causes, evolved lifespan, the relation
//! between per-offspring investment and lifetime offspring count -- and
//! ADR-0022 A5 requires every one of them to be reduced to a single
//! world-level number before analysis. That reduction happens here.
//!
//! Everything comes from the event log, which is the right source for two
//! reasons. It is a complete record of births and deaths, so a lifespan is
//! an arithmetic fact rather than an estimate; and it is written during the
//! run and read after it, so nothing computed here can reach a rule
//! (ADR-0016).
//!
//! One thing this cannot see, and says so rather than guessing: an organism
//! alive at the end of the run has no death event, so its lifespan is
//! censored. Censored individuals are counted and reported, never imputed,
//! and the lifespan statistic is explicitly the median **completed**
//! lifespan. A run that ends with most of its population alive will say so.

use sim_core::{DeathCause, Event, EventKind};
use std::collections::BTreeMap;

pub const DEMOGRAPHY_INDEX_VERSION: &str = "lifesim-demography-index-v1";

/// One world's demographic summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldDemography {
    pub deaths_total: u64,
    pub deaths_starvation: u64,
    pub deaths_old_age: u64,
    pub deaths_damage: u64,
    pub deaths_senescence: u64,
    pub deaths_extrinsic: u64,
    /// Starvation deaths as a fraction of all deaths, milli-units. This is
    /// C8.1's primary quantity.
    pub starvation_share_milli: i64,
    /// Number of causes accounting for at least 5 percent of deaths each.
    /// C8.1 asks for a *mixed* distribution, and a share alone does not say
    /// whether the remainder is spread or concentrated.
    pub causes_above_five_percent: u32,
    /// Median completed lifespan in ticks: C8.5's quantity.
    pub median_lifespan_ticks: u64,
    /// Individuals still alive at the end, whose lifespan is censored.
    pub censored_individuals: u64,
    pub completed_lifespans: u64,
    /// Spearman correlation between a parent's mean per-offspring
    /// investment and its lifetime offspring count, milli-units. C8.6
    /// predicts this is negative.
    pub investment_offspring_rho_milli: i64,
    /// Parents with at least one recorded paired birth.
    pub parents_observed: u64,
}

/// Reduce one world's event log to its demographic summary.
pub fn world_demography(events: &[Event]) -> WorldDemography {
    let mut birth_tick: BTreeMap<u64, u64> = BTreeMap::new();
    let mut lifespans: Vec<u64> = Vec::new();
    let mut summary = WorldDemography::default();
    // Per parent: total invested milli-EU and number of offspring.
    let mut parent_investment: BTreeMap<u64, (i128, u64)> = BTreeMap::new();

    for event in events.iter().map(|event| event.kind) {
        match event {
            EventKind::Birth { id, .. } => {
                birth_tick.insert(id, 0);
            }
            EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                invest_a_milli,
                invest_b_milli,
                ..
            } => {
                birth_tick.insert(id, 0);
                for (parent, invested) in [(parent_a, invest_a_milli), (parent_b, invest_b_milli)] {
                    let entry = parent_investment.entry(parent).or_insert((0, 0));
                    entry.0 += i128::from(invested);
                    entry.1 += 1;
                }
            }
            EventKind::Death { id, cause } => {
                summary.deaths_total += 1;
                match cause {
                    DeathCause::Starvation => summary.deaths_starvation += 1,
                    DeathCause::OldAge => summary.deaths_old_age += 1,
                    DeathCause::Damage => summary.deaths_damage += 1,
                    DeathCause::Senescence => summary.deaths_senescence += 1,
                    DeathCause::Extrinsic => summary.deaths_extrinsic += 1,
                }
                if let Some(born) = birth_tick.remove(&id) {
                    let _ = born;
                }
            }
            _ => {}
        }
    }

    // Lifespans need the ticks, which the iterator above discards; a second
    // pass keyed on tick is clearer than threading it through the match.
    let mut born_at: BTreeMap<u64, u64> = BTreeMap::new();
    for (tick, kind) in events.iter().map(|event| (event.tick, event.kind)) {
        match kind {
            EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                born_at.insert(id, tick);
            }
            EventKind::Death { id, .. } => {
                if let Some(born) = born_at.remove(&id) {
                    lifespans.push(tick.saturating_sub(born));
                }
            }
            _ => {}
        }
    }
    summary.censored_individuals = born_at.len() as u64;
    summary.completed_lifespans = lifespans.len() as u64;
    lifespans.sort_unstable();
    summary.median_lifespan_ticks = if lifespans.is_empty() {
        0
    } else {
        lifespans[(lifespans.len() - 1) / 2]
    };

    if summary.deaths_total > 0 {
        summary.starvation_share_milli = (i128::from(summary.deaths_starvation) * 1_000
            / i128::from(summary.deaths_total)) as i64;
        let threshold = summary.deaths_total / 20; // 5 percent
        summary.causes_above_five_percent = [
            summary.deaths_starvation,
            summary.deaths_old_age,
            summary.deaths_damage,
            summary.deaths_senescence,
            summary.deaths_extrinsic,
        ]
        .iter()
        .filter(|count| **count > threshold)
        .count() as u32;
    }

    // C8.6: mean per-offspring investment against lifetime offspring count,
    // one point per parent, reduced to a single rank correlation.
    let points: Vec<(i64, i64)> = parent_investment
        .values()
        .filter(|(_, count)| *count > 0)
        .map(|(total, count)| ((total / i128::from(*count)) as i64, *count as i64))
        .collect();
    summary.parents_observed = points.len() as u64;
    summary.investment_offspring_rho_milli = spearman_milli(&points);
    summary
}

/// Spearman rank correlation in milli-units. Ties take average ranks, which
/// matters here because lifetime offspring counts are small integers and
/// most parents share one.
pub fn spearman_milli(points: &[(i64, i64)]) -> i64 {
    let n = points.len();
    if n < 3 {
        return 0;
    }
    let rank_of = |values: &[i64]| -> Vec<f64> {
        let mut order: Vec<usize> = (0..values.len()).collect();
        order.sort_by_key(|&index| values[index]);
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
    };
    let xs: Vec<i64> = points.iter().map(|(x, _)| *x).collect();
    let ys: Vec<i64> = points.iter().map(|(_, y)| *y).collect();
    let rx = rank_of(&xs);
    let ry = rank_of(&ys);
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

/// Thermal-matching statistic for C8.7.
///
/// For every living organism at the snapshot tick, pair the temperature its
/// thermal-preference gene prefers with the temperature of the cell it is
/// actually standing in, and reduce the pairing to one rank correlation.
///
/// The control that makes this measurable is that under a
/// thermoregulation-disabled condition the gene is inert, so the same
/// statistic computed there must show no correlation. A positive number
/// with no control attached would be indistinguishable from the terrain
/// happening to correlate with wherever organisms end up.
pub fn thermal_match_rho_milli(world: &sim_core::World) -> Option<(i64, u64)> {
    let cells_x = world.terrain().cells_x;
    let cell_fp = i64::from(world.config().cell_size_m) * i64::from(sim_core::FP_PER_METER);
    let physiology = world.config().physiology;
    let mut points = Vec::new();
    for id in world.organism_ids_view() {
        let Some(phase2) = world.organism_detail(*id).and_then(|detail| detail.phase2) else {
            continue;
        };
        let detail = world.organism_detail(*id).expect("detail exists");
        let cell_x = (i64::from(detail.x_fp) / cell_fp).max(0) as u32;
        let cell_y = (i64::from(detail.y_fp) / cell_fp).max(0) as u32;
        let cell = (cell_y as usize) * (cells_x as usize) + cell_x as usize;
        // `None` here means the world has no temperature field at all, in
        // which case the statistic is undefined rather than zero -- so the
        // whole call returns `None` and the caller reports it as absent.
        let actual = world.temperature_milli(cell)?;
        let preferred =
            crate::preferred_temperature_milli(&physiology, phase2.phenotype.thermal_pref_milli);
        points.push((i64::from(preferred), i64::from(actual)));
    }
    let observed = points.len() as u64;
    Some((spearman_milli(&points), observed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::EventKind;

    fn scan(events: Vec<(u64, EventKind)>) -> Vec<Event> {
        events
            .into_iter()
            .map(|(tick, kind)| Event { tick, kind })
            .collect()
    }

    fn birth(id: u64) -> EventKind {
        EventKind::Birth { id, parent_id: 0 }
    }

    fn death(id: u64, cause: DeathCause) -> EventKind {
        EventKind::Death { id, cause }
    }

    #[test]
    fn the_starvation_share_and_mix_are_computed_from_the_causes() {
        let summary = world_demography(&scan(vec![
            (1, birth(1)),
            (1, birth(2)),
            (1, birth(3)),
            (1, birth(4)),
            (10, death(1, DeathCause::Starvation)),
            (10, death(2, DeathCause::Senescence)),
            (10, death(3, DeathCause::Extrinsic)),
            (10, death(4, DeathCause::Damage)),
        ]));
        assert_eq!(summary.deaths_total, 4);
        assert_eq!(summary.starvation_share_milli, 250);
        assert_eq!(summary.causes_above_five_percent, 4);
    }

    #[test]
    fn a_starvation_dominated_world_is_visible_as_one() {
        // The Phase 2 pathology this whole phase exists to fix: one cause
        // takes essentially everything and the mix count collapses to one.
        let mut events = vec![(1_u64, birth(0))];
        for id in 1..=100_u64 {
            events.push((1, birth(id)));
            events.push((10, death(id, DeathCause::Starvation)));
        }
        let summary = world_demography(&scan(events));
        assert_eq!(summary.starvation_share_milli, 1_000);
        assert_eq!(summary.causes_above_five_percent, 1);
    }

    #[test]
    fn lifespan_is_completed_only_and_censoring_is_counted_not_imputed() {
        let summary = world_demography(&scan(vec![
            (10, birth(1)),
            (10, birth(2)),
            (10, birth(3)),
            (110, death(1, DeathCause::Extrinsic)),
            (310, death(2, DeathCause::Extrinsic)),
            // Organism 3 never dies: censored, never imputed.
        ]));
        assert_eq!(summary.completed_lifespans, 2);
        assert_eq!(summary.censored_individuals, 1);
        assert_eq!(summary.median_lifespan_ticks, 100);
    }

    #[test]
    fn the_investment_correlation_recovers_a_sign_it_was_given() {
        // Parents who invest more per offspring have fewer of them. The
        // statistic must report that as negative; C8.6 predicts the sign
        // and this checks the estimator can see one.
        let mut events = Vec::new();
        let mut child = 1_000_u64;
        for parent in 1..=20_u64 {
            let offspring = 21 - parent; // 20 down to 1
            let invest = 1_000 + (parent as i64) * 100; // rising
            for _ in 0..offspring {
                child += 1;
                events.push((
                    10,
                    EventKind::PairedBirth {
                        id: child,
                        parent_a: parent,
                        parent_b: parent + 100,
                        genome_hash: 0,
                        invest_a_milli: invest,
                        invest_b_milli: invest,
                        mutated_trait_genes: 0,
                        mutated_neural_genes: 0,
                    },
                ));
            }
        }
        let summary = world_demography(&scan(events));
        assert!(summary.parents_observed >= 20);
        assert!(
            summary.investment_offspring_rho_milli < -500,
            "expected a strong negative correlation, got {}",
            summary.investment_offspring_rho_milli
        );
    }

    #[test]
    fn spearman_handles_ties_and_degenerate_input() {
        assert_eq!(spearman_milli(&[]), 0);
        assert_eq!(spearman_milli(&[(1, 1), (2, 2)]), 0);
        // Every y identical: no variance, so no correlation rather than a
        // division by zero.
        assert_eq!(spearman_milli(&[(1, 5), (2, 5), (3, 5), (4, 5)]), 0);
        // Perfect monotone increase.
        let rho = spearman_milli(&[(1, 1), (2, 2), (3, 3), (4, 4), (5, 5)]);
        assert!((rho - 1_000).abs() <= 1, "got {rho}");
        let rho = spearman_milli(&[(1, 5), (2, 4), (3, 3), (4, 2), (5, 1)]);
        assert!((rho + 1_000).abs() <= 1, "got {rho}");
    }
}
