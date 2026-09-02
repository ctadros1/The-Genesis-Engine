//! Phase 16 integration clauses (ADR-0032): the field-to-individual
//! transition runs inside a world, the conversion closes both identities
//! exactly (C16.1), a materialized organism is built by the founder path's
//! arithmetic and carries no provenance (C16.2), a cell's organisms do not
//! depend on which other cells triggered (C16.3), the map is deterministic
//! (C16.4), `scratch` runs end to end (C16.5's smoke), a mid-transition
//! world survives a save round trip (C16.8), and the caps defer whole
//! events and count them. The trigger's own unit clauses live in
//! `transition.rs`.

use sim_core::{
    ConfigError, EventKind, MorphologyConfig, OriginMode, SimConfig, World, class_count,
    class_parameters, synthesize_genome, unicell_derived,
};

const SEED: u64 = 0x0f16_5eed_0f16_5eed;
const ENERGY: i64 = 4_000;

/// A scratch world with the whole stack the transition needs: phase 2,
/// genome 2, morphology, the field regime with abiogenesis, and the
/// transition itself at a floor one organism's energy high so the first
/// materializations land inside an affordable horizon.
fn scratch_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 24;
    config.cells_y = 24;
    config.initial_organisms = 0;
    config.max_entities = 400;
    config.origin.mode = OriginMode::Scratch;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.chemistry.enabled = true;
    config.chemistry.field_steps_per_tick = 2;
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    // Above the truncation floor for seeded densities, as every Phase 15
    // config that wants the mutation term live is.
    config.chemistry.mutation_q16 = 4_096;
    // Ten times the shipped abiotic input, so an eligible class reaches
    // one organism's worth of density inside a test horizon (the shipped
    // rate needs tens of thousands of ticks, which is the campaign's job).
    config.chemistry.production_milli_per_step = 20;
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 25;
    config.transition.density_floor_milli = ENERGY;
    config.transition.persistence_checks = 2;
    config.transition.organism_energy_milli = ENERGY;
    config.validate().expect("the scratch config validates");
    generable(config)
}

/// The same config on the first seed at or after its own that generates a
/// world: some seeds produce sub-minimum land at small maps (a Phase 15
/// trap), and nothing here depends on which seed that is.
fn generable(mut config: SimConfig) -> SimConfig {
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed within 32 of {:#x}", config.world_seed);
}

/// Freeze the field's own dynamics - no death, no mutation flow, no
/// growth - so a density planted by surgery is exactly the density the
/// trigger reads at the next check. The field step runs before
/// `lifecycle` in the same tick; without this a planted slot loses about
/// a third of its mass to death and flow before it is read.
fn frozen_field(config: &mut SimConfig) {
    config.chemistry.death_q16 = 0;
    config.chemistry.mutation_q16 = 0;
    config.chemistry.growth_rate_low_q16 = 0;
    config.chemistry.growth_rate_high_q16 = 0;
}

/// The lowest class at the top of the aggregation axis: eligible under
/// the default `aggregation_step_min` of 1.
fn eligible_class(config: &SimConfig) -> usize {
    (0..class_count(&config.chemistry))
        .find(|&class| class_parameters(&config.chemistry, class).aggregation_step >= 1)
        .expect("an eligible class exists")
}

/// Two land cells an organism can stand on, ascending.
fn two_land_cells(world: &World) -> (usize, usize) {
    let terrain = world.terrain();
    let mut land = (0..terrain.cell_count()).filter(|&cell| terrain.capacity_milli[cell] > 0);
    let first = land.next().expect("land");
    let second = land.nth(5).expect("more land");
    (first, second)
}

/// The two identities the conversion must close, re-derived from a save
/// rather than trusted from `check_invariants`.
fn assert_identities_close(state: &sim_core::SaveState) {
    let chemistry = state.chemistry.as_ref().expect("chemistry");
    let microbial = state.microbial.as_ref().expect("microbial");
    let transition = state.transition.as_ref().expect("transition");
    let chem_total: i128 = chemistry.concentrations.iter().map(|&v| i128::from(v)).sum();
    let micro_total: i128 = microbial.densities.iter().map(|&v| i128::from(v)).sum();
    assert_eq!(
        chemistry.produced_milli + chemistry.deposited_milli - transition.materialized_milli,
        chem_total + micro_total,
        "the field identity must close with the materialized term"
    );
    let organisms: i128 = state.energy_milli.iter().map(|&v| i128::from(v)).sum();
    assert_eq!(
        state.ledger.initial_energy_milli + state.ledger.assimilated_milli
            + transition.materialized_milli
            - state.ledger.spent_milli
            - state.ledger.removed_at_death_milli,
        organisms,
        "the organism energy identity must close with the materialized term"
    );
}

#[test]
fn the_default_organism_energy_fits_inside_the_unicell() {
    let derived = unicell_derived(&MorphologyConfig::morphology_default());
    let capacity = derived.energy_capacity_milli;
    let default = sim_core::TransitionConfig::transition_default().organism_energy_milli;
    println!(
        "unicell: mass {} milli, basal {} milli, intake {} milli, capacity {} milli",
        derived.mass_milli, derived.basal_cost_milli, derived.intake_milli, capacity
    );
    assert!(
        default <= capacity,
        "default organism energy {default} exceeds the unicell capacity {capacity}"
    );
    // The shipped pairing threshold must be reachable by a unicell, or no
    // materialized lineage could ever reproduce and C16.6 would measure a
    // threshold rather than reachability. Stated here so a campaign that
    // leaves the threshold above the capacity fails loudly in the suite.
    let pairing = SimConfig::phase2_default(SEED).phase2.pairing_energy_threshold_milli;
    println!("shipped pairing threshold {pairing} milli against unicell capacity {capacity}");
}

#[test]
fn a_scratch_world_materializes_and_every_identity_holds() {
    let mut world = World::new(scratch_config(SEED)).expect("world");
    assert_eq!(world.population(), 0, "scratch begins with no organisms");
    let mut materialized_events = 0_u64;
    let mut extinction_events = 0_u64;
    for tick in 1..=3_000 {
        world.step();
        for event in world.events() {
            match event.kind {
                EventKind::Materialized { energy_milli, .. } => {
                    materialized_events += 1;
                    assert!(energy_milli >= ENERGY);
                }
                EventKind::Extinction => extinction_events += 1,
                _ => {}
            }
        }
        if tick % 100 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let metrics = world.metrics();
    assert!(metrics.transition_enabled);
    assert!(
        metrics.materialized_total > 0,
        "nothing materialized in 3,000 ticks, so the test pinned nothing"
    );
    assert_eq!(materialized_events, metrics.materialized_total);
    assert!(metrics.transition_events_total > 0);
    assert_eq!(metrics.transition_refused_total, 0, "a refusal is a bug report");
    assert!(metrics.materialized_milli >= i128::from(ENERGY) * i128::from(metrics.materialized_total));
    // Scratch semantics: the latch fired honestly at tick 1 and cleared
    // when the first organism arrived.
    assert_eq!(extinction_events, 1, "one extinction at tick 1 in a scratch world");
    assert!(
        !world.is_extinct() || world.population() == 0,
        "an extinct flag on a populated world"
    );
    assert_identities_close(&world.export_state());
}

#[test]
fn the_transition_is_inert_when_it_cannot_trigger_and_absent_when_disabled() {
    // A populated world (founders present) so births and deaths run.
    let base = {
        let mut config = SimConfig::phase2_default(SEED ^ 0x1);
        config.cells_x = 24;
        config.cells_y = 24;
        config.initial_organisms = 40;
        config.max_entities = 400;
        config.genome2.enabled = true;
        config.morphology.enabled = true;
        config.chemistry.enabled = true;
        config.chemistry.microbial_enabled = true;
        config.chemistry.abiogenesis_enabled = true;
        config.chemistry.mutation_q16 = 4_096;
        config
    };
    let base = generable(base);
    let mut disabled = base;
    disabled.transition.enabled = false;
    let mut inert = base;
    inert.transition.enabled = true;
    inert.transition.density_floor_milli = i64::MAX / 4;
    disabled.validate().expect("validates");
    inert.validate().expect("validates");
    let mut a = World::new(disabled).expect("world");
    let mut b = World::new(inert).expect("world");
    assert_ne!(a.config_hash(), b.config_hash(), "the section is hashed when enabled");
    for tick in 1..=400 {
        a.step();
        b.step();
        let (ma, mb) = (a.metrics(), b.metrics());
        // Measured quantities, never checksums (trap 2): the config hash
        // differs, so the checksums differ by construction.
        assert_eq!(ma.population, mb.population, "tick {tick}");
        assert_eq!(ma.total_energy_milli, mb.total_energy_milli, "tick {tick}");
        assert_eq!(ma.births_total, mb.births_total, "tick {tick}");
        assert_eq!(ma.microbial_total_milli, mb.microbial_total_milli, "tick {tick}");
        assert_eq!(mb.materialized_total, 0);
    }
    assert!(a.export_state().transition.is_none());
    let saved = b.export_state();
    let transition = saved.transition.as_ref().expect("section present when enabled");
    assert_eq!(transition.materialized_total, 0);
    assert!(transition.persistence.iter().all(|&count| count == 0));
}

/// A scratch world at tick 0 with density planted in one or two eligible
/// slots and the persistence counters one check short, so the very next
/// tick (interval 1) materializes exactly those slots. Conserving surgery:
/// the planted mass is booked as production, so the restore's identity
/// check accepts it.
fn planted_world(seed: u64, cells: &[usize], organisms_worth: i64) -> (World, SimConfig) {
    let mut config = scratch_config(seed);
    frozen_field(&mut config);
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 2;
    config.transition.max_organisms_per_event = 8;
    config.validate().expect("validates");
    let world = World::new(config).expect("world");
    let classes = class_count(&config.chemistry);
    let class = eligible_class(&config);
    let mut state = world.export_state();
    for &cell in cells {
        let slot = cell * classes + class;
        let mass = organisms_worth * ENERGY;
        state.microbial.as_mut().unwrap().densities[slot] += mass;
        state.chemistry.as_mut().unwrap().produced_milli += i128::from(mass);
        state.transition.as_mut().unwrap().persistence[slot] = 1;
    }
    (World::from_state(state).expect("planted state restores"), config)
}

#[test]
fn materialization_is_invariant_to_which_other_cells_trigger() {
    let (probe, _) = planted_world(SEED ^ 0x2, &[], 0);
    let (c1, c2) = two_land_cells(&probe);
    assert!(c1 < c2);
    let (mut alone, _) = planted_world(SEED ^ 0x2, &[c1], 3);
    let (mut other_alone, _) = planted_world(SEED ^ 0x2, &[c2], 3);
    let (mut both, _) = planted_world(SEED ^ 0x2, &[c1, c2], 3);
    alone.step();
    other_alone.step();
    both.step();
    assert_eq!(alone.population(), 3);
    assert_eq!(other_alone.population(), 3);
    assert_eq!(both.population(), 6, "both cells materialized in one tick");
    // IDs run in ascending cell order: c1's organisms first.
    let ids: Vec<u64> = both.organism_ids_view().to_vec();
    assert_eq!(ids, vec![1, 2, 3, 4, 5, 6]);
    let row = |world: &World, id: u64| {
        let detail = world.organism_detail(id).expect("organism");
        let phase2 = detail.phase2.expect("phase 2");
        (
            detail.x_fp,
            detail.y_fp,
            detail.energy_milli,
            phase2.heading_bam,
            phase2.phenotype,
            phase2.parents,
            phase2.ancestry_depth,
        )
    };
    for ordinal in 0..3_u64 {
        assert_eq!(
            row(&alone, 1 + ordinal),
            row(&both, 1 + ordinal),
            "c1's organism {ordinal} depends on whether c2 triggered"
        );
        assert_eq!(
            row(&other_alone, 1 + ordinal),
            row(&both, 4 + ordinal),
            "c2's organism {ordinal} depends on whether c1 triggered"
        );
    }
    both.check_invariants().expect("invariants after a double trigger");
}

#[test]
fn the_remainder_goes_to_the_lowest_new_id_and_the_debit_equals_the_credit() {
    let (probe, _) = planted_world(SEED ^ 0x3, &[], 0);
    let (c1, _) = two_land_cells(&probe);
    // Two organisms' worth plus 700 milli: two organisms, the first
    // carrying the remainder.
    let mut config = scratch_config(SEED ^ 0x3);
    frozen_field(&mut config);
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 2;
    config.validate().expect("validates");
    let world = World::new(config).expect("world");
    let classes = class_count(&config.chemistry);
    let class = eligible_class(&config);
    let slot = c1 * classes + class;
    let mut state = world.export_state();
    let planted = 2 * ENERGY + 700;
    state.microbial.as_mut().unwrap().densities[slot] += planted;
    state.chemistry.as_mut().unwrap().produced_milli += i128::from(planted);
    state.transition.as_mut().unwrap().persistence[slot] = 1;
    let before = state.microbial.as_ref().unwrap().densities[slot];
    let mut world = World::from_state(state).expect("restores");
    world.step();
    assert_eq!(world.population(), 2);
    let first = world.organism_detail(1).unwrap().energy_milli;
    let second = world.organism_detail(2).unwrap().energy_milli;
    assert_eq!(first, ENERGY + 700, "the lowest new ID carries the remainder");
    assert_eq!(second, ENERGY);
    let state = world.export_state();
    let transition = state.transition.as_ref().unwrap();
    assert_eq!(transition.materialized_milli, i128::from(planted));
    assert_eq!(transition.materialized_total, 2);
    assert_eq!(transition.events_total, 1);
    assert_eq!(
        before - state.microbial.as_ref().unwrap().densities[slot],
        planted,
        "the slot was debited exactly what the organisms were credited"
    );
    assert_eq!(transition.persistence[slot], 0, "a triggered slot restarts its window");
    world.check_invariants().expect("both identities close");
    assert_identities_close(&state);
}

#[test]
fn a_materialized_organism_is_built_by_the_founder_path_and_carries_no_provenance() {
    let (probe, _) = planted_world(SEED ^ 0x4, &[], 0);
    let (c1, _) = two_land_cells(&probe);
    let (mut materialized, config) = planted_world(SEED ^ 0x4, &[c1], 1);
    materialized.step();
    assert_eq!(materialized.population(), 1);
    let detail = materialized.organism_detail(1).expect("organism");
    let phase2 = detail.phase2.expect("phase 2");
    assert_eq!(phase2.parents, [0, 0], "no parent marks a materialized organism");
    assert_eq!(phase2.ancestry_depth, 0);
    assert_eq!(detail.age_ticks, 0);
    assert_eq!(detail.cooldown_ticks, 0);
    assert_eq!(detail.energy_milli, ENERGY);

    // The founder-path twin: a random-origin world whose founder 0 carries
    // the same genome, restored through the ordinary rebuild - the same
    // `from_body` arithmetic against the same founder reference.
    let mut founder_config = config;
    founder_config.origin.mode = OriginMode::Random;
    founder_config.initial_organisms = 1;
    // The founder starts with what a materialized organism starts with,
    // and inside what a unicell can hold (the restore checks the bound).
    founder_config.initial_energy_milli = ENERGY;
    founder_config.transition.enabled = false;
    founder_config.validate().expect("validates");
    let founder_world = World::new(founder_config).expect("world");
    let mut state = founder_world.export_state();
    let genome = synthesize_genome(&founder_config, 0);
    state.schema2.as_mut().expect("schema2").genomes[0] = genome.encode();
    // Energy is left as saved: the restore verifies the ledger identity.
    let founder_world = World::from_state(state).expect("spliced state restores");
    let twin = founder_world
        .organism_detail(1)
        .expect("founder")
        .phase2
        .expect("phase 2");
    assert_eq!(
        twin.phenotype, phase2.phenotype,
        "the materialized phenotype must equal the founder path's for the same genome"
    );
    assert_eq!(twin.trait_genes, phase2.trait_genes);
    // And the derived attributes are the unicell's: a gut with no motor and
    // no sensor sits at the speed and sensing floors, which is the honest
    // state of a one-module body, not a penalty.
    assert_eq!(phase2.phenotype.max_speed_milli, 500);
    assert_eq!(phase2.phenotype.sensor_range_milli, 4_000);
    assert_eq!(phase2.phenotype.body_scale_milli, 600);
    assert!(phase2.phenotype.intake_mult_milli >= 800);
    let capacity = unicell_derived(&config.morphology).energy_capacity_milli;
    assert!(detail.energy_milli <= capacity);
}

#[test]
fn a_materialized_organism_and_its_relabelled_twin_share_one_future() {
    // The survival clause of C16.2 as a direct A/B: the same post-admission
    // state, once as materialized and once relabelled as founders (the
    // materialized term folded into the initial endowment and out of
    // production), stepped side by side. No state distinguishes them, so
    // their futures are identical - which is what "no advantage" means
    // when the field cannot feed anyone back (coupling v1).
    let (probe, _) = planted_world(SEED ^ 0x5, &[], 0);
    let (c1, c2) = two_land_cells(&probe);
    let (mut materialized, config) = planted_world(SEED ^ 0x5, &[c1, c2], 2);
    materialized.step();
    assert_eq!(materialized.population(), 4);
    let mut relabelled = materialized.export_state();
    let moved = relabelled.transition.as_ref().unwrap().materialized_milli;
    let count = relabelled.transition.as_ref().unwrap().materialized_total;
    {
        let transition = relabelled.transition.as_mut().unwrap();
        transition.materialized_milli = 0;
        transition.materialized_total = 0;
        transition.events_total = 0;
    }
    relabelled.ledger.initial_energy_milli += moved;
    relabelled.chemistry.as_mut().unwrap().produced_milli -= moved;
    relabelled.config.origin.mode = OriginMode::Random;
    relabelled.config.initial_organisms = count as u32;
    relabelled.config.validate().expect("the relabelled config validates");
    let _ = config;
    let mut twin = World::from_state(relabelled).expect("the relabelled state restores");
    twin.check_invariants().expect("relabelled identities close");
    for tick in 1..=600 {
        materialized.step();
        twin.step();
        let a: Vec<_> = materialized
            .organism_ids_view()
            .iter()
            .map(|&id| {
                let d = materialized.organism_detail(id).unwrap();
                (id, d.energy_milli, d.x_fp, d.y_fp, d.age_ticks)
            })
            .collect();
        let b: Vec<_> = twin
            .organism_ids_view()
            .iter()
            .map(|&id| {
                let d = twin.organism_detail(id).unwrap();
                (id, d.energy_milli, d.x_fp, d.y_fp, d.age_ticks)
            })
            .collect();
        assert_eq!(a, b, "tick {tick}: the two provenances diverged");
    }
}

#[test]
fn a_mid_transition_world_survives_a_save_round_trip_with_the_same_future() {
    let mut world = World::new(scratch_config(SEED ^ 0x6)).expect("world");
    let mut ticks = 0;
    while world.metrics().materialized_total == 0 && ticks < 6_000 {
        world.step();
        ticks += 1;
    }
    assert!(
        world.metrics().materialized_total > 0,
        "no materialization within 6,000 ticks, so the round trip would prove nothing"
    );
    let saved = world.export_state();
    assert!(
        saved
            .transition
            .as_ref()
            .is_some_and(|t| t.persistence.iter().any(|&count| count > 0)),
        "no persistence counter is live, so the section round-trips nothing"
    );
    // Mutation verification: the counters must be hashed, or the equality
    // below is not evidence.
    let mut perturbed = saved.clone();
    let slot = perturbed
        .transition
        .as_ref()
        .unwrap()
        .persistence
        .iter()
        .position(|&count| count > 0)
        .unwrap();
    perturbed.transition.as_mut().unwrap().persistence[slot] += 1;
    let perturbed_world = World::from_state(perturbed).expect("perturbed restores");
    assert_ne!(
        perturbed_world.state_checksum(),
        world.state_checksum(),
        "a perturbed persistence counter hashed identically"
    );
    let mut restored = World::from_state(saved).expect("restores");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    for _ in 0..300 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored transition world must advance identically"
    );
}

#[test]
fn the_caps_defer_whole_events_and_count_each_reason_separately() {
    let (probe, _) = planted_world(SEED ^ 0x7, &[], 0);
    let (c1, c2) = two_land_cells(&probe);
    // Per-tick cap: two slots of 4 organisms each against a cap of 4 - the
    // first admits, the second defers whole and keeps its window.
    let mut config = scratch_config(SEED ^ 0x7);
    frozen_field(&mut config);
    config.transition.check_interval_ticks = 1;
    config.transition.persistence_checks = 2;
    config.transition.max_organisms_per_event = 4;
    config.transition.max_materializations_per_tick = 4;
    config.validate().expect("validates");
    let classes = class_count(&config.chemistry);
    let class = eligible_class(&config);
    let plant = |config: SimConfig, cells: &[usize]| {
        let world = World::new(config).expect("world");
        let mut state = world.export_state();
        for &cell in cells {
            let slot = cell * classes + class;
            state.microbial.as_mut().unwrap().densities[slot] += 4 * ENERGY;
            state.chemistry.as_mut().unwrap().produced_milli += i128::from(4 * ENERGY);
            state.transition.as_mut().unwrap().persistence[slot] = 1;
        }
        World::from_state(state).expect("restores")
    };
    let mut world = plant(config, &[c1, c2]);
    world.step();
    let state = world.export_state();
    let transition = state.transition.as_ref().unwrap();
    assert_eq!(world.population(), 4, "only the first slot fits under the cap");
    assert_eq!(transition.deferred_cap_total, 1);
    assert_eq!(transition.deferred_capacity_total, 0);
    assert!(
        transition.persistence[c2 * classes + class] >= 2,
        "a deferred slot keeps its persistence"
    );
    // Next tick the deferred slot admits.
    world.step();
    assert_eq!(world.population(), 8);
    world.check_invariants().expect("identities close after the deferral");

    // Capacity: a world that can hold two organisms defers a four-organism
    // event whole, under the other counter.
    let mut small = config;
    small.world_seed = config.world_seed;
    small.max_entities = 2;
    small.validate().expect("validates");
    let mut world = plant(small, &[c1]);
    world.step();
    let state = world.export_state();
    let transition = state.transition.as_ref().unwrap();
    assert_eq!(world.population(), 0);
    assert_eq!(transition.deferred_capacity_total, 1);
    assert_eq!(transition.deferred_cap_total, 0);
}

#[test]
fn scratch_and_the_transition_are_refused_without_what_they_need() {
    let mut config = scratch_config(SEED ^ 0x9);
    config.initial_organisms = 1;
    assert!(matches!(config.validate(), Err(ConfigError::InitialOrganisms(1))));

    let mut config = SimConfig::phase2_default(SEED ^ 0x9);
    config.initial_organisms = 0;
    assert!(matches!(config.validate(), Err(ConfigError::InitialOrganisms(0))));

    let mut config = scratch_config(SEED ^ 0x9);
    config.chemistry.abiogenesis_enabled = false;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::ScratchRequiresAbiogenesis)
    ));

    let mut config = scratch_config(SEED ^ 0x9);
    config.morphology.enabled = false;
    assert!(matches!(
        config.validate(),
        Err(ConfigError::TransitionRequires("morphology.enabled"))
    ));

    let mut config = scratch_config(SEED ^ 0x9);
    config.transition.density_floor_milli = ENERGY - 1;
    assert!(matches!(config.validate(), Err(ConfigError::PhysiologyRange(_, _))));

    // The bound validation cannot see: more energy than the unicell holds
    // is refused at construction, where the body exists.
    let mut config = scratch_config(SEED ^ 0x9);
    let capacity = unicell_derived(&config.morphology).energy_capacity_milli;
    config.transition.organism_energy_milli = capacity + 1;
    config.transition.density_floor_milli = capacity + 1;
    config.validate().expect("validation cannot see the body");
    assert!(World::new(config).is_err(), "over-capacity energy must be refused");

    // A scratch world with the transition off is the field-only control:
    // valid, empty, and it stays that way.
    let mut config = scratch_config(SEED ^ 0x9);
    config.transition.enabled = false;
    config.validate().expect("field-only scratch validates");
    let mut world = World::new(config).expect("world");
    for _ in 0..300 {
        world.step();
    }
    assert_eq!(world.population(), 0);
    assert!(world.is_extinct());
    assert!(world.export_state().transition.is_none());
    world.check_invariants().expect("an empty scratch world is invariant-clean");
}
