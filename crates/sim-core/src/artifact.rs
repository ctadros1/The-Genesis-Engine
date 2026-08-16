//! Objects as first-class entities (Phase 12 artifact half,
//! `lifesim-artifact-v1`).
//!
//! An object is a body in the shared object-ID space
//! (`specifications/determinism-extensions.md` Rule 2): it takes its ID from
//! the same monotonic counter organisms do, so there is one total order over
//! everything in the world and no cross-space tie-break policy is needed.
//! The design and every departure from the commissioned review are in
//! ADR-0028; the field-level contract is
//! `specifications/artifact-and-material-ontology.md`.
//!
//! # Two structs, one logical
//!
//! [`ObjectTable`] is the logical state: struct-of-arrays sorted by ID, the
//! object ledger, the counters, and the allocation count. It is what is
//! hashed, saved and restored, and it carries no derived field, so
//! `SaveState` reuses it directly the way it reuses `TerrainModState`.
//! [`ObjectState`] wraps a table with the caches the tick needs - the
//! per-cell index of free objects, the per-organism held lists, and the
//! per-tick intent buffers - none of which is saved or hashed and all of
//! which are rebuilt from the table.
//!
//! # Why an `Option<ObjectState>` on `World`
//!
//! The same reason every section since Phase 6 is an option: C12.8 requires
//! four fixtures to reproduce with the section disabled, and an option that
//! is `None` appends nothing to the checksum, costs nothing in the tick, and
//! leaves every pre-existing code path byte for byte.
//!
//! # Fixed point everywhere
//!
//! Integrity, mass and energy integrate over the life of the world (Rule 7).
//! The ledger is `i128` like the world's, so a million-tick run of churn
//! cannot overflow it.

use crate::checksum::Fnv1a64;
use crate::material::{MaterialDef, material};

/// Recorded in the config hash and in the state checksum tag.
pub const ARTIFACT_POLICY_VERSION: &str = "lifesim-artifact-v1";

/// One whole, in integrity Q16.
pub const INTEGRITY_WHOLE_Q16: i32 = 65_536;

// Provenance causes. Permanent values; append, never renumber.
pub const CAUSE_EXTRACTED: u8 = 1;
pub const CAUSE_FRACTURED: u8 = 2;
pub const CAUSE_COMBINED: u8 = 3;
pub const CAUSE_CARCASS: u8 = 4;

pub fn cause_is_known(cause: u8) -> bool {
    (CAUSE_EXTRACTED..=CAUSE_CARCASS).contains(&cause)
}

/// The five actions. IDs are permanent: they are the event payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum ObjectAction {
    PickUp,
    Drop,
    Place,
    Strike,
    Combine,
}

impl ObjectAction {
    pub const ALL: [ObjectAction; 5] = [
        ObjectAction::PickUp,
        ObjectAction::Drop,
        ObjectAction::Place,
        ObjectAction::Strike,
        ObjectAction::Combine,
    ];

    pub fn id(self) -> u8 {
        match self {
            ObjectAction::PickUp => 1,
            ObjectAction::Drop => 2,
            ObjectAction::Place => 3,
            ObjectAction::Strike => 4,
            ObjectAction::Combine => 5,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.id() == id)
    }

    pub fn name(self) -> &'static str {
        match self {
            ObjectAction::PickUp => "pick_up",
            ObjectAction::Drop => "drop",
            ObjectAction::Place => "place",
            ObjectAction::Strike => "strike",
            ObjectAction::Combine => "combine",
        }
    }
}

/// Every reason an action can be refused. One variant per reason, never a
/// shared boolean: C12.7 requires caps to reject deterministically, count,
/// and event, and a single "refused" flag is how a cap becomes invisible in
/// a report (D-074). IDs are permanent: they are the event payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub enum RefuseReason {
    /// Nothing admissible within reach.
    NoTarget,
    /// Something is in reach but nothing fits the remaining carry capacity.
    CapacityExceeded,
    /// `max_held_objects` reached.
    HeldCap,
    /// Lost a claim on the target to a higher (priority, distance, id).
    Contested,
    /// `drop`, `place`, `combine` with nothing held.
    NothingHeld,
    /// The destination cell is at `max_objects_per_cell`.
    OccupancyCap,
    /// `place` off the map or into a cell organisms cannot enter.
    InvalidCell,
    /// Terrain strike on a cell whose yield is exhausted.
    Depleted,
    /// Terrain strike on a cell that yields nothing (water).
    NoYield,
    /// The world is at `max_objects`.
    ObjectCap,
    /// The composite would exceed `max_composition_depth`.
    DepthCap,
    /// The composite would exceed `max_composition_breadth`.
    BreadthCap,
    /// The joint draw fell below `joint_floor_q16`.
    JointFailed,
}

impl RefuseReason {
    pub const ALL: [RefuseReason; 13] = [
        RefuseReason::NoTarget,
        RefuseReason::CapacityExceeded,
        RefuseReason::HeldCap,
        RefuseReason::Contested,
        RefuseReason::NothingHeld,
        RefuseReason::OccupancyCap,
        RefuseReason::InvalidCell,
        RefuseReason::Depleted,
        RefuseReason::NoYield,
        RefuseReason::ObjectCap,
        RefuseReason::DepthCap,
        RefuseReason::BreadthCap,
        RefuseReason::JointFailed,
    ];

    pub fn id(self) -> u8 {
        match self {
            RefuseReason::NoTarget => 1,
            RefuseReason::CapacityExceeded => 2,
            RefuseReason::HeldCap => 3,
            RefuseReason::Contested => 4,
            RefuseReason::NothingHeld => 5,
            RefuseReason::OccupancyCap => 6,
            RefuseReason::InvalidCell => 7,
            RefuseReason::Depleted => 8,
            RefuseReason::NoYield => 9,
            RefuseReason::ObjectCap => 10,
            RefuseReason::DepthCap => 11,
            RefuseReason::BreadthCap => 12,
            RefuseReason::JointFailed => 13,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|reason| reason.id() == id)
    }

    /// Whether the reason is a configured cap binding, as opposed to a
    /// physical or ecological refusal. C12.7 wants the cap subset filterable.
    pub fn is_cap(self) -> bool {
        matches!(
            self,
            RefuseReason::HeldCap
                | RefuseReason::OccupancyCap
                | RefuseReason::ObjectCap
                | RefuseReason::DepthCap
                | RefuseReason::BreadthCap
        )
    }
}

/// Why an object left the table. Permanent values.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DestroyCause {
    /// Integrity or energy reached zero through passive decay.
    Decayed,
    /// Struck harder than its hardness (a simple object) or worn to zero.
    Fractured,
    /// A composite that came apart, restoring its constituents.
    Disassembled,
    /// Energy fully consumed.
    Consumed,
    /// Refused by the world cap at creation, or a sub-minimum fragment.
    Dust,
    /// Condition B: an ephemeral world destroys placed objects at tick end.
    Ephemeral,
}

impl DestroyCause {
    pub fn id(self) -> u8 {
        match self {
            DestroyCause::Decayed => 1,
            DestroyCause::Fractured => 2,
            DestroyCause::Disassembled => 3,
            DestroyCause::Consumed => 4,
            DestroyCause::Dust => 5,
            DestroyCause::Ephemeral => 6,
        }
    }

    pub fn from_id(id: u8) -> Option<Self> {
        [
            DestroyCause::Decayed,
            DestroyCause::Fractured,
            DestroyCause::Disassembled,
            DestroyCause::Consumed,
            DestroyCause::Dust,
            DestroyCause::Ephemeral,
        ]
        .into_iter()
        .find(|cause| cause.id() == id)
    }
}

/// The object ledger: every path mass or energy takes into or out of the
/// object pool. Exact to the milli, `i128` like the world's ledger.
///
/// Terrain yield is outside it, as biomass regrowth is outside the energy
/// ledger: extraction is the source term. Consumption is a transfer to the
/// consumer's `assimilated_milli`; decay, and dust (a refused creation, a
/// sub-minimum fragment, a simple object worn or decayed to zero) are sinks.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObjectLedger {
    pub mass_extracted_milli: i128,
    pub mass_carcass_milli: i128,
    pub mass_decayed_milli: i128,
    pub mass_consumed_milli: i128,
    pub mass_dust_milli: i128,
    pub energy_extracted_milli: i128,
    pub energy_carcass_milli: i128,
    pub energy_decayed_milli: i128,
    pub energy_consumed_milli: i128,
    pub energy_dust_milli: i128,
}

impl ObjectLedger {
    /// Mass the table must hold for the identity to be exact.
    pub fn expected_mass_milli(&self) -> i128 {
        self.mass_extracted_milli + self.mass_carcass_milli
            - self.mass_decayed_milli
            - self.mass_consumed_milli
            - self.mass_dust_milli
    }

    /// Energy the table must hold for the identity to be exact.
    pub fn expected_energy_milli(&self) -> i128 {
        self.energy_extracted_milli + self.energy_carcass_milli
            - self.energy_decayed_milli
            - self.energy_consumed_milli
            - self.energy_dust_milli
    }

    /// Destructured with no `..` (D-077): a term added here fails to compile
    /// until it is hashed. Byte order permanent; append, never reorder.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        } = *self;
        for value in [
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        ] {
            hasher.update_i128(value);
        }
    }

    pub const FIELD_COUNT: usize = 10;

    /// The terms' names, in the permanent `to_array` order: the manifest's
    /// column suffixes and the report's labels.
    pub const FIELD_NAMES: [&'static str; Self::FIELD_COUNT] = [
        "mass_extracted_milli",
        "mass_carcass_milli",
        "mass_decayed_milli",
        "mass_consumed_milli",
        "mass_dust_milli",
        "energy_extracted_milli",
        "energy_carcass_milli",
        "energy_decayed_milli",
        "energy_consumed_milli",
        "energy_dust_milli",
    ];

    /// The terms in hash order, for the codec.
    pub fn to_array(&self) -> [i128; Self::FIELD_COUNT] {
        let Self {
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        } = *self;
        [
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        ]
    }

    pub fn from_array(values: [i128; Self::FIELD_COUNT]) -> Self {
        let [
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        ] = values;
        Self {
            mass_extracted_milli,
            mass_carcass_milli,
            mass_decayed_milli,
            mass_consumed_milli,
            mass_dust_milli,
            energy_extracted_milli,
            energy_carcass_milli,
            energy_decayed_milli,
            energy_consumed_milli,
            energy_dust_milli,
        }
    }
}

/// Counters for the artifact half. On this struct rather than on
/// `world::Counters` because `Counters` is hashed field by field into every
/// world's checksum, so a field added there would move four fixtures; here
/// they are hashed under the section tag and only when the section exists.
///
/// The disposition half counts what happened; the refusal half counts what
/// was refused and why (D-074: never merge the two). Every refusal reason
/// has its own counter so "a run silently pressed against a cap" is visible.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObjectCounters {
    // Disposition.
    pub created_extracted: u64,
    pub created_fractured: u64,
    pub created_combined: u64,
    pub created_carcass: u64,
    pub picked_up: u64,
    pub dropped: u64,
    pub placed: u64,
    pub struck_objects: u64,
    pub struck_terrain: u64,
    pub fractured: u64,
    pub disassembled: u64,
    pub combined: u64,
    pub consumed_events: u64,
    pub decayed_away: u64,
    pub worn_away: u64,
    pub death_drops: u64,
    pub ephemeral_destroyed: u64,
    // Refusals, one per `RefuseReason` in `RefuseReason::ALL` order.
    pub refused_no_target: u64,
    pub refused_capacity: u64,
    pub refused_held_cap: u64,
    pub refused_contested: u64,
    pub refused_nothing_held: u64,
    pub refused_occupancy_cap: u64,
    pub refused_invalid_cell: u64,
    pub refused_depleted: u64,
    pub refused_no_yield: u64,
    pub refused_object_cap: u64,
    pub refused_depth_cap: u64,
    pub refused_breadth_cap: u64,
    pub refused_joint_failed: u64,
}

impl ObjectCounters {
    pub const DISPOSITION_COUNT: usize = 17;
    pub const REFUSAL_COUNT: usize = 13;
    pub const FIELD_COUNT: usize = Self::DISPOSITION_COUNT + Self::REFUSAL_COUNT;

    /// The counters' names in the permanent `to_array` order: the manifest's
    /// column suffixes and the report's labels. Checked against the struct
    /// by a test that round-trips every field with a distinct value.
    pub const FIELD_NAMES: [&'static str; Self::FIELD_COUNT] = [
        "created_extracted",
        "created_fractured",
        "created_combined",
        "created_carcass",
        "picked_up",
        "dropped",
        "placed",
        "struck_objects",
        "struck_terrain",
        "fractured",
        "disassembled",
        "combined",
        "consumed_events",
        "decayed_away",
        "worn_away",
        "death_drops",
        "ephemeral_destroyed",
        "refused_no_target",
        "refused_capacity",
        "refused_held_cap",
        "refused_contested",
        "refused_nothing_held",
        "refused_occupancy_cap",
        "refused_invalid_cell",
        "refused_depleted",
        "refused_no_yield",
        "refused_object_cap",
        "refused_depth_cap",
        "refused_breadth_cap",
        "refused_joint_failed",
    ];

    /// Destructured with no `..` (D-077). Concatenation order is declaration
    /// order and is permanent: it is the byte order `hash_into` feeds the
    /// hasher and the codec writes. Append, never reorder.
    fn partitioned(
        &self,
    ) -> (
        [u64; Self::DISPOSITION_COUNT],
        [u64; Self::REFUSAL_COUNT],
    ) {
        let Self {
            created_extracted,
            created_fractured,
            created_combined,
            created_carcass,
            picked_up,
            dropped,
            placed,
            struck_objects,
            struck_terrain,
            fractured,
            disassembled,
            combined,
            consumed_events,
            decayed_away,
            worn_away,
            death_drops,
            ephemeral_destroyed,
            refused_no_target,
            refused_capacity,
            refused_held_cap,
            refused_contested,
            refused_nothing_held,
            refused_occupancy_cap,
            refused_invalid_cell,
            refused_depleted,
            refused_no_yield,
            refused_object_cap,
            refused_depth_cap,
            refused_breadth_cap,
            refused_joint_failed,
        } = *self;
        (
            [
                created_extracted,
                created_fractured,
                created_combined,
                created_carcass,
                picked_up,
                dropped,
                placed,
                struck_objects,
                struck_terrain,
                fractured,
                disassembled,
                combined,
                consumed_events,
                decayed_away,
                worn_away,
                death_drops,
                ephemeral_destroyed,
            ],
            [
                refused_no_target,
                refused_capacity,
                refused_held_cap,
                refused_contested,
                refused_nothing_held,
                refused_occupancy_cap,
                refused_invalid_cell,
                refused_depleted,
                refused_no_yield,
                refused_object_cap,
                refused_depth_cap,
                refused_breadth_cap,
                refused_joint_failed,
            ],
        )
    }

    pub fn to_array(&self) -> [u64; Self::FIELD_COUNT] {
        let (disposition, refusals) = self.partitioned();
        let mut out = [0_u64; Self::FIELD_COUNT];
        out[..Self::DISPOSITION_COUNT].copy_from_slice(&disposition);
        out[Self::DISPOSITION_COUNT..].copy_from_slice(&refusals);
        out
    }

    pub fn from_array(values: [u64; Self::FIELD_COUNT]) -> Self {
        let mut counters = Self::default();
        let slots = counters.slots_mut();
        for (slot, value) in slots.into_iter().zip(values) {
            *slot = value;
        }
        counters
    }

    /// Mutable references in the permanent order. The single place that
    /// enumerates fields for writing, so `from_array` cannot drift from
    /// `partitioned` without a compile error on the count.
    fn slots_mut(&mut self) -> [&mut u64; Self::FIELD_COUNT] {
        let Self {
            created_extracted,
            created_fractured,
            created_combined,
            created_carcass,
            picked_up,
            dropped,
            placed,
            struck_objects,
            struck_terrain,
            fractured,
            disassembled,
            combined,
            consumed_events,
            decayed_away,
            worn_away,
            death_drops,
            ephemeral_destroyed,
            refused_no_target,
            refused_capacity,
            refused_held_cap,
            refused_contested,
            refused_nothing_held,
            refused_occupancy_cap,
            refused_invalid_cell,
            refused_depleted,
            refused_no_yield,
            refused_object_cap,
            refused_depth_cap,
            refused_breadth_cap,
            refused_joint_failed,
        } = self;
        [
            created_extracted,
            created_fractured,
            created_combined,
            created_carcass,
            picked_up,
            dropped,
            placed,
            struck_objects,
            struck_terrain,
            fractured,
            disassembled,
            combined,
            consumed_events,
            decayed_away,
            worn_away,
            death_drops,
            ephemeral_destroyed,
            refused_no_target,
            refused_capacity,
            refused_held_cap,
            refused_contested,
            refused_nothing_held,
            refused_occupancy_cap,
            refused_invalid_cell,
            refused_depleted,
            refused_no_yield,
            refused_object_cap,
            refused_depth_cap,
            refused_breadth_cap,
            refused_joint_failed,
        ]
    }

    /// Count one refusal under its own reason.
    pub fn refuse(&mut self, reason: RefuseReason) {
        let slot = match reason {
            RefuseReason::NoTarget => &mut self.refused_no_target,
            RefuseReason::CapacityExceeded => &mut self.refused_capacity,
            RefuseReason::HeldCap => &mut self.refused_held_cap,
            RefuseReason::Contested => &mut self.refused_contested,
            RefuseReason::NothingHeld => &mut self.refused_nothing_held,
            RefuseReason::OccupancyCap => &mut self.refused_occupancy_cap,
            RefuseReason::InvalidCell => &mut self.refused_invalid_cell,
            RefuseReason::Depleted => &mut self.refused_depleted,
            RefuseReason::NoYield => &mut self.refused_no_yield,
            RefuseReason::ObjectCap => &mut self.refused_object_cap,
            RefuseReason::DepthCap => &mut self.refused_depth_cap,
            RefuseReason::BreadthCap => &mut self.refused_breadth_cap,
            RefuseReason::JointFailed => &mut self.refused_joint_failed,
        };
        *slot += 1;
    }

    /// Refusals for any reason.
    pub fn refusals(&self) -> u64 {
        let (_, refusals) = self.partitioned();
        refusals.iter().sum()
    }

    /// Refusals that were a configured cap binding.
    pub fn cap_refusals(&self) -> u64 {
        self.refused_held_cap
            + self.refused_occupancy_cap
            + self.refused_object_cap
            + self.refused_depth_cap
            + self.refused_breadth_cap
    }

    /// Successful actions of every kind: the numerator C12.1 reads.
    pub fn successes(&self) -> u64 {
        self.picked_up + self.dropped + self.placed + self.struck_objects + self.struck_terrain + self.combined
    }

    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        for value in self.to_array() {
            hasher.update_u64(value);
        }
    }
}

/// One object, as a value: the shape `ObjectTable::push` takes and the codec
/// round-trips. Fields documented in the specification's object table.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectRecord {
    pub id: u64,
    pub material_id: u16,
    pub x_fp: i32,
    pub y_fp: i32,
    pub integrity_q16: i32,
    pub mass_milli: i64,
    pub energy_milli: i64,
    pub hardness_q16: u32,
    pub durability_q16: u32,
    pub decay_q16: u32,
    pub holder_id: u64,
    pub owner_id: u64,
    pub depth: u8,
    pub created_tick: u64,
    pub creator_id: u64,
    pub cause: u8,
    pub parent_id: u64,
    pub composition: Vec<u64>,
}

impl ObjectRecord {
    /// A whole simple object of one material with the given extracted volume.
    pub fn simple(
        id: u64,
        def: &MaterialDef,
        volume_milli: i64,
        x_fp: i32,
        y_fp: i32,
        created_tick: u64,
        cause: u8,
        parent_id: u64,
    ) -> Self {
        let mass_milli = volume_milli * def.density_milli / 1_000;
        let energy_milli = mass_milli * def.energy_content_milli / 1_000;
        Self {
            id,
            material_id: def.id,
            x_fp,
            y_fp,
            integrity_q16: INTEGRITY_WHOLE_Q16,
            mass_milli,
            energy_milli,
            hardness_q16: def.hardness_q16,
            durability_q16: def.durability_q16,
            decay_q16: def.decay_per_tick_q16,
            holder_id: 0,
            owner_id: 0,
            depth: 0,
            created_tick,
            creator_id: 0,
            cause,
            parent_id,
            composition: Vec::new(),
        }
    }
}

/// The logical object table. Everything here is hashed and saved.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectTable {
    // Struct-of-arrays, strictly ascending by `ids`.
    pub ids: Vec<u64>,
    pub material_id: Vec<u16>,
    pub x_fp: Vec<i32>,
    pub y_fp: Vec<i32>,
    pub integrity_q16: Vec<i32>,
    pub mass_milli: Vec<i64>,
    pub energy_milli: Vec<i64>,
    pub hardness_q16: Vec<u32>,
    pub durability_q16: Vec<u32>,
    pub decay_q16: Vec<u32>,
    pub holder_id: Vec<u64>,
    pub owner_id: Vec<u64>,
    pub depth: Vec<u8>,
    pub created_tick: Vec<u64>,
    pub creator_id: Vec<u64>,
    pub cause: Vec<u8>,
    pub parent_id: Vec<u64>,
    pub composition: Vec<Vec<u64>>,
    /// Every object ID ever drawn from the shared counter by this section.
    /// The term the allocation identity in `check_invariants` needs:
    /// `initial + births + objects_allocated + 1 == next_entity_id`.
    pub objects_allocated_total: u64,
    pub ledger: ObjectLedger,
    pub counters: ObjectCounters,
    /// Per-organism observation, parallel to the world's organism arrays:
    /// ticks the organism has spent standing in a cell that held a live
    /// **placed** free object (`creator_id != 0`). C12.2's exposure
    /// measure. Written every tick and read by nothing in the tick (ADR-0016);
    /// saved and hashed for the reason the action census is, so a restored
    /// world agrees with the one it was saved from about every organism's
    /// history; emitted in `ObjectExposure` at death so the analysis can
    /// pair it with the organism's reproductive output.
    pub exposure_ticks: Vec<u64>,
    /// Ticks the organism has held at least one object. Same terms.
    pub carry_ticks: Vec<u64>,
    /// The baseline-capacity band (0..=4, quintile of the terrain's habitable
    /// cells) of the cell the organism was born in. The stratifier C12.2's
    /// matched comparison needs: placed objects sit where organisms are, and
    /// where organisms are is where the food is. Fixed at birth; founders
    /// take their spawn cell's band.
    pub birth_band: Vec<u8>,
}

/// Structural defects a decoded or live table can carry. Each is its own
/// variant so a restore is told which one it is, not sent to a checksum.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableViolation {
    /// Parallel arrays of unequal length.
    Ragged,
    /// IDs not strictly ascending at this index.
    Order(usize),
    /// Unknown material or cause, or an out-of-domain integrity or depth.
    Domain(usize),
    /// Held and owned at once, or held/owned by itself, or a mass or energy
    /// below zero.
    Exclusivity(usize),
    /// A composition list is not ascending, names an absent object, names an
    /// object not owned by this composite, or a constituent's derived
    /// properties disagree with the composite's stored ones.
    Composition(usize),
    /// An owned object whose owner is absent, not a composite, or does not
    /// list it.
    Owner(usize),
    /// The mass identity does not hold.
    MassLedger { expected: i128, actual: i128 },
    /// The energy identity does not hold.
    EnergyLedger { expected: i128, actual: i128 },
}

impl ObjectTable {
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Binary search by ID.
    pub fn index_of(&self, id: u64) -> Option<usize> {
        self.ids.binary_search(&id).ok()
    }

    /// Whether the object at `index` is in the world (not held, not owned).
    pub fn is_free(&self, index: usize) -> bool {
        self.holder_id[index] == 0 && self.owner_id[index] == 0
    }

    /// Append an object. IDs come from the shared monotonic counter, so a
    /// new object is always the largest ID and appends; the debug assertion
    /// is the invariant, not a hope.
    pub fn push(&mut self, record: ObjectRecord) -> usize {
        debug_assert!(
            self.ids.last().is_none_or(|last| *last < record.id),
            "object ids must be pushed in ascending order"
        );
        let ObjectRecord {
            id,
            material_id,
            x_fp,
            y_fp,
            integrity_q16,
            mass_milli,
            energy_milli,
            hardness_q16,
            durability_q16,
            decay_q16,
            holder_id,
            owner_id,
            depth,
            created_tick,
            creator_id,
            cause,
            parent_id,
            composition,
        } = record;
        self.ids.push(id);
        self.material_id.push(material_id);
        self.x_fp.push(x_fp);
        self.y_fp.push(y_fp);
        self.integrity_q16.push(integrity_q16);
        self.mass_milli.push(mass_milli);
        self.energy_milli.push(energy_milli);
        self.hardness_q16.push(hardness_q16);
        self.durability_q16.push(durability_q16);
        self.decay_q16.push(decay_q16);
        self.holder_id.push(holder_id);
        self.owner_id.push(owner_id);
        self.depth.push(depth);
        self.created_tick.push(created_tick);
        self.creator_id.push(creator_id);
        self.cause.push(cause);
        self.parent_id.push(parent_id);
        self.composition.push(composition);
        self.ids.len() - 1
    }

    /// The object at `index` as a value.
    pub fn record(&self, index: usize) -> ObjectRecord {
        ObjectRecord {
            id: self.ids[index],
            material_id: self.material_id[index],
            x_fp: self.x_fp[index],
            y_fp: self.y_fp[index],
            integrity_q16: self.integrity_q16[index],
            mass_milli: self.mass_milli[index],
            energy_milli: self.energy_milli[index],
            hardness_q16: self.hardness_q16[index],
            durability_q16: self.durability_q16[index],
            decay_q16: self.decay_q16[index],
            holder_id: self.holder_id[index],
            owner_id: self.owner_id[index],
            depth: self.depth[index],
            created_tick: self.created_tick[index],
            creator_id: self.creator_id[index],
            cause: self.cause[index],
            parent_id: self.parent_id[index],
            composition: self.composition[index].clone(),
        }
    }

    /// Compact with removal flags, preserving order.
    pub fn retain(&mut self, remove: &[bool]) {
        debug_assert_eq!(remove.len(), self.ids.len());
        crate::world::retain_by_flags(&mut self.ids, remove);
        crate::world::retain_by_flags(&mut self.material_id, remove);
        crate::world::retain_by_flags(&mut self.x_fp, remove);
        crate::world::retain_by_flags(&mut self.y_fp, remove);
        crate::world::retain_by_flags(&mut self.integrity_q16, remove);
        crate::world::retain_by_flags(&mut self.mass_milli, remove);
        crate::world::retain_by_flags(&mut self.energy_milli, remove);
        crate::world::retain_by_flags(&mut self.hardness_q16, remove);
        crate::world::retain_by_flags(&mut self.durability_q16, remove);
        crate::world::retain_by_flags(&mut self.decay_q16, remove);
        crate::world::retain_by_flags(&mut self.holder_id, remove);
        crate::world::retain_by_flags(&mut self.owner_id, remove);
        crate::world::retain_by_flags(&mut self.depth, remove);
        crate::world::retain_by_flags(&mut self.created_tick, remove);
        crate::world::retain_by_flags(&mut self.creator_id, remove);
        crate::world::retain_by_flags(&mut self.cause, remove);
        crate::world::retain_by_flags(&mut self.parent_id, remove);
        let mut index = 0;
        self.composition.retain(|_| {
            let keep = !remove[index];
            index += 1;
            keep
        });
    }

    /// Mass in the pool, counting each unit once: a composite stores the sum
    /// of its constituents and its constituents are owned, so the sum runs
    /// over **unowned** objects only (free or held). An owned constituent's
    /// mass is inside its composite's.
    pub fn total_mass_milli(&self) -> i128 {
        (0..self.len())
            .filter(|&index| self.owner_id[index] == 0)
            .map(|index| i128::from(self.mass_milli[index]))
            .sum()
    }

    /// Energy in the pool, on the same terms as [`Self::total_mass_milli`].
    pub fn total_energy_milli(&self) -> i128 {
        (0..self.len())
            .filter(|&index| self.owner_id[index] == 0)
            .map(|index| i128::from(self.energy_milli[index]))
            .sum()
    }

    /// Objects that are free (in the world, not held, not owned).
    pub fn free_count(&self) -> usize {
        (0..self.len()).filter(|&index| self.is_free(index)).count()
    }

    /// Composites of at least the given depth. C12.3's numerator.
    pub fn count_with_depth_at_least(&self, depth: u8) -> usize {
        self.depth.iter().filter(|&&d| d >= depth).count()
    }

    /// The first structural defect, or `None`.
    ///
    /// Checked rather than assumed: the tick's own writes cannot produce
    /// most of these, so the path this defends is the one that does not go
    /// through them - a restore decoding an untrusted payload. It also
    /// re-derives every composite's stored properties from its constituents,
    /// so a stored value that disagrees with its derivation is a defect and
    /// not a fact.
    pub fn violation(&self, max_depth: u8) -> Option<TableViolation> {
        let n = self.ids.len();
        if [
            self.material_id.len(),
            self.x_fp.len(),
            self.y_fp.len(),
            self.integrity_q16.len(),
            self.mass_milli.len(),
            self.energy_milli.len(),
            self.hardness_q16.len(),
            self.durability_q16.len(),
            self.decay_q16.len(),
            self.holder_id.len(),
            self.owner_id.len(),
            self.depth.len(),
            self.created_tick.len(),
            self.creator_id.len(),
            self.cause.len(),
            self.parent_id.len(),
            self.composition.len(),
        ]
        .iter()
        .any(|&len| len != n)
        {
            return Some(TableViolation::Ragged);
        }
        if self.carry_ticks.len() != self.exposure_ticks.len()
            || self.birth_band.len() != self.exposure_ticks.len()
            || self.birth_band.iter().any(|&band| band > 4)
        {
            return Some(TableViolation::Ragged);
        }
        for index in 1..n {
            if self.ids[index - 1] >= self.ids[index] {
                return Some(TableViolation::Order(index));
            }
        }
        for index in 0..n {
            let id = self.ids[index];
            if id == 0
                || !crate::material::material_exists(self.material_id[index])
                || !cause_is_known(self.cause[index])
                || self.integrity_q16[index] < 0
                || self.integrity_q16[index] > INTEGRITY_WHOLE_Q16
                || self.depth[index] > max_depth
            {
                return Some(TableViolation::Domain(index));
            }
            if (self.holder_id[index] != 0 && self.owner_id[index] != 0)
                || self.holder_id[index] == id
                || self.owner_id[index] == id
                || self.mass_milli[index] < 0
                || self.energy_milli[index] < 0
            {
                return Some(TableViolation::Exclusivity(index));
            }
            let composition = &self.composition[index];
            if composition.is_empty() {
                if self.depth[index] != 0 {
                    return Some(TableViolation::Composition(index));
                }
                // A simple object's stored properties are its material's.
                let def = material(self.material_id[index]).expect("checked above");
                if self.hardness_q16[index] != def.hardness_q16
                    || self.durability_q16[index] != def.durability_q16
                    || self.decay_q16[index] != def.decay_per_tick_q16
                {
                    return Some(TableViolation::Composition(index));
                }
            } else {
                if composition.len() < 2 {
                    return Some(TableViolation::Composition(index));
                }
                let (mut mass, mut energy) = (0_i128, 0_i128);
                let (mut hardness, mut durability, mut decay, mut depth) = (0_u32, u32::MAX, 0_u32, 0_u8);
                for (position, &constituent) in composition.iter().enumerate() {
                    if position > 0 && composition[position - 1] >= constituent {
                        return Some(TableViolation::Composition(index));
                    }
                    let Some(other) = self.index_of(constituent) else {
                        return Some(TableViolation::Composition(index));
                    };
                    if self.owner_id[other] != id || self.holder_id[other] != 0 {
                        return Some(TableViolation::Composition(index));
                    }
                    mass += i128::from(self.mass_milli[other]);
                    energy += i128::from(self.energy_milli[other]);
                    hardness = hardness.max(self.hardness_q16[other]);
                    durability = durability.min(self.durability_q16[other]);
                    decay = decay.max(self.decay_q16[other]);
                    depth = depth.max(self.depth[other]);
                }
                if mass != i128::from(self.mass_milli[index])
                    || energy != i128::from(self.energy_milli[index])
                    || hardness != self.hardness_q16[index]
                    || durability != self.durability_q16[index]
                    || decay != self.decay_q16[index]
                    || depth.saturating_add(1) != self.depth[index]
                {
                    return Some(TableViolation::Composition(index));
                }
            }
            if self.owner_id[index] != 0 {
                let Some(owner) = self.index_of(self.owner_id[index]) else {
                    return Some(TableViolation::Owner(index));
                };
                if self.composition[owner].binary_search(&id).is_err() {
                    return Some(TableViolation::Owner(index));
                }
            }
        }
        let expected_mass = self.ledger.expected_mass_milli();
        let actual_mass = self.total_mass_milli();
        if expected_mass != actual_mass {
            return Some(TableViolation::MassLedger {
                expected: expected_mass,
                actual: actual_mass,
            });
        }
        let expected_energy = self.ledger.expected_energy_milli();
        let actual_energy = self.total_energy_milli();
        if expected_energy != actual_energy {
            return Some(TableViolation::EnergyLedger {
                expected: expected_energy,
                actual: actual_energy,
            });
        }
        None
    }

    /// Hash every field under the section tag.
    ///
    /// **Destructured with no `..` (D-077).** A field added to this struct
    /// fails to compile here until it is either hashed or given an explicit
    /// reason not to be. The byte order below is permanent: it is the
    /// definition of an artifact world's checksum. Append, never reorder.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            ids,
            material_id,
            x_fp,
            y_fp,
            integrity_q16,
            mass_milli,
            energy_milli,
            hardness_q16,
            durability_q16,
            decay_q16,
            holder_id,
            owner_id,
            depth,
            created_tick,
            creator_id,
            cause,
            parent_id,
            composition,
            objects_allocated_total,
            ledger,
            counters,
            exposure_ticks,
            carry_ticks,
            birth_band,
        } = self;
        hasher.update(b"lifesim-object-state-v1");
        hasher.update(ARTIFACT_POLICY_VERSION.as_bytes());
        // The count is hashed so a truncated table is a difference and not
        // a prefix that could re-align with a shorter one.
        hasher.update_u64(ids.len() as u64);
        for index in 0..ids.len() {
            hasher.update_u64(ids[index]);
            hasher.update_u32(u32::from(material_id[index]));
            hasher.update_i32(x_fp[index]);
            hasher.update_i32(y_fp[index]);
            hasher.update_i32(integrity_q16[index]);
            hasher.update_i64(mass_milli[index]);
            hasher.update_i64(energy_milli[index]);
            hasher.update_u32(hardness_q16[index]);
            hasher.update_u32(durability_q16[index]);
            hasher.update_u32(decay_q16[index]);
            hasher.update_u64(holder_id[index]);
            hasher.update_u64(owner_id[index]);
            hasher.update_u32(u32::from(depth[index]));
            hasher.update_u64(created_tick[index]);
            hasher.update_u64(creator_id[index]);
            hasher.update_u32(u32::from(cause[index]));
            hasher.update_u64(parent_id[index]);
            hasher.update_u64(composition[index].len() as u64);
            for &constituent in &composition[index] {
                hasher.update_u64(constituent);
            }
        }
        hasher.update_u64(*objects_allocated_total);
        ledger.hash_into(hasher);
        counters.hash_into(hasher);
        // The per-organism observations, after everything above so the
        // object-table bytes are a prefix of the whole. Length-framed.
        hasher.update_u64(exposure_ticks.len() as u64);
        for index in 0..exposure_ticks.len() {
            hasher.update_u64(exposure_ticks[index]);
            hasher.update_u64(carry_ticks[index]);
            hasher.update_u32(u32::from(birth_band[index]));
        }
    }
}

/// One organism's requests for this tick. `None` when the channel is unbound
/// or below threshold; `Some(priority_milli)` otherwise. Rebuilt every tick,
/// never logical state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ObjectIntent {
    pub pick_up: Option<i32>,
    pub drop: Option<i32>,
    pub place: Option<i32>,
    pub strike: Option<i32>,
    pub combine: Option<i32>,
}

/// The table plus the caches the tick needs. Only `table` is hashed and
/// saved; everything else is rebuilt from it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObjectState {
    pub table: ObjectTable,
    /// Free objects per cell, ascending by ID. Built once per tick in the
    /// `SpatialIndex` phase; a bucket built in scan order is never read for
    /// a decision (Rule 5), which the sort at build time guarantees.
    pub cell_index: Vec<Vec<u32>>,
    /// Held object IDs per organism, ascending, parallel to the world's
    /// organism arrays. Derived from `holder_id`; cross-checked by
    /// `check_invariants`.
    pub held: Vec<Vec<u64>>,
    /// Per-tick intents, parallel to organisms.
    pub intents: Vec<ObjectIntent>,
    /// Per-tick perception cues, parallel to organisms: present, distance,
    /// bearing, heft, hardness, carried load. Written in `Sense`, read by
    /// the controller gather, never saved or hashed.
    pub perception: Vec<[f32; 6]>,
    /// The four baseline-capacity quintile boundaries over the terrain's
    /// habitable cells, for `birth_band`. A pure function of the terrain,
    /// computed once at construction and again at restore; never saved.
    pub band_thresholds: [i64; 4],
}

impl ObjectState {
    pub fn with_capacity(organisms: usize) -> Self {
        Self {
            table: ObjectTable::default(),
            cell_index: Vec::new(),
            held: Vec::with_capacity(organisms),
            intents: Vec::with_capacity(organisms),
            perception: Vec::with_capacity(organisms),
            band_thresholds: [i64::MAX; 4],
        }
    }

    pub fn from_table(table: ObjectTable) -> Self {
        Self {
            table,
            ..Default::default()
        }
    }

    /// Quintile boundaries of the given habitable-cell capacities. Sorted
    /// once; the boundaries are the values at the 20/40/60/80 percent ranks,
    /// so a cell's band is the number of boundaries at or below its capacity.
    pub fn band_thresholds_of(mut capacities: Vec<i64>) -> [i64; 4] {
        if capacities.is_empty() {
            return [i64::MAX; 4];
        }
        capacities.sort_unstable();
        let n = capacities.len();
        let at = |fraction: usize| capacities[(n * fraction / 5).min(n - 1)];
        [at(1), at(2), at(3), at(4)]
    }

    /// The band (0..=4) a cell of the given baseline capacity falls in.
    pub fn band_of(&self, capacity_milli: i64) -> u8 {
        self.band_thresholds
            .iter()
            .filter(|&&threshold| capacity_milli >= threshold)
            .count() as u8
    }

    /// One more organism (a birth): nothing held, no intent, no history, born
    /// into `birth_band`.
    pub fn push_organism(&mut self, birth_band: u8) {
        self.held.push(Vec::new());
        self.intents.push(ObjectIntent::default());
        self.perception.push([0.0; 6]);
        self.table.exposure_ticks.push(0);
        self.table.carry_ticks.push(0);
        self.table.birth_band.push(birth_band.min(4));
    }

    /// Compact the per-organism arrays with the world's removal flags.
    pub fn retain_organisms(&mut self, remove: &[bool]) {
        let mut index = 0;
        self.held.retain(|_| {
            let keep = !remove[index];
            index += 1;
            keep
        });
        crate::world::retain_by_flags(&mut self.intents, remove);
        crate::world::retain_by_flags(&mut self.perception, remove);
        crate::world::retain_by_flags(&mut self.table.exposure_ticks, remove);
        crate::world::retain_by_flags(&mut self.table.carry_ticks, remove);
        crate::world::retain_by_flags(&mut self.table.birth_band, remove);
    }

    /// Rebuild `held` from `holder_id` for the given ascending organism IDs.
    /// Used on restore and by the invariant check.
    pub fn rebuild_held(&mut self, organism_ids: &[u64]) {
        self.held = vec![Vec::new(); organism_ids.len()];
        for index in 0..self.table.len() {
            let holder = self.table.holder_id[index];
            if holder == 0 {
                continue;
            }
            if let Ok(organism) = organism_ids.binary_search(&holder) {
                self.held[organism].push(self.table.ids[index]);
            }
        }
        for list in &mut self.held {
            list.sort_unstable();
        }
    }

    /// Whether `held` agrees with `holder_id`, and every holder is alive.
    pub fn held_is_consistent(&self, organism_ids: &[u64]) -> bool {
        let mut expected = vec![Vec::new(); organism_ids.len()];
        for index in 0..self.table.len() {
            let holder = self.table.holder_id[index];
            if holder == 0 {
                continue;
            }
            match organism_ids.binary_search(&holder) {
                Ok(organism) => expected[organism].push(self.table.ids[index]),
                Err(_) => return false,
            }
        }
        expected == self.held
    }

    /// Rebuild the per-cell index of free objects.
    pub fn rebuild_cell_index(&mut self, cell_count: usize, cell_of: impl Fn(i32, i32) -> usize) {
        if self.cell_index.len() != cell_count {
            self.cell_index = vec![Vec::new(); cell_count];
        } else {
            for bucket in &mut self.cell_index {
                bucket.clear();
            }
        }
        for index in 0..self.table.len() {
            if !self.table.is_free(index) {
                continue;
            }
            let cell = cell_of(self.table.x_fp[index], self.table.y_fp[index]);
            self.cell_index[cell].push(index as u32);
        }
        // Table order is ID order, so pushing in index order leaves every
        // bucket ascending by ID already; the sort is the guarantee rather
        // than the hope, and it is a no-op on sorted input.
        for bucket in &mut self.cell_index {
            bucket.sort_unstable();
        }
    }

    /// Free objects in a cell.
    pub fn free_in_cell(&self, cell: usize) -> usize {
        self.cell_index.get(cell).map_or(0, |bucket| bucket.len())
    }

    /// Mass held by one organism.
    pub fn held_mass_milli(&self, organism: usize) -> i64 {
        self.held[organism]
            .iter()
            .filter_map(|&id| self.table.index_of(id))
            .map(|index| self.table.mass_milli[index])
            .sum()
    }

    /// Whether a free object in `cell` blocks entry.
    pub fn cell_is_blocked(&self, cell: usize, blocking_mass_milli: i64) -> bool {
        self.cell_index.get(cell).is_some_and(|bucket| {
            bucket
                .iter()
                .any(|&index| self.table.mass_milli[index as usize] >= blocking_mass_milli)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::{MATERIAL_STONE, MATERIAL_WOOD, material};

    fn stone(id: u64) -> ObjectRecord {
        ObjectRecord::simple(id, material(MATERIAL_STONE).unwrap(), 800, 100, 100, 1, CAUSE_EXTRACTED, 0)
    }

    fn table_with(records: Vec<ObjectRecord>) -> ObjectTable {
        let mut table = ObjectTable::default();
        for record in records {
            table.ledger.mass_extracted_milli += i128::from(record.mass_milli);
            table.ledger.energy_extracted_milli += i128::from(record.energy_milli);
            table.push(record);
        }
        table
    }

    #[test]
    fn a_simple_object_derives_mass_and_energy_from_its_material() {
        let record = stone(7);
        assert_eq!(record.mass_milli, 800 * 2_500 / 1_000);
        assert_eq!(record.energy_milli, 0);
        let fiber = ObjectRecord::simple(8, material(crate::material::MATERIAL_FIBER).unwrap(), 800, 0, 0, 1, CAUSE_EXTRACTED, 0);
        assert_eq!(fiber.mass_milli, 240);
        assert_eq!(fiber.energy_milli, 120);
    }

    #[test]
    fn a_fresh_table_with_matching_ledger_has_no_violation() {
        let table = table_with(vec![stone(3), stone(5)]);
        assert_eq!(table.violation(4), None);
        assert_eq!(table.free_count(), 2);
    }

    #[test]
    fn every_violation_class_is_reported_by_name() {
        // Order.
        let mut table = table_with(vec![stone(3), stone(5)]);
        table.ids.swap(0, 1);
        assert_eq!(table.violation(4), Some(TableViolation::Order(1)));
        // Domain: unknown material.
        let mut table = table_with(vec![stone(3)]);
        table.material_id[0] = 99;
        assert_eq!(table.violation(4), Some(TableViolation::Domain(0)));
        // Exclusivity: held and owned.
        let mut table = table_with(vec![stone(3)]);
        table.holder_id[0] = 1;
        table.owner_id[0] = 2;
        assert_eq!(table.violation(4), Some(TableViolation::Exclusivity(0)));
        // Ledger.
        let mut table = table_with(vec![stone(3)]);
        table.mass_milli[0] += 1;
        assert!(matches!(table.violation(4), Some(TableViolation::MassLedger { .. })));
        // Composition: stored hardness disagrees with the material.
        let mut table = table_with(vec![stone(3)]);
        table.hardness_q16[0] += 1;
        assert_eq!(table.violation(4), Some(TableViolation::Composition(0)));
        // Ragged.
        let mut table = table_with(vec![stone(3)]);
        table.parent_id.pop();
        assert_eq!(table.violation(4), Some(TableViolation::Ragged));
    }

    #[test]
    fn a_composite_is_checked_against_its_derivation() {
        let stone_def = material(MATERIAL_STONE).unwrap();
        let wood_def = material(MATERIAL_WOOD).unwrap();
        let mut table = table_with(vec![
            ObjectRecord::simple(3, stone_def, 800, 0, 0, 1, CAUSE_EXTRACTED, 0),
            ObjectRecord::simple(5, wood_def, 800, 0, 0, 1, CAUSE_EXTRACTED, 0),
        ]);
        let mass = table.mass_milli[0] + table.mass_milli[1];
        let composite = ObjectRecord {
            id: 9,
            material_id: MATERIAL_STONE,
            x_fp: 0,
            y_fp: 0,
            integrity_q16: 40_000,
            mass_milli: mass,
            energy_milli: 0,
            hardness_q16: stone_def.hardness_q16,
            durability_q16: stone_def.durability_q16.min(wood_def.durability_q16),
            decay_q16: wood_def.decay_per_tick_q16,
            holder_id: 0,
            owner_id: 0,
            depth: 1,
            created_tick: 2,
            creator_id: 11,
            cause: CAUSE_COMBINED,
            parent_id: 0,
            composition: vec![3, 5],
        };
        table.push(composite);
        table.owner_id[0] = 9;
        table.owner_id[1] = 9;
        // Constituents keep their mass and the composite stores the sum, so
        // the pool total runs over unowned objects only and each unit of
        // mass is counted exactly once. Combining is mass-neutral by
        // construction, which is what the ledger identity asserts.
        assert_eq!(table.total_mass_milli(), i128::from(mass));
        assert_eq!(table.violation(4), None);
        // A composite of depth above the cap is a domain defect.
        assert_eq!(table.violation(0), Some(TableViolation::Domain(2)));
        // A constituent whose owner does not list it.
        table.composition[2] = vec![3];
        assert_ne!(table.violation(4), None);
    }

    #[test]
    fn counters_and_ledger_round_trip_through_their_arrays_in_permanent_order() {
        let mut counters = ObjectCounters::default();
        for (position, reason) in RefuseReason::ALL.iter().enumerate() {
            for _ in 0..=position {
                counters.refuse(*reason);
            }
        }
        counters.picked_up = 7;
        let array = counters.to_array();
        assert_eq!(array[4], 7);
        assert_eq!(array[ObjectCounters::DISPOSITION_COUNT], 1);
        assert_eq!(array[ObjectCounters::FIELD_COUNT - 1], 13);
        assert_eq!(ObjectCounters::from_array(array), counters);
        assert_eq!(counters.refusals(), (1..=13).sum::<u64>());
        assert_eq!(counters.cap_refusals(), 3 + 6 + 10 + 11 + 12);

        let ledger = ObjectLedger {
            mass_extracted_milli: 1,
            mass_carcass_milli: 2,
            mass_decayed_milli: 3,
            mass_consumed_milli: 4,
            mass_dust_milli: 5,
            energy_extracted_milli: 6,
            energy_carcass_milli: 7,
            energy_decayed_milli: 8,
            energy_consumed_milli: 9,
            energy_dust_milli: 10,
        };
        assert_eq!(ObjectLedger::from_array(ledger.to_array()), ledger);
        assert_eq!(ledger.expected_mass_milli(), 1 + 2 - 3 - 4 - 5);
        assert_eq!(ledger.expected_energy_milli(), 6 + 7 - 8 - 9 - 10);
    }

    #[test]
    fn field_names_follow_the_permanent_array_order() {
        // Distinct values, then read them back by name through the array:
        // a name out of place would pair a value with the wrong field.
        let mut counters = ObjectCounters::default();
        counters.picked_up = 5;
        counters.refused_joint_failed = 30;
        counters.created_extracted = 1;
        let array = counters.to_array();
        let at = |name: &str| array[ObjectCounters::FIELD_NAMES.iter().position(|n| *n == name).unwrap()];
        assert_eq!(at("picked_up"), 5);
        assert_eq!(at("refused_joint_failed"), 30);
        assert_eq!(at("created_extracted"), 1);
        assert_eq!(ObjectCounters::FIELD_NAMES.len(), ObjectCounters::FIELD_COUNT);
        let unique: std::collections::BTreeSet<&str> = ObjectCounters::FIELD_NAMES.iter().copied().collect();
        assert_eq!(unique.len(), ObjectCounters::FIELD_COUNT);
        let ledger = ObjectLedger { energy_dust_milli: 7, ..Default::default() };
        let array = ledger.to_array();
        let at = |name: &str| array[ObjectLedger::FIELD_NAMES.iter().position(|n| *n == name).unwrap()];
        assert_eq!(at("energy_dust_milli"), 7);
        assert_eq!(at("mass_extracted_milli"), 0);
    }

    #[test]
    fn ids_actions_reasons_and_causes_round_trip_and_reject_the_unknown() {
        for action in ObjectAction::ALL {
            assert_eq!(ObjectAction::from_id(action.id()), Some(action));
        }
        assert_eq!(ObjectAction::from_id(0), None);
        assert_eq!(ObjectAction::from_id(6), None);
        for reason in RefuseReason::ALL {
            assert_eq!(RefuseReason::from_id(reason.id()), Some(reason));
        }
        assert_eq!(RefuseReason::from_id(0), None);
        assert_eq!(RefuseReason::from_id(14), None);
        for cause in [DestroyCause::Decayed, DestroyCause::Fractured, DestroyCause::Disassembled, DestroyCause::Consumed, DestroyCause::Dust, DestroyCause::Ephemeral] {
            assert_eq!(DestroyCause::from_id(cause.id()), Some(cause));
        }
        assert_eq!(DestroyCause::from_id(0), None);
        assert!(cause_is_known(CAUSE_CARCASS));
        assert!(!cause_is_known(0));
        assert!(!cause_is_known(5));
    }

    #[test]
    fn held_lists_and_cell_index_are_rebuilt_from_the_table() {
        let mut state = ObjectState::from_table(table_with(vec![stone(3), stone(5), stone(8)]));
        state.table.holder_id[1] = 20;
        state.table.x_fp[2] = 900;
        let organisms = [10_u64, 20, 30];
        state.rebuild_held(&organisms);
        assert_eq!(state.held, vec![vec![], vec![5], vec![]]);
        assert!(state.held_is_consistent(&organisms));
        state.held[0].push(3);
        assert!(!state.held_is_consistent(&organisms));
        state.rebuild_held(&organisms);
        assert_eq!(state.held_mass_milli(1), 2_000);
        state.rebuild_cell_index(4, |x, _| if x >= 500 { 1 } else { 0 });
        assert_eq!(state.cell_index[0], vec![0]);
        assert_eq!(state.cell_index[1], vec![2]);
        assert_eq!(state.free_in_cell(0), 1);
        assert!(state.cell_is_blocked(0, 2_000));
        assert!(!state.cell_is_blocked(0, 2_001));
        assert!(!state.cell_is_blocked(3, 0) || state.free_in_cell(3) > 0);
    }

    #[test]
    fn retain_compacts_every_array_in_lockstep() {
        let mut table = table_with(vec![stone(3), stone(5), stone(8)]);
        table.composition[1] = vec![1, 2];
        table.retain(&[false, true, false]);
        assert_eq!(table.ids, vec![3, 8]);
        assert_eq!(table.composition, vec![Vec::<u64>::new(), Vec::new()]);
        assert_eq!(table.len(), 2);
        assert_eq!(table.index_of(8), Some(1));
        assert_eq!(table.index_of(5), None);
    }
}
