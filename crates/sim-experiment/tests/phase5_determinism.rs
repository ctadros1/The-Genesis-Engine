//! Phase 5 acceptance criterion A5.2: scheduling is result-neutral.
//!
//! > For a fixed set of 30 seeds and one config, per-world final state
//! > checksums are identical at scheduler concurrency 1, 2, and C (the
//! > configured maximum), and identical to running each world alone. Any
//! > work-stealing, thread-count, or completion-order dependency shows up
//! > here as a checksum difference. This is the determinism criterion that
//! > makes every later multi-seed claim trustworthy.
//!
//! This is determinism rule 10's required proof
//! (`specifications/determinism-extensions.md`): an equality test, not an
//! argument.
//!
//! Seeds 24 and 29 are omitted from the 1..32 range because world
//! generation rejects them at this map size for land fraction. They are
//! omitted explicitly rather than by silently accepting a 28-seed run,
//! which is the failure mode `preflight` exists to prevent, and the test
//! asserts the seed set is exactly 30.

use sim_core::World;
use sim_experiment::{Campaign, SchedulerOptions, enumerate_units, preflight, run_campaign};

/// One condition, 30 seeds. Long enough for populations to diverge, short
/// enough to run five times in a test.
const CAMPAIGN: &str = "\
campaign scheduler-determinism
ticks 400
seeds 1..23 25..28 30..32
base preset phase2
base cells_x 32
base cells_y 32
base initial_organisms 24
base max_entities 240
condition only
output events off
output snapshots off
";

fn campaign() -> Campaign {
    Campaign::parse(CAMPAIGN).expect("campaign parses")
}

fn checksums_at(campaign: &Campaign, workers: usize) -> Vec<u64> {
    run_campaign(campaign, &SchedulerOptions::in_memory(workers))
        .into_iter()
        .map(|result| {
            result
                .unwrap_or_else(|error| panic!("run failed at {workers} workers: {error}"))
                .state_checksum
        })
        .collect()
}

#[test]
fn a5_2_scheduling_is_result_neutral() {
    let campaign = campaign();
    assert_eq!(campaign.seeds.len(), 30, "the design declares 30 seeds");
    assert_eq!(
        preflight(&campaign),
        Vec::new(),
        "every declared world must be constructible, or the design is not the one declared"
    );

    // Concurrency 1 is the reference.
    let reference = checksums_at(&campaign, 1);
    assert_eq!(reference.len(), 30);

    // C is the configured maximum the scheduler will use.
    for workers in [2_usize, 4, 8, 16, 64] {
        let observed = checksums_at(&campaign, workers);
        assert_eq!(
            observed, reference,
            "per-world checksums differ at {workers} workers; scheduling reached a result"
        );
    }

    // ...and identical to running each world entirely alone, outside the
    // scheduler, with no other world in the process at the same time.
    for unit in enumerate_units(&campaign) {
        let condition = campaign
            .conditions
            .iter()
            .find(|condition| condition.name == unit.condition)
            .expect("condition");
        let config = campaign.config_for(condition, unit.seed).expect("config");
        let mut alone = World::new(config).expect("world");
        for _ in 0..campaign.ticks {
            alone.step();
        }
        assert_eq!(
            reference[unit.index],
            alone.state_checksum(),
            "seed 0x{:016x} diverged between the scheduler and a solo run",
            unit.seed
        );
    }
}

#[test]
fn a5_2_repeated_runs_at_the_same_concurrency_are_stable() {
    // A scheduler that is merely *usually* deterministic passes a single
    // comparison by luck. Repeating the highest-contention configuration
    // makes an intermittent shared-state dependency much likelier to show.
    let campaign = campaign();
    let reference = checksums_at(&campaign, 8);
    for attempt in 0..4 {
        assert_eq!(
            checksums_at(&campaign, 8),
            reference,
            "attempt {attempt} diverged at the same worker count"
        );
    }
}

#[test]
fn a5_2_worlds_are_not_accidentally_identical() {
    // The equality above would be trivially satisfiable if every world
    // produced the same checksum, so confirm the seeds genuinely diverge.
    let campaign = campaign();
    let checksums = checksums_at(&campaign, 4);
    let mut unique = checksums.clone();
    unique.sort_unstable();
    unique.dedup();
    assert_eq!(
        unique.len(),
        checksums.len(),
        "two seeds produced identical worlds; the equality test is vacuous"
    );
}

/// Test-plan item: "worker crash isolation; one world failing does not
/// corrupt or stall another; a failed world is reported, not silently
/// dropped."
///
/// `preflight` normally refuses a campaign like this before it runs, so the
/// scheduler is exercised directly here. That is the point: the refusal is
/// the first line of defence, and this is the behaviour when something gets
/// past it.
#[test]
fn a_failing_world_is_isolated_and_reported_never_silently_dropped() {
    // Seeds 3, 4, and 5 fail land-fraction validation at 48x48.
    let campaign = Campaign::parse(
        "campaign isolation\nticks 200\nseeds 1..6\nbase preset phase2\n\
         base cells_x 48\nbase cells_y 48\nbase initial_organisms 40\n\
         base max_entities 400\ncondition only\noutput events off\n\
         output snapshots off\n",
    )
    .expect("campaign parses");
    assert_eq!(
        preflight(&campaign).len(),
        3,
        "the fixture needs 3 bad seeds"
    );

    let results = run_campaign(&campaign, &SchedulerOptions::in_memory(4));
    assert_eq!(results.len(), 6, "every unit must produce a result slot");

    let units = enumerate_units(&campaign);
    let mut failed = Vec::new();
    let mut succeeded = Vec::new();
    for (unit, result) in units.iter().zip(results.iter()) {
        match result {
            Ok(run) => succeeded.push((unit.seed, run.state_checksum)),
            Err(reason) => {
                assert!(
                    reason.contains("land fraction"),
                    "a failure must carry its reason: {reason}"
                );
                failed.push(unit.seed);
            }
        }
    }
    failed.sort_unstable();
    assert_eq!(failed, vec![3, 4, 5], "failures are reported, not dropped");
    assert_eq!(succeeded.len(), 3, "the healthy worlds still ran");

    // The surviving worlds are bit-identical to the same worlds run in a
    // campaign with no failures in it at all, so a sibling's failure
    // corrupted nothing.
    for (seed, checksum) in succeeded {
        let condition = &campaign.conditions[0];
        let config = campaign.config_for(condition, seed).expect("config");
        let mut alone = World::new(config).expect("world");
        for _ in 0..campaign.ticks {
            alone.step();
        }
        assert_eq!(
            checksum,
            alone.state_checksum(),
            "seed {seed} was affected by a sibling world's failure"
        );
    }
}
