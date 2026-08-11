//! Phase 11: the per-individual action series a campaign writes must be
//! complete, correctly timed, equal to what the world actually contained, and
//! **incapable of changing it**.
//!
//! Copied in shape from `spatial_sampling.rs`, which is the precedent for a
//! sampled artifact. The neutrality test at the bottom is the one that had to
//! be reasoned about rather than copied, and the reason is stated there.

use sim_core::World;
use sim_experiment::{Campaign, SchedulerOptions, run_campaign};
use sim_persist::decode_action;

const TICKS: u64 = 600;
const INTERVAL: u64 = 100;

const CAMPAIGN: &str = "\
campaign action-sampling
ticks 600
seeds 11..12
base preset phase2
base cells_x 64
base cells_y 64
base initial_organisms 120
base max_entities 1200
base genome2.enabled true
base probe.enabled true
base probe.action_census_enabled true
condition only
output events off
output snapshots off
output actions 100
";

fn temp_directory(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!(
        "lifesim-actions-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).expect("temp dir");
    directory
}

/// What the kernel actually held at each sample tick, obtained by stepping an
/// independent world with no sampling code involved.
type Reference = Vec<(u64, Vec<(u64, u64, [u32; sim_core::ACTION_CLASS_COUNT])>)>;

fn reference_census(seed: u64, campaign: &Campaign) -> Reference {
    let condition = &campaign.conditions[0];
    let config = campaign.config_for(condition, seed).expect("config");
    let mut world = World::new(config).expect("world");
    let mut out = Vec::new();
    for _ in 0..TICKS {
        world.step();
        if world.tick_number().is_multiple_of(INTERVAL) {
            out.push((
                world.tick_number(),
                world
                    .action_census()
                    .into_iter()
                    .map(|sample| (sample.id, sample.age_ticks, sample.counts))
                    .collect(),
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
            "{}.alac",
            sim_experiment::run_stem(&result.condition, seed)
        ));
        let bytes = std::fs::read(&path).expect("sample file exists");
        let scan = decode_action(&bytes).expect("sample file decodes");

        // Provenance must identify this exact world, not merely a world.
        assert_eq!(scan.info.seed, seed);
        assert_eq!(scan.info.config_hash, result.config_hash);
        assert_eq!(scan.info.terrain_checksum, result.terrain_checksum);
        assert_eq!(scan.info.sample_interval_ticks as u64, INTERVAL);
        assert_eq!(scan.info.class_count, sim_core::ACTION_CLASS_COUNT as u32);
        assert_eq!(scan.info.policy_hash, sim_persist::action_policy_hash());

        // The series is complete: exactly ticks/interval samples, at exactly
        // the expected ticks, and the manifest agrees.
        let expected_ticks: Vec<u64> = (1..=TICKS / INTERVAL).map(|n| n * INTERVAL).collect();
        assert_eq!(scan.samples.len(), expected_ticks.len());
        assert_eq!(result.action_samples, expected_ticks.len() as u64);
        let recorded: Vec<u64> = scan.samples.iter().map(|s| s.tick).collect();
        assert_eq!(recorded, expected_ticks);

        // ...and every recorded histogram equals what the kernel held. A file
        // of the right shape carrying the wrong world passes everything above.
        let reference = reference_census(seed, &campaign);
        assert_eq!(reference.len(), scan.samples.len());
        for (sample, (tick, rows)) in scan.samples.iter().zip(reference.iter()) {
            assert_eq!(sample.tick, *tick);
            let recorded: Vec<(u64, u64, [u32; sim_core::ACTION_CLASS_COUNT])> = sample
                .records
                .iter()
                .map(|record| (record.id, record.age_ticks, record.counts))
                .collect();
            assert_eq!(&recorded, rows, "histograms differ at tick {tick}");
        }

        // The run must not have been trivially empty, or all of the above is
        // satisfied by a world that died immediately - and the counts must
        // actually be counts, or it is satisfied by a world of zeros.
        let last = scan.samples.last().expect("last");
        assert!(last.records.len() > 1);
        assert!(
            last.records
                .iter()
                .all(|record| record.counts.iter().any(|value| *value > 0))
        );

        // **The property the whole artifact exists for**: an organism present
        // in two samples is the same organism, identified by id rather than
        // by row position, and its counts are cumulative so a window is a
        // difference. Row position is *not* stable - organisms die - so an
        // analysis that lined samples up by index would be comparing
        // different individuals, which is the failure C11.1 is defined
        // against.
        let first = &scan.samples[0];
        let survivors: Vec<u64> = first
            .records
            .iter()
            .map(|record| record.id)
            .filter(|id| last.records.iter().any(|record| record.id == *id))
            .collect();
        assert!(
            survivors.len() > 1,
            "nobody survived the whole series, so there is no within-lifetime pair"
        );
        let mut position_moved = false;
        for id in survivors {
            let (a, ai) = first
                .records
                .iter()
                .enumerate()
                .find_map(|(index, r)| (r.id == id).then_some((r, index)))
                .expect("present");
            let (b, bi) = last
                .records
                .iter()
                .enumerate()
                .find_map(|(index, r)| (r.id == id).then_some((r, index)))
                .expect("present");
            position_moved |= ai != bi;
            assert!(b.age_ticks > a.age_ticks);
            for slot in 0..sim_core::ACTION_CLASS_COUNT {
                assert!(
                    b.counts[slot] >= a.counts[slot],
                    "counts are not cumulative, so a window is not a difference"
                );
            }
        }
        assert!(
            position_moved,
            "no survivor changed row position, so an index-keyed analysis would have \
             passed here and this file does not demonstrate why ids are needed"
        );
    }
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn sampling_does_not_change_what_the_world_computes() {
    // **This does not hold trivially, and saying why is the point.**
    //
    // `spatial_sampling.rs`'s version of this test is easy: positions are
    // read through an observer view and nothing about sampling touches state.
    // The action census is different - it *is* checksummed kernel state - so
    // a sampler that took its before/after windows by resetting the counters
    // at each sample would move the checksum, and this assertion would be
    // false for a real reason rather than a bug.
    //
    // The design answer is that the artifact records **cumulative** rows and
    // an analysis differences two of them. `execute_unit` therefore never
    // calls `reset_action_census`, the sampling block only reads
    // `World::action_census`, and the equality below is exact rather than
    // approximate. Both campaigns enable the census, so this compares
    // "sampled" against "unsampled" and not "instrument" against "no
    // instrument" - the latter is `phase11_probe.rs`'s job and would confound
    // this one, because the two configs would differ.
    let sampled = Campaign::parse(CAMPAIGN).expect("campaign parses");
    let unsampled =
        Campaign::parse(&CAMPAIGN.replace("output actions 100\n", "")).expect("campaign parses");
    assert_eq!(
        sampled.conditions, unsampled.conditions,
        "the two campaigns differ in more than their output policy"
    );
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

    // Guard the guard: the checksum has to be capable of noticing a reset, or
    // the equality above is satisfied by a census nothing hashes. A single
    // `reset_action_census` on an equivalent world must move it.
    let condition = &sampled.conditions[0];
    let config = sampled
        .config_for(condition, sampled.seeds[0])
        .expect("config");
    let mut world = World::new(config).expect("world");
    for _ in 0..TICKS {
        world.step();
    }
    assert_eq!(world.state_checksum(), with[0]);
    world.reset_action_census();
    assert_ne!(
        world.state_checksum(),
        with[0],
        "the census is not in the checksum, so the equality above proves nothing"
    );
    let _ = std::fs::remove_dir_all(&directory);
}

#[test]
fn a_campaign_that_asks_for_actions_without_the_instrument_writes_an_empty_series() {
    // The honest failure mode, made visible rather than silent. `output
    // actions` is a policy of the harness and `probe.action_census_enabled`
    // is a property of the world; a campaign that sets the first and not the
    // second gets a well-formed file with empty samples, which a reader can
    // tell apart from "no file" and from "everybody was dead".
    let campaign =
        Campaign::parse(&CAMPAIGN.replace("base probe.action_census_enabled true\n", ""))
            .expect("campaign parses");
    let directory = temp_directory("uninstrumented");
    let options = SchedulerOptions {
        workers: 1,
        output_dir: Some(directory.clone()),
        progress: None,
    };
    let results = run_campaign(&campaign, &options);
    let result = results[0].as_ref().expect("run succeeded");
    let path = directory.join(format!(
        "{}.alac",
        sim_experiment::run_stem(&result.condition, campaign.seeds[0])
    ));
    let scan = decode_action(&std::fs::read(&path).expect("file")).expect("decodes");
    assert_eq!(scan.samples.len() as u64, TICKS / INTERVAL);
    assert!(scan.samples.iter().all(|sample| sample.records.is_empty()));
    let _ = std::fs::remove_dir_all(&directory);
}
