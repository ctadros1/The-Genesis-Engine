//! C17.1: zero feedback, proven. A world analyzed by the era detector at
//! every supported cadence - from its own collected events, mid-run,
//! repeatedly - has exactly the checksum trajectory of an identical world
//! never analyzed (ADR-0016, ADR-0033).
//!
//! The detector is a pure function over a copy of the log, so this is
//! structurally certain; the test exists because the specification names
//! it as the phase's most important criterion, and a certainty that is
//! not asserted is a convention (the D-108 lesson, applied to analysis).

use sim_analysis::{EraPlan, FeatureGates, world_era};
use sim_core::{Event, SimConfig, World};

const SEED: u64 = 0x0f17_5eed_0f17_5eed;

fn config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 32;
    config.cells_y = 32;
    config.initial_organisms = 60;
    config.max_entities = 600;
    config.genome2.enabled = true;
    config.contest.enabled = true;
    config.validate().expect("validates");
    config
}

fn plan(config: &SimConfig, run_ticks: u64) -> EraPlan {
    EraPlan {
        window_ticks: 100,
        burn_in_ticks: 0,
        penalty_milli: 1_000_000,
        max_segments: 4,
        initial_organisms: config.initial_organisms,
        max_entities: config.max_entities,
        run_ticks,
        gates: FeatureGates {
            contest: config.contest.enabled,
            artifact: config.artifact.enabled,
            social: config.social.enabled,
            ontogeny: config.physiology.enabled && config.physiology.ontogeny_enabled,
            transition: config.transition.enabled,
        },
    }
}

#[test]
fn analysis_at_every_cadence_leaves_the_world_checksum_trajectory_untouched() {
    let config = config();
    let ticks = 1_200_u64;
    // The control: never analyzed.
    let mut control = World::new(config).expect("world");
    let mut control_trail = Vec::with_capacity(ticks as usize);
    for _ in 0..ticks {
        control.step();
        control_trail.push(control.state_checksum());
    }

    for cadence in [1_u64, 7, 100, 500] {
        let mut world = World::new(config).expect("world");
        let mut events: Vec<Event> = Vec::new();
        let mut analyses = 0_u32;
        for tick in 1..=ticks {
            world.step();
            events.extend_from_slice(world.events());
            if tick % cadence == 0 {
                // Analyze from the log so far, at this tick's horizon.
                let era = world_era(&events, &plan(&config, tick)).expect("analysis");
                analyses += 1;
                // Use the result, so the analysis cannot be optimized away
                // and so a future "helpful" feedback path would have
                // something to feed.
                assert!(era.segments >= 1);
            }
            assert_eq!(
                world.state_checksum(),
                control_trail[(tick - 1) as usize],
                "cadence {cadence}: the analyzed world diverged at tick {tick}"
            );
        }
        assert!(analyses > 0, "cadence {cadence} never analyzed");
    }
}

#[test]
fn the_same_log_yields_a_byte_identical_report_twice() {
    let config = config();
    let mut world = World::new(config).expect("world");
    let mut events: Vec<Event> = Vec::new();
    for _ in 0..800 {
        world.step();
        events.extend_from_slice(world.events());
    }
    let plan = plan(&config, 800);
    let first = world_era(&events, &plan).expect("analysis");
    let second = world_era(&events, &plan).expect("analysis");
    assert_eq!(first, second);
    let rendered_a = sim_analysis::render_world("A", SEED, world.config_hash(), 11, &first, true);
    let rendered_b = sim_analysis::render_world("A", SEED, world.config_hash(), 11, &second, true);
    assert_eq!(rendered_a, rendered_b, "the report must be byte-identical");
    assert!(rendered_a.starts_with("world condition=A "));
}
