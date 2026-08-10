//! Per-organism learned synaptic state (Phase 11).
//!
//! **This is world state, and it is the only state in the engine that cannot
//! be recomputed from the genome.** A body is a pure function of `(genome,
//! config)` and is therefore regrown on load (`morphstate.rs`); an activation
//! buffer is not, and is saved (`controller2.rs`). Learned deltas are the
//! second case and the stronger one: they are the record of what one
//! individual's lifetime did to it, and recomputing them would not be an
//! approximation, it would be a reset.
//!
//! # Why this is its own subsystem rather than two more vectors on
//! `Schema2State`
//!
//! C11.8 requires a schema-2 world with plasticity disabled to reproduce the
//! Phase 9 fixture **byte for byte**. A section hung off `Schema2State` is
//! present whenever genome2 is, so it would exist - and hash, and cost - in
//! every schema-2 world that already exists, which is exactly the lineage
//! break `specifications/determinism-extensions.md` Rule 0 names Phase 11 as
//! the first candidate for. An `Option<LearnState>` gated on the plasticity
//! config section is absent in those worlds, so nothing is appended and
//! `0x5f0c4e95e4f5170f` is unchanged.
//!
//! The cost of that choice is real and is paid explicitly: lockstep with the
//! population is maintained by hand across two birth paths, one death path,
//! and one restore path, exactly as `MorphologyState` and `PhysiologyState`
//! are. `InvariantViolation::LearnDesync` is what makes a missed push a
//! typed failure at the next `check_invariants` instead of an index panic
//! several thousand ticks later, and `LearnBounds` is what makes a value
//! outside the clamp a typed failure instead of a checksum nobody can
//! explain.
//!
//! # There is no reward in this file either
//!
//! The accessors at the bottom are **observation only** (ADR-0016): they
//! return numbers to a campaign and instruct nothing. None of them is read by
//! the tick, and none of them can be - a quantity computed here that fed back
//! into learning would be a fitness signal delivered to a network, which is
//! the one thing this phase exists to keep out.

use crate::checksum::Fnv1a64;
use crate::plasticity::{LEARN_LIMIT_Q16, PlasticityCounters};

/// Parallel per-organism arrays, kept in lockstep with the world's primary
/// arrays exactly as every other subsystem's are.
///
/// Storage is sparse by organism *and* by edge: only plastic edges have an
/// entry, in the plan's ascending-`homology_id` order. The specification
/// gives the reason and it is a budget rather than a preference - the Phase 4
/// record already has snapshot size dominated by per-organism genome arrays
/// at roughly 2.8 KB each with a synchronous checkpoint on the tick thread,
/// so a dense learned copy of every weight would roughly double it.
#[derive(Clone, Debug, Default)]
pub(crate) struct LearnState {
    /// Q16 learned delta per plastic edge, per organism.
    pub learned_q16: Vec<Vec<i32>>,
    /// Q16 eligibility trace per plastic edge, per organism. Nonzero only
    /// under rule 4, and carried for every plastic edge anyway: a ragged
    /// second array indexed by a different rule would be a second thing to
    /// keep in lockstep for the sake of a few bytes.
    pub trace_q16: Vec<Vec<i32>>,
    /// Non-finite deltas neutralized over this organism's lifetime. The
    /// counterpart of `ActivationState::faults`, and the value the
    /// `PlasticityFault` event reports the per-tick delta of.
    pub faults: Vec<u32>,
    pub counters: PlasticityCounters,
    /// Total energy charged for plastic edges, milli-EU.
    ///
    /// Not needed for conservation - every milli is already in
    /// `Ledger::spent_milli`, which is what `check_invariants` compares
    /// against with no tolerance - and carried because "the ledger balances"
    /// and "plasticity cost what we think it cost" are different claims. A
    /// cost path that debited nothing would leave the ledger exact and this
    /// at zero.
    pub cost_milli: i128,
}

impl LearnState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            learned_q16: Vec::with_capacity(capacity),
            trace_q16: Vec::with_capacity(capacity),
            faults: Vec::with_capacity(capacity),
            counters: PlasticityCounters::default(),
            cost_milli: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.learned_q16.len()
    }

    /// Admit one organism with `plastic_edges` plastic edges, **all learned
    /// state zero**.
    ///
    /// That zero is C11.4, and it is an invariant by construction rather than
    /// a default that a later caller could override: this is the only way an
    /// organism enters the array, it takes no initial-state parameter, and
    /// there is no setter. A child of two parents carrying maximal learned
    /// deltas starts here, at zero, on every plastic edge, because nothing in
    /// the birth path has anywhere to put a parent's delta.
    ///
    /// The property that buys is Phase 13's question. If learned state were
    /// inherited, a discovery would be a heritable trait and transmission
    /// would be indistinguishable from inheritance - so this is not a
    /// conservative default, it is the thing that makes the experiment
    /// possible. `lamarckian_fraction_q16` is where a *declared* exception
    /// would live, and it is validated to zero because no policy implements
    /// one yet.
    pub fn push_organism(&mut self, plastic_edges: usize) {
        self.learned_q16.push(vec![0; plastic_edges]);
        self.trace_q16.push(vec![0; plastic_edges]);
        self.faults.push(0);
    }

    /// Compact after deaths, copying `Schema2State::retain` exactly.
    ///
    /// A deleted organism takes its learned state with it, which is the same
    /// statement the specification makes about a deleted edge and for the
    /// same reason: learned state is per-organism and per-edge, and there is
    /// nothing for it to survive into.
    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.learned_q16.swap(write, read);
                    self.trace_q16.swap(write, read);
                    self.faults.swap(write, read);
                }
                write += 1;
            }
        }
        self.learned_q16.truncate(write);
        self.trace_q16.truncate(write);
        self.faults.truncate(write);
    }

    /// Whether every stored value is inside the clamp the update arithmetic
    /// promises. `Some(index)` names the first organism that is not.
    ///
    /// Checked rather than assumed. `accumulate_clamped` cannot produce an
    /// out-of-range value, so the paths this defends are the ones that do not
    /// go through it: a restore reading a corrupted section, and a future
    /// initialization policy. Both would otherwise put a value outside the
    /// clamp into the checksum and into `effective_weight`.
    pub fn bounds_violation(&self) -> Option<usize> {
        let inside = |value: &i32| (-LEARN_LIMIT_Q16..=LEARN_LIMIT_Q16).contains(value);
        (0..self.learned_q16.len()).find(|index| {
            !self.learned_q16[*index].iter().all(inside)
                || !self.trace_q16[*index].iter().all(inside)
        })
    }

    /// Hash every field under the section tag.
    ///
    /// **Destructured with no `..` (D-077).** A field added to this struct
    /// fails to compile here until it is either hashed or given an explicit
    /// reason not to be, which is what stops the next field from being world
    /// state that a restored world silently disagrees about - the defect
    /// that made two restored checksums differ in Phase 9.
    ///
    /// The byte order below is permanent: it is the definition of a
    /// plasticity world's checksum. Append, never reorder.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            learned_q16,
            trace_q16,
            faults,
            counters,
            cost_milli,
        } = self;
        hasher.update(b"lifesim-learn-state-v1");
        for (index, learned) in learned_q16.iter().enumerate() {
            // The per-organism length is hashed too, so an organism losing a
            // plastic edge is a checksum difference rather than a shift that
            // could coincidentally re-align.
            hasher.update_u64(learned.len() as u64);
            for value in learned {
                hasher.update_i32(*value);
            }
            for value in &trace_q16[index] {
                hasher.update_i32(*value);
            }
            hasher.update_u32(faults[index]);
        }
        counters.hash_into(hasher);
        hasher.update_i128(*cost_milli);
    }

    // --- Observation only (ADR-0016) -------------------------------------

    /// Plastic edges this organism carries.
    pub fn plastic_edges(&self, index: usize) -> usize {
        self.learned_q16[index].len()
    }

    /// Plastic edges across the living population.
    pub fn total_plastic_edges(&self) -> u64 {
        self.learned_q16
            .iter()
            .map(|edges| edges.len() as u64)
            .sum()
    }

    /// Mean fraction of an organism's expressed edges that are plastic, in
    /// milli.
    ///
    /// C11.2's second quantity. The denominator is supplied by the caller
    /// because it lives in the compiled plan: copying it here would create a
    /// second per-organism array to keep in lockstep, and a stale copy would
    /// make the fraction wrong in exactly the direction that looks like a
    /// result. Organisms whose plan and learned state disagree in length are
    /// a desync the invariant catches; this returns zero rather than
    /// panicking, because an observer must not be able to stop a world.
    pub fn mean_plastic_fraction_milli(&self, edges_per_organism: &[usize]) -> u64 {
        if self.learned_q16.is_empty() || edges_per_organism.len() != self.learned_q16.len() {
            return 0;
        }
        let mut total = 0_u64;
        for (index, edges) in edges_per_organism.iter().enumerate() {
            if *edges > 0 {
                total += self.learned_q16[index].len() as u64 * 1_000 / *edges as u64;
            }
        }
        total / self.learned_q16.len() as u64
    }

    /// Mean absolute learned delta over every plastic edge alive, in milli
    /// weight units.
    ///
    /// Milli weight units rather than Q16 so it is comparable with the
    /// `VALUE_LIMIT` of 8.0 that bounds it: 8000 is an edge pinned at the
    /// clamp. Zero when nothing is plastic, which is the same number a world
    /// where nothing has learned yet reports - the two are distinguished by
    /// `total_plastic_edges`, not by this.
    pub fn mean_abs_learned_milli(&self) -> u64 {
        let count = self.total_plastic_edges();
        if count == 0 {
            return 0;
        }
        let total: u64 = self
            .learned_q16
            .iter()
            .flat_map(|edges| edges.iter())
            .map(|value| u64::from(value.unsigned_abs()))
            .sum();
        total * 1_000 / 65_536 / count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plasticity::ONE_Q16;

    fn hash(state: &LearnState) -> u64 {
        let mut hasher = Fnv1a64::new();
        state.hash_into(&mut hasher);
        hasher.finish()
    }

    /// Three organisms with 1, 2 and 3 plastic edges and distinct learned
    /// values, so a mis-paired row is visible rather than coincidental.
    fn populated() -> LearnState {
        let mut state = LearnState::with_capacity(3);
        for (index, edges) in [1_usize, 2, 3].into_iter().enumerate() {
            state.push_organism(edges);
            for slot in 0..edges {
                state.learned_q16[index][slot] = (index as i32 + 1) * 1_000 + slot as i32;
                state.trace_q16[index][slot] = -(index as i32 + 1) * 10 - slot as i32;
            }
            state.faults[index] = index as u32;
        }
        state
    }

    #[test]
    fn a_new_organism_starts_at_exactly_zero_on_every_plastic_edge() {
        // C11.4 at the array level, and the reason it is an invariant rather
        // than a default: there is no way to push an organism with anything
        // else, so a birth path cannot get this wrong by forgetting a reset.
        let mut state = populated();
        assert!(state.learned_q16[2].iter().any(|value| *value != 0));
        state.push_organism(4);
        assert_eq!(state.learned_q16[3], vec![0; 4]);
        assert_eq!(state.trace_q16[3], vec![0; 4]);
        assert_eq!(state.faults[3], 0);
        // ...and the organisms that were already there are untouched.
        assert_eq!(state.learned_q16[0], vec![1_000]);
    }

    #[test]
    fn compaction_leaves_the_survivors_matching_a_state_built_from_them() {
        // Three arrays can be the right length and hold the wrong organisms'
        // contents, which is the whole failure `LearnDesync` names, so this
        // compares content rather than lengths. Keep 0 and 2, which forces
        // the last survivor to move.
        let mut state = populated();
        let mut expected = LearnState::with_capacity(2);
        for index in [0_usize, 2] {
            expected.push_organism(state.learned_q16[index].len());
            let slot = expected.len() - 1;
            expected.learned_q16[slot].clone_from(&state.learned_q16[index]);
            expected.trace_q16[slot].clone_from(&state.trace_q16[index]);
            expected.faults[slot] = state.faults[index];
        }
        assert_ne!(hash(&state), hash(&expected), "nothing was removed yet");

        state.retain(&[false, true, false]);
        assert_eq!(state.len(), 2);
        assert_eq!(hash(&state), hash(&expected));
        // The lengths moved with the contents: organism 2 had three edges and
        // is now at index 1, so a `retain` that compacted the values and left
        // the row lengths behind would fail here.
        assert_eq!(state.plastic_edges(1), 3);
        assert_eq!(state.faults[1], 2);
    }

    #[test]
    fn every_field_reaches_the_checksum() {
        let base = populated();
        let reference = hash(&base);
        let mutators: [fn(&mut LearnState); 5] = [
            |state| state.learned_q16[1][0] += 1,
            |state| state.trace_q16[1][0] += 1,
            |state| state.faults[1] += 1,
            |state| state.counters.updates_applied += 1,
            |state| state.cost_milli += 1,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut moved = base.clone();
            mutate(&mut moved);
            assert_ne!(hash(&moved), reference, "field {index} missed the hash");
        }
        // An organism gaining or losing a plastic edge must move it even when
        // the new edge is zero, which the per-row length is what guarantees.
        let mut grown = base.clone();
        grown.learned_q16[0].push(0);
        grown.trace_q16[0].push(0);
        assert_ne!(hash(&grown), reference);
    }

    #[test]
    fn the_bounds_check_finds_a_value_outside_the_clamp() {
        let mut state = populated();
        assert_eq!(state.bounds_violation(), None);
        state.learned_q16[1][1] = LEARN_LIMIT_Q16 + 1;
        assert_eq!(state.bounds_violation(), Some(1));
        state.learned_q16[1][1] = LEARN_LIMIT_Q16;
        assert_eq!(state.bounds_violation(), None, "the clamp itself is legal");
        // The trace is bounded on the same terms and is easy to leave out.
        state.trace_q16[2][0] = -LEARN_LIMIT_Q16 - 1;
        assert_eq!(state.bounds_violation(), Some(2));
    }

    #[test]
    fn the_observation_accessors_report_what_c11_2_measures() {
        let mut state = LearnState::with_capacity(2);
        state.push_organism(2);
        state.push_organism(4);
        // Half of organism 0's four edges are plastic, and all four of
        // organism 1's eight: 500 and 500 in milli, mean 500.
        assert_eq!(state.mean_plastic_fraction_milli(&[4, 8]), 500);
        assert_eq!(state.total_plastic_edges(), 6);
        // Nothing learned yet.
        assert_eq!(state.mean_abs_learned_milli(), 0);

        // One edge pinned at the clamp and one at 1.0; the other four are
        // zero, so the mean is (8000 + 1000) / 6 = 1500.
        state.learned_q16[0][0] = LEARN_LIMIT_Q16;
        state.learned_q16[1][3] = -ONE_Q16;
        assert_eq!(state.mean_abs_learned_milli(), 1_500);

        // A denominator that does not line up with the population is a
        // desync, and an observer reports zero rather than panicking.
        assert_eq!(state.mean_plastic_fraction_milli(&[4]), 0);
        assert_eq!(LearnState::default().mean_plastic_fraction_milli(&[]), 0);
    }
}
