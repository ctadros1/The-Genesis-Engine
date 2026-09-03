//! Phase 20's reproduction test (ADR-0035, Branches B and C): what the
//! shipped genetics does with a two-module genome. A two-module genome is
//! found by applying the kernel's own recombination and structural
//! mutation to the unicell genome with keyed draws until development
//! yields a body of two or more modules; that genome is then reproduced
//! 1,000 times against the unicell genome and 1,000 times against itself,
//! each child classified by the same sequence the birth loop runs -
//! recombine, mutate (only a valid recombinant), develop, node budget -
//! into refused non-viable, refused budget, one module, or two or more.
//! The four counts sum to the draws; their values are the measurement
//! and are printed for the record, never asserted.

use sim_core::{
    DevelopCounters, EventKind, Genome2, MutationCounters, OriginMode, SimConfig, World,
    composition_counts, develop, mutate as mutate_alias, recombine2,
    synthesize_genome,
};

const SEED: u64 = 0x0f20_ba5e_0f20_ba5e;
const DRAWS: u64 = 1_000;

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
    config.chemistry.microbial_enabled = true;
    config.chemistry.abiogenesis_enabled = true;
    config.chemistry.consumption_fraction_q16 = 65_536;
    config.transition.enabled = true;
    config.validate().expect("validates");
    config
}

#[derive(Default, Debug)]
struct Outcome {
    refused_nonviable: u64,
    refused_budget: u64,
    one_module: u64,
    multi_module: u64,
    /// Type index of the module a multi-module child carries beyond the
    /// unicell's gut, histogrammed in registry order.
    extra_types: [u64; 7],
}

/// One child by the birth loop's sequence. `tick` and `child_id` key the
/// draws exactly as the world keys them.
fn child_of(
    config: &SimConfig,
    a: (&Genome2, u64),
    b: (&Genome2, u64),
    tick: u64,
    child_id: u64,
    mutation_counters: &mut MutationCounters,
    develop_counters: &mut DevelopCounters,
) -> Result<(Genome2, sim_core::Body), &'static str> {
    let mut child = recombine2(a, b, &config.genome2.meiosis, config.world_seed, tick, child_id);
    if child.validate_structure(&config.genome2.caps).is_ok() {
        let _ = mutate_alias(
            &mut child,
            &config.genome2.mutation,
            &config.genome2.caps,
            mutation_counters,
            config.world_seed,
            tick,
            child_id,
            config.plasticity_rule_draw_count(),
            config.channel_registry_version(),
        );
    }
    let body = develop(
        &child,
        config.morphology.lattice,
        &config.morphology.caps,
        develop_counters,
    )
    .map_err(|_| "nonviable")?;
    let derived = body.derive();
    let budget = config.morphology.base_node_budget + derived.node_budget();
    if child.express_network().nodes.len() as u32 > budget {
        return Err("budget");
    }
    Ok((child, body))
}

fn classify(config: &SimConfig, a: &Genome2, b: &Genome2, tick_base: u64) -> Outcome {
    let mut out = Outcome::default();
    let mut mc = MutationCounters::default();
    let mut dc = DevelopCounters::default();
    for i in 0..DRAWS {
        match child_of(config, (a, 1), (b, 2), tick_base + i, 1_000_000 + i, &mut mc, &mut dc) {
            Err("nonviable") => out.refused_nonviable += 1,
            Err(_) => out.refused_budget += 1,
            Ok((_, body)) => {
                if body.len() >= 2 {
                    out.multi_module += 1;
                    let counts = composition_counts(&body);
                    for (slot, &c) in counts.iter().enumerate() {
                        let baseline = if slot == 3 { 1 } else { 0 }; // the unicell's gut
                        if u64::from(c) > baseline {
                            out.extra_types[slot] += u64::from(c) - baseline;
                        }
                    }
                } else {
                    out.one_module += 1;
                }
            }
        }
    }
    out
}

#[test]
fn a_two_module_genome_reproduced_by_the_shipped_genetics_is_measured_not_assumed() {
    let config = scratch_config(SEED);
    let unicell = synthesize_genome(&config, 0);
    let mut mc = MutationCounters::default();
    let mut dc = DevelopCounters::default();
    // Search: keyed draws over child ids until the kernel's own mutation
    // and development produce a body above one module.
    let mut found: Option<(Genome2, u64)> = None;
    for child_id in 1..=50_000_u64 {
        if let Ok((genome, body)) =
            child_of(&config, (&unicell, 1), (&unicell, 2), 100, child_id, &mut mc, &mut dc)
            && body.len() >= 2
        {
            found = Some((genome, child_id));
            break;
        }
    }
    let (two, at) = found.expect("no two-module genome in 50,000 keyed draws from the unicell");
    let two_body = develop(&two, config.morphology.lattice, &config.morphology.caps, &mut dc).expect("develops");
    println!(
        "PHASE20-REPRO found two-module genome at draw {at}: composition {:?}",
        composition_counts(&two_body)
    );
    // Mate access: the pairing check refuses a candidate whose
    // compatibility distance exceeds `phase2.compatibility_threshold_q16`
    // (0.5 shipped). Half of that distance is the fraction of expressed
    // network homology ids not shared, so a duplication that touched the
    // network could isolate its carrier from every unicell mate.
    let distance = sim_core::compatibility_distance(&two, &unicell);
    let threshold = config.phase2.compatibility_threshold_q16 as f32 / 65_536.0;
    println!(
        "PHASE20-REPRO compatibility two-module vs unicell = {distance:.4} (threshold {threshold:.4}; {})",
        if distance > threshold { "REFUSED as a mate" } else { "accepted as a mate" }
    );
    let self_distance = sim_core::compatibility_distance(&two, &two);
    println!("PHASE20-REPRO compatibility two-module vs itself = {self_distance:.4}");
    let against_unicell = classify(&config, &two, &unicell, 200);
    let against_itself = classify(&config, &two, &two, 300);
    for (name, o) in [("vs-unicell", &against_unicell), ("vs-itself", &against_itself)] {
        let total = o.refused_nonviable + o.refused_budget + o.one_module + o.multi_module;
        assert_eq!(total, DRAWS, "{name}: the four classes must partition the draws");
        println!(
            "PHASE20-REPRO {name} draws={DRAWS} refused_nonviable={} refused_budget={} one_module={} multi_module={} extra_types={:?}",
            o.refused_nonviable, o.refused_budget, o.one_module, o.multi_module, o.extra_types
        );
    }
    // The measurement is printed; the only claims asserted are structural.
    assert!(against_unicell.one_module + against_unicell.multi_module > 0, "no viable child at all against the unicell");
}

#[test]
fn the_unicell_genome_reproduced_with_itself_yields_one_module_bodies_almost_always() {
    // The baseline the test above is read against: the unicell x unicell
    // cross, 1,000 draws, same sequence. Its multi-module count is the
    // per-birth appearance rate of a second module in the shipped world.
    let config = scratch_config(SEED ^ 0x1);
    let unicell = synthesize_genome(&config, 0);
    let o = classify(&config, &unicell, &unicell, 400);
    let total = o.refused_nonviable + o.refused_budget + o.one_module + o.multi_module;
    assert_eq!(total, DRAWS);
    println!(
        "PHASE20-REPRO unicell-x-unicell draws={DRAWS} refused_nonviable={} refused_budget={} one_module={} multi_module={} extra_types={:?}",
        o.refused_nonviable, o.refused_budget, o.one_module, o.multi_module, o.extra_types
    );
    assert!(o.one_module > DRAWS / 2, "the unicell cross should mostly reproduce one-module bodies");
    let _ = EventKind::Birth { id: 0, parent_id: 0 };
    let _ = World::new(config);
}
