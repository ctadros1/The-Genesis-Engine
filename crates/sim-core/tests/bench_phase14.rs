//! Phase 14 benchmarks: what ontogeny and mate choice cost per tick,
//! emitted as `PHASE14-BENCH` markers collected by
//! `scripts/run-phase14-benchmarks.sh`.
//!
//! The arms isolate the seam from the work, as every phase's benchmark
//! does. `disabled` is a morphology+physiology world with both ADR-0030
//! gates off; `steady` turns the gates on over a fully grown population,
//! which prices the standing overhead - the growth pass's skip-if-grown
//! scan and the pairing path's gate checks - with nothing actually
//! growing or choosing; `growing` splices every founder to a six-module
//! body at one grown module with the growth rate slowed so payment and
//! activation span the whole sample window; `choosing` scripts every
//! founder to intend mating every tick with a floor-level pairing
//! threshold, so preference scoring and the choice event run at the
//! pairing cadence the mechanism actually has. The plan's headline number
//! is the per-organism delta of `steady` over `disabled`: the price every
//! campaign organism pays whether or not it is a juvenile or a chooser.

use sim_core::{
    Activation, COND_MODULE_COUNT, Genome2, GenomeCaps, Locus, LocusKind, ModuleType, NodeRole,
    OP_LT, Regulatory, STRUCTURAL_HOMOLOGY_BASE, SimConfig, World,
};
use std::time::Instant;

const SEED: u64 = 0x5eed_cafe_f00d_beef;
const WARMUP_TICKS: u64 = 100;
const SAMPLE_TICKS: u64 = 500;
/// The mate output channel: `registry.rs` names output 106 "mate".
const CHANNEL_MATE: u16 = 106;

fn median(samples: &mut [f64]) -> f64 {
    samples.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    samples[samples.len() / 2]
}

fn base_config(gates: bool, founders: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 128;
    config.cells_y = 128;
    config.initial_organisms = founders;
    config.max_entities = 4_000;
    config.genome2.enabled = true;
    config.morphology.enabled = true;
    config.physiology.enabled = true;
    config.physiology.senescence_enabled = false;
    config.physiology.extrinsic_hazard_q16_per_s = 0;
    config.physiology.juvenile_hazard_multiplier_q16 = 65_536;
    if gates {
        config.physiology.ontogeny_enabled = true;
        config.physiology.birth_modules_min = 1;
        config.physiology.growth_cost_milli_per_mass_milli = 100;
        config.physiology.growth_rate_milli_per_s = 50;
        config.physiology.mate_choice_enabled = true;
    }
    config.validate().expect("validates");
    config
}

/// Surgery over the save path: optionally give every founder the
/// six-module growth program with one grown module, and optionally an
/// always-on mate-intent output.
fn scripted_world(config: SimConfig, six_module: bool, always_mate: bool) -> World {
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let schema2 = state.schema2.as_mut().expect("schema 2");
    const MATE_NODE: u32 = STRUCTURAL_HOMOLOGY_BASE + 60_000;
    const GROWTH_RULE: u32 = STRUCTURAL_HOMOLOGY_BASE + 61_000;
    for index in 0..schema2.genomes.len() {
        let mut genome = Genome2::decode(&schema2.genomes[index], &caps).expect("decodes");
        for haplotype in &mut genome.haplotypes {
            let chromosome = &mut haplotype.chromosomes[0];
            if always_mate {
                chromosome.push(Locus {
                    homology_id: MATE_NODE,
                    gene_lineage_id: u64::from(MATE_NODE),
                    mutation_event_id: 0,
                    kind: LocusKind::Node {
                        role: NodeRole::Output,
                        activation_id: Activation::TanhApprox.id(),
                        bias: 8.0,
                        time_constant: 0,
                    },
                });
                chromosome.push(Locus {
                    homology_id: MATE_NODE + 1,
                    gene_lineage_id: u64::from(MATE_NODE + 1),
                    mutation_event_id: 0,
                    kind: LocusKind::IoBinding {
                        node: MATE_NODE,
                        channel_id: CHANNEL_MATE,
                        gain: 1.0,
                    },
                });
            }
            if six_module {
                chromosome.push(Locus {
                    homology_id: GROWTH_RULE,
                    gene_lineage_id: u64::from(GROWTH_RULE),
                    mutation_event_id: 0,
                    kind: LocusKind::Regulatory {
                        rule: Regulatory {
                            condition_kind: COND_MODULE_COUNT,
                            condition_op: OP_LT,
                            condition_param: 0,
                            threshold: 6,
                            action_kind: sim_core::ACT_PLACE,
                            action_type: ModuleType::Motor.id(),
                            direction: 0,
                            scale_milli: 1_000,
                        },
                    },
                });
            }
            chromosome.sort_unstable_by_key(|locus| locus.homology_id);
        }
        genome.validate_structure(&caps).expect("validates");
        schema2.genomes[index] = genome.encode();
        if always_mate {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    if six_module
        && let Some(ontogeny) = state.ontogeny.as_mut()
    {
        for grown in ontogeny.grown_modules.iter_mut() {
            *grown = 1;
        }
        for paid in ontogeny.growth_paid_milli.iter_mut() {
            *paid = 0;
        }
    }
    if always_mate {
        for age in state.age_ticks.iter_mut() {
            *age = 700;
        }
    }
    World::from_state(state).expect("restores")
}

fn tick_cost_of(mut world: World) -> (f64, World) {
    for _ in 0..WARMUP_TICKS {
        world.step();
    }
    let mut samples = Vec::with_capacity(SAMPLE_TICKS as usize);
    for _ in 0..SAMPLE_TICKS {
        let started = Instant::now();
        world.step();
        samples.push(started.elapsed().as_secs_f64() * 1_000_000.0);
    }
    (median(&mut samples), world)
}

#[test]
#[ignore = "timed benchmark; run with --ignored"]
fn physiology_v2_tick_cost_disabled_steady_growing_choosing() {
    for founders in [200_u32, 1_000] {
        let (disabled_us, disabled) =
            tick_cost_of(World::new(base_config(false, founders)).expect("w"));
        let (steady_us, steady) = tick_cost_of(World::new(base_config(true, founders)).expect("w"));
        // The growing arm's rate is computed from the six-module body's
        // own exact cost so the window ends mid-growth by construction:
        // fast enough that activations happen inside it, slow enough that
        // juveniles remain at its end - guessing module masses here failed
        // both ways before this was computed.
        let mut growing_config = base_config(true, founders);
        growing_config.physiology.growth_cost_milli_per_mass_milli = 1_000;
        let mut counters = sim_core::DevelopCounters::default();
        let mut rules = sim_core::rules_of(&sim_core::founder_with_morphology(
            &[0.5; sim_core::TRAIT_COUNT],
        ));
        rules.push((
            STRUCTURAL_HOMOLOGY_BASE + 61_000,
            Regulatory {
                condition_kind: COND_MODULE_COUNT,
                condition_op: OP_LT,
                condition_param: 0,
                threshold: 6,
                action_kind: sim_core::ACT_PLACE,
                action_type: ModuleType::Motor.id(),
                direction: 0,
                scale_milli: 1_000,
            },
        ));
        let body = sim_core::grow(
            &rules,
            growing_config.morphology.lattice,
            &growing_config.morphology.caps,
            &mut counters,
        );
        let order = sim_core::growth_order(&body, growing_config.morphology.lattice);
        let total_cost: i64 = order
            .iter()
            .skip(1)
            .map(|&index| {
                (body.modules()[usize::from(index)].mass_milli()
                    * growing_config.physiology.growth_cost_milli_per_mass_milli
                    / 1_000)
                    .max(1)
            })
            .sum();
        // Aim to have paid ~60% of the total by the window's end.
        let window = (WARMUP_TICKS + SAMPLE_TICKS) as i64;
        let budget_per_tick = (total_cost * 3 / 5 / window).max(1);
        growing_config.physiology.growth_rate_milli_per_s =
            (budget_per_tick * 1_000 / i64::from(growing_config.dt_ms)).max(10);
        let (growing_us, growing) = tick_cost_of(scripted_world(growing_config, true, false));
        let mut choosing_config = base_config(true, founders);
        choosing_config.phase2.pairing_energy_threshold_milli = 100;
        choosing_config.morphology.base_node_budget = 96;
        let (choosing_us, choosing) = tick_cost_of(scripted_world(choosing_config, false, true));
        let growing_metrics = growing.metrics();
        let choosing_metrics = choosing.metrics();
        assert!(
            growing_metrics.juveniles_growing > 0,
            "the growing arm finished growing inside the window, so its \
             number prices nothing (evidence trap 1)"
        );
        assert!(
            growing_metrics.modules_grown_total > 0,
            "the growing arm never activated a module, so its number \
             prices a scan and calls it growth (evidence trap 1 - and the \
             rate-floors-to-zero validation exists because this assert \
             once failed silently)"
        );
        assert!(
            choosing_metrics.choices_total > 0,
            "the choosing arm never chose, so its number prices nothing \
             (evidence trap 1)"
        );
        println!(
            "PHASE14-BENCH physiology-tick founders={} disabled_us={:.1} \
             disabled_population={} steady_us={:.1} steady_population={} \
             growing_us={:.1} growing_juveniles={} modules_grown={} \
             choosing_us={:.1} choices={} choosing_population={}",
            founders,
            disabled_us,
            disabled.population(),
            steady_us,
            steady.population(),
            growing_us,
            growing_metrics.juveniles_growing,
            growing_metrics.modules_grown_total,
            choosing_us,
            choosing_metrics.choices_total,
            choosing.population(),
        );
    }
}
