//! Development: genome to body (Phase 10, `lifesim-develop-v1`).
//!
//! The genome does not store a module list. It stores a **growth program** of
//! regulatory loci that is executed to produce the body. That is the indirect
//! encoding ADR-0019 adopts, and the reason is that a direct encoding cannot
//! produce repeated structure cheaply: doubling a limb count would mean
//! duplicating every locus of that limb, whereas a rule that fires wherever
//! its condition holds gives repetition, symmetry, and segmentation from one
//! mutation.
//!
//! ## The cost of that choice, and the gate that checks it
//!
//! Two commissioned reviews recommend against developmental encodings as a
//! baseline (`genetics` 1.6, `neuroevolution` 1.4). ADR-0022 D1 partially
//! declines that for morphology only, and the concession is that Phase 10's
//! genotype-phenotype discontinuity measurement is a **gate, not a metric**:
//! if a typical single-locus mutation produces an unrelated body, selection
//! cannot act on morphology at all and the specified parameterized fallback
//! is taken. `phenotypic_distance` is what that gate measures.
//!
//! ## Execution is bounded, ordered, and fail-closed
//!
//! Every property `genetics` 1.6 requires of a generative encoding is a
//! property of this loop rather than a convention:
//!
//! - **Maximum expansion steps** and **maximum emitted modules** are config
//!   caps, both enforced by refusal rather than truncation.
//! - **Deterministic rule-match and conflict order**: loci evaluate in
//!   ascending `homology_id`, against modules in ascending lattice index, and
//!   the collected actions apply in ascending `(homology_id, lattice_index)`.
//! - **Phenotype-overflow behaviour** is rejection: a body that exceeds a cap
//!   is non-viable, never trimmed to fit. A trimmed body is one no genome
//!   encoded.
//! - **Provenance**: every emitted module carries the `homology_id` of the
//!   locus that emitted it.
//!
//! The default development policy is **fully deterministic and draws no
//! randomness at all**. The `Morphogenesis` stream exists so that adopting a
//! stochastic developmental term later cannot renumber the streams.

use crate::genome2::Genome2;
use crate::morphology::{
    Body, LatticeKind, LatticePos, MAX_SCALE_MILLI, MIN_SCALE_MILLI, Module, ModuleType,
    MorphologyCaps, ViabilityFailure,
};

pub const DEVELOP_POLICY_VERSION: &str = "lifesim-develop-v1";

/// What a regulatory locus looks at. Codes are permanent.
pub const COND_ALWAYS: u8 = 0;
/// The module's own type equals `param`.
pub const COND_SELF_TYPE: u8 = 1;
/// The developmental step compared against `threshold`.
pub const COND_STEP: u8 = 2;
/// Total module count compared against `threshold`.
pub const COND_MODULE_COUNT: u8 = 3;
/// Count of modules of type `param` compared against `threshold`.
pub const COND_TYPE_COUNT: u8 = 4;
/// Number of occupied neighbours of this module compared against `threshold`.
pub const COND_NEIGHBOURS: u8 = 5;
/// Distance of this module from the origin compared against `threshold`.
pub const COND_DISTANCE: u8 = 6;
pub const CONDITION_KIND_COUNT: u8 = 7;

/// Comparison operators. Permanent codes.
pub const OP_LT: u8 = 0;
pub const OP_EQ: u8 = 1;
pub const OP_GE: u8 = 2;
pub const OPERATOR_COUNT: u8 = 3;

/// Place a module of `action_type` at the neighbour in `direction`.
pub const ACT_PLACE: u8 = 0;
/// Change this module's type to `action_type`.
pub const ACT_DIFFERENTIATE: u8 = 1;
/// Set this module's scale to `scale_milli`.
pub const ACT_SET_SCALE: u8 = 2;
/// Stop this module from matching any further rule.
pub const ACT_TERMINATE: u8 = 3;
pub const ACTION_KIND_COUNT: u8 = 4;

/// A growth rule. Fixed width, every field bounded, no executable content:
/// this is a table row, not a program in a language, which is what keeps
/// "never introduce unrestricted executable genomes" true by construction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Regulatory {
    pub condition_kind: u8,
    pub condition_op: u8,
    /// Module type code for the conditions that name one.
    pub condition_param: u8,
    pub threshold: u16,
    pub action_kind: u8,
    pub action_type: u8,
    pub direction: u8,
    pub scale_milli: u16,
}

impl Regulatory {
    pub const ENCODED_LEN: usize = 1 + 1 + 1 + 2 + 1 + 1 + 1 + 2;

    /// Every field is reduced into range rather than rejected.
    ///
    /// This is the one place in the codebase that normalizes instead of
    /// failing closed, and the reason is specific: these fields are *mutation
    /// targets*. A point mutation perturbs a byte, and if an out-of-range
    /// byte made the genome invalid, most mutations of a regulatory locus
    /// would be lethal and the encoding would be unevolvable for a reason
    /// that has nothing to do with morphology. Reduction keeps the genotype
    /// space total: every bit pattern names some rule.
    ///
    /// The bound is still enforced at decode - a locus is *stored* reduced -
    /// so nothing downstream ever sees an out-of-range code.
    pub fn normalized(self) -> Self {
        Self {
            condition_kind: self.condition_kind % CONDITION_KIND_COUNT,
            condition_op: self.condition_op % OPERATOR_COUNT,
            condition_param: self.condition_param % crate::morphology::MODULE_TYPE_COUNT as u8,
            threshold: self.threshold,
            action_kind: self.action_kind % ACTION_KIND_COUNT,
            action_type: self.action_type % crate::morphology::MODULE_TYPE_COUNT as u8,
            direction: self.direction,
            scale_milli: self.scale_milli.clamp(MIN_SCALE_MILLI, MAX_SCALE_MILLI),
        }
    }

    fn module_type(&self) -> ModuleType {
        ModuleType::from_id(self.action_type % crate::morphology::MODULE_TYPE_COUNT as u8)
            .expect("normalized action type")
    }

    fn condition_type(&self) -> ModuleType {
        ModuleType::from_id(self.condition_param % crate::morphology::MODULE_TYPE_COUNT as u8)
            .expect("normalized condition param")
    }
}

/// Counters for what development did and refused. World state; hashed.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DevelopCounters {
    pub bodies_grown: u64,
    pub modules_placed: u64,
    pub differentiations: u64,
    pub scale_changes: u64,
    /// Placements refused because the target cell was occupied.
    pub refused_occupied: u64,
    /// Placements refused because the target cell was outside the lattice.
    pub refused_out_of_bounds: u64,
    /// Placements refused because the body was already at `max_modules`.
    pub refused_max_modules: u64,
    /// Births refused because the controller needed more nodes than the
    /// body's neural tissue could support. C10.7's coupling, counted so a
    /// campaign can tell "brains are constrained" from "brains are free".
    pub refused_node_budget: u64,
    /// Bodies that failed validation, by class.
    pub nonviable_empty: u64,
    pub nonviable_disconnected: u64,
    pub nonviable_missing_type: u64,
    pub nonviable_other: u64,
}

impl DevelopCounters {
    pub fn total_nonviable(&self) -> u64 {
        self.nonviable_empty
            + self.nonviable_disconnected
            + self.nonviable_missing_type
            + self.nonviable_other
    }

    fn record_failure(&mut self, failure: ViabilityFailure) {
        match failure {
            ViabilityFailure::Empty => self.nonviable_empty += 1,
            ViabilityFailure::Disconnected => self.nonviable_disconnected += 1,
            ViabilityFailure::MissingRequiredType(_) => self.nonviable_missing_type += 1,
            _ => self.nonviable_other += 1,
        }
    }

    pub fn hash_into(&self, hasher: &mut crate::checksum::Fnv1a64) {
        hasher.update(b"lifesim-morphology-state-v1");
        for value in [
            self.bodies_grown,
            self.modules_placed,
            self.differentiations,
            self.scale_changes,
            self.refused_occupied,
            self.refused_out_of_bounds,
            self.refused_max_modules,
            self.refused_node_budget,
            self.nonviable_empty,
            self.nonviable_disconnected,
            self.nonviable_missing_type,
            self.nonviable_other,
        ] {
            hasher.update_u64(value);
        }
    }
}

/// One collected action, before ordering.
struct Pending {
    locus: u32,
    lattice_index: u32,
    module_index: usize,
    rule: Regulatory,
}

/// Grow a body from a genome's regulatory loci.
///
/// Returns the body whether or not it is viable; the caller validates and
/// decides, because "grew a body" and "the body is viable" are different
/// facts and a caller that wants the second usually wants to count the first.
pub fn grow(
    rules: &[(u32, Regulatory)],
    lattice: LatticeKind,
    caps: &MorphologyCaps,
    counters: &mut DevelopCounters,
) -> Body {
    // Step 1: a single Structural module at the lattice origin. Every body
    // starts identically, so all morphological difference is developmental
    // rather than seeded - the same reason Phase 9's founders are minimal.
    let mut modules = vec![Module {
        position: LatticePos::ORIGIN,
        module_type: ModuleType::Structural,
        scale_milli: 1_000,
        orientation: 0,
        source_locus: 0,
    }];
    let mut terminated = vec![false];

    for step in 0..caps.max_growth_steps {
        let mut pending: Vec<Pending> = Vec::new();

        // Loci in ascending homology_id; modules in ascending lattice index.
        // `rules` is required sorted by the caller and `modules` is kept
        // sorted below, so neither loop needs to sort here.
        for (homology_id, rule) in rules {
            for (module_index, module) in modules.iter().enumerate() {
                if terminated[module_index] {
                    continue;
                }
                if !matches(rule, module, module_index, &modules, lattice, step) {
                    continue;
                }
                let Some(lattice_index) = module.position.index(caps.lattice_radius) else {
                    continue;
                };
                pending.push(Pending {
                    locus: *homology_id,
                    lattice_index,
                    module_index,
                    rule: *rule,
                });
            }
        }

        if pending.is_empty() {
            // Step 3's third stop condition: no locus matches, so the body is
            // finished and further steps would be identical no-ops.
            break;
        }

        // Conflict order, exactly as specified: ascending (locus, lattice
        // index). Two rules that target the same cell resolve by which locus
        // is older in homology space, never by iteration accident.
        pending.sort_by_key(|action| (action.locus, action.lattice_index));

        let mut grew = false;
        for action in pending {
            match action.rule.action_kind {
                ACT_PLACE => {
                    if modules.len() >= usize::from(caps.max_modules) {
                        counters.refused_max_modules += 1;
                        continue;
                    }
                    let source = modules[action.module_index].position;
                    let target = source.step(lattice, usize::from(action.rule.direction));
                    if target.index(caps.lattice_radius).is_none() {
                        counters.refused_out_of_bounds += 1;
                        continue;
                    }
                    if modules.iter().any(|module| module.position == target) {
                        counters.refused_occupied += 1;
                        continue;
                    }
                    modules.push(Module {
                        position: target,
                        module_type: action.rule.module_type(),
                        scale_milli: action.rule.scale_milli,
                        orientation: action.rule.direction,
                        source_locus: action.locus,
                    });
                    terminated.push(false);
                    counters.modules_placed += 1;
                    grew = true;
                }
                ACT_DIFFERENTIATE => {
                    let module = &mut modules[action.module_index];
                    let target_type = action.rule.module_type();
                    if module.module_type != target_type {
                        module.module_type = target_type;
                        counters.differentiations += 1;
                        grew = true;
                    }
                }
                ACT_SET_SCALE => {
                    let module = &mut modules[action.module_index];
                    if module.scale_milli != action.rule.scale_milli {
                        module.scale_milli = action.rule.scale_milli;
                        counters.scale_changes += 1;
                        grew = true;
                    }
                }
                ACT_TERMINATE if !terminated[action.module_index] => {
                    terminated[action.module_index] = true;
                    grew = true;
                }
                _ => {}
            }
        }

        // Re-sort so the next step's module iteration is in lattice order and
        // so `Body::from_modules` has nothing left to do. `terminated` must
        // follow the same permutation or a terminated module would silently
        // become a different one.
        let mut paired: Vec<(Module, bool)> = modules
            .iter()
            .copied()
            .zip(terminated.iter().copied())
            .collect();
        paired.sort_by_key(|(module, _)| {
            (
                module
                    .position
                    .index(caps.lattice_radius)
                    .unwrap_or(u32::MAX),
                module.position.q,
                module.position.r,
            )
        });
        modules = paired.iter().map(|(module, _)| *module).collect();
        terminated = paired.iter().map(|(_, stop)| *stop).collect();

        if !grew {
            // Every matching action was refused or was a no-op, so another
            // step would refuse exactly the same ones. Stopping here is what
            // makes the loop terminate early rather than burning the full
            // step budget re-deriving nothing.
            break;
        }
    }

    counters.bodies_grown += 1;
    Body::from_modules(modules, caps.lattice_radius)
}

fn matches(
    rule: &Regulatory,
    module: &Module,
    module_index: usize,
    modules: &[Module],
    lattice: LatticeKind,
    step: u16,
) -> bool {
    let compare = |observed: i64, threshold: i64| -> bool {
        match rule.condition_op {
            OP_LT => observed < threshold,
            OP_EQ => observed == threshold,
            _ => observed >= threshold,
        }
    };
    match rule.condition_kind {
        COND_ALWAYS => true,
        COND_SELF_TYPE => module.module_type == rule.condition_type(),
        COND_STEP => compare(i64::from(step), i64::from(rule.threshold)),
        COND_MODULE_COUNT => compare(modules.len() as i64, i64::from(rule.threshold)),
        COND_TYPE_COUNT => {
            let count = modules
                .iter()
                .filter(|other| other.module_type == rule.condition_type())
                .count();
            compare(count as i64, i64::from(rule.threshold))
        }
        COND_NEIGHBOURS => {
            let occupied = (0..lattice.neighbour_count())
                .filter(|direction| {
                    let neighbour = module.position.step(lattice, *direction);
                    modules.iter().any(|other| other.position == neighbour)
                })
                .count();
            compare(occupied as i64, i64::from(rule.threshold))
        }
        COND_DISTANCE => {
            let _ = module_index;
            let distance = i64::from(module.position.q.abs()) + i64::from(module.position.r.abs());
            compare(distance, i64::from(rule.threshold))
        }
        _ => false,
    }
}

/// Collect a genome's expressed growth program, in ascending
/// `(homology_id, haplotype)`.
///
/// **Regulatory loci are codominant: both alleles express.** Every other
/// locus type blends its two alleles by dominance, but a growth rule has no
/// scalar to blend - averaging two condition codes would name a third rule
/// neither parent carried. The biological analogue of a discrete-valued
/// heterozygote is that both alleles are transcribed and both products act,
/// so a heterozygote runs both rules.
///
/// Identical alleles collapse to one rule, so a homozygote and a hemizygote
/// grow the same body. Without that, a homozygote would run every rule twice
/// and gene dosage would silently become the dominant morphological signal.
///
/// An earlier version took the first haplotype's copy and discarded the
/// second, which made **every** locus on haplotype 1 morphologically inert -
/// half the growth program did nothing, and mutations landing there were
/// invisible. It was caught by C10.4's own guard against measuring too few
/// effective mutations, not by inspection.
pub fn rules_of(genome: &Genome2) -> Vec<(u32, Regulatory)> {
    let mut rules: Vec<(u32, u8, Regulatory)> = Vec::new();
    for (slot, haplotype) in genome.haplotypes.iter().enumerate() {
        for locus in haplotype.chromosomes.iter().flatten() {
            if let crate::genome2::LocusKind::Regulatory { rule } = locus.kind {
                let normalized = rule.normalized();
                if rules
                    .iter()
                    .any(|(id, _, existing)| *id == locus.homology_id && *existing == normalized)
                {
                    continue;
                }
                rules.push((locus.homology_id, slot as u8, normalized));
            }
        }
    }
    // Haplotype slot is the tiebreak, so two alleles of the same locus have a
    // deterministic order without either shadowing the other.
    rules.sort_by_key(|(homology_id, slot, _)| (*homology_id, *slot));
    rules
        .into_iter()
        .map(|(homology_id, _, rule)| (homology_id, rule))
        .collect()
}

/// Grow a body directly from a genome.
pub fn develop(
    genome: &Genome2,
    lattice: LatticeKind,
    caps: &MorphologyCaps,
    counters: &mut DevelopCounters,
) -> Result<Body, ViabilityFailure> {
    let body = grow(&rules_of(genome), lattice, caps, counters);
    match body.validate(lattice, caps) {
        Ok(()) => Ok(body),
        Err(failure) => {
            counters.record_failure(failure);
            Err(failure)
        }
    }
}

/// The founder growth program: gut, motor, sensor.
///
/// **Minimal but functional**, which is the same standard Phase 9's founder
/// network meets - three nodes, two edges, and both channel bindings, rather
/// than the smallest thing that decodes. A one-module gut is a *valid* body
/// and a hopeless organism: with no motor it has zero thrust and sits at the
/// speed floor, and with no sensory module it sits at the sensing floor. A
/// campaign founded on those measures immobility and blindness, not
/// morphology, and the first run of this phase's campaign died in 29 worlds
/// of 30 for exactly that reason.
///
/// Three modules is therefore the floor, not a design preference: each one
/// is the minimum needed for a derived attribute to be something other than
/// its clamp. Everything beyond it - size, symmetry, extra organs, tissue
/// specialisation - is left to evolution, which is what lets a campaign
/// attribute morphology to mutation rather than to what was seeded.
pub fn founder_program() -> Vec<(u32, Regulatory)> {
    let base = crate::genome2::STRUCTURAL_HOMOLOGY_BASE + 20_000;
    let rule = |action_kind: u8, action_type: u8, direction: u8| Regulatory {
        condition_kind: COND_MODULE_COUNT,
        condition_op: OP_LT,
        condition_param: 0,
        // Fires only while the body is still smaller than the founder plan,
        // so growth stops on its own rather than by hitting a cap.
        threshold: 3,
        action_kind,
        action_type,
        direction,
        scale_milli: 1_000,
    };
    vec![
        // The origin becomes a gut.
        (
            base,
            Regulatory {
                condition_kind: COND_SELF_TYPE,
                condition_op: OP_GE,
                condition_param: crate::morphology::TYPE_STRUCTURAL,
                threshold: 0,
                action_kind: ACT_DIFFERENTIATE,
                action_type: crate::morphology::TYPE_DIGESTIVE,
                direction: 0,
                scale_milli: 1_000,
            },
        ),
        // ...then a motor and a sensor, one step apart.
        (
            base + 100,
            rule(ACT_PLACE, crate::morphology::TYPE_MOTOR, 0),
        ),
        (
            base + 200,
            rule(ACT_PLACE, crate::morphology::TYPE_SENSORY, 1),
        ),
    ]
}

/// Homology slots the founder program occupies. Fixed, so two independently
/// created founders align at meiosis.
pub fn founder_program_homology_ids() -> Vec<u32> {
    founder_program().into_iter().map(|(id, _)| id).collect()
}

/// Phenotypic distance between two bodies, milli, in `0..=1000`.
///
/// Lattice-occupancy difference over the union of occupied positions, as
/// `specifications/morphology-and-development.md` specifies for morphological
/// distance - with one addition: a position occupied in both bodies but by
/// **different module types** counts as half a difference. Occupancy alone
/// would call a body of seven motors identical to a body of seven guts,
/// which is not a distance anyone should use as a gate.
///
/// 0 means identical; 1000 means the two bodies share no occupied cell.
pub fn phenotypic_distance_milli(left: &Body, right: &Body) -> i64 {
    if left.is_empty() && right.is_empty() {
        return 0;
    }
    let mut union = 0_i64;
    let mut difference = 0_i64;
    for module in left.modules() {
        union += 1;
        match right
            .modules()
            .iter()
            .find(|other| other.position == module.position)
        {
            Some(other) if other.module_type == module.module_type => {}
            Some(_) => difference += 1,
            None => difference += 2,
        }
    }
    for module in right.modules() {
        if !left.occupied(module.position) {
            union += 1;
            difference += 2;
        }
    }
    if union == 0 {
        return 0;
    }
    // `difference` counts 2 per unshared cell and 1 per type mismatch, so the
    // denominator is 2 * union and a wholly disjoint pair scores exactly 1000.
    difference * 1_000 / (2 * union)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caps() -> MorphologyCaps {
        MorphologyCaps::provisional()
    }

    fn rule(
        condition_kind: u8,
        condition_op: u8,
        condition_param: u8,
        threshold: u16,
        action_kind: u8,
        action_type: u8,
        direction: u8,
    ) -> Regulatory {
        Regulatory {
            condition_kind,
            condition_op,
            condition_param,
            threshold,
            action_kind,
            action_type,
            direction,
            scale_milli: 1_000,
        }
        .normalized()
    }

    /// A rule set that grows a small viable body: differentiate the origin to
    /// digestive, then grow one motor to the east.
    fn simple_rules() -> Vec<(u32, Regulatory)> {
        vec![
            (
                10,
                rule(COND_MODULE_COUNT, OP_LT, 0, 2, ACT_DIFFERENTIATE, 3, 0),
            ),
            (20, rule(COND_MODULE_COUNT, OP_LT, 0, 2, ACT_PLACE, 2, 0)),
        ]
    }

    #[test]
    fn development_is_a_pure_function_of_its_inputs() {
        // C10.1's first clause: same rules and config, same body, every time.
        let mut first = DevelopCounters::default();
        let mut second = DevelopCounters::default();
        let left = grow(&simple_rules(), LatticeKind::Square, &caps(), &mut first);
        let right = grow(&simple_rules(), LatticeKind::Square, &caps(), &mut second);
        assert_eq!(left, right);
        assert_eq!(first, second);
        assert!(left.len() > 1, "the rule set must actually grow something");
    }

    #[test]
    fn permuting_locus_storage_order_gives_an_identical_body() {
        // C10.1's third clause, and the one a storage-layout change would
        // break silently. `rules_of` sorts, and `grow` relies on that; this
        // pins the guarantee at the level a compaction would disturb.
        let mut forward = simple_rules();
        let mut reversed = forward.clone();
        reversed.reverse();
        forward.sort_by_key(|(id, _)| *id);
        reversed.sort_by_key(|(id, _)| *id);
        let mut counters = DevelopCounters::default();
        let left = grow(&forward, LatticeKind::Square, &caps(), &mut counters);
        let right = grow(&reversed, LatticeKind::Square, &caps(), &mut counters);
        assert_eq!(left, right);
    }

    #[test]
    fn growth_always_terminates_within_its_step_budget() {
        // A rule that always matches and always places would run forever
        // without the caps. Property-style: every condition/action
        // combination must terminate and stay inside `max_modules`.
        let caps = caps();
        for condition_kind in 0..CONDITION_KIND_COUNT {
            for action_kind in 0..ACTION_KIND_COUNT {
                for direction in 0..6_u8 {
                    let rules = vec![(
                        1,
                        rule(condition_kind, OP_GE, 0, 0, action_kind, 2, direction),
                    )];
                    let mut counters = DevelopCounters::default();
                    let body = grow(&rules, LatticeKind::Hex, &caps, &mut counters);
                    assert!(
                        body.len() <= usize::from(caps.max_modules),
                        "condition {condition_kind} action {action_kind} exceeded max_modules"
                    );
                }
            }
        }
    }

    #[test]
    fn an_unbounded_rule_hits_the_module_cap_and_is_counted_not_truncated() {
        // "Always place" is the pathological program. It must stop at the
        // cap, and the refusals must be counted rather than silent, or a
        // campaign could not tell a cap-bound body from a finished one.
        let mut caps = caps();
        caps.max_modules = 8;
        caps.max_growth_steps = 32;
        let rules = vec![(1, rule(COND_ALWAYS, OP_GE, 0, 0, ACT_PLACE, 3, 0))];
        let mut counters = DevelopCounters::default();
        let body = grow(&rules, LatticeKind::Square, &caps, &mut counters);
        assert!(body.len() <= 8);
        assert!(
            counters.refused_max_modules > 0 || counters.refused_occupied > 0,
            "the cap bound but nothing was counted"
        );
    }

    #[test]
    fn a_disconnected_body_is_refused_with_a_typed_reason_and_never_repaired() {
        // There is no repair path by construction, so this checks the reason
        // is typed and counted rather than that a repair was skipped.
        // The required-type mask is empty by default, so this world asks for
        // a digestive module explicitly. Under the default caps the same body
        // is viable, which is the intended policy: gutless is unfit, not
        // invalid.
        let mut caps = caps();
        assert_eq!(
            {
                let mut counters = DevelopCounters::default();
                let rules = vec![(1, rule(COND_ALWAYS, OP_GE, 0, 0, ACT_TERMINATE, 0, 0))];
                grow(&rules, LatticeKind::Square, &caps, &mut counters)
                    .validate(LatticeKind::Square, &caps)
            },
            Ok(())
        );
        caps.required_types_mask = 1 << crate::morphology::TYPE_DIGESTIVE;
        let mut counters = DevelopCounters::default();
        // A rule set that terminates immediately: one structural module,
        // which is missing the required digestive type.
        let rules = vec![(1, rule(COND_ALWAYS, OP_GE, 0, 0, ACT_TERMINATE, 0, 0))];
        let body = grow(&rules, LatticeKind::Square, &caps, &mut counters);
        assert_eq!(body.len(), 1);
        let failure = body.validate(LatticeKind::Square, &caps).unwrap_err();
        assert_eq!(
            failure,
            ViabilityFailure::MissingRequiredType(ModuleType::Digestive)
        );
        counters.record_failure(failure);
        assert_eq!(counters.nonviable_missing_type, 1);
        assert_eq!(counters.total_nonviable(), 1);
    }

    #[test]
    fn every_emitted_module_records_the_locus_that_emitted_it() {
        // Provenance is what makes an indirect encoding answerable: without
        // it no analysis can attribute a morphological change to the mutation
        // that caused it (`genetics` 1.6).
        let mut counters = DevelopCounters::default();
        let body = grow(&simple_rules(), LatticeKind::Square, &caps(), &mut counters);
        let placed: Vec<&Module> = body
            .modules()
            .iter()
            .filter(|module| module.position != LatticePos::ORIGIN)
            .collect();
        assert!(!placed.is_empty());
        for module in placed {
            assert_eq!(
                module.source_locus, 20,
                "an emitted module does not name the locus that placed it"
            );
        }
    }

    #[test]
    fn phenotypic_distance_is_zero_for_identical_and_one_for_disjoint() {
        let radius = caps().lattice_radius;
        let make = |positions: &[(i16, i16)], module_type: ModuleType| {
            Body::from_modules(
                positions
                    .iter()
                    .map(|(q, r)| Module {
                        position: LatticePos::new(*q, *r),
                        module_type,
                        scale_milli: 1_000,
                        orientation: 0,
                        source_locus: 1,
                    })
                    .collect(),
                radius,
            )
        };
        let a = make(&[(0, 0), (1, 0)], ModuleType::Digestive);
        assert_eq!(phenotypic_distance_milli(&a, &a), 0);

        let disjoint = make(&[(4, 4), (5, 4)], ModuleType::Digestive);
        assert_eq!(phenotypic_distance_milli(&a, &disjoint), 1_000);

        // Same cells, different types: a real difference, and exactly half of
        // a disjoint one. Occupancy alone would score this zero.
        let recoloured = make(&[(0, 0), (1, 0)], ModuleType::Motor);
        assert_eq!(phenotypic_distance_milli(&a, &recoloured), 500);

        // Half the cells shared.
        let overlapping = make(&[(0, 0), (0, 1)], ModuleType::Digestive);
        assert_eq!(phenotypic_distance_milli(&a, &overlapping), 666);
    }

    #[test]
    fn normalization_makes_every_bit_pattern_a_legal_rule() {
        // Regulatory fields are mutation targets. If an out-of-range byte
        // made a genome invalid, most mutations of a regulatory locus would
        // be lethal for reasons unrelated to morphology, and the encoding
        // would be unevolvable by construction.
        for raw in 0..=255_u8 {
            let normalized = Regulatory {
                condition_kind: raw,
                condition_op: raw,
                condition_param: raw,
                threshold: u16::from(raw),
                action_kind: raw,
                action_type: raw,
                direction: raw,
                scale_milli: u16::from(raw) * 37,
            }
            .normalized();
            assert!(normalized.condition_kind < CONDITION_KIND_COUNT);
            assert!(normalized.condition_op < OPERATOR_COUNT);
            assert!(normalized.action_kind < ACTION_KIND_COUNT);
            assert!(ModuleType::from_id(normalized.action_type).is_some());
            assert!(ModuleType::from_id(normalized.condition_param).is_some());
            assert!(normalized.scale_milli >= MIN_SCALE_MILLI);
            assert!(normalized.scale_milli <= MAX_SCALE_MILLI);
        }
    }
}
