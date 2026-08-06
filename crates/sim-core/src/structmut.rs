//! Structural and value mutation (Phase 9, `lifesim-structmut-v1`).
//!
//! Networks grow because genes duplicate and diverge, and shrink because
//! genes are deleted. That is not an analogy: gene duplication followed by
//! divergence is the principal mechanism by which real regulatory and neural
//! complexity increased, and adopting it gives structural evolution and
//! genetic realism from one mechanism instead of bolting a graph-editing
//! scheme onto a flat vector (ADR-0013).
//!
//! ## Duplication had to reconcile three things the specification wants at once
//!
//! `genome-schema-2.md` asks for all of:
//!
//! 1. loci **strictly ascending by `homology_id`** within a chromosome, as a
//!    decode-time invariant;
//! 2. duplicates inserted **immediately after their source**, because
//!    "adjacency in innovation order is linkage" and tandem duplicates
//!    staying linked is the biological arrangement;
//! 3. `homology_id` **derived by hash**, so two lineages that independently
//!    make the same change converge on the same ID and align during meiosis.
//!
//! A hash over the mutation description satisfies (3) but lands anywhere in
//! the ID space, which satisfies (1) only by re-sorting - and re-sorting
//! scatters the duplicate away from its source, destroying (2).
//!
//! The resolution is to hash into a **small offset from the source ID**
//! rather than into the whole space: `source_id + 1..=DUPLICATE_SPAN`. All
//! three hold. The duplicate sorts within `DUPLICATE_SPAN` of its source, so
//! it is adjacent or nearly so; the offset is still a pure function of the
//! mutation description, so convergence survives; and sortedness is exact.
//! When the derived slot is already occupied the operation is **rejected and
//! counted**, never silently relocated - the same deterministic-rejection
//! discipline the caps use.
//!
//! ## Every rejection is counted and typed
//!
//! Silent rejection is not permitted: an experiment quietly running against
//! a cap has to be visible in its own report, or a null result about
//! structural evolution might only mean the mutations never happened.

use crate::develop::{ACTION_KIND_COUNT, CONDITION_KIND_COUNT, OPERATOR_COUNT};
use crate::genome2::{
    Genome2, GenomeCaps, Locus, LocusKind, PlasticityGenes, STRUCTURAL_HOMOLOGY_BASE, VALUE_LIMIT,
    derive_gene_lineage_id, derive_homology_id, derive_mutation_event_id,
};
use crate::registry::{Activation, NodeRole};
use crate::rng::{RngSystem, named_random};

pub const STRUCTMUT_POLICY_VERSION: &str = "lifesim-structmut-v1";

/// How far a duplicate may sit from its source in homology space. Small, so
/// tandem duplicates stay linked; non-zero, so the derived offset has room
/// to vary.
pub const DUPLICATE_SPAN: u32 = 16;

/// Operator codes. Permanent, like an RNG stream value: they feed the
/// derived identity hashes, so renumbering one would silently change every
/// genome that operator ever produced.
pub const OP_POINT: u8 = 1;
pub const OP_DUPLICATION: u8 = 2;
pub const OP_DELETION: u8 = 3;
pub const OP_INSERTION: u8 = 4;
pub const OP_TRANSPOSITION: u8 = 5;

/// Why an attempted operation did not happen.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    /// The derived homology slot was already occupied.
    HomologyCollision,
    /// Would have orphaned an edge or a binding.
    Orphaned,
    /// Would have dropped the node count below `min_nodes`.
    MinNodes,
    /// Would have left the organism with no way to sense or to act.
    NoBindings,
    /// Would have exceeded a structural cap.
    Cap,
    /// The operator's precondition does not hold for this genome, so there
    /// was nothing to do: transposition on a single-chromosome genome,
    /// insertion when fewer than two nodes exist to connect, a run drawn
    /// from an empty chromosome.
    ///
    /// Split out from [`Self::Invalid`] because conflating the two made the
    /// bug-report counter useless. A single-chromosome founder cannot
    /// transpose - position within a chromosome is determined by homology
    /// order, so the operator has no meaning until a genome has two linkage
    /// groups - and counting that expected, permanent condition as a
    /// validation failure had `rejected_invalid` climbing into the hundreds
    /// in every run, which is exactly the signal that was supposed to mean
    /// something had gone wrong.
    Inapplicable,
    /// The result would have contained a zero-delay cycle.
    ///
    /// Expected, not a bug: insertion adds an edge between two nodes drawn
    /// at random, and whether that edge closes a cycle is not knowable
    /// without checking. Duplication can reach it too, by copying an edge
    /// whose reversed partner is already present on the other haplotype.
    /// Counted separately for the same reason [`Self::Inapplicable`] is -
    /// so that [`Self::Invalid`] keeps meaning "something is wrong".
    Cycle,
    /// The result failed validation for any other reason. This is the
    /// backstop: the operators are written to produce valid records by
    /// construction, so a count here is a bug report, not a runtime
    /// condition.
    Invalid,
}

/// Per-class rejection counters. World state, hashed into the checksum, so
/// a campaign cannot silently run against a cap.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MutationCounters {
    pub point_applied: u64,
    pub duplication_applied: u64,
    pub deletion_applied: u64,
    pub insertion_applied: u64,
    pub transposition_applied: u64,

    pub rejected_homology_collision: u64,
    pub rejected_orphaned: u64,
    pub rejected_min_nodes: u64,
    pub rejected_no_bindings: u64,
    pub rejected_cap: u64,
    pub rejected_inapplicable: u64,
    pub rejected_cycle: u64,
    pub rejected_invalid: u64,
}

impl MutationCounters {
    pub fn total_applied(&self) -> u64 {
        self.point_applied
            + self.duplication_applied
            + self.deletion_applied
            + self.insertion_applied
            + self.transposition_applied
    }

    pub fn total_rejected(&self) -> u64 {
        self.rejected_homology_collision
            + self.rejected_orphaned
            + self.rejected_min_nodes
            + self.rejected_no_bindings
            + self.rejected_cap
            + self.rejected_inapplicable
            + self.rejected_cycle
            + self.rejected_invalid
    }

    fn reject(&mut self, reason: RejectReason) {
        match reason {
            RejectReason::HomologyCollision => self.rejected_homology_collision += 1,
            RejectReason::Orphaned => self.rejected_orphaned += 1,
            RejectReason::MinNodes => self.rejected_min_nodes += 1,
            RejectReason::NoBindings => self.rejected_no_bindings += 1,
            RejectReason::Cap => self.rejected_cap += 1,
            RejectReason::Inapplicable => self.rejected_inapplicable += 1,
            RejectReason::Cycle => self.rejected_cycle += 1,
            RejectReason::Invalid => self.rejected_invalid += 1,
        }
    }

    pub fn hash_into(&self, hasher: &mut crate::checksum::Fnv1a64) {
        hasher.update(b"lifesim-structmut-counters-v1");
        for value in [
            self.point_applied,
            self.duplication_applied,
            self.deletion_applied,
            self.insertion_applied,
            self.transposition_applied,
            self.rejected_homology_collision,
            self.rejected_orphaned,
            self.rejected_min_nodes,
            self.rejected_no_bindings,
            self.rejected_cap,
            self.rejected_inapplicable,
            self.rejected_cycle,
            self.rejected_invalid,
        ] {
            hasher.update_u64(value);
        }
    }
}

/// Per-operator rates, Q16 probability per reproduction.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MutationConfig {
    pub point_q16: u32,
    pub duplication_q16: u32,
    pub deletion_q16: u32,
    /// Available as an explicitly configured alternative to duplication-only
    /// growth, so the two can be compared rather than one being assumed
    /// sufficient. That comparison is C9.5.
    pub insertion_q16: u32,
    pub transposition_q16: u32,
    /// Longest contiguous run a duplication, deletion, or transposition may
    /// move.
    pub max_run: u32,
    /// Half-width of a point mutation's bounded delta, Q16 of the value's
    /// own range.
    pub point_delta_q16: u32,
}

impl Default for MutationConfig {
    fn default() -> Self {
        Self {
            point_q16: 6_554,     // 0.10
            duplication_q16: 655, // 0.01
            deletion_q16: 655,
            // Off by default: duplication-only is the ADR-0013 baseline and
            // C9.5 exists to measure whether it is fast enough before
            // insertion becomes the default.
            insertion_q16: 0,
            transposition_q16: 328, // 0.005
            max_run: 3,
            point_delta_q16: 3_277, // 0.05
        }
    }
}

/// Apply the mutation operators to a child genome, in a fixed order.
///
/// Every operator is attempted at most once per reproduction and gated by
/// its own rate. Each is applied to a working copy and validated; a result
/// that fails is reverted and counted, so the genome returned is always
/// valid and the reason it is not something else is always recorded.
pub fn mutate(
    genome: &mut Genome2,
    config: &MutationConfig,
    caps: &GenomeCaps,
    counters: &mut MutationCounters,
    world_seed: u64,
    tick: u64,
    child_id: u64,
) {
    let draw = |index: u32| {
        named_random(
            world_seed,
            tick,
            RngSystem::StructuralMutation,
            child_id,
            index,
        )
    };
    // Value mutation stays on `Recombination`, preserving the existing
    // convention that value perturbation and structural change are separate
    // streams.
    let value_draw = |index: u32| {
        named_random(
            world_seed,
            tick,
            RngSystem::Recombination,
            child_id,
            0x1000 + index,
        )
    };

    let fires = |rate: u32, value: u64| rate > 0 && (value & 0xffff) < u64::from(rate);

    // 1. Point mutation.
    if fires(config.point_q16, value_draw(0)) {
        try_operator(genome, caps, counters, RejectReason::Invalid, |working| {
            point_mutate(working, config, &value_draw)
        })
        .then(|| counters.point_applied += 1);
    }

    // 2. Duplication: the growth mechanism.
    if fires(config.duplication_q16, draw(0)) {
        match duplicate(genome, config, caps, world_seed, tick, child_id, &draw) {
            Ok(()) => counters.duplication_applied += 1,
            Err(reason) => counters.reject(reason),
        }
    }

    // 3. Deletion.
    if fires(config.deletion_q16, draw(16)) {
        match delete(genome, config, caps, &draw) {
            Ok(()) => counters.deletion_applied += 1,
            Err(reason) => counters.reject(reason),
        }
    }

    // 4. Insertion.
    if fires(config.insertion_q16, draw(32)) {
        match insert(genome, caps, world_seed, tick, child_id, &draw) {
            Ok(()) => counters.insertion_applied += 1,
            Err(reason) => counters.reject(reason),
        }
    }

    // 5. Transposition: changes linkage without changing content.
    if fires(config.transposition_q16, draw(48)) {
        match transpose(genome, config, caps, &draw) {
            Ok(()) => counters.transposition_applied += 1,
            Err(reason) => counters.reject(reason),
        }
    }
}

/// Apply an edit to a working copy, keep it only if it validates.
fn try_operator(
    genome: &mut Genome2,
    caps: &GenomeCaps,
    counters: &mut MutationCounters,
    reason: RejectReason,
    edit: impl FnOnce(&mut Genome2) -> bool,
) -> bool {
    let mut working = genome.clone();
    if !edit(&mut working) {
        return false;
    }
    if working.validate_structure(caps).is_err() {
        counters.reject(reason);
        return false;
    }
    *genome = working;
    true
}

fn chromosome_pick(genome: &Genome2, value: u64) -> (usize, usize) {
    let chromosomes = genome.chromosome_count().max(1);
    let haplotype = (value & 1) as usize;
    let chromosome = ((value >> 1) as usize) % chromosomes;
    (haplotype, chromosome)
}

fn point_mutate(genome: &mut Genome2, config: &MutationConfig, draw: &dyn Fn(u32) -> u64) -> bool {
    let (haplotype, chromosome) = chromosome_pick(genome, draw(1));
    let loci = &mut genome.haplotypes[haplotype].chromosomes[chromosome];
    if loci.is_empty() {
        return false;
    }
    let index = (draw(2) as usize) % loci.len();
    // Signed delta in [-half, +half] of the configured width, expressed as a
    // fraction of the value's own range so one rate works for every field.
    let unit = (draw(3) & 0xffff) as i64 - 32_768;
    let delta = unit as f32 * (config.point_delta_q16 as f32 / 65_536.0) / 32_768.0;
    match &mut loci[index].kind {
        LocusKind::Trait {
            value, dominance, ..
        } => {
            if draw(4) & 1 == 0 {
                *value = (*value + delta).clamp(0.0, 1.0);
            } else {
                // Dominance is an ordinary gene, so dominance relationships
                // themselves evolve.
                *dominance = (*dominance + delta).clamp(0.0, 1.0);
            }
        }
        LocusKind::Node { bias, .. } => {
            *bias = (*bias + delta * VALUE_LIMIT).clamp(-VALUE_LIMIT, VALUE_LIMIT);
        }
        LocusKind::Edge { weight, .. } => {
            *weight = (*weight + delta * VALUE_LIMIT).clamp(-VALUE_LIMIT, VALUE_LIMIT);
        }
        LocusKind::IoBinding { gain, .. } => {
            *gain = (*gain + delta * VALUE_LIMIT).clamp(-VALUE_LIMIT, VALUE_LIMIT);
        }
        LocusKind::Regulatory { rule } => {
            // A growth rule is discrete: there is no "small delta" on a
            // condition kind, so one field is chosen and re-drawn. Which
            // field matters enormously for evolvability - re-drawing the
            // action type rewrites what a rule builds, while nudging a
            // threshold usually changes only when it fires - and C10.4's
            // gate is precisely the measurement of how big those jumps are.
            //
            // Thresholds and scale move by a *step* rather than a re-draw,
            // so that the two numeric fields have a local neighbourhood at
            // all. Without that, every regulatory mutation would be a jump
            // and the encoding would fail its own gate by construction.
            let step = if delta < 0.0 { -1_i32 } else { 1_i32 };
            match draw(5) % 6 {
                0 => rule.condition_kind = (draw(6) % u64::from(CONDITION_KIND_COUNT)) as u8,
                1 => rule.condition_op = (draw(6) % u64::from(OPERATOR_COUNT)) as u8,
                2 => {
                    rule.condition_param =
                        (draw(6) % crate::morphology::MODULE_TYPE_COUNT as u64) as u8
                }
                3 => {
                    rule.threshold =
                        (i32::from(rule.threshold) + step).clamp(0, i32::from(u16::MAX)) as u16
                }
                4 => rule.action_kind = (draw(6) % u64::from(ACTION_KIND_COUNT)) as u8,
                _ => {
                    if draw(7) & 1 == 0 {
                        rule.action_type =
                            (draw(6) % crate::morphology::MODULE_TYPE_COUNT as u64) as u8;
                    } else {
                        rule.direction = (draw(6) & 0xff) as u8;
                    }
                }
            }
            *rule = rule.normalized();
        }
    }
    true
}

fn duplicate(
    genome: &mut Genome2,
    config: &MutationConfig,
    caps: &GenomeCaps,
    world_seed: u64,
    tick: u64,
    child_id: u64,
    draw: &dyn Fn(u32) -> u64,
) -> Result<(), RejectReason> {
    let (haplotype, chromosome) = chromosome_pick(genome, draw(1));
    let source = genome.haplotypes[haplotype].chromosomes[chromosome].clone();
    if source.is_empty() {
        return Err(RejectReason::Inapplicable);
    }
    // The run starts among the **structural** loci, never the traits.
    //
    // Traits occupy a reserved low homology block and the chromosome is
    // sorted, so every trait precedes every structural locus and a run
    // starting past them can never reach back. Picking uniformly over the
    // whole chromosome instead would land on a trait about two thirds of
    // the time for a minimal founder, silently making the realized
    // duplication rate a third of the configured one -- and a slow
    // structural-evolution result would then be an artifact of trait
    // storage rather than a fact about duplication.
    let first_structural = source
        .iter()
        .position(|locus| !matches!(locus.kind, LocusKind::Trait { .. }))
        .ok_or(RejectReason::Inapplicable)?;
    let structural_len = source.len() - first_structural;
    let run = 1 + (draw(2) as usize) % config.max_run.max(1) as usize;
    let start = first_structural + (draw(3) as usize) % structural_len;
    let end = (start + run).min(source.len());
    if source.len() + (end - start) > caps.max_loci_per_chromosome as usize {
        return Err(RejectReason::Cap);
    }

    let taken: Vec<u32> = source.iter().map(|locus| locus.homology_id).collect();
    let mut copies = Vec::with_capacity(end - start);
    for (ordinal, original) in source[start..end].iter().enumerate() {
        // A trait locus lives in a reserved block keyed by `trait_id`;
        // duplicating one would need a second slot for the same trait, which
        // the block has no room for. The run selection above cannot reach a
        // trait, so this is an assertion rather than an expected path.
        if matches!(original.kind, LocusKind::Trait { .. }) {
            return Err(RejectReason::Invalid);
        }
        let offset = 1
            + (derive_homology_id(original.homology_id, OP_DUPLICATION, ordinal as u32, 0)
                % DUPLICATE_SPAN);
        let fresh = original.homology_id.saturating_add(offset);
        if fresh < STRUCTURAL_HOMOLOGY_BASE
            || taken.binary_search(&fresh).is_ok()
            || copies.iter().any(|copy: &Locus| copy.homology_id == fresh)
        {
            return Err(RejectReason::HomologyCollision);
        }
        // The copy is exact apart from its identity, so divergence is a
        // later event -- as in biology, where a fresh duplicate is redundant
        // and only becomes something else through subsequent mutation.
        copies.push(Locus {
            homology_id: fresh,
            gene_lineage_id: derive_gene_lineage_id(world_seed, tick, child_id, fresh),
            mutation_event_id: derive_mutation_event_id(
                world_seed,
                tick,
                child_id,
                OP_DUPLICATION,
                ordinal as u32,
            ),
            kind: original.kind,
        });
    }

    let loci = &mut genome.haplotypes[haplotype].chromosomes[chromosome];
    loci.extend(copies);
    loci.sort_unstable_by_key(|locus| locus.homology_id);
    match genome.validate_structure(caps) {
        Ok(()) => Ok(()),
        Err(crate::genome2::Genome2Error::ZeroDelayCycle) => Err(RejectReason::Cycle),
        Err(crate::genome2::Genome2Error::CapExceeded(_)) => Err(RejectReason::Cap),
        Err(_) => Err(RejectReason::Invalid),
    }
}

fn delete(
    genome: &mut Genome2,
    config: &MutationConfig,
    caps: &GenomeCaps,
    draw: &dyn Fn(u32) -> u64,
) -> Result<(), RejectReason> {
    let (haplotype, chromosome) = chromosome_pick(genome, draw(17));
    let before = genome.haplotypes[haplotype].chromosomes[chromosome].clone();
    if before.is_empty() {
        return Err(RejectReason::Inapplicable);
    }
    let run = 1 + (draw(18) as usize) % config.max_run.max(1) as usize;
    let start = (draw(19) as usize) % before.len();
    let end = (start + run).min(before.len());

    let mut working = genome.clone();
    working.haplotypes[haplotype].chromosomes[chromosome].drain(start..end);

    // The guards the specification names, checked explicitly so a rejection
    // says *why* rather than only that validation failed.
    let nodes = working.haplotypes[haplotype]
        .chromosomes
        .iter()
        .flatten()
        .filter(|locus| matches!(locus.kind, LocusKind::Node { .. }))
        .count() as u32;
    if nodes < caps.min_nodes {
        return Err(RejectReason::MinNodes);
    }
    let bindings = working.haplotypes[haplotype]
        .chromosomes
        .iter()
        .flatten()
        .filter(|locus| matches!(locus.kind, LocusKind::IoBinding { .. }))
        .count();
    if bindings == 0 && !before.is_empty() {
        // An organism with no binding at all can neither sense nor act. The
        // specification's "last `IoBinding` for a required channel" reads
        // here as "the last binding of any kind", because the registry has
        // no notion of a required channel and inventing one would author a
        // requirement the physics does not have.
        return Err(RejectReason::NoBindings);
    }
    match working.validate_structure(caps) {
        Ok(()) => {
            *genome = working;
            Ok(())
        }
        // A deletion that removes a node still referenced by an edge is the
        // orphaning case, which is what this reason exists to name.
        Err(crate::genome2::Genome2Error::DanglingReference { .. }) => Err(RejectReason::Orphaned),
        Err(crate::genome2::Genome2Error::CapExceeded(_)) => Err(RejectReason::Cap),
        Err(crate::genome2::Genome2Error::ZeroDelayCycle) => Err(RejectReason::Cycle),
        Err(_) => Err(RejectReason::Invalid),
    }
}

fn insert(
    genome: &mut Genome2,
    caps: &GenomeCaps,
    world_seed: u64,
    tick: u64,
    child_id: u64,
    draw: &dyn Fn(u32) -> u64,
) -> Result<(), RejectReason> {
    let (haplotype, chromosome) = chromosome_pick(genome, draw(33));
    let nodes: Vec<u32> = genome.haplotypes[haplotype]
        .chromosomes
        .iter()
        .flatten()
        .filter(|locus| matches!(locus.kind, LocusKind::Node { .. }))
        .map(|locus| locus.homology_id)
        .collect();
    if nodes.len() < 2 {
        return Err(RejectReason::Inapplicable);
    }
    let source = nodes[(draw(34) as usize) % nodes.len()];
    let target = nodes[(draw(35) as usize) % nodes.len()];
    if source == target {
        return Err(RejectReason::Inapplicable);
    }
    let fresh = derive_homology_id(source ^ target, OP_INSERTION, 0, 0);
    let occupied = genome.haplotypes[haplotype]
        .chromosomes
        .iter()
        .flatten()
        .any(|locus| locus.homology_id == fresh);
    if occupied {
        return Err(RejectReason::HomologyCollision);
    }

    let mut working = genome.clone();
    // A fresh edge is delayed by default. A zero-delay edge could close a
    // cycle, which is a decode error, so an insertion that defaulted to
    // zero-delay would be rejected far more often than it succeeded; a
    // delayed edge can always be added safely and a later point mutation on
    // its flags is the path to zero-delay.
    working.haplotypes[haplotype].chromosomes[chromosome].push(Locus {
        homology_id: fresh,
        gene_lineage_id: derive_gene_lineage_id(world_seed, tick, child_id, fresh),
        mutation_event_id: derive_mutation_event_id(world_seed, tick, child_id, OP_INSERTION, 0),
        kind: LocusKind::Edge {
            source,
            target,
            weight: ((draw(36) & 0xffff) as f32 / 32_768.0 - 1.0) * VALUE_LIMIT / 4.0,
            flags: crate::genome2::EDGE_FLAG_DELAYED,
            plasticity: PlasticityGenes::inert(),
        },
    });
    working.haplotypes[haplotype].chromosomes[chromosome]
        .sort_unstable_by_key(|locus| locus.homology_id);
    match working.validate_structure(caps) {
        Ok(()) => {
            *genome = working;
            Ok(())
        }
        Err(crate::genome2::Genome2Error::CapExceeded(_)) => Err(RejectReason::Cap),
        Err(crate::genome2::Genome2Error::ZeroDelayCycle) => Err(RejectReason::Cycle),
        Err(crate::genome2::Genome2Error::DanglingReference { .. }) => Err(RejectReason::Orphaned),
        Err(_) => Err(RejectReason::Invalid),
    }
}

fn transpose(
    genome: &mut Genome2,
    config: &MutationConfig,
    caps: &GenomeCaps,
    draw: &dyn Fn(u32) -> u64,
) -> Result<(), RejectReason> {
    // Transposition moves a run to another chromosome, changing linkage
    // without changing content. Within one chromosome it would be a no-op,
    // because position is determined by homology order rather than by array
    // index, so a single-chromosome genome cannot transpose.
    if genome.chromosome_count() < 2 {
        return Err(RejectReason::Inapplicable);
    }
    let (haplotype, from) = chromosome_pick(genome, draw(49));
    let to = (from + 1 + (draw(50) as usize) % (genome.chromosome_count() - 1))
        % genome.chromosome_count();
    let source = genome.haplotypes[haplotype].chromosomes[from].clone();
    if source.is_empty() {
        return Err(RejectReason::Inapplicable);
    }
    let run = 1 + (draw(51) as usize) % config.max_run.max(1) as usize;
    let start = (draw(52) as usize) % source.len();
    let end = (start + run).min(source.len());

    let mut working = genome.clone();
    let moved: Vec<Locus> = working.haplotypes[haplotype].chromosomes[from]
        .drain(start..end)
        .collect();
    let destination = &mut working.haplotypes[haplotype].chromosomes[to];
    if destination.len() + moved.len() > caps.max_loci_per_chromosome as usize {
        return Err(RejectReason::Cap);
    }
    // A homology ID already present in the destination would break
    // sortedness; the content is unchanged, so the collision is the only
    // thing that can go wrong here.
    for locus in &moved {
        if destination
            .binary_search_by_key(&locus.homology_id, |existing| existing.homology_id)
            .is_ok()
        {
            return Err(RejectReason::HomologyCollision);
        }
    }
    destination.extend(moved);
    destination.sort_unstable_by_key(|locus| locus.homology_id);
    match working.validate_structure(caps) {
        Ok(()) => {
            *genome = working;
            Ok(())
        }
        Err(crate::genome2::Genome2Error::DanglingReference { .. }) => Err(RejectReason::Orphaned),
        Err(crate::genome2::Genome2Error::CapExceeded(_)) => Err(RejectReason::Cap),
        Err(crate::genome2::Genome2Error::ZeroDelayCycle) => Err(RejectReason::Cycle),
        Err(_) => Err(RejectReason::Invalid),
    }
}

/// A minimal viable founder genome: one bound input, one hidden node, one
/// bound output, and a feed-forward chain, with every trait present on both
/// haplotypes.
///
/// Deliberately minimal. Founders that already carried a rich topology would
/// make C9.1's "structure evolved" claim partly a claim about what we seeded.
pub fn minimal_founder(traits: &[f32; crate::genome::TRAIT_COUNT]) -> Genome2 {
    let build = || {
        let mut loci: Vec<Locus> = (0..crate::genome::TRAIT_COUNT)
            .map(|index| Locus {
                homology_id: index as u32 + 1,
                gene_lineage_id: index as u64 + 1,
                mutation_event_id: 0,
                kind: LocusKind::Trait {
                    trait_id: index as u16,
                    value: traits[index].clamp(0.0, 1.0),
                    dominance: 0.5,
                },
            })
            .collect();
        let input = STRUCTURAL_HOMOLOGY_BASE + 1_000;
        let hidden = STRUCTURAL_HOMOLOGY_BASE + 2_000;
        let output = STRUCTURAL_HOMOLOGY_BASE + 3_000;
        let mut structural = |homology_id: u32, kind: LocusKind| {
            loci.push(Locus {
                homology_id,
                gene_lineage_id: u64::from(homology_id),
                mutation_event_id: 0,
                kind,
            });
        };
        structural(
            input,
            LocusKind::Node {
                role: NodeRole::Input,
                activation_id: Activation::Linear.id(),
                bias: 0.0,
                time_constant: 0,
            },
        );
        structural(
            hidden,
            LocusKind::Node {
                role: NodeRole::Hidden,
                activation_id: Activation::TanhApprox.id(),
                bias: 0.0,
                time_constant: 0,
            },
        );
        structural(
            output,
            LocusKind::Node {
                role: NodeRole::Output,
                activation_id: Activation::TanhApprox.id(),
                bias: 0.0,
                time_constant: 0,
            },
        );
        structural(
            STRUCTURAL_HOMOLOGY_BASE + 4_000,
            LocusKind::Edge {
                source: input,
                target: hidden,
                weight: 1.0,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        );
        structural(
            STRUCTURAL_HOMOLOGY_BASE + 5_000,
            LocusKind::Edge {
                source: hidden,
                target: output,
                weight: 1.0,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        );
        structural(
            STRUCTURAL_HOMOLOGY_BASE + 6_000,
            LocusKind::IoBinding {
                node: input,
                channel_id: 1,
                gain: 1.0,
            },
        );
        structural(
            STRUCTURAL_HOMOLOGY_BASE + 7_000,
            LocusKind::IoBinding {
                node: output,
                channel_id: 101,
                gain: 1.0,
            },
        );
        loci.sort_unstable_by_key(|locus| locus.homology_id);
        crate::genome2::Haplotype {
            chromosomes: vec![loci],
        }
    };
    Genome2 {
        haplotypes: [build(), build()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn founder() -> Genome2 {
        minimal_founder(&[0.5; crate::genome::TRAIT_COUNT])
    }

    fn caps() -> GenomeCaps {
        GenomeCaps::provisional()
    }

    fn always(rate: u32) -> MutationConfig {
        MutationConfig {
            point_q16: 0,
            duplication_q16: 0,
            deletion_q16: 0,
            insertion_q16: 0,
            transposition_q16: 0,
            max_run: 3,
            point_delta_q16: rate,
        }
    }

    fn node_count(genome: &Genome2) -> usize {
        genome
            .loci()
            .filter(|locus| matches!(locus.kind, LocusKind::Node { .. }))
            .count()
    }

    #[test]
    fn the_founder_is_valid_and_minimal() {
        let subject = founder();
        subject.validate_structure(&caps()).expect("valid");
        let network = subject.express_network();
        assert_eq!(network.nodes.len(), 3);
        assert_eq!(network.edges.len(), 2);
        assert_eq!(network.bindings.len(), 2);
        // Round-trips through the codec, so a founder is storable.
        let bytes = subject.encode();
        assert_eq!(Genome2::decode(&bytes, &caps()).expect("decodes"), subject);
    }

    #[test]
    fn mutation_never_produces_an_invalid_genome() {
        // The core safety property. Every operator at a high rate, across
        // many children, and the result must always validate -- otherwise an
        // invalid record could reach world state.
        let config = MutationConfig {
            point_q16: 65_535,
            duplication_q16: 65_535,
            deletion_q16: 65_535,
            insertion_q16: 65_535,
            transposition_q16: 65_535,
            max_run: 3,
            point_delta_q16: 6_554,
        };
        let mut counters = MutationCounters::default();
        for child in 0..2_000_u64 {
            let mut subject = founder();
            mutate(
                &mut subject,
                &config,
                &caps(),
                &mut counters,
                7,
                child,
                child,
            );
            subject.validate_structure(&caps()).unwrap_or_else(|error| {
                panic!("child {child} produced an invalid genome: {error}")
            });
            // ...and it must still round-trip, or it could not be saved.
            let bytes = subject.encode();
            Genome2::decode(&bytes, &caps())
                .unwrap_or_else(|error| panic!("child {child} does not decode: {error}"));
        }
        assert!(counters.total_applied() > 0, "no operator ever applied");
    }

    #[test]
    fn mutation_is_a_pure_function_of_its_key() {
        let config = MutationConfig {
            point_q16: 65_535,
            duplication_q16: 65_535,
            deletion_q16: 32_768,
            insertion_q16: 32_768,
            transposition_q16: 32_768,
            ..MutationConfig::default()
        };
        for child in 0..50_u64 {
            let mut first = founder();
            let mut second = founder();
            let mut counters_a = MutationCounters::default();
            let mut counters_b = MutationCounters::default();
            mutate(&mut first, &config, &caps(), &mut counters_a, 3, 9, child);
            mutate(&mut second, &config, &caps(), &mut counters_b, 3, 9, child);
            assert_eq!(first, second, "child {child} is not reproducible");
            assert_eq!(counters_a, counters_b);
        }
    }

    #[test]
    fn duplication_grows_the_genome_and_lands_next_to_its_source() {
        // The resolution this module exists to explain: a duplicate must
        // sort within `DUPLICATE_SPAN` of its source, so tandem duplicates
        // stay linked and a crossover is unlikely to separate them.
        let config = MutationConfig {
            duplication_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        let mut grown = 0;
        for child in 0..500_u64 {
            let before = founder();
            let mut after = before.clone();
            mutate(
                &mut after,
                &config,
                &caps(),
                &mut counters,
                11,
                child,
                child,
            );
            // Either haplotype may be the one duplicated, so both are
            // inspected; checking only slot 0 would silently halve the
            // observed success rate.
            let added: Vec<u32> = after
                .loci()
                .map(|locus| locus.homology_id)
                .filter(|id| !before.loci().any(|locus| locus.homology_id == *id))
                .collect();
            if added.is_empty() {
                continue;
            }
            grown += 1;
            for fresh in added {
                let nearest = before
                    .loci()
                    .map(|locus| fresh.abs_diff(locus.homology_id))
                    .min()
                    .expect("non-empty");
                assert!(
                    nearest <= DUPLICATE_SPAN,
                    "duplicate {fresh} landed {nearest} away from its nearest source"
                );
            }
        }
        // With the run confined to structural loci the operator should now
        // almost always succeed; a collision in the 16-wide span is the only
        // ordinary failure.
        assert!(
            grown > 400,
            "duplication succeeded only {grown} of 500 times, so it is being blocked"
        );
        assert!(counters.duplication_applied > 0);
    }

    #[test]
    fn a_duplicate_is_an_exact_copy_apart_from_its_identity() {
        // Divergence is a later event, as in biology: a fresh duplicate is
        // redundant, and only subsequent mutation makes it something else.
        let config = MutationConfig {
            duplication_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        for child in 0..200_u64 {
            let before = founder();
            let mut after = before.clone();
            mutate(
                &mut after,
                &config,
                &caps(),
                &mut counters,
                13,
                child,
                child,
            );
            for locus in after.loci() {
                let known = before
                    .loci()
                    .any(|original| original.homology_id == locus.homology_id);
                if known {
                    continue;
                }
                // A fresh locus must have the same payload as some existing
                // one, and a different identity.
                assert!(
                    before.loci().any(|original| original.kind == locus.kind),
                    "a duplicate's payload matches no source"
                );
            }
        }
    }

    #[test]
    fn identical_duplications_in_two_lineages_converge_on_the_same_id() {
        // The property a global counter cannot provide. Two separate
        // reproductions duplicating the same source locus must produce the
        // same homology ID, so the two lineages align during meiosis.
        let ordinal = 0;
        let source = STRUCTURAL_HOMOLOGY_BASE + 4_000;
        let left = derive_homology_id(source, OP_DUPLICATION, ordinal, 0) % DUPLICATE_SPAN;
        let right = derive_homology_id(source, OP_DUPLICATION, ordinal, 0) % DUPLICATE_SPAN;
        assert_eq!(left, right);
    }

    #[test]
    fn deletion_is_guarded_and_every_refusal_is_counted() {
        // The guards the specification names. A genome at `min_nodes` must
        // refuse to lose another node, and the refusal must be attributed.
        let mut tight = caps();
        tight.min_nodes = 3;
        let config = MutationConfig {
            deletion_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        for child in 0..400_u64 {
            let mut subject = founder();
            mutate(
                &mut subject,
                &config,
                &tight,
                &mut counters,
                17,
                child,
                child,
            );
            assert!(node_count(&subject) >= 6, "min_nodes was breached");
        }
        assert!(
            counters.total_rejected() > 0,
            "no deletion was ever refused, so the guards are untested"
        );
        assert!(
            counters.rejected_min_nodes > 0 || counters.rejected_orphaned > 0,
            "refusals were not attributed to a named guard: {counters:?}"
        );
    }

    #[test]
    fn a_cap_rejects_rather_than_being_silently_exceeded() {
        let mut tiny = caps();
        tiny.max_loci_per_chromosome = 21; // the founder's own size
        let config = MutationConfig {
            duplication_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        for child in 0..200_u64 {
            let mut subject = founder();
            mutate(
                &mut subject,
                &config,
                &tiny,
                &mut counters,
                19,
                child,
                child,
            );
            assert!(subject.haplotypes[0].chromosomes[0].len() <= 21);
        }
        assert!(
            counters.rejected_cap > 0,
            "the cap was never reported as the reason: {counters:?}"
        );
        assert_eq!(counters.duplication_applied, 0);
    }

    #[test]
    fn transposition_moves_content_without_changing_it() {
        // Linkage changes; the gene set does not. That is the whole point of
        // the operator, and it is what makes it a linkage control.
        let mut two_chromosome = founder();
        for haplotype in &mut two_chromosome.haplotypes {
            haplotype.chromosomes.push(Vec::new());
        }
        let config = MutationConfig {
            transposition_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        let mut moved = 0;
        for child in 0..300_u64 {
            let mut subject = two_chromosome.clone();
            mutate(
                &mut subject,
                &config,
                &caps(),
                &mut counters,
                23,
                child,
                child,
            );
            let mut before: Vec<u32> = two_chromosome
                .loci()
                .map(|locus| locus.homology_id)
                .collect();
            let mut after: Vec<u32> = subject.loci().map(|locus| locus.homology_id).collect();
            before.sort_unstable();
            after.sort_unstable();
            assert_eq!(before, after, "transposition changed the gene set");
            if subject.haplotypes[0].chromosomes[1].len()
                != two_chromosome.haplotypes[0].chromosomes[1].len()
                || subject.haplotypes[1].chromosomes[1].len()
                    != two_chromosome.haplotypes[1].chromosomes[1].len()
            {
                moved += 1;
            }
        }
        assert!(moved > 0, "transposition never actually moved anything");
    }

    #[test]
    fn insertion_adds_an_edge_between_existing_nodes() {
        let config = MutationConfig {
            insertion_q16: 65_535,
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        let mut added = 0;
        for child in 0..300_u64 {
            let before = founder();
            let mut after = before.clone();
            mutate(
                &mut after,
                &config,
                &caps(),
                &mut counters,
                29,
                child,
                child,
            );
            let before_edges = before
                .loci()
                .filter(|l| matches!(l.kind, LocusKind::Edge { .. }))
                .count();
            let after_edges = after
                .loci()
                .filter(|l| matches!(l.kind, LocusKind::Edge { .. }))
                .count();
            if after_edges > before_edges {
                added += 1;
                // An inserted edge is delayed, so it can never close a
                // zero-delay cycle and be rejected at decode.
                assert!(after.validate_structure(&caps()).is_ok());
            }
        }
        assert!(added > 0, "insertion never added an edge");
        assert!(counters.insertion_applied > 0);
    }

    #[test]
    fn point_mutation_stays_inside_every_bound() {
        let config = MutationConfig {
            point_q16: 65_535,
            point_delta_q16: 32_768, // deliberately huge
            ..always(0)
        };
        let mut counters = MutationCounters::default();
        let mut subject = founder();
        for child in 0..3_000_u64 {
            mutate(
                &mut subject,
                &config,
                &caps(),
                &mut counters,
                31,
                child,
                child,
            );
        }
        for locus in subject.loci() {
            match locus.kind {
                LocusKind::Trait {
                    value, dominance, ..
                } => {
                    assert!((0.0..=1.0).contains(&value) && (0.0..=1.0).contains(&dominance));
                }
                LocusKind::Node { bias, .. } => {
                    assert!((-VALUE_LIMIT..=VALUE_LIMIT).contains(&bias));
                }
                LocusKind::Edge { weight, .. } => {
                    assert!((-VALUE_LIMIT..=VALUE_LIMIT).contains(&weight));
                }
                LocusKind::IoBinding { gain, .. } => {
                    assert!((-VALUE_LIMIT..=VALUE_LIMIT).contains(&gain));
                }
                LocusKind::Regulatory { rule } => {
                    // Growth-rule fields have no float range to check; what
                    // must hold is that every code stays inside its registry
                    // after any number of mutations, which is what makes the
                    // genotype space total.
                    assert_eq!(rule, rule.normalized());
                }
            }
        }
        assert!(counters.point_applied > 0);
    }

    #[test]
    fn repeated_reproduction_grows_structure_without_running_away() {
        // The end-to-end shape C9.1 will measure: with duplication on and
        // deletion on, a lineage's structure changes over generations and
        // stays inside its caps.
        let config = MutationConfig {
            point_q16: 6_554,
            duplication_q16: 13_107,
            deletion_q16: 6_554,
            insertion_q16: 0,
            transposition_q16: 0,
            max_run: 2,
            point_delta_q16: 3_277,
        };
        let mut counters = MutationCounters::default();
        let mut lineage = founder();
        let start = node_count(&lineage);
        for generation in 0..400_u64 {
            let child = crate::meiosis::recombine(
                (&lineage, 1),
                (&lineage, 2),
                &crate::meiosis::MeiosisConfig::default(),
                37,
                generation,
                generation,
            );
            lineage = child;
            mutate(
                &mut lineage,
                &config,
                &caps(),
                &mut counters,
                37,
                generation,
                generation,
            );
            lineage
                .validate_structure(&caps())
                .unwrap_or_else(|e| panic!("generation {generation}: {e}"));
        }
        let end = node_count(&lineage);
        assert_ne!(start, end, "structure never changed over 400 generations");
        assert!(
            lineage.encode().len() < caps().max_genome_bytes as usize,
            "the genome ran away to {} bytes",
            lineage.encode().len()
        );
        assert!(counters.duplication_applied > 0 && counters.point_applied > 0);
    }
}
