//! Phase 9 world integration: a schema-2 world runs, evolves structure, and
//! saves and restores.
//!
//! What this establishes is that the seam is narrow and correct. Schema 2
//! replaces the genome and the controller; everything else runs the same
//! code, which is what makes a schema-1 world a usable baseline rather than
//! a different simulation.
//!
//! C9.1, C9.2, C9.5, and C9.8 are multi-seed campaign claims and are not
//! here. What is here is the evidence that the campaigns would be measuring
//! something real.

use sim_core::{SimConfig, World};

fn schema2_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 2_000;
    config.genome2.enabled = true;
    config
}

fn schema1_config(seed: u64) -> SimConfig {
    let mut config = schema2_config(seed);
    config.genome2.enabled = false;
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

#[test]
fn a_schema_2_world_runs_and_its_organisms_act() {
    // The baseline claim: enabling schema 2 produces a world that ticks,
    // feeds, reproduces, and does not die of its own machinery.
    let world = run(schema2_config(7), 3_000);
    let metrics = world.metrics();
    assert!(metrics.genome2_enabled);
    assert!(world.population() > 0, "the schema-2 world went extinct");
    assert!(
        metrics.births_total > 0,
        "nothing ever reproduced, so meiosis was never exercised in a world"
    );
    // Organisms must actually be *doing* something: a world where every
    // controller output is zero would still tick and still reproduce on
    // energy alone, and would prove nothing about the controller.
    assert!(
        world.ledger().consumed_biomass_milli > 0,
        "nothing ever ate, so no action channel ever fired"
    );
}

#[test]
fn the_schema_1_world_is_untouched_by_schema_2_existing() {
    // The rollback story, and the load-bearing one: the section is
    // config-gated, so a schema-1 world takes the same code paths it always
    // did. Checked behaviorally rather than only on the fixture, because a
    // config-hash difference alone would not prove it.
    let disabled = schema1_config(11);
    assert!(!disabled.genome2.enabled);
    let mut fiddled = disabled;
    fiddled.genome2.caps.max_nodes = 9;
    fiddled.genome2.mutation.duplication_q16 = 65_535;
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
fn a_schema_2_world_is_deterministic_and_restores_identically() {
    let config = schema2_config(23);
    let first = run(config, 1_200);
    let second = run(config, 1_200);
    assert_eq!(first.state_checksum(), second.state_checksum());

    // Save and restore through the kernel's own logical path. Activations
    // are logical state, so a restored recurrent organism must resume with
    // the memory it had rather than a fresh one.
    let restored_state = first.export_state();
    let mut restored = World::from_state(restored_state).expect("restore");
    assert_eq!(restored.state_checksum(), first.state_checksum());
    assert!(restored.genome2_enabled());

    let mut original = run(config, 1_200);
    for _ in 0..400 {
        original.step();
        restored.step();
    }
    assert_eq!(
        original.state_checksum(),
        restored.state_checksum(),
        "a restored schema-2 world diverged"
    );
}

#[test]
fn structure_evolves_and_stays_inside_its_caps() {
    // The shape C9.1 will measure at campaign scale. Founders are minimal by
    // construction - three nodes and two edges - so any change is evolved
    // rather than seeded, and that is the whole reason founders are minimal.
    // **Structural evolution is birth-limited, and that is the finding this
    // test's own first configuration produced.** At the default carrying
    // capacity this world managed 25 births in 6,000 ticks, so eleven
    // applied mutations spread across fifty-five organisms moved the mean
    // node count not at all and left one distinct structure. The mechanism
    // was working the whole time.
    //
    // A C9.1 campaign run in a world like that would return a null caused by
    // too few births rather than by anything about duplication - the same
    // failure mode Phase 8 existed to prevent for the culture stack,
    // recurring one phase later. Raising the carrying capacity gives 191
    // births and the numbers below.
    let mut config = schema2_config(31);
    config.cell_capacity_milli = 120_000;
    config.max_entities = 4_000;
    config.genome2.mutation.duplication_q16 = 13_107;
    config.genome2.mutation.deletion_q16 = 3_277;
    config.genome2.mutation.point_q16 = 13_107;

    let founder_world = World::new(config).expect("world");
    let founding = founder_world.metrics();
    assert_eq!(founding.mean_nodes_milli, 3_000, "founders are minimal");
    assert_eq!(founding.mean_edges_milli, 2_000);
    assert_eq!(founding.distinct_structures, 1);

    let world = run(config, 6_000);
    let metrics = world.metrics();
    assert!(world.population() > 0, "the world went extinct");
    assert!(
        metrics.structural_mutations_applied > 0,
        "no structural mutation was ever applied"
    );
    assert!(
        metrics.mean_nodes_milli != 3_000 || metrics.mean_edges_milli != 2_000,
        "structure never changed from the founding topology"
    );
    assert!(
        metrics.distinct_structures > 1,
        "the population is structurally uniform, so nothing diversified"
    );
    // Births are the currency structural evolution is bought with, so a
    // world that barely reproduces cannot demonstrate it whatever the
    // mutation rate.
    assert!(
        metrics.births_total > 50,
        "only {} births, so this world is birth-limited and the assertions above \
         would be measuring the birth rate rather than duplication",
        metrics.births_total
    );
    // ...and it must not have run away. The caps are provisional, but they
    // are enforced, and a genome beyond them would have failed the invariant
    // check inside `run` already.
    assert!(
        metrics.mean_nodes_milli < u64::from(config.genome2.caps.max_nodes) * 1_000,
        "mean node count reached the cap"
    );
}

#[test]
fn every_rejection_is_counted_rather_than_silent() {
    // An experiment quietly running against a cap has to be visible in its
    // own report, or a null result about structural evolution might only
    // mean the mutations never happened.
    let mut config = schema2_config(37);
    config.genome2.mutation.duplication_q16 = 65_535;
    config.genome2.caps.max_loci_per_chromosome = 21; // the founder's own size
    let world = run(config, 2_000);
    let metrics = world.metrics();
    assert!(
        metrics.structural_mutations_rejected > 0,
        "the cap was never reported as a reason"
    );
    assert_eq!(
        metrics.mean_nodes_milli, 3_000,
        "the cap did not actually bind"
    );
}

#[test]
fn the_two_schemas_reach_comparable_ecologies() {
    // C9.2's shape: structural freedom must not destabilize the ecology.
    // Not a criterion here - that needs 30 seeds - but if a single schema-2
    // world collapsed while its schema-1 twin thrived, the campaign would be
    // measuring a broken integration rather than an effect.
    let schema1 = run(schema1_config(41), 4_000);
    let schema2 = run(schema2_config(41), 4_000);
    assert!(schema1.population() > 0 && schema2.population() > 0);
    let ratio = schema2.population() as f64 / schema1.population().max(1) as f64;
    assert!(
        (0.05..=20.0).contains(&ratio),
        "populations differ by {ratio:.2}x, which is a broken integration rather than an effect"
    );
}

#[test]
fn a_schema_2_organism_reports_its_expressed_traits() {
    // The observer path must work for both schemas: `organism_detail` reads
    // a flat genome in schema 1 and expressed traits in schema 2, and a
    // caller cannot tell which.
    let world = run(schema2_config(43), 500);
    let id = *world.organism_ids_view().first().expect("a survivor");
    let detail = world.organism_detail(id).expect("detail");
    let phase2 = detail.phase2.expect("phase 2 detail");
    assert!(
        phase2
            .trait_genes
            .iter()
            .all(|value| value.is_finite() && (0.0..=1.0).contains(value)),
        "expressed traits are out of range: {:?}",
        phase2.trait_genes
    );
}

#[test]
fn a_nonviable_recombinant_is_refused_at_pairing_rather_than_admitted() {
    // Crossover cuts a haplotype at an arbitrary point, so a gamete can
    // carry an edge whose node stayed on the other side of the cut. That is
    // a real genetic outcome, and the world has to refuse the child without
    // damaging itself.
    //
    // The regression this pins: the birth path used to push `ids`,
    // positions, energy and age *before* asking whether the schema-2
    // organism could be admitted, then `continue` on refusal - under a
    // comment claiming the arrays stayed in lockstep. They did not. The
    // organism arrays grew by one, the phase-2 arrays did not, and the next
    // sense phase indexed `phenotypes` out of bounds and panicked. It took a
    // campaign-scale run to surface, so the assertion below deliberately
    // uses a configuration where the path is reached within seconds.
    let mut config = schema2_config(7);
    config.initial_organisms = 200;
    config.max_entities = 40_000;
    config.cell_capacity_milli = 120_000;
    config.physiology.enabled = true;
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.insertion_q16 = 6_554;

    let world = run(config, 10_000);
    let counters = world.phase2_counters();
    assert!(
        counters.pair_rejected_nonviable_total > 0,
        "no non-viable recombinant occurred, so this test proved nothing; \
         it must exercise the refusal path to be a regression test for it"
    );
    assert!(world.population() > 0, "the world went extinct");
    // The refusal must cost a mating opportunity and nothing else: the
    // energy ledger and population accounting are checked by
    // `check_invariants` inside `run`, which is the actual assertion here.
    assert!(world.metrics().births_total > 0);
}

#[test]
fn the_invalid_counter_stays_zero_because_it_is_the_bug_signal() {
    // `RejectReason::Invalid` is documented as "a bug report, not a runtime
    // condition". That is only true if the expected conditions have their
    // own reasons, and twice they did not.
    //
    // Transposition on a single-chromosome genome - which every founder is -
    // was counted as `Invalid`, so a clean run reported hundreds of them.
    // Then the operators were handed recombinants nobody had validated, so
    // each operator reported *its input* as invalid. Both are now typed:
    // `Inapplicable` for a precondition that does not hold, and refusal at
    // pairing for a recombinant that is not viable.
    let mut config = schema2_config(13);
    config.cells_x = 64;
    config.cells_y = 64;
    config.cell_capacity_milli = 120_000;
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.insertion_q16 = 6_554;
    config.genome2.mutation.transposition_q16 = 6_554;

    let world = run(config, 10_000);
    let counters = world.mutation_counters().expect("schema 2 is enabled");
    assert_eq!(
        counters.rejected_invalid, 0,
        "an operator produced a genome that failed validation for a reason \
         nothing anticipated, which is a bug rather than a run outcome"
    );
    // ...and the run must actually have attempted the operators, or a zero
    // above would only mean nothing happened.
    assert!(
        counters.total_applied() > 0,
        "no mutation was applied, so the zero above is vacuous"
    );
    assert!(
        counters.rejected_inapplicable > 0,
        "transposition never reported a precondition failure, so the \
         reason that replaced the miscount is not being exercised"
    );
    assert_eq!(
        counters.transposition_applied, 0,
        "a single-chromosome founder cannot transpose; if this ever becomes \
         non-zero the founder layout changed and C9.5's operator set with it"
    );
}

#[test]
fn every_structural_rejection_is_evented_as_well_as_counted() {
    // C9.6's remaining half. The criterion asks that a cap "reject
    // deterministically, count, *and* event"; the counters and the checksum
    // had it and the event log did not, so a cap that bound was invisible to
    // every offline analysis that reads the log rather than the snapshot.
    //
    // The assertion that makes this worth having is the **reconciliation**:
    // the events emitted must equal the counters, class by class. A test
    // that only checked "at least one event appeared" would pass while seven
    // of the eight reasons were silently dropped, and a test that compared
    // totals would pass while two classes were swapped.
    use sim_core::{EventKind, RejectReason};

    let mut config = schema2_config(13);
    config.cells_x = 64;
    config.cells_y = 64;
    config.cell_capacity_milli = 120_000;
    // Every operator at a rate high enough that rejections actually happen.
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.insertion_q16 = 6_554;
    config.genome2.mutation.transposition_q16 = 6_554;

    let mut world = World::new(config).expect("world");
    // Indexed by `RejectReason::code() - 1`, which is the permanent wire
    // code rather than a declaration order.
    let mut evented = [0_u64; 8];
    for _ in 0..10_000 {
        world.step();
        for event in world.events() {
            if let EventKind::StructuralMutationRejected {
                operator, reason, ..
            } = event.kind
            {
                assert!(
                    (sim_core::OP_POINT..=sim_core::OP_TRANSPOSITION).contains(&operator),
                    "operator code {operator} is not one of the five"
                );
                evented[usize::from(reason.code() - 1)] += 1;
            }
        }
    }

    let counters = world.mutation_counters().expect("schema 2 is enabled");
    assert_eq!(
        world.counters().dropped_events_total,
        0,
        "events were dropped, so the reconciliation below would compare a \
         truncated stream against a complete counter and could not fail"
    );

    let expected = [
        (
            RejectReason::HomologyCollision,
            counters.rejected_homology_collision,
        ),
        (RejectReason::Orphaned, counters.rejected_orphaned),
        (RejectReason::MinNodes, counters.rejected_min_nodes),
        (RejectReason::NoBindings, counters.rejected_no_bindings),
        (RejectReason::Cap, counters.rejected_cap),
        (RejectReason::Inapplicable, counters.rejected_inapplicable),
        (RejectReason::Cycle, counters.rejected_cycle),
        (RejectReason::Invalid, counters.rejected_invalid),
    ];
    for (reason, counted) in expected {
        assert_eq!(
            evented[usize::from(reason.code() - 1)],
            counted,
            "{reason:?}: {counted} counted but \
             {} evented - the log and the checksum disagree",
            evented[usize::from(reason.code() - 1)]
        );
    }

    // ...and the run must actually have produced rejections, or every
    // equality above is 0 == 0 and the test defends nothing. This is the
    // trap that made `every_rejection_is_counted_rather_than_silent` drive a
    // cap until it bound rather than trusting a quiet run.
    assert!(
        counters.total_rejected() > 0,
        "no structural rejection occurred, so this test is vacuous"
    );
    assert!(
        evented.iter().filter(|count| **count > 0).count() >= 2,
        "only one rejection class was exercised, so a per-class mix-up \
         would still pass; counts were {evented:?}"
    );
}
