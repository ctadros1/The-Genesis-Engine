//! Offline similarity analysis: deterministic threshold clustering over
//! normalized parameter distance.
//!
//! This is an analytical job run outside the hot tick loop. Cluster labels
//! are provisional observations; they never affect entity behavior,
//! pairing, resource access, or survival.

use crate::world::World;

/// Algorithm identifier recorded in every report; bump on any change to
/// sampling, distance, or clustering rules.
pub const SIMILARITY_ALGORITHM_VERSION: &str = "lifesim-similarity-v1";

#[derive(Clone, Debug)]
pub struct SimilarityReport {
    pub algorithm: &'static str,
    pub analysis_tick: u64,
    pub config_hash: u64,
    pub genome_schema_version: u16,
    pub population: usize,
    pub sampled: usize,
    /// Every `sample_stride`-th organism in stable ID order was sampled.
    pub sample_stride: usize,
    pub threshold_q16: u32,
    pub neural_weight_q16: u32,
    pub cluster_count: usize,
    /// Cluster sizes, largest first.
    pub cluster_sizes: Vec<u32>,
    /// (entity ID, cluster label) for each sampled organism, in stable ID
    /// order. Labels are assigned by first appearance in ID order.
    pub labels: Vec<(u64, u32)>,
    /// Mean pairwise normalized distance across the sample (diversity
    /// diagnostic; 0.0 when fewer than two organisms were sampled).
    pub mean_pairwise_distance: f32,
}

/// Run the analysis against a Phase 2 world. Returns `None` when Phase 2 is
/// disabled. Sample size and therefore runtime are bounded by
/// `phase2.cluster_sample_max`.
pub fn analyze(world: &World) -> Option<SimilarityReport> {
    let p2 = world.phase2_state()?;
    let ids = world.organism_ids();
    let config = world.config();
    let population = ids.len();
    let sample_max = config.phase2.cluster_sample_max as usize;
    let sample_stride = if population > sample_max {
        population.div_ceil(sample_max)
    } else {
        1
    };
    let sampled_indices: Vec<usize> = (0..population).step_by(sample_stride.max(1)).collect();
    let sampled = sampled_indices.len();

    let threshold = config.phase2.cluster_threshold_q16 as f32 / 65536.0;
    let neural_weight = config.phase2.cluster_neural_weight_q16;

    // Union-find over the bounded sample.
    let mut parent: Vec<usize> = (0..sampled).collect();
    fn find(parent: &mut [usize], node: usize) -> usize {
        let mut root = node;
        while parent[root] != root {
            root = parent[root];
        }
        let mut walk = node;
        while parent[walk] != root {
            let next = parent[walk];
            parent[walk] = root;
            walk = next;
        }
        root
    }

    let mut distance_sum = 0.0_f64;
    let mut distance_count = 0_u64;
    for left in 0..sampled {
        for right in (left + 1)..sampled {
            let distance = p2.genomes[sampled_indices[left]]
                .normalized_distance(&p2.genomes[sampled_indices[right]], neural_weight);
            distance_sum += f64::from(distance);
            distance_count += 1;
            if distance <= threshold {
                let root_left = find(&mut parent, left);
                let root_right = find(&mut parent, right);
                if root_left != root_right {
                    // Deterministic union: smaller sample index wins.
                    let (low, high) = if root_left < root_right {
                        (root_left, root_right)
                    } else {
                        (root_right, root_left)
                    };
                    parent[high] = low;
                }
            }
        }
    }

    // Assign labels by first appearance in stable ID order.
    let mut label_of_root: Vec<Option<u32>> = vec![None; sampled];
    let mut next_label = 0_u32;
    let mut labels = Vec::with_capacity(sampled);
    let mut sizes: Vec<u32> = Vec::new();
    for (sample_index, &organism_index) in sampled_indices.iter().enumerate() {
        let root = find(&mut parent, sample_index);
        let label = match label_of_root[root] {
            Some(existing) => existing,
            None => {
                let assigned = next_label;
                label_of_root[root] = Some(assigned);
                next_label += 1;
                sizes.push(0);
                assigned
            }
        };
        sizes[label as usize] += 1;
        labels.push((ids[organism_index], label));
    }
    let mut cluster_sizes = sizes.clone();
    cluster_sizes.sort_unstable_by(|a, b| b.cmp(a));

    Some(SimilarityReport {
        algorithm: SIMILARITY_ALGORITHM_VERSION,
        analysis_tick: world.tick_number(),
        config_hash: world.config_hash(),
        genome_schema_version: crate::genome::GENOME_SCHEMA_VERSION,
        population,
        sampled,
        sample_stride,
        threshold_q16: config.phase2.cluster_threshold_q16,
        neural_weight_q16: neural_weight,
        cluster_count: cluster_sizes.len(),
        cluster_sizes,
        labels,
        mean_pairwise_distance: if distance_count == 0 {
            0.0
        } else {
            (distance_sum / distance_count as f64) as f32
        },
    })
}
