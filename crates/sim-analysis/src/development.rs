//! C14.1: are juveniles measurably constrained? (`lifesim-development-v1`)
//!
//! The census joins the event log's birth/completion/death schedule to the
//! spatial series and classes every sampled inter-sample interval as
//! juvenile or adult by the organism's state at the interval's opening
//! sample: a non-founder is juvenile from birth until its
//! `GrowthCompleted` record and adult after; founders are admitted fully
//! grown and are adult from tick zero; in a world whose ontogeny gate is
//! off everyone is adult at birth and the juvenile columns are zero by
//! construction rather than by accident (the caller states the gate,
//! because the artifacts do not carry the config).
//!
//! Realized speed is displacement between consecutive samples, reported in
//! milli-metres per tick; mortality is deaths per million sampled
//! organism-observations in the same class - opportunity denominators,
//! never raw counts (social-organization review 12.1). Censoring is
//! counted, never imputed. Medians take the LOWER central value, the crate
//! convention D-125 pinned.

use std::collections::BTreeMap;

use sim_core::{Event, EventKind, FP_PER_METER};
use sim_persist::SpatialSample;

pub const DEVELOPMENT_POLICY_VERSION: &str = "lifesim-development-v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DevelopmentError {
    EventsOutOfOrder {
        tick: u64,
    },
    PopulationMismatch {
        tick: u64,
        expected: usize,
        actual: usize,
    },
}

impl std::fmt::Display for DevelopmentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventsOutOfOrder { tick } => {
                write!(formatter, "events out of order at tick {tick}")
            }
            Self::PopulationMismatch {
                tick,
                expected,
                actual,
            } => write!(
                formatter,
                "spatial sample at tick {tick} carries {actual} positions but the \
                 event log implies {expected} alive"
            ),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DevelopmentCensus {
    /// Growth completions recorded in the log.
    pub completions: u32,
    /// Sampled organism-observations in each class (the mortality
    /// denominators).
    pub juvenile_observations: u64,
    pub adult_observations: u64,
    /// Median per-organism mean speed in each class, milli-metres per
    /// tick. `None` when no organism contributed an interval to the class.
    pub juvenile_speed_milli: Option<i64>,
    pub adult_speed_milli: Option<i64>,
    /// Organisms contributing at least one interval to each speed median.
    pub juvenile_speed_organisms: u32,
    pub adult_speed_organisms: u32,
    /// Deaths by the class the organism was in when it died.
    pub juvenile_deaths: u32,
    pub adult_deaths: u32,
    /// Deaths per million sampled observations in the class. `None` at a
    /// zero denominator.
    pub juvenile_mortality_micro: Option<i64>,
    pub adult_mortality_micro: Option<i64>,
    /// Alive at the last sample: censored, counted, never imputed.
    pub censored_alive: u32,
    /// Non-founders that died before completing growth.
    pub died_growing: u32,
}

/// Reduce one world's event log and spatial series to its development
/// census. `ontogeny_enabled` states the arm's gate: when false, every
/// organism is adult at birth and the juvenile columns stay zero.
pub fn development_census(
    events: &[Event],
    samples: &[SpatialSample],
    founder_count: u32,
    ontogeny_enabled: bool,
) -> Result<DevelopmentCensus, DevelopmentError> {
    let mut born_at: BTreeMap<u64, u64> =
        (1..=u64::from(founder_count)).map(|id| (id, 0)).collect();
    let mut died_at: BTreeMap<u64, u64> = BTreeMap::new();
    let mut completed_at: BTreeMap<u64, u64> = BTreeMap::new();
    let mut last_event_tick = 0_u64;
    for event in events {
        if event.tick < last_event_tick {
            return Err(DevelopmentError::EventsOutOfOrder { tick: event.tick });
        }
        last_event_tick = event.tick;
        match event.kind {
            EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                born_at.insert(id, event.tick);
            }
            EventKind::Death { id, .. } => {
                died_at.insert(id, event.tick);
            }
            EventKind::GrowthCompleted { id, .. } => {
                completed_at.insert(id, event.tick);
            }
            _ => {}
        }
    }

    // Juvenile at `tick` means: ontogeny on, not a founder, and no
    // completion record at or before the tick. An organism that never
    // completes stays juvenile to its death or the horizon.
    let is_juvenile = |id: u64, tick: u64| -> bool {
        ontogeny_enabled
            && id > u64::from(founder_count)
            && completed_at.get(&id).is_none_or(|&done| tick < done)
    };

    let mut census = DevelopmentCensus {
        completions: completed_at.len() as u32,
        ..Default::default()
    };

    // Per-organism speed accumulators: (juvenile disp-per-interval sum,
    // juvenile intervals, adult disp-per-interval sum, adult intervals),
    // displacement in fp.
    let mut motion: BTreeMap<u64, (i64, u32, i64, u32)> = BTreeMap::new();
    let mut previous: Option<&SpatialSample> = None;
    let mut alive_before: BTreeMap<u64, (i32, i32)> = BTreeMap::new();
    for sample in samples {
        // The alive set at this sample, in ascending-ID order (births in,
        // deaths out at post-step sample ticks - arrival.rs's join rule).
        let alive_ids: Vec<u64> = born_at
            .iter()
            .filter(|&(id, &born)| {
                born <= sample.tick && died_at.get(id).is_none_or(|&died| died > sample.tick)
            })
            .map(|(&id, _)| id)
            .collect();
        if alive_ids.len() != sample.positions.len() {
            return Err(DevelopmentError::PopulationMismatch {
                tick: sample.tick,
                expected: alive_ids.len(),
                actual: sample.positions.len(),
            });
        }
        let mut alive_now: BTreeMap<u64, (i32, i32)> = BTreeMap::new();
        for (&id, &position) in alive_ids.iter().zip(sample.positions.iter()) {
            if is_juvenile(id, sample.tick) {
                census.juvenile_observations += 1;
            } else {
                census.adult_observations += 1;
            }
            if let (Some(&(px, py)), Some(previous_sample)) = (alive_before.get(&id), previous) {
                let interval = (sample.tick - previous_sample.tick).max(1);
                let dx = i64::from(position.0) - i64::from(px);
                let dy = i64::from(position.1) - i64::from(py);
                // L1 displacement: exact in integers, monotone in real
                // displacement, and identical in both classes, so the
                // juvenile-adult CONTRAST it feeds is fair - an exact-L2
                // would drag an isqrt rounding convention in for no
                // discriminating power.
                let displacement = dx.abs() + dy.abs();
                let entry = motion.entry(id).or_default();
                if is_juvenile(id, previous_sample.tick) {
                    entry.0 += displacement / interval as i64;
                    entry.1 += 1;
                } else {
                    entry.2 += displacement / interval as i64;
                    entry.3 += 1;
                }
            }
            alive_now.insert(id, position);
        }
        alive_before = alive_now;
        previous = Some(sample);
    }

    // Deaths by class at the death tick; censoring counted at the horizon.
    for (&id, &died) in &died_at {
        if is_juvenile(id, died) {
            census.juvenile_deaths += 1;
            if id > u64::from(founder_count) && !completed_at.contains_key(&id) {
                census.died_growing += 1;
            }
        } else {
            census.adult_deaths += 1;
        }
    }
    census.censored_alive = alive_before.len() as u32;

    // Per-organism mean speeds, then the class medians (lower central
    // value), converted fp-per-tick to milli-metres per tick.
    let mut juvenile_speeds: Vec<i64> = Vec::new();
    let mut adult_speeds: Vec<i64> = Vec::new();
    for (_, (juvenile_disp, juvenile_n, adult_disp, adult_n)) in motion {
        if juvenile_n > 0 {
            juvenile_speeds
                .push(juvenile_disp * 1_000 / i64::from(FP_PER_METER) / i64::from(juvenile_n));
        }
        if adult_n > 0 {
            adult_speeds.push(adult_disp * 1_000 / i64::from(FP_PER_METER) / i64::from(adult_n));
        }
    }
    let lower_median = |values: &mut Vec<i64>| -> Option<i64> {
        if values.is_empty() {
            return None;
        }
        values.sort_unstable();
        Some(values[(values.len() - 1) / 2])
    };
    census.juvenile_speed_organisms = juvenile_speeds.len() as u32;
    census.adult_speed_organisms = adult_speeds.len() as u32;
    census.juvenile_speed_milli = lower_median(&mut juvenile_speeds);
    census.adult_speed_milli = lower_median(&mut adult_speeds);

    census.juvenile_mortality_micro = (census.juvenile_observations > 0).then(|| {
        i64::from(census.juvenile_deaths) * 1_000_000 / census.juvenile_observations as i64
    });
    census.adult_mortality_micro = (census.adult_observations > 0)
        .then(|| i64::from(census.adult_deaths) * 1_000_000 / census.adult_observations as i64);

    Ok(census)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn birth(tick: u64, id: u64) -> Event {
        Event {
            tick,
            kind: EventKind::PairedBirth {
                id,
                parent_a: 1,
                parent_b: 2,
                genome_hash: 0,
                invest_a_milli: 0,
                invest_b_milli: 0,
                mutated_trait_genes: 0,
                mutated_neural_genes: 0,
            },
        }
    }

    fn death(tick: u64, id: u64) -> Event {
        Event {
            tick,
            kind: EventKind::Death {
                id,
                cause: sim_core::DeathCause::Starvation,
            },
        }
    }

    fn completed(tick: u64, id: u64) -> Event {
        Event {
            tick,
            kind: EventKind::GrowthCompleted { id, modules: 6 },
        }
    }

    fn sample(tick: u64, positions: Vec<(i32, i32)>) -> SpatialSample {
        SpatialSample { tick, positions }
    }

    /// One founder (id 1, adult throughout, moving 2048 fp = 2 m per
    /// 10-tick interval) and one child (id 2, born tick 5, completing at
    /// tick 25, moving 512 fp while juvenile and 2048 fp when adult).
    /// Every number below is hand-computable from that schedule.
    fn scripted() -> (Vec<Event>, Vec<SpatialSample>) {
        let events = vec![birth(5, 2), completed(25, 2)];
        let samples = vec![
            sample(10, vec![(0, 0), (10_000, 0)]),
            sample(20, vec![(2_048, 0), (10_512, 0)]),
            sample(30, vec![(4_096, 0), (11_024, 0)]),
            sample(40, vec![(6_144, 0), (13_072, 0)]),
        ];
        (events, samples)
    }

    #[test]
    fn juvenile_and_adult_speeds_come_out_exactly_as_scripted() {
        let (events, samples) = scripted();
        let census = development_census(&events, &samples, 1, true).expect("census");
        assert_eq!(census.completions, 1);
        // Founder: three adult intervals of 2048 fp / 10 ticks = 204 fp
        // per tick (truncated) -> 204 * 1000 / 1024 = 199 milli-m/tick.
        assert_eq!(census.adult_speed_milli, Some(199));
        // Child: two juvenile intervals (10..20, 20..30 - the interval is
        // classed by its OPENING sample, and at tick 20 the child is still
        // juvenile) of 512 fp / 10 = 51 fp per tick -> 49 milli; one adult
        // interval (30..40) of 2048 -> 199. Median of juvenile speeds over
        // organisms = the child's 49.
        assert_eq!(census.juvenile_speed_milli, Some(49));
        assert_eq!(census.juvenile_speed_organisms, 1);
        assert_eq!(census.adult_speed_organisms, 2);
        // Observations: founder 4 adult; child 2 juvenile (ticks 10, 20)
        // + 2 adult (30, 40).
        assert_eq!(census.adult_observations, 6);
        assert_eq!(census.juvenile_observations, 2);
        assert_eq!(census.censored_alive, 2);
        assert_eq!(census.juvenile_deaths, 0);
        assert_eq!(census.adult_deaths, 0);
    }

    #[test]
    fn a_death_before_completion_is_a_juvenile_death_and_counted_as_growing() {
        let events = vec![birth(5, 2), death(15, 2)];
        let samples = vec![
            sample(10, vec![(0, 0), (10_000, 0)]),
            sample(20, vec![(2_048, 0)]),
        ];
        let census = development_census(&events, &samples, 1, true).expect("census");
        assert_eq!(census.juvenile_deaths, 1);
        assert_eq!(census.died_growing, 1);
        assert_eq!(census.adult_deaths, 0);
        // One juvenile observation (tick 10) -> mortality 1e6 per
        // observation.
        assert_eq!(census.juvenile_mortality_micro, Some(1_000_000));
    }

    #[test]
    fn the_gate_off_arm_classes_everyone_adult_by_construction() {
        let (events, samples) = scripted();
        let census = development_census(&events, &samples, 1, false).expect("census");
        assert_eq!(census.juvenile_observations, 0);
        assert_eq!(census.juvenile_speed_milli, None);
        assert_eq!(census.adult_observations, 8);
    }

    #[test]
    fn a_population_mismatch_refuses_rather_than_zips() {
        let (events, mut samples) = scripted();
        samples[1].positions.pop();
        let error = development_census(&events, &samples, 1, true).unwrap_err();
        assert!(matches!(
            error,
            DevelopmentError::PopulationMismatch { tick: 20, .. }
        ));
    }
}
