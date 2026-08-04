//! Offline similarity-analysis tests: determinism, bounds, and neutrality.

use sim_core::{SIMILARITY_ALGORITHM_VERSION, SimConfig, World, analyze};

const SEED: u64 = 0x5eed_cafe_f00d_beef;

fn phase2_config() -> SimConfig {
    let mut config = SimConfig::phase2_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 100;
    config.max_entities = 1_000;
    config
}

#[test]
fn analysis_is_deterministic_and_bounded() {
    let mut world = World::new(phase2_config()).unwrap();
    for _ in 0..500 {
        world.step();
    }
    let first = analyze(&world).unwrap();
    let second = analyze(&world).unwrap();
    assert_eq!(first.algorithm, SIMILARITY_ALGORITHM_VERSION);
    assert_eq!(first.labels, second.labels);
    assert_eq!(first.cluster_sizes, second.cluster_sizes);
    assert_eq!(first.mean_pairwise_distance, second.mean_pairwise_distance);
    assert_eq!(first.population, world.population());
    assert_eq!(first.sampled, first.labels.len());
    let total: u32 = first.cluster_sizes.iter().sum();
    assert_eq!(total as usize, first.sampled);
    assert!((0.0..=1.0).contains(&first.mean_pairwise_distance));
    // Labels are compact and assigned by first appearance in ID order.
    for (index, &(id, label)) in first.labels.iter().enumerate() {
        assert!(label < first.cluster_count as u32);
        if index > 0 {
            assert!(id > first.labels[index - 1].0);
        }
    }
}

#[test]
fn sampling_respects_the_configured_bound() {
    let mut config = phase2_config();
    config.phase2.cluster_sample_max = 16;
    let mut world = World::new(config).unwrap();
    for _ in 0..100 {
        world.step();
    }
    let report = analyze(&world).unwrap();
    assert!(report.sampled <= 16);
    assert!(report.sample_stride >= world.population().div_ceil(16));
}

#[test]
fn threshold_extremes_produce_expected_cluster_structure() {
    let mut world = World::new(phase2_config()).unwrap();
    for _ in 0..200 {
        world.step();
    }
    // A threshold of 1.0 merges every founder into one cluster.
    let mut merged_config = phase2_config();
    merged_config.phase2.cluster_threshold_q16 = 65_536;
    // Rebuild deterministically with the changed analysis config: the
    // world trajectory differs (new config hash), but the structural
    // property still holds for any population.
    let mut merged_world = World::new(merged_config).unwrap();
    for _ in 0..200 {
        merged_world.step();
    }
    let merged = analyze(&merged_world).unwrap();
    assert_eq!(merged.cluster_count, 1);

    // A threshold of 0 puts every distinct genome in its own cluster.
    let mut split_config = phase2_config();
    split_config.phase2.cluster_threshold_q16 = 0;
    let mut split_world = World::new(split_config).unwrap();
    for _ in 0..200 {
        split_world.step();
    }
    let split = analyze(&split_world).unwrap();
    assert_eq!(split.cluster_count, split.sampled);

    // The default threshold is somewhere between the extremes.
    let default = analyze(&world).unwrap();
    assert!(default.cluster_count >= 1);
    assert!(default.cluster_count <= default.sampled);
}

#[test]
fn analysis_returns_none_for_phase1_worlds() {
    let mut config = SimConfig::phase1_default(SEED);
    config.cells_x = 64;
    config.cells_y = 64;
    config.initial_organisms = 50;
    config.max_entities = 500;
    let world = World::new(config).unwrap();
    assert!(analyze(&world).is_none());
}
