//! Ontogeny (Phase 14, ADR-0030, `lifesim-physiology-v2`).
//!
//! The developed body is revealed one module at a time in **canonical BFS
//! order from the origin module**, each activation paid through the ledger.
//! Development itself still runs entirely at birth (`develop.rs`); ontogeny
//! never re-runs it, and the adult body an organism is growing toward is
//! fixed the moment it is born.
//!
//! Why BFS order and not the growth program's emission order: the body's
//! representation is deliberately order-free (`Body::from_modules` sorts
//! into canonical lattice order - the C10.1 property), so emission order
//! does not survive into the body, and an emission-order prefix has no
//! connectivity guarantee anyway. A BFS prefix is connected by
//! construction, so every partially grown body satisfies the same
//! contiguity a full body does, and the traversal is a pure function of
//! the body - the growth *order* needs no saving, only the progress does.
//!
//! What is state and what is cache, exactly (the D-065 distinction):
//! `grown_modules` and `growth_paid_milli` depend on the energy history and
//! are real state - saved, restored, and hashed under
//! `lifesim-ontogeny-state-v1`. `order` and `derived_grown` are pure
//! functions of the saved body and the saved progress, recomputed on load
//! exactly as bodies themselves are (`morphstate.rs`).
//!
//! The controller is the recorded exception to "juveniles are their grown
//! prefix": it is compiled at birth from the adult body's neural budget and
//! does not grow, because lifetime topology change is a first-policy
//! non-goal of the learning stack (neuroevolution review 10.3; ADR-0030
//! records the deviation).

use crate::checksum::Fnv1a64;
use crate::morphology::{Body, DerivedBody, LatticeKind};

/// Canonical growth order: breadth-first from the body's first module (the
/// lowest canonical lattice index, which for a body grown from a single
/// origin is the origin region), neighbours visited in ascending direction
/// index. Every module is reachable because the body validated connected.
pub fn growth_order(body: &Body, lattice: LatticeKind) -> Vec<u16> {
    let modules = body.modules();
    let mut order = Vec::with_capacity(modules.len());
    let mut reached = vec![false; modules.len()];
    let mut queue = std::collections::VecDeque::with_capacity(modules.len());
    if modules.is_empty() {
        return order;
    }
    reached[0] = true;
    queue.push_back(0_usize);
    while let Some(index) = queue.pop_front() {
        order.push(index as u16);
        let position = modules[index].position;
        for direction in 0..lattice.neighbour_count() {
            let neighbour = position.step(lattice, direction);
            if let Some(found) = modules
                .iter()
                .position(|module| module.position == neighbour)
                && !reached[found]
            {
                reached[found] = true;
                queue.push_back(found);
            }
        }
    }
    order
}

/// Derived attributes of the first `grown` modules of `order`.
pub fn derive_prefix(body: &Body, order: &[u16], grown: u32) -> DerivedBody {
    let mut mask = vec![false; body.len()];
    for &index in order.iter().take(grown as usize) {
        mask[usize::from(index)] = true;
    }
    body.derive_masked(&mask)
}

/// Per-organism ontogeny state plus the section's own counters.
///
/// `None` exactly when the gate is off, so a disabled world takes the
/// existing code paths and reproduces the Phase 13 fixture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OntogenyState {
    /// Modules grown: a prefix length of the canonical growth order.
    /// Equal to the body's module count when fully grown.
    pub grown_modules: Vec<u32>,
    /// Paid toward the next module, milli-EU. Fixed point per Rule 7: it
    /// accumulates across ticks. Zero when fully grown.
    pub growth_paid_milli: Vec<i64>,

    /// Cache: the canonical growth order per organism. Pure function of
    /// the body; recomputed on load, never saved or hashed.
    pub order: Vec<Vec<u16>>,
    /// Cache: derived attributes of the grown prefix. Pure function of the
    /// body and `grown_modules`; recomputed on load, never saved or hashed.
    pub derived_grown: Vec<DerivedBody>,

    /// Module activations, all organisms, whole run.
    pub modules_grown_total: u64,
    /// Whole milli-EU debited for growth; the ledger's `spent_milli` gets
    /// the same amount, so attribution and the ledger agree exactly.
    pub growth_spent_milli_total: i128,
}

impl OntogenyState {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            grown_modules: Vec::with_capacity(capacity),
            growth_paid_milli: Vec::with_capacity(capacity),
            order: Vec::with_capacity(capacity),
            derived_grown: Vec::with_capacity(capacity),
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.grown_modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.grown_modules.is_empty()
    }

    /// Admit an organism with `grown` modules already active. Founders are
    /// admitted fully grown (they seed the population, as in every phase
    /// before this one); children at the configured birth minimum.
    pub fn push_organism(&mut self, body: &Body, lattice: LatticeKind, grown: u32) {
        let order = growth_order(body, lattice);
        let grown = grown.min(body.len() as u32);
        let derived = derive_prefix(body, &order, grown);
        self.grown_modules.push(grown);
        self.growth_paid_milli.push(0);
        self.order.push(order);
        self.derived_grown.push(derived);
    }

    pub fn fully_grown(&self, index: usize, body_modules: u32) -> bool {
        self.grown_modules[index] >= body_modules
    }

    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.grown_modules[write] = self.grown_modules[read];
                    self.growth_paid_milli[write] = self.growth_paid_milli[read];
                    self.order.swap(write, read);
                    self.derived_grown[write] = self.derived_grown[read];
                }
                write += 1;
            }
        }
        self.grown_modules.truncate(write);
        self.growth_paid_milli.truncate(write);
        self.order.truncate(write);
        self.derived_grown.truncate(write);
    }

    /// Only the saved fields and the counters enter the checksum: `order`
    /// and `derived_grown` are derived caches, and hashing them would add
    /// no discriminating power for the same reason bodies are not hashed
    /// (`morphstate.rs` - divergent caches imply divergent saved state,
    /// which the saved fields already catch).
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-ontogeny-state-v1");
        for &grown in &self.grown_modules {
            hasher.update_u32(grown);
        }
        for &paid in &self.growth_paid_milli {
            hasher.update_i64(paid);
        }
        hasher.update_u64(self.modules_grown_total);
        hasher.update_i128(self.growth_spent_milli_total);
    }
}

/// The saved half of [`OntogenyState`]: progress and counters, no caches.
///
/// A save-shaped twin rather than the live type, because the live type
/// carries derived caches (`order`, `derived_grown`) that must not be
/// trusted from a file - the `worldmod`/`social` live-type reuse applies
/// only to cache-free types, and this is not one.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OntogenySave {
    pub grown_modules: Vec<u32>,
    pub growth_paid_milli: Vec<i64>,
    pub modules_grown_total: u64,
    pub growth_spent_milli_total: i128,
}

impl OntogenyState {
    pub fn to_save(&self) -> OntogenySave {
        OntogenySave {
            grown_modules: self.grown_modules.clone(),
            growth_paid_milli: self.growth_paid_milli.clone(),
            modules_grown_total: self.modules_grown_total,
            growth_spent_milli_total: self.growth_spent_milli_total,
        }
    }

    /// Rebuild the live state from a save against the restored bodies.
    ///
    /// Untrusted input, checked structurally by name before it reaches a
    /// world: lengths against the population, every prefix against its own
    /// body, every payment non-negative and zero once fully grown (there
    /// is no next module to have paid toward).
    pub fn from_save(
        save: OntogenySave,
        bodies: &[Body],
        lattice: LatticeKind,
    ) -> Result<Self, &'static str> {
        if save.grown_modules.len() != bodies.len()
            || save.growth_paid_milli.len() != bodies.len()
        {
            return Err("ontogeny arrays do not match the population");
        }
        let mut state = Self::with_capacity(bodies.len());
        for (index, body) in bodies.iter().enumerate() {
            let grown = save.grown_modules[index];
            let paid = save.growth_paid_milli[index];
            if grown == 0 || grown > body.len() as u32 {
                return Err("grown_modules outside 1..=body modules");
            }
            if paid < 0 {
                return Err("growth_paid_milli is negative");
            }
            if grown == body.len() as u32 && paid != 0 {
                return Err("growth_paid_milli nonzero on a fully grown body");
            }
            let order = growth_order(body, lattice);
            let derived = derive_prefix(body, &order, grown);
            state.grown_modules.push(grown);
            state.growth_paid_milli.push(paid);
            state.order.push(order);
            state.derived_grown.push(derived);
        }
        state.modules_grown_total = save.modules_grown_total;
        state.growth_spent_milli_total = save.growth_spent_milli_total;
        Ok(state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::morphology::{LatticePos, Module, ModuleType};

    fn module(q: i16, r: i16, module_type: ModuleType) -> Module {
        Module {
            position: LatticePos { q, r },
            module_type,
            scale_milli: 1_000,
            orientation: 0,
            source_locus: 7,
        }
    }

    fn line_body() -> Body {
        // Three modules in a row; canonical order puts (-1,0) first, so the
        // BFS root is the leftmost and the order walks right.
        Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 0, ModuleType::Motor),
                module(-1, 0, ModuleType::Sensory),
            ],
            4,
        )
    }

    #[test]
    fn the_growth_order_is_a_connected_bfs_over_every_module() {
        let body = line_body();
        let order = growth_order(&body, LatticeKind::Square);
        assert_eq!(order.len(), body.len());
        // Every prefix of the order is connected: each later module is a
        // lattice neighbour of some earlier one.
        let modules = body.modules();
        for (position_in_order, &module_index) in order.iter().enumerate().skip(1) {
            let position = modules[usize::from(module_index)].position;
            let touches_earlier = order[..position_in_order].iter().any(|&earlier| {
                let earlier = modules[usize::from(earlier)].position;
                (0..LatticeKind::Square.neighbour_count())
                    .any(|direction| earlier.step(LatticeKind::Square, direction) == position)
            });
            assert!(touches_earlier, "prefix disconnected at {position_in_order}");
        }
    }

    #[test]
    fn the_full_prefix_derivation_equals_the_whole_body_derivation() {
        let body = line_body();
        let order = growth_order(&body, LatticeKind::Square);
        let full = derive_prefix(&body, &order, body.len() as u32);
        assert_eq!(full, body.derive());
    }

    #[test]
    fn a_partial_prefix_carries_only_its_own_modules_attributes() {
        let body = line_body();
        let order = growth_order(&body, LatticeKind::Square);
        let one = derive_prefix(&body, &order, 1);
        assert_eq!(one.modules, 1);
        // The BFS root is the canonical first module, the sensory one at
        // (-1,0): a one-module juvenile of this body senses and cannot move
        // or eat.
        assert!(one.sensor_range_milli > 0);
        assert_eq!(one.thrust_milli, 0);
        assert_eq!(one.intake_milli, 0);
        let two = derive_prefix(&body, &order, 2);
        assert!(two.mass_milli > one.mass_milli);
    }

    #[test]
    fn push_organism_clamps_grown_to_the_body_and_caches_the_prefix() {
        let body = line_body();
        let mut state = OntogenyState::with_capacity(1);
        state.push_organism(&body, LatticeKind::Square, 99);
        assert_eq!(state.grown_modules[0], body.len() as u32);
        assert_eq!(state.derived_grown[0], body.derive());
        assert!(state.fully_grown(0, body.len() as u32));
    }

    #[test]
    fn the_checksum_covers_progress_and_counters_but_not_the_caches() {
        let body = line_body();
        let mut a = OntogenyState::with_capacity(1);
        a.push_organism(&body, LatticeKind::Square, 1);
        let mut b = a.clone();
        // Divergent cache alone must not change the hash...
        b.derived_grown[0].mass_milli += 1;
        let hash = |state: &OntogenyState| {
            let mut hasher = Fnv1a64::default();
            state.hash_into(&mut hasher);
            hasher.finish()
        };
        assert_eq!(hash(&a), hash(&b));
        // ...while divergent saved progress must.
        b.growth_paid_milli[0] += 1;
        assert_ne!(hash(&a), hash(&b));
    }
}
