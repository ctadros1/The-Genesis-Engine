//! Independent-world scheduler.
//!
//! Determinism rule 10 (`specifications/determinism-extensions.md`): each
//! world runs in a schedulable unit that shares no mutable state with any
//! other, and the proof is an equality test rather than an argument.
//!
//! Two structural choices carry that guarantee, and both matter more than
//! they look:
//!
//! - **A worker owns its world completely.** It builds the config, builds
//!   the world, ticks it, and writes its own files. Nothing is borrowed
//!   across worlds, so there is no shared allocator arena, no shared RNG,
//!   and no shared buffer that could carry state from one world into
//!   another.
//! - **Results are stored by unit index, never appended on completion.**
//!   A worker writes into `results[index]`, so the output ordering is the
//!   campaign's canonical (condition, seed) order no matter which worker
//!   finished first, at any worker count. Appending on completion would
//!   make the manifest depend on thread scheduling, which is exactly the
//!   class of bug A5.2 exists to catch.
//!
//! Work is claimed from a single shared counter. Which worker claims which
//! unit is genuinely nondeterministic and is *supposed* to be: if that
//! choice could reach a result, A5.2 fails, and it is designed to fail
//! loudly rather than to be argued away.
//!
//! A panicking world is isolated: its unit is recorded as failed and the
//! campaign continues. One world dying never corrupts or stalls another.

use crate::campaign::{Campaign, Condition};
use crate::manifest::RunResult;
use sim_core::{Counters, Phase2Counters, RenderEntity, World};
use sim_persist::{
    EventLogInfo, EventLogRecorder, EventLogWriter, SnapshotStore, SpatialLogInfo,
    SpatialLogWriter, encode_snapshot,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// One (condition, seed) world.
#[derive(Clone, Debug)]
pub struct RunUnit {
    pub index: usize,
    pub condition: String,
    pub seed: u64,
}

/// Progress callback. Called from worker threads, so it must be cheap and
/// must not influence anything the run computes.
pub type ProgressFn = dyn Fn(&RunUnit, &Result<RunResult, String>) + Send + Sync;

pub struct SchedulerOptions {
    pub workers: usize,
    /// Directory for per-run artifacts. `None` runs in memory only, which
    /// is what the determinism tests use.
    pub output_dir: Option<PathBuf>,
    pub progress: Option<Arc<ProgressFn>>,
}

impl SchedulerOptions {
    pub fn in_memory(workers: usize) -> Self {
        Self {
            workers,
            output_dir: None,
            progress: None,
        }
    }
}

/// Run every (condition, seed) unit of a campaign.
///
/// The returned vector is indexed by unit and is byte-identical across
/// worker counts for the same campaign.
pub fn run_campaign(
    campaign: &Campaign,
    options: &SchedulerOptions,
) -> Vec<Result<RunResult, String>> {
    let units = enumerate_units(campaign);
    let total = units.len();
    let results: Vec<Mutex<Option<Result<RunResult, String>>>> =
        (0..total).map(|_| Mutex::new(None)).collect();
    let next = AtomicUsize::new(0);
    let workers = options.workers.clamp(1, 64).min(total.max(1));

    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let index = next.fetch_add(1, Ordering::Relaxed);
                    if index >= total {
                        return;
                    }
                    let unit = &units[index];
                    // A panic inside one world must not take down the
                    // campaign or any sibling world.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        execute_unit(campaign, unit, options.output_dir.as_deref())
                    }))
                    .unwrap_or_else(|_| {
                        Err(format!(
                            "world panicked (condition '{}', seed {})",
                            unit.condition, unit.seed
                        ))
                    });
                    if let Some(progress) = options.progress.as_ref() {
                        progress(unit, &outcome);
                    }
                    *results[index].lock().expect("result slot") = Some(outcome);
                }
            });
        }
    });

    results
        .into_iter()
        .enumerate()
        .map(|(index, slot)| {
            slot.into_inner()
                .expect("result slot")
                .unwrap_or_else(|| Err(format!("unit {index} produced no result")))
        })
        .collect()
}

/// Canonical unit order: condition-major, then ascending seed. Fixed so a
/// manifest is comparable across runs and worker counts.
pub fn enumerate_units(campaign: &Campaign) -> Vec<RunUnit> {
    let mut units = Vec::with_capacity(campaign.run_count());
    let mut index = 0;
    for condition in &campaign.conditions {
        for &seed in &campaign.seeds {
            units.push(RunUnit {
                index,
                condition: condition.name.clone(),
                seed,
            });
            index += 1;
        }
    }
    units
}

fn condition_named<'a>(campaign: &'a Campaign, name: &str) -> Option<&'a Condition> {
    campaign
        .conditions
        .iter()
        .find(|condition| condition.name == name)
}

/// One unit whose world cannot be constructed at all.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreflightFailure {
    pub condition: String,
    pub seed: u64,
    pub reason: String,
}

/// Construct every world at tick 0 without ticking any of them.
///
/// This exists because of a failure mode that is easy to miss and expensive
/// to discover late: world generation rejects a seed whose land fraction
/// falls outside the configured bounds, so a declared 30-seed design can
/// quietly execute as a 24-seed one. A campaign that runs a different
/// design from the one it declares is not a weaker experiment, it is a
/// different experiment, and the seeds it dropped were not dropped at
/// random — they were dropped by a terrain property that may well correlate
/// with the outcome being measured.
///
/// The cost is one world generation per unit, paid once before any ticks
/// run. Against a campaign of any serious length that is negligible, and
/// against a short one it is still cheaper than discovering the hole
/// afterwards.
pub fn preflight(campaign: &Campaign) -> Vec<PreflightFailure> {
    let mut failures = Vec::new();
    for unit in enumerate_units(campaign) {
        let Some(condition) = condition_named(campaign, &unit.condition) else {
            continue;
        };
        let outcome = campaign
            .config_for(condition, unit.seed)
            .map_err(|error| error.to_string())
            .and_then(|config| World::new(config).map_err(|error| error.to_string()));
        if let Err(reason) = outcome {
            failures.push(PreflightFailure {
                condition: unit.condition.clone(),
                seed: unit.seed,
                reason,
            });
        }
    }
    failures
}

/// Filesystem-safe stem for one run's artifacts.
pub fn run_stem(condition: &str, seed: u64) -> String {
    let safe: String = condition
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '_'
            }
        })
        .collect();
    format!("{safe}-seed{seed:016x}")
}

fn execute_unit(
    campaign: &Campaign,
    unit: &RunUnit,
    output_dir: Option<&Path>,
) -> Result<RunResult, String> {
    let condition = condition_named(campaign, &unit.condition)
        .ok_or_else(|| format!("condition '{}' vanished", unit.condition))?;
    let config = campaign
        .config_for(condition, unit.seed)
        .map_err(|error| error.to_string())?;
    let config_hash = config.stable_hash();
    let mut world = World::new(config).map_err(|error| error.to_string())?;
    let terrain_checksum = world.terrain().terrain_checksum;

    let stem = run_stem(&unit.condition, unit.seed);
    let mut recorder = match (output_dir, campaign.output.events) {
        (Some(directory), true) => {
            let path = directory.join(format!("{stem}.alev"));
            let writer = EventLogWriter::create(
                &path,
                &EventLogInfo {
                    format_version: sim_persist::EVENT_LOG_FORMAT_VERSION,
                    world_id: unit.index as u64 + 1,
                    seed: unit.seed,
                    config_hash,
                    event_schema_version: sim_core::EVENT_SCHEMA_VERSION,
                    max_events_per_tick: sim_core::MAX_EVENTS_PER_TICK as u32,
                    start_tick: 0,
                    build_version: sim_persist::BUILD_VERSION.to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            Some(EventLogRecorder::new(writer))
        }
        _ => None,
    };

    // Spatial sampling reads positions through the existing read-only
    // observer view, so the kernel is untouched and both fixtures stay
    // unmovable by this file's existence.
    let mut spatial = match (output_dir, campaign.output.spatial_interval) {
        (Some(directory), interval) if interval > 0 => {
            let path = directory.join(format!("{stem}.alss"));
            let writer = SpatialLogWriter::create(
                &path,
                &SpatialLogInfo {
                    format_version: sim_persist::SPATIAL_LOG_FORMAT_VERSION,
                    world_id: unit.index as u64 + 1,
                    seed: unit.seed,
                    config_hash,
                    terrain_checksum,
                    cells_x: world.config().cells_x,
                    cells_y: world.config().cells_y,
                    cell_size_m: world.config().cell_size_m,
                    sample_interval_ticks: u32::try_from(interval)
                        .map_err(|_| "spatial interval exceeds u32".to_owned())?,
                    max_organisms: world.config().max_entities,
                    build_version: sim_persist::BUILD_VERSION.to_owned(),
                },
            )
            .map_err(|error| error.to_string())?;
            Some(writer)
        }
        _ => None,
    };
    // Morphology series. Plain text rather than a versioned binary like
    // ALSS, because a sample here is six scalars and not a dense position
    // dump - the artifact that justified a binary format does not apply, and
    // a readable series is cheaper to audit.
    let mut morphology_series: Option<String> =
        (output_dir.is_some() && campaign.output.morphology_interval > 0).then(|| {
            format!(
                "morphology-series 1 policy {} seed {:#018x} interval {}\n",
                sim_core::MORPHOLOGY_POLICY_VERSION,
                unit.seed,
                campaign.output.morphology_interval
            )
        });
    let mut render_buffer: Vec<RenderEntity> = Vec::new();
    let mut positions: Vec<(i32, i32)> = Vec::new();

    for _ in 0..campaign.ticks {
        world.step();
        if let Some(recorder) = recorder.as_mut() {
            recorder.record(&world).map_err(|error| error.to_string())?;
        }
        if let Some(writer) = spatial.as_mut()
            && world.tick_number() % campaign.output.spatial_interval == 0
        {
            // Unbounded bounds rather than the computed world extent: an
            // organism exactly on the far edge must not be dropped by an
            // off-by-one in a measurement's own framing.
            world.render_entities_in(i32::MIN, i32::MIN, i32::MAX, i32::MAX, &mut render_buffer);
            positions.clear();
            positions.extend(
                render_buffer
                    .iter()
                    .map(|entity| (entity.x_fp, entity.y_fp)),
            );
            writer
                .append(world.tick_number(), &positions)
                .map_err(|error| error.to_string())?;
        }
        if let Some(series) = morphology_series.as_mut()
            && world.tick_number() % campaign.output.morphology_interval == 0
        {
            let metrics = world.metrics();
            series.push_str(&format!(
                "sample tick={} population={} mean_modules_milli={} median_modules={} \
                 distinct={} nonviable={} refused_node_budget={}\n",
                metrics.tick,
                metrics.population,
                metrics.mean_modules_milli,
                metrics.median_modules,
                metrics.distinct_morphologies,
                metrics.nonviable_bodies,
                metrics.refused_node_budget,
            ));
        }
        if campaign.check_interval > 0 && world.tick_number() % campaign.check_interval == 0 {
            world
                .check_invariants()
                .map_err(|violation| format!("invariant violated: {violation}"))?;
        }
    }
    let spatial_samples = match spatial.as_mut() {
        Some(writer) => {
            writer.sync().map_err(|error| error.to_string())?;
            writer.samples()
        }
        None => 0,
    };
    if let (Some(directory), Some(series)) = (output_dir, morphology_series.as_ref()) {
        std::fs::write(directory.join(format!("{stem}.almo")), series)
            .map_err(|error| error.to_string())?;
    }
    world
        .check_invariants()
        .map_err(|violation| format!("invariant violated at end of run: {violation}"))?;

    let event_log_offset = match recorder.as_mut() {
        Some(recorder) => {
            recorder
                .writer_mut()
                .sync()
                .map_err(|error| error.to_string())?;
            recorder.writer().offset()
        }
        None => 0,
    };

    let state_checksum = world.state_checksum();
    let metrics = world.metrics();
    let mut snapshot_bytes = 0_u64;
    if let (Some(directory), true) = (output_dir, campaign.output.snapshot) {
        let bytes = encode_snapshot(
            &world.export_state(),
            unit.index as u64 + 1,
            0,
            state_checksum,
            sim_persist::BUILD_VERSION,
            event_log_offset,
            campaign.output.compression_level,
        )
        .map_err(|error| error.to_string())?;
        snapshot_bytes = bytes.len() as u64;
        write_atomic(&directory.join(format!("{stem}.alif")), &bytes)?;
    }

    let counters: Counters = world.counters();
    let phase2: Option<Phase2Counters> = world.phase2_enabled().then(|| world.phase2_counters());

    Ok(RunResult {
        index: unit.index,
        condition: unit.condition.clone(),
        seed: unit.seed,
        condition_delta_hash: condition.delta_hash(),
        config_hash,
        terrain_checksum,
        state_checksum,
        ticks: campaign.ticks,
        population: metrics.population,
        extinct: metrics.extinct,
        total_energy_milli: metrics.total_energy_milli,
        total_biomass_milli: metrics.total_biomass_milli,
        max_ancestry_depth: metrics.max_ancestry_depth,
        counters,
        phase2,
        event_log_offset,
        snapshot_bytes,
        spatial_samples,
        deaths_senescence_total: metrics.deaths_senescence_total,
        deaths_extrinsic_total: metrics.deaths_extrinsic_total,
        deaths_juvenile_total: metrics.deaths_juvenile_total,
        max_age_ticks_observed: metrics.max_age_ticks_observed,
        total_capacity_milli: metrics.total_capacity_milli,
        mean_nodes_milli: metrics.mean_nodes_milli,
        mean_edges_milli: metrics.mean_edges_milli,
        median_nodes: metrics.median_nodes,
        median_edges: metrics.median_edges,
        distinct_structures: metrics.distinct_structures,
        mean_modules_milli: metrics.mean_modules_milli,
        median_modules: metrics.median_modules,
        distinct_morphologies: metrics.distinct_morphologies,
        nonviable_bodies: metrics.nonviable_bodies,
        refused_node_budget: metrics.refused_node_budget,
        structural_mutations_applied: metrics.structural_mutations_applied,
        structural_mutations_rejected: metrics.structural_mutations_rejected,
        attacks_total: metrics.attacks_total,
        deaths_by_damage_total: metrics.deaths_by_damage_total,
        carcasses: metrics.carcasses,
    })
}

/// Same durability ordering as the snapshot store: write a temp file, fsync
/// it, rename, then sync the directory.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::fs::{File, OpenOptions};
    use std::io::Write;

    let temp = path.with_extension("tmp");
    {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temp)
            .map_err(|error| error.to_string())?;
        file.write_all(bytes).map_err(|error| error.to_string())?;
        file.sync_all().map_err(|error| error.to_string())?;
    }
    std::fs::rename(&temp, path).map_err(|error| error.to_string())?;
    if let Some(parent) = path.parent()
        && let Ok(directory) = File::open(parent)
    {
        let _ = directory.sync_all();
    }
    Ok(())
}

/// Open (or create) the snapshot store a campaign directory uses for
/// catalog-backed saves. Campaign runs write plain `.alif` files by
/// default; this exists so a campaign directory can be inspected with the
/// same tooling as a server data directory.
pub fn open_store(directory: &Path) -> Result<SnapshotStore, String> {
    SnapshotStore::open(directory)
        .map(|(store, _)| store)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::campaign::Campaign;

    fn campaign(text: &str) -> Campaign {
        Campaign::parse(text).expect("campaign parses")
    }

    // `basal_cost_milli_per_s` is varied rather than a crowding parameter
    // because it applies to every organism on every tick, so the treatment
    // is guaranteed to bite. A crowding parameter is only expressed when
    // local density crosses the threshold, and at these populations it
    // frequently never does.
    const SMALL: &str = "\
campaign scheduler-equality
ticks 120
seeds 1..6
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 24
base max_entities 240
condition control
condition treatment
set treatment basal_cost_milli_per_s 160
vary basal_cost_milli_per_s
output events off
output snapshots off
";

    #[test]
    fn unit_order_is_canonical_and_independent_of_worker_count() {
        let campaign = campaign(SMALL);
        let units = enumerate_units(&campaign);
        assert_eq!(units.len(), 12);
        assert_eq!(units[0].condition, "control");
        assert_eq!(units[0].seed, 1);
        assert_eq!(units[6].condition, "treatment");
        assert_eq!(units[6].seed, 1);
    }

    #[test]
    fn concurrency_never_reaches_a_result() {
        let campaign = campaign(SMALL);
        let baseline = run_campaign(&campaign, &SchedulerOptions::in_memory(1));
        let checksums: Vec<u64> = baseline
            .iter()
            .map(|result| result.as_ref().expect("run succeeded").state_checksum)
            .collect();

        for workers in [2_usize, 4, 8] {
            let other = run_campaign(&campaign, &SchedulerOptions::in_memory(workers));
            let observed: Vec<u64> = other
                .iter()
                .map(|result| result.as_ref().expect("run succeeded").state_checksum)
                .collect();
            assert_eq!(observed, checksums, "divergence at {workers} workers");
        }
    }

    #[test]
    fn a_scheduled_world_matches_the_same_world_run_alone() {
        let campaign = campaign(SMALL);
        let scheduled = run_campaign(&campaign, &SchedulerOptions::in_memory(4));
        for unit in enumerate_units(&campaign) {
            let condition = condition_named(&campaign, &unit.condition).unwrap();
            let config = campaign.config_for(condition, unit.seed).unwrap();
            let mut alone = World::new(config).unwrap();
            for _ in 0..campaign.ticks {
                alone.step();
            }
            let result = scheduled[unit.index].as_ref().expect("run succeeded");
            assert_eq!(
                result.state_checksum,
                alone.state_checksum(),
                "condition {} seed {} diverged from a solo run",
                unit.condition,
                unit.seed
            );
        }
    }

    #[test]
    fn conditions_produce_behaviorally_different_worlds_on_the_same_seed() {
        let campaign = campaign(SMALL);
        let results = run_campaign(&campaign, &SchedulerOptions::in_memory(2));
        let seeds = campaign.seeds.len();
        let mut differed = 0;
        for offset in 0..seeds {
            let control = results[offset].as_ref().unwrap();
            let treatment = results[seeds + offset].as_ref().unwrap();
            assert_eq!(control.seed, treatment.seed);
            assert_ne!(control.config_hash, treatment.config_hash);
            // The state checksum is NOT the test. `World::state_checksum`
            // hashes the config hash into its preamble, so two conditions
            // always produce different checksums even when the treatment is
            // behaviorally inert. Only a measured quantity can distinguish
            // "the config changed" from "the world changed".
            if control.total_energy_milli != treatment.total_energy_milli {
                differed += 1;
            }
        }
        assert_eq!(
            differed, seeds,
            "the treatment changed no world's energy; the condition is inert"
        );
    }

    #[test]
    fn a_checksum_difference_alone_does_not_prove_a_behavioral_difference() {
        // Guards the reasoning above: a condition that changes the config
        // without changing any behavior still changes every checksum.
        // `cluster_neural_weight_q16` is offline-analysis policy only, so it
        // cannot touch a tick.
        let campaign = campaign(
            "campaign inert-treatment\nticks 60\nseeds 1..3\nbase preset phase2\n\
             base cells_x 32\nbase cells_y 32\nbase initial_organisms 24\n\
             base max_entities 240\ncondition control\ncondition inert\n\
             set inert phase2.cluster_neural_weight_q16 32768\n\
             vary phase2.cluster_neural_weight_q16\noutput events off\noutput snapshots off\n",
        );
        let results = run_campaign(&campaign, &SchedulerOptions::in_memory(2));
        let seeds = campaign.seeds.len();
        for offset in 0..seeds {
            let control = results[offset].as_ref().unwrap();
            let inert = results[seeds + offset].as_ref().unwrap();
            assert_ne!(
                control.state_checksum, inert.state_checksum,
                "the config hash is in the checksum preamble, so these must differ"
            );
            assert_eq!(
                control.total_energy_milli, inert.total_energy_milli,
                "an analysis-only parameter changed the simulation"
            );
            assert_eq!(control.population, inert.population);
            assert_eq!(control.counters, inert.counters);
        }
    }

    #[test]
    fn preflight_names_every_seed_whose_world_cannot_be_generated() {
        // Seeds 3, 4, and 5 fail land-fraction validation at this size, so
        // a campaign declaring 1..6 would silently execute as a 3-seed
        // design without this check.
        let campaign = campaign(
            "campaign preflight\nticks 1\nseeds 1..6\nbase cells_x 48\nbase cells_y 48\n\
             base initial_organisms 40\nbase max_entities 400\ncondition only\n",
        );
        let failures = preflight(&campaign);
        let mut seeds: Vec<u64> = failures.iter().map(|failure| failure.seed).collect();
        seeds.sort_unstable();
        assert_eq!(seeds, vec![3, 4, 5]);
        assert!(failures[0].reason.contains("land fraction"));
    }

    #[test]
    fn preflight_is_silent_when_every_world_generates() {
        let campaign = campaign(SMALL);
        assert_eq!(preflight(&campaign), Vec::new());
    }

    #[test]
    fn run_stems_are_filesystem_safe() {
        assert_eq!(run_stem("a b/c", 1), "a_b_c-seed0000000000000001");
        assert_eq!(run_stem("control", 255), "control-seed00000000000000ff");
    }
}
