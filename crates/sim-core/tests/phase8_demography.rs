//! Phase 8 acceptance criteria C8.4 (allometry is what it claims) and C8.8
//! (exactness and determinism), plus the unit and property obligations of
//! the test plan.
//!
//! C8.1, C8.2, C8.3, C8.5, C8.6, and C8.7 are multi-seed campaign claims
//! and live in the experiment harness, not here. What this file establishes
//! is that the mechanisms do what they say and that a demography-disabled
//! world is still the Phase 7 world byte for byte.

use sim_core::{DeathCause, EventKind, SimConfig, World};

/// Small, fast, no climate. Thermoregulation is inert without a
/// temperature field -- a documented precondition, asserted by its own test
/// below -- so every mechanism except that one is exercised here.
fn demography_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    config.physiology.enabled = true;
    config
}

/// With the Phase 6 temperature field, which thermoregulation needs.
///
/// 256x256 is not a preference: the Phase 6 record measured that `Arid`
/// requires an interior far enough from water to be dry, and a 64x64
/// continent does not have one, so world generation rejects the map. That
/// finding is in `research/performance-notes.md` and this is the operational
/// consequence of it.
fn climate_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.initial_organisms = 200;
    config.max_entities = 2_000;
    config.climate.enabled = true;
    config.climate.worldgen_version = sim_core::WorldgenVersion::V2;
    config.physiology.enabled = true;
    config
}

fn run(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for tick in 1..=ticks {
        world.step();
        if tick % 200 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    world.check_invariants().expect("final invariants");
    world
}

// --- C8.8 determinism and disabled-section equality ------------------------

#[test]
fn c8_8_demography_disabled_reproduces_the_phase_2_and_phase_7_fixtures() {
    // The rollback story: disabled, the world takes the Phase 7 code paths,
    // and with contest also disabled it is still the Phase 2 world.
    let phase2 = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    assert!(!phase2.physiology.enabled);
    assert_eq!(phase2.stable_hash(), 0xf83d_3981_bf7d_d189);

    let mut world = World::new(phase2).expect("world");
    for _ in 0..500 {
        world.step();
    }
    assert_eq!(world.state_checksum(), 0xff9d_fcff_5dff_bf42);

    // A disabled section's parameters cannot move the hash either. This is
    // the property that keeps every earlier fixture reproducible forever,
    // and it has to be checked for each new section rather than assumed.
    let mut fiddled = phase2;
    fiddled.physiology.extrinsic_hazard_q16_per_s = 9_999;
    fiddled.physiology.senescence_onset_ticks = 17;
    fiddled.physiology.basal_exponent_quarters = 6;
    fiddled.physiology.thermal_cost_milli_per_s_per_degree = 500;
    assert_eq!(fiddled.stable_hash(), phase2.stable_hash());

    // ...and it must be behaviorally inert too, not merely hash-inert.
    let mut fiddled_world = World::new(fiddled).expect("world");
    for _ in 0..500 {
        fiddled_world.step();
    }
    assert_eq!(fiddled_world.state_checksum(), 0xff9d_fcff_5dff_bf42);
}

#[test]
fn c8_8_an_enabled_section_changes_behavior_not_merely_the_checksum() {
    // Evidence trap 2: `state_checksum` hashes the config hash into its
    // preamble, so two conditions *always* differ on it. The difference has
    // to be asserted on a measured quantity.
    let disabled = {
        let mut config = demography_config(11);
        config.physiology.enabled = false;
        config
    };
    let enabled = demography_config(11);

    let off = run(disabled, 3_000);
    let on = run(enabled, 3_000);
    let off_metrics = off.metrics();
    let on_metrics = on.metrics();

    assert!(!off_metrics.physiology_enabled);
    assert!(on_metrics.physiology_enabled);
    assert!(
        on_metrics.deaths_senescence_total + on_metrics.deaths_extrinsic_total > 0,
        "the hazard never killed anything, so the section is inert"
    );
    assert_eq!(off_metrics.deaths_senescence_total, 0);
    assert_eq!(off_metrics.deaths_extrinsic_total, 0);
    assert_ne!(
        off_metrics.population, on_metrics.population,
        "populations are identical, so the section changed nothing measurable"
    );
}

#[test]
fn c8_8_the_ledger_stays_exact_with_every_mechanism_active() {
    let mut config = climate_config(3);
    config.contest.enabled = true;
    let world = run(config, 4_000);
    let ledger = world.ledger();
    let expected = ledger.initial_energy_milli + ledger.assimilated_milli
        - ledger.spent_milli
        - ledger.removed_at_death_milli;
    let actual: i128 = world.metrics().total_energy_milli.into();
    assert_eq!(
        actual, expected,
        "energy ledger drifted with thermoregulation, allometry, and hazard active"
    );
    // The run must not have been trivially empty, or exactness proves
    // nothing (evidence trap 1).
    assert!(world.population() > 0, "the world went extinct");
    assert!(
        ledger.assimilated_milli > 0 && ledger.spent_milli > 0,
        "no energy ever moved"
    );
}

#[test]
fn c8_8_replay_is_identical_and_a_restored_world_continues_identically() {
    let config = demography_config(23);
    let first = run(config, 1_500);
    let second = run(config, 1_500);
    assert_eq!(first.state_checksum(), second.state_checksum());

    let state = first.export_state();
    let mut restored = World::from_state(state).expect("restore");
    assert_eq!(restored.state_checksum(), first.state_checksum());

    let mut original = run(config, 1_500);
    for _ in 0..500 {
        original.step();
        restored.step();
    }
    assert_eq!(
        original.state_checksum(),
        restored.state_checksum(),
        "a restored world diverged from the one it was captured from"
    );
    assert!(restored.physiology_enabled());
}

// --- C8.4 allometry is what it claims --------------------------------------

#[test]
fn c8_4_basal_cost_follows_the_configured_power_law() {
    // The criterion: measured basal metabolic rate against body mass fits a
    // power law whose exponent matches the configured one. Measured through
    // the public API against the actual multiplier the tick applies, at
    // every configurable exponent rather than only the default.
    for quarters in 1..=6_u32 {
        let mut config = SimConfig::phase2_default(1);
        config.physiology.enabled = true;
        config.physiology.allometry_enabled = true;
        config.physiology.basal_exponent_quarters = quarters;

        // A power law means log(cost) is linear in log(mass) with slope
        // `quarters/4`. Check it as a ratio identity instead, which needs no
        // logarithm: cost(m^2) / cost(m) must equal cost(m) / cost(1) when
        // masses are in geometric progression.
        let at = |mass_milli: i64| {
            sim_core::allometry_multiplier_milli(&config.physiology, mass_milli) as f64 / 1000.0
        };
        let low = at(800);
        let mid = at(1_000);
        let high = at(1_250); // 1000/800 == 1250/1000, a geometric step

        let first_ratio = mid / low;
        let second_ratio = high / mid;
        assert!(
            (first_ratio - second_ratio).abs() < 0.01,
            "quarters {quarters}: ratios {first_ratio} and {second_ratio} are not a power law"
        );

        // And the exponent itself: (1250/1000)^(q/4) == high/mid.
        let expected = 1.25_f64.powf(f64::from(quarters) / 4.0);
        assert!(
            (second_ratio - expected).abs() < 0.01,
            "quarters {quarters}: measured exponent ratio {second_ratio}, expected {expected}"
        );
    }
}

#[test]
fn c8_4_allometry_actually_reaches_the_tick() {
    // The check that the multiplier is not merely correct in isolation.
    // Two worlds identical except for the exponent must diverge, and the
    // steeper exponent must cost more, because body scale averages above
    // the 1000 reference.
    // Measured after exactly one tick, with reproduction and every hazard
    // off. Over a long run the comparison would be confounded: a steeper
    // exponent kills more organisms, and a smaller population spends less
    // in total however expensive each individual is.
    let spend_after_one_tick = |quarters: u32| {
        let mut config = demography_config(5);
        config.reproduction_enabled = false;
        config.physiology.senescence_enabled = false;
        config.physiology.thermoregulation_enabled = false;
        config.physiology.extrinsic_hazard_q16_per_s = 0;
        config.physiology.basal_exponent_quarters = quarters;
        let mut world = World::new(config).expect("world");
        let population_before = world.population();
        world.step();
        assert_eq!(world.population(), population_before, "an organism died");
        world.ledger().spent_milli
    };
    let shallow = spend_after_one_tick(1); // 0.25
    let default = spend_after_one_tick(3); // 0.75
    let steep = spend_after_one_tick(6); // 1.5
    assert!(
        shallow < default && default < steep,
        "spend must rise with the exponent: {shallow} / {default} / {steep}"
    );
}

// --- Mechanism obligations from the test plan ------------------------------

#[test]
fn senescence_replaces_the_hard_cutoff_rather_than_joining_it() {
    // With senescence live, organisms must be able to exceed
    // `max_age_ticks` -- otherwise C8.5's lifespan comparison would measure
    // the cutoff instead of the evolved trait -- and no invariant may fire.
    let mut config = demography_config(7);
    config.max_age_ticks = 1_000;
    config.physiology.senescence_enabled = true;
    config.physiology.senescence_onset_ticks = 3_000;
    config.physiology.senescence_scale_ticks = 20_000;
    config.physiology.extrinsic_hazard_q16_per_s = 0;
    let world = run(config, 4_000);
    assert!(
        world.metrics().max_age_ticks_observed > 1_000,
        "no organism outlived the cutoff senescence is supposed to have replaced"
    );
    assert_eq!(
        world.metrics().deaths_old_age_total,
        0,
        "the hard cutoff still fired while senescence was enabled"
    );

    // With senescence disabled the cutoff is back and binding.
    let mut cutoff = demography_config(7);
    cutoff.max_age_ticks = 1_000;
    cutoff.physiology.senescence_enabled = false;
    let capped = run(cutoff, 4_000);
    assert!(capped.metrics().max_age_ticks_observed < 1_000);
    assert!(capped.metrics().deaths_old_age_total > 0);
}

#[test]
fn every_death_cause_is_reachable_and_named_exactly_once() {
    // C8.1 is a claim about the death-cause distribution, so each cause has
    // to be reachable and a death must carry exactly one.
    let mut config = demography_config(31);
    config.contest.enabled = true;
    config.contest.attack_threshold_q16 = -32_768;
    config.contest.attack_range_m = 6;
    config.physiology.senescence_onset_ticks = 500;
    config.physiology.senescence_scale_ticks = 2_000;
    config.physiology.senescence_hazard_q16_per_s = 655;
    config.physiology.extrinsic_hazard_q16_per_s = 65;

    let mut world = World::new(config).expect("world");
    let mut seen = std::collections::BTreeMap::new();
    let mut deaths_by_id: std::collections::BTreeMap<u64, usize> =
        std::collections::BTreeMap::new();
    for _ in 0..4_000 {
        world.step();
        for event in world.events() {
            if let EventKind::Death { id, cause } = event.kind {
                *seen.entry(cause.name()).or_insert(0_usize) += 1;
                *deaths_by_id.entry(id).or_insert(0) += 1;
            }
        }
    }
    for cause in ["starvation", "damage", "senescence", "extrinsic"] {
        assert!(
            seen.get(cause).copied().unwrap_or(0) > 0,
            "cause {cause} never occurred; seen: {seen:?}"
        );
    }
    assert!(
        deaths_by_id.values().all(|count| *count == 1),
        "an organism died more than once"
    );
}

#[test]
fn thermoregulation_costs_energy_only_where_the_temperature_is_wrong() {
    // Thermal preference was inherited-but-inert from Phase 2 to Phase 7.
    // Enabling it must change the energy budget, and widening the neutral
    // band until it covers the whole field must make it free again -- which
    // is the check that the cost really is the deviation term and not some
    // other change enabling the section happened to make.
    let spend_of = |band_milli: i32| {
        let mut config = climate_config(13);
        config.physiology.senescence_enabled = false;
        config.physiology.extrinsic_hazard_q16_per_s = 0;
        config.physiology.allometry_enabled = false;
        config.physiology.thermoregulation_enabled = true;
        config.physiology.thermal_neutral_band_milli = band_milli;
        run(config, 1_500).ledger().spent_milli
    };
    let narrow = spend_of(0);
    let wide = spend_of(1_000_000); // wider than any temperature deviation
    assert!(
        narrow > wide,
        "a narrow neutral band must cost more than a band that covers the field"
    );
}

#[test]
fn a_fully_disabled_physiology_section_is_bit_identical_to_phase_7() {
    // Each mechanism is independently gated; all off must equal the section
    // being absent, or "rollback" is not a real property.
    let mut all_off = demography_config(17);
    all_off.physiology.allometry_enabled = false;
    all_off.physiology.thermoregulation_enabled = false;
    all_off.physiology.senescence_enabled = false;
    all_off.physiology.extrinsic_hazard_q16_per_s = 0;

    let mut absent = demography_config(17);
    absent.physiology.enabled = false;

    let with_section = run(all_off, 1_500);
    let without = run(absent, 1_500);
    // The config hashes differ (the section is enabled in one), so compare
    // a measured quantity rather than the checksum.
    assert_eq!(with_section.population(), without.population());
    assert_eq!(
        with_section.ledger().spent_milli,
        without.ledger().spent_milli
    );
    assert_eq!(
        with_section.metrics().deaths_starvation_total,
        without.metrics().deaths_starvation_total
    );
}

#[test]
fn the_hazard_never_leaves_its_bounds_under_adversarial_configuration() {
    // Property obligation: hazard probability stays in range even when
    // every knob is at its maximum, and the world neither panics nor
    // violates an invariant.
    let mut config = demography_config(41);
    config.physiology.extrinsic_hazard_q16_per_s = u32::MAX;
    config.physiology.senescence_hazard_q16_per_s = u32::MAX;
    config.physiology.juvenile_hazard_multiplier_q16 = u32::MAX;
    config.physiology.senescence_onset_ticks = 0;
    config.physiology.senescence_power = 4;
    let mut world = World::new(config).expect("world");
    for _ in 0..50 {
        world.step();
        world.check_invariants().expect("invariants hold");
    }
    // Everything dies immediately at a certain hazard, which is the correct
    // behavior for a probability clamped to 1 rather than an overflow.
    assert_eq!(world.population(), 0);
    assert!(world.is_extinct());
}

#[test]
fn deaths_and_causes_reconcile_with_the_population_change() {
    let mut config = demography_config(19);
    config.reproduction_enabled = false;
    let mut world = World::new(config).expect("world");
    let initial = world.population() as u64;
    let mut deaths = 0_u64;
    for _ in 0..3_000 {
        world.step();
        deaths += world
            .events()
            .iter()
            .filter(|event| matches!(event.kind, EventKind::Death { .. }))
            .count() as u64;
    }
    assert_eq!(
        initial - world.population() as u64,
        deaths,
        "the death events do not account for the population change"
    );
    let metrics = world.metrics();
    let by_cause = metrics.deaths_starvation_total
        + metrics.deaths_old_age_total
        + metrics.deaths_senescence_total
        + metrics.deaths_extrinsic_total
        + metrics.deaths_by_damage_total;
    assert_eq!(
        by_cause, deaths,
        "the cause counters do not sum to the deaths"
    );
    assert!(deaths > 0, "nothing died, so the reconciliation is vacuous");
}

#[test]
fn a_damage_death_is_not_also_counted_as_a_hazard_death() {
    // Cause precedence: damage is the most specific and a death has exactly
    // one cause. With both contest and a heavy hazard live, the counters
    // must still partition the deaths.
    let mut config = demography_config(29);
    config.contest.enabled = true;
    config.contest.attack_threshold_q16 = -32_768;
    config.contest.attack_range_m = 6;
    config.physiology.extrinsic_hazard_q16_per_s = 655;
    let mut world = World::new(config).expect("world");
    let mut causes = Vec::new();
    for _ in 0..2_000 {
        world.step();
        for event in world.events() {
            if let EventKind::Death { cause, .. } = event.kind {
                causes.push(cause);
            }
        }
    }
    let metrics = world.metrics();
    assert_eq!(
        causes
            .iter()
            .filter(|cause| **cause == DeathCause::Damage)
            .count() as u64,
        metrics.deaths_by_damage_total
    );
    assert_eq!(
        causes
            .iter()
            .filter(|cause| **cause == DeathCause::Extrinsic)
            .count() as u64,
        metrics.deaths_extrinsic_total
    );
    assert!(metrics.deaths_by_damage_total > 0 && metrics.deaths_extrinsic_total > 0);
}

#[test]
fn thermoregulation_is_inert_without_a_temperature_field() {
    // A documented precondition rather than an error: a world with no
    // climate section has no temperature for a preference to be preferred
    // against. Asserted rather than assumed, because the alternative is a
    // Phase 8 campaign that silently measures nothing for C8.7.
    let mut with_flag = demography_config(37);
    with_flag.physiology.thermoregulation_enabled = true;
    let mut without_flag = demography_config(37);
    without_flag.physiology.thermoregulation_enabled = false;
    assert!(!with_flag.climate.enabled);

    let on = run(with_flag, 1_000);
    let off = run(without_flag, 1_000);
    assert_eq!(on.ledger().spent_milli, off.ledger().spent_milli);
    assert_eq!(on.population(), off.population());
}
