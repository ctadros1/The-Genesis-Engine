//! Per-organism action counting (Phase 11 measurement substrate for C11.1).
//!
//! C11.1 asks whether an **individual's** behaviour changes inside one
//! lifetime. A population-level shift answers a different question: births
//! and deaths between the two observations make selection a complete
//! explanation of it, so a population histogram cannot distinguish "these
//! organisms changed" from "different organisms are alive now". The unit of
//! measurement therefore has to be the organism, and it has to be the same
//! organism before and after the environmental event.
//!
//! Nothing recorded that. Intents live in `pub(crate)` scratch on
//! `Phase2State` and `ContestState`, are cleared and rebuilt every tick, and
//! have no accessor. This module is the accumulator that turns them into a
//! per-organism series.
//!
//! # A row is a partition block plus an indicator block, and the first
//! design was a measurement that could not vary
//!
//! The obvious layout is a single partition: one class per organism per tick,
//! arbitrated by a precedence like `Attack > Mate > Eat > move > rest`. It was
//! built that way and **measured before being believed**, which is what
//! caught it. In a 120-organism schema-2 world over 8,000 ticks the recorded
//! histogram was
//!
//! `[rest 0, ahead 0, left 0, right 0, eat 0, mate 458212, attack 0]`
//!
//! Every organism-tick landed in the `Mate` column and every other column was
//! empty, in **every** seed and at every horizon. The cause is not a bug in
//! the precedence: `mate_threshold_q16` is negative (mate unless the channel
//! says otherwise) and an unbound action channel reads 0, so `intent_mate` is
//! true for every organism on every tick until evolution binds that channel.
//! A top-priority class that is always set makes every other column
//! unreachable, and C11.1 would have been measured against a constant. That
//! is trap 1 in this repo's evidence list arriving through the instrument
//! rather than through the world.
//!
//! So there is no precedence. Columns 0..[`LOCOMOTION_CLASS_COUNT`] are a
//! genuine **partition** of the tick - an organism is resting or moving in
//! exactly one of three heading bands - and the remaining columns are
//! independent **indicators**, each incremented when its intent is set,
//! whether or not the others are. A locomotion row therefore sums to the
//! organism's age, and each indicator is a count in `0..=age`.
//!
//! Two things follow, and both are improvements rather than compromises.
//! First, no arbitration policy is authored at all, so there is no ordering
//! for a later reader to disagree with. Second, a saturated indicator - which
//! `Mate` is, in every world measured so far - is visible as a saturated
//! column instead of erasing the rest of the row.
//!
//! `TURN_BAND_MILLI` is the one authored value left: the turn intent is
//! continuous, and splitting locomotion into left/ahead/right needs a band
//! around zero. A relocating patch is the event C11.1 aligns on and the
//! response to it that is not confounded by where the organism happens to
//! stand is a change in **heading policy**, so collapsing all locomotion into
//! one class would throw away the channel most likely to carry the effect.
//!
//! # This is observation, and it cannot become instruction (ADR-0016)
//!
//! Nothing in the tick reads a count. `apply_phase2` writes one row entry per
//! organism per tick and never looks at the array again; no controller input,
//! no config trigger, no selection term, and no RNG draw depends on it. A
//! counter that is written and never read cannot change a trajectory, and the
//! five fixtures are the assertion of that rather than the claim.
//!
//! # It is world state, so it is saved and checksummed
//!
//! A lifetime's action counts have no source but the save: they are
//! accumulated from intents that were computed from activations that are
//! themselves stored state, and re-deriving them would need the run replayed
//! from tick zero. That is the same argument `learnstate.rs` makes, and the
//! answer is the same one - the section is saved, hashed, and appended last,
//! gated on presence so a world without the probe section hashes exactly as
//! it did before this module existed.
//!
//! The cost of that choice is stated plainly: **`reset` changes the
//! checksum**. A world whose counters were zeroed at tick 5,000 is not the
//! same world as one whose were not, and hiding that behind an unhashed field
//! would make a measurement invisible to replay. The consequence is that the
//! sampling path must not call `reset` - and it does not: the artifact
//! records cumulative rows at every sample and a before/after window is the
//! difference of two samples, which subsumes a reset and keeps sampling
//! provably read-only.

use crate::checksum::Fnv1a64;

/// Versioned measurement policy: the class set, the partition/indicator
/// split, and the turn band. A change to any of the three is a new version,
/// because a histogram under one is not comparable with a histogram under
/// another.
pub const ACTION_CENSUS_POLICY_VERSION: &str = "lifesim-action-census-v1";

/// Half-width of the "moving straight ahead" band, in milli of the turn
/// intent's own `[-1, 1]` range.
pub const TURN_BAND_MILLI: i32 = 125;

/// Coarse action classes. Bounded, ordered, and permanent for this policy
/// version: the discriminants are the column indices of every recorded
/// histogram, so they are append-only exactly as an RNG stream id is.
///
/// The first [`LOCOMOTION_CLASS_COUNT`] are mutually exclusive and partition
/// the tick; the rest are independent indicators that may co-occur with the
/// locomotion class and with each other.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionClass {
    /// No locomotion this tick.
    Rest = 0,
    /// Moving with the turn intent inside the straight-ahead band.
    MoveAhead = 1,
    /// Moving and turning left (negative turn intent).
    TurnLeft = 2,
    /// Moving and turning right (positive turn intent).
    TurnRight = 3,
    /// Indicator: the organism asked to feed.
    Eat = 4,
    /// Indicator: the organism was willing to mate.
    Mate = 5,
    /// Indicator: the organism asked to attack.
    Attack = 6,
}

/// How many classes a row carries.
pub const ACTION_CLASS_COUNT: usize = 7;

/// How many leading columns form the mutually exclusive locomotion
/// partition. Columns at or above this index are independent indicators.
pub const LOCOMOTION_CLASS_COUNT: usize = 4;

impl ActionClass {
    pub const ALL: [ActionClass; ACTION_CLASS_COUNT] = [
        ActionClass::Rest,
        ActionClass::MoveAhead,
        ActionClass::TurnLeft,
        ActionClass::TurnRight,
        ActionClass::Eat,
        ActionClass::Mate,
        ActionClass::Attack,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ActionClass::Rest => "rest",
            ActionClass::MoveAhead => "move_ahead",
            ActionClass::TurnLeft => "turn_left",
            ActionClass::TurnRight => "turn_right",
            ActionClass::Eat => "eat",
            ActionClass::Mate => "mate",
            ActionClass::Attack => "attack",
        }
    }
}

/// Which locomotion class one organism's movement intents fall in.
///
/// A free function rather than a method so it can be unit-tested at its
/// boundaries without a world, and so the world's accumulation site has
/// nothing in it but a call and an increment.
///
/// `turn` is the bounded turn intent in `[-1, 1]`; `speed_milli` is the speed
/// intent, which is zero exactly when the organism asked to rest or the
/// throttle mapped to zero.
pub fn locomotion(turn: f32, speed_milli: i64) -> ActionClass {
    if speed_milli <= 0 {
        return ActionClass::Rest;
    }
    // A non-finite turn intent cannot reach here through the controller,
    // which clamps to [-1, 1], but the comparison below would silently
    // report `MoveAhead` for a NaN and that is the wrong direction to fail
    // in for a measurement. Converted through milli so the band is an exact
    // integer comparison rather than a float threshold that differs by
    // rounding between two builds.
    let turn_milli = if turn.is_finite() {
        (turn * 1_000.0) as i32
    } else {
        0
    };
    if turn_milli < -TURN_BAND_MILLI {
        ActionClass::TurnLeft
    } else if turn_milli > TURN_BAND_MILLI {
        ActionClass::TurnRight
    } else {
        ActionClass::MoveAhead
    }
}

/// Census-wide counters, hashed alongside the rows.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ActionCensusCounters {
    /// Organism-ticks classified over the run. The denominator a reader needs
    /// to tell "nothing was recorded" from "every organism rested".
    pub classified_total: u64,
    /// Times [`ActionCensus::reset`] has run.
    ///
    /// Hashed on purpose. Without it, resetting a census whose rows were
    /// already zero would be invisible in the checksum, and a probe boundary
    /// that left no trace is a boundary a replay cannot reproduce.
    pub resets_total: u64,
}

impl ActionCensusCounters {
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        // Destructured with no `..` (D-077).
        let Self {
            classified_total,
            resets_total,
        } = self;
        hasher.update_u64(*classified_total);
        hasher.update_u64(*resets_total);
    }
}

/// Parallel per-organism rows, kept in lockstep with the world's primary
/// arrays exactly as `LearnState` and `MorphologyState` are.
#[derive(Clone, Debug, Default)]
pub(crate) struct ActionCensus {
    /// One histogram per organism, in entity-ID order.
    pub counts: Vec<[u32; ACTION_CLASS_COUNT]>,
    pub counters: ActionCensusCounters,
}

impl ActionCensus {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            counts: Vec::with_capacity(capacity),
            counters: ActionCensusCounters::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.counts.len()
    }

    /// Admit one organism with an all-zero histogram.
    ///
    /// Takes no initial value, for the reason `LearnState::push_organism`
    /// takes none: a child's action history is its own, and a birth path that
    /// could pass a parent's counts would make the series meaningless without
    /// any test failing.
    pub fn push_organism(&mut self) {
        self.counts.push([0; ACTION_CLASS_COUNT]);
    }

    /// Compact after deaths, with the same removal flags every other
    /// subsystem gets.
    pub fn retain(&mut self, remove: &[bool]) {
        let mut write = 0_usize;
        for (read, removed) in remove.iter().enumerate() {
            if !removed {
                if write != read {
                    self.counts.swap(write, read);
                }
                write += 1;
            }
        }
        self.counts.truncate(write);
    }

    /// Record one organism-tick: one locomotion column and any indicators
    /// that are set.
    ///
    /// `saturating_add` rather than a wrap: a u32 column holds 4.29e9 ticks
    /// against a 10^6-tick ledger run, so saturation is unreachable in
    /// practice - and a silent wrap would turn a long-run histogram into
    /// noise that looks like data, which is the failure this cannot be
    /// allowed to have.
    ///
    /// `classified_total` counts **organism-ticks**, not increments, so it
    /// stays the denominator a reader needs whatever the indicators did.
    pub fn record(
        &mut self,
        index: usize,
        turn: f32,
        speed_milli: i64,
        eat: bool,
        mate: bool,
        attack: bool,
    ) {
        let row = &mut self.counts[index];
        let mut bump = |class: ActionClass| {
            row[class as usize] = row[class as usize].saturating_add(1);
        };
        bump(locomotion(turn, speed_milli));
        if eat {
            bump(ActionClass::Eat);
        }
        if mate {
            bump(ActionClass::Mate);
        }
        if attack {
            bump(ActionClass::Attack);
        }
        self.counters.classified_total = self.counters.classified_total.saturating_add(1);
    }

    /// Zero every organism's histogram, leaving the population untouched.
    ///
    /// The probe boundary C11.1 needs, and a **state change**: it is counted
    /// and it moves the checksum. Called by no tick phase and by no sampling
    /// path; a caller that wants before/after windows without perturbing the
    /// world differences two cumulative samples instead.
    pub fn reset(&mut self) {
        for row in &mut self.counts {
            *row = [0; ACTION_CLASS_COUNT];
        }
        self.counters.resets_total = self.counters.resets_total.saturating_add(1);
    }

    /// Hash every field under the section tag.
    ///
    /// **Destructured with no `..` (D-077).** The byte order is permanent:
    /// it is the definition of a probe world's checksum. Append, never
    /// reorder.
    pub fn hash_into(&self, hasher: &mut Fnv1a64) {
        let Self { counts, counters } = self;
        hasher.update(b"lifesim-action-census-v1");
        for row in counts {
            for value in row {
                hasher.update_u32(*value);
            }
        }
        counters.hash_into(hasher);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Organism-ticks one row's locomotion block accounts for.
    fn locomotion_total(census: &ActionCensus, index: usize) -> u64 {
        census.counts[index][..LOCOMOTION_CLASS_COUNT]
            .iter()
            .map(|v| u64::from(*v))
            .sum()
    }

    fn hash(census: &ActionCensus) -> u64 {
        let mut hasher = Fnv1a64::new();
        census.hash_into(&mut hasher);
        hasher.finish()
    }

    fn populated() -> ActionCensus {
        let mut census = ActionCensus::with_capacity(3);
        for index in 0..3_usize {
            census.push_organism();
            for slot in 0..ACTION_CLASS_COUNT {
                census.counts[index][slot] = (index as u32 + 1) * 10 + slot as u32;
            }
        }
        census
    }

    #[test]
    fn the_locomotion_block_partitions_and_the_indicators_do_not() {
        // The property the single-partition design could not have: an
        // organism that is moving, feeding, willing to mate and attacking on
        // the same tick contributes to four columns, exactly one of which is
        // a locomotion column.
        let mut census = ActionCensus::with_capacity(1);
        census.push_organism();
        census.record(0, 1.0, 500, true, true, true);
        assert_eq!(census.counts[0][ActionClass::TurnRight as usize], 1);
        assert_eq!(census.counts[0][ActionClass::Eat as usize], 1);
        assert_eq!(census.counts[0][ActionClass::Mate as usize], 1);
        assert_eq!(census.counts[0][ActionClass::Attack as usize], 1);
        assert_eq!(
            locomotion_total(&census, 0),
            1,
            "two locomotion columns moved"
        );
        assert_eq!(census.counters.classified_total, 1, "one organism-tick");

        // A saturated indicator - which `Mate` is in every world measured so
        // far - must not erase anything. This is the assertion that fails on
        // any return to a precedence.
        for _ in 0..9 {
            census.record(0, 0.0, 0, false, true, false);
        }
        assert_eq!(census.counts[0][ActionClass::Rest as usize], 9);
        assert_eq!(census.counts[0][ActionClass::Mate as usize], 10);
        assert_eq!(locomotion_total(&census, 0), 10);
        assert_eq!(census.counters.classified_total, 10);
    }

    #[test]
    fn the_turn_band_is_closed_on_both_sides_and_symmetric() {
        let band = TURN_BAND_MILLI as f32 / 1_000.0;
        assert_eq!(locomotion(band, 1), ActionClass::MoveAhead);
        assert_eq!(locomotion(-band, 1), ActionClass::MoveAhead);
        // One milli outside the band, on each side.
        assert_eq!(locomotion(band + 0.002, 1), ActionClass::TurnRight);
        assert_eq!(locomotion(-band - 0.002, 1), ActionClass::TurnLeft);
        // A zero or negative speed intent is rest, not reverse, and rest wins
        // over any turn intent because a stationary organism has no heading
        // change to record.
        assert_eq!(locomotion(1.0, 0), ActionClass::Rest);
        assert_eq!(locomotion(1.0, -5), ActionClass::Rest);
        // A non-finite turn is reported as straight rather than as a
        // silently-signed direction.
        assert_eq!(locomotion(f32::NAN, 1), ActionClass::MoveAhead);
        // Every locomotion result is inside the partition block, which is
        // what makes `LOCOMOTION_CLASS_COUNT` a fact rather than a comment.
        for turn in [-1.0_f32, -0.2, 0.0, 0.2, 1.0] {
            for speed in [0_i64, 1, 900] {
                assert!((locomotion(turn, speed) as usize) < LOCOMOTION_CLASS_COUNT);
            }
        }
    }

    #[test]
    fn a_new_organism_starts_at_zero_and_leaves_its_neighbours_alone() {
        let mut census = populated();
        assert!(census.counts[2].iter().any(|value| *value != 0));
        census.push_organism();
        assert_eq!(census.counts[3], [0; ACTION_CLASS_COUNT]);
        assert_eq!(census.counts[0][0], 10);
    }

    #[test]
    fn compaction_leaves_the_survivors_matching_a_census_built_from_them() {
        // Lengths alone cannot catch a mis-paired row, which is the whole
        // failure `ActionCensusDesync` names, so this compares contents.
        // Keeping 0 and 2 forces the last survivor to move.
        let mut census = populated();
        let mut expected = ActionCensus::with_capacity(2);
        for index in [0_usize, 2] {
            expected.push_organism();
            let slot = expected.len() - 1;
            expected.counts[slot] = census.counts[index];
        }
        expected.counters = census.counters;
        assert_ne!(hash(&census), hash(&expected), "nothing was removed yet");
        census.retain(&[false, true, false]);
        assert_eq!(census.len(), 2);
        assert_eq!(hash(&census), hash(&expected));
        assert_eq!(census.counts[1][0], 30);
    }

    #[test]
    fn every_field_reaches_the_checksum() {
        let base = populated();
        let reference = hash(&base);
        let mutators: [fn(&mut ActionCensus); 3] = [
            |census| census.counts[1][0] += 1,
            |census| census.counters.classified_total += 1,
            |census| census.counters.resets_total += 1,
        ];
        for (index, mutate) in mutators.into_iter().enumerate() {
            let mut moved = base.clone();
            mutate(&mut moved);
            assert_ne!(hash(&moved), reference, "field {index} missed the hash");
        }
        // The last column is as hashed as the first: a loop bound off by one
        // would leave `Attack` out and nothing else would notice.
        let mut last = base.clone();
        last.counts[0][ACTION_CLASS_COUNT - 1] += 1;
        assert_ne!(hash(&last), reference);
    }

    #[test]
    fn a_reset_is_visible_in_the_checksum_even_when_every_row_was_already_zero() {
        // The case a `resets_total` counter exists for. Without it this
        // assertion is false and a probe boundary leaves no trace at all.
        let mut census = ActionCensus::with_capacity(2);
        census.push_organism();
        census.push_organism();
        let before = hash(&census);
        census.reset();
        assert_ne!(hash(&census), before);
        assert_eq!(census.counters.resets_total, 1);
        assert_eq!(census.len(), 2, "a reset must not remove an organism");
    }

    #[test]
    fn recording_lands_in_the_named_column_and_a_reset_clears_it() {
        let mut census = ActionCensus::with_capacity(1);
        census.push_organism();
        census.record(0, 0.0, 500, true, false, false);
        census.record(0, 0.0, 500, true, false, false);
        census.record(0, -1.0, 500, false, false, false);
        assert_eq!(census.counts[0][ActionClass::Eat as usize], 2);
        assert_eq!(census.counts[0][ActionClass::TurnLeft as usize], 1);
        assert_eq!(census.counts[0][ActionClass::MoveAhead as usize], 2);
        assert_eq!(locomotion_total(&census, 0), 3);
        assert_eq!(census.counters.classified_total, 3);
        census.reset();
        assert_eq!(locomotion_total(&census, 0), 0);
        assert_eq!(census.counts[0], [0; ACTION_CLASS_COUNT]);
    }

    #[test]
    fn the_class_names_are_distinct_and_cover_the_column_set() {
        let mut names: Vec<&str> = ActionClass::ALL.iter().map(|c| c.name()).collect();
        assert_eq!(names.len(), ACTION_CLASS_COUNT);
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), ACTION_CLASS_COUNT);
        for (index, class) in ActionClass::ALL.into_iter().enumerate() {
            assert_eq!(class as usize, index, "ALL is not in discriminant order");
        }
    }
}
