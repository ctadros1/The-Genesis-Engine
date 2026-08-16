//! Phase 11 lifetime learning: the tick integration, not the arithmetic.
//!
//! The update arithmetic is unit-tested in `plasticity.rs` and the compile
//! and evaluation seams in `controller2.rs`. What is left, and what this file
//! is for, is everything that only exists once a whole world is running:
//! that learned state actually moves, that a child starts at zero, that the
//! energy ledger stays exact with a new debit path in it, that effective
//! weight never leaves its bound in a real population, and that a world with
//! the section disabled is byte-identical to the world it was before Phase 11
//! existed.
//!
//! # Getting plastic edges into a world without a hand-authored genome
//!
//! Founders are `minimal_founder`: two edges, both flagged non-plastic with
//! inert genes. Only point mutation can flip a flag, and it takes
//! generations, so a test that waited for evolution to produce a plastic edge
//! would be slow, seed-dependent, and would fail for reasons that have
//! nothing to do with what it is asserting.
//!
//! `plastic_world` instead builds an ordinary world, exports it, rewrites
//! every organism's genome to the same genome with its two edges flagged
//! plastic, and restores. That is entirely through the public codec, it
//! preserves each founder's own traits, and it exercises the restore path as
//! a side effect. **The genomes it produces are legal records**: the plastic
//! flag and the plasticity genes are ordinary schema-2 fields, and
//! `from_state` decodes and validates them through the same fail-closed path
//! any other save takes.

use sim_core::{
    EventKind, Genome2, GenomeCaps, InheritanceMode, LearnedEdgeSave, LocusKind, PlasticityBudget,
    PlasticityGenes, RULE_HEBBIAN, RestoreError, SaveState, SimConfig, World,
    compile_network_with_budget,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

/// The Phase 9 fixture's configuration, pinned field by field (D-078).
///
/// Duplicated from `phase9_determinism.rs` rather than shared, deliberately:
/// the point of pinning is that the fixture does not move when a default
/// does, and a helper imported from elsewhere is one more place a default
/// could leak in from.
fn phase9_fixture_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.phase2.enabled = true;
    config.genome2.enabled = true;

    let caps = &mut config.genome2.caps;
    caps.max_chromosomes = 4;
    caps.max_loci_per_chromosome = 160;
    caps.max_nodes = 160;
    caps.max_edges = 160;
    caps.max_edges_per_node = 32;
    caps.max_genome_bytes = 16_384;
    caps.min_nodes = 2;

    config.genome2.meiosis.mode = InheritanceMode::Meiotic;
    config.genome2.meiosis.max_extra_crossovers = 2;

    let mutation = &mut config.genome2.mutation;
    mutation.point_q16 = 6_554;
    mutation.duplication_q16 = 655;
    mutation.deletion_q16 = 655;
    mutation.insertion_q16 = 0;
    mutation.transposition_q16 = 328;
    mutation.regulatory_enabled = true;
    mutation.max_run = 3;
    mutation.point_delta_q16 = 3_277;
    mutation.plasticity_enabled = false;
    config.plasticity.enabled = false;
    config
}

/// A small schema-2 world with the plasticity section live.
fn learning_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase11_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;
    // Off the default so the cost path is exercised at a rate that is visible
    // in the ledger without dominating metabolism: two plastic edges at 20
    // milli/s is 4 milli per tick against a basal 10.
    config.plasticity.plastic_edge_cost_milli_per_s = 20;
    config
}

/// Rewrite one genome so every edge locus is plastic under `rule_id`.
///
/// Node count, edge count, sources, targets, weights and bindings are all
/// untouched, so the restored organism has the same network it had - only its
/// edges now learn. That matters: it makes the plastic world and its control
/// the same topology, so a difference between them is plasticity and not
/// structure.
fn make_every_edge_plastic(genome: &mut Genome2, rule_id: u8, eta: f32) {
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            for locus in chromosome.iter_mut() {
                if let LocusKind::Edge {
                    flags, plasticity, ..
                } = &mut locus.kind
                {
                    *flags |= sim_core::EDGE_FLAG_PLASTIC;
                    *plasticity = PlasticityGenes {
                        rule_id,
                        eta,
                        // a = 1: the delta is exactly x*y, the plainest
                        // correlation the registry offers, so "did anything
                        // learn" is not a question about coefficient
                        // cancellation.
                        coefficients: [1.0, 0.0, 0.0, 0.0],
                        decay: 0.0,
                        modulator_node: 0,
                    };
                }
            }
        }
    }
}

/// Add one plastic edge that **no behaviour reads**.
///
/// A new Hidden node fed from the founder's input node by a plastic edge. The
/// node binds no output channel and nothing reads it, so the organism's
/// intents are exactly what they were - and that is the point.
///
/// `make_every_edge_plastic` at a high learning rate does not leave a
/// population to test with: the founder's two edges are the whole path from
/// the energy sensor to the turn channel, so a saturated learned delta pins
/// the turn output at 1.0, every organism orbits in a tight circle, grazes
/// out its own cell and starves. Measured, not assumed: 120 founders fall to
/// 6 by tick 5,000 with no births at all, against 79 alive and 32 births in
/// the same world with the section off. That collapse is a real observation
/// about runaway plasticity - it is the risk the phase plan names - but it
/// makes "a child of parents with large learned deltas" a state the world
/// never reaches, and a birth-reset test needs parents.
///
/// Separating the two lets each test say one thing: this edge learns to
/// saturation without touching behaviour, so births happen at the control's
/// rate while every parent carries the largest delta the clamp allows.
fn add_neutral_plastic_edge(genome: &mut Genome2, rule_id: u8, eta: f32) {
    const BASE: u32 = sim_core::STRUCTURAL_HOMOLOGY_BASE;
    // `minimal_founder`'s input node. Its activation is the energy fraction,
    // which is strictly positive, so `x*y` is a real drive rather than a
    // signal that happens to sit at zero.
    const FOUNDER_INPUT: u32 = BASE + 1_000;
    const NEUTRAL_NODE: u32 = BASE + 8_000;
    const NEUTRAL_EDGE: u32 = BASE + 9_000;
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            // Appended, and the founder's largest structural id is
            // BASE + 7_000, so the chromosome stays sorted by `homology_id`
            // as `validate_structure` requires.
            chromosome.push(sim_core::Locus {
                homology_id: NEUTRAL_NODE,
                gene_lineage_id: u64::from(NEUTRAL_NODE),
                mutation_event_id: 0,
                kind: LocusKind::Node {
                    role: sim_core::NodeRole::Hidden,
                    activation_id: sim_core::Activation::TanhApprox.id(),
                    bias: 0.0,
                    time_constant: 0,
                },
            });
            chromosome.push(sim_core::Locus {
                homology_id: NEUTRAL_EDGE,
                gene_lineage_id: u64::from(NEUTRAL_EDGE),
                mutation_event_id: 0,
                kind: LocusKind::Edge {
                    source: FOUNDER_INPUT,
                    target: NEUTRAL_NODE,
                    weight: 1.0,
                    flags: sim_core::EDGE_FLAG_PLASTIC,
                    plasticity: PlasticityGenes {
                        rule_id,
                        eta,
                        coefficients: [1.0, 0.0, 0.0, 0.0],
                        decay: 0.0,
                        modulator_node: 0,
                    },
                },
            });
        }
    }
}

/// The plastic edges a genome expresses under `budget`, by `homology_id`.
///
/// Computed through the same public compile the world uses rather than read
/// off the loci, so a rewritten save describes the plan the restore will
/// actually build - including the budget truncation, which drops edges the
/// loci still flag.
fn plastic_edge_ids(genome: &Genome2, budget: PlasticityBudget) -> Vec<u32> {
    compile_network_with_budget(&genome.express_network(), budget)
        .expect("a rewritten genome compiles")
        .plastic_edges
        .iter()
        .map(|edge| edge.homology_id)
        .collect()
}

/// Build a world by rewriting every founder genome through the public codec.
///
/// `extra_nodes` is how many nodes the rewrite added, because the saved
/// activation vectors are sized to the old network and `from_state` refuses a
/// length mismatch - correctly, since that check is what catches a save whose
/// genome and activation buffer disagree.
///
/// The learn section has to be rewritten too, and that is not bookkeeping:
/// the exported world had no plastic edge anywhere, so its learn rows are all
/// empty, and a genome rewritten to carry two plastic edges no longer matches
/// them. `from_state` refuses that outright - which is the whole point of the
/// edge-id check - so the rewrite supplies rows that name the edges the
/// rewritten genome expresses, at zero. It is the same shape a founder push
/// produces, arrived at through the save path.
fn rewritten_world(
    config: SimConfig,
    extra_nodes: usize,
    expected_plastic: u32,
    rewrite: impl Fn(&mut Genome2),
) -> World {
    let world = World::new(config).expect("world");
    let mut state = world.export_state();
    let caps: GenomeCaps = state.config.genome2.caps;
    let budget = state.config.plasticity_budget();
    let mut learn_rows: Vec<Vec<LearnedEdgeSave>> = Vec::new();
    let schema2 = state.schema2.as_mut().expect("a schema-2 world");
    for index in 0..schema2.genomes.len() {
        let mut genome =
            Genome2::decode(&schema2.genomes[index], &caps).expect("a live genome decodes");
        rewrite(&mut genome);
        learn_rows.push(
            plastic_edge_ids(&genome, budget)
                .into_iter()
                .map(|edge_homology_id| LearnedEdgeSave {
                    edge_homology_id,
                    learned_q16: 0,
                    trace_q16: 0,
                })
                .collect(),
        );
        schema2.genomes[index] = genome.encode();
        for _ in 0..extra_nodes {
            schema2.activation_values[index].push(0.0);
            schema2.activation_prior[index].push(0.0);
        }
    }
    if let Some(learn) = state.learn.as_mut() {
        learn.edges = learn_rows;
    }
    let world = World::from_state(state).expect("the rewritten genomes restore");
    let census = world.learned_census();
    assert_eq!(census.len(), world.population());
    assert!(
        census
            .iter()
            .all(|sample| sample.plastic_edges == expected_plastic),
        "the rewrite did not reach the compiled plans, so every assertion \
         below would be about a world with no plastic edges"
    );
    world
}

/// A world whose founders' own two edges are plastic.
fn plastic_world(config: SimConfig, rule_id: u8, eta: f32) -> World {
    rewritten_world(config, 0, 2, |genome| {
        make_every_edge_plastic(genome, rule_id, eta)
    })
}

/// A world whose founders carry one plastic edge that nothing reads.
fn neutral_plastic_world(config: SimConfig, rule_id: u8, eta: f32) -> World {
    rewritten_world(config, 1, 1, |genome| {
        add_neutral_plastic_edge(genome, rule_id, eta)
    })
}

fn run(world: &mut World, ticks: u64) {
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
}

#[test]
fn the_learn_phase_moves_learned_state_in_a_running_world() {
    let mut world = plastic_world(learning_config(SEED), RULE_HEBBIAN, 0.01);
    // Nothing has learned yet, and the mechanism has not run yet: both
    // halves are asserted, because a metric that is zero because the phase
    // never fired reads exactly like a metric that is zero because nothing
    // changed.
    let before = world.metrics();
    assert_eq!(before.mean_abs_learned_milli, 0);
    assert_eq!(before.plasticity_updates_total, 0);
    assert!(before.plasticity_enabled);
    assert_eq!(before.plastic_edges_total, 2 * world.population() as u64);

    run(&mut world, 200);
    let after = world.metrics();

    // The phase ran on every plastic edge of every organism alive, every
    // tick. A count that fell short would mean the loop skipped edges.
    assert!(
        after.plasticity_updates_total >= 200 * before.plastic_edges_total / 2,
        "the learn phase did not visit the edges it should have: {}",
        after.plasticity_updates_total
    );
    assert!(
        after.mean_abs_learned_milli > 0,
        "200 ticks of plasticity left every learned delta at zero"
    );
    assert_eq!(after.plasticity_anomalies_total, 0, "faults or saturation");
    assert!(
        world
            .learned_census()
            .iter()
            .any(|sample| sample.sum_abs_learned_q16 > 0),
        "no individual moved, so the population mean is an artifact"
    );

    // The control that makes the assertion above about *plasticity*: the
    // same world with the section disabled runs the same organisms over the
    // same seed and learns nothing, because there is nothing to learn with.
    let mut control_config = learning_config(SEED);
    control_config.plasticity.enabled = false;
    let mut control = World::new(control_config).expect("world");
    run(&mut control, 200);
    assert!(control.learned_census().is_empty());
    assert_eq!(control.metrics().plasticity_updates_total, 0);
}

#[test]
fn a_child_of_parents_with_large_learned_deltas_starts_at_exactly_zero() {
    // C11.4, asserted directly on the child rather than inferred from a
    // population statistic. eta 1 with a = 1 drives the delta to the clamp
    // within tens of ticks, so by the time anything reproduces every parent
    // is carrying the largest learned state the arithmetic permits - which is
    // the state a Lamarckian leak would have the most to leak.
    let mut world = neutral_plastic_world(learning_config(SEED), RULE_HEBBIAN, 1.0);
    run(&mut world, 900);

    // Every organism old enough to have learned is pinned at the clamp.
    // Restricted by age rather than asserted over the whole census, because
    // by tick 900 the first children have already been born and are sitting
    // at zero - which is the property under test, not a counterexample to it,
    // and folding them into this count would make the assertion circular.
    let census = world.learned_census();
    let grown: Vec<_> = census
        .iter()
        .filter(|sample| sample.age_ticks >= 200)
        .collect();
    assert!(grown.len() > 50, "too few grown organisms: {}", grown.len());
    assert!(
        grown
            .iter()
            .all(|sample| sample.max_abs_learned_q16 == sim_core::LEARN_LIMIT_Q16),
        "not every parent is pinned at the clamp, so a child starting at zero \
         would be a weaker claim than C11.4 makes"
    );

    // Step until a birth happens, then find the newborns by age. A child is
    // pushed at the end of `lifecycle` with age 0 and is aged on the next
    // tick's apply pass, so "age_ticks == 0" is exactly "born this tick".
    let mut newborns = 0_usize;
    for _ in 0..4_000 {
        world.step();
        let births = world
            .events()
            .iter()
            .filter(|event| matches!(event.kind, EventKind::PairedBirth { .. }))
            .count();
        if births == 0 {
            continue;
        }
        for sample in world.learned_census() {
            if sample.age_ticks > 0 {
                continue;
            }
            newborns += 1;
            assert!(
                sample.plastic_edges > 0,
                "a newborn with no plastic edges cannot demonstrate a reset"
            );
            assert_eq!(
                sample.sum_abs_learned_q16, 0,
                "a newborn inherited a learned delta"
            );
            assert_eq!(sample.sum_abs_trace_q16, 0, "a newborn inherited a trace");
            assert_eq!(sample.faults, 0);
        }
        if newborns >= 4 {
            break;
        }
    }
    assert!(
        newborns >= 4,
        "only {newborns} newborns were observed, which is too few for this \
         to be evidence about the birth path"
    );
    world.check_invariants().expect("invariants");
}

#[test]
fn the_energy_ledger_stays_exact_with_plasticity_costs_flowing_through_it() {
    // C11.6. `check_invariants` compares the ledger against the summed energy
    // with **no tolerance**, so it is the assertion; everything else here is
    // what stops it from being satisfied by a cost path that charged nothing.
    let mut control_config = learning_config(SEED);
    control_config.plasticity.enabled = false;
    let mut control = World::new(control_config).expect("world");
    let mut world = neutral_plastic_world(learning_config(SEED), RULE_HEBBIAN, 0.25);

    // **The exact assertion, taken on the first tick.** The plastic edge is
    // behaviourally inert and every learned delta is still zero, so the two
    // worlds do exactly the same thing on tick 1 and the whole difference in
    // spending is the plasticity debit: one edge, at 20 milli/s over a 100 ms
    // tick, for every organism alive. An approximate version of this test
    // would pass on a cost that was off by a factor, and a long-run version
    // cannot be exact because the two populations diverge.
    let population = world.population() as i128;
    let per_edge_tick = 20 * 100 / 1_000;
    let control_before = control.ledger().spent_milli;
    let world_before = world.ledger().spent_milli;
    control.step();
    world.step();
    let expected = population * per_edge_tick;
    assert_eq!(
        (world.ledger().spent_milli - world_before)
            - (control.ledger().spent_milli - control_before),
        expected,
        "the plasticity debit is not exactly one edge per organism per tick"
    );
    assert_eq!(world.metrics().plasticity_cost_milli, expected as i64);
    assert_eq!(control.metrics().plasticity_cost_milli, 0);

    // ...and it stays exact over a long run, where the debit interacts with
    // starvation flooring, births, and deaths.
    run(&mut world, 5_000);
    world.check_invariants().expect("the ledger balances");
    run(&mut control, 5_000);
    control.check_invariants().expect("the ledger balances");

    let metrics = world.metrics();
    assert!(
        metrics.plasticity_cost_milli > expected as i64,
        "the cost stopped accruing after the first tick"
    );
    // The plasticity cost is inside the total, never beside it: an unledgered
    // debit is what the invariant catches, and this is what shows the number
    // reported is the same money.
    assert!(world.ledger().spent_milli - world_before > i128::from(metrics.plasticity_cost_milli));
    assert!(
        metrics.births_total > 0,
        "nothing reproduced, so the long run \
         never exercised the birth path the debit has to survive"
    );
}

#[test]
fn effective_weight_never_leaves_its_bound_in_a_running_population() {
    // C11.5's bounds clause at world level. eta 1 with no decay is the most
    // aggressive setting the genome validator permits, so the clamp is under
    // real pressure rather than being asserted on values that never got near
    // it.
    let mut world = plastic_world(learning_config(SEED), RULE_HEBBIAN, 1.0);
    let mut saturated = 0_usize;
    let mut checked = 0_usize;
    for _ in 0..600 {
        world.step();
        for weight in world.plastic_effective_weights() {
            assert!(weight.is_finite(), "a non-finite effective weight");
            assert!(
                (-8.0..=8.0).contains(&weight),
                "effective weight left [-8, 8]: {weight}"
            );
            checked += 1;
            if weight.abs() >= 7.9 {
                saturated += 1;
            }
        }
        for sample in world.learned_census() {
            assert!(sample.max_abs_learned_q16 <= sim_core::LEARN_LIMIT_Q16);
        }
    }
    assert!(checked > 10_000, "only {checked} weights were checked");
    assert!(
        saturated > 0,
        "nothing ever approached the bound, so the bound was never tested"
    );
    world.check_invariants().expect("invariants");
}

#[test]
fn a_plasticity_disabled_schema_2_world_reproduces_the_phase_9_fixture() {
    // C11.8. The fixture is 8,000 ticks because that is the horizon at which
    // it stops being a control: `maturity_age_ticks` is 600, so a shorter run
    // pins meiosis, structural mutation and the schema-2 birth path by
    // pinning none of them.
    let config = phase9_fixture_config();
    assert_eq!(
        config.stable_hash(),
        0x9abc_0cd4_7914_127f,
        "the plasticity section moved the Phase 9 config hash while disabled"
    );
    let mut world = World::new(config).expect("world");
    run(&mut world, 8_000);
    assert_eq!(
        world.state_checksum(),
        0x5f0c_4e95_e4f5_170f,
        "a disabled plasticity section changed a schema-2 world's state"
    );
    // The fixture is not vacuous: the mechanisms it claims to pin ran.
    let metrics = world.metrics();
    assert!(metrics.births_total > 0 && metrics.structural_mutations_applied > 0);
    // ...and nothing plastic exists to be hashed.
    assert!(!metrics.plasticity_enabled);
    assert_eq!(metrics.plastic_edges_total, 0);
    assert!(world.learned_census().is_empty());

    // **The control, and the reason the equality above is not vacuous.**
    // Enabling the section must move the config hash, or "disabled is inert"
    // would be a statement about a field nothing reads.
    let mut enabled = phase9_fixture_config();
    enabled.plasticity.enabled = true;
    assert_ne!(enabled.stable_hash(), 0x9abc_0cd4_7914_127f);
    let mut mutating = phase9_fixture_config();
    mutating.genome2.mutation.plasticity_enabled = true;
    assert_ne!(mutating.stable_hash(), 0x9abc_0cd4_7914_127f);
    assert_ne!(
        enabled.stable_hash(),
        mutating.stable_hash(),
        "two behaviorally different worlds share a config hash"
    );
}

#[test]
fn a_disabled_section_leaves_a_flagged_genome_completely_inert() {
    // The stronger form of C11.8: not only does a world without plastic
    // edges reproduce, a world whose genomes *are* flagged plastic behaves
    // identically to one whose genomes are not, as long as the section is
    // off. That is what "the gate, not the flag, decides" means, and it is
    // the clause `EDGE_FLAG_PLASTIC` already being in every schema-2 genome
    // makes necessary.
    let mut config = SimConfig::phase11_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 1_200;

    let mut flagged_off = config;
    flagged_off.plasticity.enabled = false;
    let mut flagged = {
        // Build the flagged genomes under the enabled config, then restore
        // them into a world whose section is off.
        let seeded = plastic_world(config, RULE_HEBBIAN, 1.0);
        let mut state = seeded.export_state();
        state.config = flagged_off;
        // The learn section goes with the config it belongs to. Dropping it
        // is required, not convenient: presence must match the configuration
        // or `from_state` refuses the save, and that refusal is the same
        // guard that stops a plasticity save from being restored into a world
        // that would never run a learn phase over it.
        state.learn = None;
        World::from_state(state).expect("a flagged genome restores with the section off")
    };
    let mut plain = World::new(flagged_off).expect("world");

    // The two worlds differ in their genomes (one set carries flags and
    // genes) and must agree on everything the flags are supposed not to
    // touch, so behaviour is compared rather than checksums.
    assert!(flagged.learned_census().is_empty());
    for tick in 0..300 {
        flagged.step();
        plain.step();
        assert_eq!(
            flagged.population(),
            plain.population(),
            "the flag changed behaviour at tick {tick} with the section off"
        );
        assert_eq!(flagged.total_energy_milli(), plain.total_energy_milli());
    }
    assert_eq!(flagged.metrics().plasticity_cost_milli, 0);
    flagged.check_invariants().expect("invariants");
}

// --- save, restore, and the learned state that has nowhere else to come from -

/// A world whose founders carry one behaviourally inert plastic edge, run
/// long enough that its learned deltas are nonzero and **not yet saturated**.
///
/// Both halves matter for the save tests. Zero deltas would make a round trip
/// a check on an all-zero array, which is the exact defect this repo has
/// shipped three times. Saturated deltas would make every organism's row
/// identical, and a permutation of identical rows is an identity - so the
/// scramble sweep would report "the world did not notice" and be right.
/// `eta = 0.002` over 1,200 ticks reaches roughly a tenth of the clamp.
fn learning_population(seed: u64, ticks: u64) -> World {
    let mut world = neutral_plastic_world(learning_config(seed), RULE_HEBBIAN, 0.002);
    run(&mut world, ticks);
    let census = world.learned_census();
    let moved = census
        .iter()
        .filter(|sample| sample.sum_abs_learned_q16 > 0)
        .count();
    assert!(
        moved > 40,
        "only {moved} organisms learned anything, so a save test over this \
         world would be a test on zeros"
    );
    assert!(
        census
            .iter()
            .any(|sample| sample.max_abs_learned_q16 < sim_core::LEARN_LIMIT_Q16),
        "every delta is pinned at the clamp, so every row is identical and a \
         permutation of them cannot be detected"
    );
    world
}

#[test]
fn a_plasticity_world_saves_restores_and_continues_identically() {
    // C11.3 at the kernel level: the logical save round trip, with plastic
    // edges carrying nonzero learned deltas. The codec-level version is
    // `sim-persist`'s `config_round_trip.rs`; this one is what fails first if
    // the section is wrong rather than merely mis-encoded.
    let mut world = learning_population(SEED, 1_200);
    let census = world.learned_census();
    let checksum = world.state_checksum();
    let state = world.export_state();

    // The section exists, is sparse, and carries values - asserted on the
    // record itself rather than inferred from the checksum matching, because
    // a checksum can match for a world that stored nothing and rebuilt
    // nothing.
    let learn = state
        .learn
        .as_ref()
        .expect("a plasticity world saves learning");
    assert_eq!(learn.edges.len(), world.population());
    assert!(
        learn
            .edges
            .iter()
            .flatten()
            .any(|edge| edge.learned_q16 != 0),
        "the saved section is all zeros"
    );
    // Sparse: one plastic edge per organism, not one entry per weight.
    assert!(
        learn.edges.iter().all(|row| row.len() <= 2),
        "the section stopped being sparse"
    );

    let mut restored = World::from_state(state).expect("restore");
    assert_eq!(restored.state_checksum(), checksum);
    // The values themselves, not only their hash: a checksum equality would
    // also hold for a restore that zeroed the learned state *and* left it out
    // of the checksum, which is a pair of defects that cancel.
    assert_eq!(restored.learned_census(), census);

    for _ in 0..400 {
        world.step();
        restored.step();
    }
    assert_eq!(
        world.state_checksum(),
        restored.state_checksum(),
        "a restored plasticity world diverged from the one it was saved from"
    );
    restored.check_invariants().expect("invariants");
}

#[test]
fn a_restore_that_zeroes_learned_state_is_visible_as_a_divergence() {
    // C11.3's "not reconstructible" clause, at the kernel level and stated as
    // a **behavioural** difference rather than a checksum one. Learned state
    // is hashed, so erasing it moves the checksum no matter what the tick
    // does with it; the claim that matters is that the tick does something.
    //
    // This is the restore path that shipped for one stage - rows the right
    // width, all zero - and it has to be detectable, or "save, restore,
    // continue is bit-identical" would be a statement about a section nothing
    // reads.
    //
    // The neutral-plastic-edge world used everywhere else in this file is
    // **the wrong instrument for this one test** and would pass it for the
    // wrong reason: that edge feeds a node nothing reads, so erasing its
    // learning is genuinely a no-op on behaviour. Measured, not assumed - the
    // first version of this test asserted divergence on the neutral world and
    // the two position arrays came back identical to the last unit. So this
    // one puts the plasticity on the founder's own two edges, which are the
    // whole path from the energy sensor to the turn channel, at an `eta` low
    // enough that the population survives the comparison.
    let mut world = plastic_world(learning_config(SEED), RULE_HEBBIAN, 0.01);
    run(&mut world, 200);
    let census = world.learned_census();
    assert!(
        census
            .iter()
            .filter(|sample| sample.sum_abs_learned_q16 > 0)
            .count()
            > 40,
        "too little was learned for erasing it to be able to matter"
    );

    let mut state = world.export_state();
    for row in state.learn.as_mut().expect("section").edges.iter_mut() {
        for edge in row.iter_mut() {
            edge.learned_q16 = 0;
            edge.trace_q16 = 0;
        }
    }
    let mut zeroed = World::from_state(state).expect("a zeroed save is still legal");
    assert_ne!(
        zeroed.state_checksum(),
        world.state_checksum(),
        "zeroing every learned delta did not move the checksum"
    );

    for _ in 0..300 {
        world.step();
        zeroed.step();
    }
    assert!(world.population() > 10, "the world died before it diverged");
    assert_ne!(
        world.export_state().x_fp,
        zeroed.export_state().x_fp,
        "a world whose learning was erased followed the same trajectory, so \
         learned state is not load-bearing and C11.3 is unmet"
    );
}

#[test]
fn a_learned_row_that_does_not_match_its_plan_is_refused() {
    // Every way a learn section can disagree with the world it is restored
    // into, each refused rather than applied. The failure this defends
    // against is not a crash: it is a restore that lines the rows up wrongly
    // and continues, so an organism resumes with another edge's lifetime of
    // learning and nothing anywhere says so.
    let world = learning_population(SEED, 400);
    let good = world.export_state();
    assert!(
        World::from_state(good.clone()).is_ok(),
        "the unmodified save must restore, or every refusal below is vacuous"
    );

    let mutate = |edit: fn(&mut SaveState)| -> RestoreError {
        let mut state = good.clone();
        edit(&mut state);
        match World::from_state(state) {
            Ok(_) => panic!("a corrupted learn section was admitted"),
            Err(error) => error,
        }
    };
    // **Which guard fired is asserted, not just that one did**, and that is
    // not pedantry. `check_invariants` runs at the end of `from_state` and
    // its `LearnDesync` and `LearnBounds` clauses catch a wrong row width and
    // an out-of-clamp value on their own - so deleting the checks in
    // `from_state` leaves every one of these restores still refused, and a
    // test that only matched `StateInvalid(_)` would pass. Measured: removing
    // the row-length check and removing the clamp check were both injected
    // and both went undetected until this became a message assertion.
    //
    // Defence in depth is the right outcome; a test that cannot tell the two
    // layers apart is not. The specific messages pin the near guard, which is
    // the one that names the organism and the slot.
    let refused_by = |edit: fn(&mut SaveState), needle: &str| {
        let error = mutate(edit);
        let RestoreError::StateInvalid(message) = &error else {
            panic!("expected StateInvalid, got {error:?}");
        };
        assert!(
            message.contains(needle),
            "refused by the wrong guard: wanted {needle:?}, got {message:?}"
        );
    };

    // An edge id naming an edge the rebuilt plan does not have plastic. This
    // is the check that makes the section self-describing: without it the
    // values would be applied positionally to whatever the plan happened to
    // compile - and because the lengths still agree, nothing downstream would
    // notice at all.
    refused_by(
        |state| state.learn.as_mut().unwrap().edges[0][0].edge_homology_id ^= 0x5a5a,
        "and its rebuilt plan has",
    );
    // A row that is the wrong length for its plan.
    refused_by(
        |state| {
            state.learn.as_mut().unwrap().edges[0].pop();
        },
        "learned edges and its rebuilt plan has",
    );
    // A learned delta outside the clamp `accumulate_clamped` promises. It
    // would otherwise go straight into `effective_weight`.
    refused_by(
        |state| {
            state.learn.as_mut().unwrap().edges[0][0].learned_q16 = sim_core::LEARN_LIMIT_Q16 + 1
        },
        "outside the clamp",
    );
    // The trace is bounded on the same terms and is the easy one to leave out
    // of a bounds check.
    refused_by(
        |state| {
            state.learn.as_mut().unwrap().edges[0][0].trace_q16 = -sim_core::LEARN_LIMIT_Q16 - 1
        },
        "outside the clamp",
    );
    // Length parity with the population, through the same helper every other
    // section uses.
    assert_eq!(
        mutate(|state| {
            state.learn.as_mut().unwrap().faults.pop();
        }),
        RestoreError::LengthMismatch {
            field: "learn.faults"
        }
    );
    assert_eq!(
        mutate(|state| {
            state.learn.as_mut().unwrap().edges.pop();
        }),
        RestoreError::LengthMismatch {
            field: "learn.edges"
        }
    );
    // Presence must match the configuration in both directions.
    assert!(matches!(
        mutate(|state| state.learn = None),
        RestoreError::StateInvalid(_)
    ));
    let mut disabled = good.clone();
    disabled.config.plasticity.enabled = false;
    disabled.config.genome2.mutation.plasticity_enabled = false;
    assert!(matches!(
        World::from_state(disabled),
        Err(RestoreError::StateInvalid(_))
    ));
}

#[test]
fn scrambling_the_learned_state_changes_a_plasticity_world() {
    // Phase 11's half of C11.8's storage-permutation clause. C9.7's sweep in
    // `phase9_determinism.rs` covers every other per-organism array but runs
    // on a Phase 9 world, which has no plasticity section - so these two
    // arrays would be carried in the save and never swept anywhere, which is
    // the precise defect that sweep exists to make impossible.
    //
    // Rotate-by-one, for the reason that file gives: it has exactly one fixed
    // point, a constant array, so "the world did not notice" means "there was
    // nothing to notice" and the test can say which.
    let world = learning_population(SEED, 1_200);
    let original = world.export_state();
    assert!(original.ids.len() > 20, "too few organisms to scramble");

    let advance = |state: SaveState| -> Option<(u64, u64)> {
        let mut world = World::from_state(state).ok()?;
        let at_restore = world.state_checksum();
        for _ in 0..200 {
            world.step();
        }
        Some((at_restore, world.state_checksum()))
    };
    let baseline = advance(original.clone()).expect("the baseline restores");

    let mut rotated = original.clone();
    {
        let learn = rotated.learn.as_mut().expect("section");
        assert!(
            learn.edges.windows(2).any(|pair| pair[0] != pair[1]),
            "every organism's learned row is identical, so rotating them is \
             the identity and this test would prove nothing"
        );
        learn.edges.rotate_left(1);
    }
    assert_ne!(
        advance(rotated),
        Some(baseline),
        "learn.edges was rotated and the world did not notice, so learned \
         state is carried in the save and never read"
    );

    // `learn.faults` is constant-zero and the test says so rather than
    // crediting itself with a pass: a non-finite activation is unreachable
    // through validated genes, so nothing in a legal world can make an
    // organism's fault count differ from its neighbour's. The array is
    // rotated anyway, so that a world which *can* fault fails here instead of
    // slipping past.
    let mut rotated = original.clone();
    {
        let learn = rotated.learn.as_mut().expect("section");
        let uniform = learn.faults.windows(2).all(|pair| pair[0] == pair[1]);
        learn.faults.rotate_left(1);
        if !uniform {
            assert_ne!(
                advance(rotated.clone()),
                Some(baseline),
                "learn.faults varies across organisms and rotating it changed \
                 nothing, so it is carried and never read"
            );
        }
        assert!(uniform, "faults became reachable: keep the branch above");
    }
}

#[test]
fn an_eligibility_trace_survives_the_round_trip_as_well_as_the_learned_delta() {
    // **The trap this closes has been hit three times in this repo**: a round
    // trip that does not perturb the field it is meant to defend. Every other
    // save test in this file uses rule 1, which never writes `trace_q16`, so
    // all of them would pass unchanged against a codec and a restore path
    // that dropped the trace entirely - the values compared would be zero on
    // both sides.
    //
    // Rule 4 with `modulator_node = 0` is exactly the world that separates
    // them. The eligibility trace accumulates every tick regardless of the
    // modulator; only the discharge is gated, and an absent modulator is
    // handed an activation of 0.0. So the trace fills and the learned delta
    // stays at zero, which is the opposite pattern to every other test here.
    let mut world = neutral_plastic_world(
        learning_config(SEED),
        sim_core::RULE_ELIGIBILITY_TRACE,
        0.05,
    );
    run(&mut world, 800);

    let census = world.learned_census();
    let traced = census
        .iter()
        .filter(|sample| sample.sum_abs_trace_q16 > 0)
        .count();
    assert!(
        traced > 40,
        "only {traced} organisms have a nonzero trace, so this world does not \
         exercise the field it exists to defend"
    );
    assert!(
        census.iter().all(|sample| sample.sum_abs_learned_q16 == 0),
        "the learned delta moved, so a save that carried only `learned_q16` \
         could still pass this test"
    );

    let checksum = world.state_checksum();
    let state = world.export_state();
    let learn = state.learn.as_ref().expect("section");
    assert!(
        learn.edges.iter().flatten().any(|edge| edge.trace_q16 != 0),
        "the saved section carries no trace"
    );
    assert!(
        learn
            .edges
            .iter()
            .flatten()
            .all(|edge| edge.learned_q16 == 0),
        "the premise is that only the trace is nonzero"
    );

    let mut restored = World::from_state(state).expect("restore");
    assert_eq!(restored.state_checksum(), checksum);
    assert_eq!(restored.learned_census(), census);
    for _ in 0..300 {
        world.step();
        restored.step();
    }
    assert_eq!(world.state_checksum(), restored.state_checksum());
}

#[test]
fn a_cheap_plastic_edge_costs_something_instead_of_nothing() {
    // The defect this closes: the per-edge debit was
    // `edges * milli_per_s * dt_ms / 1000` truncated to whole milli every
    // tick, with the remainder discarded. At the **shipped default** of 2
    // milli/s and `dt_ms = 100` that is `200 / 1000 = 0`, so a plastic edge
    // was free - and 10 milli/s was the cheapest rate that charged anything
    // at all, a tenth of basal per edge. The only expressible prices were
    // "free" and "ruinous", so "many cheap plastic edges" had no price and
    // C11.2 could not ask its own question.
    //
    // The remainder is now carried, so the charge is exact.
    let mut config = learning_config(SEED);
    config.plasticity.plastic_edge_cost_milli_per_s = SimConfig::phase11_default(SEED)
        .plasticity
        .plastic_edge_cost_milli_per_s;
    let rate = config.plasticity.plastic_edge_cost_milli_per_s;
    let dt = i64::from(config.dt_ms);
    assert!(
        rate * dt / 1_000 == 0,
        "this test is about a rate that truncates to zero per tick; at {rate} \
         milli/s and dt {dt} ms it does not, so it proves nothing"
    );

    // One plastic edge that nothing reads, so the only difference from the
    // control is the debit itself.
    let mut world = neutral_plastic_world(config, RULE_HEBBIAN, 0.0);
    let population = world.population() as i128;
    assert!(population > 0);

    // Exactly one tick: still below a whole milli, so nothing is charged yet
    // and the remainder is carrying it.
    world.step();
    assert_eq!(
        world.metrics().plasticity_cost_milli,
        0,
        "a single tick at a sub-milli rate should charge nothing yet"
    );

    // Five ticks at 200 thousandths each is exactly 1000, so exactly one
    // milli per organism - the first tick at which the old model and the new
    // one differ, and the whole point of the change.
    for _ in 0..4 {
        world.step();
    }
    let after_five = i128::from(world.metrics().plasticity_cost_milli);
    assert_eq!(
        after_five, population,
        "five ticks at {rate} milli/s should charge exactly one milli per \
         organism; the old model charged zero forever"
    );

    // ...and it keeps accruing at the exact rate rather than drifting. 100
    // ticks is 20 milli per organism, with no remainder left over.
    for _ in 0..95 {
        world.step();
    }
    let expected = i128::from(100 * rate * dt / 1_000) * population;
    assert_eq!(
        i128::from(world.metrics().plasticity_cost_milli),
        expected,
        "the accumulated charge drifted from the exact cost"
    );
    world.check_invariants().expect("the ledger balances");
}

#[test]
fn the_cost_remainder_survives_a_save_and_is_refused_when_it_is_not_a_fraction() {
    // The remainder is lifetime-accumulating state, so dropping it on save
    // would restart every organism's bill at zero - a slow, invisible refund
    // that no checksum comparison at rest would catch, because a restored
    // world would agree with itself.
    let mut config = learning_config(SEED);
    config.plasticity.plastic_edge_cost_milli_per_s = 2;
    let mut world = neutral_plastic_world(config, RULE_HEBBIAN, 0.0);
    // Three ticks leaves 600 thousandths owed: mid-fraction, so a codec that
    // dropped the field would restore a *different* number rather than the
    // same zero it started at.
    for _ in 0..3 {
        world.step();
    }
    let state = world.export_state();
    let learn = state.learn.as_ref().expect("a plasticity world");
    assert!(
        learn.cost_remainder.iter().all(|value| *value == 600),
        "the fixture must carry a mid-fraction remainder or this is 0 == 0; \
         got {:?}",
        &learn.cost_remainder[..learn.cost_remainder.len().min(4)]
    );

    let restored = World::from_state(state.clone()).expect("restore");
    assert_eq!(restored.state_checksum(), world.state_checksum());

    // A remainder of a whole milli or more is a milli that was never
    // charged. Refused rather than normalized, because normalizing forgives
    // it silently.
    let mut tampered = state;
    tampered.learn.as_mut().expect("learn").cost_remainder[0] = 1_000;
    let error = World::from_state(tampered).expect_err("a whole milli is not a fraction");
    assert!(
        matches!(&error, RestoreError::StateInvalid(message)
            if message.contains("cost remainder")),
        "expected a typed refusal naming the remainder, got {error:?}"
    );
}

// --- the moat: D-107's price basis, and the trap it was nearly built into ----

/// The moat charges less than the flat price, **and it still charges less
/// when the chain is on.**
///
/// The second clause is the whole test. The obvious implementation prices on
/// `StepKind::Applied`, and `Applied` means "the rule form ran and the state
/// was rewritten, *possibly to the same value*". The only non-`Applied` kinds
/// are the rule-0 early return and an unreachable refusal - so under
/// `live_rule_zero`, which removes rule 0, every plastic edge is `Applied`,
/// `applied == plastic_edges.len()`, and a moat priced on it charges exactly
/// what the unpriced engine charges. The moat would be a **no-op in both arms
/// where the chain is on**, collapsing the 2x2's fourth arm into its second,
/// and every test that only checked the chain-off case would still pass.
///
/// D-107 named this failure before either half was built:
/// "applied-step pricing must charge on the learned state actually moving,
/// not on a step being taken, or it would re-charge exactly what it set out
/// to stop charging for."
///
/// So the price basis is state movement, and this asserts it where it would
/// otherwise silently not hold.
#[test]
fn the_moat_still_charges_less_than_the_flat_price_when_the_chain_is_on() {
    fn cost_after(moat: bool, chain: bool, rule_id: u8, eta: f32) -> i128 {
        let mut config = learning_config(0x5eed_cafe_f00d_beef);
        config.plasticity.price_moved_edges_only = moat;
        config.plasticity.live_rule_zero = chain;
        let mut world = plastic_world(config, rule_id, eta);
        run(&mut world, 400);
        world.metrics().plasticity_cost_milli as i128
    }

    // Rule 0 with the chain off: every edge is a no-op, and the flat price
    // charges for all of them anyway. This is the 95.43 percent the
    // confirmatory campaign paid for nothing.
    let flat_dead = cost_after(false, false, sim_core::RULE_STATIC, 0.0);
    let moat_dead = cost_after(true, false, sim_core::RULE_STATIC, 0.0);
    assert!(
        flat_dead > 0,
        "the flat price charged nothing, so there is no moat to measure against"
    );
    assert_eq!(
        moat_dead, 0,
        "an edge whose rule writes nothing must cost nothing under the moat; \
         charged {moat_dead} against a flat {flat_dead}"
    );

    // **The clause that catches applied-step pricing.** With the chain on,
    // rule 0 is remapped to a live rule, so every edge returns `Applied` -
    // but `eta` is still zero, so no learned state moves and the moat must
    // still charge nothing. Priced on `Applied`, this would equal the flat
    // price.
    let flat_chained = cost_after(false, true, sim_core::RULE_STATIC, 0.0);
    let moat_chained = cost_after(true, true, sim_core::RULE_STATIC, 0.0);
    assert!(
        flat_chained > 0,
        "the flat price charged nothing with the chain on: {flat_chained}"
    );
    assert_eq!(
        moat_chained, 0,
        "under the chain every edge is StepKind::Applied, so a moat priced on \
         Applied charges the flat price and the 2x2's B arm collapses into C. \
         Charged {moat_chained} against a flat {flat_chained}"
    );

    // ...and the moat is not simply "charge nothing": an edge that does move
    // its learned state still pays.
    let moat_live = cost_after(true, false, sim_core::RULE_HEBBIAN, 0.5);
    assert!(
        moat_live > 0,
        "the moat charged nothing for edges that actually learned, so it is a \
         cost removal rather than a repricing"
    );

    // **The eligibility trace is half of `LearnedState`, and testing only the
    // weight leaves it uncovered.** A mutation run found that comparing only
    // `learned_q16` survives the whole workspace, because every rule this test
    // used moved the weight. Rule 4 is the case that separates them: `step`
    // writes `trace_q16` every tick from `decay(trace) + eta * raw`, then
    // computes the weight delta as `trace * modulator`. An edge with no
    // modulator gene is handed `0.0`, so its weight never moves while its
    // trace moves constantly - and under weight-only pricing that edge is
    // free for life while carrying live machinery, which is the exact thing
    // the moat exists to stop being free.
    let moat_trace = cost_after(true, false, sim_core::RULE_ELIGIBILITY_TRACE, 0.5);
    assert!(
        moat_trace > 0,
        "an edge whose eligibility trace moves every tick was charged nothing; \
         the moat's basis is `LearnedState` in full, not `learned_q16` alone"
    );

    // **A magnitude bound, not another shape assertion.** The three
    // assertions above are all `== 0` or `> 0`, and a mutation that hoisted
    // the per-organism `moved_edges` counter out of its loop - so organism k
    // pays for every edge that moved in organisms 0..k - survived all of
    // them and the whole workspace. Nothing pinned how *much* the moat
    // charges, only that it was zero or nonzero.
    //
    // An organism can never be charged for more edges than it carries, so the
    // moat can never exceed the flat price for the same world. Under the
    // hoisted counter, later organisms are charged multiples of their edge
    // count and the total runs away past it.
    let flat_live = cost_after(false, false, sim_core::RULE_HEBBIAN, 0.5);
    assert!(
        moat_live <= flat_live,
        "the moat charged {moat_live} against a flat {flat_live}. Charging more \
         than the price for every edge means an organism was billed for edges \
         it does not carry - check that the moved-edge counter is per organism"
    );
}

/// The moat does not move a world that has it off.
///
/// The flag is gated so that every existing fixture and every arm that does
/// not select it are untouched, and this asserts that rather than arguing it.
#[test]
fn a_world_with_the_moat_off_is_bit_identical_to_one_from_before_it_existed() {
    let base = learning_config(0x5eed_cafe_f00d_beef);
    let mut explicit_off = base;
    explicit_off.plasticity.price_moved_edges_only = false;
    assert_eq!(base.stable_hash(), explicit_off.stable_hash());

    let mut left = plastic_world(base, sim_core::RULE_HEBBIAN, 0.25);
    let mut right = plastic_world(explicit_off, sim_core::RULE_HEBBIAN, 0.25);
    run(&mut left, 300);
    run(&mut right, 300);
    assert_eq!(left.state_checksum(), right.state_checksum());

    // ...and turning it on does move the hash, so it is a new replay lineage.
    let mut on = base;
    on.plasticity.price_moved_edges_only = true;
    assert_ne!(base.stable_hash(), on.stable_hash());
}
