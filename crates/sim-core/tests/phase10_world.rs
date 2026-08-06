//! Phase 10 world integration: a morphology world runs, and its bodies are
//! derived rather than stored.
//!
//! What this establishes is that the seam is narrow. Enabling morphology
//! replaces exactly how the phenotype is computed - from a grown body rather
//! than from trait genes - and nothing else, which is what keeps a
//! morphology-off world a usable control rather than a different simulation.
//!
//! C10.3 and C10.6 are multi-seed campaign claims and are not here.

use sim_core::{LatticeKind, SimConfig, World};

fn morphology_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 4_000;
    config.cell_capacity_milli = 240_000;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config
}

fn flat_config(seed: u64) -> SimConfig {
    let mut config = morphology_config(seed);
    config.morphology.enabled = false;
    config
}

fn run(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for tick in 1..=ticks {
        world.step();
        if tick % 250 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    world.check_invariants().expect("final invariants");
    world
}

#[test]
fn a_morphology_world_runs_and_its_organisms_act() {
    let world = run(morphology_config(7), 3_000);
    let metrics = world.metrics();
    assert!(metrics.morphology_enabled);
    assert!(world.population() > 0, "the morphology world went extinct");
    assert!(
        metrics.births_total > 0,
        "nothing reproduced, so development never ran at a birth"
    );
    assert!(
        world.ledger().consumed_biomass_milli > 0,
        "nothing ever ate, so the derived intake path is not connected"
    );
    // Founders are one differentiated module, so a world that has grown
    // nothing is a world where the growth program never ran.
    assert!(metrics.bodies_grown > 0);
    assert!(
        metrics.mean_modules_milli >= 1_000,
        "bodies must have modules"
    );
}

#[test]
fn the_flat_world_is_untouched_by_morphology_existing() {
    // The rollback story: the section is config-gated, so a morphology-off
    // world takes the same code paths it did before Phase 10. Checked
    // behaviorally as well as on the config hash, because a hash match alone
    // would not prove the tick agrees.
    let disabled = flat_config(11);
    let mut fiddled = disabled;
    fiddled.morphology.caps.max_modules = 9;
    fiddled.morphology.base_node_budget = 99;
    assert_eq!(
        fiddled.stable_hash(),
        disabled.stable_hash(),
        "a disabled section moved the config hash"
    );
    let plain = run(disabled, 1_500);
    let poked = run(fiddled, 1_500);
    assert_eq!(plain.state_checksum(), poked.state_checksum());
}

#[test]
fn bodies_are_derived_so_a_restored_world_regrows_them_identically() {
    // C10.1 and C10.10 together: nothing in the save carries a body, and the
    // restored world is nevertheless identical - which is only true because
    // development is a pure function of the genome.
    let config = morphology_config(23);
    let first = run(config, 1_200);
    let second = run(config, 1_200);
    assert_eq!(first.state_checksum(), second.state_checksum());

    let state = first.export_state();
    assert!(
        state.morphology.is_some(),
        "a morphology world must save its developmental counters"
    );
    let mut restored = World::from_state(state).expect("restore");
    assert_eq!(restored.state_checksum(), first.state_checksum());
    assert!(restored.morphology_enabled());

    let mut original = run(config, 1_200);
    for _ in 0..400 {
        original.step();
        restored.step();
    }
    assert_eq!(
        original.state_checksum(),
        restored.state_checksum(),
        "a restored morphology world diverged, so a body is not a pure \
         function of its genome"
    );
}

#[test]
fn morphology_changes_the_phenotype_and_therefore_the_world() {
    // The seam has to *do* something. Two worlds identical but for
    // morphology must diverge, or the phenotype is not actually coming from
    // the body and every later campaign would be comparing a condition
    // against itself.
    let with = run(morphology_config(31), 1_500);
    let without = run(flat_config(31), 1_500);
    assert!(with.population() > 0 && without.population() > 0);
    let with_metrics = with.metrics();
    let without_metrics = without.metrics();
    assert!(without_metrics.mean_modules_milli == 0);
    assert!(with_metrics.mean_modules_milli > 0);
    // Behavioural difference, not a checksum difference: the checksum
    // includes the config hash and would differ even for an inert change.
    assert!(
        with.population() != without.population()
            || with.ledger().consumed_biomass_milli != without.ledger().consumed_biomass_milli,
        "enabling morphology changed no measured quantity, so the derived \
         phenotype is not reaching the tick"
    );
}

#[test]
fn a_bigger_brain_needs_more_body_and_the_refusals_are_counted() {
    // C10.7's coupling at world level. With the node floor set below the
    // founder network's three nodes, every child whose body grows no neural
    // tissue is refused - so the budget must bind and be counted rather than
    // silently trimming a network no genome encoded.
    let mut config = morphology_config(37);
    config.morphology.base_node_budget = 0;
    let world = run(config, 1_500);
    let metrics = world.metrics();
    assert!(
        metrics.refused_node_budget > 0,
        "the node budget never bound, so 'brain costs body' is not being enforced"
    );
    // ...and with a generous floor the same world must not refuse on budget,
    // or the refusal is unconditional rather than a coupling.
    let mut generous = morphology_config(37);
    generous.morphology.base_node_budget = 64;
    let relaxed = run(generous, 1_500);
    assert_eq!(relaxed.metrics().refused_node_budget, 0);
}

#[test]
fn both_lattices_produce_working_worlds() {
    for lattice in [LatticeKind::Square, LatticeKind::Hex] {
        let mut config = morphology_config(43);
        config.morphology.lattice = lattice;
        let world = run(config, 1_000);
        assert!(
            world.population() > 0,
            "the {} lattice world went extinct",
            lattice.name()
        );
        assert!(world.metrics().mean_modules_milli > 0);
    }
}

#[test]
fn morphology_requires_the_genome_that_carries_its_loci() {
    // Refused at config validation rather than silently ignored: a world
    // that asked for bodies and got none would report morphology metrics of
    // zero and read as a null result.
    let mut config = morphology_config(47);
    config.genome2.enabled = false;
    assert!(
        World::new(config).is_err(),
        "morphology without genome2 was accepted, so a campaign could run a \
         condition that silently does nothing"
    );
}

#[test]
fn subsystem_validation_runs_when_contest_is_disabled() {
    // A regression test for a defect Phase 10 uncovered rather than caused:
    // the genome2 and physiology cap checks had been appended to
    // `validate_contest`, which early-returns when contest is disabled. So
    // in every world without contest - which is most of them, including
    // every Phase 8 and Phase 9 campaign - those checks were silently
    // skipped. A skipped validation looks exactly like a passing one, which
    // is why it survived three phases.
    let mut config = SimConfig::phase2_default(3);
    assert!(!config.contest.enabled, "the premise is contest being off");
    config.genome2.enabled = true;
    config.genome2.caps.max_nodes = 0;
    assert!(
        config.validate().is_err(),
        "a zero genome2 cap was accepted with contest disabled"
    );

    let mut physiology = SimConfig::phase2_default(3);
    physiology.physiology.enabled = true;
    physiology.physiology.senescence_scale_ticks = 0;
    assert!(
        physiology.validate().is_err(),
        "a zero senescence scale was accepted with contest disabled"
    );

    // ...and the checks still run when contest is on, so relocating them
    // did not trade one gap for another.
    let mut both = SimConfig::phase2_default(3);
    both.contest.enabled = true;
    both.genome2.enabled = true;
    both.genome2.caps.max_nodes = 0;
    assert!(both.validate().is_err());
}

#[test]
fn a_founder_body_is_energetically_comparable_to_a_schema_2_organism() {
    // **The calibration test.** Morphology replaces how the phenotype is
    // computed, and every derived quantity has to land on the same scale the
    // trait-derived one did, or enabling morphology is a metabolic penalty
    // rather than a change of representation - and C10.3 and C10.6 would be
    // measuring that penalty instead of morphology.
    //
    // The founder body is one digestive module at unit scale, so it is the
    // reference point: it must be near-neutral on every multiplier.
    let with = World::new(morphology_config(7)).expect("world");
    let without = World::new(flat_config(7)).expect("world");
    let id = *with.organism_ids_view().first().expect("a founder");
    let flat_id = *without.organism_ids_view().first().expect("a founder");
    let body = with
        .organism_detail(id)
        .and_then(|detail| detail.phase2)
        .expect("phase 2 detail")
        .phenotype;
    let flat = without
        .organism_detail(flat_id)
        .and_then(|detail| detail.phase2)
        .expect("phase 2 detail")
        .phenotype;
    println!(
        "PHASE10-CALIB body basal={} intake={} scale={} speed={} sensor={} | \
         flat basal={} intake={} scale={} speed={} sensor={}",
        body.basal_mult_milli,
        body.intake_mult_milli,
        body.body_scale_milli,
        body.max_speed_milli,
        body.sensor_range_milli,
        flat.basal_mult_milli,
        flat.intake_mult_milli,
        flat.body_scale_milli,
        flat.max_speed_milli,
        flat.sensor_range_milli,
    );
    // Basal cost is the one that kills a world if it is wrong: it is paid
    // every tick by every organism for its whole life.
    assert!(
        (900..=1_100).contains(&body.basal_mult_milli),
        "a founder body's basal multiplier is {} - it must be near-neutral, \
         or enabling morphology is a metabolic tax on every organism",
        body.basal_mult_milli
    );
    assert!(
        (800..=1_200).contains(&body.intake_mult_milli),
        "a founder body's intake multiplier is {}",
        body.intake_mult_milli
    );
}

#[test]
fn the_fixed_morphology_control_actually_holds_morphology_fixed() {
    // C10.3's control, and it did not control. `regulatory_enabled` gated
    // point mutation only, while duplication and deletion pick runs of loci
    // without knowing what kind they are - so they duplicated and deleted
    // growth rules happily, and the "fixed morphology" arm diverged in 21 of
    // 30 campaign worlds. A control that drifts is worse than no control,
    // because it makes the treatment look less exceptional than it is.
    let mut fixed = morphology_config(53);
    fixed.genome2.mutation.duplication_q16 = 6_554;
    fixed.genome2.mutation.deletion_q16 = 655;
    fixed.genome2.mutation.regulatory_enabled = false;
    let world = run(fixed, 4_000);
    let metrics = world.metrics();
    assert!(world.population() > 0 && metrics.births_total > 0);
    assert_eq!(
        metrics.distinct_morphologies, 1,
        "the fixed-morphology control produced {} distinct bodies; it is not fixed",
        metrics.distinct_morphologies
    );

    // ...and the treatment on the same seed must diverge, or the control is
    // trivially satisfied by a world where nothing was going to happen.
    let mut evolvable = morphology_config(53);
    evolvable.genome2.mutation.duplication_q16 = 6_554;
    evolvable.genome2.mutation.deletion_q16 = 655;
    let treated = run(evolvable, 4_000);
    assert!(
        treated.metrics().distinct_morphologies > 1,
        "the evolvable arm did not diverge either, so this test proves nothing"
    );
}
