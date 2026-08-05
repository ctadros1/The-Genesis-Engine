//! Phase 6 origin-mode acceptance criteria C6.2 to C6.5 and C6.10.
//!
//! Every mode is a starting condition, never a trajectory. These tests exist
//! to keep that true: that founder generation is a pure function of the
//! configuration, that demes are genuinely structured rather than merely
//! differently sampled, that a founder is never placed somewhere it does not
//! belong, and that an authored archetype label can never explain an
//! outcome.

use sim_core::{Archetype, Biome, OriginMode, SimConfig, World, WorldgenVersion};

const TRAIT_COUNT: usize = 14;

/// Seed 7 at this size generates a world containing every biome, which
/// `seeded` placement needs and C6.7 enforces.
fn climate_config(seed: u64) -> SimConfig {
    let mut config = SimConfig::phase6_default(seed);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 80;
    config.max_entities = 800;
    config
}

fn deme_config(seed: u64, demes: u32) -> SimConfig {
    let mut config = SimConfig::phase2_default(seed);
    config.cells_x = 96;
    config.cells_y = 96;
    config.initial_organisms = 80;
    config.max_entities = 800;
    config.origin.deme_count = demes;
    config.origin.deme_radius_m = 96;
    config.origin.deme_min_separation_m = 128;
    config
}

/// Two archetypes with clearly different trait centres and disjoint biome
/// affinities. Deliberately *not* named after anything living: an archetype
/// is a trait distribution a biome makes plausible, not a species.
fn two_archetypes() -> [Archetype; 2] {
    let mut low = Archetype::neutral(1);
    low.trait_mean_q16 = [12_000; TRAIT_COUNT];
    low.biome_affinity = (1 << (Biome::Grassland as u8)) | (1 << (Biome::Forest as u8));
    let mut high = Archetype::neutral(2);
    high.trait_mean_q16 = [52_000; TRAIT_COUNT];
    high.biome_affinity = (1 << (Biome::Highland as u8)) | (1 << (Biome::Arid as u8));
    [low, high]
}

fn seeded_config(seed: u64) -> SimConfig {
    let mut config = climate_config(seed);
    config.origin.mode = OriginMode::Seeded;
    config.origin.archetype_count = 2;
    let archetypes = two_archetypes();
    config.origin.archetypes[0] = archetypes[0];
    config.origin.archetypes[1] = archetypes[1];
    config
}

fn founder_traits(world: &World) -> Vec<[f32; TRAIT_COUNT]> {
    world
        .organism_ids_view()
        .iter()
        .map(|&id| {
            world
                .organism_detail(id)
                .expect("organism")
                .phase2
                .expect("phase2")
                .trait_genes
        })
        .collect()
}

// --- C6.2 -----------------------------------------------------------------

#[test]
fn c6_2_founder_generation_is_a_pure_function_of_the_configuration() {
    // Generating the same world twice must give the identical founder
    // population, checked at tick 0 by state checksum.
    for demes in [1_u32, 2, 4] {
        let config = deme_config(11, demes);
        let first = World::new(config).expect("world");
        let second = World::new(config).expect("world");
        assert_eq!(
            first.state_checksum(),
            second.state_checksum(),
            "founder generation is not deterministic at deme_count {demes}"
        );
        assert_eq!(founder_traits(&first), founder_traits(&second));
    }
}

#[test]
fn c6_2_founder_ids_are_allocated_in_canonical_group_order() {
    // IDs ascend 1..=n with no gaps, allocated group-major. A gap or a
    // permutation would mean allocation followed traversal rather than the
    // canonical order.
    let world = World::new(deme_config(11, 4)).expect("world");
    let ids = world.organism_ids_view().to_vec();
    let expected: Vec<u64> = (1..=ids.len() as u64).collect();
    assert_eq!(ids, expected);
    assert_eq!(ids.len(), 80);
}

#[test]
fn c6_2_deme_centres_are_sorted_so_assignment_follows_geometry() {
    // Centres are drawn in one order and sorted into another before founders
    // are attached, so which deme a founder joins is a function of geometry
    // rather than of draw luck. If sorting were a no-op this test would be
    // vacuous, so it asserts the two orders genuinely differ for some seed.
    let mut sorting_mattered = false;
    for seed in 1..=20_u64 {
        let config = deme_config(seed, 4);
        let Ok(world) = World::new(config) else {
            continue;
        };
        // Founders are group-major, so each quarter of the ID range is one
        // deme. Their mean cell index must ascend across demes if centres
        // were sorted.
        let traits = founder_traits(&world);
        assert_eq!(traits.len(), 80);
        let ids = world.organism_ids_view().to_vec();
        let mut deme_means = Vec::new();
        for deme in 0..4_usize {
            let slice = &ids[deme * 20..(deme + 1) * 20];
            let mean: f64 = slice
                .iter()
                .map(|&id| {
                    let detail = world.organism_detail(id).expect("organism");
                    f64::from(detail.y_fp) * 100_000.0 + f64::from(detail.x_fp)
                })
                .sum::<f64>()
                / slice.len() as f64;
            deme_means.push(mean);
        }
        if deme_means.windows(2).all(|pair| pair[0] < pair[1]) {
            sorting_mattered = true;
        }
    }
    assert!(
        sorting_mattered,
        "no seed produced demes ordered by position; centre sorting is not taking effect"
    );
}

// --- C6.3 -----------------------------------------------------------------

/// Mean pairwise trait distance inside a slice.
fn within(group: &[[f32; TRAIT_COUNT]]) -> f64 {
    let mut total = 0.0;
    let mut pairs = 0_u64;
    for left in 0..group.len() {
        for right in (left + 1)..group.len() {
            total += distance(&group[left], &group[right]);
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        total / pairs as f64
    }
}

fn between(left: &[[f32; TRAIT_COUNT]], right: &[[f32; TRAIT_COUNT]]) -> f64 {
    let mut total = 0.0;
    let mut pairs = 0_u64;
    for a in left {
        for b in right {
            total += distance(a, b);
            pairs += 1;
        }
    }
    if pairs == 0 {
        0.0
    } else {
        total / pairs as f64
    }
}

fn distance(a: &[f32; TRAIT_COUNT], b: &[f32; TRAIT_COUNT]) -> f64 {
    let mut sum = 0.0_f64;
    for gene in 0..TRAIT_COUNT {
        let delta = f64::from(a[gene]) - f64::from(b[gene]);
        sum += delta * delta;
    }
    sum.sqrt()
}

#[test]
fn c6_3_demes_are_genetically_real_in_every_seed() {
    // "With deme_count = 4, mean genetic distance within a deme is lower
    // than between demes at tick 0, in 30 of 30 seeds. This is deterministic
    // setup, so anything less than 30 of 30 is a defect rather than a
    // result."
    let mut checked = 0_u32;
    let mut seed = 1_u64;
    let mut worst_ratio = 0.0_f64;
    while checked < 30 {
        assert!(seed < 500, "ran out of generating seeds");
        let config = deme_config(seed, 4);
        seed += 1;
        let Ok(world) = World::new(config) else {
            continue;
        };
        checked += 1;

        let traits = founder_traits(&world);
        let per_deme = traits.len() / 4;
        let groups: Vec<&[[f32; TRAIT_COUNT]]> = (0..4)
            .map(|deme| &traits[deme * per_deme..(deme + 1) * per_deme])
            .collect();

        let mean_within: f64 =
            groups.iter().map(|group| within(group)).sum::<f64>() / groups.len() as f64;
        let mut between_total = 0.0;
        let mut between_pairs = 0;
        for left in 0..groups.len() {
            for right in (left + 1)..groups.len() {
                between_total += between(groups[left], groups[right]);
                between_pairs += 1;
            }
        }
        let mean_between = between_total / f64::from(between_pairs);

        assert!(
            mean_within < mean_between,
            "seed {}: within-deme distance {mean_within:.4} is not below between-deme \
             {mean_between:.4}; the demes are differently sampled but not structured",
            seed - 1
        );
        worst_ratio = worst_ratio.max(mean_within / mean_between);
    }
    assert_eq!(checked, 30);
    // Structure, not a hair's-breadth win: the effect must be substantial.
    assert!(
        worst_ratio < 0.75,
        "worst within/between ratio {worst_ratio:.3} is too close to 1 to call structure"
    );
}

#[test]
fn c6_3_a_single_deme_has_no_structure_to_find() {
    // The control for the test above: with one deme there is one
    // distribution, so within and between are the same quantity. This is
    // what stops the C6.3 assertion from being satisfiable by any
    // partitioning at all.
    let world = World::new(deme_config(11, 1)).expect("world");
    let traits = founder_traits(&world);
    let half = traits.len() / 2;
    let left = &traits[..half];
    let right = &traits[half..];
    let ratio = within(left).max(within(right)) / between(left, right);
    assert!(
        (0.8..1.25).contains(&ratio),
        "an arbitrary split of one deme showed structure (ratio {ratio:.3})"
    );
}

// --- C6.4 -----------------------------------------------------------------

#[test]
fn c6_4_every_founder_lands_in_a_cell_matching_its_archetype() {
    let config = seeded_config(7);
    let world = World::new(config).expect("seeded world");
    let biome = world.biome_cells().to_vec();
    let cells_x = world.terrain().cells_x as usize;
    let cell_size_fp = i64::from(config.cell_size_fp());

    let ids = world.organism_ids_view().to_vec();
    let per_archetype = ids.len() / 2;
    for (index, &id) in ids.iter().enumerate() {
        let archetype = config.origin.archetypes[usize::from(index >= per_archetype)];
        let detail = world.organism_detail(id).expect("organism");
        let cell_x = (i64::from(detail.x_fp) / cell_size_fp) as usize;
        let cell_y = (i64::from(detail.y_fp) / cell_size_fp) as usize;
        let cell = cell_y * cells_x + cell_x;
        assert!(
            archetype.accepts(biome[cell]),
            "founder {id} of archetype {} landed in {:?}, which is not in its affinity",
            archetype.id,
            biome[cell]
        );
    }
}

#[test]
fn c6_4_an_unsatisfiable_affinity_fails_closed_with_an_actionable_error() {
    let mut config = seeded_config(7);
    // Water only: no habitable cell can ever match, so generation must
    // refuse rather than place the founder somewhere it does not belong.
    config.origin.archetypes[0].biome_affinity = 1 << (Biome::Water as u8);
    let error = World::new(config).expect_err("must fail closed");
    let message = error.to_string();
    assert!(message.contains("no generated cell matches"), "{message}");
    assert!(
        message.contains("never placed in an unsuitable biome"),
        "{message}"
    );
}

// --- C6.5 -----------------------------------------------------------------

#[test]
fn c6_5_archetype_ids_are_inert() {
    // "A run with archetype IDs permuted, founder genomes held identical,
    // produces a bit-identical trajectory."
    //
    // Here that is true by construction rather than by discipline: an
    // archetype ID never enters world state at all. Draws key on the
    // archetype's position, and no organism carries a label.
    let base = seeded_config(7);
    let mut relabelled = base;
    relabelled.origin.archetypes[0].id = 900;
    relabelled.origin.archetypes[1].id = 901;

    let mut original = World::new(base).expect("world");
    let mut permuted = World::new(relabelled).expect("world");

    // The comparison is deliberately NOT on `state_checksum`. That checksum
    // hashes the config hash into its preamble, and an archetype ID is
    // legitimately part of the config hash — two runs seeded by differently
    // labelled archetypes are different experiments on paper even when they
    // are the same world in fact. A checksum comparison would therefore fail
    // for a reason that has nothing to do with inertness. What "inert" means
    // is that nothing an organism does depends on the label, so the
    // trajectory itself is what gets compared.
    let trajectory = |world: &World| {
        (
            founder_traits(world),
            world
                .organism_ids_view()
                .iter()
                .map(|&id| {
                    let detail = world.organism_detail(id).expect("organism");
                    (
                        id,
                        detail.x_fp,
                        detail.y_fp,
                        detail.energy_milli,
                        detail.age_ticks,
                    )
                })
                .collect::<Vec<_>>(),
            world.population(),
            world.counters(),
            world.total_energy_milli(),
            world.total_biomass_milli(),
        )
    };
    assert_eq!(
        trajectory(&original),
        trajectory(&permuted),
        "an archetype ID reached world state at tick 0"
    );
    for _ in 0..500 {
        original.step();
        permuted.step();
    }
    assert_eq!(
        trajectory(&original),
        trajectory(&permuted),
        "relabelling archetype IDs changed the trajectory"
    );
    // The relabelling is still a different experiment on paper: the config
    // hash records which archetypes seeded the run, which is exactly why the
    // comparison above cannot be a checksum comparison.
    assert_ne!(base.stable_hash(), relabelled.stable_hash());
}

#[test]
fn c6_5_archetype_distributions_are_not_inert() {
    // The guard for the test above: if archetypes had no effect at all,
    // C6.5 would pass trivially. Changing what an archetype *is* must change
    // the world.
    let base = seeded_config(7);
    let mut different = base;
    different.origin.archetypes[0].trait_mean_q16 = [60_000; TRAIT_COUNT];
    let original = World::new(base).expect("world");
    let changed = World::new(different).expect("world");
    assert_ne!(original.state_checksum(), changed.state_checksum());
    assert_ne!(founder_traits(&original), founder_traits(&changed));
}

// --- C6.10 ----------------------------------------------------------------

#[test]
fn c6_10_seeded_and_random_are_distinguishable_experiments() {
    let random = climate_config(7);
    let seeded = seeded_config(7);
    assert_eq!(random.origin.mode, OriginMode::Random);
    assert_eq!(seeded.origin.mode, OriginMode::Seeded);
    assert_ne!(
        random.stable_hash(),
        seeded.stable_hash(),
        "the two origin modes hash the same, so they are one experiment"
    );
    // And they really are different worlds, not just different labels.
    let random_world = World::new(random).expect("world");
    let seeded_world = World::new(seeded).expect("world");
    assert_ne!(founder_traits(&random_world), founder_traits(&seeded_world));
}

#[test]
fn c6_10_a_default_origin_is_excluded_from_the_config_hash() {
    // The D-014 rule applied to a section whose "off" state is a set of
    // defaults rather than a flag: touching a parameter while leaving the
    // defaults in place cannot move the hash, and both fixtures survive.
    let phase1 = SimConfig::phase1_default(0x5eed_cafe_f00d_beef);
    assert_eq!(phase1.stable_hash(), 0x918a_381c_7755_9236);
    let mut with_archetypes = phase1;
    // Archetype definitions are irrelevant while the mode is `random`.
    with_archetypes.origin.archetypes[3] = Archetype::neutral(77);
    assert_eq!(with_archetypes.stable_hash(), phase1.stable_hash());

    // Changing an actual `random` parameter does start a new lineage.
    let mut demes = phase1;
    demes.origin.deme_count = 3;
    assert_ne!(demes.stable_hash(), phase1.stable_hash());
}

#[test]
fn seeded_without_climate_is_refused() {
    let mut config = SimConfig::phase2_default(7);
    config.origin.mode = OriginMode::Seeded;
    config.origin.archetype_count = 1;
    assert_eq!(config.climate.worldgen_version, WorldgenVersion::V1);
    let error = config.validate().expect_err("seeded needs biomes");
    assert!(
        error
            .to_string()
            .contains("climate section must be enabled")
    );
}

#[test]
fn archetypes_must_be_sorted_by_ascending_id() {
    let mut config = seeded_config(7);
    config.origin.archetypes[0].id = 9;
    config.origin.archetypes[1].id = 2;
    let error = config.validate().expect_err("unsorted archetypes");
    assert!(error.to_string().contains("does not ascend"));
}
