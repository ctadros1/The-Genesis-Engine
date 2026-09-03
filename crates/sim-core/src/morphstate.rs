//! Per-organism morphology state (Phase 10).
//!
//! **Bodies are derived, not stored.** Development is a pure function of
//! `(genome, config)`, so a body is recomputed from the genome on load
//! exactly as a phenotype is, and the save carries none of it. That is a
//! deliberate choice recorded in ADR-0019: it avoids stacking a fourth
//! growth term onto a snapshot budget already strained by ADR-0013,
//! ADR-0014, and ADR-0015, and C10.10 verifies it rather than assuming it.
//!
//! The bodies here are therefore a **cache**, and the D-065 trap applies:
//! a field documented as derived is only safe to recompute on load if it
//! really is a pure function of state that survives the save. It is - the
//! genome is saved and the growth program is deterministic - which is
//! exactly what made the biome map unsafe and makes this safe. The
//! distinction is worth stating because the two look identical from outside.
//!
//! Only the counters enter the checksum. Bodies do not need to: a body is a
//! function of a genome, genomes are already hashed under
//! `lifesim-genome2-state-v1`, so hashing bodies too would add no
//! discriminating power. Divergent bodies imply divergent genomes and the
//! genome section catches that.

use crate::checksum::Fnv1a64;
use crate::config::MorphologyConfig;
use crate::develop::{DevelopCounters, develop};
use crate::genome2::Genome2;
use crate::morphology::{Body, BodyReference, DerivedBody, ViabilityFailure};

/// Parallel per-organism arrays, kept in lockstep with the world's primary
/// arrays exactly as every other subsystem's are.
#[derive(Clone, Debug)]
pub(crate) struct MorphologyState {
    pub bodies: Vec<Body>,
    pub derived: Vec<DerivedBody>,
    pub counters: DevelopCounters,
    /// The founder body's derived values. Computed once from the founder
    /// growth program - which is a constant of the build - so it is the same
    /// in a fresh world and a restored one without being saved.
    pub reference: BodyReference,
}

/// Derive the founder body once, for use as the neutral reference.
///
/// Pure: the founder growth program and the module registry are both build
/// constants, so this is the same value in every world and does not need
/// saving. It is computed rather than hard-coded so that changing a registry
/// coefficient re-centres the phenotype instead of silently biasing it.
pub fn founder_reference() -> BodyReference {
    let config = crate::config::MorphologyConfig::morphology_default();
    let mut counters = DevelopCounters::default();
    let body = crate::develop::grow(
        &crate::develop::founder_program(),
        config.lattice,
        &config.caps,
        &mut counters,
    );
    BodyReference::of(&body.derive())
}

impl Default for MorphologyState {
    fn default() -> Self {
        Self::with_capacity(0)
    }
}

impl MorphologyState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bodies: Vec::with_capacity(capacity),
            derived: Vec::with_capacity(capacity),
            counters: DevelopCounters::default(),
            reference: founder_reference(),
        }
    }

    pub fn len(&self) -> usize {
        self.bodies.len()
    }

    /// Develop a genome and admit the organism, or refuse it.
    ///
    /// Returns the typed failure rather than a bool, so the caller can event
    /// and count the reason. A non-viable body is a refused birth, never a
    /// repaired one.
    pub fn push_organism(
        &mut self,
        genome: &Genome2,
        config: &MorphologyConfig,
    ) -> Result<(), ViabilityFailure> {
        let body = develop(genome, config.lattice, &config.caps, &mut self.counters)?;
        self.derived.push(body.derive());
        self.bodies.push(body);
        Ok(())
    }

    /// Admit an organism whose body was already grown at pairing time.
    ///
    /// The body is not re-validated here: it was validated when it was
    /// grown, and re-running the check would either agree (wasted work) or
    /// disagree (which would mean development is not a pure function, and
    /// the right response to that is C10.1's test failing, not a silent
    /// second opinion at the birth path).
    pub fn push_body(&mut self, body: Body) {
        self.derived.push(body.derive());
        self.bodies.push(body);
    }

    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.bodies.swap(write, read);
                    self.derived.swap(write, read);
                }
                write += 1;
            }
        }
        self.bodies.truncate(write);
        self.derived.truncate(write);
    }

    /// Energy capacity for one organism: tissue plus storage, wholly
    /// derived from the body with no config floor.
    pub fn energy_capacity_milli(&self, index: usize) -> i64 {
        self.derived[index].energy_capacity_milli
    }

    pub fn mean_modules_milli(&self) -> u64 {
        if self.bodies.is_empty() {
            return 0;
        }
        let total: u64 = self.bodies.iter().map(|body| body.len() as u64).sum();
        total * 1_000 / self.bodies.len() as u64
    }

    /// Median module count among the living.
    ///
    /// The same argument as C9.1's median: evolved body sizes are expected
    /// to be right-skewed, so a mean can describe no organism actually
    /// alive. Founders are one module, so a median above one means half the
    /// population has grown past the founding body.
    pub fn median_modules(&self) -> u64 {
        if self.bodies.is_empty() {
            return 0;
        }
        let mut counts: Vec<usize> = self.bodies.iter().map(|body| body.len()).collect();
        counts.sort_unstable();
        counts[(counts.len() - 1) / 2] as u64
    }

    /// Distinct module-count/type-signature pairs among the living. C10.3's
    /// divergence measure, and deliberately not a module count on its own:
    /// A13 says novelty is not progress, and two bodies of equal size with
    /// different tissue are genuinely different morphologies.
    pub fn distinct_morphologies(&self) -> usize {
        let mut seen: Vec<u64> = self
            .bodies
            .iter()
            .map(|body| {
                let mut hasher = Fnv1a64::new();
                body.hash_into(&mut hasher);
                hasher.finish()
            })
            .collect();
        seen.sort_unstable();
        seen.dedup();
        seen.len()
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        self.counters.hash_into(hasher);
    }
}
