//! Phase 14 mate choice (ADR-0030 decision 2): pairing selects the
//! highest-preference candidate over its perceived cues; ties - and the
//! all-neutral preference - reproduce proximity pairing exactly; the
//! P-scramble arm permutes which cue vector belongs to which candidate
//! and is checked by counter.
//!
//! Gate E scripted ground truth: three organisms with engineered positions
//! and engineered preference genomes, movement frozen to below one
//! fixed-point unit per tick, everyone else far outside pairing range - so
//! whichever of the trio pairs first, the test can compute from the saved
//! positions exactly which partner an informed chooser must take.

use sim_core::{EventKind, FP_PER_METER, LocusKind, PREFERENCE_TRAIT_BASE, SimConfig, World};

const SEED: u64 = 0x0f14_c405_0f14_c405;

fn matechoice_config(seed: u64, scramble: bool) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 48;
    config.cells_y = 48;
    config.initial_organisms = 24;
    config.max_entities = 2_000;
    config.cell_capacity_milli = 240_000;
    // Movement frozen to under one fixed-point unit per tick (validation
    // refuses zero), so the engineered distances hold for the whole test.
    config.speed_mps_q16 = 1;
    config.genome2.enabled = true;
    config.physiology.enabled = true;
    config.physiology.senescence_enabled = false;
    config.physiology.extrinsic_hazard_q16_per_s = 0;
    config.physiology.juvenile_hazard_multiplier_q16 = 65_536;
    config.physiology.mate_choice_enabled = true;
    config.physiology.mate_choice_scramble = scramble;
    config
}

/// Set the proximity-cue preference locus (cue 1) to `gene` on both
/// haplotypes of an organism's OWN genome: 0.0 expresses weight -1
/// (prefer far), 0.5 is neutral, 1.0 expresses +1 (prefer near). Editing
/// in place keeps the organism's controller and traits exactly as the
/// world built them - replacing the whole genome replaced the controller
/// too, and a controller that never emits mate intent pairs with no one.
fn set_proximity_preference(encoded: &mut Vec<u8>, caps: &sim_core::GenomeCaps, gene: f32) {
    let mut genome = sim_core::Genome2::decode(encoded, caps).expect("stored genome decodes");
    for haplotype in &mut genome.haplotypes {
        for chromosome in &mut haplotype.chromosomes {
            for locus in chromosome.iter_mut() {
                if let LocusKind::Trait {
                    trait_id, value, ..
                } = &mut locus.kind
                    && *trait_id == PREFERENCE_TRAIT_BASE + 1
                {
                    *value = gene;
                }
            }
        }
    }
    *encoded = genome.encode();
}

/// A horizontal strip of traversable cells wide enough for the trio and
/// its births, probed from the world's own composed terrain rather than
/// assumed - position validity is checked on restore, and a seed's water
/// cells are wherever they are.
fn traversable_strip(world: &World) -> (i32, i32) {
    let metre = FP_PER_METER;
    let extent_x_m = 48 * 4; // cells_x * cell_size_m (4 m cells)
    let extent_y_m = extent_x_m;
    for y_m in (6..extent_y_m - 6).step_by(2) {
        'columns: for x_m in (6..extent_x_m - 12).step_by(2) {
            for dx_m in 0..=4 {
                let cell = world.cell_index_of((x_m + dx_m) * metre, y_m * metre);
                // Traversable AND fertile: an infertile strip starves the
                // trio below the pairing energy threshold and the test
                // then watches nothing but hunger.
                if !world.effective_traversable(cell)
                    || world.terrain().capacity_milli[cell] < 2_000
                {
                    continue 'columns;
                }
            }
            return (x_m * metre, y_m * metre);
        }
    }
    panic!("no fertile traversable 5 m strip in this seed's terrain");
}

/// A world whose organisms 0, 1, 2 sit at engineered positions with the
/// given proximity-preference gene, mature; everyone else stays exactly
/// where the world put them but at age zero - immature, so ineligible as
/// chooser or candidate for the whole test window. Returns the world and
/// the trio's entity ids.
fn trio_world(config: &SimConfig, gene: f32) -> (World, [u64; 3], [(i32, i32); 3]) {
    let world = World::new(config.clone()).expect("world");
    let mut state = world.export_state();
    let metre = FP_PER_METER;
    let (base_x, base_y) = traversable_strip(&world);
    // A at the strip base; B 0.5 m right of A; C 1.5 m right of A. All
    // pairwise distances sit well inside the 4 m pairing range, with slack
    // for drift: movement is slowed to centimetres per tick, not frozen.
    let trio = [
        (base_x, base_y),
        (base_x + metre / 2, base_y),
        (base_x + 3 * metre / 2, base_y),
    ];
    let population = state.ids.len();
    for index in 0..population {
        if index < 3 {
            state.x_fp[index] = trio[index].0;
            state.y_fp[index] = trio[index].1;
            state.age_ticks[index] = 700;
        } else {
            state.age_ticks[index] = 0;
        }
    }
    let schema2 = state.schema2.as_mut().expect("schema2 section");
    for index in 0..3 {
        set_proximity_preference(&mut schema2.genomes[index], &config.genome2.caps, gene);
    }
    let ids = [state.ids[0], state.ids[1], state.ids[2]];
    let world = World::from_state(state).expect("trio state restores");
    (world, ids, trio)
}

/// Step until the first MateChoice event whose chooser is one of the trio
/// and whose candidate set is both partners - energy dips below the
/// pairing threshold can thin the set on some ticks, and a one-candidate
/// choice decides nothing about preference. Returns the chosen candidate's
/// true proximity cue and the candidate-set proximity sum, both in milli:
/// organisms drift centimetres per tick, so the *initial* layout goes
/// stale, but the event records the truth at the choice itself.
fn first_trio_choice(world: &mut World, ids: &[u64; 3], ticks: u64) -> (i32, i64, bool) {
    for _ in 0..ticks {
        world.step();
        for event in world.events() {
            if let EventKind::MateChoice {
                chooser,
                candidates,
                scrambled,
                chosen_cues_milli,
                cue_sums_milli,
                ..
            } = event.kind
                && ids.contains(&chooser)
                && candidates == 2
            {
                return (chosen_cues_milli[1], cue_sums_milli[1], scrambled);
            }
        }
    }
    panic!("no two-candidate trio pairing within {ticks} ticks");
}

#[test]
fn a_far_preferring_chooser_takes_the_farther_candidate() {
    let config = matechoice_config(SEED, false);
    let (mut world, ids, positions) = trio_world(&config, 0.0);
    let _ = positions;
    let (chosen_proximity, proximity_sum, scrambled) = first_trio_choice(&mut world, &ids, 800);
    assert!(!scrambled);
    // Two candidates: the other one's proximity is the sum minus the
    // chosen's. A weight of -1 on the proximity cue must take the farther
    // candidate - the one with the LOWER proximity - which proximity
    // pairing can never do (ties excepted, and the trio's spacing keeps
    // the two distances apart).
    let other_proximity = proximity_sum - i64::from(chosen_proximity);
    assert!(
        i64::from(chosen_proximity) < other_proximity,
        "the far-preferring chooser took the nearer candidate \
         (chosen proximity {chosen_proximity}, other {other_proximity})"
    );
}

#[test]
fn a_neutral_preference_reproduces_proximity_pairing() {
    // The same base seed as the far-preference test: identical worldgen,
    // founders and controllers, so the tick-1 two-candidate window that
    // test demonstrated exists here too - only the preference gene
    // differs.
    let config = matechoice_config(SEED, false);
    let (mut world, ids, positions) = trio_world(&config, 0.5);
    let _ = positions;
    let (chosen_proximity, proximity_sum, _) = first_trio_choice(&mut world, &ids, 800);
    let other_proximity = proximity_sum - i64::from(chosen_proximity);
    assert!(
        i64::from(chosen_proximity) >= other_proximity,
        "an all-neutral preference must pick the nearest candidate, exactly as \
         the pre-Phase-14 loop did (chosen proximity {chosen_proximity}, \
         other {other_proximity})"
    );
}

#[test]
fn the_scramble_arm_is_checked_by_its_own_counter() {
    // Same base seed again; only the scramble flag differs.
    let config = matechoice_config(SEED, true);
    let (mut world, ids, _positions) = trio_world(&config, 0.0);
    let (_, _, scrambled) = first_trio_choice(&mut world, &ids, 800);
    let _ = _positions;
    assert!(scrambled, "a two-candidate choice under the gate must scramble");
    let metrics = world.metrics();
    assert!(metrics.mate_choice_enabled);
    assert!(metrics.choices_total > 0);
    assert_eq!(
        metrics.choices_total, metrics.scrambled_choices_total,
        "every choice in a scramble world must be counted as scrambled \
         (single-candidate choices excepted, and the trio always has two)"
    );
}

#[test]
fn the_gate_off_world_saves_no_section_and_counts_nothing() {
    let mut config = matechoice_config(SEED ^ 0x3, false);
    config.physiology.mate_choice_enabled = false;
    config.physiology.mate_choice_scramble = false;
    let mut world = World::new(config).expect("world");
    for _ in 0..300 {
        world.step();
    }
    let state = world.export_state();
    assert!(state.matechoice.is_none(), "gate off must save no section");
    let metrics = world.metrics();
    assert!(!metrics.mate_choice_enabled);
    assert_eq!(metrics.choices_total, 0);
    for event in world.events() {
        assert!(
            !matches!(event.kind, EventKind::MateChoice { .. }),
            "a gate-off world must emit no MateChoice event"
        );
    }
}

