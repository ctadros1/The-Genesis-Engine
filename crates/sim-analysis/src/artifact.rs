//! Phase 12 artifact-half criteria (`lifesim-artifact-analysis-v1`): C12.1,
//! C12.2 and C12.3, decided from campaign artifacts alone.
//!
//! **Analysis observes; it never instructs** (ADR-0016). Every quantity here
//! is a pure function of the manifest, the per-run event log, and the final
//! snapshot's object table, reduced per world before anything is compared,
//! because the world is the replicate (ADR-0022 A5). The decision rules and
//! every constant they use are stated in the campaign's pre-registration
//! (`experiments/phase12-artifact-preregistration.md`) and echoed into the
//! rendered report, so a reader can see what was fixed before the run.
//!
//! # What each criterion is operationalised as
//!
//! - **C12.1** - the rate of *successful* pick-up, place and combine actions
//!   per million organism-ticks under A minus the same rate under C, where
//!   under C every requested action counts as a success (the kernel's
//!   `artifact.inert` rule: fire, pay, no effect). A world counts when the
//!   difference reaches the pre-registered SESOI in the increasing direction;
//!   the bar is a count of worlds. The *fire* rate under A (successes plus
//!   refusals of the same three actions, from the log) is reported beside
//!   it, so "use" and "firing" are separable.
//! - **C12.2** - two halves, both required. (a) The median lifetime of
//!   placed-object episodes (from `ObjectReleased{placed:true}` to the same
//!   id's next `ObjectPickedUp` or `ObjectDestroyed`, censored at the
//!   horizon at its observed length) exceeds the median organism lifespan
//!   (birth to `Death`, censored likewise). (b) Within a world, organisms
//!   whose exposure fraction reached the pre-registered floor - ticks spent
//!   in a cell holding a live placed object over ticks lived, from
//!   `ObjectExposure` at death or the final table for the living - produce
//!   more offspring per thousand ticks of life than unexposed organisms
//!   *born in the same capacity band* (`birth_band`, the terrain quintile of
//!   the birth cell), the per-world effect being the weighted mean over
//!   bands of the exposed-minus-unexposed difference. A world with fewer
//!   exposed organisms than the pre-registered floor counts as not showing
//!   the effect, never as excluded.
//! - **C12.3** - live composites of depth two or more per thousand living
//!   organisms, sampled every thousand ticks from the log (each
//!   `ObjectCombined` with `depth >= 2` opens one, the same id's
//!   `ObjectDestroyed` closes it), averaged over the first and the last
//!   third of the run; a world counts when the last-third mean exceeds the
//!   first-third mean by the pre-registered SESOI. Under D the count is zero
//!   by construction and the report asserts it.
//!
//! Extinct worlds are analysed to their extinction and count against every
//! bar; nothing is excluded for having died.

use std::collections::BTreeMap;

use sim_core::{Event, EventKind, ObjectAction};

use crate::paired::{Direction, Pair, PairedResult, compare, median_milli};

pub const ARTIFACT_ANALYSIS_VERSION: &str = "lifesim-artifact-analysis-v1";

/// The pre-registered constants. Every field is echoed into the report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArtifactPlan {
    /// C12.1: successful actions per million organism-ticks, A minus C.
    pub sesoi_c121_ppm: i64,
    pub bar_c121: usize,
    /// C12.2 (a): worlds whose median placed lifetime exceeds the median
    /// organism lifespan.
    pub bar_c122_lifetime: usize,
    /// C12.2 (b): worlds with a positive stratified exposure effect.
    pub bar_c122_fitness: usize,
    /// C12.2 (b): the exposure fraction (milli) at or above which an
    /// organism counts as exposed, and the fewest exposed organisms a world
    /// needs for its effect to be defined.
    pub exposure_floor_milli: i64,
    pub exposure_min_organisms: usize,
    /// C12.3: depth-two composites per thousand organisms, last third minus
    /// first third, in milli.
    pub sesoi_c123_milli: i64,
    pub bar_c123: usize,
    pub seeds: usize,
    pub analysis_seed: u64,
}

impl ArtifactPlan {
    /// The pre-registered plan. `bar_c123` is the N C12.3 left unstated:
    /// 20 of 30, the smallest count with a one-sided binomial p below 0.05
    /// at a null rate of one half, the same bar C12.1 states.
    pub fn preregistered() -> Self {
        Self {
            sesoi_c121_ppm: 10,
            bar_c121: 20,
            bar_c122_lifetime: 15,
            bar_c122_fitness: 20,
            exposure_floor_milli: 50,
            exposure_min_organisms: 20,
            sesoi_c123_milli: 500,
            bar_c123: 20,
            seeds: 30,
            analysis_seed: 0xa11f_ac75_0b1e_c751,
        }
    }
}

/// One living organism at the end of a run, from the final snapshot.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LivingOrganism {
    pub id: u64,
    pub age_ticks: u64,
    pub exposure_ticks: u64,
    pub birth_band: u8,
}

/// The per-run inputs the CLI hands over: manifest scalars, the whole event
/// log, and the living organisms' histories from the final snapshot.
pub struct WorldInputs<'a> {
    pub condition: &'a str,
    pub seed: u64,
    pub horizon: u64,
    pub initial_organisms: u64,
    pub extinct: bool,
    pub organism_ticks: u64,
    pub picked_up: u64,
    pub placed: u64,
    pub combined: u64,
    pub composites_depth2_final: u64,
    pub events: &'a [Event],
    pub living: &'a [LivingOrganism],
}

/// One world's reduced statistics.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldArtifact {
    pub condition: String,
    pub seed: u64,
    pub extinct: bool,
    pub ticks_run: u64,
    pub organism_ticks: u64,
    pub successes: u64,
    pub fires: u64,
    pub success_rate_ppm: i64,
    pub fire_rate_ppm: i64,
    pub placed_episodes: usize,
    pub placed_censored: usize,
    pub median_placed_lifetime: Option<u64>,
    pub organism_lifespans: usize,
    pub organisms_censored: usize,
    pub median_organism_lifespan: Option<u64>,
    pub exposed: usize,
    pub unexposed: usize,
    pub strata_used: usize,
    /// Offspring per thousand ticks of life, exposed minus unexposed,
    /// weighted over birth bands, milli. `None` when undefined.
    pub exposure_effect_milli: Option<i64>,
    pub depth2_ever: u64,
    pub depth2_first_third_milli: i64,
    pub depth2_last_third_milli: i64,
    pub depth2_samples: usize,
}

fn rate_ppm(count: u64, organism_ticks: u64) -> i64 {
    if organism_ticks == 0 {
        return 0;
    }
    ((u128::from(count) * 1_000_000) / u128::from(organism_ticks)) as i64
}

fn median_u64(values: &mut [u64]) -> Option<u64> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    Some(values[values.len() / 2])
}

/// Reduce one world under the plan's exposure floor.
pub fn world_artifact(input: &WorldInputs<'_>, plan: &ArtifactPlan) -> WorldArtifact {
    let events = input.events;
    // The run's last tick: the horizon, or the extinction tick.
    let extinction_tick = events
        .iter()
        .find(|event| matches!(event.kind, EventKind::Extinction))
        .map(|event| event.tick);
    let ticks_run = extinction_tick.unwrap_or(input.horizon).min(input.horizon);

    // --- C12.1 -----------------------------------------------------------
    let successes = input.picked_up + input.placed + input.combined;
    let refusals = events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                EventKind::ObjectActionRefused { action, .. }
                    if action == ObjectAction::PickUp.id()
                        || action == ObjectAction::Place.id()
                        || action == ObjectAction::Combine.id()
            )
        })
        .count() as u64;
    let fires = successes + refusals;

    // --- C12.2 (a): placed-object episodes and organism lifespans -----------
    let mut open_placed: BTreeMap<u64, u64> = BTreeMap::new();
    let mut placed_lifetimes: Vec<u64> = Vec::new();
    let mut placed_censored = 0_usize;
    for event in events {
        match event.kind {
            EventKind::ObjectReleased { id, placed: true, .. } => {
                open_placed.entry(id).or_insert(event.tick);
            }
            EventKind::ObjectPickedUp { id, .. } | EventKind::ObjectDestroyed { id, .. } => {
                if let Some(start) = open_placed.remove(&id) {
                    placed_lifetimes.push(event.tick.saturating_sub(start));
                }
            }
            _ => {}
        }
    }
    for (_, start) in open_placed {
        placed_lifetimes.push(ticks_run.saturating_sub(start));
        placed_censored += 1;
    }
    let placed_episodes = placed_lifetimes.len();
    let median_placed_lifetime = median_u64(&mut placed_lifetimes);

    let mut born_at: BTreeMap<u64, u64> = (1..=input.initial_organisms).map(|id| (id, 0)).collect();
    let mut lifespans: Vec<u64> = Vec::new();
    let mut offspring: BTreeMap<u64, u64> = BTreeMap::new();
    let mut histories: Vec<(u64, u64, u64, u8)> = Vec::new(); // id, age, exposure, band
    for event in events {
        match event.kind {
            EventKind::Birth { id, .. } => {
                born_at.insert(id, event.tick);
            }
            EventKind::PairedBirth {
                id,
                parent_a,
                parent_b,
                ..
            } => {
                born_at.insert(id, event.tick);
                *offspring.entry(parent_a).or_insert(0) += 1;
                *offspring.entry(parent_b).or_insert(0) += 1;
            }
            EventKind::Death { id, .. } => {
                if let Some(birth) = born_at.remove(&id) {
                    lifespans.push(event.tick.saturating_sub(birth));
                }
            }
            EventKind::ObjectExposure {
                id,
                exposure_ticks,
                age_ticks,
                birth_band,
                ..
            } => histories.push((id, age_ticks, exposure_ticks, birth_band)),
            _ => {}
        }
    }
    let organisms_censored = born_at.len();
    for (_, birth) in born_at {
        lifespans.push(ticks_run.saturating_sub(birth));
    }
    let organism_lifespans = lifespans.len();
    let median_organism_lifespan = median_u64(&mut lifespans);
    for living in input.living {
        histories.push((living.id, living.age_ticks, living.exposure_ticks, living.birth_band));
    }

    // --- C12.2 (b): stratified exposure effect ---------------------------------
    // Per band: (sum of offspring rate, n) for exposed and unexposed, where the
    // rate is offspring per thousand ticks of life, in milli.
    let mut strata: BTreeMap<u8, ([i128; 2], [usize; 2])> = BTreeMap::new();
    let mut exposed = 0_usize;
    let mut unexposed = 0_usize;
    for (id, age, exposure, band) in &histories {
        if *age == 0 {
            continue;
        }
        let fraction_milli = (u128::from(*exposure) * 1_000 / u128::from(*age)) as i64;
        let side = usize::from(fraction_milli >= plan.exposure_floor_milli && *exposure > 0);
        let is_exposed = side == 1;
        if is_exposed {
            exposed += 1;
        } else {
            unexposed += 1;
        }
        let count = offspring.get(id).copied().unwrap_or(0);
        let rate_milli = (u128::from(count) * 1_000_000 / u128::from(*age)) as i128;
        let entry = strata.entry(*band).or_insert(([0; 2], [0; 2]));
        entry.0[side] += rate_milli;
        entry.1[side] += 1;
    }
    let mut weighted = 0_i128;
    let mut weight = 0_i128;
    let mut strata_used = 0_usize;
    for (_, (sums, counts)) in &strata {
        if counts[0] == 0 || counts[1] == 0 {
            continue;
        }
        let mean_unexposed = sums[0] / counts[0] as i128;
        let mean_exposed = sums[1] / counts[1] as i128;
        let w = counts[0].min(counts[1]) as i128;
        weighted += (mean_exposed - mean_unexposed) * w;
        weight += w;
        strata_used += 1;
    }
    let exposure_effect_milli = (weight > 0).then(|| (weighted / weight) as i64);

    // --- C12.3: live depth-two composites over time -------------------------------
    let mut depth_of: BTreeMap<u64, u8> = BTreeMap::new();
    let mut live_depth2: i64 = 0;
    let mut depth2_ever: u64 = 0;
    let mut population: i64 = input.initial_organisms as i64;
    let mut samples: Vec<(u64, i64)> = Vec::new(); // (tick, depth2 per 1000 organisms, milli)
    let mut next_sample: u64 = 1_000;
    let mut cursor = 0_usize;
    while next_sample <= ticks_run {
        while cursor < events.len() && events[cursor].tick <= next_sample {
            match events[cursor].kind {
                EventKind::Birth { .. } | EventKind::PairedBirth { .. } => population += 1,
                EventKind::Death { .. } => population -= 1,
                EventKind::ObjectCombined { composite, depth, .. } => {
                    depth_of.insert(composite, depth);
                    if depth >= 2 {
                        live_depth2 += 1;
                        depth2_ever += 1;
                    }
                }
                EventKind::ObjectDestroyed { id, .. } => {
                    if let Some(depth) = depth_of.remove(&id)
                        && depth >= 2
                    {
                        live_depth2 -= 1;
                    }
                }
                _ => {}
            }
            cursor += 1;
        }
        let per_thousand_milli = if population > 0 {
            live_depth2 * 1_000_000 / population
        } else {
            0
        };
        samples.push((next_sample, per_thousand_milli));
        next_sample += 1_000;
    }
    // Any combinations after the last sample still count toward "ever".
    while cursor < events.len() {
        if let EventKind::ObjectCombined { depth, .. } = events[cursor].kind
            && depth >= 2
        {
            depth2_ever += 1;
        }
        cursor += 1;
    }
    let third = samples.len() / 3;
    let mean = |slice: &[(u64, i64)]| -> i64 {
        if slice.is_empty() {
            0
        } else {
            slice.iter().map(|(_, value)| i128::from(*value)).sum::<i128>() as i64 / slice.len() as i64
        }
    };
    let depth2_first_third_milli = mean(&samples[..third]);
    let depth2_last_third_milli = if third > 0 { mean(&samples[samples.len() - third..]) } else { 0 };
    let _ = input.composites_depth2_final;

    WorldArtifact {
        condition: input.condition.to_owned(),
        seed: input.seed,
        extinct: input.extinct,
        ticks_run,
        organism_ticks: input.organism_ticks,
        successes,
        fires,
        success_rate_ppm: rate_ppm(successes, input.organism_ticks),
        fire_rate_ppm: rate_ppm(fires, input.organism_ticks),
        placed_episodes,
        placed_censored,
        median_placed_lifetime,
        organism_lifespans,
        organisms_censored,
        median_organism_lifespan,
        exposed,
        unexposed,
        strata_used,
        exposure_effect_milli,
        depth2_ever,
        depth2_first_third_milli,
        depth2_last_third_milli,
        depth2_samples: samples.len(),
    }
}

/// One criterion's decision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Verdict {
    pub criterion: String,
    pub count: usize,
    pub worlds: usize,
    pub bar: usize,
    pub met: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ArtifactReport {
    pub plan: ArtifactPlan,
    /// The names of conditions A, C and D as the campaign spells them.
    pub condition_a: String,
    pub condition_c: String,
    pub condition_d: String,
    pub worlds: Vec<WorldArtifact>,
    pub c121: Verdict,
    pub c121_paired: PairedResult,
    pub c121_fire_paired: PairedResult,
    pub c122_lifetime: Verdict,
    pub c122_fitness: Verdict,
    pub c122_met: bool,
    pub c123: Verdict,
    pub c123_d_zero: bool,
}

fn worlds_of<'a>(worlds: &'a [WorldArtifact], condition: &str) -> Vec<&'a WorldArtifact> {
    worlds.iter().filter(|world| world.condition == condition).collect()
}

/// Decide the three criteria. `a`, `c`, `d` name the conditions.
pub fn decide(plan: &ArtifactPlan, worlds: Vec<WorldArtifact>, a: &str, c: &str, d: &str) -> ArtifactReport {
    let a_worlds = worlds_of(&worlds, a);
    let c_worlds = worlds_of(&worlds, c);
    let d_worlds = worlds_of(&worlds, d);
    let seeds = plan.seeds;

    // C12.1: paired on seed, A minus C, absolute rate difference in ppm.
    let mut pairs = Vec::new();
    let mut fire_pairs = Vec::new();
    let mut c121_count = 0_usize;
    for world in &a_worlds {
        if let Some(control) = c_worlds.iter().find(|control| control.seed == world.seed) {
            pairs.push(Pair {
                seed: world.seed,
                treatment_milli: world.success_rate_ppm,
                control_milli: control.success_rate_ppm,
            });
            fire_pairs.push(Pair {
                seed: world.seed,
                treatment_milli: world.fire_rate_ppm,
                control_milli: control.fire_rate_ppm,
            });
            if world.success_rate_ppm - control.success_rate_ppm >= plan.sesoi_c121_ppm {
                c121_count += 1;
            }
        }
    }
    let c121_paired = compare(&pairs, plan.sesoi_c121_ppm, 500, Direction::Increase, plan.analysis_seed);
    let c121_fire_paired = compare(&fire_pairs, plan.sesoi_c121_ppm, 500, Direction::Increase, plan.analysis_seed);
    let c121 = Verdict {
        criterion: "C12.1".to_owned(),
        count: c121_count,
        worlds: seeds,
        bar: plan.bar_c121,
        met: c121_count >= plan.bar_c121,
    };

    // C12.2 (a) and (b) under A alone.
    let lifetime_count = a_worlds
        .iter()
        .filter(|world| {
            matches!(
                (world.median_placed_lifetime, world.median_organism_lifespan),
                (Some(placed), Some(organism)) if placed > organism
            )
        })
        .count();
    let fitness_count = a_worlds
        .iter()
        .filter(|world| {
            world.exposed >= plan.exposure_min_organisms
                && world.exposure_effect_milli.is_some_and(|effect| effect > 0)
        })
        .count();
    let c122_lifetime = Verdict {
        criterion: "C12.2a".to_owned(),
        count: lifetime_count,
        worlds: seeds,
        bar: plan.bar_c122_lifetime,
        met: lifetime_count >= plan.bar_c122_lifetime,
    };
    let c122_fitness = Verdict {
        criterion: "C12.2b".to_owned(),
        count: fitness_count,
        worlds: seeds,
        bar: plan.bar_c122_fitness,
        met: fitness_count >= plan.bar_c122_fitness,
    };
    let c122_met = c122_lifetime.met && c122_fitness.met;

    // C12.3 under A; D asserted zero.
    let c123_count = a_worlds
        .iter()
        .filter(|world| world.depth2_last_third_milli - world.depth2_first_third_milli >= plan.sesoi_c123_milli)
        .count();
    let c123 = Verdict {
        criterion: "C12.3".to_owned(),
        count: c123_count,
        worlds: seeds,
        bar: plan.bar_c123,
        met: c123_count >= plan.bar_c123,
    };
    let c123_d_zero = d_worlds.iter().all(|world| world.depth2_ever == 0);

    ArtifactReport {
        plan: *plan,
        condition_a: a.to_owned(),
        condition_c: c.to_owned(),
        condition_d: d.to_owned(),
        worlds,
        c121,
        c121_paired,
        c121_fire_paired,
        c122_lifetime,
        c122_fitness,
        c122_met,
        c123,
        c123_d_zero,
    }
}

pub fn render(campaign_id: &str, report: &ArtifactReport) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let plan = &report.plan;
    let _ = writeln!(out, "artifact analysis {ARTIFACT_ANALYSIS_VERSION} campaign {campaign_id}");
    let _ = writeln!(
        out,
        "plan sesoi_c121_ppm={} bar_c121={} bar_c122_lifetime={} bar_c122_fitness={} \
         exposure_floor_milli={} exposure_min_organisms={} sesoi_c123_milli={} bar_c123={} \
         seeds={} analysis_seed=0x{:016x}",
        plan.sesoi_c121_ppm,
        plan.bar_c121,
        plan.bar_c122_lifetime,
        plan.bar_c122_fitness,
        plan.exposure_floor_milli,
        plan.exposure_min_organisms,
        plan.sesoi_c123_milli,
        plan.bar_c123,
        plan.seeds,
        plan.analysis_seed
    );
    for world in &report.worlds {
        let _ = writeln!(
            out,
            "world condition={} seed={} extinct={} ticks_run={} organism_ticks={} successes={} \
             fires={} success_rate_ppm={} fire_rate_ppm={} placed_episodes={} placed_censored={} \
             median_placed_lifetime={} organism_lifespans={} organisms_censored={} \
             median_organism_lifespan={} exposed={} unexposed={} strata_used={} \
             exposure_effect_milli={} depth2_ever={} depth2_first_third_milli={} \
             depth2_last_third_milli={} depth2_samples={}",
            world.condition,
            world.seed,
            world.extinct,
            world.ticks_run,
            world.organism_ticks,
            world.successes,
            world.fires,
            world.success_rate_ppm,
            world.fire_rate_ppm,
            world.placed_episodes,
            world.placed_censored,
            world.median_placed_lifetime.map_or("none".to_owned(), |v| v.to_string()),
            world.organism_lifespans,
            world.organisms_censored,
            world.median_organism_lifespan.map_or("none".to_owned(), |v| v.to_string()),
            world.exposed,
            world.unexposed,
            world.strata_used,
            world.exposure_effect_milli.map_or("undefined".to_owned(), |v| v.to_string()),
            world.depth2_ever,
            world.depth2_first_third_milli,
            world.depth2_last_third_milli,
            world.depth2_samples,
        );
    }
    let verdict = |v: &Verdict| {
        format!(
            "{} count={} of {} bar={} met={}",
            v.criterion, v.count, v.worlds, v.bar, v.met
        )
    };
    let _ = writeln!(out, "{}", verdict(&report.c121));
    let paired = |label: &str, p: &PairedResult| {
        format!(
            "{label} pairs={} reaching_directed={} positive={} mean_diff={} median_diff={} \
             ci=[{}, {}] p_milli={}",
            p.pairs,
            p.reaching_sesoi_directed,
            p.positive_differences,
            p.mean_difference_milli,
            p.median_difference_milli,
            p.ci_low_milli,
            p.ci_high_milli,
            p.sesoi_p_value_milli
        )
    };
    let _ = writeln!(out, "{}", paired("C12.1 success-rate A-C", &report.c121_paired));
    let _ = writeln!(out, "{}", paired("C12.1 fire-rate A-C (supplementary)", &report.c121_fire_paired));
    let _ = writeln!(out, "{}", verdict(&report.c122_lifetime));
    let _ = writeln!(out, "{}", verdict(&report.c122_fitness));
    let _ = writeln!(out, "C12.2 met={} (both halves required)", report.c122_met);
    let _ = writeln!(out, "{}", verdict(&report.c123));
    let _ = writeln!(out, "C12.3 condition D zero by construction: {}", report.c123_d_zero);
    let a_worlds: Vec<&WorldArtifact> = report
        .worlds
        .iter()
        .filter(|world| world.condition == report.condition_a)
        .collect();
    let median_of = |pick: fn(&WorldArtifact) -> i64| {
        median_milli(&a_worlds.iter().map(|world| pick(world)).collect::<Vec<_>>())
    };
    let _ = writeln!(
        out,
        "medians over {} worlds: success_rate_ppm={} fire_rate_ppm={} placed_episodes={} \
         depth2_ever={} exposed={}",
        report.condition_a,
        median_of(|w| w.success_rate_ppm),
        median_of(|w| w.fire_rate_ppm),
        median_of(|w| w.placed_episodes as i64),
        median_of(|w| w.depth2_ever as i64),
        median_of(|w| w.exposed as i64),
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(tick: u64, kind: EventKind) -> Event {
        Event { tick, kind }
    }

    fn inputs<'a>(condition: &'a str, events: &'a [Event], living: &'a [LivingOrganism]) -> WorldInputs<'a> {
        WorldInputs {
            condition,
            seed: 7,
            horizon: 9_000,
            initial_organisms: 2,
            extinct: false,
            organism_ticks: 18_000,
            picked_up: 3,
            placed: 1,
            combined: 0,
            composites_depth2_final: 0,
            events,
            living,
        }
    }

    #[test]
    fn rates_episodes_lifespans_and_exposure_reduce_as_stated() {
        let events = vec![
            event(10, EventKind::ObjectActionRefused { id: 1, action: ObjectAction::PickUp.id(), reason: 1 }),
            event(10, EventKind::ObjectActionRefused { id: 1, action: ObjectAction::Strike.id(), reason: 1 }),
            event(100, EventKind::ObjectReleased { id: 50, holder: 1, placed: true, cell: 3 }),
            event(600, EventKind::ObjectDestroyed { id: 50, cause: 1 }),
            event(200, EventKind::ObjectReleased { id: 51, holder: 1, placed: true, cell: 3 }),
            event(300, EventKind::PairedBirth { id: 3, parent_a: 1, parent_b: 2, genome_hash: 0, invest_a_milli: 0, invest_b_milli: 0, mutated_trait_genes: 0, mutated_neural_genes: 0 }),
            event(4_000, EventKind::Death { id: 1, cause: sim_core::DeathCause::Starvation }),
            event(4_000, EventKind::ObjectExposure { id: 1, exposure_ticks: 2_000, carry_ticks: 0, age_ticks: 4_000, birth_band: 2 }),
            event(4_100, EventKind::Death { id: 2, cause: sim_core::DeathCause::Starvation }),
            event(4_100, EventKind::ObjectExposure { id: 2, exposure_ticks: 0, carry_ticks: 0, age_ticks: 4_100, birth_band: 2 }),
        ];
        let living = [LivingOrganism { id: 3, age_ticks: 8_700, exposure_ticks: 0, birth_band: 2 }];
        let world = world_artifact(&inputs("A", &events, &living), &ArtifactPlan::preregistered());
        // Successes 4 (3 pick-ups + 1 place); fires add the one pick-up
        // refusal and not the strike refusal.
        assert_eq!(world.successes, 4);
        assert_eq!(world.fires, 5);
        assert_eq!(world.success_rate_ppm, 4 * 1_000_000 / 18_000);
        // Episodes: 500 closed, and 8,800 censored at the horizon.
        assert_eq!(world.placed_episodes, 2);
        assert_eq!(world.placed_censored, 1);
        assert_eq!(world.median_placed_lifetime, Some(8_800));
        // Lifespans: 4000, 4100, and the child censored at 9000-300.
        assert_eq!(world.organism_lifespans, 3);
        assert_eq!(world.organisms_censored, 1);
        assert_eq!(world.median_organism_lifespan, Some(4_100));
        // Exposure: organism 1 exposed (50%), 2 and 3 not; band 2 has both
        // sides, so the effect is defined: 1's rate (1 offspring / 4000
        // ticks = 250 milli per thousand ticks) minus the unexposed mean
        // ((1/4100 + 0/8700)/2 = ~122).
        assert_eq!(world.exposed, 1);
        assert_eq!(world.unexposed, 2);
        assert_eq!(world.strata_used, 1);
        let effect = world.exposure_effect_milli.expect("defined");
        assert!(effect > 100 && effect < 200, "{effect}");
        assert_eq!(world.depth2_ever, 0);
    }

    #[test]
    fn depth_two_frequency_is_sampled_per_thousand_ticks_and_closed_by_destruction() {
        let events = vec![
            event(500, EventKind::ObjectCombined { composite: 9, held: 1, target: 2, combiner: 1, depth: 1, joint_q16: 0 }),
            event(2_500, EventKind::ObjectCombined { composite: 10, held: 9, target: 3, combiner: 1, depth: 2, joint_q16: 0 }),
            event(7_500, EventKind::ObjectDestroyed { id: 10, cause: 3 }),
        ];
        let world = world_artifact(&inputs("A", &events, &[]), &ArtifactPlan::preregistered());
        // Nine samples (1000..=9000); depth-2 live from 2500 to 7500: samples
        // at 3000..=7000 (five of nine) read 1 per 2 organisms = 500 per
        // thousand = 500_000 milli. First third (1000-3000): one of three
        // samples nonzero -> 166_666; last third (7000-9000): one of three.
        assert_eq!(world.depth2_samples, 9);
        assert_eq!(world.depth2_ever, 1);
        assert_eq!(world.depth2_first_third_milli, 500_000 / 3);
        assert_eq!(world.depth2_last_third_milli, 500_000 / 3);
    }

    #[test]
    fn the_decision_counts_worlds_against_the_bars_and_asserts_d_is_zero() {
        let plan = ArtifactPlan {
            seeds: 2,
            bar_c121: 1,
            bar_c122_lifetime: 1,
            bar_c122_fitness: 1,
            bar_c123: 1,
            exposure_min_organisms: 1,
            ..ArtifactPlan::preregistered()
        };
        let mk = |condition: &str, seed: u64, success_ppm: i64, placed: Option<u64>, lifespan: Option<u64>, effect: Option<i64>, exposed: usize, first: i64, last: i64, ever: u64| WorldArtifact {
            condition: condition.to_owned(),
            seed,
            extinct: false,
            ticks_run: 60_000,
            organism_ticks: 1,
            successes: 0,
            fires: 0,
            success_rate_ppm: success_ppm,
            fire_rate_ppm: 0,
            placed_episodes: 0,
            placed_censored: 0,
            median_placed_lifetime: placed,
            organism_lifespans: 0,
            organisms_censored: 0,
            median_organism_lifespan: lifespan,
            exposed,
            unexposed: 0,
            strata_used: 0,
            exposure_effect_milli: effect,
            depth2_ever: ever,
            depth2_first_third_milli: first,
            depth2_last_third_milli: last,
            depth2_samples: 0,
        };
        let worlds = vec![
            mk("A", 1, 100, Some(5_000), Some(4_000), Some(10), 5, 0, 1_000, 3),
            mk("A", 2, 0, Some(1_000), Some(4_000), Some(-5), 5, 0, 0, 0),
            mk("C", 1, 50, None, None, None, 0, 0, 0, 0),
            mk("C", 2, 50, None, None, None, 0, 0, 0, 0),
            mk("D", 1, 0, None, None, None, 0, 0, 0, 0),
            mk("D", 2, 0, None, None, None, 0, 0, 0, 1),
        ];
        let report = decide(&plan, worlds, "A", "C", "D");
        assert_eq!(report.c121.count, 1);
        assert!(report.c121.met);
        assert_eq!(report.c122_lifetime.count, 1);
        assert_eq!(report.c122_fitness.count, 1);
        assert!(report.c122_met);
        assert_eq!(report.c123.count, 1);
        assert!(!report.c123_d_zero, "a D world with a depth-two composite is caught");
        let text = render("test", &report);
        assert!(text.contains("C12.1 count=1 of 2 bar=1 met=true"));
        assert!(text.contains("condition D zero by construction: false"));
    }
}
