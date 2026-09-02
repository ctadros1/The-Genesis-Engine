//! Mate choice (Phase 14, ADR-0030 decision 2, `lifesim-physiology-v2`).
//!
//! The act stays with the controller (`intent_mate`, unchanged); the taste
//! is genome (nine Trait-kind loci in the reserved preference band,
//! `genome2::PREFERENCE_TRAIT_BASE`); the engine applies the taste at
//! pairing time by scoring each eligible candidate's perceived cue values
//! against the chooser's expressed weights. An all-neutral preference
//! scores every candidate identically and the `(distance^2, id)` tie-break
//! reproduces proximity pairing exactly - which is what makes the B arm a
//! control and the enabled-but-unevolved world a no-op.
//!
//! What is state and what is cache: the two counters are world state,
//! hashed under `lifesim-matechoice-state-v1` and saved. The expressed
//! weights are a pure function of the genome, recomputed at admission and
//! on load exactly as bodies and phenotypes are - never saved, never
//! hashed.

use crate::checksum::Fnv1a64;
use crate::genome2::{Genome2, PREFERENCE_CUE_COUNT};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MateChoiceState {
    /// Cache: expressed preference weights per organism, in entity order.
    /// Pure function of the genome; recomputed on load, never saved or
    /// hashed.
    pub weights: Vec<[f32; PREFERENCE_CUE_COUNT]>,

    /// Pairings decided by preference scoring, whole run.
    pub choices_total: u64,
    /// Pairings whose candidate cue vectors were permuted before scoring
    /// (the P-scramble arm). The arm is *checked* by this counter, never
    /// merely configured (methodology review 6.4; the Phase 13 D-arm
    /// precedent).
    pub scrambled_choices_total: u64,
}

impl MateChoiceState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            weights: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.weights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.weights.is_empty()
    }

    pub fn push_organism(&mut self, genome: &Genome2) {
        self.weights.push(genome.express_preference());
    }

    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.weights[write] = self.weights[read];
                }
                write += 1;
            }
        }
        self.weights.truncate(write);
    }

    /// Only the counters enter the checksum: the weights are a derived
    /// cache, and divergent caches imply divergent genomes, which the
    /// genome section already catches (`morphstate.rs`'s argument).
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-matechoice-state-v1");
        hasher.update_u64(self.choices_total);
        hasher.update_u64(self.scrambled_choices_total);
    }
}

/// The saved half: counters only, on the terms `MorphologySaveState` is -
/// everything else in the live state is recomputed from genomes on load.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MateChoiceSave {
    pub choices_total: u64,
    pub scrambled_choices_total: u64,
}

impl MateChoiceState {
    pub fn to_save(&self) -> MateChoiceSave {
        MateChoiceSave {
            choices_total: self.choices_total,
            scrambled_choices_total: self.scrambled_choices_total,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_checksum_covers_counters_but_not_the_weights_cache() {
        let mut a = MateChoiceState::with_capacity(1);
        a.weights.push([0.25; PREFERENCE_CUE_COUNT]);
        let mut b = a.clone();
        b.weights[0][3] = -0.5;
        let hash = |state: &MateChoiceState| {
            let mut hasher = Fnv1a64::new();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&a), hash(&b));
        b.scrambled_choices_total += 1;
        assert_ne!(hash(&a), hash(&b));
    }

    #[test]
    fn retain_keeps_the_survivors_rows() {
        let mut state = MateChoiceState::with_capacity(3);
        state.weights.push([0.1; PREFERENCE_CUE_COUNT]);
        state.weights.push([0.2; PREFERENCE_CUE_COUNT]);
        state.weights.push([0.3; PREFERENCE_CUE_COUNT]);
        state.retain(&[false, true, false]);
        assert_eq!(state.len(), 2);
        assert_eq!(state.weights[1][0], 0.3);
    }
}
