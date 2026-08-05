//! Genome schema 2: diploid, chromosomal, variable topology (Phase 9).
//!
//! Full design in `specifications/genome-schema-2.md`; decision in ADR-0013
//! as amended by ADR-0022 A8/A9 and D-066.
//!
//! Schema 1 stays in the build, evaluable and fixture-covered, forever.
//! There is no migration between them and there never will be: converting a
//! schema-1 genome would produce a genome that never existed and a lineage
//! that cannot be replayed.
//!
//! Three design points carry most of the weight.
//!
//! **Identity is four fields, not one.** An earlier draft used a single
//! `innovation_id` for alignment, ancestry, and event identity at once,
//! allocated from a global counter. `neuroevolution` section 1.6 calls that a
//! false-equivalence hazard and a global counter order-fragile. So:
//! `homology_id` aligns loci during meiosis, `gene_lineage_id` tracks
//! ancestry, `mutation_event_id` joins to the event log, and
//! `structural_signature` detects genuinely equivalent structure. The first
//! three are stored; the signature is **computed on demand** rather than
//! stored, because a stored derivation can drift from what it derives.
//!
//! **IDs are derived, not allocated.** Each is a domain-separated hash over
//! a canonical event key. That removes the last piece of shared mutable
//! state from reproduction, and it buys a property a counter cannot: two
//! lineages that independently evolve the same structural change converge on
//! the same `homology_id` and therefore *align* during meiosis, instead of
//! being treated as disjoint. That is the problem NEAT's innovation record
//! exists to patch, obtained here for free.
//!
//! **Sortedness is a decode-time invariant, not a convention.** Loci within
//! a chromosome are strictly ascending by `homology_id`, so adjacency in
//! that ordering *is* linkage: a crossover between two loci separates them,
//! and duplication inserts copies next to their source. Crossover positions
//! are positions in homology space, which is what makes crossover meaningful
//! between homologues of different lengths.

use crate::checksum::{Fnv1a64, fnv1a64};
use crate::registry::{
    ACTIVATION_REGISTRY_VERSION, Activation, CHANNEL_REGISTRY_VERSION, NodeRole, channel_exists,
};

pub const GENOME2_POLICY_VERSION: &str = "lifesim-genome-v2";
pub const GENOME2_SCHEMA_VERSION: u16 = 2;
pub const GENOME2_MAGIC: &[u8; 4] = b"ALG2";
pub const PLOIDY: u8 = 2;

/// Header is fixed-width and read before any allocation happens.
const HEADER_LEN: usize = 16;
/// `type_tag u8 | homology_id u32 | gene_lineage_id u64 | mutation_event_id u64`
const LOCUS_COMMON_LEN: usize = 21;

const TAG_TRAIT: u8 = 1;
const TAG_NODE: u8 = 2;
const TAG_EDGE: u8 = 3;
const TAG_IO_BINDING: u8 = 4;
// Tag 5 `Regulatory` is reserved and unallocated: a coarse regulatory locus
// is an open question, not a commitment.

/// `Edge.flags` bits. Bit 2 was added by D-066: the hybrid evaluation
/// ADR-0022 A9 adopted needs every edge typed zero-delay or delayed, and the
/// flags byte had nowhere to put it.
pub const EDGE_FLAG_PLASTIC: u8 = 1 << 0;
pub const EDGE_FLAG_DISABLED: u8 = 1 << 1;
pub const EDGE_FLAG_DELAYED: u8 = 1 << 2;
const EDGE_FLAG_MASK: u8 = EDGE_FLAG_PLASTIC | EDGE_FLAG_DISABLED | EDGE_FLAG_DELAYED;

/// Weight and bias bound, unchanged from schema 1.
pub const VALUE_LIMIT: f32 = 8.0;

/// Trait loci occupy a reserved low range of the homology space, sorted by
/// `trait_id`, so a trait can never collide with a structural innovation and
/// the two sort blocks never interleave.
pub const TRAIT_HOMOLOGY_BASE: u32 = 1;
pub const TRAIT_HOMOLOGY_LIMIT: u32 = 1 << 16;
/// Structural innovations start above the trait block.
pub const STRUCTURAL_HOMOLOGY_BASE: u32 = TRAIT_HOMOLOGY_LIMIT;

/// Structural caps. Every one rejects deterministically and is counted;
/// none is ever silently exceeded.
///
/// **These values are provisional** and C9.8 says so explicitly: they must
/// be restated against a measured snapshot budget under a realistic evolved
/// topology distribution. They are recorded here as the starting point that
/// measurement will replace, not as settled policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GenomeCaps {
    pub max_chromosomes: u8,
    pub max_loci_per_chromosome: u32,
    pub max_nodes: u32,
    pub max_edges: u32,
    pub max_edges_per_node: u32,
    pub max_genome_bytes: u32,
    pub min_nodes: u32,
}

impl GenomeCaps {
    pub fn provisional() -> Self {
        Self {
            max_chromosomes: 4,
            max_loci_per_chromosome: 512,
            max_nodes: 256,
            max_edges: 1_024,
            max_edges_per_node: 64,
            // Phase 4 recorded snapshot size already dominated by
            // per-organism genome arrays at roughly 2.8 KB each; 16 KB is
            // deliberately generous and deliberately provisional.
            max_genome_bytes: 16_384,
            min_nodes: 2,
        }
    }
}

/// Plasticity genes, carried by every edge whether or not it is plastic.
///
/// Inherited, validated, dominance-expressed, and **behaviorally inert
/// until Phase 11** - exactly the pattern thermal preference and defense
/// tendency followed from Phase 2 to Phase 8. Occupying the space now means
/// enabling plasticity is a flag flip rather than a schema change.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlasticityGenes {
    pub rule_id: u8,
    pub eta: f32,
    pub coefficients: [f32; 4],
    pub decay: f32,
    /// Node `homology_id` gating this edge, or 0 for ungated.
    pub modulator_node: u32,
}

impl PlasticityGenes {
    pub const ENCODED_LEN: usize = 1 + 4 + 16 + 4 + 4;

    pub fn inert() -> Self {
        Self {
            rule_id: 0,
            eta: 0.0,
            coefficients: [0.0; 4],
            decay: 0.0,
            modulator_node: 0,
        }
    }

    fn valid(&self) -> bool {
        self.eta.is_finite()
            && (0.0..=1.0).contains(&self.eta)
            && self.decay.is_finite()
            && (0.0..=1.0).contains(&self.decay)
            && self
                .coefficients
                .iter()
                .all(|value| value.is_finite() && (-1.0..=1.0).contains(value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LocusKind {
    Trait {
        trait_id: u16,
        value: f32,
        dominance: f32,
    },
    Node {
        role: NodeRole,
        activation_id: u8,
        bias: f32,
        time_constant: u16,
    },
    Edge {
        source: u32,
        target: u32,
        weight: f32,
        flags: u8,
        plasticity: PlasticityGenes,
    },
    IoBinding {
        node: u32,
        channel_id: u16,
        gain: f32,
    },
}

impl LocusKind {
    fn tag(&self) -> u8 {
        match self {
            LocusKind::Trait { .. } => TAG_TRAIT,
            LocusKind::Node { .. } => TAG_NODE,
            LocusKind::Edge { .. } => TAG_EDGE,
            LocusKind::IoBinding { .. } => TAG_IO_BINDING,
        }
    }

    fn payload_len(tag: u8) -> Option<usize> {
        Some(match tag {
            TAG_TRAIT => 2 + 4 + 4,
            TAG_NODE => 1 + 1 + 4 + 2,
            TAG_EDGE => 4 + 4 + 4 + 1 + PlasticityGenes::ENCODED_LEN,
            TAG_IO_BINDING => 4 + 2 + 4,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Locus {
    /// The structural slot two loci share. Sort key, alignment key during
    /// meiosis, and the identifier edges and bindings refer to.
    pub homology_id: u32,
    pub gene_lineage_id: u64,
    pub mutation_event_id: u64,
    pub kind: LocusKind,
}

impl Locus {
    /// Canonical phenotype-relevant identity, computed rather than stored.
    ///
    /// Two loci with equal signatures are genuinely the same structure even
    /// if they arose independently and carry different lineage or event
    /// identity. Storing this alongside the fields it derives from would let
    /// the two disagree; computing it means they cannot.
    pub fn structural_signature(&self) -> u64 {
        let mut hasher = Fnv1a64::new();
        hasher.update(b"lifesim-structsig-v1");
        hasher.update(&[self.kind.tag()]);
        match self.kind {
            LocusKind::Trait { trait_id, .. } => hasher.update_u32(u32::from(trait_id)),
            LocusKind::Node {
                role,
                activation_id,
                ..
            } => {
                hasher.update_u32(u32::from(role.id()));
                hasher.update_u32(u32::from(activation_id));
            }
            LocusKind::Edge {
                source,
                target,
                flags,
                ..
            } => {
                hasher.update_u32(source);
                hasher.update_u32(target);
                // Only the delay bit is phenotype-relevant structure;
                // `plastic` and `disabled` are expression state.
                hasher.update_u32(u32::from(flags & EDGE_FLAG_DELAYED));
            }
            LocusKind::IoBinding {
                node, channel_id, ..
            } => {
                hasher.update_u32(node);
                hasher.update_u32(u32::from(channel_id));
            }
        }
        hasher.finish()
    }
}

/// One haplotype: `C` chromosomes, each a sorted locus list.
#[derive(Clone, Debug, PartialEq)]
pub struct Haplotype {
    pub chromosomes: Vec<Vec<Locus>>,
}

/// A diploid genome. Haplotype slot 0 comes from the lower-ID parent and
/// slot 1 from the higher-ID parent (determinism Rule 3), so no traversal
/// order enters the record.
#[derive(Clone, Debug, PartialEq)]
pub struct Genome2 {
    pub haplotypes: [Haplotype; 2],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Genome2Error {
    TooShort,
    BadMagic,
    UnsupportedSchema(u16),
    UnsupportedChannelRegistry(u16),
    UnsupportedActivationRegistry(u16),
    UnsupportedPloidy(u8),
    ChromosomeCount(u8),
    UnknownFlags(u16),
    /// `total_len` disagrees with the buffer it describes.
    LengthMismatch {
        declared: u32,
        actual: usize,
    },
    ChecksumMismatch,
    /// A declared locus count above the cap, refused before allocation.
    LocusCountTooLarge {
        chromosome: usize,
        count: u32,
    },
    UnknownLocusType(u8),
    TrailingBytes,
    /// Loci must strictly ascend by `homology_id` within a chromosome.
    NotSorted {
        chromosome: usize,
        at: usize,
    },
    /// A trait locus outside the reserved trait block, or a structural
    /// locus inside it.
    HomologyBlockViolation {
        homology_id: u32,
        tag: u8,
    },
    ValueOutOfRange(&'static str),
    UnknownActivation(u8),
    UnknownNodeRole(u8),
    UnknownChannel(u16),
    /// An edge or binding referring to a node that is not in the same
    /// haplotype.
    DanglingReference {
        homology_id: u32,
        target: u32,
    },
    CapExceeded(&'static str),
    /// A cycle among zero-delay edges. A decode-time error, not a runtime
    /// condition: the hybrid evaluation ADR-0022 A9 adopted breaks cycles
    /// through delayed edges, and a zero-delay cycle has no fixed point.
    ZeroDelayCycle,
}

impl std::fmt::Display for Genome2Error {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

// --- Derived identity -------------------------------------------------------

/// `homology_id = H(policy, parent_homology, operator, target_slot, attempt)`
///
/// Derived rather than allocated, so the same structural mutation applied to
/// the same parent under the same policy always yields the same identity -
/// including when two lineages do it independently, which is what makes them
/// align during meiosis instead of being treated as disjoint.
///
/// The result is forced above the reserved trait block, so a structural
/// innovation can never collide with a trait locus.
pub fn derive_homology_id(
    parent_homology: u32,
    operator: u8,
    target_slot: u32,
    attempt_ordinal: u32,
) -> u32 {
    let mut hasher = Fnv1a64::new();
    hasher.update(b"lifesim-homology-v1");
    hasher.update(GENOME2_POLICY_VERSION.as_bytes());
    hasher.update_u32(parent_homology);
    hasher.update_u32(u32::from(operator));
    hasher.update_u32(target_slot);
    hasher.update_u32(attempt_ordinal);
    let span = u32::MAX - STRUCTURAL_HOMOLOGY_BASE;
    STRUCTURAL_HOMOLOGY_BASE + (hasher.finish() % u64::from(span)) as u32
}

pub fn derive_mutation_event_id(
    world_seed: u64,
    tick: u64,
    child_object_id: u64,
    operator: u8,
    attempt_ordinal: u32,
) -> u64 {
    let mut hasher = Fnv1a64::new();
    hasher.update(b"lifesim-mutevent-v1");
    hasher.update_u64(world_seed);
    hasher.update_u64(tick);
    hasher.update_u64(child_object_id);
    hasher.update_u32(u32::from(operator));
    hasher.update_u32(attempt_ordinal);
    hasher.finish()
}

pub fn derive_gene_lineage_id(
    world_seed: u64,
    tick: u64,
    child_object_id: u64,
    homology_id: u32,
) -> u64 {
    let mut hasher = Fnv1a64::new();
    hasher.update(b"lifesim-genelineage-v1");
    hasher.update_u64(world_seed);
    hasher.update_u64(tick);
    hasher.update_u64(child_object_id);
    hasher.update_u32(homology_id);
    hasher.finish()
}

// --- Encoding ---------------------------------------------------------------

fn put_u16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_u64(out: &mut Vec<u8>, value: u64) {
    out.extend_from_slice(&value.to_le_bytes());
}
fn put_f32(out: &mut Vec<u8>, value: f32) {
    out.extend_from_slice(&value.to_le_bytes());
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], Genome2Error> {
        if self.offset + count > self.bytes.len() {
            return Err(Genome2Error::TooShort);
        }
        let slice = &self.bytes[self.offset..self.offset + count];
        self.offset += count;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, Genome2Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Genome2Error> {
        let slice = self.take(2)?;
        Ok(u16::from_le_bytes([slice[0], slice[1]]))
    }
    fn u32(&mut self) -> Result<u32, Genome2Error> {
        let slice = self.take(4)?;
        Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }
    fn u64(&mut self) -> Result<u64, Genome2Error> {
        let slice = self.take(8)?;
        let mut buffer = [0_u8; 8];
        buffer.copy_from_slice(slice);
        Ok(u64::from_le_bytes(buffer))
    }
    fn f32(&mut self) -> Result<f32, Genome2Error> {
        let slice = self.take(4)?;
        Ok(f32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
    }
}

impl Genome2 {
    pub fn chromosome_count(&self) -> usize {
        self.haplotypes[0].chromosomes.len()
    }

    pub fn loci(&self) -> impl Iterator<Item = &Locus> {
        self.haplotypes
            .iter()
            .flat_map(|haplotype| haplotype.chromosomes.iter().flatten())
    }

    pub fn encode(&self) -> Vec<u8> {
        let chromosomes = self.chromosome_count() as u8;
        let mut body = Vec::new();
        for haplotype in &self.haplotypes {
            for chromosome in &haplotype.chromosomes {
                put_u32(&mut body, chromosome.len() as u32);
                for locus in chromosome {
                    body.push(locus.kind.tag());
                    put_u32(&mut body, locus.homology_id);
                    put_u64(&mut body, locus.gene_lineage_id);
                    put_u64(&mut body, locus.mutation_event_id);
                    match locus.kind {
                        LocusKind::Trait {
                            trait_id,
                            value,
                            dominance,
                        } => {
                            put_u16(&mut body, trait_id);
                            put_f32(&mut body, value);
                            put_f32(&mut body, dominance);
                        }
                        LocusKind::Node {
                            role,
                            activation_id,
                            bias,
                            time_constant,
                        } => {
                            body.push(role.id());
                            body.push(activation_id);
                            put_f32(&mut body, bias);
                            put_u16(&mut body, time_constant);
                        }
                        LocusKind::Edge {
                            source,
                            target,
                            weight,
                            flags,
                            plasticity,
                        } => {
                            put_u32(&mut body, source);
                            put_u32(&mut body, target);
                            put_f32(&mut body, weight);
                            body.push(flags);
                            body.push(plasticity.rule_id);
                            put_f32(&mut body, plasticity.eta);
                            for coefficient in plasticity.coefficients {
                                put_f32(&mut body, coefficient);
                            }
                            put_f32(&mut body, plasticity.decay);
                            put_u32(&mut body, plasticity.modulator_node);
                        }
                        LocusKind::IoBinding {
                            node,
                            channel_id,
                            gain,
                        } => {
                            put_u32(&mut body, node);
                            put_u16(&mut body, channel_id);
                            put_f32(&mut body, gain);
                        }
                    }
                }
            }
        }

        let total_len = (HEADER_LEN + body.len() + 8) as u32;
        let mut out = Vec::with_capacity(total_len as usize);
        out.extend_from_slice(GENOME2_MAGIC);
        put_u16(&mut out, GENOME2_SCHEMA_VERSION);
        put_u16(&mut out, CHANNEL_REGISTRY_VERSION);
        out.push(PLOIDY);
        out.push(chromosomes);
        put_u16(&mut out, 0); // flags
        put_u32(&mut out, total_len);
        debug_assert_eq!(out.len(), HEADER_LEN);
        out.extend_from_slice(&body);
        let checksum = fnv1a64(&out);
        put_u64(&mut out, checksum);
        out
    }

    /// Bounded, fail-closed decode. There is no repair path and no partial
    /// acceptance: a `Genome2` in world state is valid by construction.
    ///
    /// Order matters and follows the specification exactly. Framing and
    /// `total_len` are checked **before any allocation**; the checksum is
    /// verified **before any payload byte is interpreted**; every declared
    /// locus count is checked against the cap before that chromosome is
    /// allocated; then values, then structure.
    pub fn decode(bytes: &[u8], caps: &GenomeCaps) -> Result<Self, Genome2Error> {
        if bytes.len() < HEADER_LEN + 8 {
            return Err(Genome2Error::TooShort);
        }
        if &bytes[0..4] != GENOME2_MAGIC {
            return Err(Genome2Error::BadMagic);
        }
        let mut reader = Reader { bytes, offset: 4 };
        let schema = reader.u16()?;
        if schema != GENOME2_SCHEMA_VERSION {
            return Err(Genome2Error::UnsupportedSchema(schema));
        }
        let registry = reader.u16()?;
        if registry != CHANNEL_REGISTRY_VERSION {
            return Err(Genome2Error::UnsupportedChannelRegistry(registry));
        }
        let ploidy = reader.u8()?;
        if ploidy != PLOIDY {
            return Err(Genome2Error::UnsupportedPloidy(ploidy));
        }
        let chromosomes = reader.u8()?;
        if chromosomes == 0 || chromosomes > caps.max_chromosomes {
            return Err(Genome2Error::ChromosomeCount(chromosomes));
        }
        let flags = reader.u16()?;
        if flags != 0 {
            return Err(Genome2Error::UnknownFlags(flags));
        }
        let total_len = reader.u32()?;
        if total_len as usize != bytes.len() {
            return Err(Genome2Error::LengthMismatch {
                declared: total_len,
                actual: bytes.len(),
            });
        }
        if total_len > caps.max_genome_bytes {
            return Err(Genome2Error::CapExceeded("max_genome_bytes"));
        }

        // The checksum covers everything before it, and is verified before
        // any payload byte is interpreted.
        let split = bytes.len() - 8;
        let declared = u64::from_le_bytes(bytes[split..].try_into().expect("8 bytes"));
        if fnv1a64(&bytes[..split]) != declared {
            return Err(Genome2Error::ChecksumMismatch);
        }

        let mut haplotypes = Vec::with_capacity(2);
        for _ in 0..2 {
            let mut chromosome_list = Vec::with_capacity(chromosomes as usize);
            for chromosome_index in 0..chromosomes as usize {
                let count = reader.u32()?;
                if count > caps.max_loci_per_chromosome {
                    return Err(Genome2Error::LocusCountTooLarge {
                        chromosome: chromosome_index,
                        count,
                    });
                }
                // A count that cannot possibly fit in the remaining bytes is
                // refused before the vector is sized to it.
                let remaining = split.saturating_sub(reader.offset);
                if (count as usize).saturating_mul(LOCUS_COMMON_LEN) > remaining {
                    return Err(Genome2Error::LocusCountTooLarge {
                        chromosome: chromosome_index,
                        count,
                    });
                }
                let mut loci = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    loci.push(decode_locus(&mut reader)?);
                }
                for index in 1..loci.len() {
                    if loci[index].homology_id <= loci[index - 1].homology_id {
                        return Err(Genome2Error::NotSorted {
                            chromosome: chromosome_index,
                            at: index,
                        });
                    }
                }
                chromosome_list.push(loci);
            }
            haplotypes.push(Haplotype {
                chromosomes: chromosome_list,
            });
        }
        if reader.offset != split {
            return Err(Genome2Error::TrailingBytes);
        }

        let genome = Genome2 {
            haplotypes: [haplotypes.remove(0), haplotypes.remove(0)],
        };
        genome.validate_structure(caps)?;
        Ok(genome)
    }

    /// Structural invariants. Separated from decode so the mutation
    /// operators can re-assert them on their own output: after mutation the
    /// specification treats a validation failure as a bug rather than a
    /// runtime condition, and this is the assertion that catches it.
    pub fn validate_structure(&self, caps: &GenomeCaps) -> Result<(), Genome2Error> {
        let mut total_nodes = 0_u32;
        let mut total_edges = 0_u32;
        for haplotype in &self.haplotypes {
            if haplotype.chromosomes.len() != self.chromosome_count() {
                return Err(Genome2Error::ChromosomeCount(
                    haplotype.chromosomes.len() as u8
                ));
            }
            // Node presence is per haplotype, because an edge may only refer
            // to a node on its own haplotype: a reference across haplotypes
            // would make expression depend on the other parent's genome.
            let mut nodes: Vec<u32> = Vec::new();
            for locus in haplotype.chromosomes.iter().flatten() {
                if let LocusKind::Node { .. } = locus.kind {
                    nodes.push(locus.homology_id);
                }
            }
            nodes.sort_unstable();
            let present = |id: u32| nodes.binary_search(&id).is_ok();

            let mut edges_out: Vec<(u32, u32)> = Vec::new();
            let mut incoming: Vec<u32> = Vec::new();
            for locus in haplotype.chromosomes.iter().flatten() {
                match locus.kind {
                    LocusKind::Trait { .. } => {
                        if locus.homology_id >= TRAIT_HOMOLOGY_LIMIT {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_TRAIT,
                            });
                        }
                    }
                    LocusKind::Node { .. } => {
                        if locus.homology_id < STRUCTURAL_HOMOLOGY_BASE {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_NODE,
                            });
                        }
                        total_nodes += 1;
                    }
                    LocusKind::Edge {
                        source,
                        target,
                        flags,
                        plasticity,
                        ..
                    } => {
                        if locus.homology_id < STRUCTURAL_HOMOLOGY_BASE {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_EDGE,
                            });
                        }
                        if !present(source) {
                            return Err(Genome2Error::DanglingReference {
                                homology_id: locus.homology_id,
                                target: source,
                            });
                        }
                        if !present(target) {
                            return Err(Genome2Error::DanglingReference {
                                homology_id: locus.homology_id,
                                target,
                            });
                        }
                        if plasticity.modulator_node != 0 && !present(plasticity.modulator_node) {
                            return Err(Genome2Error::DanglingReference {
                                homology_id: locus.homology_id,
                                target: plasticity.modulator_node,
                            });
                        }
                        total_edges += 1;
                        incoming.push(target);
                        if flags & EDGE_FLAG_DELAYED == 0 && flags & EDGE_FLAG_DISABLED == 0 {
                            edges_out.push((source, target));
                        }
                    }
                    LocusKind::IoBinding { node, .. } => {
                        if locus.homology_id < STRUCTURAL_HOMOLOGY_BASE {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_IO_BINDING,
                            });
                        }
                        if !present(node) {
                            return Err(Genome2Error::DanglingReference {
                                homology_id: locus.homology_id,
                                target: node,
                            });
                        }
                    }
                }
            }
            if (nodes.len() as u32) < caps.min_nodes {
                return Err(Genome2Error::CapExceeded("min_nodes"));
            }
            incoming.sort_unstable();
            let mut run = 1_u32;
            for index in 1..incoming.len() {
                if incoming[index] == incoming[index - 1] {
                    run += 1;
                    if run > caps.max_edges_per_node {
                        return Err(Genome2Error::CapExceeded("max_edges_per_node"));
                    }
                } else {
                    run = 1;
                }
            }
            if has_cycle(&nodes, &edges_out) {
                return Err(Genome2Error::ZeroDelayCycle);
            }
        }
        if total_nodes > caps.max_nodes * 2 {
            return Err(Genome2Error::CapExceeded("max_nodes"));
        }
        if total_edges > caps.max_edges * 2 {
            return Err(Genome2Error::CapExceeded("max_edges"));
        }

        // **The merged network is the one that gets compiled, so it is the
        // one that has to be acyclic.** The per-haplotype check above is
        // necessary and not sufficient: haplotype 0 carrying `A -> B` and
        // haplotype 1 carrying `B -> A` is two acyclic haplotypes whose
        // expression is a zero-delay cycle, because expression takes the
        // union over homology IDs. Meiosis assembles exactly that
        // combination from two viable parents, and insertion makes it easy
        // to reach, so this is a routine genetic event rather than a corner
        // case.
        //
        // Found by a campaign-scale run panicking in the sense phase: the
        // genome validated, `compile` refused it, and the birth path
        // admitted a half-registered organism. Checked here by expressing
        // and re-running the same Kahn pass, rather than by a second merge
        // rule written to match, because two merge rules that must agree
        // forever is how this bug would come back.
        let network = self.express_network();
        let mut nodes: Vec<u32> = network.nodes.iter().map(|node| node.homology_id).collect();
        nodes.sort_unstable();
        let edges: Vec<(u32, u32)> = network
            .edges
            .iter()
            .filter(|edge| !edge.delayed && !edge.disabled)
            .map(|edge| (edge.source, edge.target))
            .collect();
        if has_cycle(&nodes, &edges) {
            return Err(Genome2Error::ZeroDelayCycle);
        }
        Ok(())
    }
}

/// Kahn's algorithm over the zero-delay subgraph.
///
/// A cycle here is a decode-time error rather than something evaluation
/// works around: under the hybrid update a zero-delay cycle has no fixed
/// point, and the honest response is to refuse the genome rather than to
/// invent an iteration order that produces *a* number.
fn has_cycle(nodes: &[u32], edges: &[(u32, u32)]) -> bool {
    if edges.is_empty() {
        return false;
    }
    let index_of = |id: u32| nodes.binary_search(&id).ok();
    let mut in_degree = vec![0_u32; nodes.len()];
    let mut adjacency: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    for &(source, target) in edges {
        let (Some(from), Some(to)) = (index_of(source), index_of(target)) else {
            // A dangling edge is caught by the caller; treat it as acyclic
            // here so the two errors do not mask each other.
            continue;
        };
        adjacency[from].push(to);
        in_degree[to] += 1;
    }
    let mut queue: Vec<usize> = (0..nodes.len()).filter(|i| in_degree[*i] == 0).collect();
    let mut visited = 0_usize;
    while let Some(node) = queue.pop() {
        visited += 1;
        for &next in &adjacency[node] {
            in_degree[next] -= 1;
            if in_degree[next] == 0 {
                queue.push(next);
            }
        }
    }
    visited != nodes.len()
}

fn decode_locus(reader: &mut Reader<'_>) -> Result<Locus, Genome2Error> {
    let tag = reader.u8()?;
    let payload = LocusKind::payload_len(tag).ok_or(Genome2Error::UnknownLocusType(tag))?;
    let homology_id = reader.u32()?;
    let gene_lineage_id = reader.u64()?;
    let mutation_event_id = reader.u64()?;
    // The payload length is known from the tag, so a truncated record is
    // caught by the reader rather than by mis-parsing the next locus.
    if reader.offset + payload > reader.bytes.len() {
        return Err(Genome2Error::TooShort);
    }

    let kind = match tag {
        TAG_TRAIT => {
            let trait_id = reader.u16()?;
            let value = reader.f32()?;
            let dominance = reader.f32()?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(Genome2Error::ValueOutOfRange("trait value"));
            }
            if !dominance.is_finite() || !(0.0..=1.0).contains(&dominance) {
                return Err(Genome2Error::ValueOutOfRange("trait dominance"));
            }
            if usize::from(trait_id) >= crate::genome::TRAIT_COUNT {
                return Err(Genome2Error::ValueOutOfRange("trait_id"));
            }
            LocusKind::Trait {
                trait_id,
                value,
                dominance,
            }
        }
        TAG_NODE => {
            let role_id = reader.u8()?;
            let activation_id = reader.u8()?;
            let bias = reader.f32()?;
            let time_constant = reader.u16()?;
            let role = NodeRole::from_id(role_id).ok_or(Genome2Error::UnknownNodeRole(role_id))?;
            if Activation::from_id(activation_id).is_none() {
                return Err(Genome2Error::UnknownActivation(activation_id));
            }
            if !bias.is_finite() || !(-VALUE_LIMIT..=VALUE_LIMIT).contains(&bias) {
                return Err(Genome2Error::ValueOutOfRange("node bias"));
            }
            LocusKind::Node {
                role,
                activation_id,
                bias,
                time_constant,
            }
        }
        TAG_EDGE => {
            let source = reader.u32()?;
            let target = reader.u32()?;
            let weight = reader.f32()?;
            let flags = reader.u8()?;
            let plasticity = PlasticityGenes {
                rule_id: reader.u8()?,
                eta: reader.f32()?,
                coefficients: [reader.f32()?, reader.f32()?, reader.f32()?, reader.f32()?],
                decay: reader.f32()?,
                modulator_node: reader.u32()?,
            };
            if !weight.is_finite() || !(-VALUE_LIMIT..=VALUE_LIMIT).contains(&weight) {
                return Err(Genome2Error::ValueOutOfRange("edge weight"));
            }
            if flags & !EDGE_FLAG_MASK != 0 {
                return Err(Genome2Error::ValueOutOfRange("edge flags"));
            }
            if !plasticity.valid() {
                return Err(Genome2Error::ValueOutOfRange("plasticity genes"));
            }
            LocusKind::Edge {
                source,
                target,
                weight,
                flags,
                plasticity,
            }
        }
        TAG_IO_BINDING => {
            let node = reader.u32()?;
            let channel_id = reader.u16()?;
            let gain = reader.f32()?;
            if !channel_exists(channel_id) {
                return Err(Genome2Error::UnknownChannel(channel_id));
            }
            if !gain.is_finite() || !(-VALUE_LIMIT..=VALUE_LIMIT).contains(&gain) {
                return Err(Genome2Error::ValueOutOfRange("binding gain"));
            }
            LocusKind::IoBinding {
                node,
                channel_id,
                gain,
            }
        }
        other => return Err(Genome2Error::UnknownLocusType(other)),
    };

    Ok(Locus {
        homology_id,
        gene_lineage_id,
        mutation_event_id,
        kind,
    })
}

/// Registry versions recorded so a config hash can include them.
pub fn registry_versions() -> (u16, u16) {
    (CHANNEL_REGISTRY_VERSION, ACTIVATION_REGISTRY_VERSION)
}

// --- Expression -------------------------------------------------------------

/// The expressed network: the union of both haplotypes keyed by
/// `homology_id`, with scalar parameters combined by dominance.
///
/// Expression is a **pure function of the genome**, recomputed on load and
/// never persisted as truth, which keeps the existing rule that derived
/// state is recomputed rather than trusted from a save.
#[derive(Clone, Debug, PartialEq)]
pub struct ExpressedNetwork {
    /// Nodes in ascending `homology_id` order.
    pub nodes: Vec<ExpressedNode>,
    /// Edges in ascending `homology_id` order. **Summation order is this
    /// order, never storage order**: float addition is not associative, so a
    /// storage-order sum is a replay bug that stays invisible until a
    /// compaction changes layout.
    pub edges: Vec<ExpressedEdge>,
    pub bindings: Vec<ExpressedBinding>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpressedNode {
    pub homology_id: u32,
    pub role: NodeRole,
    pub activation: Activation,
    pub bias: f32,
    pub time_constant: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpressedEdge {
    pub homology_id: u32,
    pub source: u32,
    pub target: u32,
    pub weight: f32,
    pub disabled: bool,
    pub plastic: bool,
    pub delayed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExpressedBinding {
    pub homology_id: u32,
    pub node: u32,
    pub channel_id: u16,
    pub gain: f32,
}

/// One haplotype's contribution at a homology slot, or its absence.
/// Slot 0 is the lower-ID parent's, slot 1 the higher-ID parent's.
type HaplotypePair<T> = [Option<T>; 2];
/// `(role, activation_id, bias, time_constant, dominance)`
type NodeAllele = (NodeRole, u8, f32, u16, f32);
/// `(source, target, weight, flags, dominance)`
type EdgeAllele = (u32, u32, f32, u8, f32);
/// `(node, channel_id, gain, dominance)`
type BindingAllele = (u32, u16, f32, f32);

/// Dominance-weighted blend of two allele values.
///
/// Spans the biological range continuously: equal dominance gives additive
/// (codominant) inheritance, `d0 = 1, d1 = 0` gives complete dominance, and
/// intermediate values give incomplete dominance. Dominance is itself a
/// gene, so dominance relationships evolve.
///
/// Order-free given the fixed haplotype slot assignment, which is what makes
/// it safe to call without knowing which parent contributed which slot.
pub fn blend_by_dominance(v0: f32, d0: f32, v1: f32, d1: f32) -> f32 {
    let total = d0 + d1;
    if total > 0.0 {
        (d0 * v0 + d1 * v1) / total
    } else {
        0.5 * (v0 + v1)
    }
}

impl Genome2 {
    /// Expressed trait values, indexed by `trait_id`.
    ///
    /// A trait present on one haplotype only is hemizygous and expresses its
    /// single value. A trait absent from both is `None`, which the caller
    /// resolves against its own default rather than this module inventing
    /// one.
    pub fn express_traits(&self) -> [Option<f32>; crate::genome::TRAIT_COUNT] {
        let mut left: [Option<(f32, f32)>; crate::genome::TRAIT_COUNT] =
            [None; crate::genome::TRAIT_COUNT];
        let mut right = left;
        for (slot, store) in [(0_usize, &mut left), (1, &mut right)] {
            for locus in self.haplotypes[slot].chromosomes.iter().flatten() {
                if let LocusKind::Trait {
                    trait_id,
                    value,
                    dominance,
                } = locus.kind
                {
                    store[usize::from(trait_id)] = Some((value, dominance));
                }
            }
        }
        let mut out = [None; crate::genome::TRAIT_COUNT];
        for index in 0..crate::genome::TRAIT_COUNT {
            out[index] = match (left[index], right[index]) {
                (Some((v0, d0)), Some((v1, d1))) => Some(blend_by_dominance(v0, d0, v1, d1)),
                (Some((value, _)), None) | (None, Some((value, _))) => Some(value),
                (None, None) => None,
            };
        }
        out
    }

    /// The expressed network.
    ///
    /// Flag combination is versioned policy, recorded rather than incidental:
    /// `disabled` is expressed only when disabled on **both** haplotypes
    /// (dominance of function, so one working copy is enough), and `plastic`
    /// and `delayed` are expressed when set on **either**. An innovation
    /// present on one haplotype only expresses at its single value, which is
    /// what makes a fresh duplication immediately hemizygous and
    /// heterozygous - the biological situation after a duplication, and the
    /// source of the divergence that follows.
    pub fn express_network(&self) -> ExpressedNetwork {
        let mut nodes: Vec<ExpressedNode> = Vec::new();
        let mut edges: Vec<ExpressedEdge> = Vec::new();
        let mut bindings: Vec<ExpressedBinding> = Vec::new();

        // Gather each haplotype's contribution keyed by homology, then
        // combine. Both passes walk in sorted order, so the merge below is
        // deterministic without any further sorting decision.
        let mut node_pairs: Vec<(u32, HaplotypePair<NodeAllele>)> = Vec::new();
        let mut edge_pairs: Vec<(u32, HaplotypePair<EdgeAllele>)> = Vec::new();
        let mut binding_pairs: Vec<(u32, HaplotypePair<BindingAllele>)> = Vec::new();

        for slot in 0..2_usize {
            // Dominance for a structural locus is taken from the trait-like
            // dominance of its own record; nodes and edges have no dominance
            // field of their own, so they combine additively unless one side
            // is absent. This is the specification's "scalar parameters
            // combine by the same dominance formula" with both dominances
            // equal, which is exactly codominance.
            let dominance = 1.0_f32;
            for locus in self.haplotypes[slot].chromosomes.iter().flatten() {
                match locus.kind {
                    LocusKind::Trait { .. } => {}
                    LocusKind::Node {
                        role,
                        activation_id,
                        bias,
                        time_constant,
                    } => {
                        upsert(
                            &mut node_pairs,
                            locus.homology_id,
                            slot,
                            (role, activation_id, bias, time_constant, dominance),
                        );
                    }
                    LocusKind::Edge {
                        source,
                        target,
                        weight,
                        flags,
                        ..
                    } => {
                        upsert(
                            &mut edge_pairs,
                            locus.homology_id,
                            slot,
                            (source, target, weight, flags, dominance),
                        );
                    }
                    LocusKind::IoBinding {
                        node,
                        channel_id,
                        gain,
                    } => {
                        upsert(
                            &mut binding_pairs,
                            locus.homology_id,
                            slot,
                            (node, channel_id, gain, dominance),
                        );
                    }
                }
            }
        }

        for (homology_id, sides) in node_pairs {
            let (role, activation_id, bias, time_constant) = match (sides[0], sides[1]) {
                (Some((r0, a0, b0, t0, d0)), Some((_, _, b1, t1, d1))) => (
                    r0,
                    a0,
                    blend_by_dominance(b0, d0, b1, d1),
                    ((u32::from(t0) + u32::from(t1)) / 2) as u16,
                ),
                (Some((r, a, b, t, _)), None) | (None, Some((r, a, b, t, _))) => (r, a, b, t),
                (None, None) => continue,
            };
            nodes.push(ExpressedNode {
                homology_id,
                role,
                activation: Activation::from_id(activation_id).unwrap_or(Activation::Linear),
                bias,
                time_constant,
            });
        }
        for (homology_id, sides) in edge_pairs {
            let (source, target, weight, flags) = match (sides[0], sides[1]) {
                (Some((s0, t0, w0, f0, d0)), Some((_, _, w1, f1, d1))) => {
                    // `disabled` needs both; `plastic` and `delayed` need
                    // either. Recorded policy, not an accident of `&`/`|`.
                    let disabled = f0 & f1 & EDGE_FLAG_DISABLED;
                    let permissive = (f0 | f1) & (EDGE_FLAG_PLASTIC | EDGE_FLAG_DELAYED);
                    (
                        s0,
                        t0,
                        blend_by_dominance(w0, d0, w1, d1),
                        disabled | permissive,
                    )
                }
                (Some((s, t, w, f, _)), None) | (None, Some((s, t, w, f, _))) => (s, t, w, f),
                (None, None) => continue,
            };
            edges.push(ExpressedEdge {
                homology_id,
                source,
                target,
                weight,
                disabled: flags & EDGE_FLAG_DISABLED != 0,
                plastic: flags & EDGE_FLAG_PLASTIC != 0,
                delayed: flags & EDGE_FLAG_DELAYED != 0,
            });
        }
        for (homology_id, sides) in binding_pairs {
            let (node, channel_id, gain) = match (sides[0], sides[1]) {
                (Some((n0, c0, g0, d0)), Some((_, _, g1, d1))) => {
                    (n0, c0, blend_by_dominance(g0, d0, g1, d1))
                }
                (Some((n, c, g, _)), None) | (None, Some((n, c, g, _))) => (n, c, g),
                (None, None) => continue,
            };
            bindings.push(ExpressedBinding {
                homology_id,
                node,
                channel_id,
                gain,
            });
        }

        nodes.sort_unstable_by_key(|node| node.homology_id);
        edges.sort_unstable_by_key(|edge| edge.homology_id);
        bindings.sort_unstable_by_key(|binding| binding.homology_id);
        ExpressedNetwork {
            nodes,
            edges,
            bindings,
        }
    }
}

fn upsert<T: Copy>(store: &mut Vec<(u32, HaplotypePair<T>)>, key: u32, slot: usize, value: T) {
    match store.binary_search_by_key(&key, |(existing, _)| *existing) {
        Ok(index) => store[index].1[slot] = Some(value),
        Err(index) => {
            let mut sides = [None, None];
            sides[slot] = Some(value);
            store.insert(index, (key, sides));
        }
    }
}
