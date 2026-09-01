//! C13.1's per-world reduction: naive-individual arrival at a resource
//! patch (`lifesim-arrival-detector-v1`).
//!
//! The criterion reads "naive individuals (born after tick t and never
//! personally present at resource patch P) reach P measurably faster under
//! A than under C". This module computes the per-world half of that claim:
//! given one world's event log and its spatial sample series, it produces
//! the naive cohort, each member's arrival latency or censoring, and the
//! world-level summary (ADR-0022 A5: the world is the replicate). The
//! A-versus-C and A-versus-D comparisons are made across worlds by the
//! campaign analysis, not here.
//!
//! **The patch is an analysis input, never a kernel concept** (ADR-0016).
//! The kernel has no notion of "patch P"; the analyst names a circle in the
//! same fixed-point frame the samples use, and nothing computed here can
//! reach a rule.
//!
//! What "present" can mean here is bounded by the instrument: positions
//! exist only at sample ticks, so presence between samples is invisible.
//! The census carries `sample_interval_ticks` so every downstream report
//! states its own temporal resolution rather than implying continuity
//! (the `SpatialLogInfo` field exists for exactly this reason). Three
//! consequences are made explicit rather than left to leak:
//!
//! - An arrival is the first *sample* tick at which the organism is inside
//!   the patch; the true crossing happened at most one interval earlier.
//! - An organism whose very first observation is already inside the patch
//!   was, as far as the instrument can see, present from birth. The
//!   criterion's own wording ("never personally present") excludes it from
//!   the naive cohort; it is counted in `born_in_patch` instead of being
//!   silently dropped.
//! - A naive individual that dies unarrived is censored at its death tick;
//!   one alive and unarrived at the last sample is censored there. Censored
//!   individuals are counted, never imputed, and the latency statistic is
//!   explicitly the median **completed** latency (the demography module's
//!   standing convention for censoring).
//!
//! The join between the two artifacts is the dangerous seam. Samples carry
//! positions in ascending entity-ID order but not the IDs themselves; the
//! IDs come from replaying the event log (founders are IDs
//! `1..=founder_count` by the kernel's allocation rule; every later
//! organism has a `Birth` or `PairedBirth` event; every removal has a
//! `Death` event, applied births-before-deaths within a tick). If the
//! reconstructed living count at a sample tick does not equal the sample's
//! position count, the detector refuses with the tick named - it never
//! repairs, because a plausible-looking repair would silently misassign
//! every later track to the wrong organism. The ascending-ID claim itself
//! is pinned against the kernel by a test in this module, so a future
//! storage-order change breaks a named dependent instead of an analysis.

use sim_core::{Event, EventKind};
use sim_persist::SpatialSample;
use std::collections::BTreeMap;

pub const ARRIVAL_DETECTOR_VERSION: &str = "lifesim-arrival-detector-v1";

/// The analyst's circle, in the samples' fixed-point frame. Membership is
/// inclusive: a point at exactly `radius_fp` is inside, so the boundary
/// cannot be one short (the trap every decoder range here has hit once).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PatchSpec {
    pub x_fp: i32,
    pub y_fp: i32,
    pub radius_fp: i32,
}

impl PatchSpec {
    fn contains(&self, x_fp: i32, y_fp: i32) -> bool {
        let dx = i64::from(x_fp) - i64::from(self.x_fp);
        let dy = i64::from(y_fp) - i64::from(self.y_fp);
        let radius = i64::from(self.radius_fp);
        dx * dx + dy * dy <= radius * radius
    }
}

/// One organism's arrival record. Retained per individual because the
/// campaign analysis needs more than the world summary: the age confound in
/// C13.1 (naive cohorts under different conditions are born at different
/// ticks) is handled downstream from exactly these fields.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct IndividualArrival {
    pub id: u64,
    /// Birth tick; 0 for founders.
    pub born_tick: u64,
    /// First sample tick at which this organism was observed inside the
    /// patch, whether or not it is naive.
    pub first_inside_tick: Option<u64>,
    /// First sample tick at which this organism was observed at all.
    pub first_seen_tick: Option<u64>,
    /// Death tick, or the last sample tick that covered it.
    pub observed_until: u64,
    /// Born after the naive threshold, first observed outside the patch.
    pub naive: bool,
}

/// One world's arrival summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ArrivalCensus {
    pub sample_interval_ticks: u32,
    /// Naive cohort size: born strictly after the threshold tick, first
    /// observed outside the patch, observed at least once.
    pub naive_total: u64,
    pub naive_arrived: u64,
    pub naive_censored: u64,
    /// Organisms born after the threshold whose first observation was
    /// already inside the patch: present from birth as far as the
    /// instrument can see, so excluded from the cohort - but counted, so a
    /// world where the patch sits on the nursery is visibly that world.
    pub born_in_patch: u64,
    /// Born after the threshold but dead before any sample covered them.
    /// Invisible to the instrument; counted, never guessed at.
    pub never_observed: u64,
    /// Median completed latency (first inside tick minus birth tick) over
    /// naive arrivers. None when nobody arrived.
    pub median_naive_latency_ticks: Option<u64>,
    /// `naive_arrived * 1000 / naive_total`; 0 when the cohort is empty.
    pub arrival_fraction_milli: i64,
    pub individuals: Vec<IndividualArrival>,
}

/// The join refused. Every variant names where, because a repaired join
/// would misassign tracks silently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ArrivalError {
    /// Reconstructed living population differs from the sample's position
    /// count: the founder count is wrong, the event log is truncated, or
    /// the artifacts are from different worlds.
    PopulationMismatch {
        tick: u64,
        reconstructed: usize,
        sampled: usize,
    },
    /// Sample ticks must be strictly increasing.
    SamplesOutOfOrder { tick: u64 },
    /// An event log tick ran backwards; the replay would be wrong.
    EventsOutOfOrder { tick: u64 },
}

/// Reduce one world's event log and spatial series to its arrival census.
///
/// `naive_after_tick` is the criterion's t: naive means born **strictly
/// after** it. Founders (born tick 0) are never naive.
pub fn arrival_census(
    events: &[Event],
    samples: &[SpatialSample],
    founder_count: u32,
    sample_interval_ticks: u32,
    patch: PatchSpec,
    naive_after_tick: u64,
) -> Result<ArrivalCensus, ArrivalError> {
    // Birth and death schedules from the log. Founders exist from tick 0 by
    // the kernel's allocation rule and have no birth event.
    let mut born_at: BTreeMap<u64, u64> =
        (1..=u64::from(founder_count)).map(|id| (id, 0)).collect();
    let mut died_at: BTreeMap<u64, u64> = BTreeMap::new();
    let mut last_event_tick = 0_u64;
    for event in events {
        if event.tick < last_event_tick {
            return Err(ArrivalError::EventsOutOfOrder { tick: event.tick });
        }
        last_event_tick = event.tick;
        match event.kind {
            EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                born_at.insert(id, event.tick);
            }
            EventKind::Death { id, .. } => {
                died_at.insert(id, event.tick);
            }
            _ => {}
        }
    }

    // Replay the living set forward through the sample ticks. A sample is
    // taken after the tick's step, so a birth at the sample tick is present
    // and a death at it is absent.
    let mut alive: BTreeMap<u64, IndividualArrival> = BTreeMap::new();
    let mut finished: Vec<IndividualArrival> = Vec::new();
    let mut births: Vec<(u64, u64)> = born_at.iter().map(|(id, tick)| (*id, *tick)).collect();
    births.sort_unstable_by_key(|&(id, tick)| (tick, id));
    let mut next_birth = 0_usize;
    let mut previous_sample_tick: Option<u64> = None;

    for sample in samples {
        if previous_sample_tick.is_some_and(|previous| sample.tick <= previous) {
            return Err(ArrivalError::SamplesOutOfOrder { tick: sample.tick });
        }
        previous_sample_tick = Some(sample.tick);

        // Births first, then deaths: an organism born and dead inside the
        // same inter-sample gap passes through `alive` and out again
        // without ever being joined to a position, which is the truth.
        while next_birth < births.len() && births[next_birth].1 <= sample.tick {
            let (id, born_tick) = births[next_birth];
            next_birth += 1;
            alive.insert(
                id,
                IndividualArrival {
                    id,
                    born_tick,
                    first_inside_tick: None,
                    first_seen_tick: None,
                    observed_until: born_tick,
                    naive: false,
                },
            );
        }
        let dead_now: Vec<u64> = alive
            .iter()
            .filter(|(id, _)| died_at.get(id).is_some_and(|&at| at <= sample.tick))
            .map(|(&id, _)| id)
            .collect();
        for id in dead_now {
            let mut record = alive.remove(&id).expect("listed from alive");
            record.observed_until = died_at[&id];
            finished.push(record);
        }

        if alive.len() != sample.positions.len() {
            return Err(ArrivalError::PopulationMismatch {
                tick: sample.tick,
                reconstructed: alive.len(),
                sampled: sample.positions.len(),
            });
        }
        for (record, &(x_fp, y_fp)) in alive.values_mut().zip(sample.positions.iter()) {
            if record.first_seen_tick.is_none() {
                record.first_seen_tick = Some(sample.tick);
            }
            record.observed_until = sample.tick;
            if record.first_inside_tick.is_none() && patch.contains(x_fp, y_fp) {
                record.first_inside_tick = Some(sample.tick);
            }
        }
    }

    // Organisms born after the last sample, or still alive at it, close at
    // their latest known point.
    while next_birth < births.len() {
        let (id, born_tick) = births[next_birth];
        next_birth += 1;
        finished.push(IndividualArrival {
            id,
            born_tick,
            first_inside_tick: None,
            first_seen_tick: None,
            observed_until: died_at.get(&id).copied().unwrap_or(born_tick),
            naive: false,
        });
    }
    for (id, mut record) in std::mem::take(&mut alive) {
        if let Some(&at) = died_at.get(&id) {
            record.observed_until = at;
        }
        finished.push(record);
    }
    finished.sort_unstable_by_key(|record| record.id);

    let mut census = ArrivalCensus {
        sample_interval_ticks,
        ..ArrivalCensus::default()
    };
    let mut latencies: Vec<u64> = Vec::new();
    for record in &mut finished {
        if record.born_tick <= naive_after_tick {
            continue;
        }
        match (record.first_seen_tick, record.first_inside_tick) {
            (None, _) => census.never_observed += 1,
            (Some(seen), Some(inside)) if inside == seen => census.born_in_patch += 1,
            (Some(_), inside) => {
                record.naive = true;
                census.naive_total += 1;
                match inside {
                    Some(tick) => {
                        census.naive_arrived += 1;
                        latencies.push(tick - record.born_tick);
                    }
                    None => census.naive_censored += 1,
                }
            }
        }
    }
    latencies.sort_unstable();
    census.median_naive_latency_ticks = if latencies.is_empty() {
        None
    } else {
        Some(latencies[latencies.len() / 2])
    };
    census.arrival_fraction_milli = if census.naive_total == 0 {
        0
    } else {
        (census.naive_arrived as i64 * 1000) / census.naive_total as i64
    };
    census.individuals = finished;
    Ok(census)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::{DeathCause, SimConfig, World};

    fn event(tick: u64, kind: EventKind) -> Event {
        Event { tick, kind }
    }

    fn birth(tick: u64, id: u64) -> Event {
        event(tick, EventKind::Birth { id, parent_id: 1 })
    }

    fn death(tick: u64, id: u64) -> Event {
        event(
            tick,
            EventKind::Death {
                id,
                cause: DeathCause::Starvation,
            },
        )
    }

    fn sample(tick: u64, positions: Vec<(i32, i32)>) -> SpatialSample {
        SpatialSample { tick, positions }
    }

    const PATCH: PatchSpec = PatchSpec {
        x_fp: 1_000,
        y_fp: 1_000,
        radius_fp: 100,
    };
    const FAR: (i32, i32) = (0, 0);
    const IN: (i32, i32) = (1_000, 1_000);

    /// Scripted ground truth, recovered exactly (Gate E). Two founders stay
    /// out; organism 3 is naive and arrives with a known latency; organism
    /// 4 is naive and dies unarrived; organism 5 is born inside the patch.
    #[test]
    fn the_detector_recovers_scripted_arrivals_exactly() {
        let events = vec![birth(15, 3), birth(15, 4), birth(25, 5), death(38, 4)];
        let samples = vec![
            sample(10, vec![FAR, FAR]),
            sample(20, vec![FAR, FAR, FAR, FAR]),
            sample(30, vec![FAR, FAR, FAR, FAR, IN]),
            sample(40, vec![FAR, FAR, IN, IN]),
        ];
        let census = arrival_census(&events, &samples, 2, 10, PATCH, 10).expect("join holds");
        assert_eq!(census.naive_total, 2);
        assert_eq!(census.naive_arrived, 1);
        assert_eq!(census.naive_censored, 1);
        assert_eq!(census.born_in_patch, 1);
        assert_eq!(census.never_observed, 0);
        // Organism 3: born 15, first inside at sample 40 -> latency 25.
        assert_eq!(census.median_naive_latency_ticks, Some(25));
        assert_eq!(census.arrival_fraction_milli, 500);
        let three = census.individuals.iter().find(|r| r.id == 3).expect("3");
        assert_eq!(
            (three.naive, three.first_inside_tick, three.first_seen_tick),
            (true, Some(40), Some(20))
        );
        let four = census.individuals.iter().find(|r| r.id == 4).expect("4");
        assert_eq!(
            (four.naive, four.first_inside_tick, four.observed_until),
            (true, None, 38)
        );
        // Founders are never naive, arrivals or not.
        assert!(
            census
                .individuals
                .iter()
                .filter(|r| r.id <= 2)
                .all(|r| !r.naive)
        );
    }

    /// The membership boundary is inclusive at exactly the radius and
    /// excludes one fixed-point unit past it - the range that is otherwise
    /// always one short.
    #[test]
    fn the_patch_boundary_is_inclusive_and_one_unit_past_it_is_not() {
        let on_edge = (PATCH.x_fp + PATCH.radius_fp, PATCH.y_fp);
        let past_edge = (PATCH.x_fp + PATCH.radius_fp + 1, PATCH.y_fp);
        for (position, arrives) in [(on_edge, true), (past_edge, false)] {
            let events = vec![birth(15, 2)];
            let samples = vec![sample(20, vec![FAR, FAR]), sample(30, vec![FAR, position])];
            let census = arrival_census(&events, &samples, 1, 10, PATCH, 10).expect("join holds");
            assert_eq!(census.naive_arrived, u64::from(arrives), "at {position:?}");
        }
    }

    /// Born strictly after the threshold: an organism born exactly at t is
    /// not naive, one tick later is.
    #[test]
    fn born_exactly_at_the_threshold_is_not_naive() {
        for (born, naive_total) in [(10, 0), (11, 1)] {
            let events = vec![birth(born, 2)];
            let samples = vec![sample(20, vec![FAR, FAR])];
            let census = arrival_census(&events, &samples, 1, 10, PATCH, 10).expect("join holds");
            assert_eq!(census.naive_total, naive_total, "born at {born}");
        }
    }

    /// Sample-tick boundary semantics: a death at exactly the sample tick
    /// is absent from it, a birth at exactly the sample tick is present -
    /// the post-step convention. Wrong either way, the counts differ and
    /// the join must refuse.
    #[test]
    fn the_sample_tick_boundary_is_births_in_deaths_out() {
        let events = vec![birth(20, 2), death(30, 1)];
        let samples = vec![
            sample(10, vec![FAR]),
            sample(20, vec![FAR, FAR]),
            sample(30, vec![FAR]),
        ];
        let census = arrival_census(&events, &samples, 1, 10, PATCH, 0).expect("join holds");
        let founder = census.individuals.iter().find(|r| r.id == 1).expect("1");
        assert_eq!(founder.observed_until, 30);
        // The complementary wrong-count case refuses by name.
        let torn = vec![sample(10, vec![FAR]), sample(20, vec![FAR])];
        assert_eq!(
            arrival_census(&events, &torn, 1, 10, PATCH, 0),
            Err(ArrivalError::PopulationMismatch {
                tick: 20,
                reconstructed: 2,
                sampled: 1
            })
        );
    }

    /// An organism born and dead between two samples is never joined to a
    /// position and lands in `never_observed`, not in the cohort.
    #[test]
    fn a_life_between_samples_is_counted_never_guessed() {
        let events = vec![birth(21, 2), death(28, 2)];
        let samples = vec![sample(20, vec![FAR]), sample(30, vec![FAR])];
        let census = arrival_census(&events, &samples, 1, 10, PATCH, 0).expect("join holds");
        assert_eq!((census.never_observed, census.naive_total), (1, 0));
    }

    /// The ascending-entity-ID order the join depends on, pinned against
    /// the real kernel: at every tick of a world with deaths compacting the
    /// arrays, `render_entities_in` yields ascending IDs that equal the
    /// event-log reconstruction. If storage order ever stops being ID
    /// order, this fails and names the arrival join as the dependent.
    #[test]
    fn the_kernel_samples_ascending_ids_that_match_the_event_log() {
        let mut config = SimConfig::phase1_default(0x1373_0001);
        config.initial_organisms = 40;
        config.physiology.enabled = true;
        config.physiology.extrinsic_hazard_q16_per_s = 20_000;
        config.validate().expect("valid");
        let mut world = World::new(config).expect("world");
        let mut alive: std::collections::BTreeSet<u64> = (1..=40).collect();
        let mut deaths = 0_u64;
        let mut buffer = Vec::new();
        for _ in 0..400 {
            world.step();
            for event in world.events() {
                match event.kind {
                    EventKind::Birth { id, .. } | EventKind::PairedBirth { id, .. } => {
                        alive.insert(id);
                    }
                    EventKind::Death { id, .. } => {
                        deaths += 1;
                        assert!(alive.remove(&id), "death of an unknown organism");
                    }
                    _ => {}
                }
            }
            world.render_entities_in(i32::MIN, i32::MIN, i32::MAX, i32::MAX, &mut buffer);
            let sampled: Vec<u64> = buffer.iter().map(|entity| entity.id).collect();
            let mut ascending = sampled.clone();
            ascending.sort_unstable();
            assert_eq!(sampled, ascending, "sample order is not ID order");
            assert_eq!(
                sampled,
                alive.iter().copied().collect::<Vec<u64>>(),
                "event-log reconstruction diverged from the kernel"
            );
        }
        assert!(
            deaths > 0,
            "the hazard never killed, so compaction never ran"
        );
    }
}
