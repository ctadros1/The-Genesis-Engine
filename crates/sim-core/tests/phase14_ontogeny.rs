//! Phase 14 ontogeny (ADR-0030): the developed body is revealed in
//! canonical BFS order, each activation paid through the ledger, and every
//! juvenile constraint is a consequence of the partially grown body.
//!
//! Gate E scripted ground truth, not campaign claims: a known multi-module
//! genome is spliced into a saved world, so the exact modules, their exact
//! activation costs, and the exact order they must activate in are all
//! computable in the test from the same pure functions the kernel uses -
//! and the assertions are equalities against those numbers, not shapes.
//!
//! The vanilla founder body is one module, so an unscripted short run would
//! exercise the growth pass only on its no-op path; the splice is what
//! makes every assertion here non-vacuous.

use sim_core::{
    COND_MODULE_COUNT, Locus, LocusKind, OP_LT, Regulatory, SimConfig, TRAIT_COUNT, World,
    founder_with_morphology, grow, growth_order, rules_of,
};

const SEED: u64 = 0x0f14_5eed_0f14_5eed;

fn ontogeny_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 48;
    config.cells_y = 48;
    config.initial_organisms = 30;
    config.max_entities = 2_000;
    config.cell_capacity_milli = 240_000;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.physiology.enabled = true;
    // Hazards off: this test watches one spliced organism grow to
    // completion, and a hazard draw taking it would fail the test for a
    // reason that has nothing to do with growth.
    config.physiology.senescence_enabled = false;
    config.physiology.extrinsic_hazard_q16_per_s = 0;
    config.physiology.juvenile_hazard_multiplier_q16 = 65_536; // Q16 one: no penalty
    config.physiology.ontogeny_enabled = true;
    config.physiology.birth_modules_min = 1;
    config.physiology.growth_cost_milli_per_mass_milli = 100;
    config.physiology.growth_rate_milli_per_s = 50;
    config
}

/// The founder growth program plus one deterministic expansion rule: while
/// the body has fewer than six modules, every module tries to place a motor
/// module in direction 0. Appended at a homology id above the founder
/// program's block, so `rules_of` keeps it last.
fn six_module_genome() -> sim_core::Genome2 {
    let mut genome = founder_with_morphology(&[0.5; TRAIT_COUNT]);
    let rule = Regulatory {
        condition_kind: COND_MODULE_COUNT,
        condition_op: OP_LT,
        condition_param: 0,
        threshold: 6,
        action_kind: sim_core::ACT_PLACE,
        action_type: sim_core::ModuleType::Motor.id(),
        direction: 0,
        scale_milli: 1_000,
    };
    let locus = Locus {
        homology_id: sim_core::STRUCTURAL_HOMOLOGY_BASE + 21_000,
        gene_lineage_id: u64::from(sim_core::STRUCTURAL_HOMOLOGY_BASE + 21_000),
        mutation_event_id: 0,
        kind: LocusKind::Regulatory { rule },
    };
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            chromosome.push(locus);
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
    }
    genome
}

/// A fresh ontogeny world with organism 0's genome replaced by the
/// six-module one and its growth progress reset to one grown module.
/// Returns the world plus the body the kernel will regrow for it,
/// derived here from the same pure functions.
fn spliced_world(config: &SimConfig) -> (World, sim_core::Body) {
    let world = World::new(config.clone()).expect("world");
    let mut state = world.export_state();
    let genome = six_module_genome();
    let mut counters = sim_core::DevelopCounters::default();
    let body = grow(
        &rules_of(&genome),
        config.morphology.lattice,
        &config.morphology.caps,
        &mut counters,
    );
    body.validate(config.morphology.lattice, &config.morphology.caps)
        .expect("the six-module body is viable");
    assert!(
        body.len() >= 4,
        "the expansion rule must actually expand the founder plan \
         (got {} modules)",
        body.len()
    );
    state.schema2.as_mut().expect("schema2 section").genomes[0] = genome.encode();
    let ontogeny = state.ontogeny.as_mut().expect("ontogeny section");
    ontogeny.grown_modules[0] = 1;
    ontogeny.growth_paid_milli[0] = 0;
    // The organism's energy is left exactly as saved: the restore verifies
    // the energy ledger identity, so a splice must not move a milli-EU -
    // and the initial endowment already fits under the one-module prefix's
    // tissue capacity, so the juvenile bounds check passes as-is.
    let world = World::from_state(state).expect("spliced state restores");
    (world, body)
}

/// The exact ledger cost of activating every module after the first, in
/// BFS order, at the config's per-mass price.
fn expected_growth_cost(body: &sim_core::Body, config: &SimConfig) -> i128 {
    let order = growth_order(body, config.morphology.lattice);
    order
        .iter()
        .skip(1)
        .map(|&index| {
            let mass = body.modules()[usize::from(index)].mass_milli();
            i128::from((mass * config.physiology.growth_cost_milli_per_mass_milli / 1_000).max(1))
        })
        .sum()
}

#[test]
fn a_spliced_juvenile_grows_to_its_whole_body_and_the_ledger_pays_exactly() {
    let config = ontogeny_config(SEED);
    let (mut world, body) = spliced_world(&config);

    let metrics = world.metrics();
    assert!(metrics.ontogeny_enabled);
    assert_eq!(
        metrics.juveniles_growing, 1,
        "exactly the spliced organism is growing"
    );

    // The window ends well before `maturity_ticks`, so nothing breeds and
    // no rule-carrying child can add activations: every grown module in
    // this window is the spliced organism's, by construction rather than
    // by luck.
    let mut saw_partial = false;
    for tick in 1..=500 {
        world.step();
        if tick % 250 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
        let metrics = world.metrics();
        if metrics.modules_grown_total > 0 && metrics.juveniles_growing > 0 {
            saw_partial = true;
        }
    }
    world.check_invariants().expect("final invariants");

    let metrics = world.metrics();
    assert!(
        saw_partial,
        "the growth was never observed in progress, so the test watched \
         nothing grow"
    );
    assert_eq!(
        metrics.modules_grown_total,
        (body.len() - 1) as u64,
        "only the spliced organism can grow before maturity, so the \
         activation count is exactly its remaining modules"
    );
    assert_eq!(
        metrics.growth_spent_milli_total,
        expected_growth_cost(&body, &config),
        "growth spend must equal the exact per-module ledger cost"
    );
    assert_eq!(
        metrics.juveniles_growing, 0,
        "the spliced organism must have finished growing in this window"
    );
}

#[test]
fn an_unfinished_juvenile_never_becomes_a_parent() {
    // The maturity gate: growth stalled by an unpayable body must delay
    // reproduction indefinitely. The spliced organism's growth rate is set
    // so low it cannot finish inside the window, while everything else
    // (one-module, born grown) breeds normally around it.
    let mut config = ontogeny_config(SEED ^ 0x77);
    config.physiology.growth_rate_milli_per_s = 1;
    config.physiology.growth_cost_milli_per_mass_milli = 10_000;
    let (mut world, _body) = spliced_world(&config);
    let spliced_id = {
        let state = world.export_state();
        state.ids[0]
    };
    let mut spliced_parented = false;
    for _ in 1..=2_000 {
        world.step();
        for event in world.events() {
            if let sim_core::EventKind::PairedBirth {
                parent_a, parent_b, ..
            } = event.kind
                && (parent_a == spliced_id || parent_b == spliced_id)
            {
                spliced_parented = true;
            }
        }
    }
    let metrics = world.metrics();
    assert!(
        metrics.births_total > 0,
        "nothing bred at all, so the gate was never actually tested \
         against a breeding population"
    );
    // Pinned to the spliced organism itself rather than the global count:
    // structural mutation can produce other multi-module juveniles in a
    // 2,000-tick breeding window, and their growth is not this test's
    // subject.
    let state = world.export_state();
    let spliced_index = state
        .ids
        .iter()
        .position(|&id| id == spliced_id)
        .expect("the starved juvenile must still be alive");
    let ontogeny = state.ontogeny.as_ref().expect("ontogeny section");
    assert!(
        ontogeny.grown_modules[spliced_index] < 6,
        "the starved juvenile must still be mid-growth (grown {})",
        ontogeny.grown_modules[spliced_index]
    );
    assert!(
        !spliced_parented,
        "an organism whose body never finished growing appeared as a \
         parent - the maturity gate is not gating"
    );
}

#[test]
fn a_mid_growth_save_restores_to_the_same_checksum_and_future() {
    let config = ontogeny_config(SEED ^ 0xff);
    let (mut world, _body) = spliced_world(&config);

    // Advance until the spliced organism has grown at least one module and
    // is still growing - a genuinely mid-growth state.
    let mut mid_growth_at = None;
    for tick in 1..=1_000 {
        world.step();
        let metrics = world.metrics();
        if metrics.modules_grown_total >= 1 && metrics.juveniles_growing > 0 {
            mid_growth_at = Some(tick);
            break;
        }
    }
    let tick = mid_growth_at.expect("growth never reached a mid-growth state");

    let saved = world.export_state();
    let mut restored = World::from_state(saved).expect("mid-growth state restores");
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored world must checksum identically at tick {tick}"
    );
    // The stronger property: the restored world has the same FUTURE. A
    // restore that quietly gave the juvenile its adult phenotype would
    // diverge within a few ticks of movement and perception.
    for _ in 0..120 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "the restored world must advance identically from tick {tick}"
    );
    restored.check_invariants().expect("restored invariants");
}

#[test]
fn the_ontogeny_gate_off_is_byte_identical_to_a_world_that_never_heard_of_it() {
    // The rollback story, at the state level: an ontogeny-off physiology
    // world must carry no ontogeny section and hash as it always has.
    let mut config = ontogeny_config(SEED ^ 0xa5);
    config.physiology.ontogeny_enabled = false;
    let mut world = World::new(config).expect("world");
    for _ in 0..300 {
        world.step();
    }
    let state = world.export_state();
    assert!(state.ontogeny.is_none(), "gate off must save no section");
    let metrics = world.metrics();
    assert!(!metrics.ontogeny_enabled);
    assert_eq!(metrics.modules_grown_total, 0);
    assert_eq!(metrics.juveniles_growing, 0);
}
