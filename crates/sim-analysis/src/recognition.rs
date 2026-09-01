//! C13.7's per-world reduction: recognition from cues
//! (`lifesim-recognition-v1`).
//!
//! The criterion: organisms discriminate between conspecifics in a way
//! that correlates with phenotype similarity, with no genotype-distance
//! channel available; under a scrambled control the discrimination
//! disappears. The discriminating behavior this classifier reads is the
//! one behavior the artifacts record pair-directed: attack. For each
//! `Damage` event, the attacker had a set of available conspecifics (the
//! organisms within the candidate radius of it at the nearest spatial
//! sample), and the question is whether the chosen target's cue-visible
//! phenotype stands out from that set - not whether attacks happen at
//! all.
//!
//! Phenotype comes from the `PhenotypeAtBirth` record (event schema 8):
//! body scale, the static cue-visible value, known for every *born*
//! organism whatever its lifespan. Founders carry no record and are
//! excluded by construction (they are a vanishing share of any window
//! this classifier is pointed at); every exclusion is counted.
//!
//! Per attack with at least one candidate: `delta = mean over candidates
//! of |scale_attacker - scale_candidate| - |scale_attacker -
//! scale_target|`, milli - positive when the chosen target is more
//! similar to the attacker than the average alternative. The world
//! statistic is the mean delta over usable attacks. The scrambled
//! control is a permutation of the scale values over the recorded
//! organisms (the kernel's `Analysis` stream, keyed on the recorded
//! analysis seed), recomputed through the same attack structure per
//! shuffle: under scrambled cues the statistic's magnitude falls inside
//! the null band by construction, which is the criterion's own control
//! stated as a computation. Nothing here decides; the addendum
//! pre-registers the reading (ADR-0016).

use crate::arrival::ArrivalError;
use crate::fidelity::spatial_positions_by_id;
use sim_core::{Event, EventKind, RngSystem, named_random};
use sim_persist::SpatialSample;
use std::collections::BTreeMap;

pub const RECOGNITION_VERSION: &str = "lifesim-recognition-v1";

/// Analysis parameters, named so a report can echo every one.
#[derive(Clone, Copy, Debug)]
pub struct RecognitionPlan {
    pub founder_count: u32,
    /// Candidate radius around the attacker, fixed-point.
    pub candidate_radius_fp: i32,
    /// Scale-permutation shuffles for the null band.
    pub shuffles: u32,
    pub analysis_seed: u64,
}

/// One world's recognition summary.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WorldRecognition {
    pub attacks_total: u64,
    /// Attacks where attacker, target, and at least one candidate all
    /// carry phenotype records and positions.
    pub attacks_usable: u64,
    pub attacks_missing_phenotype: u64,
    pub attacks_no_candidates: u64,
    /// Mean per-attack delta, milli: positive = targets more similar to
    /// their attackers than the average available alternative. None when
    /// no attack was usable.
    pub mean_delta_milli: Option<i64>,
    /// The null band: p95 of |mean delta| over scale-permutation
    /// shuffles.
    pub null_p95_abs_milli: i64,
    pub shuffles: u32,
}

fn mean_delta(attacks: &[(i64, i64, Vec<i64>)]) -> Option<i64> {
    if attacks.is_empty() {
        return None;
    }
    let total: i128 = attacks
        .iter()
        .map(|(attacker, target, candidates)| {
            let candidate_mean: i128 = candidates
                .iter()
                .map(|&candidate| i128::from((attacker - candidate).abs()))
                .sum::<i128>()
                / candidates.len() as i128;
            candidate_mean - i128::from((attacker - target).abs())
        })
        .sum();
    Some((total / attacks.len() as i128) as i64)
}

/// Reduce one world's artifacts to its recognition summary.
pub fn world_recognition(
    events: &[Event],
    spatial: &[SpatialSample],
    plan: &RecognitionPlan,
) -> Result<WorldRecognition, ArrivalError> {
    let mut summary = WorldRecognition {
        shuffles: plan.shuffles,
        ..WorldRecognition::default()
    };
    let positions = spatial_positions_by_id(events, spatial, plan.founder_count)?;

    let mut scales: BTreeMap<u64, i64> = BTreeMap::new();
    for event in events {
        if let EventKind::PhenotypeAtBirth {
            id,
            body_scale_milli,
            ..
        } = event.kind
        {
            scales.insert(id, body_scale_milli);
        }
    }

    // Per usable attack: attacker id, target id, candidate ids - the
    // STRUCTURE, so the scramble can rewalk it with permuted values.
    let radius = i64::from(plan.candidate_radius_fp);
    let radius_squared = radius * radius;
    let mut usable: Vec<(u64, u64, Vec<u64>)> = Vec::new();
    for event in events {
        let EventKind::Damage {
            attacker, target, ..
        } = event.kind
        else {
            continue;
        };
        summary.attacks_total += 1;
        if !scales.contains_key(&attacker) || !scales.contains_key(&target) {
            summary.attacks_missing_phenotype += 1;
            continue;
        }
        let Some((_, sample)) = positions
            .iter()
            .min_by_key(|(tick, _)| tick.abs_diff(event.tick))
        else {
            summary.attacks_no_candidates += 1;
            continue;
        };
        let Some(&(ax, ay)) = sample.get(&attacker) else {
            summary.attacks_no_candidates += 1;
            continue;
        };
        let candidates: Vec<u64> = sample
            .iter()
            .filter(|entry| *entry.0 != attacker && scales.contains_key(entry.0))
            .filter(|entry| {
                let (x, y) = *entry.1;
                let dx = i64::from(x) - i64::from(ax);
                let dy = i64::from(y) - i64::from(ay);
                dx * dx + dy * dy <= radius_squared
            })
            .map(|(&id, _)| id)
            .collect();
        if candidates.is_empty() {
            summary.attacks_no_candidates += 1;
            continue;
        }
        summary.attacks_usable += 1;
        usable.push((attacker, target, candidates));
    }

    let score = |lookup: &BTreeMap<u64, i64>| -> Option<i64> {
        let resolved: Vec<(i64, i64, Vec<i64>)> = usable
            .iter()
            .map(|(attacker, target, candidates)| {
                (
                    lookup[attacker],
                    lookup[target],
                    candidates.iter().map(|id| lookup[id]).collect(),
                )
            })
            .collect();
        mean_delta(&resolved)
    };

    summary.mean_delta_milli = score(&scales);

    // The scrambled control: permute the recorded scale values over the
    // recorded organisms, once per shuffle. The attack structure and the
    // candidate sets are untouched - only which body wears which scale
    // moves.
    let ids: Vec<u64> = scales.keys().copied().collect();
    let values: Vec<i64> = ids.iter().map(|id| scales[id]).collect();
    let mut null_abs: Vec<i64> = Vec::with_capacity(plan.shuffles as usize);
    for shuffle in 0..plan.shuffles {
        let mut permuted = values.clone();
        for index in (1..permuted.len()).rev() {
            let draw = named_random(
                plan.analysis_seed,
                u64::from(shuffle),
                RngSystem::Analysis,
                0x1373_0007,
                index as u32,
            );
            permuted.swap(index, (draw % (index as u64 + 1)) as usize);
        }
        let scrambled: BTreeMap<u64, i64> = ids.iter().copied().zip(permuted).collect();
        null_abs.push(score(&scrambled).map_or(0, i64::abs));
    }
    null_abs.sort_unstable();
    summary.null_p95_abs_milli = if null_abs.is_empty() {
        0
    } else {
        null_abs[(null_abs.len() * 95).div_ceil(100).saturating_sub(1)]
    };
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_plan() -> RecognitionPlan {
        RecognitionPlan {
            founder_count: 0,
            candidate_radius_fp: 2_000,
            shuffles: 40,
            analysis_seed: 0x1373_0007,
        }
    }

    fn phenotype(tick: u64, id: u64, scale: i64) -> Event {
        Event {
            tick,
            kind: EventKind::PhenotypeAtBirth {
                id,
                body_scale_milli: scale,
                max_speed_milli: 1_000,
            },
        }
    }

    fn birth(tick: u64, id: u64) -> Event {
        Event {
            tick,
            kind: EventKind::Birth { id, parent_id: 0 },
        }
    }

    fn damage(tick: u64, attacker: u64, target: u64) -> Event {
        Event {
            tick,
            kind: EventKind::Damage {
                attacker,
                target,
                raw_milli: 100,
                applied_milli: 100,
                health_milli: 500,
            },
        }
    }

    /// Six organisms in one huddle, scales 1000..6000. Attacker 1 (scale
    /// 1000) always strikes its nearest-scale neighbour (2, scale 2000):
    /// candidates 2..6 have mean scale distance 3000 against a target
    /// distance of 1000, so every attack's delta is exactly +2000 - and
    /// the scrambled null band sits below it (Gate E: the statistic
    /// recovers a scripted discrimination and the scramble erases it).
    #[test]
    fn scripted_discrimination_is_recovered_and_the_scramble_erases_it() {
        let mut events: Vec<Event> = Vec::new();
        for id in 1..=6_u64 {
            events.push(birth(0, id));
            events.push(phenotype(0, id, id as i64 * 1_000));
        }
        // A chain of six distinct attackers, each striking its
        // nearest-scale neighbour, so the permutation null averages six
        // dependent contrasts instead of replaying one.
        for (tick, (attacker, target)) in [(1, 2), (2, 3), (3, 4), (4, 5), (5, 6), (6, 5)]
            .iter()
            .enumerate()
        {
            events.push(damage((tick as u64 + 1) * 10, *attacker, *target));
        }
        events.sort_by_key(|event| event.tick);
        let positions: Vec<(i32, i32)> = (0..6).map(|index| (index * 100, 0)).collect();
        let spatial = vec![
            SpatialSample {
                tick: 0,
                positions: positions.clone(),
            },
            SpatialSample {
                tick: 100,
                positions,
            },
        ];
        let summary = world_recognition(&events, &spatial, &base_plan()).expect("join holds");
        assert_eq!(summary.attacks_total, 6);
        assert_eq!(summary.attacks_usable, 6);
        // Per-attack deltas: 2000, 1200, 800, 800, 1200, 2000 - mean
        // 8000/6 = 1333 exactly (integer division).
        assert_eq!(summary.mean_delta_milli, Some(1_333));
        assert!(
            summary.null_p95_abs_milli < 1_333,
            "the scramble must erase the discrimination: {summary:?}"
        );
    }

    /// An attacker or target without a phenotype record (a founder) is
    /// counted out, never guessed at.
    #[test]
    fn missing_phenotypes_are_counted_out_never_guessed() {
        let events = vec![
            birth(0, 2),
            phenotype(0, 2, 2_000),
            damage(10, 1, 2),
            damage(20, 2, 1),
        ];
        let spatial = vec![SpatialSample {
            tick: 0,
            positions: vec![(0, 0), (100, 0)],
        }];
        let mut plan = base_plan();
        plan.founder_count = 1;
        plan.shuffles = 0;
        let summary = world_recognition(&events, &spatial, &plan).expect("join holds");
        assert_eq!(summary.attacks_total, 2);
        assert_eq!(summary.attacks_missing_phenotype, 2);
        assert_eq!(summary.attacks_usable, 0);
        assert_eq!(summary.mean_delta_milli, None);
    }

    /// The candidate radius is inclusive at exactly the boundary and one
    /// unit past it is out - with no candidate, the attack is counted as
    /// such rather than scored against nobody.
    #[test]
    fn the_candidate_radius_is_inclusive_and_no_candidates_is_counted() {
        for (distance, usable, no_candidates) in [(2_000_i32, 1_u64, 0_u64), (2_001, 0, 1)] {
            let events = vec![
                birth(0, 1),
                birth(0, 2),
                birth(0, 3),
                phenotype(0, 1, 1_000),
                phenotype(0, 2, 2_000),
                phenotype(0, 3, 3_000),
                damage(10, 1, 2),
            ];
            // Target 2 is far (still attackable - the Damage event is
            // its own proof of range at the true tick); the only
            // would-be candidate 3 sits at the boundary.
            let spatial = vec![SpatialSample {
                tick: 0,
                positions: vec![(0, 0), (50_000, 0), (distance, 0)],
            }];
            let mut plan = base_plan();
            plan.shuffles = 0;
            let summary = world_recognition(&events, &spatial, &plan).expect("join holds");
            assert_eq!(summary.attacks_usable, usable, "at {distance}");
            assert_eq!(
                summary.attacks_no_candidates, no_candidates,
                "at {distance}"
            );
        }
    }
}
