//! The social channel (Phase 13, `lifesim-social-v1`, ADR-0029).
//!
//! Perception of the K nearest conspecifics through cues, a bounded
//! continuous costly signal field with no authored meaning, and (behind its
//! own gate) the observational plasticity rule. The design record is
//! ADR-0029; the field-level contract is
//! `specifications/social-signal-channel.md`; every place the implementation
//! departs from that specification's text is recorded in the ADR beside its
//! reason.
//!
//! This module carries the constants and (as the phase lands) the social
//! state; the tick work lives in `world/social_tick.rs` beside the artifact
//! half's, for the same reason.

/// Recorded in the config hash and in the state checksum tag.
pub const SOCIAL_POLICY_VERSION: &str = "lifesim-social-v1";

/// The registry carries nine cue channels for each of this many neighbour
/// slots, so `perception_k` is validated against it: a channel ID is
/// permanent, and a K the registry cannot express would be a binding to a
/// channel that does not exist.
pub const PERCEPTION_K_MAX: u32 = 4;

/// The registry carries this many `signal_in`/`signal_emit` pairs;
/// `signal_channels` is validated against it for the same reason.
pub const SIGNAL_CHANNELS_MAX: u32 = 4;

use crate::checksum::Fnv1a64;

/// Cue slots per organism in the perception scratch buffer: nine cues for
/// each of the four registry slots, then the four `signal_in` values.
pub const SOCIAL_CUE_COUNT: usize =
    (PERCEPTION_K_MAX as usize) * 9 + (SIGNAL_CHANNELS_MAX as usize);

/// The reference magnitude for the `neighbour_object_delta` cue: an
/// object-state change of this many milli reads as 1.0.
pub const SOCIAL_DELTA_REFERENCE_MILLI: i64 = 1_000;

/// Social counters, checksummed with the table. `..`-free destructuring in
/// `hash_into` (D-077), so a counter added later cannot be silently outside
/// the checksum its doc promises it is inside.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SocialCounters {
    /// Organism-ticks on which at least one channel was emitted.
    pub signals_emitted_total: u64,
    /// Whole milli-EU charged for emission, remainder-exact (D-094).
    pub signal_cost_milli_total: u64,
    /// Non-finite emission requests neutralized (counted and evented).
    pub perception_faults_total: u64,
    /// Reception corruption draws taken (zero in a corruption-free world).
    pub corruption_draws_total: u64,
    /// Condition-D deliveries recentred on a drawn receiver.
    pub scrambled_deliveries_total: u64,
    /// Rule-5 (Observational) plasticity updates applied. Zero until the
    /// rule lands, and the condition-S verification counter thereafter:
    /// the S arm asserts this stays zero (ADR-0029 section 5).
    pub rule5_updates_total: u64,
}

impl SocialCounters {
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            signals_emitted_total,
            signal_cost_milli_total,
            perception_faults_total,
            corruption_draws_total,
            scrambled_deliveries_total,
            rule5_updates_total,
        } = self;
        hasher.update_u64(*signals_emitted_total);
        hasher.update_u64(*signal_cost_milli_total);
        hasher.update_u64(*perception_faults_total);
        hasher.update_u64(*corruption_draws_total);
        hasher.update_u64(*scrambled_deliveries_total);
        hasher.update_u64(*rule5_updates_total);
    }
}

/// The logical social state: what is hashed, saved and restored. Carries
/// nothing derived, so `SaveState` reuses it directly the way it reuses
/// `ObjectTable` (the caches live on [`SocialState`], outside it).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SocialTable {
    /// The committed signal field, `cells * signal_channels` values in
    /// cell-major order (`cell * channels + channel`), each in
    /// `0..=Q16_ONE`. Read by the next tick's sense phase; written only at
    /// `Finalize` (decay then add, then clamp).
    pub committed_field_q16: Vec<i32>,
    /// Per organism: whether it was in contact last tick (fed, consumed,
    /// held an object, dealt or took damage). One tick of committed memory,
    /// which is why it is state: it cannot be recomputed at restore.
    pub prior_contact: Vec<bool>,
    /// Per organism: the committed magnitude of object-state change it
    /// caused last tick, normalized against
    /// [`SOCIAL_DELTA_REFERENCE_MILLI`], in `0..=Q16_ONE`.
    pub prior_object_delta_q16: Vec<i32>,
    /// Per organism: the sub-milli emission cost remainder, in Q16
    /// fractional milli (`0..65_536`), so the charge is exact to the bit
    /// (D-094). A lifetime accumulator, so integer, saved and hashed; reset
    /// at birth - a child inherits no part of its parent's bill.
    pub emission_remainder_milli: Vec<i64>,
    pub counters: SocialCounters,
}

impl SocialTable {
    pub fn new(cell_count: usize, signal_channels: u32, population: usize) -> Self {
        Self {
            committed_field_q16: vec![0; cell_count * signal_channels as usize],
            prior_contact: vec![false; population],
            prior_object_delta_q16: vec![0; population],
            emission_remainder_milli: vec![0; population],
            counters: SocialCounters::default(),
        }
    }

    /// Restore-time validation: every length in lockstep with the world it
    /// is entering and every value in its domain. Returns the first
    /// violation's name, `None` for a clean table.
    pub fn violation(
        &self,
        cell_count: usize,
        signal_channels: u32,
        population: usize,
    ) -> Option<&'static str> {
        let Self {
            committed_field_q16,
            prior_contact,
            prior_object_delta_q16,
            emission_remainder_milli,
            counters: _,
        } = self;
        if committed_field_q16.len() != cell_count * signal_channels as usize {
            return Some("committed_field_len");
        }
        if prior_contact.len() != population
            || prior_object_delta_q16.len() != population
            || emission_remainder_milli.len() != population
        {
            return Some("per_organism_len");
        }
        let q16 = crate::config::Q16_ONE as i32;
        if committed_field_q16
            .iter()
            .any(|&value| value < 0 || value > q16)
        {
            return Some("field_value_range");
        }
        if prior_object_delta_q16
            .iter()
            .any(|&value| value < 0 || value > q16)
        {
            return Some("object_delta_range");
        }
        // The remainder is Q16 fractional milli; a restored value at or
        // above one whole milli is a milli that was never charged, refused
        // rather than normalized (D-094).
        if emission_remainder_milli
            .iter()
            .any(|&value| !(0..65_536).contains(&value))
        {
            return Some("emission_remainder_range");
        }
        None
    }

    /// Checksum contribution, appended after `lifesim-object-state-v1`
    /// (Rule 8 as amended by ADR-0028: append last). `..`-free
    /// destructuring, so a field added later fails to compile rather than
    /// silently escaping the hash.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self {
            committed_field_q16,
            prior_contact,
            prior_object_delta_q16,
            emission_remainder_milli,
            counters,
        } = self;
        hasher.update(b"lifesim-social-state-v1");
        hasher.update(SOCIAL_POLICY_VERSION.as_bytes());
        hasher.update_u64(committed_field_q16.len() as u64);
        for &value in committed_field_q16 {
            hasher.update_i32(value);
        }
        hasher.update_u64(prior_contact.len() as u64);
        for &value in prior_contact {
            hasher.update_u32(u32::from(value));
        }
        for &value in prior_object_delta_q16 {
            hasher.update_i32(value);
        }
        for &value in emission_remainder_milli {
            hasher.update_i64(value);
        }
        counters.hash_into(hasher);
    }
}

/// The live wrapper: the table plus the per-tick caches nothing saves or
/// hashes - the emission staging field, the perception scratch, the
/// captured emission requests, and the current tick's contact and
/// object-delta accumulation (committed into the table at `Finalize`).
#[derive(Clone, Debug, Default)]
pub struct SocialState {
    pub table: SocialTable,
    /// Emission staging, `i64` so simultaneous emitters sum before the
    /// commit clamp. Cleared at `Finalize`.
    pub staged_field: Vec<i64>,
    /// Per organism, rebuilt each `Sense`: the 36 neighbour cues then the
    /// four `signal_in` values, in registry order.
    pub perception: Vec<[f32; SOCIAL_CUE_COUNT]>,
    /// Per organism, captured in `Controllers`: the requested emission
    /// amplitude per channel, Q16 in `0..=Q16_ONE`.
    pub emission_q16: Vec<[i32; SIGNAL_CHANNELS_MAX as usize]>,
    /// This tick's contact flags, committed into `prior_contact` at
    /// `Finalize`.
    pub contact_now: Vec<bool>,
    /// This tick's object-state-change accumulation in milli, normalized
    /// into `prior_object_delta_q16` at `Finalize`.
    pub object_delta_now_milli: Vec<i64>,
}

impl SocialState {
    pub fn from_table(table: SocialTable) -> Self {
        let field = table.committed_field_q16.len();
        let population = table.prior_contact.len();
        Self {
            table,
            staged_field: vec![0; field],
            perception: vec![[0.0; SOCIAL_CUE_COUNT]; population],
            emission_q16: vec![[0; SIGNAL_CHANNELS_MAX as usize]; population],
            contact_now: vec![false; population],
            object_delta_now_milli: vec![0; population],
        }
    }

    /// A new organism appended at birth: every per-organism array grows in
    /// lockstep, remainder zeroed (a child inherits no part of the bill).
    pub fn push_organism(&mut self) {
        self.table.prior_contact.push(false);
        self.table.prior_object_delta_q16.push(0);
        self.table.emission_remainder_milli.push(0);
        self.perception.push([0.0; SOCIAL_CUE_COUNT]);
        self.emission_q16.push([0; SIGNAL_CHANNELS_MAX as usize]);
        self.contact_now.push(false);
        self.object_delta_now_milli.push(0);
    }

    /// Compaction after deaths, dropping row `index` iff `remove[index]`,
    /// the same convention every sibling per-organism state uses. A dead
    /// organism takes its one-tick cue memory and its cost remainder with
    /// it; a row that outlived its organism would attach last tick's
    /// contact to whichever organism compacted into the slot.
    pub fn retain(&mut self, remove: &[bool]) {
        fn keep<T: Copy>(values: &mut Vec<T>, remove: &[bool]) {
            let mut write = 0_usize;
            for read in 0..values.len() {
                if !remove[read] {
                    values[write] = values[read];
                    write += 1;
                }
            }
            values.truncate(write);
        }
        keep(&mut self.table.prior_contact, remove);
        keep(&mut self.table.prior_object_delta_q16, remove);
        keep(&mut self.table.emission_remainder_milli, remove);
        keep(&mut self.perception, remove);
        keep(&mut self.emission_q16, remove);
        keep(&mut self.contact_now, remove);
        keep(&mut self.object_delta_now_milli, remove);
    }
}
