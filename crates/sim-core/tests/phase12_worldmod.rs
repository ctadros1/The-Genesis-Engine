//! Phase 12 mutable world: the tick integration, not the set arithmetic.
//!
//! Sortedness, uniqueness, the per-layer cap, the value domains, and the
//! composed-checksum computation are unit-tested in `terrainmod.rs`. What is
//! left, and what this file is for, is everything that only exists once a
//! whole world is running: that a **disabled** section reproduces the
//! fixtures byte for byte, that the composed accessors are the identity when
//! the section is absent, that lowering a capacity trims and ledgers exactly,
//! that the biomass identity survives a long run full of relocations, and
//! that the zero-magnitude control is a real control rather than a
//! differently-shaped run.
//!
//! # The four fixtures, and which of them is reachable from here
//!
//! C12.8 names Phase 1 `0x1e3158a26afd3b39`, Phase 2 `0xff9dfcff5dffbf42`,
//! Phase 9 `0x5f0c4e95e4f5170f`, and Phase 11 `0x53b354bd94e82bcf`. The first
//! three are ordinary worlds and are pinned in process below. The fourth is a
//! 10^6-tick single-organism trace whose founder genomes are rewritten
//! through the save path by `crates/sim-cli/src/main.rs`, so it is not
//! constructible from `sim-core` and it is not affordable in `cargo test`
//! either; `scripts/verify-phase11-determinism.sh` is its check, and the
//! plasticity **lineage** is covered here by a shorter equality run that
//! would catch a Phase 12 field leaking into a schema-2 plasticity world.

use sim_core::{
    InheritanceMode, LAYER_CAPACITY_SCALE, LAYER_TRAVERSABLE, ModOutcome, SimConfig, World,
};

const SEED: u64 = 0x5eed_cafe_f00d_beef;
/// Q16 one, restated rather than imported so the arithmetic below reads.
const ONE_Q16: u32 = 65_536;

/// The Phase 9 fixture's configuration, pinned field by field (D-078).
///
/// Duplicated from `phase9_determinism.rs` rather than shared, deliberately
/// and for the reason that file gives: the point of pinning is that the
/// fixture does not move when a default does, and a helper imported from
/// elsewhere is one more place a default could leak in from.
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

/// A world with the section on and the schedule off: the artifact half's
/// starting point, and the narrowest world the write path can be tested in.
fn quiet_worldmod_config() -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = false;
    config
}

/// A world with the relocating patch live. `interval` and `scale` are the two
/// knobs the treatment/control pair differs in - and only `scale` differs
/// between the arms.
fn patch_config(interval: u64, radius: u32, scale_q16: u32) -> SimConfig {
    let mut config = SimConfig::phase1_default(SEED);
    config.worldmod.enabled = true;
    config.worldmod.patch_enabled = true;
    config.worldmod.relocate_interval_ticks = interval;
    config.worldmod.patch_radius_cells = radius;
    config.worldmod.patch_capacity_scale_q16 = scale_q16;
    config
}

// --- C12.8: a disabled section changes nothing ------------------------------

#[test]
fn a_disabled_worldmod_section_reproduces_every_reachable_fixture_exactly() {
    // The whole of C12.8 for this stage, and the assertion is against the
    // *published constants*, not against a second run of the same build. Two
    // runs of one build agree whether or not the constants moved, which is
    // how a fixture stops being evidence without anyone noticing.
    let cases: [(SimConfig, u64, u64, u64); 3] = [
        (
            SimConfig::phase1_default(SEED),
            500,
            0x918a_381c_7755_9236,
            0x1e31_58a2_6afd_3b39,
        ),
        (
            SimConfig::phase2_default(SEED),
            500,
            0xf83d_3981_bf7d_d189,
            0xff9d_fcff_5dff_bf42,
        ),
        (
            phase9_fixture_config(),
            8_000,
            0x9abc_0cd4_7914_127f,
            0x5f0c_4e95_e4f5_170f,
        ),
    ];
    for (config, ticks, config_hash, state_checksum) in cases {
        assert!(!config.worldmod.enabled, "the section defaults to off");
        assert_eq!(config.stable_hash(), config_hash, "config hash moved");
        let world = advance(config, ticks);
        assert_eq!(world.state_checksum(), state_checksum, "checksum moved");

        // ...and it stays out even when every other field of the section is
        // moved, which is the assertion an `enabled` check alone would not
        // make: a hash that folded a disabled section's fields in would tie
        // the fixture to defaults that are explicitly provisional.
        let mut moved = config;
        moved.worldmod.dense_threshold_q16 = 1;
        moved.worldmod.max_traversable_overrides = 1;
        moved.worldmod.max_capacity_overrides = 1;
        moved.worldmod.max_material_overrides = 1;
        moved.worldmod.patch_enabled = true;
        moved.worldmod.relocate_interval_ticks = 1;
        moved.worldmod.patch_radius_cells = 1;
        moved.worldmod.patch_capacity_scale_q16 = 1;
        assert_eq!(moved.stable_hash(), config_hash, "a disabled field hashed");
        let moved_world = advance(moved, ticks);
        assert_eq!(
            moved_world.state_checksum(),
            state_checksum,
            "a disabled section's fields reached the tick"
        );
        assert!(!moved_world.metrics().worldmod_enabled);
        assert_eq!(moved_world.metrics().worldmod_overrides, 0);
    }
}

#[test]
fn a_disabled_worldmod_section_leaves_the_plasticity_lineage_untouched() {
    // The Phase 11 fixture itself is a CLI-level 10^6-tick trace; this is the
    // clause of C12.8 that a `cargo test` run can afford. It compares a
    // schema-2 plasticity world with the section's fields moved against the
    // same world with them at their defaults, which is what would catch a
    // Phase 12 field leaking into `stable_hash` or into the tick.
    let base = SimConfig::phase11_default(SEED);
    let reference = advance(base, 2_000);
    let mut moved = base;
    moved.worldmod.patch_enabled = true;
    moved.worldmod.patch_capacity_scale_q16 = 7 * ONE_Q16;
    moved.worldmod.relocate_interval_ticks = 3;
    assert_eq!(moved.stable_hash(), base.stable_hash());
    assert_eq!(
        advance(moved, 2_000).state_checksum(),
        reference.state_checksum()
    );
    // Non-vacuity: this world has to be a live schema-2 plasticity world
    // that reproduced, or the equality above is between two empty runs.
    let metrics = reference.metrics();
    assert!(metrics.plasticity_enabled && metrics.genome2_enabled);
    assert!(metrics.population > 0, "the reference world died out");
    assert!(
        metrics.births_total > 0,
        "the schema-2 birth path never ran"
    );
}

#[test]
fn the_composed_accessors_are_the_identity_when_the_section_is_absent() {
    // The property that makes the fixtures above possible, asserted directly
    // instead of inferred from them: with no section, every cell's composed
    // view equals its raw terrain value, on both accessors, for every cell.
    let world = advance(SimConfig::phase1_default(SEED), 50);
    assert!(world.worldmod_state().is_none());
    let terrain = world.terrain();
    for cell in 0..terrain.cell_count() {
        assert_eq!(world.effective_traversable(cell), terrain.land[cell]);
        assert_eq!(
            world.effective_capacity_milli(cell),
            terrain.capacity_milli[cell]
        );
    }
    assert_eq!(world.composed_terrain_checksum(), terrain.terrain_checksum);
    assert_eq!(world.worldmod_capacity_loss_milli(), 0);

    // With climate on, the composed capacity is the **biome-scaled** value
    // and not the raw one, on at least some cell. That is the assertion that
    // says the worldmod arm was appended to the climate arm rather than
    // replacing it: a `effective_capacity_milli` rewritten to compose the
    // override with the *raw* capacity would pass every test above and
    // silently drop Phase 6 out of every climate world.
    let climate = advance(SimConfig::phase6_default(7), 50);
    assert!(climate.worldmod_state().is_none());
    let terrain = climate.terrain();
    assert!(
        (0..terrain.cell_count())
            .any(|cell| climate.effective_capacity_milli(cell) != terrain.capacity_milli[cell]),
        "no cell's capacity was biome-scaled, so the climate arm is unreached"
    );
    for cell in 0..terrain.cell_count() {
        assert_eq!(climate.effective_traversable(cell), terrain.land[cell]);
    }
}

#[test]
fn an_enabled_but_empty_section_changes_the_lineage_and_not_the_terrain() {
    // D-014's rule has two halves and they point opposite ways. Enabling a
    // section **must** change the config hash and the state checksum - a
    // world whose terrain organisms can edit is a different experiment - and
    // it must **not** change what any cell currently is.
    let quiet = quiet_worldmod_config();
    let plain = SimConfig::phase1_default(SEED);
    assert_ne!(quiet.stable_hash(), plain.stable_hash());

    let world = advance(quiet, 200);
    let reference = advance(plain, 200);
    assert_ne!(world.state_checksum(), reference.state_checksum());
    assert_eq!(world.worldmod_state().expect("section").len(), 0);
    assert_eq!(
        world.composed_terrain_checksum(),
        world.terrain().terrain_checksum
    );
    // Everything an organism could observe is identical, which is what makes
    // "enabled but empty" a usable baseline for the artifact half.
    assert_eq!(
        world.metrics().total_biomass_milli,
        reference.metrics().total_biomass_milli
    );
    assert_eq!(world.metrics().population, reference.metrics().population);
    assert_eq!(world.organism_ids_view(), reference.organism_ids_view());

    // ...and the section's *contents* reach the state checksum, not only its
    // presence. Without this the section could be omitted from
    // `state_checksum` entirely and every other assertion in this file would
    // still pass: the config hash alone already separates an enabled world
    // from a disabled one, so "the checksums differ" proves nothing about
    // the section being hashed.
    let mut modified = world.clone();
    let before = modified.state_checksum();
    let cell = richest_cell(&modified);
    assert_eq!(
        modified.apply_terrain_modification(
            LAYER_CAPACITY_SCALE,
            cell,
            Some(4 * i64::from(ONE_Q16))
        ),
        ModOutcome::Inserted
    );
    assert_eq!(
        modified.metrics().total_biomass_milli,
        world.metrics().total_biomass_milli
    );
    assert_ne!(
        modified.state_checksum(),
        before,
        "the modification set does not reach the state checksum"
    );
}

// --- The write path: trimming, ledgering, and the occupancy policy ----------

/// The habitable cell holding the most biomass, so a capacity cut has
/// something to trim. Panics rather than returning `None`: a world with no
/// biomass would make every assertion below vacuous.
fn richest_cell(world: &World) -> usize {
    let biomass = world.biomass_cells();
    let best = (0..biomass.len())
        .max_by_key(|cell| biomass[*cell])
        .expect("cells exist");
    assert!(biomass[best] > 0, "no cell holds biomass");
    best
}

#[test]
fn lowering_a_capacity_trims_the_excess_and_ledgers_it_to_the_milli() {
    let mut world = World::new(quiet_worldmod_config()).expect("world");
    for _ in 0..200 {
        world.step();
    }
    let cell = richest_cell(&world);
    let before = world.biomass_cells()[cell];
    let capacity_before = world.effective_capacity_milli(cell);
    assert!(before > 0 && capacity_before > 0);

    // Half capacity. `before` may already be under half, so the expected
    // trim is computed rather than assumed - and asserted to be nonzero, or
    // the test would pass by trimming nothing.
    let halved = capacity_before / 2;
    assert!(before > halved, "the chosen cell has nothing to trim");
    assert_eq!(
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell, Some(i64::from(ONE_Q16 / 2))),
        ModOutcome::Inserted
    );
    let expected_loss = i128::from(before - halved);
    assert_eq!(world.effective_capacity_milli(cell), halved);
    assert_eq!(world.biomass_cells()[cell], halved);
    assert_eq!(world.worldmod_capacity_loss_milli(), expected_loss);
    assert_eq!(world.metrics().worldmod_cells_trimmed, 1);
    // The conservation identity is exact with the new sink in it. Without
    // the sink this is off by `expected_loss` and the invariant says so.
    world.check_invariants().expect("ledger stays exact");

    // Raising the capacity back trims nothing - the headroom fills through
    // `grow_food`'s ordinary term - and the loss already taken does not
    // come back, because it did not go anywhere it could come back from.
    assert_eq!(
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell, Some(i64::from(4 * ONE_Q16))),
        ModOutcome::Replaced
    );
    assert_eq!(world.worldmod_capacity_loss_milli(), expected_loss);
    assert_eq!(world.metrics().worldmod_cells_trimmed, 1);
    assert_eq!(world.biomass_cells()[cell], halved);
    world.check_invariants().expect("raising conserves");

    // Clearing an override is a capacity change too, and it can lower: the
    // cell was at 4x and returns to baseline. It has not had time to grow
    // past baseline here, so nothing is trimmed - the assertion that matters
    // is that the clear went through the same trim path at all, which the
    // relocation test below exercises for real.
    assert_eq!(
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell, None),
        ModOutcome::Cleared
    );
    assert_eq!(world.effective_capacity_milli(cell), capacity_before);
    assert_eq!(world.worldmod_state().expect("section").len(), 0);
    world.check_invariants().expect("clearing conserves");
}

#[test]
fn a_cell_an_organism_stands_on_cannot_be_made_non_traversable() {
    // **The safety property the specification asks this phase to resolve
    // explicitly rather than discover.** Layer 0 has no producer until the
    // artifact half, so this is the only thing that has ever exercised the
    // policy, which is exactly why it is here rather than there.
    let mut world = World::new(quiet_worldmod_config()).expect("world");
    world.step();
    let state = world.export_state();
    let occupied = world.cell_index_of(state.x_fp[0], state.y_fp[0]);

    assert_eq!(
        world.apply_terrain_modification(LAYER_TRAVERSABLE, occupied, Some(0)),
        ModOutcome::RefusedOccupied
    );
    assert!(world.effective_traversable(occupied), "the block took hold");
    assert_eq!(world.metrics().worldmod_refusals, 1);
    world
        .check_invariants()
        .expect("a refused write changes nothing");

    // An unoccupied land cell blocks fine, so the refusal above is about
    // occupancy and not about the layer being inert.
    let terrain_land: Vec<usize> = (0..world.terrain().cell_count())
        .filter(|cell| world.terrain().land[*cell])
        .collect();
    let empty = *terrain_land
        .iter()
        .find(|cell| {
            !(0..state.ids.len())
                .any(|index| world.cell_index_of(state.x_fp[index], state.y_fp[index]) == **cell)
        })
        .expect("an empty land cell");
    assert_eq!(
        world.apply_terrain_modification(LAYER_TRAVERSABLE, empty, Some(0)),
        ModOutcome::Inserted
    );
    assert!(!world.effective_traversable(empty));
    world
        .check_invariants()
        .expect("blocking an empty cell is legal");
}

#[test]
fn a_permitted_water_cell_admits_organisms_and_then_cannot_be_un_permitted() {
    // Two claims in one run, and the first is the positive control for the
    // second. If no organism ever walks onto permitted water then
    // `effective_traversable`'s permit direction is unreached, the movement
    // sites might as well still read `terrain.land`, and the un-permit
    // refusal below would be asserting something about a situation that
    // cannot arise. So the search is asserted to succeed.
    let mut config = quiet_worldmod_config();
    // Every coastal water cell, so the cap has to hold them all.
    config.worldmod.max_traversable_overrides = config.cells_x * config.cells_y;
    let mut world = World::new(config).expect("world");

    let cells_x = world.terrain().cells_x as usize;
    let cells_y = world.terrain().cells_y as usize;
    let coastal: Vec<usize> = (0..world.terrain().cell_count())
        .filter(|&cell| {
            if world.terrain().land[cell] {
                return false;
            }
            let (x, y) = (cell % cells_x, cell / cells_x);
            [(1_i64, 0_i64), (-1, 0), (0, 1), (0, -1)]
                .into_iter()
                .any(|(dx, dy)| {
                    let (nx, ny) = (x as i64 + dx, y as i64 + dy);
                    nx >= 0
                        && ny >= 0
                        && (nx as usize) < cells_x
                        && (ny as usize) < cells_y
                        && world.terrain().land[(ny as usize) * cells_x + nx as usize]
                })
        })
        .collect();
    assert!(!coastal.is_empty(), "the continent has no coastline");
    for cell in &coastal {
        assert_eq!(
            world.apply_terrain_modification(LAYER_TRAVERSABLE, *cell, Some(1)),
            ModOutcome::Inserted
        );
    }

    // Movement jitter is one draw in eight, so a coastal organism reaches
    // permitted water quickly. The horizon is generous rather than tuned.
    let mut stranded = None;
    for _ in 0..2_000 {
        world.step();
        let state = world.export_state();
        if let Some(index) = (0..state.ids.len()).find(|&index| {
            let cell = world.cell_index_of(state.x_fp[index], state.y_fp[index]);
            !world.terrain().land[cell]
        }) {
            stranded = Some(world.cell_index_of(state.x_fp[index], state.y_fp[index]));
            break;
        }
    }
    let cell = stranded.expect(
        "no organism ever entered a permitted water cell, so the permit \
         direction of effective_traversable is untested",
    );
    // The composed position invariant accepts it: an organism on a permitted
    // water cell is legal, and a check still reading `terrain.land` would
    // fail right here.
    world
        .check_invariants()
        .expect("a permit makes water legal");

    // ...and un-permitting it is refused on exactly the same terms as
    // blocking a land cell, because it strands the organism identically.
    // This is the asymmetry that would have been the bug.
    assert_eq!(
        world.apply_terrain_modification(LAYER_TRAVERSABLE, cell, None),
        ModOutcome::RefusedOccupied
    );
    assert!(world.effective_traversable(cell));
    world.check_invariants().expect("still legal");
}

// --- The relocating patch and its zero-magnitude control --------------------

#[test]
fn the_patch_relocates_on_schedule_and_the_set_stays_sorted_and_unique() {
    let mut world = World::new(patch_config(200, 8, 2 * ONE_Q16)).expect("world");
    assert_eq!(world.worldmod_state().expect("section").len(), 0);

    let mut sizes = Vec::new();
    for tick in 1..=1_000_u64 {
        world.step();
        let state = world.worldmod_state().expect("section");
        // Sortedness and uniqueness after every single write, not only at
        // the end: a relocation that produced an unsorted array and then
        // happened to be overwritten by the next one would pass an
        // end-of-run check.
        assert_eq!(state.order_violation(), None, "unsorted at tick {tick}");
        assert_eq!(state.bounds_violation(world.terrain().cell_count()), None);
        // The patch is one contiguous footprint, so the set can never hold
        // more than `(2r+1)^2` entries - never the union of two patches. This
        // is what catches a relocation that adds the arriving cells and
        // forgets to clear the leaving ones: the world would still run, the
        // ledger would still balance, and the set would simply grow forever.
        assert!(
            state.len() <= 17 * 17,
            "the set holds {} entries at tick {tick}, more than one patch",
            state.len()
        );
        if tick.is_multiple_of(200) {
            sizes.push(state.len());
            assert_eq!(state.len(), state.layer_len(LAYER_CAPACITY_SCALE));
        }
        world.check_invariants().expect("invariants");
    }
    assert_eq!(sizes.len(), 5);
    assert!(sizes.iter().all(|size| *size > 0), "the patch is empty");
    assert!(
        sizes.windows(2).any(|pair| pair[0] != pair[1]),
        "every patch covered the same number of cells, so it may not be moving"
    );
    assert_eq!(world.metrics().worldmod_relocations, 5);
    assert_eq!(world.metrics().worldmod_refusals, 0);
    // The patch is a pure function of (seed, epoch), so a second world of the
    // same config lands in the same place - and a different seed does not.
    let repeat = advance(patch_config(200, 8, 2 * ONE_Q16), 1_000);
    assert_eq!(repeat.state_checksum(), world.state_checksum());
    let mut other_seed = patch_config(200, 8, 2 * ONE_Q16);
    other_seed.world_seed = SEED ^ 0xff;
    assert_ne!(
        advance(other_seed, 1_000).state_checksum(),
        repeat.state_checksum()
    );
}

#[test]
fn the_ledger_identity_holds_over_a_long_run_full_of_relocations() {
    // The identity is checked every tick by `check_invariants`, which is why
    // this test is a horizon rather than an assertion: the failure mode being
    // defended against is a trim path that is exact for one write and drifts
    // over thousands.
    let mut world = World::new(patch_config(500, 12, 3 * ONE_Q16)).expect("world");
    for tick in 1..=20_000_u64 {
        world.step();
        if tick.is_multiple_of(250) {
            world.check_invariants().expect("invariants");
        }
    }
    world.check_invariants().expect("invariants at the horizon");
    let metrics = world.metrics();
    assert_eq!(metrics.worldmod_relocations, 40);
    // Non-vacuity, and it is the whole test. A run in which the patch never
    // lowered a capacity would keep the identity exact by never using the new
    // term at all, and would look identical to a run that did.
    assert!(
        metrics.worldmod_capacity_loss_milli > 0,
        "no biomass was ever trimmed, so the new ledger term was never exercised"
    );
    assert!(metrics.worldmod_cells_trimmed > 0);
    assert!(metrics.population > 0, "the world died out");
}

#[test]
fn the_zero_magnitude_control_runs_the_identical_schedule_and_loses_no_biomass() {
    // **The control this phase's design turns on.** A schedule-free arm is
    // the wrong control: relocating a patch trims biomass into the loss sink
    // every time it leaves a cell, so a treatment arm carries lower standing
    // biomass than a schedule-free arm for reasons unrelated to the move. At
    // scale 1.0 the schedule runs identically and composes to exactly the
    // baseline capacity.
    const TICKS: u64 = 4_000;
    let control = advance(patch_config(400, 12, ONE_Q16), TICKS);
    let treatment = advance(patch_config(400, 12, 3 * ONE_Q16), TICKS);
    let schedule_free = advance(quiet_worldmod_config(), TICKS);

    // Clause 1: the two arms ran the same schedule. Same relocations, same
    // override count, same write dispositions - only the values differ.
    let control_state = control.worldmod_state().expect("section");
    let treatment_state = treatment.worldmod_state().expect("section");
    assert_eq!(control.metrics().worldmod_relocations, 10);
    assert_eq!(
        control.metrics().worldmod_relocations,
        treatment.metrics().worldmod_relocations
    );
    assert_eq!(control_state.len(), treatment_state.len());
    assert_eq!(control_state.cells, treatment_state.cells);
    assert_eq!(control_state.layers, treatment_state.layers);
    assert_ne!(control_state.values, treatment_state.values);
    assert!(!control_state.is_empty(), "neither arm wrote anything");

    // Clause 2: the sink separates them, and that is the point.
    assert_eq!(
        control.worldmod_capacity_loss_milli(),
        0,
        "the zero-magnitude control lost biomass, so it is not a control"
    );
    assert!(
        treatment.worldmod_capacity_loss_milli() > 0,
        "the treatment lost nothing, so the control's zero proves nothing"
    );
    assert_eq!(control.metrics().worldmod_cells_trimmed, 0);
    assert!(treatment.metrics().worldmod_cells_trimmed > 0);

    // Clause 3: the control is behaviorally the schedule-free world. Same
    // population, same organisms, same standing biomass to the milli - so
    // the only thing the schedule costs the control is the code path, which
    // is exactly what a matched control has to be.
    assert_eq!(
        control.metrics().total_biomass_milli,
        schedule_free.metrics().total_biomass_milli
    );
    assert_eq!(
        control.organism_ids_view(),
        schedule_free.organism_ids_view()
    );
    assert_ne!(
        treatment.metrics().total_biomass_milli,
        schedule_free.metrics().total_biomass_milli,
        "the treatment matched the schedule-free world, so the arms are not separated"
    );
    // ...and the two arms are nevertheless distinct replay lineages, because
    // the scale is in the config hash. Two arms sharing a hash would be two
    // runs of one experiment.
    assert_ne!(
        patch_config(400, 12, ONE_Q16).stable_hash(),
        patch_config(400, 12, 3 * ONE_Q16).stable_hash()
    );
}

#[test]
fn the_composed_checksum_tracks_the_live_world_over_a_run() {
    // The full recompute is the only composed checksum there is - see
    // `TerrainModState::composed_checksum` for why an incremental FNV-1a does
    // not exist - so what has to be checked is that it follows the world:
    // equal to the baseline while the set is empty, different once it is not,
    // and back to the baseline if the set empties.
    let mut world = World::new(patch_config(300, 10, 2 * ONE_Q16)).expect("world");
    let baseline = world.terrain().terrain_checksum;
    assert_eq!(world.composed_terrain_checksum(), baseline);
    for _ in 0..300 {
        world.step();
    }
    let after_first = world.composed_terrain_checksum();
    assert_ne!(after_first, baseline, "the first patch changed nothing");
    for _ in 0..300 {
        world.step();
    }
    assert_ne!(
        world.composed_terrain_checksum(),
        after_first,
        "the patch moved and the composed checksum did not"
    );
    // Emptying the set by hand returns it to the baseline exactly, which is
    // the identity the format 3 to format 4 migration is built on.
    let cells: Vec<u32> = world.worldmod_state().expect("section").cells.clone();
    for cell in cells {
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cell as usize, None);
    }
    assert_eq!(world.worldmod_state().expect("section").len(), 0);
    assert_eq!(world.composed_terrain_checksum(), baseline);
}

#[test]
fn an_invalid_write_is_refused_and_counted_rather_than_stored() {
    let mut world = World::new(quiet_worldmod_config()).expect("world");
    let cells = world.terrain().cell_count();
    // Layer id outside the registry.
    assert_eq!(
        world.apply_terrain_modification(3, 0, Some(1)),
        ModOutcome::RefusedInvalid
    );
    // Cell index past the end of the map.
    assert_eq!(
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, cells, Some(65_536)),
        ModOutcome::RefusedInvalid
    );
    // Value outside the layer's domain: a negative scale would make the
    // biomass bounds check unsatisfiable.
    assert_eq!(
        world.apply_terrain_modification(LAYER_CAPACITY_SCALE, 0, Some(-1)),
        ModOutcome::RefusedInvalid
    );
    // A traversability value that is neither 0 nor 1.
    assert_eq!(
        world.apply_terrain_modification(LAYER_TRAVERSABLE, 0, Some(2)),
        ModOutcome::RefusedInvalid
    );
    assert_eq!(world.worldmod_state().expect("section").len(), 0);
    assert_eq!(world.metrics().worldmod_refusals, 4);
    world.check_invariants().expect("refusals store nothing");

    // A world without the section refuses every write and stores nothing,
    // rather than panicking on a `None` it did not expect.
    let mut plain = World::new(SimConfig::phase1_default(SEED)).expect("world");
    assert_eq!(
        plain.apply_terrain_modification(LAYER_CAPACITY_SCALE, 0, Some(65_536)),
        ModOutcome::RefusedInvalid
    );
    assert!(plain.worldmod_state().is_none());
}
