//! Phase 7: the spatial sample series a campaign writes must be complete,
//! correctly timed, and equal to what the world actually contained.
//!
//! The failure this guards against is not "no file was written". It is a
//! file that decodes cleanly, looks plausible, and is a shortened, shifted,
//! or stale picture of the run -- which would silently corrupt the C7.1
//! index computed from it while every test still passed.

use sim_core::{RenderEntity, World};
use sim_experiment::{Campaign, SchedulerOptions, run_campaign};
use sim_persist::decode_spatial;

const TICKS: u64 = 200;
const INTERVAL: u64 = 20;

const CAMPAIGN: &str = "\
campaign spatial-sampling
ticks 200
seeds 1..2
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 24
base max_entities 240
condition only
output events off
output snapshots off
output spatial 20
";

fn temp_directory(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "lifesim-spatial-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("temp dir");
    directory
}

/// Positions the kernel actually held at each sample tick, obtained by
/// stepping an independent world with no sampling code involved.
fn reference_positions(seed: u64, campaign: &Campaign) -> Vec<(u64, Vec<(i32, i32)>)> {
    let condition = &campaign.conditions[0];
    let config = campaign.config_for(condition, seed).expect("config");
    let mut world = World::new(config).expect("world");
    let mut buffer: Vec<RenderEntity> = Vec::new();
    let mut out = Vec::new();
    for _ in 0..TICKS {
        world.step();
        if world.tick_number().is_multiple_of(INTERVAL) {
            world.render_entities_in(i32::MIN, i32::MIN, i32::MAX, i32::MAX, &mut buffer);
            out.push((
                world.tick_number(),
                buffer.iter().map(|e| (e.x_fp, e.y_fp)).collect(),
            ));
        }
    }
    out
}

#[test]
fn the_sample_series_is_complete_correctly_timed_and_matches_the_world() {
    let campaign = Campaign::parse(CAMPAIGN).expect("campaign parses");
    let directory = temp_directory("series");
    let options = SchedulerOptions {
        workers: 2,
        output_dir: Some(directory.clone()),
        progress: None,
    };
    let results = run_campaign(&campaign, &options);
    assert_eq!(results.len(), 2);

    for (result, seed) in results.into_iter().zip(campaign.seeds.iter().copied()) {
        let result = result.expect("run succeeded");
        let path = directory.join(format!(
            "{}.alss",
            sim_experiment::run_stem(&result.condition, seed)
        ));
        let bytes = std::fs::read(&path).expect("sample file exists");
        let scan = decode_spatial(&bytes).expect("sample file decodes");

        // Provenance must identify this exact world, not merely a world.
        assert_eq!(scan.info.seed, seed);
        assert_eq!(scan.info.config_hash, result.config_hash);
        assert_eq!(scan.info.terrain_checksum, result.terrain_checksum);
        assert_eq!(scan.info.sample_interval_ticks as u64, INTERVAL);

        // The series is complete: exactly ticks/interval samples, at
        // exactly the expected ticks, and the manifest agrees.
        let expected_ticks: Vec<u64> = (1..=TICKS / INTERVAL).map(|n| n * INTERVAL).collect();
        assert_eq!(scan.samples.len(), expected_ticks.len());
        assert_eq!(result.spatial_samples, expected_ticks.len() as u64);
        let recorded_ticks: Vec<u64> = scan.samples.iter().map(|s| s.tick).collect();
        assert_eq!(recorded_ticks, expected_ticks);

        // ...and every recorded configuration equals what the kernel held.
        // A sample file of the right shape carrying the wrong world would
        // pass every check above.
        let reference = reference_positions(seed, &campaign);
        assert_eq!(reference.len(), scan.samples.len());
        for (sample, (tick, positions)) in scan.samples.iter().zip(reference.iter()) {
            assert_eq!(sample.tick, *tick);
            assert_eq!(
                &sample.positions, positions,
                "positions differ at tick {tick}"
            );
        }

        // The run must not have been trivially empty, or all of the above
        // is satisfied by a world that died immediately.
        let total: usize = scan.samples.iter().map(|s| s.positions.len()).sum();
        assert!(
            total > 0 && scan.samples.last().expect("last").positions.len() > 1,
            "the sampled world was empty, so the equality above proves nothing"
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn sampling_does_not_change_what_the_world_computes() {
    // The sampling path only reads, so a sampled campaign and an unsampled
    // one must reach identical final state. This is the check that keeps a
    // measurement from becoming an intervention (ADR-0016).
    let sampled = Campaign::parse(CAMPAIGN).expect("campaign parses");
    let unsampled =
        Campaign::parse(&CAMPAIGN.replace("output spatial 20\n", "")).expect("campaign parses");
    let directory = temp_directory("neutral");
    let options = SchedulerOptions {
        workers: 1,
        output_dir: Some(directory.clone()),
        progress: None,
    };

    let with: Vec<u64> = run_campaign(&sampled, &options)
        .into_iter()
        .map(|r| r.expect("run").state_checksum)
        .collect();
    let without: Vec<u64> = run_campaign(&unsampled, &SchedulerOptions::in_memory(1))
        .into_iter()
        .map(|r| r.expect("run").state_checksum)
        .collect();
    assert_eq!(with, without);
    let _ = std::fs::remove_dir_all(&directory);
}
