//! Versioned organism parameter record (genome) for Phase 2.
//!
//! A genome is 14 normalized trait genes plus the fixed-topology controller
//! parameters (weights and biases). Every value is a finite, bounded f32.
//! Decoding is fail-closed: malformed bytes produce a typed error and never
//! a partially repaired record. All randomness used for founder creation,
//! recombination, and bounded variation flows through named streams.

use crate::checksum::{Fnv1a64, fnv1a64};
use crate::rng::{RngSystem, named_random};
use std::fmt;

/// Bumped whenever gene meaning, count, bounds, or encoding changes.
pub const GENOME_SCHEMA_VERSION: u16 = 1;

/// The only controller topology registered for schema 1.
pub const TOPOLOGY_ID: u16 = 1;

/// Version string folded into the config hash when Phase 2 is enabled.
pub const GENOME_POLICY_VERSION: &str = "lifesim-genome-v1";

/// Topology 1 dimensions (see `specifications/neural-network-schema.md`).
pub const CONTROLLER_INPUTS: usize = 20;
pub const HIDDEN_1: usize = 16;
pub const HIDDEN_2: usize = 12;
pub const CONTROLLER_OUTPUTS: usize = 12;
pub const MEMORY_VALUES: usize = 4;

/// Layer-major neural gene count: weights then biases per layer.
pub const NEURAL_COUNT: usize = (CONTROLLER_INPUTS * HIDDEN_1 + HIDDEN_1)
    + (HIDDEN_1 * HIDDEN_2 + HIDDEN_2)
    + (HIDDEN_2 * CONTROLLER_OUTPUTS + CONTROLLER_OUTPUTS);

/// Trait gene indices. Genes are normalized f32 in [0, 1]; phenotype
/// mappings live in `Phenotype::derive`.
pub const TRAIT_COUNT: usize = 14;
pub const GENE_PIGMENT_HUE: usize = 0;
pub const GENE_PIGMENT_PATTERN: usize = 1;
pub const GENE_BODY_SCALE: usize = 2;
pub const GENE_SPEED_POTENTIAL: usize = 3;
pub const GENE_SENSOR_RANGE: usize = 4;
pub const GENE_SENSOR_SENSITIVITY: usize = 5;
pub const GENE_METABOLISM: usize = 6;
pub const GENE_THERMAL_PREFERENCE: usize = 7;
pub const GENE_DIET_AFFINITY: usize = 8;
pub const GENE_APPROACH_TENDENCY: usize = 9;
pub const GENE_DEFENSE_TENDENCY: usize = 10;
pub const GENE_MATURITY: usize = 11;
pub const GENE_REPRO_INVESTMENT: usize = 12;
pub const GENE_REPRO_COOLDOWN: usize = 13;

/// Neural weights and biases are clamped to this symmetric range.
pub const WEIGHT_LIMIT: f32 = 8.0;

const GENOME_MAGIC: &[u8; 4] = b"ALGN";
const HEADER_LEN: usize = 4 + 2 + 2 + 2 + 4;
const CHECKSUM_LEN: usize = 8;
/// Exact encoded length for schema 1; decoders verify before any allocation.
pub const ENCODED_LEN: usize = HEADER_LEN + (TRAIT_COUNT + NEURAL_COUNT) * 4 + CHECKSUM_LEN;

/// A validated parameter record. Construction always passes validation, so
/// a `Genome` in world state is valid by construction.
#[derive(Clone, Debug, PartialEq)]
pub struct Genome {
    traits: [f32; TRAIT_COUNT],
    neural: Vec<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GenomeError {
    TooShort { actual: usize },
    LengthMismatch { expected: usize, actual: usize },
    BadMagic,
    UnknownSchema(u16),
    UnknownTopology(u16),
    WrongTraitCount(u16),
    WrongNeuralCount(u32),
    ChecksumMismatch,
    NonFiniteTrait { index: usize },
    TraitOutOfRange { index: usize },
    NonFiniteNeural { index: usize },
    NeuralOutOfRange { index: usize },
}

impl fmt::Display for GenomeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for GenomeError {}

impl Genome {
    /// Validate and construct. The only path into a `Genome`.
    pub fn validated(traits: [f32; TRAIT_COUNT], neural: Vec<f32>) -> Result<Self, GenomeError> {
        if neural.len() != NEURAL_COUNT {
            return Err(GenomeError::WrongNeuralCount(neural.len() as u32));
        }
        for (index, &gene) in traits.iter().enumerate() {
            if !gene.is_finite() {
                return Err(GenomeError::NonFiniteTrait { index });
            }
            if !(0.0..=1.0).contains(&gene) {
                return Err(GenomeError::TraitOutOfRange { index });
            }
        }
        for (index, &gene) in neural.iter().enumerate() {
            if !gene.is_finite() {
                return Err(GenomeError::NonFiniteNeural { index });
            }
            if gene.abs() > WEIGHT_LIMIT {
                return Err(GenomeError::NeuralOutOfRange { index });
            }
        }
        Ok(Self { traits, neural })
    }

    pub fn traits(&self) -> &[f32; TRAIT_COUNT] {
        &self.traits
    }

    pub fn neural(&self) -> &[f32] {
        &self.neural
    }

    /// Canonical encoded bytes: header, trait genes, neural genes, checksum.
    /// Little-endian f32 bit patterns; used for both the codec and the hash.
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(ENCODED_LEN);
        bytes.extend_from_slice(GENOME_MAGIC);
        bytes.extend_from_slice(&GENOME_SCHEMA_VERSION.to_le_bytes());
        bytes.extend_from_slice(&TOPOLOGY_ID.to_le_bytes());
        bytes.extend_from_slice(&(TRAIT_COUNT as u16).to_le_bytes());
        bytes.extend_from_slice(&(NEURAL_COUNT as u32).to_le_bytes());
        for &gene in &self.traits {
            bytes.extend_from_slice(&gene.to_le_bytes());
        }
        for &gene in &self.neural {
            bytes.extend_from_slice(&gene.to_le_bytes());
        }
        let checksum = fnv1a64(&bytes);
        bytes.extend_from_slice(&checksum.to_le_bytes());
        debug_assert_eq!(bytes.len(), ENCODED_LEN);
        bytes
    }

    /// Fail-closed bounded decode. Length and header fields are verified
    /// before any allocation; values are validated before construction.
    pub fn decode(bytes: &[u8]) -> Result<Self, GenomeError> {
        if bytes.len() < HEADER_LEN {
            return Err(GenomeError::TooShort {
                actual: bytes.len(),
            });
        }
        if &bytes[0..4] != GENOME_MAGIC {
            return Err(GenomeError::BadMagic);
        }
        let schema = u16::from_le_bytes([bytes[4], bytes[5]]);
        if schema != GENOME_SCHEMA_VERSION {
            return Err(GenomeError::UnknownSchema(schema));
        }
        let topology = u16::from_le_bytes([bytes[6], bytes[7]]);
        if topology != TOPOLOGY_ID {
            return Err(GenomeError::UnknownTopology(topology));
        }
        let trait_count = u16::from_le_bytes([bytes[8], bytes[9]]);
        if usize::from(trait_count) != TRAIT_COUNT {
            return Err(GenomeError::WrongTraitCount(trait_count));
        }
        let neural_count = u32::from_le_bytes([bytes[10], bytes[11], bytes[12], bytes[13]]);
        if neural_count as usize != NEURAL_COUNT {
            return Err(GenomeError::WrongNeuralCount(neural_count));
        }
        if bytes.len() != ENCODED_LEN {
            return Err(GenomeError::LengthMismatch {
                expected: ENCODED_LEN,
                actual: bytes.len(),
            });
        }
        let body_end = ENCODED_LEN - CHECKSUM_LEN;
        let declared = u64::from_le_bytes(bytes[body_end..].try_into().expect("eight bytes"));
        if declared != fnv1a64(&bytes[..body_end]) {
            return Err(GenomeError::ChecksumMismatch);
        }
        let mut traits = [0.0_f32; TRAIT_COUNT];
        for (index, chunk) in bytes[HEADER_LEN..HEADER_LEN + TRAIT_COUNT * 4]
            .chunks_exact(4)
            .enumerate()
        {
            traits[index] = f32::from_le_bytes(chunk.try_into().expect("four bytes"));
        }
        let mut neural = Vec::with_capacity(NEURAL_COUNT);
        for chunk in bytes[HEADER_LEN + TRAIT_COUNT * 4..body_end].chunks_exact(4) {
            neural.push(f32::from_le_bytes(chunk.try_into().expect("four bytes")));
        }
        Self::validated(traits, neural)
    }

    /// Canonical genome hash over the encoded bytes without the trailing
    /// checksum (schema and topology are inside the hashed header).
    pub fn stable_hash(&self) -> u64 {
        let bytes = self.encode();
        fnv1a64(&bytes[..bytes.len() - CHECKSUM_LEN])
    }

    /// Deterministic founder genome for an initial-population slot. Traits
    /// start in the middle half of their range; controller parameters start
    /// small so founder behavior is mild rather than saturated.
    pub fn founder(world_seed: u64, entity_id: u64) -> Self {
        let mut traits = [0.0_f32; TRAIT_COUNT];
        for (index, gene) in traits.iter_mut().enumerate() {
            let draw = named_random(
                world_seed,
                0,
                RngSystem::GenomeInit,
                entity_id,
                index as u32,
            );
            *gene = 0.25 + unit_f32(draw) * 0.5;
        }
        let mut neural = Vec::with_capacity(NEURAL_COUNT);
        for index in 0..NEURAL_COUNT {
            let draw = named_random(
                world_seed,
                0,
                RngSystem::GenomeInit,
                entity_id,
                (TRAIT_COUNT + index) as u32,
            );
            neural.push((unit_f32(draw) - 0.5) * 1.0);
        }
        Self::validated(traits, neural).expect("founder genome is valid by construction")
    }

    /// Normalized trait distance in [0, 1]: mean absolute gene difference.
    /// Optionally mixes a normalized neural distance with Q16 weight
    /// `neural_weight_q16` (0 omits controller parameters entirely).
    pub fn normalized_distance(&self, other: &Self, neural_weight_q16: u32) -> f32 {
        let mut trait_sum = 0.0_f32;
        for index in 0..TRAIT_COUNT {
            trait_sum += (self.traits[index] - other.traits[index]).abs();
        }
        let trait_distance = trait_sum / TRAIT_COUNT as f32;
        if neural_weight_q16 == 0 {
            return trait_distance;
        }
        let mut neural_sum = 0.0_f32;
        for index in 0..NEURAL_COUNT {
            neural_sum += (self.neural[index] - other.neural[index]).abs();
        }
        let neural_distance = neural_sum / (NEURAL_COUNT as f32 * 2.0 * WEIGHT_LIMIT);
        let weight = neural_weight_q16 as f32 / 65536.0;
        trait_distance * (1.0 - weight) + neural_distance * weight
    }
}

/// Summary of the bounded variation applied to one child genome. Recorded
/// in the paired-birth event for auditability.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VariationSummary {
    pub mutated_trait_genes: u32,
    pub mutated_neural_genes: u32,
}

/// Bounded variation policy parameters (from the Phase 2 config section).
#[derive(Clone, Copy, Debug)]
pub struct VariationPolicy {
    /// Per-gene variation probability, Q16.
    pub probability_q16: u32,
    /// Maximum absolute trait-gene delta (gene units), Q16.
    pub trait_sigma_q16: u32,
    /// Maximum absolute neural-gene delta as a Q16 fraction of WEIGHT_LIMIT.
    pub neural_sigma_q16: u32,
}

/// Deterministic two-source recombination plus bounded uniform variation
/// (`uniform-bounded-v1` distribution policy). Draws are keyed by the
/// prospective child entity ID so unrelated draws cannot shift outcomes.
/// Always returns a validated genome.
pub fn recombine(
    parent_a: &Genome,
    parent_b: &Genome,
    policy: VariationPolicy,
    world_seed: u64,
    tick: u64,
    child_id: u64,
) -> (Genome, VariationSummary) {
    let mut summary = VariationSummary::default();
    let mut traits = [0.0_f32; TRAIT_COUNT];
    let mut draw_index = 0_u32;
    let next_draw = |index: &mut u32| -> u64 {
        let value = named_random(world_seed, tick, RngSystem::Recombination, child_id, *index);
        *index += 1;
        value
    };

    for (gene, slot) in traits.iter_mut().enumerate() {
        let pick = next_draw(&mut draw_index);
        let base = if pick & 1 == 0 {
            parent_a.traits[gene]
        } else {
            parent_b.traits[gene]
        };
        let mutate = next_draw(&mut draw_index);
        *slot = if (mutate & 0xffff) < u64::from(policy.probability_q16) {
            summary.mutated_trait_genes += 1;
            let magnitude = next_draw(&mut draw_index);
            let sigma = policy.trait_sigma_q16 as f32 / 65536.0;
            let delta = (unit_f32(magnitude) * 2.0 - 1.0) * sigma;
            (base + delta).clamp(0.0, 1.0)
        } else {
            base
        };
    }

    let mut neural = Vec::with_capacity(NEURAL_COUNT);
    for gene in 0..NEURAL_COUNT {
        let pick = next_draw(&mut draw_index);
        let base = if pick & 1 == 0 {
            parent_a.neural[gene]
        } else {
            parent_b.neural[gene]
        };
        let mutate = next_draw(&mut draw_index);
        let value = if (mutate & 0xffff) < u64::from(policy.probability_q16) {
            summary.mutated_neural_genes += 1;
            let magnitude = next_draw(&mut draw_index);
            let sigma = (policy.neural_sigma_q16 as f32 / 65536.0) * WEIGHT_LIMIT;
            let delta = (unit_f32(magnitude) * 2.0 - 1.0) * sigma;
            (base + delta).clamp(-WEIGHT_LIMIT, WEIGHT_LIMIT)
        } else {
            base
        };
        neural.push(value);
    }

    let genome =
        Genome::validated(traits, neural).expect("recombination preserves bounds by construction");
    (genome, summary)
}

/// Map a 64-bit draw to f32 in [0, 1). Uses the top 24 bits so the mapping
/// is exact in f32.
fn unit_f32(draw: u64) -> f32 {
    ((draw >> 40) as f32) / 16_777_216.0
}

/// Derived fixed-point runtime attributes. Every mapping is linear from a
/// normalized gene to a documented closed range; rounding is deterministic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Phenotype {
    /// Body scale multiplier, milli (600..=1600).
    pub body_scale_milli: i64,
    /// Maximum speed, milli-meters per second (500..=3000).
    pub max_speed_milli: i64,
    /// Sensor range, milli-meters (4000..=12000).
    pub sensor_range_milli: i64,
    /// Sensor sensitivity multiplier, milli (500..=1500).
    pub sensor_sensitivity_milli: i64,
    /// Basal metabolic cost multiplier, milli (600..=1600).
    pub basal_mult_milli: i64,
    /// Intake-rate multiplier, milli (800..=1200).
    pub intake_mult_milli: i64,
    /// Thermal preference, milli (0..=1000). Recorded for analysis only in
    /// Phase 2; no temperature field exists yet.
    pub thermal_pref_milli: i64,
    /// Approach/follow tendency multiplier, milli (0..=1000).
    pub approach_milli: i64,
    /// Defense tendency, milli (0..=1000). Analysis-only in Phase 2.
    pub defense_milli: i64,
    /// Maturity age, ticks (400..=1200).
    pub maturity_ticks: u64,
    /// Per-parent offspring energy investment basis, milli-EU (3000..=6000).
    pub invest_milli: i64,
    /// Reproduction cooldown, ticks (200..=600).
    pub cooldown_ticks: u64,
}

impl Phenotype {
    pub fn derive(genome: &Genome) -> Self {
        Self::from_traits(genome.traits())
    }

    /// Derive a phenotype from bare trait values.
    ///
    /// Schema 2 expresses its traits diploidly and has no `Genome` to hand,
    /// but the mapping from trait to phenotype is the same one and must stay
    /// the same one: a second copy would let the two schemas drift apart in
    /// a way that would look like an effect of structural freedom.
    pub fn from_traits(traits: &[f32; TRAIT_COUNT]) -> Self {
        Self {
            body_scale_milli: lerp_milli(traits[GENE_BODY_SCALE], 600, 1600),
            max_speed_milli: lerp_milli(traits[GENE_SPEED_POTENTIAL], 500, 3000),
            sensor_range_milli: lerp_milli(traits[GENE_SENSOR_RANGE], 4000, 12000),
            sensor_sensitivity_milli: lerp_milli(traits[GENE_SENSOR_SENSITIVITY], 500, 1500),
            basal_mult_milli: lerp_milli(traits[GENE_METABOLISM], 600, 1600),
            intake_mult_milli: lerp_milli(traits[GENE_DIET_AFFINITY], 800, 1200),
            thermal_pref_milli: lerp_milli(traits[GENE_THERMAL_PREFERENCE], 0, 1000),
            approach_milli: lerp_milli(traits[GENE_APPROACH_TENDENCY], 0, 1000),
            defense_milli: lerp_milli(traits[GENE_DEFENSE_TENDENCY], 0, 1000),
            maturity_ticks: lerp_milli(traits[GENE_MATURITY], 400, 1200) as u64,
            invest_milli: lerp_milli(traits[GENE_REPRO_INVESTMENT], 3000, 6000),
            cooldown_ticks: lerp_milli(traits[GENE_REPRO_COOLDOWN], 200, 600) as u64,
        }
    }

    /// Phase 10: the retired trait genes replaced by their grown body.
    ///
    /// **Three traits are retired and only three.** Body scale, speed
    /// potential, and sensor range are now consequences of the module set,
    /// per `specifications/morphology-and-development.md`. Everything else -
    /// metabolism, diet affinity, thermal preference, approach and defense
    /// tendency, maturity, investment, cooldown - stays a trait gene,
    /// because none of those is a fact about shape.
    ///
    /// The retired trait IDs are **never reused**. They keep their slots in
    /// the trait block, keep being inherited, and stop being read. That is
    /// the same treatment `PlasticityGenes` gets before Phase 11, and it is
    /// what makes turning morphology off a config flip rather than a schema
    /// change.
    ///
    /// **Every quantity is a ratio to the founder body**, so a founder lands
    /// mid-range on all of them and any deviation is a genuine consequence
    /// of a different body. The first version of this function mapped module
    /// sums onto the clamps directly, using whatever constant looked
    /// dimensionally plausible, and produced a founder pinned at *three*
    /// clamp extremes at once - maximum basal cost, minimum speed, minimum
    /// sensing. Enabling morphology was therefore a systematic handicap
    /// rather than a change of representation, and 29 of 30 campaign worlds
    /// went extinct. A derived phenotype needs its reference point
    /// calibrated to the founder or the derivation is a tax.
    pub fn from_body(
        traits: &[f32; TRAIT_COUNT],
        derived: &crate::morphology::DerivedBody,
        reference: &crate::morphology::BodyReference,
    ) -> Self {
        let mut phenotype = Self::from_traits(traits);
        phenotype.apply_body(derived, reference);
        phenotype
    }

    /// Overwrite the body-derived fields from a derived body, leaving every
    /// trait-derived field untouched. `from_body` routes through this at
    /// birth, and Phase 14 ontogeny calls it again on each module
    /// activation with the grown prefix's derivation - one code path, so a
    /// juvenile's phenotype and a newborn's are computed by the same
    /// arithmetic.
    pub fn apply_body(
        &mut self,
        derived: &crate::morphology::DerivedBody,
        reference: &crate::morphology::BodyReference,
    ) {
        self.body_scale_milli =
            (derived.mass_milli * 1_000 / reference.mass_milli).clamp(600, 1_600);
        self.basal_mult_milli =
            (derived.basal_cost_milli * 1_000 / reference.upkeep_milli).clamp(600, 1_600);
        self.intake_mult_milli =
            (derived.intake_milli * 1_000 / reference.intake_milli).clamp(800, 1_200);
        // Speed is thrust per unit mass relative to the founder's, centred so
        // a founder sits mid-range and can move either way from there. A body
        // with no motor still has zero thrust and lands on the floor, which
        // is correct: it cannot move.
        let ratio = derived.thrust_milli * 1_000 / derived.mass_milli.max(1);
        self.max_speed_milli = (1_500 * ratio / reference.thrust_ratio_milli).clamp(500, 3_000);
        // Sensing comes from the best sensory module. A body with none is
        // blind, which is a real morphology and lands on the floor.
        self.sensor_range_milli = if derived.sensory_modules == 0 {
            4_000
        } else {
            derived.sensor_range_milli.clamp(4_000, 12_000)
        };
    }
}

fn lerp_milli(gene: f32, min: i64, max: i64) -> i64 {
    let clamped = gene.clamp(0.0, 1.0);
    let value = min as f32 + clamped * (max - min) as f32;
    // Truncation after +0.5 gives deterministic round-half-up for the
    // non-negative ranges used here.
    (value + 0.5) as i64
}

/// Hash a genome's canonical identity into an existing state hasher.
pub fn hash_genome_into(hasher: &mut Fnv1a64, genome: &Genome) {
    for &gene in genome.traits() {
        hasher.update_u32(gene.to_bits());
    }
    for &gene in genome.neural() {
        hasher.update_u32(gene.to_bits());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_valid() -> Genome {
        Genome::validated([0.5; TRAIT_COUNT], vec![0.0; NEURAL_COUNT]).unwrap()
    }

    #[test]
    fn neural_count_matches_topology_math() {
        assert_eq!(NEURAL_COUNT, 320 + 16 + 192 + 12 + 144 + 12);
        assert_eq!(ENCODED_LEN, 14 + (14 + 696) * 4 + 8);
    }

    #[test]
    fn round_trip_and_canonical_hash() {
        let genome = Genome::founder(0x5eed, 42);
        let encoded = genome.encode();
        assert_eq!(encoded.len(), ENCODED_LEN);
        let decoded = Genome::decode(&encoded).unwrap();
        assert_eq!(decoded, genome);
        assert_eq!(decoded.stable_hash(), genome.stable_hash());
        assert_ne!(
            genome.stable_hash(),
            Genome::founder(0x5eed, 43).stable_hash()
        );
    }

    #[test]
    fn malformed_records_fail_closed() {
        let valid = minimal_valid().encode();

        assert!(matches!(
            Genome::decode(&valid[..8]),
            Err(GenomeError::TooShort { .. })
        ));

        let mut bad_magic = valid.clone();
        bad_magic[0] = b'X';
        assert_eq!(Genome::decode(&bad_magic), Err(GenomeError::BadMagic));

        let mut bad_schema = valid.clone();
        bad_schema[4..6].copy_from_slice(&9_u16.to_le_bytes());
        assert_eq!(
            Genome::decode(&bad_schema),
            Err(GenomeError::UnknownSchema(9))
        );

        let mut bad_topology = valid.clone();
        bad_topology[6..8].copy_from_slice(&7_u16.to_le_bytes());
        assert_eq!(
            Genome::decode(&bad_topology),
            Err(GenomeError::UnknownTopology(7))
        );

        let mut bad_trait_count = valid.clone();
        bad_trait_count[8..10].copy_from_slice(&13_u16.to_le_bytes());
        assert_eq!(
            Genome::decode(&bad_trait_count),
            Err(GenomeError::WrongTraitCount(13))
        );

        // A huge declared neural count must be rejected before allocation.
        let mut huge = valid.clone();
        huge[10..14].copy_from_slice(&u32::MAX.to_le_bytes());
        assert_eq!(
            Genome::decode(&huge),
            Err(GenomeError::WrongNeuralCount(u32::MAX))
        );

        let truncated = &valid[..valid.len() - 1];
        assert!(matches!(
            Genome::decode(truncated),
            Err(GenomeError::LengthMismatch { .. })
        ));

        let mut bad_checksum = valid.clone();
        let last = bad_checksum.len() - 1;
        bad_checksum[last] ^= 0xff;
        assert_eq!(
            Genome::decode(&bad_checksum),
            Err(GenomeError::ChecksumMismatch)
        );

        // Non-finite trait: patch a NaN into the first trait, refresh the
        // checksum so only value validation can reject it.
        let mut nan_trait = valid.clone();
        nan_trait[HEADER_LEN..HEADER_LEN + 4].copy_from_slice(&f32::NAN.to_le_bytes());
        let body_end = ENCODED_LEN - CHECKSUM_LEN;
        let checksum = fnv1a64(&nan_trait[..body_end]);
        nan_trait[body_end..].copy_from_slice(&checksum.to_le_bytes());
        assert_eq!(
            Genome::decode(&nan_trait),
            Err(GenomeError::NonFiniteTrait { index: 0 })
        );

        // Out-of-range neural gene.
        let mut oversized = valid.clone();
        let neural_start = HEADER_LEN + TRAIT_COUNT * 4;
        oversized[neural_start..neural_start + 4].copy_from_slice(&100.0_f32.to_le_bytes());
        let checksum = fnv1a64(&oversized[..body_end]);
        oversized[body_end..].copy_from_slice(&checksum.to_le_bytes());
        assert_eq!(
            Genome::decode(&oversized),
            Err(GenomeError::NeuralOutOfRange { index: 0 })
        );
    }

    #[test]
    fn validated_rejects_bad_values_directly() {
        assert!(matches!(
            Genome::validated([2.0; TRAIT_COUNT], vec![0.0; NEURAL_COUNT]),
            Err(GenomeError::TraitOutOfRange { index: 0 })
        ));
        assert!(matches!(
            Genome::validated([0.5; TRAIT_COUNT], vec![f32::INFINITY; NEURAL_COUNT]),
            Err(GenomeError::NonFiniteNeural { index: 0 })
        ));
        assert!(matches!(
            Genome::validated([0.5; TRAIT_COUNT], vec![0.0; 3]),
            Err(GenomeError::WrongNeuralCount(3))
        ));
    }

    #[test]
    fn founders_are_deterministic_and_distinct() {
        assert_eq!(Genome::founder(7, 1), Genome::founder(7, 1));
        assert_ne!(Genome::founder(7, 1), Genome::founder(7, 2));
        assert_ne!(Genome::founder(7, 1), Genome::founder(8, 1));
    }

    #[test]
    fn recombination_is_deterministic_bounded_and_audited() {
        let parent_a = Genome::founder(1, 1);
        let parent_b = Genome::founder(1, 2);
        let policy = VariationPolicy {
            probability_q16: 6554, // 0.1
            trait_sigma_q16: 3277,
            neural_sigma_q16: 3277,
        };
        let (child_1, summary_1) = recombine(&parent_a, &parent_b, policy, 1, 100, 501);
        let (child_2, summary_2) = recombine(&parent_a, &parent_b, policy, 1, 100, 501);
        assert_eq!(child_1, child_2);
        assert_eq!(summary_1, summary_2);
        let (other_child, _) = recombine(&parent_a, &parent_b, policy, 1, 100, 502);
        assert_ne!(child_1, other_child);
        // Validity is guaranteed, not assumed.
        Genome::validated(*child_1.traits(), child_1.neural().to_vec()).unwrap();
    }

    #[test]
    fn variation_saturates_at_bounds() {
        // Force mutation on every gene with maximum magnitude; results must
        // stay inside bounds regardless.
        let extreme_a =
            Genome::validated([1.0; TRAIT_COUNT], vec![WEIGHT_LIMIT; NEURAL_COUNT]).unwrap();
        let extreme_b =
            Genome::validated([0.0; TRAIT_COUNT], vec![-WEIGHT_LIMIT; NEURAL_COUNT]).unwrap();
        let policy = VariationPolicy {
            probability_q16: 65536,
            trait_sigma_q16: 65536,
            neural_sigma_q16: 65536,
        };
        for child_id in 0..20 {
            let (child, summary) = recombine(&extreme_a, &extreme_b, policy, 9, 5, child_id);
            assert_eq!(summary.mutated_trait_genes, TRAIT_COUNT as u32);
            assert_eq!(summary.mutated_neural_genes, NEURAL_COUNT as u32);
            Genome::validated(*child.traits(), child.neural().to_vec()).unwrap();
        }
    }

    #[test]
    fn unrelated_draws_do_not_shift_recombination() {
        let parent_a = Genome::founder(3, 1);
        let parent_b = Genome::founder(3, 2);
        let policy = VariationPolicy {
            probability_q16: 6554,
            trait_sigma_q16: 3277,
            neural_sigma_q16: 3277,
        };
        let (before, _) = recombine(&parent_a, &parent_b, policy, 3, 50, 900);
        // Unrelated named draws from other systems/subjects.
        let _ = named_random(3, 50, RngSystem::Movement, 900, 7);
        let _ = named_random(3, 50, RngSystem::Recombination, 901, 0);
        let (after, _) = recombine(&parent_a, &parent_b, policy, 3, 50, 900);
        assert_eq!(before, after);
    }

    #[test]
    fn phenotype_mappings_hit_documented_ranges() {
        let low = Genome::validated([0.0; TRAIT_COUNT], vec![0.0; NEURAL_COUNT]).unwrap();
        let high = Genome::validated([1.0; TRAIT_COUNT], vec![0.0; NEURAL_COUNT]).unwrap();
        let low_phenotype = Phenotype::derive(&low);
        let high_phenotype = Phenotype::derive(&high);
        assert_eq!(low_phenotype.body_scale_milli, 600);
        assert_eq!(high_phenotype.body_scale_milli, 1600);
        assert_eq!(low_phenotype.max_speed_milli, 500);
        assert_eq!(high_phenotype.max_speed_milli, 3000);
        assert_eq!(low_phenotype.sensor_range_milli, 4000);
        assert_eq!(high_phenotype.sensor_range_milli, 12000);
        assert_eq!(low_phenotype.maturity_ticks, 400);
        assert_eq!(high_phenotype.maturity_ticks, 1200);
        assert_eq!(low_phenotype.invest_milli, 3000);
        assert_eq!(high_phenotype.invest_milli, 6000);
        assert_eq!(low_phenotype.cooldown_ticks, 200);
        assert_eq!(high_phenotype.cooldown_ticks, 600);
        // Derivation is deterministic.
        assert_eq!(Phenotype::derive(&low), Phenotype::derive(&low));
    }

    #[test]
    fn normalized_distance_is_bounded_and_symmetric() {
        let genome_a = Genome::founder(11, 1);
        let genome_b = Genome::founder(11, 2);
        let distance = genome_a.normalized_distance(&genome_b, 0);
        assert!((0.0..=1.0).contains(&distance));
        assert_eq!(distance, genome_b.normalized_distance(&genome_a, 0));
        assert_eq!(genome_a.normalized_distance(&genome_a, 0), 0.0);
        // Mixing neural distance stays bounded.
        let mixed = genome_a.normalized_distance(&genome_b, 16384);
        assert!((0.0..=1.0).contains(&mixed));
    }
}
