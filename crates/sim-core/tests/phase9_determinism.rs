//! Phase 9 acceptance criterion C9.7: the Phase 9 fixture, storage-permutation
//! equality, and compaction.
//!
//! The criterion has four clauses - clean-process replay of the Phase 9
//! fixture, storage-permutation equality, edge-summation order independence
//! from storage layout proven by a compaction test, and schema-1 configured
//! worlds still reproducing `0xff9dfcff5dffbf42`. The last one lives in
//! `phase9_genome2.rs`, which is the cheapest place to catch a schema-2 change
//! that leaks into a schema-1 world; the other three are here.
//!
//! Two things about this file's shape are deliberate and worth stating,
//! because the obvious versions of these tests are vacuous:
//!
//! - **`World::from_state` refuses a population that is not in ascending id
//!   order** (`RestoreError::EntityOrder`), so determinism-extensions Rule 4's
//!   literal recipe - permute the storage order of a saved population and
//!   restore it - cannot be executed as written. A joint permutation of every
//!   per-organism array followed by a sort back into id order *is* executable,
//!   and it is exactly an identity on the record, which is why the evidence
//!   here is the negative sweep in
//!   `scrambling_any_one_per_organism_array_changes_the_world` rather than the
//!   round trip itself.
//! - **Storage order does change for real under compaction.** A death shifts
//!   every later survivor down one index, so the same organism occupies a
//!   different slot in two runs that differ only in who died. That is this
//!   engine's storage permutation, and it is why C9.7 pairs the permutation
//!   clause with a compaction test.

use sim_core::{
    Genome2, GenomeCaps, InheritanceMode, RestoreError, SaveState, SimConfig, World,
    compile_network,
};

const FIXTURE_SEED: u64 = 0x5eed_cafe_f00d_beef;
const FIXTURE_TICKS: u64 = 8_000;

/// The Phase 9 fixture's configuration, written out literally.
///
/// **This must stay identical to `apply_pinned_genome2_policy` in
/// `crates/sim-cli/src/main.rs`.** It is duplicated rather than shared
/// because `sim-core` cannot depend on the CLI, and the duplication is safe
/// in the only way that matters: both sides pin the same two constants, so a
/// drift between them fails here and in `crates/sim-cli/tests/cli.rs` rather
/// than producing two silently different fixtures.
///
/// Pinned rather than inherited from `Genome2Config::genome2_default()` for
/// the reason `experiments/phase9-c91-confirmatory.campaign` gives for its own
/// caps block (D-078): `SimConfig::stable_hash` folds the whole genome2
/// section in when it is enabled - four policy strings, both registry
/// versions, all seven `GenomeCaps` fields, the meiosis mode and crossover
/// bound, and all eight `MutationConfig` fields - so a fixture built from
/// defaults breaks the moment any one of them is revised, and the break looks
/// like a determinism failure rather than what it is. The caps have already
/// been restated once, by C9.8's measurement.
fn phase9_fixture_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(FIXTURE_SEED);
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
    config
}

fn advance(config: SimConfig, ticks: u64) -> World {
    let mut world = World::new(config).expect("world");
    for _ in 0..ticks {
        world.step();
    }
    world.check_invariants().expect("invariants");
    world
}

#[test]
fn the_phase9_fixture_reproduces_its_pinned_constants() {
    // C9.7 clause 1, in process. `scripts/verify-phase9-determinism.sh` is the
    // clean-process half; this is the half that fails fast in `cargo test`.
    let config = phase9_fixture_config();
    assert_eq!(
        config.stable_hash(),
        0x9abc_0cd4_7914_127f,
        "the Phase 9 fixture's config hash moved"
    );

    let world = advance(config, FIXTURE_TICKS);
    let metrics = world.metrics();
    assert_eq!(
        world.state_checksum(),
        0x5f0c_4e95_e4f5_170f,
        "the Phase 9 fixture's state checksum moved"
    );
    assert_eq!(world.terrain().terrain_checksum, 0x6004_9f78_e188_1044);

    // **Non-vacuity, and it is not decoration.** `maturity_age_ticks` is 600
    // and founders spawn at age 0, so at the Phase 1/2 horizon of 500 ticks
    // this world has had *zero* births - a 500-tick schema-2 fixture would
    // pin meiosis, structural mutation, and the schema-2 birth path by
    // pinning none of them, and would look exactly as authoritative. The
    // horizon is 8,000 ticks because that is where all three are exercised.
    assert!(
        metrics.births_total > 0 && metrics.paired_births_total > 0,
        "nothing reproduced, so the fixture pins nothing about meiosis"
    );
    assert!(
        metrics.deaths_starvation_total + metrics.deaths_old_age_total > 0,
        "nothing died, so the fixture pins nothing about compaction"
    );
    let counters = world.mutation_counters().expect("schema 2 is enabled");
    assert!(
        counters.duplication_applied > 0,
        "no structural operator ever applied. `total_applied` counts point \
         mutation too, and a point mutation changes no structure, so this is \
         checked on duplication specifically"
    );
    assert!(
        counters.total_rejected() > 0,
        "no structural rejection occurred, so the fixture pins nothing about \
         the refusal path"
    );
    assert!(
        metrics.mean_nodes_milli != 3_000,
        "structure never left the founding topology of three nodes"
    );
    assert!(
        metrics.distinct_structures > 1,
        "the population is structurally uniform"
    );

    // ...and the same config replayed in the same process must agree, which
    // is the weakest of the three replay claims and the one that fails first.
    let again = advance(phase9_fixture_config(), FIXTURE_TICKS);
    assert_eq!(world.state_checksum(), again.state_checksum());
}

// --- storage permutation ----------------------------------------------------

/// A schema-2 world with every optional per-organism section switched on, so
/// the array sweep below covers all of them rather than the schema-2 four.
fn permutation_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 120;
    config.max_entities = 4_000;
    config.cell_capacity_milli = 120_000;
    config.genome2.enabled = true;
    config.contest.enabled = true;
    config.physiology.enabled = true;
    config
}

/// Rotate exactly one per-organism array left by one, and report the full
/// inventory of per-organism arrays as `(name, is_uniform)`.
///
/// **Destructured with no `..` (D-077).** A per-organism array added to
/// `SaveState` fails to compile here until it is either put in the sweep or
/// named as deliberately not per-organism. Field access would let a new array
/// join the save without joining the test, and an array that is carried but
/// never read is exactly the desync class `InvariantViolation::Schema2Desync`
/// exists for.
///
/// Rotate-by-one rather than reverse, because rotate-by-one has exactly one
/// fixed point: a constant array. "The world did not notice" therefore means
/// "there was nothing to notice", and the sweep can assert that rather than
/// letting a palindrome pass for a proof.
fn rotate_one_array(state: &mut SaveState, target: Option<&str>) -> Vec<(&'static str, bool)> {
    let mut inventory: Vec<(&'static str, bool)> = Vec::new();
    macro_rules! per_organism {
        ($name:literal, $values:expr) => {{
            let values = $values;
            // An empty array is trivially constant, and `rotate_left(1)`
            // panics on one.
            inventory.push(($name, values.windows(2).all(|pair| pair[0] == pair[1])));
            if target == Some($name) && !values.is_empty() {
                values.rotate_left(1);
            }
        }};
    }

    let SaveState {
        // Not per organism: world-scope scalars, the per-cell food field, and
        // the two aggregate accounting records.
        config: _,
        tick: _,
        paused: _,
        extinct: _,
        next_entity_id: _,
        terrain_checksum: _,
        composed_terrain_checksum: _,
        biomass_milli: _,
        ledger: _,
        counters: _,
        // Per cell, not per organism.
        climate: _,
        // No per-organism array: development is a pure function of the genome,
        // so this section carries counters only.
        morphology: _,
        // Per cell and per layer, not per organism.
        worldmod: _,

        ids,
        x_fp,
        y_fp,
        energy_milli,
        age_ticks,
        cooldown_ticks,
        phase2,
        contest,
        physiology,
        schema2,
        learn,
        action_census,
    } = state;

    per_organism!("ids", ids);
    per_organism!("x_fp", x_fp);
    per_organism!("y_fp", y_fp);
    per_organism!("energy_milli", energy_milli);
    per_organism!("age_ticks", age_ticks);
    per_organism!("cooldown_ticks", cooldown_ticks);

    if let Some(phase2) = phase2 {
        let sim_core::Phase2SaveState {
            traits,
            neural,
            memory,
            heading_bam,
            speed_milli,
            last_turn,
            parents,
            depth,
            child_count,
            birth_tick,
            // Aggregate, not per organism.
            counters: _,
        } = phase2;
        // `traits` and `neural` are empty in a schema-2 world by construction:
        // the genome lives in the schema-2 section. They are swept anyway, so
        // that a future world which populates them cannot skip the sweep.
        per_organism!("phase2.traits", traits);
        per_organism!("phase2.neural", neural);
        per_organism!("phase2.memory", memory);
        per_organism!("phase2.heading_bam", heading_bam);
        per_organism!("phase2.speed_milli", speed_milli);
        per_organism!("phase2.last_turn", last_turn);
        per_organism!("phase2.parents", parents);
        per_organism!("phase2.depth", depth);
        per_organism!("phase2.child_count", child_count);
        per_organism!("phase2.birth_tick", birth_tick);
    }
    if let Some(contest) = contest {
        let sim_core::ContestSaveState {
            health_milli,
            recent_damage_milli,
            // Carcasses are their own entities, not a per-organism array.
            carcasses: _,
            carcass_created_milli: _,
            carcass_consumed_milli: _,
            carcass_decayed_milli: _,
            attacks_total: _,
            damage_dealt_milli: _,
            deaths_by_damage_total: _,
            healed_milli: _,
        } = contest;
        per_organism!("contest.health_milli", health_milli);
        per_organism!("contest.recent_damage_milli", recent_damage_milli);
    }
    if let Some(physiology) = physiology {
        let sim_core::PhysiologySaveState {
            cumulative_hazard_q16,
            deaths_senescence_total: _,
            deaths_extrinsic_total: _,
            deaths_juvenile_total: _,
            thermal_cost_milli: _,
            allometric_cost_milli: _,
        } = physiology;
        per_organism!("physiology.cumulative_hazard_q16", cumulative_hazard_q16);
    }
    if let Some(schema2) = schema2 {
        let sim_core::Schema2SaveState {
            genomes,
            activation_values,
            activation_prior,
            activation_faults,
            // Aggregate, not per organism.
            counters: _,
        } = schema2;
        per_organism!("schema2.genomes", genomes);
        per_organism!("schema2.activation_values", activation_values);
        per_organism!("schema2.activation_prior", activation_prior);
        per_organism!("schema2.activation_faults", activation_faults);
    }
    if let Some(learn) = learn {
        let sim_core::LearnSaveState {
            edges,
            faults,
            cost_remainder,
            // Aggregate, not per organism.
            counters: _,
            cost_milli: _,
        } = learn;
        per_organism!("learn.edges", edges);
        per_organism!("learn.faults", faults);
        per_organism!("learn.cost_remainder", cost_remainder);
    }
    if let Some(census) = action_census {
        let sim_core::ActionCensusSaveState {
            counts,
            // Aggregate, not per organism.
            counters: _,
        } = census;
        per_organism!("action_census.counts", counts);
    }
    // `permutation_config` is a **Phase 9** world and leaves plasticity off,
    // so the branch above does not run here and these two arrays contribute
    // nothing to this file's inventory. They are named and swept anyway, for
    // the reason the doc comment gives: a per-organism array that can join
    // `SaveState` without joining this function is the defect this function
    // exists to make impossible, and the guard has to be in place before the
    // section is switched on rather than after. The plasticity world's own
    // sweep of exactly these two arrays is
    // `phase11_learning::scrambling_the_learned_state_changes_a_plasticity_world`,
    // which is where Phase 11's half of the permutation clause lives - here
    // would entangle C9.7's evidence with a section Phase 9 does not have.
    inventory
}

/// Restore and advance, returning `(checksum at restore, checksum after
/// `ticks`)`, or the refusal.
fn restore_and_advance(state: SaveState, ticks: u64) -> Result<(u64, u64), RestoreError> {
    let mut world = World::from_state(state)?;
    let at_restore = world.state_checksum();
    for _ in 0..ticks {
        world.step();
    }
    Ok((at_restore, world.state_checksum()))
}

#[test]
fn permuting_every_per_organism_array_together_and_sorting_back_is_a_round_trip() {
    // Determinism-extensions Rule 4's recipe, in the only form `from_state`
    // admits: one permutation applied jointly to every per-organism array,
    // then the population sorted back into ascending-id order.
    //
    // **This is a positive control, not the evidence.** A complete joint
    // permutation followed by a sort back by id is an identity on the record
    // by construction, so this assertion cannot fail while the permutation is
    // complete. Its job is to establish that the transform is exact, so that
    // when the sweep below leaves one array out, the difference it produces is
    // a real desync rather than a harness artifact. The intermediate state is
    // asserted to differ from the original, so "the permutation did nothing"
    // cannot be why this passes.
    let world = advance(permutation_config(23), 3_000);
    assert!(world.population() > 20, "too few organisms to permute");
    let original = world.export_state();
    let baseline = restore_and_advance(original.clone(), 200).expect("baseline restores");

    let population = original.ids.len();
    // A rotation by a third of the population: no fixed points, and not an
    // involution, so it cannot be accidentally self-cancelling.
    let shift = (population / 3).max(1);
    let forward: Vec<usize> = (0..population)
        .map(|index| (index + shift) % population)
        .collect();
    let mut permuted = original.clone();
    apply_order(&mut permuted, &forward);
    assert_ne!(
        permuted.ids, original.ids,
        "the permutation is the identity, so this test compares a value with itself"
    );
    assert_eq!(
        World::from_state(permuted.clone()).err(),
        Some(RestoreError::EntityOrder),
        "a genuinely unsorted population was admitted; the fail-closed guard \
         this test relies on is not guarding"
    );

    let mut back: Vec<usize> = (0..population).collect();
    back.sort_by_key(|index| permuted.ids[*index]);
    apply_order(&mut permuted, &back);
    assert_eq!(
        permuted, original,
        "permute-then-canonicalize was not exact"
    );
    assert_eq!(
        restore_and_advance(permuted, 200).expect("canonicalized state restores"),
        baseline
    );
}

/// Reorder every per-organism array by `order`, jointly.
///
/// The same no-`..` destructure as `rotate_one_array`, for the same reason
/// and against the same failure: a new per-organism array must not be able to
/// join the save without joining this permutation, because the round trip
/// below would then quietly stop being a round trip.
fn apply_order(state: &mut SaveState, order: &[usize]) {
    fn reorder<T: Clone>(values: &mut Vec<T>, order: &[usize]) {
        *values = order.iter().map(|index| values[*index].clone()).collect();
    }
    let SaveState {
        config: _,
        tick: _,
        paused: _,
        extinct: _,
        next_entity_id: _,
        terrain_checksum: _,
        composed_terrain_checksum: _,
        biomass_milli: _,
        ledger: _,
        counters: _,
        climate: _,
        morphology: _,
        // Per cell and per layer, not per organism: the terrain delta is
        // world state that no permutation of the population can touch.
        worldmod: _,
        ids,
        x_fp,
        y_fp,
        energy_milli,
        age_ticks,
        cooldown_ticks,
        phase2,
        contest,
        physiology,
        schema2,
        learn,
        action_census,
    } = state;
    reorder(ids, order);
    reorder(x_fp, order);
    reorder(y_fp, order);
    reorder(energy_milli, order);
    reorder(age_ticks, order);
    reorder(cooldown_ticks, order);
    if let Some(phase2) = phase2 {
        let sim_core::Phase2SaveState {
            traits,
            neural,
            memory,
            heading_bam,
            speed_milli,
            last_turn,
            parents,
            depth,
            child_count,
            birth_tick,
            counters: _,
        } = phase2;
        // Empty in a schema-2 world: the flat genome lives in the schema-2
        // section instead. Asserted rather than quietly skipped, so a future
        // world that populates them fails here and has to be added to the
        // permutation deliberately - a silent skip is exactly the omission
        // this permutation exists to make impossible.
        assert!(
            traits.is_empty() && neural.is_empty(),
            "a schema-2 save carries flat genome arrays; they must join this \
             permutation before this test means anything again"
        );
        reorder(memory, order);
        reorder(heading_bam, order);
        reorder(speed_milli, order);
        reorder(last_turn, order);
        reorder(parents, order);
        reorder(depth, order);
        reorder(child_count, order);
        reorder(birth_tick, order);
    }
    if let Some(contest) = contest {
        let sim_core::ContestSaveState {
            health_milli,
            recent_damage_milli,
            carcasses: _,
            carcass_created_milli: _,
            carcass_consumed_milli: _,
            carcass_decayed_milli: _,
            attacks_total: _,
            damage_dealt_milli: _,
            deaths_by_damage_total: _,
            healed_milli: _,
        } = contest;
        reorder(health_milli, order);
        reorder(recent_damage_milli, order);
    }
    if let Some(physiology) = physiology {
        let sim_core::PhysiologySaveState {
            cumulative_hazard_q16,
            deaths_senescence_total: _,
            deaths_extrinsic_total: _,
            deaths_juvenile_total: _,
            thermal_cost_milli: _,
            allometric_cost_milli: _,
        } = physiology;
        reorder(cumulative_hazard_q16, order);
    }
    if let Some(schema2) = schema2 {
        let sim_core::Schema2SaveState {
            genomes,
            activation_values,
            activation_prior,
            activation_faults,
            counters: _,
        } = schema2;
        reorder(genomes, order);
        reorder(activation_values, order);
        reorder(activation_prior, order);
        reorder(activation_faults, order);
    }
    if let Some(learn) = learn {
        let sim_core::LearnSaveState {
            edges,
            faults,
            cost_remainder,
            counters: _,
            cost_milli: _,
        } = learn;
        reorder(edges, order);
        reorder(faults, order);
        reorder(cost_remainder, order);
    }
    if let Some(census) = action_census {
        let sim_core::ActionCensusSaveState {
            counts,
            counters: _,
        } = census;
        reorder(counts, order);
    }
}

#[test]
fn scrambling_any_one_per_organism_array_changes_the_world() {
    // **The evidence clause.** Leaving one array out of a permutation is
    // precisely the desync a storage-permutation test exists to catch, and it
    // is the one the round trip above cannot see. Every per-organism array is
    // scrambled on its own and the world must notice - by refusing the
    // restore, by checksumming differently, or by diverging within 200 ticks.
    //
    // An array that is carried in the save and never read would pass silently
    // under any equality-shaped test. Here it fails, unless it is constant, in
    // which case there was genuinely nothing to detect and the test says so
    // rather than crediting itself with a pass.
    let world = advance(permutation_config(23), 3_000);
    let original = world.export_state();
    assert!(original.ids.len() > 20, "too few organisms to scramble");
    let baseline = restore_and_advance(original.clone(), 200).expect("baseline restores");

    let inventory = rotate_one_array(&mut original.clone(), None);
    assert!(
        inventory.len() >= 20,
        "only {} per-organism arrays found; a section is missing from the sweep: {inventory:?}",
        inventory.len()
    );

    let mut detected = Vec::new();
    let mut uniform = Vec::new();
    for (name, is_uniform) in inventory {
        let mut scrambled = original.clone();
        rotate_one_array(&mut scrambled, Some(name));
        let noticed = match restore_and_advance(scrambled, 200) {
            // Fail-closed is a detection: `ids` rotated out of ascending order
            // is refused as `EntityOrder`, and a rotated activation buffer
            // whose length no longer matches its plan is refused too.
            Err(_) => true,
            Ok(result) => result != baseline,
        };
        if noticed {
            detected.push(name);
        } else {
            assert!(
                is_uniform,
                "{name} was scrambled and the world did not notice, and the \
                 array is not constant - so it is carried in the save and \
                 never read, or it never reaches the checksum"
            );
            uniform.push(name);
        }
    }
    // A world in which almost everything is constant would pass the loop above
    // while proving nothing, so the sweep has to have bitten. Measured: 18 of
    // 23 arrays detected. The five that are constant are constant for a legible
    // reason - `phase2.traits` and `phase2.neural` are empty in a schema-2
    // world, `phase2.memory` is zero because schema 2 has no memory registers,
    // `contest.recent_damage_milli` is zero because no attack landed in the
    // last window, and `schema2.activation_faults` is zero because nothing
    // faulted.
    assert!(
        detected.len() >= 16,
        "only {} of {} arrays were detectable; the rest were constant, which \
         means this world is too uniform to be evidence. detected={detected:?} \
         constant={uniform:?}",
        detected.len(),
        detected.len() + uniform.len()
    );
    assert!(
        detected.contains(&"schema2.genomes")
            && detected.contains(&"schema2.activation_values")
            && detected.contains(&"schema2.activation_prior"),
        "the schema-2 arrays this criterion is about were not detected: \
         detected={detected:?} constant={uniform:?}"
    );
}

#[test]
fn a_save_whose_population_is_not_in_id_order_is_refused() {
    // The fail-closed control the sweep above leans on, stated on its own so
    // it cannot be lost inside a loop. `from_state` is the only door into a
    // world from untrusted bytes, and admitting an unsorted population would
    // make every ordering guarantee downstream an assumption.
    let world = advance(permutation_config(31), 800);
    let mut state = world.export_state();
    assert!(state.ids.len() > 2);
    state.ids.swap(0, 1);
    assert_eq!(
        World::from_state(state).err(),
        Some(RestoreError::EntityOrder)
    );
}

// --- compaction --------------------------------------------------------------

/// A world configured to reproduce and die hard enough that compaction is
/// exercised many times, and to grow edges so that in-degree exceeds one.
fn compaction_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 200;
    config.max_entities = 8_000;
    config.cell_capacity_milli = 120_000;
    config.genome2.enabled = true;
    // Insertion is what raises in-degree: duplication copies a locus, while
    // insertion adds an edge between nodes that already exist. Without it,
    // every organism is the founder chain, every node has at most one incoming
    // edge, and "the incoming lists are ascending" would be a claim about
    // singleton lists.
    config.genome2.mutation.duplication_q16 = 6_554;
    config.genome2.mutation.insertion_q16 = 6_554;
    config
}

#[test]
fn compaction_leaves_survivors_evaluating_exactly_as_they_did() {
    // C9.7's third clause. Compaction is real - `retain_by_flags` in
    // `world.rs` and `Schema2State::retain` in `schema2.rs`, both called from
    // the lifecycle phase on death - and it is the only thing in this engine
    // that permutes storage order, because ids are otherwise strictly
    // ascending and appended.
    //
    // **The point is a regression guard against a future layout refactor, not
    // a bug hunt.** Nothing today reorders loci within an organism: a genome's
    // loci ascend by `homology_id`, `express_network` sorts, and `compile`
    // walks the sorted edge list, so each node's incoming list is ascending by
    // construction. Compaction moves whole organisms between slots and nothing
    // finer. This test is what would fail if that ever stopped being true.
    //
    // The load-bearing assertion is the save-restore-continue equality, and
    // the reason is worth stating: **a restore rebuilds every compiled plan
    // from the saved genome**, so if the live world's `plans[i]` had drifted
    // away from its `genomes[i]` during a compaction, the restored world would
    // evaluate different networks and diverge. The equality is therefore
    // evidence about the live world's arrays, not just about the codec.
    let mut world = World::new(compaction_config(53)).expect("world");
    for tick in 1..=8_000_u64 {
        world.step();
        if tick % 1_000 == 0 {
            world
                .check_invariants()
                .unwrap_or_else(|violation| panic!("tick {tick}: {violation}"));
        }
    }
    let metrics = world.metrics();
    assert!(
        metrics.deaths_starvation_total + metrics.deaths_old_age_total > 0,
        "nothing died, so compaction never ran and this test is vacuous"
    );
    assert!(
        metrics.births_total > 0,
        "nothing was born, so the arrays only ever shrank"
    );
    assert!(world.population() > 10, "the world went extinct");

    // Every survivor's incoming-edge lists must still ascend by
    // `homology_id`, and at least one node must have more than one incoming
    // edge - otherwise "ascending" is a statement about lists of length one
    // and holds for any implementation whatsoever.
    let state = world.export_state();
    let schema2 = state.schema2.as_ref().expect("schema 2 is enabled");
    let caps: GenomeCaps = state.config.genome2.caps;
    let mut max_in_degree = 0_usize;
    for (index, bytes) in schema2.genomes.iter().enumerate() {
        let genome = Genome2::decode(bytes, &caps).expect("a live genome decodes");
        let plan = compile_network(&genome.express_network()).expect("a live genome compiles");
        for incoming in &plan.incoming {
            max_in_degree = max_in_degree.max(incoming.len());
            assert!(
                incoming
                    .windows(2)
                    .all(|pair| pair[0].homology_id < pair[1].homology_id),
                "organism {index} has an incoming-edge list that is not \
                 ascending by homology_id after compaction: {incoming:?}"
            );
        }
    }
    assert!(
        max_in_degree >= 2,
        "no survivor has a node with more than one incoming edge, so the \
         ordering assertion above is about singleton lists and proves nothing"
    );

    // Save, restore, continue: the live plans must match the live genomes.
    let mut restored = World::from_state(state.clone()).expect("restore");
    assert_eq!(restored.state_checksum(), world.state_checksum());
    for _ in 0..500 {
        world.step();
        restored.step();
    }
    assert_eq!(
        restored.state_checksum(),
        world.state_checksum(),
        "a world restored after compaction diverged, which means the live \
         world's compiled plans were not the plans its genomes describe"
    );
}
