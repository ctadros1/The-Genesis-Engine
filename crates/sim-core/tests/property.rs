//! Property-style sweeps using seeded deterministic inputs (no external
//! property-testing dependency in Phase 1). Each case explores a broad input
//! region and asserts bounds/invariants rather than exact values.

use sim_core::{RngSystem, SimConfig, World, named_random};

const SWEEP_SEED: u64 = 0x00de_fec8_ab1e_5eed;

fn draw(index: u64, salt: u32) -> u64 {
    named_random(SWEEP_SEED, index, RngSystem::WorldGen, index, salt)
}

#[test]
fn worlds_across_many_seeds_generate_or_fail_closed() {
    let mut generated = 0_u32;
    for case in 0..40_u64 {
        let mut config = SimConfig::phase1_default(draw(case, 0));
        config.cells_x = 32 + (draw(case, 1) % 96) as u32;
        config.cells_y = 32 + (draw(case, 2) % 96) as u32;
        config.initial_organisms = 10 + (draw(case, 3) % 90) as u32;
        config.max_entities = config.initial_organisms * 4;
        match World::new(config) {
            Ok(world) => {
                generated += 1;
                world.check_invariants().unwrap();
                let terrain = world.terrain();
                let fraction = terrain.land_fraction_q16();
                assert!(fraction >= config.min_land_fraction_q16);
                assert!(fraction <= config.max_land_fraction_q16);
                assert!(terrain.habitable_cells > 0);
            }
            // Land-fraction rejection is the documented fail-closed path for
            // hostile seed/threshold combinations.
            Err(error) => {
                let text = error.to_string();
                assert!(text.contains("land fraction"), "unexpected error: {text}");
            }
        }
    }
    assert!(
        generated >= 30,
        "only {generated}/40 seeds generated a world"
    );
}

#[test]
fn short_runs_across_seeds_keep_energy_and_bounds() {
    for case in 0..12_u64 {
        let mut config = SimConfig::phase1_default(draw(case, 10));
        config.cells_x = 48;
        config.cells_y = 48;
        config.initial_organisms = 30;
        config.max_entities = 300;
        // Sweep the rate parameters across their plausible ranges.
        config.growth_rate_q16_per_s = 1 + (draw(case, 11) % 30_000) as u32;
        config.basal_cost_milli_per_s = (draw(case, 12) % 400) as i64;
        config.move_cost_milli_per_s = (draw(case, 13) % 1_000) as i64;
        config.intake_rate_milli_per_s = (draw(case, 14) % 4_000) as i64;
        let Ok(mut world) = World::new(config) else {
            continue;
        };
        for _ in 0..120 {
            world.step();
        }
        world
            .check_invariants()
            .unwrap_or_else(|violation| panic!("case {case}: {violation}"));
    }
}

#[test]
fn invalid_config_mutations_never_build_a_world() {
    let base = SimConfig::phase1_default(1);
    let mutations: Vec<SimConfig> = vec![
        {
            let mut config = base;
            config.cells_x = 0;
            config
        },
        {
            let mut config = base;
            config.cell_size_m = 100;
            config
        },
        {
            let mut config = base;
            config.initial_organisms = 0;
            config
        },
        {
            let mut config = base;
            config.max_entities = 0;
            config
        },
        {
            let mut config = base;
            config.dt_ms = 0;
            config
        },
        {
            let mut config = base;
            config.assimilation_q16 = 70_000;
            config
        },
        {
            let mut config = base;
            config.energy_max_milli = 0;
            config
        },
        {
            let mut config = base;
            config.initial_energy_milli = config.energy_max_milli + 1;
            config
        },
        {
            let mut config = base;
            config.speed_mps_q16 = 0;
            config
        },
        {
            let mut config = base;
            config.crowding_radius_m = 0;
            config
        },
        {
            let mut config = base;
            config.max_age_ticks = 0;
            config
        },
        {
            let mut config = base;
            config.repro_threshold_milli = 0;
            config
        },
        {
            let mut config = base;
            config.min_land_fraction_q16 = config.max_land_fraction_q16;
            config
        },
    ];
    for (index, config) in mutations.iter().enumerate() {
        assert!(
            World::new(*config).is_err(),
            "mutation {index} unexpectedly produced a world"
        );
    }
}
