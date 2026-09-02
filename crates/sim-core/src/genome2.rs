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
    ACTIVATION_REGISTRY_VERSION, Activation, CHANNEL_REGISTRY_VERSION, NodeRole,
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
/// Tag 5, allocated by Phase 10. ADR-0013 reserved it and deferred it; the
/// developmental growth program ADR-0019 adopts is what it is for. Reserving
/// the tag rather than appending later is why a schema-2 genome and a
/// schema-3 genome share a decoder.
const TAG_REGULATORY: u8 = 5;
/// Tag 6, allocated by Phase 11's measurement work: the neutral marker locus
/// (`LocusKind::Marker`). Appended, never inserted, so every genome ever
/// written keeps its meaning and `structural_signature` keeps its value for
/// every locus type that already existed.
const TAG_MARKER: u8 = 6;

/// `Marker.flags` bits. Exactly one is defined: the neutral binary allele
/// that controls for `EDGE_FLAG_PLASTIC`. The mask exists so an unknown bit
/// is a decode refusal rather than a value that survives into a census.
pub const MARKER_FLAG_NEUTRAL: u8 = 1 << 0;
const MARKER_FLAG_MASK: u8 = MARKER_FLAG_NEUTRAL;

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

/// Mate-choice preference loci (Phase 14, ADR-0030): nine Trait-kind loci
/// in a reserved `trait_id` band, one per perception cue channel. They are
/// carried, inherited, recombined and point-mutated by the ordinary trait
/// machinery (which is what makes cue production and cue response
/// separable, recombining genes - social-organization review 4.4), and
/// expressed only by [`Genome2::express_preference`], only read when the
/// mate-choice gate is on - the carried-but-inert precedent thermal
/// preference, `PlasticityGenes` and `Regulatory` all set. The gene value
/// lives in [0, 1] like every trait gene and maps to a signed weight in
/// [-1, +1] at expression; 0.5 is therefore the neutral founder value, and
/// an all-neutral genome reproduces proximity pairing exactly.
pub const PREFERENCE_TRAIT_BASE: u16 = 100;
pub const PREFERENCE_CUE_COUNT: usize = 9;

/// Structural caps. Every one rejects deterministically and is counted;
/// none is ever silently exceeded.
///
/// **Restated from measurement (C9.8, D-078); no longer provisional.** The
/// measurement is `scripts/run-phase9-benchmarks.sh`, taken under the
/// confirmatory campaign's own mutation regime after 30,000 ticks:
///
/// - marginal cost of one structural locus: **44.4 bytes**; fixed cost of a
///   founder genome (header plus the trait block on both haplotypes):
///   **1,229 bytes**;
/// - `max_genome_bytes` of 16,384 therefore admits about **341 structural
///   loci** across both haplotypes;
/// - the evolved distribution never came close: median 3 nodes and 2 edges,
///   p99 of 6 nodes and 4 edges, maximum 7 nodes and 4 edges, largest genome
///   1,692 bytes - about a tenth of the byte cap;
/// - worst case at the byte cap is 32.8 MB of genome at the 2,000-organism
///   tier, against a measured 3.35 MB actual, and the format's own
///   256 MB stored cap.
///
/// **The provisional values were mutually inconsistent, which is the finding
/// that forced this restatement.** They were chosen one at a time, and three
/// of the four could never bind: `max_genome_bytes` runs out at ~341 loci,
/// while `max_nodes` (256, checked against both haplotypes so 512 node loci)
/// and `max_edges` (1,024, so 2,048 edge loci) would need 58 KB, and
/// `max_loci_per_chromosome` (512) exceeded the byte budget on its own. A
/// cap that cannot be reached is not a guard; it is a number that reads like
/// one. The rule adopted here is that **every cap must be individually
/// reachable within `max_genome_bytes`**, so each one is a real limit, while
/// the byte cap remains the joint budget.
///
/// The caps are still far above anything evolution produced - 160 nodes
/// against an observed maximum of 7 - so they are guards rather than a
/// selection pressure at campaign scale. **They are not validated for
/// flagship-scale runs**: Soak-30 is roughly 16,500 generations against the
/// 61 measured here, and duplication above the deletion rate is a growth
/// process with that much longer to act. Genome size is a structural
/// quantity Soak-30's stationarity criterion (D-055) must watch.
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
    /// The measured caps. Kept under the name `provisional` deliberately:
    /// every call site and every campaign that referenced it still does, and
    /// renaming it would make the restatement look like a new knob rather
    /// than a revision of the same one. See the type's documentation for the
    /// measurement each value comes from.
    pub fn provisional() -> Self {
        Self {
            max_chromosomes: 4,
            // Each reachable on its own within the 16,384-byte budget, which
            // admits ~341 structural loci across both haplotypes. `max_nodes`
            // and `max_edges` are checked against the total across both, so
            // 160 permits 320 loci of that kind - inside the budget, and 23x
            // and 40x the largest evolved value measured.
            max_loci_per_chromosome: 160,
            max_nodes: 160,
            max_edges: 160,
            // Reachable given `max_edges`, and 32x the observed maximum
            // in-degree.
            max_edges_per_node: 32,
            // The joint budget, and the only cap that binds first. Measured
            // affordable: 32.8 MB of genome at the 2,000-organism tier in the
            // worst case, against 3.35 MB actual and a 256 MB format cap.
            max_genome_bytes: 16_384,
            min_nodes: 2,
        }
    }
}

/// How many plasticity rule *forms* exist: `rule_id` 0 (static) through 4
/// (eligibility trace). Rule 5 (observational) needs the Phase 13 social
/// channel and is deliberately **not** counted, because a rule form that
/// cannot be evaluated must not be reachable by mutation.
///
/// Declared here rather than imported from `plasticity.rs` because the
/// genome layer must be able to reduce a stored `rule_id` whether or not the
/// evaluator is compiled in - but two registries that must agree forever and
/// are written down in two places is precisely the shape of the bug where a
/// mutation names a rule the evaluator does not have. So the agreement is a
/// compile error rather than a convention: see the `const _` below.
pub const PLASTICITY_RULE_COUNT: u8 = 5;

const _: () = assert!(
    PLASTICITY_RULE_COUNT == crate::plasticity::RULE_COUNT,
    "the genome's rule-id reduction and the evaluator's rule registry disagree, \
     so some genome would express a rule form that cannot be evaluated"
);

/// Plasticity genes, carried by every edge whether or not it is plastic.
///
/// Inherited, validated, dominance-expressed, and **behaviorally inert
/// until Phase 11** - exactly the pattern thermal preference and defense
/// tendency followed from Phase 2 to Phase 8. Occupying the space now means
/// enabling plasticity is a flag flip rather than a schema change.
///
/// **That claim was false until Phase 11 checked it.** The genes were
/// carried, inherited and validated, but `express_network` destructured the
/// edge locus with a `..` that dropped them, `ExpressedEdge` had no field to
/// put them in, no operator could move `eta` off zero, and no production
/// path anywhere set `EDGE_FLAG_PLASTIC`. Every one of those is a mechanical
/// gap rather than a design choice, and together they guaranteed the null
/// result Phase 11's own risk table names as its most likely failure - which
/// would have been read as a fact about selection rather than about the
/// gather.
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

    /// Every field reduced into range rather than rejected.
    ///
    /// **Called at expression time, not at decode**, and the reasoning is
    /// `develop::Regulatory::normalized`'s, which is already written down:
    /// these fields are *mutation targets*, and a `rule_id` that had to name
    /// a registry entry in order to decode would make most rule-id mutations
    /// lethal for a reason that has nothing to do with learning. Reduction
    /// keeps the genotype space total - every bit pattern names some rule.
    ///
    /// There is a second reason not to move it to decode, and it is the
    /// stronger one: a schema-2 genome storing `rule_id = 7` decodes as 7
    /// today, and normalizing at decode would make the same bytes mean 2.
    /// Every genome ever written is bytes we are not free to reinterpret.
    /// So the stored value is whatever was stored, and reduction happens on
    /// the way to the phenotype - where `develop::Regulatory` differs only
    /// because its normalization predates any stored genome.
    pub fn normalized(self) -> Self {
        // Destructured with no `..` (D-077): a field added to this struct
        // fails to compile here until it is given a range or an explicit
        // reason to have none, which is what stops the next field from
        // reaching the phenotype unbounded.
        let Self {
            rule_id,
            eta,
            coefficients,
            decay,
            modulator_node,
        } = self;
        Self {
            // The space is one wider than the base registry: rule 5 exists
            // behind the social gate (ADR-0029), and every value a pre-13
            // world can store is below the base count, so the wider modulus
            // is the identity on every allele in circulation.
            rule_id: rule_id % crate::plasticity::RULE_SPACE,
            eta: clamp_finite(eta, 0.0, 1.0),
            coefficients: [
                clamp_finite(coefficients[0], -1.0, 1.0),
                clamp_finite(coefficients[1], -1.0, 1.0),
                clamp_finite(coefficients[2], -1.0, 1.0),
                clamp_finite(coefficients[3], -1.0, 1.0),
            ],
            decay: clamp_finite(decay, 0.0, 1.0),
            // Nothing to reduce: a modulator naming a node that is not on
            // this haplotype is a `DanglingReference` at validation, which
            // is a refusal rather than a value to be repaired. Silently
            // rewriting it to 0 here would turn "this genome is malformed"
            // into "this edge is ungated" and hide the malformation.
            modulator_node,
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

/// `f32::clamp` **propagates NaN** rather than removing it, so a clamp alone
/// is not a bound. Decode already refuses a non-finite plasticity gene, so
/// this is a backstop and not a live path - but expression is the last point
/// before the value reaches the learning arithmetic and from there the
/// checksum, and a backstop that never fires is cheaper than a checksum that
/// has to be re-baselined.
fn clamp_finite(value: f32, low: f32, high: f32) -> f32 {
    if value.is_finite() {
        value.clamp(low, high)
    } else {
        0.0
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
    /// A morphological growth rule (Phase 10). Behaviourally inert unless the
    /// morphology section is enabled, following the precedent thermal
    /// preference and `PlasticityGenes` both set: carried, inherited,
    /// validated, and expressed only when its phase turns on.
    Regulatory { rule: crate::develop::Regulatory },
    /// A **neutral marker locus** (Phase 11): C11.2's drift control.
    ///
    /// C11.2 asks whether `eta` and the plastic-edge fraction shift *more
    /// than drift*. Drift here is not an analytic quantity: it depends on the
    /// realized population size, the variance in reproductive success, the
    /// linkage structure and the mutation regime, none of which is a constant
    /// a formula could be evaluated at. So the control is empirical - a locus
    /// that experiences every one of those forces and **none** of the
    /// selection, measured in the same run.
    ///
    /// For that to be a control rather than a decoration, three things have
    /// to hold, and each is enforced somewhere specific:
    ///
    /// - **Never expressed.** `express_network` ignores this arm alongside
    ///   `Trait` and `Regulatory`, `express_traits` matches only `Trait`, and
    ///   `develop::rules_of` matches only `Regulatory`. There is no field on
    ///   `ExpressedNetwork` for it to land in, so the guarantee is structural
    ///   rather than a promise.
    /// - **Inherited and recombined identically.** `meiosis.rs` is entirely
    ///   kind-agnostic: it walks `homology_id` and copies whole `Locus`
    ///   values, so a marker segregates and crosses over exactly as the edge
    ///   locus beside it does, with no code added anywhere.
    /// - **Mutated at a matched rate.** `structmut::point_mutate` gives this
    ///   arm the same seven-way target draw the `Edge` arm takes, moves
    ///   `value` on the draw that would have moved `eta`, and toggles
    ///   `MARKER_FLAG_NEUTRAL` on the draw that would have toggled
    ///   `EDGE_FLAG_PLASTIC`. Per locus picked, the two alleles see exactly
    ///   the mutational input the two quantities under test see.
    ///
    /// The alleles match their targets' *shape*, not only their rate.
    /// `value` is a `[0, 1]` scalar starting at 0.0 with `eta`'s clamp and
    /// `eta`'s delta, so its random walk is reflected at the same boundary.
    /// A marker starting at 0.5 would drift symmetrically while `eta`
    /// starting at 0.0 can only rise, and the comparison would be biased
    /// before any selection acted.
    Marker { value: f32, flags: u8 },
}

impl LocusKind {
    fn tag(&self) -> u8 {
        match self {
            LocusKind::Trait { .. } => TAG_TRAIT,
            LocusKind::Node { .. } => TAG_NODE,
            LocusKind::Edge { .. } => TAG_EDGE,
            LocusKind::IoBinding { .. } => TAG_IO_BINDING,
            LocusKind::Regulatory { .. } => TAG_REGULATORY,
            LocusKind::Marker { .. } => TAG_MARKER,
        }
    }

    fn payload_len(tag: u8) -> Option<usize> {
        Some(match tag {
            TAG_TRAIT => 2 + 4 + 4,
            TAG_NODE => 1 + 1 + 4 + 2,
            TAG_EDGE => 4 + 4 + 4 + 1 + PlasticityGenes::ENCODED_LEN,
            TAG_IO_BINDING => 4 + 2 + 4,
            TAG_REGULATORY => crate::develop::Regulatory::ENCODED_LEN,
            TAG_MARKER => 4 + 1,
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
                //
                // **Phase 11 deliberately did not change this**, and the
                // temptation was real: once an edge can evolve to be
                // plastic, "is it plastic" starts to look structural. It is
                // not admissible here for a reason that has nothing to do
                // with taste - every signature ever computed would move,
                // including the ones inside the Phase 9 fixture's lineage,
                // and two loci that are the same edge would stop aligning
                // because one of them learned. `PlasticityGenes` is dropped
                // by the `..` above for the same reason a weight is.
                hasher.update_u32(u32::from(flags & EDGE_FLAG_DELAYED));
            }
            LocusKind::IoBinding {
                node, channel_id, ..
            } => {
                hasher.update_u32(node);
                hasher.update_u32(u32::from(channel_id));
            }
            LocusKind::Regulatory { rule } => {
                // Every field is phenotype-relevant: a growth rule *is*
                // structure, unlike an edge weight.
                hasher.update_u32(u32::from(rule.condition_kind));
                hasher.update_u32(u32::from(rule.condition_op));
                hasher.update_u32(u32::from(rule.condition_param));
                hasher.update_u32(u32::from(rule.threshold));
                hasher.update_u32(u32::from(rule.action_kind));
                hasher.update_u32(u32::from(rule.action_type));
                hasher.update_u32(u32::from(rule.direction));
                hasher.update_u32(u32::from(rule.scale_milli));
            }
            // **Nothing beyond the tag.** Two markers at the same homology
            // slot are the same structure whatever their alleles say, exactly
            // as two edges are the same structure whatever their weights say.
            //
            // The flag is deliberately excluded even though it is a bit and
            // `delayed` is a bit that *is* included, and the reason is the
            // control: `EDGE_FLAG_PLASTIC` is excluded from the edge arm
            // above, so including the allele that controls for it would make
            // the marker behave differently under alignment than the thing it
            // is a control for. A marker whose neutral bit flipped would stop
            // aligning with its own ancestor, and the drift measurement would
            // be a measurement of the signature convention.
            LocusKind::Marker { .. } => {}
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

    /// Every neutral marker **allele** this genome carries, as
    /// `(homology_id, value, flags)`, haplotype 0 first and ascending within
    /// each haplotype.
    ///
    /// Observation only (ADR-0016), and deliberately **alleles rather than a
    /// blend**. Combining the two haplotypes is what expression does, and
    /// this locus has no expression - inventing one here would create the
    /// reader the type's documentation promises does not exist, and it would
    /// also throw away the thing a drift measurement most wants, which is
    /// heterozygosity. An analysis that wants a codominant mean can take it;
    /// the kernel does not take one for it.
    pub fn marker_alleles(&self) -> Vec<(u32, f32, u8)> {
        let mut out = Vec::new();
        for haplotype in &self.haplotypes {
            for locus in haplotype.chromosomes.iter().flatten() {
                if let LocusKind::Marker { value, flags } = locus.kind {
                    out.push((locus.homology_id, value, flags));
                }
            }
        }
        out
    }

    /// The smallest channel registry version that offers every channel this
    /// genome binds. 1 unless a binding names a version-2 channel.
    pub fn required_channel_registry_version(&self) -> u16 {
        let mut version = CHANNEL_REGISTRY_VERSION;
        for haplotype in &self.haplotypes {
            for chromosome in &haplotype.chromosomes {
                for locus in chromosome {
                    if let LocusKind::IoBinding { channel_id, .. } = locus.kind
                        && let Some(needed) = crate::registry::channel_version(channel_id)
                    {
                        version = version.max(needed);
                    }
                }
            }
        }
        version
    }

    /// Whether every binding names a channel registry `version` offers. What
    /// a world checks at construction and restore against the version it
    /// offers; a genome that fails it is refused, never trimmed.
    pub fn bindings_offered_by(&self, version: u16) -> bool {
        self.required_channel_registry_version() <= version
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
                        LocusKind::Regulatory { rule } => {
                            body.push(rule.condition_kind);
                            body.push(rule.condition_op);
                            body.push(rule.condition_param);
                            put_u16(&mut body, rule.threshold);
                            body.push(rule.action_kind);
                            body.push(rule.action_type);
                            body.push(rule.direction);
                            put_u16(&mut body, rule.scale_milli);
                        }
                        LocusKind::Marker { value, flags } => {
                            put_f32(&mut body, value);
                            body.push(flags);
                        }
                    }
                }
            }
        }

        let total_len = (HEADER_LEN + body.len() + 8) as u32;
        let mut out = Vec::with_capacity(total_len as usize);
        out.extend_from_slice(GENOME2_MAGIC);
        put_u16(&mut out, GENOME2_SCHEMA_VERSION);
        // The smallest registry version that offers every channel this
        // genome binds: 1 for every genome that existed before the artifact
        // half, so every byte written before it is written unchanged; 2 for
        // a genome bound to an artifact channel (ADR-0028 section 7).
        put_u16(&mut out, self.required_channel_registry_version());
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
        // Versions 1..=3 are all readable; the declared version is what
        // each binding is validated against, so a version-1 genome cannot
        // smuggle a version-2 or version-3 channel and a genome cannot
        // declare a version this build does not know.
        if registry != CHANNEL_REGISTRY_VERSION
            && registry != crate::registry::CHANNEL_REGISTRY_VERSION_ARTIFACT
            && registry != crate::registry::CHANNEL_REGISTRY_VERSION_SOCIAL
        {
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
                    loci.push(decode_locus(&mut reader, registry)?);
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
                    LocusKind::Regulatory { .. } => {
                        // A growth rule references no node and no channel, so
                        // there is nothing here that can dangle. Its fields
                        // are normalized at decode, so there is nothing out
                        // of range either. The homology block is the only
                        // invariant left to assert.
                        if locus.homology_id < STRUCTURAL_HOMOLOGY_BASE {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_REGULATORY,
                            });
                        }
                    }
                    LocusKind::Marker { .. } => {
                        // A marker references nothing, so nothing can dangle;
                        // its alleles are range-checked at decode. It sits in
                        // the structural block rather than the trait block
                        // because the trait block is keyed by `trait_id` and
                        // has no free slot, and because a marker must be
                        // reachable by duplication and deletion on the same
                        // terms an edge is - which run only over structural
                        // loci.
                        if locus.homology_id < STRUCTURAL_HOMOLOGY_BASE {
                            return Err(Genome2Error::HomologyBlockViolation {
                                homology_id: locus.homology_id,
                                tag: TAG_MARKER,
                            });
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

fn decode_locus(reader: &mut Reader<'_>, registry: u16) -> Result<Locus, Genome2Error> {
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
            let behavioral = usize::from(trait_id) < crate::genome::TRAIT_COUNT;
            let preference = (PREFERENCE_TRAIT_BASE
                ..PREFERENCE_TRAIT_BASE + PREFERENCE_CUE_COUNT as u16)
                .contains(&trait_id);
            // The behavioral block and the Phase 14 preference band are the
            // only legal trait ids; everything between and beyond stays
            // refused, so widening the bound accepted strictly more ids
            // without reinterpreting any byte a genome ever stored.
            if !behavioral && !preference {
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
            if !crate::registry::channel_offered(channel_id, registry) {
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
        TAG_REGULATORY => {
            // Stored **normalized**, so nothing downstream ever sees an
            // out-of-range code. Reduction rather than rejection is
            // deliberate and is argued at `develop::Regulatory::normalized`:
            // these fields are mutation targets, and rejecting out-of-range
            // bytes would make most mutations of a growth rule lethal for
            // reasons unrelated to morphology.
            let rule = crate::develop::Regulatory {
                condition_kind: reader.u8()?,
                condition_op: reader.u8()?,
                condition_param: reader.u8()?,
                threshold: reader.u16()?,
                action_kind: reader.u8()?,
                action_type: reader.u8()?,
                direction: reader.u8()?,
                scale_milli: reader.u16()?,
            };
            LocusKind::Regulatory {
                rule: rule.normalized(),
            }
        }
        TAG_MARKER => {
            let value = reader.f32()?;
            let flags = reader.u8()?;
            // **Rejected, not reduced**, which is the opposite of the
            // regulatory arm above and deliberately so. A growth rule's
            // fields are discrete codes where every bit pattern has to name
            // *some* rule or most mutations would be lethal for reasons
            // unrelated to morphology. A marker allele is a bounded scalar
            // exactly like `eta`, whose arm four lines up refuses out-of-range
            // values, and the marker has to be refused on the same terms or
            // its mutational neighbourhood would differ from `eta`'s at the
            // clamp - which is precisely where the reflected random walk
            // spends its time.
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(Genome2Error::ValueOutOfRange("marker value"));
            }
            if flags & !MARKER_FLAG_MASK != 0 {
                return Err(Genome2Error::ValueOutOfRange("marker flags"));
            }
            LocusKind::Marker { value, flags }
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
    /// Combined by the policy recorded on [`Genome2::express_network`], and
    /// [`PlasticityGenes::normalized`] on the way out, so nothing downstream
    /// ever sees a rule code the registry does not have. Present whether or
    /// not `plastic` is set: `plastic` says the edge learns, these say how.
    pub plasticity: PlasticityGenes,
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
/// `(source, target, weight, flags, plasticity, dominance)`
type EdgeAllele = (u32, u32, f32, u8, PlasticityGenes, f32);
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

/// Combine two edges' plasticity genes. The policy is recorded on
/// [`Genome2::express_network`] next to the flag policy it extends.
fn blend_plasticity(
    left: PlasticityGenes,
    d0: f32,
    right: PlasticityGenes,
    d1: f32,
) -> PlasticityGenes {
    // Both sides destructured with no `..` (D-077), so a field added to
    // `PlasticityGenes` fails to compile here until it is assigned to the
    // continuous half or the discrete half. A new field silently inheriting
    // haplotype 0's value because it fell through a `..` is exactly the
    // defect this whole unit exists to repair.
    let PlasticityGenes {
        rule_id,
        eta: eta0,
        coefficients: coefficients0,
        decay: decay0,
        modulator_node,
    } = left;
    let PlasticityGenes {
        rule_id: _,
        eta: eta1,
        coefficients: coefficients1,
        decay: decay1,
        modulator_node: _,
    } = right;
    let mut coefficients = [0.0_f32; 4];
    for index in 0..4 {
        coefficients[index] =
            blend_by_dominance(coefficients0[index], d0, coefficients1[index], d1);
    }
    PlasticityGenes {
        rule_id,
        eta: blend_by_dominance(eta0, d0, eta1, d1),
        coefficients,
        decay: blend_by_dominance(decay0, d0, decay1, d1),
        modulator_node,
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
                    && usize::from(trait_id) < crate::genome::TRAIT_COUNT
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

    /// Express the mate-choice preference band: one signed weight per
    /// perception cue channel, blended across haplotypes by dominance
    /// exactly as behavioral traits are, mapped from the [0, 1] gene space
    /// to [-1, +1]. A cue whose locus is absent weighs zero, so a genome
    /// that never carried the band - every genome from before Phase 14 -
    /// expresses the neutral preference and pairs by proximity exactly as
    /// it always did.
    pub fn express_preference(&self) -> [f32; PREFERENCE_CUE_COUNT] {
        let mut left: [Option<(f32, f32)>; PREFERENCE_CUE_COUNT] = [None; PREFERENCE_CUE_COUNT];
        let mut right = left;
        for (slot, store) in [(0_usize, &mut left), (1, &mut right)] {
            for locus in self.haplotypes[slot].chromosomes.iter().flatten() {
                if let LocusKind::Trait {
                    trait_id,
                    value,
                    dominance,
                } = locus.kind
                    && (PREFERENCE_TRAIT_BASE
                        ..PREFERENCE_TRAIT_BASE + PREFERENCE_CUE_COUNT as u16)
                        .contains(&trait_id)
                {
                    store[usize::from(trait_id - PREFERENCE_TRAIT_BASE)] =
                        Some((value, dominance));
                }
            }
        }
        let mut out = [0.0_f32; PREFERENCE_CUE_COUNT];
        for index in 0..PREFERENCE_CUE_COUNT {
            let gene = match (left[index], right[index]) {
                (Some((v0, d0)), Some((v1, d1))) => blend_by_dominance(v0, d0, v1, d1),
                (Some((value, _)), None) | (None, Some((value, _))) => value,
                (None, None) => 0.5,
            };
            out[index] = gene.clamp(0.0, 1.0) * 2.0 - 1.0;
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
    ///
    /// **Plasticity gene combination is policy on the same terms** (Phase
    /// 11), and it splits by whether the field has a meaningful midpoint:
    ///
    /// - the continuous fields - `eta`, the four coefficients, `decay` - go
    ///   through `blend_by_dominance`, which at the shipped structural
    ///   dominance of 1.0 is the plain mean, i.e. codominance. That is what
    ///   every other continuous field in this function already does, and a
    ///   learning rate half way between the parents' is a learning rate;
    /// - the discrete fields - `rule_id` and `modulator_node` - take the
    ///   **haplotype-0** value, exactly as `role`, `activation_id`, `source`
    ///   and `target` already do. Averaging two rule codes would name a
    ///   third rule neither parent carried, and averaging two node homology
    ///   IDs would name a node that may not exist. Haplotype 0 is the
    ///   lower-ID parent's slot (determinism Rule 3), so the choice is a
    ///   fixed function of the genome rather than of traversal order.
    ///
    /// The combined genes are then [`PlasticityGenes::normalized`], so a
    /// stored `rule_id` outside the registry reaches the phenotype reduced
    /// rather than reaching it at all. Normalizing here rather than at
    /// decode is argued at that method.
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
                    // Neither traits nor growth rules are part of the
                    // controller: one is a scalar phenotype and the other is
                    // morphology.
                    //
                    // **The marker joins them, and this line is the whole of
                    // "never expressed".** There is no `ExpressedMarker`, no
                    // field on `ExpressedNetwork`, and no other reader: a
                    // future change that wanted to express one would have to
                    // invent all three, which is a visible act rather than a
                    // `..` quietly picking a value up.
                    LocusKind::Trait { .. }
                    | LocusKind::Regulatory { .. }
                    | LocusKind::Marker { .. } => {}
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
                    // Destructured with no `..`. The `..` that used to be
                    // here is finding 2 of Phase 11's audit: it dropped
                    // `plasticity` on the floor, so the genes were
                    // inherited, validated and never expressed, and no
                    // amount of selection on them could have done anything.
                    LocusKind::Edge {
                        source,
                        target,
                        weight,
                        flags,
                        plasticity,
                    } => {
                        upsert(
                            &mut edge_pairs,
                            locus.homology_id,
                            slot,
                            (source, target, weight, flags, plasticity, dominance),
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
            let (source, target, weight, flags, plasticity) = match (sides[0], sides[1]) {
                (Some((s0, t0, w0, f0, p0, d0)), Some((_, _, w1, f1, p1, d1))) => {
                    // `disabled` needs both; `plastic` and `delayed` need
                    // either. Recorded policy, not an accident of `&`/`|`.
                    let disabled = f0 & f1 & EDGE_FLAG_DISABLED;
                    let permissive = (f0 | f1) & (EDGE_FLAG_PLASTIC | EDGE_FLAG_DELAYED);
                    (
                        s0,
                        t0,
                        blend_by_dominance(w0, d0, w1, d1),
                        disabled | permissive,
                        blend_plasticity(p0, d0, p1, d1),
                    )
                }
                (Some((s, t, w, f, p, _)), None) | (None, Some((s, t, w, f, p, _))) => {
                    (s, t, w, f, p)
                }
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
                plasticity: plasticity.normalized(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> GenomeCaps {
        GenomeCaps::provisional()
    }

    fn founder() -> Genome2 {
        crate::structmut::minimal_founder(&[0.5; crate::genome::TRAIT_COUNT])
    }

    /// Deliberately unlike `inert()` in **every** field, including a
    /// `rule_id` outside the registry and a `modulator_node` that names a
    /// real node, so that "the genes survived" cannot be satisfied by a
    /// function that returns zeros. `seed` shifts every value, so two calls
    /// never collide.
    fn loud(seed: f32, rule_id: u8, modulator_node: u32) -> PlasticityGenes {
        PlasticityGenes {
            rule_id,
            eta: 0.25 + seed,
            coefficients: [0.5 + seed, -0.5 + seed, 0.125 + seed, -0.25 + seed],
            decay: 0.0625 + seed,
            modulator_node,
        }
    }

    /// The `homology_id`s `minimal_founder` gives its two edges and its
    /// hidden node. Hard-coded rather than searched, so a change to the
    /// founder shows up as a failing test rather than as a test that
    /// quietly stops looking at an edge.
    const FOUNDER_EDGE_A: u32 = STRUCTURAL_HOMOLOGY_BASE + 4_000;
    const FOUNDER_HIDDEN: u32 = STRUCTURAL_HOMOLOGY_BASE + 2_000;

    fn set_plasticity(genome: &mut Genome2, slot: usize, edge_id: u32, genes: PlasticityGenes) {
        for locus in genome.haplotypes[slot].chromosomes.iter_mut().flatten() {
            if locus.homology_id == edge_id
                && let LocusKind::Edge { plasticity, .. } = &mut locus.kind
            {
                *plasticity = genes;
                return;
            }
        }
        panic!("no edge locus {edge_id} on haplotype {slot}");
    }

    fn expressed_edge(genome: &Genome2, edge_id: u32) -> ExpressedEdge {
        *genome
            .express_network()
            .edges
            .iter()
            .find(|edge| edge.homology_id == edge_id)
            .expect("the edge is expressed")
    }

    #[test]
    fn plasticity_genes_reach_the_expressed_edge_instead_of_being_dropped() {
        // Finding 1 of the Phase 11 audit, end to end. The gather used to
        // destructure the edge locus with a `..` that discarded
        // `plasticity`, and `ExpressedEdge` had nowhere to put it, so the
        // genes were inherited and validated for two phases while being
        // structurally incapable of affecting anything.
        let genes = loud(0.0, 3, FOUNDER_HIDDEN);
        assert_ne!(
            genes,
            PlasticityGenes::inert(),
            "the probe must differ from inert, or this test passes on a stub"
        );
        let mut subject = founder();
        set_plasticity(&mut subject, 0, FOUNDER_EDGE_A, genes);
        set_plasticity(&mut subject, 1, FOUNDER_EDGE_A, genes);
        subject.validate_structure(&caps()).expect("valid");
        assert_eq!(expressed_edge(&subject, FOUNDER_EDGE_A).plasticity, genes);
        // The untouched edge still expresses inert genes, so the assertion
        // above is about this edge rather than about a global default.
        assert_eq!(
            expressed_edge(&subject, STRUCTURAL_HOMOLOGY_BASE + 5_000).plasticity,
            PlasticityGenes::inert()
        );
    }

    #[test]
    fn a_heterozygote_blends_the_continuous_genes_and_takes_haplotype_zeros_discrete_ones() {
        // The recorded combination policy. Every continuous field is given
        // two *different* values so the mean is neither of them, and the two
        // discrete fields are given different values so "takes haplotype 0"
        // discriminates rather than being satisfied by chance.
        let left = loud(0.0, 1, FOUNDER_HIDDEN);
        let right = loud(0.5, 4, 0);
        let mut subject = founder();
        set_plasticity(&mut subject, 0, FOUNDER_EDGE_A, left);
        set_plasticity(&mut subject, 1, FOUNDER_EDGE_A, right);
        subject.validate_structure(&caps()).expect("valid");
        let expressed = expressed_edge(&subject, FOUNDER_EDGE_A).plasticity;

        // Structural dominance is 1.0 on both sides, so the blend is the
        // plain mean - codominance, exactly as weight and bias already do.
        assert_eq!(expressed.eta, 0.5 * (left.eta + right.eta));
        assert_eq!(expressed.decay, 0.5 * (left.decay + right.decay));
        for index in 0..4 {
            let mean = 0.5 * (left.coefficients[index] + right.coefficients[index]);
            assert_eq!(expressed.coefficients[index], mean);
            assert_ne!(
                mean, left.coefficients[index],
                "coefficient {index} would pass by taking haplotype 0"
            );
            assert_ne!(
                mean, right.coefficients[index],
                "coefficient {index} would pass by taking haplotype 1"
            );
        }
        // Discrete: haplotype 0 wins outright. Averaging `rule_id` would
        // name a third rule neither parent carried; averaging two node IDs
        // would name a node that need not exist.
        assert_eq!(expressed.rule_id, left.rule_id);
        assert_ne!(expressed.rule_id, right.rule_id);
        assert_eq!(expressed.modulator_node, left.modulator_node);
        assert_ne!(expressed.modulator_node, right.modulator_node);
    }

    #[test]
    fn a_hemizygous_edge_expresses_its_single_allele_unblended() {
        // The fresh-duplication case. An edge present on one haplotype only
        // must not be blended toward an allele that does not exist, or a new
        // duplicate would silently express half of whatever it carried.
        let genes = loud(0.0, 2, FOUNDER_HIDDEN);
        let mut subject = founder();
        set_plasticity(&mut subject, 0, FOUNDER_EDGE_A, genes);
        subject.haplotypes[1].chromosomes[0].retain(|locus| locus.homology_id != FOUNDER_EDGE_A);
        subject.validate_structure(&caps()).expect("valid");
        assert_eq!(expressed_edge(&subject, FOUNDER_EDGE_A).plasticity, genes);
    }

    #[test]
    fn a_rule_id_outside_the_registry_is_reduced_at_expression_and_not_at_decode() {
        // Both halves of the normalization policy, because each one alone is
        // satisfiable by the wrong implementation.
        //
        // Half one: the *stored* byte survives a codec round trip unchanged.
        // Normalizing at decode would make a genome that stores 7 start
        // meaning 2, which reinterprets every schema-2 genome ever written.
        //
        // Half two: the *expressed* value is inside the registry, so nothing
        // downstream can be handed a rule form the evaluator does not have.
        let stored = 7_u8;
        assert!(
            stored >= PLASTICITY_RULE_COUNT,
            "the probe must be outside the registry or this test asserts nothing"
        );
        let mut subject = founder();
        let genes = PlasticityGenes {
            rule_id: stored,
            ..PlasticityGenes::inert()
        };
        set_plasticity(&mut subject, 0, FOUNDER_EDGE_A, genes);
        set_plasticity(&mut subject, 1, FOUNDER_EDGE_A, genes);

        let decoded = Genome2::decode(&subject.encode(), &caps()).expect("decodes");
        let stored_again = decoded
            .loci()
            .filter_map(|locus| match locus.kind {
                LocusKind::Edge { plasticity, .. } if locus.homology_id == FOUNDER_EDGE_A => {
                    Some(plasticity.rule_id)
                }
                _ => None,
            })
            .collect::<Vec<u8>>();
        assert_eq!(
            stored_again,
            vec![stored, stored],
            "decode reinterpreted it"
        );
        assert_eq!(
            expressed_edge(&decoded, FOUNDER_EDGE_A).plasticity.rule_id,
            stored % crate::plasticity::RULE_SPACE,
            "the reduction is over the rule SPACE (six values, ADR-0029), \
             not the base registry count"
        );
    }

    #[test]
    fn normalization_removes_a_non_finite_gene_rather_than_propagating_it() {
        // `f32::clamp` propagates NaN, so a clamp on its own is not a bound.
        // Decode refuses a non-finite gene, so this is a backstop and cannot
        // be reached through the codec - which is why it is asserted on the
        // function directly rather than through a genome.
        let poisoned = PlasticityGenes {
            rule_id: 0,
            eta: f32::NAN,
            coefficients: [f32::INFINITY, f32::NEG_INFINITY, f32::NAN, 5.0],
            decay: f32::NEG_INFINITY,
            modulator_node: 0,
        }
        .normalized();
        assert_eq!(poisoned.eta, 0.0);
        assert_eq!(poisoned.decay, 0.0);
        assert_eq!(poisoned.coefficients, [0.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn a_genome_carrying_non_inert_plasticity_genes_round_trips_byte_for_byte() {
        // The wire codec needed no change for Phase 11 -
        // `PlasticityGenes::ENCODED_LEN` was already in the edge payload -
        // but "already round-trips" was an untested claim about a field that
        // had only ever been written as zeros. Zeros round-trip through a
        // codec that drops the field entirely.
        let mut subject = founder();
        set_plasticity(
            &mut subject,
            0,
            FOUNDER_EDGE_A,
            loud(0.0, 9, FOUNDER_HIDDEN),
        );
        set_plasticity(&mut subject, 1, FOUNDER_EDGE_A, loud(0.125, 4, 0));
        // Every field at its bound, so a codec that silently truncated a
        // coefficient would show up.
        set_plasticity(
            &mut subject,
            0,
            STRUCTURAL_HOMOLOGY_BASE + 5_000,
            PlasticityGenes {
                rule_id: u8::MAX,
                eta: 1.0,
                coefficients: [1.0, -1.0, 1.0, -1.0],
                decay: 1.0,
                modulator_node: FOUNDER_HIDDEN,
            },
        );
        subject.validate_structure(&caps()).expect("valid");

        let bytes = subject.encode();
        let decoded = Genome2::decode(&bytes, &caps()).expect("decodes");
        assert_eq!(decoded, subject);
        assert_eq!(decoded.encode(), bytes);
        // Non-vacuity: the genome under test is not the inert one.
        assert_ne!(subject, founder());
    }

    #[test]
    fn plasticity_is_deliberately_absent_from_the_structural_signature() {
        // Guards an instruction that is easy to undo by accident. Once an
        // edge can evolve to be plastic, "is it plastic" starts to look
        // structural - and admitting it would move every signature ever
        // computed and stop two loci that are the same edge from aligning
        // during meiosis because one of them learned.
        let plain = Locus {
            homology_id: FOUNDER_EDGE_A,
            gene_lineage_id: 1,
            mutation_event_id: 2,
            kind: LocusKind::Edge {
                source: 10,
                target: 20,
                weight: 1.0,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        };
        let with_genes = Locus {
            kind: LocusKind::Edge {
                source: 10,
                target: 20,
                weight: 1.0,
                flags: 0,
                plasticity: loud(0.0, 3, 20),
            },
            ..plain
        };
        let flagged = Locus {
            kind: LocusKind::Edge {
                source: 10,
                target: 20,
                weight: 1.0,
                flags: EDGE_FLAG_PLASTIC,
                plasticity: PlasticityGenes::inert(),
            },
            ..plain
        };
        assert_eq!(
            plain.structural_signature(),
            with_genes.structural_signature()
        );
        assert_eq!(plain.structural_signature(), flagged.structural_signature());
        // ...and the delay bit still *is* structural, so the assertions
        // above are about plasticity rather than about a signature that
        // ignores flags altogether.
        let delayed = Locus {
            kind: LocusKind::Edge {
                source: 10,
                target: 20,
                weight: 1.0,
                flags: EDGE_FLAG_DELAYED,
                plasticity: PlasticityGenes::inert(),
            },
            ..plain
        };
        assert_ne!(plain.structural_signature(), delayed.structural_signature());
    }
}
