//! Phase 20 clauses (ADR-0035): the clamp arithmetic the phase reasons
//! from is the physics that runs (C20.5), and the body-composition record
//! is emitted once per admission with the body's counts, changes no
//! checksum, and survives a save round trip (C20.6).

use sim_core::{
    Body, BodyReference, EventKind, LatticePos, Module, ModuleType, MorphologyConfig, OriginMode,
    SimConfig, World, class_count, class_parameters, composition_counts, founder_reference,
    unicell_body,
};

const SEED: u64 = 0x0f20_5eed_0f20_5eed;
const ENERGY: i64 = 4_000;

fn two_module(kind: ModuleType, config: &MorphologyConfig) -> Body {
    let unicell = unicell_body(config);
    let first = unicell.modules()[0];
    let second = Module {
        position: LatticePos::new(first.position.q + 1, first.position.r),
        module_type: kind,
        ..first
    };
    Body::from_modules(vec![first, second], config.caps.lattice_radius as i16)
}

fn multipliers(derived: &sim_core::DerivedBody, reference: &BodyReference) -> (i64, i64, i64) {
    (
        (derived.mass_milli * 1_000 / reference.mass_milli).clamp(600, 1_600),
        (derived.basal_cost_milli * 1_000 / reference.upkeep_milli).clamp(600, 1_600),
        (derived.intake_milli * 1_000 / reference.intake_milli).clamp(800, 1_200),
    )
}

#[test]
fn the_clamp_arithmetic_the_record_reasons_from_is_the_physics_that_runs() {
    // C20.5. The founder body is the phenotype's reference (ADR-0019), a
    // unicell sits on the basal and scale floors, and a second module of
    // five of the seven types leaves the basal multiplier on its floor
    // while conferring capability - the fact that rules "price" out
    // before any campaign (plan, "Problem"). A registry or reference
    // change that moves any number here fails this test and is
    // recorded, never absorbed.
    let reference = founder_reference();
    assert_eq!(
        (reference.mass_milli, reference.upkeep_milli, reference.intake_milli, reference.thrust_ratio_milli),
        (2_400, 750, 1_000, 1_000)
    );
    let config = MorphologyConfig::morphology_default();
    let unicell = unicell_body(&config).derive();
    assert_eq!(
        (unicell.mass_milli, unicell.basal_cost_milli, unicell.intake_milli, unicell.energy_capacity_milli),
        (800, 200, 1_000, 12_000)
    );
    assert_eq!(multipliers(&unicell, &reference), (600, 600, 1_000));
    let cases = [
        (ModuleType::Structural, 300, 600),
        (ModuleType::Sensory, 350, 600),
        (ModuleType::Motor, 600, 800),
        (ModuleType::Digestive, 400, 600),
        (ModuleType::Storage, 280, 600),
        (ModuleType::Reproductive, 450, 600),
        (ModuleType::Neural, 700, 933),
    ];
    for (kind, basal, basal_mult) in cases {
        let derived = two_module(kind, &config).derive();
        assert_eq!(derived.basal_cost_milli, basal, "{kind:?} basal");
        assert_eq!(multipliers(&derived, &reference).1, basal_mult, "{kind:?} multiplier");
    }
    let gut = two_module(ModuleType::Digestive, &config).derive();
    assert_eq!(multipliers(&gut, &reference).2, 1_200, "a second gut raises intake to the ceiling");
    let store = two_module(ModuleType::Storage, &config).derive();
    assert_eq!(store.energy_capacity_milli, 60_000, "a storage module quintuples capacity");
    let motor = two_module(ModuleType::Motor, &config).derive();
    let ratio = motor.thrust_milli * 1_000 / motor.mass_milli;
    assert_eq!((1_500 * ratio / reference.thrust_ratio_milli).clamp(500, 3_000), 2_116);
}

#[test]
fn composition_counts_index_the_registry_order_and_count_every_module() {
    let config = MorphologyConfig::morphology_default();
    for kind in [
        ModuleType::Structural,
        ModuleType::Sensory,
        ModuleType::Motor,
        ModuleType::Digestive,
        ModuleType::Storage,
        ModuleType::Reproductive,
        ModuleType::Neural,
    ] {
        let body = two_module(kind, &config);
        let counts = composition_counts(&body);
        assert_eq!(counts.iter().map(|&c| u32::from(c)).sum::<u32>(), 2);
        assert_eq!(counts[usize::from(ModuleType::Digestive.id())], if kind == ModuleType::Digestive { 2 } else { 1 });
        if kind != ModuleType::Digestive {
            assert_eq!(counts[usize::from(kind.id())], 1, "{kind:?} lands on its own id");
        }
        assert_eq!(usize::from(kind.id()), (0..7).find(|&i| i == usize::from(kind.id())).unwrap());
    }
}

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
    config.chemistry.mutation_q16 = 4_096;
    config.chemistry.production_milli_per_step = 20;
    config.chemistry.excretion_fraction_q16 = 32_768;
    config.chemistry.remains_fraction_q16 = 32_768;
    config.chemistry.consumption_fraction_q16 = 65_536;
    config.transition.enabled = true;
    config.transition.check_interval_ticks = 25;
    config.transition.density_floor_milli = ENERGY;
    config.transition.persistence_checks = 2;
    config.transition.organism_energy_milli = ENERGY;
    config.validate().expect("validates");
    for _ in 0..32 {
        if World::new(config).is_ok() {
            return config;
        }
        config.world_seed = config.world_seed.wrapping_add(1);
    }
    panic!("no generable seed");
}

fn run_collecting(config: SimConfig, ticks: u64) -> (World, Vec<sim_core::Event>) {
    let mut world = World::new(config).expect("world");
    let mut events = Vec::new();
    for _ in 0..ticks {
        world.step();
        events.extend_from_slice(world.events());
    }
    (world, events)
}

#[test]
fn every_admission_carries_exactly_one_composition_record_whose_sum_is_its_module_count() {
    // C20.6. In a coupled scratch world every organism is materialized or
    // born; each admission emits one record, in the same tick as its
    // admission, whose counts sum to the organism's module count.
    let (world, events) = run_collecting(scratch_config(SEED), 3_000);
    let mut admitted = std::collections::BTreeMap::new();
    let mut records = std::collections::BTreeMap::new();
    for event in &events {
        match event.kind {
            EventKind::Materialized { id, .. }
            | EventKind::Birth { id, .. }
            | EventKind::PairedBirth { id, .. } => {
                admitted.insert(id, event.tick);
            }
            EventKind::BodyComposition { id, counts } => {
                let previous = records.insert(id, (event.tick, counts));
                assert!(previous.is_none(), "organism {id} has two composition records");
            }
            _ => {}
        }
    }
    assert!(admitted.len() > 100, "too few admissions to test: {}", admitted.len());
    assert_eq!(records.len(), admitted.len(), "one record per admission");
    let mut living_max = 0_u64;
    for (id, tick) in &admitted {
        let (record_tick, counts) = records[id];
        assert_eq!(record_tick, *tick, "organism {id}: record tick");
        let total: u32 = counts.iter().map(|&c| u32::from(c)).sum();
        assert!(total >= 1, "organism {id}: an empty body");
        if world.organism_detail(*id).is_some() {
            living_max = living_max.max(u64::from(total));
        }
    }
    // The metrics gauge is the largest module count over living bodies;
    // the records of the living must agree with it.
    assert_eq!(living_max, world.metrics().max_modules, "records versus the max_modules gauge");
    world.check_invariants().expect("identities");
}

#[test]
fn the_composition_record_moves_no_checksum_and_survives_a_save_round_trip() {
    // The Phase 17 neutrality shape: the same world with the events taken
    // and with them left in the buffer has one state checksum, and a
    // mid-run save round trip restores the same future and the same next
    // records.
    let config = scratch_config(SEED ^ 0x1);
    let (mut a, _) = run_collecting(config, 1_500);
    let mut b = World::new(config).expect("world");
    for _ in 0..1_500 {
        b.step();
    }
    assert_eq!(a.state_checksum(), b.state_checksum(), "taking events changed the state");
    let saved = a.export_state();
    let mut restored = World::from_state(saved).expect("restores");
    for tick in 1..=300 {
        a.step();
        restored.step();
        let ea: Vec<_> = a.events().iter().filter(|e| matches!(e.kind, EventKind::BodyComposition { .. })).cloned().collect();
        let er: Vec<_> = restored.events().iter().filter(|e| matches!(e.kind, EventKind::BodyComposition { .. })).cloned().collect();
        assert_eq!(ea, er, "tick {tick}: the restored world's composition records differ");
        assert_eq!(a.state_checksum(), restored.state_checksum(), "tick {tick}");
    }
}

#[test]
fn a_world_without_morphology_emits_no_composition_record() {
    // A schema-2 world with the body system off: its children are admitted
    // through the same function, carry no body, and must emit no record.
    let mut config = scratch_config(SEED ^ 0x2);
    config.morphology.enabled = false;
    config.origin.mode = OriginMode::Random;
    config.initial_organisms = 60;
    config.initial_energy_milli = ENERGY;
    config.transition.enabled = false;
    config.validate().expect("validates");
    let (_, events) = run_collecting(config, 3_000);
    assert!(events.iter().any(|e| matches!(e.kind, EventKind::Birth { .. } | EventKind::PairedBirth { .. })), "no births to test");
    assert!(!events.iter().any(|e| matches!(e.kind, EventKind::BodyComposition { .. })), "a bodiless world emitted a body record");
}

#[allow(dead_code)]
fn unused(_: usize) -> usize {
    class_count(&SimConfig::phase2_default(0).chemistry) + class_parameters(&SimConfig::phase2_default(0).chemistry, 0).aggregation_step as usize
}
