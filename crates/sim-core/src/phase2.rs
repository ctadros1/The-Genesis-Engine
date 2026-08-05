//! Phase 2 per-organism state: genomes, phenotypes, controller memory,
//! heading/speed, ancestry, intents, and Phase 2 counters.
//!
//! This state exists only when `phase2.enabled` is true. A disabled world
//! carries `None` and executes the exact Phase 1 code paths, so Phase 1
//! fixtures and checksums are preserved bit for bit.

use crate::checksum::Fnv1a64;
use crate::genome::{Genome, MEMORY_VALUES, Phenotype, VariationSummary, hash_genome_into};

/// Maximum sensor range in meters for genome schema 1; bucket sizing must
/// cover it so a 3x3 bucket ring always contains every sensable neighbor.
pub const SENSOR_RANGE_MAX_M: u32 = 12;

/// One queued paired-parent child, produced in the apply phase and
/// materialized in the lifecycle phase.
#[derive(Clone, Debug)]
pub(crate) struct PendingChild {
    pub parent_a: u64,
    pub parent_b: u64,
    /// The flat schema-1 genome, `Some` exactly in a schema-1 world.
    pub genome: Option<Genome>,
    /// The diploid schema-2 genome, `Some` exactly in a schema-2 world.
    /// Exactly one of the two is present: a world is one schema or the
    /// other, and neither carries a placeholder for the one it is not.
    pub genome2: Option<crate::genome2::Genome2>,
    pub genome_hash: u64,
    pub phenotype: Phenotype,
    pub x_fp: i32,
    pub y_fp: i32,
    pub heading_bam: u16,
    pub energy_milli: i64,
    pub invest_a_milli: i64,
    pub invest_b_milli: i64,
    pub depth: u32,
    pub variation: VariationSummary,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PairRejectReason {
    Capacity,
    Placement,
    Energy,
    /// The schema-2 child genome is not structurally viable.
    ///
    /// A real genetic outcome rather than an error. Crossover cuts a
    /// haplotype at an arbitrary point, so a gamete can carry an edge whose
    /// source or target node stayed behind on the other side; the zygote
    /// then has a dangling reference and no network. Recombination in real
    /// organisms produces inviable products for structurally similar
    /// reasons, and the honest response is to refuse the child and count it,
    /// not to repair the genome into something neither parent could have
    /// produced.
    Nonviable,
}

impl PairRejectReason {
    pub fn name(self) -> &'static str {
        match self {
            PairRejectReason::Capacity => "capacity",
            PairRejectReason::Placement => "placement",
            PairRejectReason::Energy => "energy",
            PairRejectReason::Nonviable => "nonviable",
        }
    }
}

/// Deterministic Phase 2 counters; hashed into the state checksum.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Phase2Counters {
    pub paired_births_total: u64,
    pub pair_rejected_capacity_total: u64,
    pub pair_rejected_placement_total: u64,
    pub pair_rejected_energy_total: u64,
    /// Schema-2 children whose merged network would not compile, refused at
    /// admission. Should stay zero: `validate_structure` rejects the only
    /// known cause (a zero-delay cycle formed by merging two acyclic
    /// haplotypes) before the genome ever reaches here. Counted anyway,
    /// because "should stay zero" is a claim a campaign has to be able to
    /// check rather than assume.
    pub pair_rejected_nonviable_total: u64,
    pub controller_faults_total: u64,
    pub mutated_trait_genes_total: u64,
    pub mutated_neural_genes_total: u64,
}

/// Parallel per-organism Phase 2 arrays. Kept in lockstep with the world's
/// primary SoA arrays: births append, removal compacts in the same order.
#[derive(Clone, Debug)]
pub(crate) struct Phase2State {
    pub genomes: Vec<Genome>,
    pub genome_hashes: Vec<u64>,
    pub phenotypes: Vec<Phenotype>,
    pub memory: Vec<[f32; MEMORY_VALUES]>,
    pub heading_bam: Vec<u16>,
    pub speed_milli: Vec<i64>,
    pub last_turn: Vec<f32>,

    // Ancestry (bounded live state; events are the authoritative history).
    pub parents: Vec<[u64; 2]>,
    pub depth: Vec<u32>,
    pub child_count: Vec<u32>,
    pub birth_tick: Vec<u64>,

    // Per-tick buffers (not logical state; rebuilt every tick).
    pub inputs: Vec<[f32; crate::genome::CONTROLLER_INPUTS]>,
    pub intent_turn: Vec<f32>,
    pub intent_speed_milli: Vec<i64>,
    pub intent_eat: Vec<bool>,
    pub intent_mate: Vec<bool>,
    pub next_memory: Vec<[f32; MEMORY_VALUES]>,
    pub pending: Vec<PendingChild>,

    pub counters: Phase2Counters,
}

impl Phase2State {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            genomes: Vec::with_capacity(capacity),
            genome_hashes: Vec::with_capacity(capacity),
            phenotypes: Vec::with_capacity(capacity),
            memory: Vec::with_capacity(capacity),
            heading_bam: Vec::with_capacity(capacity),
            speed_milli: Vec::with_capacity(capacity),
            last_turn: Vec::with_capacity(capacity),
            parents: Vec::with_capacity(capacity),
            depth: Vec::with_capacity(capacity),
            child_count: Vec::with_capacity(capacity),
            birth_tick: Vec::with_capacity(capacity),
            inputs: Vec::new(),
            intent_turn: Vec::new(),
            intent_speed_milli: Vec::new(),
            intent_eat: Vec::new(),
            intent_mate: Vec::new(),
            next_memory: Vec::new(),
            pending: Vec::new(),
            counters: Phase2Counters::default(),
        }
    }

    /// Append one organism's Phase 2 state.
    #[allow(clippy::too_many_arguments)]
    pub fn push_organism(
        &mut self,
        genome: Option<Genome>,
        genome_hash: u64,
        phenotype: Phenotype,
        heading_bam: u16,
        parents: [u64; 2],
        depth: u32,
        birth_tick: u64,
    ) {
        // A schema-2 world keeps the flat *genome* array empty; pushing a
        // placeholder would double the memory and invite code to read a
        // genome the organism does not have. The **hash** is kept for both
        // schemas, because "which genome does this organism have" is a
        // question every schema can answer and the observer asks it.
        if let Some(genome) = genome {
            self.genomes.push(genome);
        }
        self.genome_hashes.push(genome_hash);
        self.phenotypes.push(phenotype);
        self.memory.push([0.0; MEMORY_VALUES]);
        self.heading_bam.push(heading_bam);
        self.speed_milli.push(0);
        self.last_turn.push(0.0);
        self.parents.push(parents);
        self.depth.push(depth);
        self.child_count.push(0);
        self.birth_tick.push(birth_tick);
    }

    /// Compact all logical arrays with the same removal flags the world
    /// applies to its primary arrays.
    pub fn retain(&mut self, remove: &[bool]) {
        retain_by_flags(&mut self.genomes, remove);
        retain_copy_by_flags(&mut self.genome_hashes, remove);
        retain_copy_by_flags(&mut self.phenotypes, remove);
        retain_copy_by_flags(&mut self.memory, remove);
        retain_copy_by_flags(&mut self.heading_bam, remove);
        retain_copy_by_flags(&mut self.speed_milli, remove);
        retain_copy_by_flags(&mut self.last_turn, remove);
        retain_copy_by_flags(&mut self.parents, remove);
        retain_copy_by_flags(&mut self.depth, remove);
        retain_copy_by_flags(&mut self.child_count, remove);
        retain_copy_by_flags(&mut self.birth_tick, remove);
    }

    /// Living organisms this state covers.
    ///
    /// Counted from `phenotypes` rather than `genomes` because a schema-2
    /// world keeps the flat genome arrays empty - its genome lives in the
    /// schema-2 section - while every organism still has a phenotype
    /// whichever schema expressed it.
    pub fn len(&self) -> usize {
        self.phenotypes.len()
    }

    /// Maximum ancestry depth among living organisms (generation proxy).
    pub fn max_depth(&self) -> u32 {
        self.depth.iter().copied().max().unwrap_or(0)
    }

    /// Hash all logical Phase 2 state (never the per-tick buffers).
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-phase2-state-v1");
        for index in 0..self.len() {
            // A schema-2 world has no flat genome to hash here; its genomes
            // enter the checksum through the schema-2 section instead.
            if let Some(genome) = self.genomes.get(index) {
                hash_genome_into(hasher, genome);
            }
            hasher.update_u64(self.genome_hashes[index]);
            hasher.update_u32(self.heading_bam[index] as u32);
            hasher.update_i64(self.speed_milli[index]);
            hasher.update_u32(self.last_turn[index].to_bits());
            for &value in &self.memory[index] {
                hasher.update_u32(value.to_bits());
            }
            hasher.update_u64(self.parents[index][0]);
            hasher.update_u64(self.parents[index][1]);
            hasher.update_u32(self.depth[index]);
            hasher.update_u32(self.child_count[index]);
            hasher.update_u64(self.birth_tick[index]);
        }
        hasher.update_u64(self.counters.paired_births_total);
        hasher.update_u64(self.counters.pair_rejected_capacity_total);
        hasher.update_u64(self.counters.pair_rejected_placement_total);
        hasher.update_u64(self.counters.pair_rejected_energy_total);
        hasher.update_u64(self.counters.controller_faults_total);
        hasher.update_u64(self.counters.mutated_trait_genes_total);
        hasher.update_u64(self.counters.mutated_neural_genes_total);
    }
}

fn retain_by_flags<T>(values: &mut Vec<T>, remove: &[bool]) {
    let mut index = 0_usize;
    values.retain(|_| {
        let keep = !remove[index];
        index += 1;
        keep
    });
}

fn retain_copy_by_flags<T: Copy>(values: &mut Vec<T>, remove: &[bool]) {
    let mut write = 0_usize;
    for read in 0..values.len() {
        if !remove[read] {
            values[write] = values[read];
            write += 1;
        }
    }
    values.truncate(write);
}
