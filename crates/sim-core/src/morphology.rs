//! Modular morphology: typed modules on a discrete lattice (Phase 10).
//!
//! Full design in `specifications/morphology-and-development.md`; decision in
//! ADR-0019. This module is the *representation* and the *derivation*: what a
//! body is, and what body implies about an organism. The growth program that
//! produces a body from a genome lives in `develop.rs`.
//!
//! Three properties carry the design.
//!
//! **Integer coordinates, everywhere.** A lattice position is a pair of
//! integers and nothing in morphology is a float, so a body is exactly
//! representable, exactly hashable, and exactly comparable. No geometric
//! predicate anywhere needs a tolerance.
//!
//! **Every sum iterates in ascending lattice index.** Derived attributes are
//! sums over modules, and float addition is not associative, so the iteration
//! order is pinned by construction exactly as `determinism-extensions.md`
//! Rule 6 pins per-node edge summation. In practice the derivation is all
//! integer arithmetic, which makes the order irrelevant to the result - but
//! the order is still fixed, because "it happens to be integer today" is not
//! a property a later change preserves.
//!
//! **A one-module body is legal.** That is the whole point of the
//! representation and not an edge case: a unicell is one undifferentiated
//! module, multicellularity is more than one module with more than one type,
//! and the difference is a region of the same morphospace rather than a
//! mechanic. Nothing here detects or rewards the transition (ADR-0012).
//!
//! This is deliberately **not** a physical body simulation. Modules confer
//! capability and cost; they do not swing, bend, or collide. That boundary is
//! what keeps determinism exact and cost tractable, and it is the line
//! ADR-0019 draws.

use crate::checksum::Fnv1a64;

pub const MORPHOLOGY_POLICY_VERSION: &str = "lifesim-morphology-v1";

/// Bumped whenever a module type is added or a coefficient changes. Enters
/// the config hash, because a body means something different under a
/// different registry.
pub const MODULE_REGISTRY_VERSION: u16 = 1;

/// Module type codes are **permanent**, like RNG stream values and operator
/// codes: they are encoded in genomes and feed derived identity, so
/// renumbering one would silently change every body ever grown.
pub const TYPE_STRUCTURAL: u8 = 0;
pub const TYPE_SENSORY: u8 = 1;
pub const TYPE_MOTOR: u8 = 2;
pub const TYPE_DIGESTIVE: u8 = 3;
pub const TYPE_STORAGE: u8 = 4;
pub const TYPE_REPRODUCTIVE: u8 = 5;
pub const TYPE_NEURAL: u8 = 6;
pub const MODULE_TYPE_COUNT: usize = 7;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ModuleType {
    Structural,
    Sensory,
    Motor,
    Digestive,
    Storage,
    Reproductive,
    Neural,
}

impl ModuleType {
    pub const ALL: [ModuleType; MODULE_TYPE_COUNT] = [
        ModuleType::Structural,
        ModuleType::Sensory,
        ModuleType::Motor,
        ModuleType::Digestive,
        ModuleType::Storage,
        ModuleType::Reproductive,
        ModuleType::Neural,
    ];

    pub fn id(self) -> u8 {
        match self {
            ModuleType::Structural => TYPE_STRUCTURAL,
            ModuleType::Sensory => TYPE_SENSORY,
            ModuleType::Motor => TYPE_MOTOR,
            ModuleType::Digestive => TYPE_DIGESTIVE,
            ModuleType::Storage => TYPE_STORAGE,
            ModuleType::Reproductive => TYPE_REPRODUCTIVE,
            ModuleType::Neural => TYPE_NEURAL,
        }
    }

    /// Fail-closed: an unknown code is refused, never defaulted to
    /// `Structural`. A body assembled from a code this build does not know
    /// is not a body this build can price.
    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            TYPE_STRUCTURAL => ModuleType::Structural,
            TYPE_SENSORY => ModuleType::Sensory,
            TYPE_MOTOR => ModuleType::Motor,
            TYPE_DIGESTIVE => ModuleType::Digestive,
            TYPE_STORAGE => ModuleType::Storage,
            TYPE_REPRODUCTIVE => ModuleType::Reproductive,
            TYPE_NEURAL => ModuleType::Neural,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            ModuleType::Structural => "structural",
            ModuleType::Sensory => "sensory",
            ModuleType::Motor => "motor",
            ModuleType::Digestive => "digestive",
            ModuleType::Storage => "storage",
            ModuleType::Reproductive => "reproductive",
            ModuleType::Neural => "neural",
        }
    }

    fn entry(self) -> &'static TypeEntry {
        &REGISTRY[self.id() as usize]
    }
}

/// What one module type costs and what it confers, per unit of scale-cubed.
///
/// These are **authored physics**: a price list, not a ranking. Nothing here
/// says a body is good, and no coefficient was chosen to make a particular
/// morphology win. The trade-offs are meant to be real in both directions -
/// motors buy speed and cost the most to run, storage is heavy and cheap to
/// keep, neural tissue is light and the most expensive thing an organism can
/// carry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TypeEntry {
    /// Mass per unit scale-cubed, milli.
    pub density_milli: i64,
    /// Basal upkeep per unit scale-cubed, milli-EU per second.
    pub upkeep_milli: i64,
    /// Capability per unit scale-cubed, in whatever unit the type confers.
    /// Zero for types that confer no scalar capability.
    pub capability_milli: i64,
    /// Upkeep grows with the fourth power of scale rather than the third.
    ///
    /// True only for `Neural`, and it is the specification's "upkeep,
    /// disproportionate at scale". A brain that is twice the size costs
    /// sixteen times as much to run rather than eight, so cognition is
    /// expensive *structurally* rather than by a stipulated penalty.
    pub superlinear_upkeep: bool,
}

/// The registry, indexed by type code. Order matches the code values and is
/// asserted by a test rather than by comment.
const REGISTRY: [TypeEntry; MODULE_TYPE_COUNT] = [
    // Structural: the cheapest way to occupy space and connect things.
    TypeEntry {
        density_milli: 1_000,
        upkeep_milli: 100,
        capability_milli: 0,
        superlinear_upkeep: false,
    },
    // Sensory: light, moderate upkeep, confers sensing range.
    TypeEntry {
        density_milli: 700,
        upkeep_milli: 150,
        capability_milli: 6_000,
        superlinear_upkeep: false,
    },
    // Motor: heavy and expensive to run, confers thrust.
    TypeEntry {
        density_milli: 900,
        upkeep_milli: 400,
        capability_milli: 2_400,
        superlinear_upkeep: false,
    },
    // Digestive: confers intake rate.
    TypeEntry {
        density_milli: 800,
        upkeep_milli: 200,
        capability_milli: 1_000,
        superlinear_upkeep: false,
    },
    // Storage: the heaviest tissue and the cheapest to maintain, which is
    // what makes it a real trade against motors rather than free capacity.
    TypeEntry {
        density_milli: 1_200,
        upkeep_milli: 80,
        capability_milli: 30_000,
        superlinear_upkeep: false,
    },
    // Reproductive: confers per-offspring investment capacity.
    TypeEntry {
        density_milli: 900,
        upkeep_milli: 250,
        capability_milli: 4_000,
        superlinear_upkeep: false,
    },
    // Neural: light tissue, the most expensive upkeep, and the only type
    // whose cost is superlinear in scale. Confers controller node budget.
    TypeEntry {
        density_milli: 600,
        upkeep_milli: 500,
        capability_milli: 4_000,
        superlinear_upkeep: true,
    },
];

pub fn registry_entry(module_type: ModuleType) -> &'static TypeEntry {
    module_type.entry()
}

/// Which lattice a world uses. Fixed per world and in the config hash: a
/// body means something different on a different lattice, because adjacency
/// is what connectivity means.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LatticeKind {
    /// Four neighbours. Von Neumann, not Moore: diagonal contact is not
    /// connection, so a body cannot be held together by corners.
    Square,
    /// Six neighbours, axial coordinates. Denser packing and no
    /// corner-adjacency question to answer in the first place.
    Hex,
}

impl LatticeKind {
    pub fn id(self) -> u8 {
        match self {
            LatticeKind::Square => 0,
            LatticeKind::Hex => 1,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Some(match id {
            0 => LatticeKind::Square,
            1 => LatticeKind::Hex,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            LatticeKind::Square => "square",
            LatticeKind::Hex => "hex",
        }
    }

    pub fn neighbour_count(self) -> usize {
        match self {
            LatticeKind::Square => 4,
            LatticeKind::Hex => 6,
        }
    }

    /// Neighbour offsets in a fixed canonical order. The order is part of the
    /// policy version: a growth program names a direction by index, so
    /// reordering these would re-point every direction every genome ever
    /// encoded.
    pub fn offsets(self) -> &'static [(i16, i16)] {
        match self {
            LatticeKind::Square => &[(1, 0), (0, 1), (-1, 0), (0, -1)],
            LatticeKind::Hex => &[(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)],
        }
    }
}

/// An integer lattice position. Axial coordinates under `Hex`, ordinary
/// column/row under `Square`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Default)]
pub struct LatticePos {
    pub q: i16,
    pub r: i16,
}

impl LatticePos {
    pub const ORIGIN: Self = Self { q: 0, r: 0 };

    pub fn new(q: i16, r: i16) -> Self {
        Self { q, r }
    }

    pub fn step(self, lattice: LatticeKind, direction: usize) -> Self {
        let offsets = lattice.offsets();
        let (dq, dr) = offsets[direction % offsets.len()];
        Self {
            q: self.q.saturating_add(dq),
            r: self.r.saturating_add(dr),
        }
    }

    /// Canonical index within the bounding square of the given radius. This
    /// is the ordering **every** sum over modules uses.
    ///
    /// Returns `None` outside the radius, which is how the lattice bound is
    /// enforced: a position with no index is not a position a body may
    /// occupy, so out-of-bounds growth is refused rather than wrapped or
    /// clamped. Wrapping would silently fold a body onto itself and clamping
    /// would silently stack modules.
    pub fn index(self, radius: i16) -> Option<u32> {
        if self.q < -radius || self.q > radius || self.r < -radius || self.r > radius {
            return None;
        }
        let span = i32::from(radius) * 2 + 1;
        let q = i32::from(self.q) + i32::from(radius);
        let r = i32::from(self.r) + i32::from(radius);
        Some((r * span + q) as u32)
    }
}

/// One module of a body.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Module {
    pub position: LatticePos,
    pub module_type: ModuleType,
    /// Scale, milli. Clamped into `MIN_SCALE_MILLI..=MAX_SCALE_MILLI`.
    pub scale_milli: u16,
    /// Facing, an index into the lattice's neighbour offsets. Carried and
    /// inherited; it steers growth and confers nothing on its own.
    pub orientation: u8,
    /// **Provenance**: the `homology_id` of the regulatory locus that emitted
    /// this module.
    ///
    /// Required rather than convenient. `genetics` 1.6 asks a developmental
    /// encoding for "provenance links from each emitted module back to the
    /// locus that generated it", and without it a body is an opaque output:
    /// no analysis can attribute a morphological change to the mutation that
    /// caused it, which is most of what makes an indirect encoding
    /// answerable at all.
    pub source_locus: u32,
}

pub const MIN_SCALE_MILLI: u16 = 500;
pub const MAX_SCALE_MILLI: u16 = 2_000;

/// Energy capacity conferred by ordinary tissue, per 1000 units of mass.
///
/// **Every body holds energy, not just bodies with storage organs.** An
/// earlier version derived capacity from storage modules alone, which gave a
/// storage-less body a capacity of zero - so every founder was instantly
/// over its own limit and the world could not start. Adding a config floor
/// would have papered over it with a magic constant; deriving from mass
/// instead is both physical and removes the constant, because tissue really
/// does hold energy and a bigger organism really does hold more.
///
/// Calibrated so a founder - one digestive module at unit scale, mass 800 -
/// lands at 12,000 milli-EU, which is exactly the global `energy_max_milli`
/// default a schema-2 organism gets. A morphology world therefore starts
/// from the same energetics and diverges only as bodies do.
pub const TISSUE_CAPACITY_PER_MASS_MILLI: i64 = 15_000;

impl Module {
    /// Scale cubed, in milli units: `(scale/1000)^3 * 1000`.
    ///
    /// Exact integer arithmetic in i64. At the maximum scale this is
    /// `2000^3 / 10^6 = 8000`, so the intermediate `scale^3` reaches 8e9 and
    /// needs the widening that i64 gives it.
    pub fn scale_cubed_milli(&self) -> i64 {
        let scale = i64::from(self.scale_milli);
        scale * scale * scale / 1_000_000
    }

    /// Scale to the fourth, milli: the neural upkeep curve.
    pub fn scale_fourth_milli(&self) -> i64 {
        self.scale_cubed_milli() * i64::from(self.scale_milli) / 1_000
    }

    pub fn mass_milli(&self) -> i64 {
        self.module_type.entry().density_milli * self.scale_cubed_milli() / 1_000
    }

    pub fn upkeep_milli(&self) -> i64 {
        let entry = self.module_type.entry();
        let scaled = if entry.superlinear_upkeep {
            self.scale_fourth_milli()
        } else {
            self.scale_cubed_milli()
        };
        entry.upkeep_milli * scaled / 1_000
    }

    pub fn capability_milli(&self) -> i64 {
        self.module_type.entry().capability_milli * self.scale_cubed_milli() / 1_000
    }
}

/// Why a grown body is not viable.
///
/// Typed and counted, never repaired. A body that fails is a **non-viable
/// organism** and the birth is rejected, exactly as a capacity rejection is;
/// there is no path that patches a disconnected body into a connected one,
/// because the repaired body is one no genome encoded.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ViabilityFailure {
    /// No modules at all.
    Empty,
    /// Two or more modules with no connected path between them. Checked by
    /// flood fill from the origin module over lattice adjacency.
    Disconnected,
    /// More modules than `max_modules`.
    TooManyModules,
    /// A module outside the lattice radius.
    OutOfBounds,
    /// Two modules on the same lattice cell.
    Overlap,
    /// The config requires at least one module of a type the body lacks.
    MissingRequiredType(ModuleType),
    /// A scale outside the clamp.
    ScaleOutOfRange,
}

impl ViabilityFailure {
    pub fn name(self) -> &'static str {
        match self {
            ViabilityFailure::Empty => "empty",
            ViabilityFailure::Disconnected => "disconnected",
            ViabilityFailure::TooManyModules => "too_many_modules",
            ViabilityFailure::OutOfBounds => "out_of_bounds",
            ViabilityFailure::Overlap => "overlap",
            ViabilityFailure::MissingRequiredType(_) => "missing_required_type",
            ViabilityFailure::ScaleOutOfRange => "scale_out_of_range",
        }
    }
}

/// Structural bounds on a body. Set from the C10.8 measurement, not before
/// it; the values here are the starting point that measurement replaces.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MorphologyCaps {
    pub max_modules: u16,
    pub lattice_radius: i16,
    pub max_growth_steps: u16,
    /// Types every viable body must carry at least one of. A bitmask over
    /// type codes rather than a list, so it is a fixed-width config field.
    pub required_types_mask: u8,
}

impl MorphologyCaps {
    pub fn provisional() -> Self {
        Self {
            max_modules: 64,
            lattice_radius: 8,
            max_growth_steps: 16,
            // **Empty by default: nothing is required, and that is a
            // deliberate reversal.**
            //
            // Requiring a digestive module made 84 percent of randomly drawn
            // growth programs non-viable, all for the same reason - and a
            // body with no gut is not *invalid*, it is *unfit*. Encoding that
            // as a validity rule authors the outcome: it decides by fiat what
            // ecology would decide by starvation, and it does so before
            // selection ever sees the organism. A gutless body has zero
            // intake, starves, and leaves no descendants, which is the same
            // answer arrived at by physics rather than by decree
            // (ADR-0012, ADR-0018).
            //
            // The mask stays configurable because the specification asks for
            // it and a later phase may want a genuinely structural
            // requirement. Non-viability is then reserved for bodies that are
            // structurally impossible - empty, disconnected, overlapping,
            // out of bounds - rather than merely doomed.
            required_types_mask: 0,
        }
    }

    pub fn requires(&self, module_type: ModuleType) -> bool {
        self.required_types_mask & (1 << module_type.id()) != 0
    }
}

/// A grown body: modules in ascending lattice index, always.
///
/// The invariant is maintained by construction rather than checked on
/// access, so every sum below is automatically in canonical order.
#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct Body {
    modules: Vec<Module>,
}

impl Body {
    /// Build from an unordered module list, sorting into canonical order.
    ///
    /// Sorting here rather than trusting the caller is what makes
    /// order-independence a property of the type instead of a discipline:
    /// C10.1 requires that permuting the storage order of the regulatory
    /// loci that produced these modules cannot change the body, and the
    /// cheapest way to guarantee that is to make the body's own
    /// representation order-free.
    pub fn from_modules(mut modules: Vec<Module>, radius: i16) -> Self {
        modules.sort_by_key(|module| {
            (
                module.position.index(radius).unwrap_or(u32::MAX),
                module.position.q,
                module.position.r,
            )
        });
        Self { modules }
    }

    pub fn modules(&self) -> &[Module] {
        &self.modules
    }

    pub fn len(&self) -> usize {
        self.modules.len()
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty()
    }

    pub fn count_of(&self, module_type: ModuleType) -> usize {
        self.modules
            .iter()
            .filter(|module| module.module_type == module_type)
            .count()
    }

    pub fn occupied(&self, position: LatticePos) -> bool {
        self.modules
            .iter()
            .any(|module| module.position == position)
    }

    /// Validate every structural invariant, returning the **first** failure
    /// in a fixed order so the reported reason is deterministic.
    pub fn validate(
        &self,
        lattice: LatticeKind,
        caps: &MorphologyCaps,
    ) -> Result<(), ViabilityFailure> {
        if self.modules.is_empty() {
            return Err(ViabilityFailure::Empty);
        }
        if self.modules.len() > usize::from(caps.max_modules) {
            return Err(ViabilityFailure::TooManyModules);
        }
        for module in &self.modules {
            if module.position.index(caps.lattice_radius).is_none() {
                return Err(ViabilityFailure::OutOfBounds);
            }
            if module.scale_milli < MIN_SCALE_MILLI || module.scale_milli > MAX_SCALE_MILLI {
                return Err(ViabilityFailure::ScaleOutOfRange);
            }
        }
        // Canonical order makes duplicate positions adjacent.
        for pair in self.modules.windows(2) {
            if pair[0].position == pair[1].position {
                return Err(ViabilityFailure::Overlap);
            }
        }
        if !self.is_connected(lattice) {
            return Err(ViabilityFailure::Disconnected);
        }
        for module_type in ModuleType::ALL {
            if caps.requires(module_type) && self.count_of(module_type) == 0 {
                return Err(ViabilityFailure::MissingRequiredType(module_type));
            }
        }
        Ok(())
    }

    /// Flood fill from the first module over lattice adjacency.
    ///
    /// A one-module body is trivially connected, which is the unicellular
    /// case and is deliberately not special-cased.
    fn is_connected(&self, lattice: LatticeKind) -> bool {
        let mut reached = vec![false; self.modules.len()];
        let mut stack = vec![0_usize];
        reached[0] = true;
        let mut seen = 1_usize;
        while let Some(index) = stack.pop() {
            let position = self.modules[index].position;
            for direction in 0..lattice.neighbour_count() {
                let neighbour = position.step(lattice, direction);
                if let Some(found) = self
                    .modules
                    .iter()
                    .position(|module| module.position == neighbour)
                    && !reached[found]
                {
                    reached[found] = true;
                    seen += 1;
                    stack.push(found);
                }
            }
        }
        seen == self.modules.len()
    }

    /// Every derived attribute, in one pass over modules in ascending
    /// lattice index.
    pub fn derive(&self) -> DerivedBody {
        self.derive_where(|_| true)
    }

    /// Derived attributes of the masked subset of modules, in the same
    /// ascending-lattice-index pass `derive` makes. Phase 14 ontogeny uses
    /// this for the grown prefix; `derive` is the all-true case, so there
    /// is exactly one accumulation to keep correct.
    pub fn derive_masked(&self, mask: &[bool]) -> DerivedBody {
        self.derive_where(|index| mask.get(index).copied().unwrap_or(false))
    }

    fn derive_where(&self, include: impl Fn(usize) -> bool) -> DerivedBody {
        let mut derived = DerivedBody::default();
        let mut included = 0_u32;
        for (index, module) in self.modules.iter().enumerate() {
            if !include(index) {
                continue;
            }
            included += 1;
            derived.mass_milli += module.mass_milli();
            derived.basal_cost_milli += module.upkeep_milli();
            let capability = module.capability_milli();
            match module.module_type {
                ModuleType::Structural => {}
                ModuleType::Sensory => {
                    derived.sensor_range_milli = derived.sensor_range_milli.max(capability);
                    derived.sensory_modules += 1;
                }
                ModuleType::Motor => derived.thrust_milli += capability,
                ModuleType::Digestive => derived.intake_milli += capability,
                ModuleType::Storage => derived.storage_capacity_milli += capability,
                ModuleType::Reproductive => derived.invest_capacity_milli += capability,
                ModuleType::Neural => derived.node_budget_milli += capability,
            }
        }
        derived.modules = included;
        // Tissue capacity is a function of the whole body, so it is computed
        // after the loop rather than accumulated inside it.
        derived.energy_capacity_milli = derived.mass_milli * TISSUE_CAPACITY_PER_MASS_MILLI / 1_000
            + derived.storage_capacity_milli;
        derived
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        hasher.update(b"lifesim-morphology-body-v1");
        hasher.update_u64(self.modules.len() as u64);
        for module in &self.modules {
            hasher.update_u64(module.position.q as u64);
            hasher.update_u64(module.position.r as u64);
            hasher.update_u64(u64::from(module.module_type.id()));
            hasher.update_u64(u64::from(module.scale_milli));
            hasher.update_u64(u64::from(module.orientation));
            hasher.update_u64(u64::from(module.source_locus));
        }
    }
}

/// What a body implies. All integer; no attribute here is a gene.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DerivedBody {
    pub modules: u32,
    pub mass_milli: i64,
    pub basal_cost_milli: i64,
    /// Total motor thrust. Speed is thrust over mass, computed by
    /// [`Self::max_speed_milli`] rather than stored, because it is a ratio of
    /// two things already here.
    pub thrust_milli: i64,
    /// Range of the **best** sensory module rather than the sum: sensing
    /// further is a property of one organ, not of having several. Several
    /// sensory modules still cost their mass and upkeep, so redundant eyes
    /// are a real loss rather than a free stack.
    pub sensor_range_milli: i64,
    pub sensory_modules: u32,
    pub intake_milli: i64,
    /// Capacity from storage modules alone. Reported separately from the
    /// total so an analysis can tell a body that invested in storage from
    /// one that is merely large.
    pub storage_capacity_milli: i64,
    /// Total energy capacity: tissue plus storage.
    pub energy_capacity_milli: i64,
    pub invest_capacity_milli: i64,
    pub node_budget_milli: i64,
}

/// The founder body's own derived values, used as the neutral point every
/// phenotype multiplier is expressed against.
///
/// **Self-calibrating on purpose.** Every derived attribute is a ratio to
/// this reference, so a founder lands mid-range on all of them and any
/// deviation is a genuine consequence of a *different* body. Hard-coding the
/// references instead would silently become a handicap the moment a registry
/// coefficient or the founder program changed - which is the failure this
/// phase already hit twice, once on energy capacity (D-083) and once on
/// basal cost, speed, and body scale together.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BodyReference {
    pub mass_milli: i64,
    pub upkeep_milli: i64,
    pub intake_milli: i64,
    /// Thrust per unit mass, milli. What speed is a ratio of.
    pub thrust_ratio_milli: i64,
}

impl BodyReference {
    pub fn of(derived: &DerivedBody) -> Self {
        Self {
            mass_milli: derived.mass_milli.max(1),
            upkeep_milli: derived.basal_cost_milli.max(1),
            intake_milli: derived.intake_milli.max(1),
            thrust_ratio_milli: (derived.thrust_milli * 1_000 / derived.mass_milli.max(1)).max(1),
        }
    }
}

impl DerivedBody {
    /// Realized speed: thrust over mass, scaled and clamped.
    ///
    /// The ratio is what makes a bigger body slower unless it also carries
    /// more motor, which is the structural trade the specification asks for
    /// and is not stated anywhere as a rule.
    pub fn max_speed_milli(&self, floor: i64, ceiling: i64) -> i64 {
        if self.mass_milli <= 0 {
            return floor;
        }
        let speed = self.thrust_milli * 1_000 / self.mass_milli;
        speed.clamp(floor, ceiling)
    }

    /// Controller node budget as a whole count.
    pub fn node_budget(&self) -> u32 {
        (self.node_budget_milli / 1_000).clamp(0, i64::from(u32::MAX)) as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn module(q: i16, r: i16, module_type: ModuleType) -> Module {
        Module {
            position: LatticePos::new(q, r),
            module_type,
            scale_milli: 1_000,
            orientation: 0,
            source_locus: 1,
        }
    }

    fn caps() -> MorphologyCaps {
        MorphologyCaps::provisional()
    }

    #[test]
    fn the_registry_is_indexed_by_type_code() {
        // The registry is a bare array indexed by `id()`, so a type added in
        // the wrong slot would silently price every module of two types
        // wrongly. Pin the correspondence rather than comment it.
        for module_type in ModuleType::ALL {
            assert_eq!(
                ModuleType::from_id(module_type.id()),
                Some(module_type),
                "{} does not round-trip its code",
                module_type.name()
            );
        }
        assert_eq!(REGISTRY.len(), MODULE_TYPE_COUNT);
        assert!(
            ModuleType::Neural.entry().superlinear_upkeep,
            "neural upkeep must be superlinear; it is the only type that is"
        );
        assert!(
            ModuleType::ALL
                .iter()
                .filter(|t| t.entry().superlinear_upkeep)
                .count()
                == 1
        );
    }

    #[test]
    fn an_unknown_module_code_fails_closed() {
        assert_eq!(ModuleType::from_id(MODULE_TYPE_COUNT as u8), None);
        assert_eq!(ModuleType::from_id(255), None);
    }

    #[test]
    fn a_one_module_body_is_legal_and_has_sane_attributes() {
        // C10.2's core: the unicellular case. Phase 16 depends on this and it
        // is verified here rather than discovered there.
        let body = Body::from_modules(
            vec![module(0, 0, ModuleType::Digestive)],
            caps().lattice_radius,
        );
        assert_eq!(body.validate(LatticeKind::Square, &caps()), Ok(()));
        assert_eq!(body.validate(LatticeKind::Hex, &caps()), Ok(()));
        let derived = body.derive();
        assert_eq!(derived.modules, 1);
        assert!(derived.mass_milli > 0, "a unicell must have mass");
        assert!(
            derived.basal_cost_milli > 0,
            "a unicell must cost something to run"
        );
        assert!(
            derived.intake_milli > 0,
            "a digestive unicell must be able to eat"
        );
        // No motor, so no thrust and therefore the speed floor. A sessile
        // organism is legal, not invalid.
        assert_eq!(derived.thrust_milli, 0);
        assert_eq!(derived.max_speed_milli(500, 3_000), 500);
        assert_eq!(derived.node_budget(), 0);
    }

    #[test]
    fn adjacency_is_edge_sharing_and_never_diagonal() {
        // Two modules touching only at a corner are *not* connected: a body
        // held together by corners is not held together.
        let caps = caps();
        let diagonal = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 1, ModuleType::Structural),
            ],
            caps.lattice_radius,
        );
        assert_eq!(
            diagonal.validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::Disconnected)
        );
        let adjacent = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 0, ModuleType::Structural),
            ],
            caps.lattice_radius,
        );
        assert_eq!(adjacent.validate(LatticeKind::Square, &caps), Ok(()));
    }

    #[test]
    fn the_two_lattices_disagree_about_the_same_pair_and_that_is_the_point() {
        // (1,-1) is a neighbour on the hex lattice and not on the square one.
        // If both lattices agreed everywhere, the config choice would be
        // decoration.
        let caps = caps();
        let body = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, -1, ModuleType::Structural),
            ],
            caps.lattice_radius,
        );
        assert_eq!(
            body.validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::Disconnected)
        );
        assert_eq!(body.validate(LatticeKind::Hex, &caps), Ok(()));
    }

    #[test]
    fn every_viability_failure_is_reachable_and_typed() {
        let caps = caps();
        let radius = caps.lattice_radius;
        assert_eq!(
            Body::default().validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::Empty)
        );
        let out = Body::from_modules(vec![module(radius + 1, 0, ModuleType::Digestive)], radius);
        assert_eq!(
            out.validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::OutOfBounds)
        );
        let overlapping = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(0, 0, ModuleType::Structural),
            ],
            radius,
        );
        assert_eq!(
            overlapping.validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::Overlap)
        );
        // The required-type class needs a mask set explicitly, because the
        // default requires nothing: a gutless body is unfit rather than
        // invalid, and letting ecology decide that is the point. The
        // mechanism still has to work when a world does ask for it.
        let mut demanding = caps;
        demanding.required_types_mask = 1 << TYPE_DIGESTIVE;
        let missing = Body::from_modules(vec![module(0, 0, ModuleType::Structural)], radius);
        assert_eq!(
            missing.validate(LatticeKind::Square, &demanding),
            Err(ViabilityFailure::MissingRequiredType(ModuleType::Digestive))
        );
        // ...and the same body is perfectly valid under the default caps.
        assert_eq!(missing.validate(LatticeKind::Square, &caps), Ok(()));
        let mut oversized = module(0, 0, ModuleType::Digestive);
        oversized.scale_milli = MAX_SCALE_MILLI + 1;
        assert_eq!(
            Body::from_modules(vec![oversized], radius).validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::ScaleOutOfRange)
        );
        let mut too_many = Vec::new();
        let span = radius + 1;
        for index in 0..=i16::try_from(caps.max_modules).unwrap() {
            too_many.push(module(index % span, index / span, ModuleType::Digestive));
        }
        assert_eq!(
            Body::from_modules(too_many, radius).validate(LatticeKind::Square, &caps),
            Err(ViabilityFailure::TooManyModules)
        );
    }

    #[test]
    fn a_body_is_identical_however_its_modules_arrive() {
        // C10.1's order-independence, at the representation level: the type
        // sorts into canonical order, so no caller can produce two different
        // bodies from the same module set.
        let radius = caps().lattice_radius;
        let forward = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 0, ModuleType::Motor),
                module(0, 1, ModuleType::Neural),
            ],
            radius,
        );
        let reversed = Body::from_modules(
            vec![
                module(0, 1, ModuleType::Neural),
                module(1, 0, ModuleType::Motor),
                module(0, 0, ModuleType::Digestive),
            ],
            radius,
        );
        assert_eq!(forward, reversed);
        assert_eq!(forward.derive(), reversed.derive());
        let mut left = Fnv1a64::new();
        let mut right = Fnv1a64::new();
        forward.hash_into(&mut left);
        reversed.hash_into(&mut right);
        assert_eq!(left.finish(), right.finish());
    }

    #[test]
    fn mass_speed_and_upkeep_trade_against_each_other() {
        // The trade must be real in both directions or the morphospace has a
        // free lunch in it. More motor buys speed; more storage costs speed.
        let radius = caps().lattice_radius;
        let lean = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 0, ModuleType::Motor),
            ],
            radius,
        );
        let laden = Body::from_modules(
            vec![
                module(0, 0, ModuleType::Digestive),
                module(1, 0, ModuleType::Motor),
                module(2, 0, ModuleType::Storage),
                module(3, 0, ModuleType::Storage),
            ],
            radius,
        );
        let lean_derived = lean.derive();
        let laden_derived = laden.derive();
        assert_eq!(lean_derived.thrust_milli, laden_derived.thrust_milli);
        assert!(laden_derived.mass_milli > lean_derived.mass_milli);
        assert!(
            laden_derived.max_speed_milli(500, 3_000) < lean_derived.max_speed_milli(500, 3_000),
            "carrying storage did not cost speed, so mass is not binding"
        );
        assert!(
            laden_derived.energy_capacity_milli > lean_derived.energy_capacity_milli,
            "storage bought no capacity, so the trade is one-sided"
        );
    }

    #[test]
    fn a_bigger_brain_costs_disproportionately_more_to_run() {
        // C10.7's mechanism, at the derivation level: doubling neural scale
        // must more than octuple its upkeep, or "brain costs body" is a
        // stipulation rather than a structural fact.
        let mut small = module(0, 0, ModuleType::Neural);
        small.scale_milli = 1_000;
        let mut large = module(0, 0, ModuleType::Neural);
        large.scale_milli = 2_000;
        let ratio = large.upkeep_milli() * 1_000 / small.upkeep_milli().max(1);
        assert_eq!(
            ratio, 16_000,
            "neural upkeep must scale as the fourth power"
        );
        // ...while a non-superlinear type scales as the cube, so the
        // difference is a property of neural tissue rather than of scale.
        let mut small_motor = module(0, 0, ModuleType::Motor);
        small_motor.scale_milli = 1_000;
        let mut large_motor = module(0, 0, ModuleType::Motor);
        large_motor.scale_milli = 2_000;
        let motor_ratio = large_motor.upkeep_milli() * 1_000 / small_motor.upkeep_milli().max(1);
        assert_eq!(motor_ratio, 8_000);
    }

    #[test]
    fn lattice_index_is_canonical_and_bounded() {
        let radius = 8_i16;
        assert_eq!(LatticePos::ORIGIN.index(radius), Some(8 * 17 + 8));
        // Out of bounds has no index, which is how the radius is enforced:
        // there is no wrap and no clamp, either of which would silently fold
        // or stack a body.
        assert_eq!(LatticePos::new(radius + 1, 0).index(radius), None);
        assert_eq!(LatticePos::new(0, -radius - 1).index(radius), None);
        // Distinct positions inside the radius have distinct indices.
        let mut seen = std::collections::HashSet::new();
        for q in -radius..=radius {
            for r in -radius..=radius {
                assert!(
                    seen.insert(LatticePos::new(q, r).index(radius).unwrap()),
                    "index collision at ({q}, {r})"
                );
            }
        }
    }
}
