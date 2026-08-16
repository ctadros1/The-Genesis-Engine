//! ALEV append-only event log, format version 1 (Phase 5).
//!
//! This is the file deferred as D-019: snapshots have carried a zero
//! `event_log_offset` since Phase 4 because there was nothing to point at.
//! Every multi-seed experiment and all of Phase 16 read this file.
//!
//! Layout (little-endian, matching the ALIF snapshot codec):
//!
//! header (fixed 64 bytes):
//!   magic "ALEV" | format u16 | header_len u16 | flags u32
//!   world_id u64 | seed u64 | config_hash u64
//!   event_schema_version u32 | max_events_per_tick u32
//!   build_len u16 | reserved u16 | start_tick u64 | reserved u32
//!   | header_crc32 u32
//! then: build version string (build_len <= 64 bytes)
//!
//! The header CRC covers the first 60 bytes and the build string. Unlike a
//! snapshot header, most of what this one carries is provenance — which
//! seed, which config hash, which build — rather than framing, and a
//! silently corrupted provenance field would mislabel an experiment instead
//! of failing it. Nothing else in the file would catch that, so the header
//! checks itself.
//! then: zero or more segments, one per recorded tick, in ascending tick
//! order:
//!   magic "SEG1" | tick u64 | count u32 | dropped u32 | body_len u32
//!   | body | crc32 u32
//!
//! The body is `count` events, each `type u8` followed by a fixed payload.
//!
//! Decoding treats input as hostile in the same way the snapshot codec
//! does: every length is capped before allocation, the segment CRC is
//! verified before any payload is parsed, and an unknown event type fails
//! closed rather than being skipped. A skipped event would silently corrupt
//! any analysis that counts rates, which is the whole purpose of this file
//! (`specifications/event-schema.md`).
//!
//! Appending is the only supported mutation. There is no rewrite path and
//! no repair path.

use sim_core::{
    Counters, DeathCause, EVENT_SCHEMA_VERSION, Event, EventKind, MAX_EVENTS_PER_TICK, OP_POINT,
    OP_TRANSPOSITION, PairRejectReason, Phase2Counters, RejectReason, World,
};
use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

pub const EVENT_LOG_MAGIC: &[u8; 4] = b"ALEV";
pub const EVENT_LOG_FORMAT_VERSION: u16 = 1;
const SEGMENT_MAGIC: &[u8; 4] = b"SEG1";
const EVENT_LOG_HEADER_LEN: usize = 64;
/// Byte offset of the header CRC inside the fixed header.
const HEADER_CRC_OFFSET: usize = 60;
const SEGMENT_HEADER_LEN: usize = 24;
const MAX_BUILD_LEN: usize = 64;

/// Largest encoded single event (`PairedBirth`: tag + 4x u64 + 2x i64 +
/// 2x u32 = 57 bytes; `Damage` is 1 + 2x u64 + 3x i64 = 41), rounded up so a
/// payload addition inside the cap does not silently change the bound.
const MAX_EVENT_LEN: usize = 64;

/// Absolute cap applied to a declared segment body before any allocation.
pub const MAX_SEGMENT_BODY_LEN: u64 = (MAX_EVENTS_PER_TICK * MAX_EVENT_LEN) as u64;

// Event type tags. Permanent: a tag is never reused or renumbered, exactly
// like an `RngSystem` value.
const TAG_BIRTH: u8 = 1;
const TAG_DEATH: u8 = 2;
const TAG_CAPACITY_REJECTED: u8 = 3;
const TAG_EXTINCTION: u8 = 4;
const TAG_PAIRED_BIRTH: u8 = 5;
const TAG_PAIR_REJECTED: u8 = 6;
const TAG_CONTROLLER_FAULT: u8 = 7;
// Phase 7, event schema 3. Additive; tags 1-7 are unchanged.
const TAG_DAMAGE: u8 = 8;
const TAG_DEATH_BY_DAMAGE: u8 = 9;
const TAG_CARCASS_CREATED: u8 = 10;
const TAG_CARCASS_CONSUMED: u8 = 11;
// Phase 9 C9.6, event schema 4. Additive; tags 1-11 are unchanged.
const TAG_STRUCTURAL_MUTATION_REJECTED: u8 = 12;
// Phase 11, event schema 5. Additive; tags 1-12 are unchanged, so every log
// written before this build decodes byte for byte.
const TAG_PLASTICITY_FAULT: u8 = 13;
// Phase 12 artifact half, event schema 6. Additive; tags 1-13 are unchanged.
const TAG_OBJECT_CREATED: u8 = 14;
const TAG_OBJECT_DESTROYED: u8 = 15;
const TAG_OBJECT_PICKED_UP: u8 = 16;
const TAG_OBJECT_RELEASED: u8 = 17;
const TAG_OBJECT_STRUCK: u8 = 18;
const TAG_TERRAIN_STRUCK: u8 = 19;
const TAG_OBJECT_COMBINED: u8 = 20;
const TAG_OBJECT_CONSUMED: u8 = 21;
const TAG_OBJECT_ACTION_REFUSED: u8 = 22;
const TAG_OBJECT_EXPOSURE: u8 = 23;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EventLogError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    UnsupportedEventSchema(u32),
    BuildStringTooLong(usize),
    BadBuildString,
    /// The header's own CRC does not match, so its provenance fields cannot
    /// be trusted even though the segments after it might decode.
    HeaderChecksumMismatch,
    /// A segment declared more events than the kernel's bounded per-tick
    /// buffer can hold, so the file cannot have come from this kernel.
    SegmentCountTooLarge(u32),
    SegmentBodyTooLarge(u64),
    SegmentChecksumMismatch {
        tick: u64,
    },
    /// The trailing bytes are shorter than the segment they declare. A
    /// crash between `write` and `sync` produces exactly this.
    TruncatedSegment {
        offset: usize,
    },
    BadSegmentMagic {
        offset: usize,
    },
    /// Segments must be in strictly ascending tick order.
    TickOutOfOrder {
        previous: u64,
        found: u64,
    },
    /// The body did not contain exactly `count` events.
    SegmentBodyLengthMismatch {
        tick: u64,
    },
    UnknownEventType {
        tick: u64,
        tag: u8,
    },
    ValueOutOfRange(&'static str),
    Io(String),
}

impl fmt::Display for EventLogError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for EventLogError {}

/// Provenance recorded in the file header.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogInfo {
    pub format_version: u16,
    pub world_id: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub event_schema_version: u32,
    pub max_events_per_tick: u32,
    pub start_tick: u64,
    pub build_version: String,
}

/// Counters reconstructed by replaying a log. Field-for-field the union of
/// `sim_core::Counters` and `sim_core::Phase2Counters`, which is what makes
/// A5.3's equality check exact rather than approximate.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconstructedCounters {
    pub births_total: u64,
    pub deaths_starvation_total: u64,
    pub deaths_old_age_total: u64,
    pub capacity_rejections_total: u64,
    pub extinctions_total: u64,
    pub paired_births_total: u64,
    pub pair_rejected_capacity_total: u64,
    pub pair_rejected_placement_total: u64,
    pub pair_rejected_energy_total: u64,
    pub pair_rejected_nonviable_total: u64,
    pub controller_faults_total: u64,
    pub mutated_trait_genes_total: u64,
    pub mutated_neural_genes_total: u64,
    // Phase 7. Reconstructed for analysis; `reconcile` continues to check
    // the Phase 1/2 counters, because these live in the contest section
    // rather than in `Counters` or `Phase2Counters`.
    pub attacks_total: u64,
    pub damage_dealt_milli: i128,
    pub deaths_by_damage_total: u64,
    pub carcasses_created_total: u64,
    pub carcasses_consumed_total: u64,
    /// Phase 9 C9.6. Indexed by `RejectReason::code() - 1`, so the array
    /// position is the permanent wire code rather than a declaration order
    /// that could be reshuffled.
    pub structural_rejections_by_reason: [u64; 8],
    pub structural_rejections_total: u64,
    /// Phase 11. Neutralized plastic-edge updates, summed the way
    /// `controller_faults_total` is: the kernel sums neutralized values, not
    /// fault events, so an event carrying five faults counts five.
    pub plasticity_faults_total: u64,
    /// Phase 12 artifact half. Reconstructed per cause and per reason so the
    /// log can be checked against `ObjectCounters` class by class; the
    /// arrays are indexed by the permanent wire code minus one, exactly as
    /// `structural_rejections_by_reason` is.
    pub objects_created_by_cause: [u64; 4],
    pub objects_destroyed_by_cause: [u64; 6],
    pub objects_picked_up_total: u64,
    pub objects_dropped_total: u64,
    pub objects_placed_total: u64,
    pub objects_struck_total: u64,
    pub terrain_struck_total: u64,
    pub objects_combined_total: u64,
    pub objects_consumed_total: u64,
    pub object_refusals_by_reason: [u64; 13],
    pub object_refusals_total: u64,
    /// `ObjectExposure` records seen: one per death in an artifact world.
    pub object_exposure_records: u64,
}

impl ReconstructedCounters {
    fn observe(&mut self, kind: &EventKind) {
        match *kind {
            EventKind::Birth { .. } => self.births_total += 1,
            EventKind::Death { cause, .. } => match cause {
                DeathCause::Starvation => self.deaths_starvation_total += 1,
                DeathCause::OldAge => self.deaths_old_age_total += 1,
                // Counted by the dedicated `DeathByDamage` event, which the
                // kernel emits alongside this one, so counting it here too
                // would double it.
                DeathCause::Damage => {}
                // Phase 8 causes live in the physiology section's own
                // counters, exactly as damage deaths live in the contest
                // section's, so the Phase 1/2 counter list is untouched.
                DeathCause::Senescence | DeathCause::Extrinsic => {}
            },
            EventKind::CapacityRejected { .. } => self.capacity_rejections_total += 1,
            EventKind::Extinction => self.extinctions_total += 1,
            EventKind::PairedBirth {
                mutated_trait_genes,
                mutated_neural_genes,
                ..
            } => {
                // A paired birth is a birth: the kernel increments both
                // `births_total` and `paired_births_total` (world.rs).
                self.births_total += 1;
                self.paired_births_total += 1;
                self.mutated_trait_genes_total += u64::from(mutated_trait_genes);
                self.mutated_neural_genes_total += u64::from(mutated_neural_genes);
            }
            EventKind::PairRejected { reason, .. } => match reason {
                PairRejectReason::Capacity => self.pair_rejected_capacity_total += 1,
                PairRejectReason::Placement => self.pair_rejected_placement_total += 1,
                PairRejectReason::Energy => self.pair_rejected_energy_total += 1,
                PairRejectReason::Nonviable => self.pair_rejected_nonviable_total += 1,
            },
            // The kernel sums neutralized values, not fault events.
            EventKind::ControllerFault { faults, .. } => {
                self.controller_faults_total += u64::from(faults);
            }
            EventKind::Damage { applied_milli, .. } => {
                self.attacks_total += 1;
                self.damage_dealt_milli += i128::from(applied_milli);
            }
            EventKind::DeathByDamage { .. } => self.deaths_by_damage_total += 1,
            EventKind::CarcassCreated { .. } => self.carcasses_created_total += 1,
            EventKind::CarcassConsumed { .. } => self.carcasses_consumed_total += 1,
            // Reconstructed per class, so the log can be checked against
            // `MutationCounters` reason by reason rather than only in total
            // - a single total would agree while two classes were swapped.
            EventKind::StructuralMutationRejected { reason, .. } => {
                self.structural_rejections_total += 1;
                let slot = usize::from(reason.code() - 1);
                if let Some(counter) = self.structural_rejections_by_reason.get_mut(slot) {
                    *counter += 1;
                }
            }
            EventKind::PlasticityFault { faults, .. } => {
                self.plasticity_faults_total += u64::from(faults);
            }
            EventKind::ObjectCreated { cause, .. } => {
                if let Some(slot) = self.objects_created_by_cause.get_mut(usize::from(cause.saturating_sub(1))) {
                    *slot += 1;
                }
            }
            EventKind::ObjectDestroyed { cause, .. } => {
                if let Some(slot) = self.objects_destroyed_by_cause.get_mut(usize::from(cause.saturating_sub(1))) {
                    *slot += 1;
                }
            }
            EventKind::ObjectPickedUp { .. } => self.objects_picked_up_total += 1,
            EventKind::ObjectReleased { placed, .. } => {
                if placed {
                    self.objects_placed_total += 1;
                } else {
                    self.objects_dropped_total += 1;
                }
            }
            EventKind::ObjectStruck { .. } => self.objects_struck_total += 1,
            EventKind::TerrainStruck { .. } => self.terrain_struck_total += 1,
            EventKind::ObjectCombined { .. } => {
                self.objects_combined_total += 1;
                // A composite's creation record *is* its `ObjectCombined`:
                // the kernel emits no separate `ObjectCreated` for it (the
                // constituents' records already carry the mass and energy),
                // so the created-by-cause slot for `CAUSE_COMBINED` is
                // filled from here. `phase12_object_events.rs` is the
                // check that this equals `created_combined`.
                self.objects_created_by_cause[usize::from(sim_core::CAUSE_COMBINED - 1)] += 1;
            }
            EventKind::ObjectConsumed { .. } => self.objects_consumed_total += 1,
            EventKind::ObjectActionRefused { reason, .. } => {
                self.object_refusals_total += 1;
                if let Some(slot) = self.object_refusals_by_reason.get_mut(usize::from(reason.saturating_sub(1))) {
                    *slot += 1;
                }
            }
            EventKind::ObjectExposure { .. } => self.object_exposure_records += 1,
        }
    }
}

/// Result of replaying a whole log.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventLogScan {
    pub info: EventLogInfo,
    pub segments: u64,
    pub events: u64,
    /// Sum of the per-segment drop counts the kernel reported.
    pub dropped: u64,
    pub first_tick: Option<u64>,
    pub last_tick: Option<u64>,
    pub counters: ReconstructedCounters,
    /// Bytes of valid log consumed, including the header.
    pub bytes_consumed: usize,
}

/// Why a reconstruction did not reproduce a world's counters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileError {
    /// A counter differs while the log recorded no drops at all, so the log
    /// is incomplete for a reason other than the bounded buffer.
    CounterMismatch {
        field: &'static str,
        reconstructed: u64,
        world: u64,
    },
    /// The log claims more of something than the world ever counted.
    ReconstructionExceedsWorld {
        field: &'static str,
        reconstructed: u64,
        world: u64,
    },
    /// Drops occurred but the log's own recorded drop total disagrees with
    /// the kernel's.
    DropCountMismatch { recorded: u64, world: u64 },
    /// The Phase 2 section is present on one side only.
    Phase2SectionMismatch,
}

impl fmt::Display for ReconcileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ReconcileError {}

impl EventLogScan {
    /// Check a replayed log against the counters of the world that wrote it
    /// (acceptance criterion A5.3).
    ///
    /// With zero drops the check is exact equality on every counter. With
    /// drops, exact per-counter equality is not recoverable — a dropped
    /// event leaves no record of *which* counter it would have advanced —
    /// so the check becomes the two things that are exactly checkable: the
    /// log's own recorded drop total must equal the kernel's, and no
    /// reconstructed counter may exceed the world's.
    pub fn reconcile(
        &self,
        counters: &Counters,
        phase2: Option<&Phase2Counters>,
    ) -> Result<(), ReconcileError> {
        let reconstructed = &self.counters;
        let mut pairs: Vec<(&'static str, u64, u64)> = vec![
            (
                "births_total",
                reconstructed.births_total,
                counters.births_total,
            ),
            (
                "deaths_starvation_total",
                reconstructed.deaths_starvation_total,
                counters.deaths_starvation_total,
            ),
            (
                "deaths_old_age_total",
                reconstructed.deaths_old_age_total,
                counters.deaths_old_age_total,
            ),
            (
                "capacity_rejections_total",
                reconstructed.capacity_rejections_total,
                counters.capacity_rejections_total,
            ),
        ];
        match phase2 {
            Some(phase2) => pairs.extend_from_slice(&[
                (
                    "paired_births_total",
                    reconstructed.paired_births_total,
                    phase2.paired_births_total,
                ),
                (
                    "pair_rejected_capacity_total",
                    reconstructed.pair_rejected_capacity_total,
                    phase2.pair_rejected_capacity_total,
                ),
                (
                    "pair_rejected_placement_total",
                    reconstructed.pair_rejected_placement_total,
                    phase2.pair_rejected_placement_total,
                ),
                (
                    "pair_rejected_energy_total",
                    reconstructed.pair_rejected_energy_total,
                    phase2.pair_rejected_energy_total,
                ),
                (
                    "pair_rejected_nonviable_total",
                    reconstructed.pair_rejected_nonviable_total,
                    phase2.pair_rejected_nonviable_total,
                ),
                (
                    "controller_faults_total",
                    reconstructed.controller_faults_total,
                    phase2.controller_faults_total,
                ),
                (
                    "mutated_trait_genes_total",
                    reconstructed.mutated_trait_genes_total,
                    phase2.mutated_trait_genes_total,
                ),
                (
                    "mutated_neural_genes_total",
                    reconstructed.mutated_neural_genes_total,
                    phase2.mutated_neural_genes_total,
                ),
            ]),
            None => {
                // A phase2-disabled world can never emit a Phase 2 event.
                if reconstructed.paired_births_total != 0
                    || reconstructed.pair_rejected_capacity_total != 0
                    || reconstructed.pair_rejected_placement_total != 0
                    || reconstructed.pair_rejected_energy_total != 0
                    || reconstructed.pair_rejected_nonviable_total != 0
                    || reconstructed.controller_faults_total != 0
                {
                    return Err(ReconcileError::Phase2SectionMismatch);
                }
            }
        }

        if self.dropped != counters.dropped_events_total {
            return Err(ReconcileError::DropCountMismatch {
                recorded: self.dropped,
                world: counters.dropped_events_total,
            });
        }
        for (field, reconstructed, world) in pairs {
            if self.dropped == 0 {
                if reconstructed != world {
                    return Err(ReconcileError::CounterMismatch {
                        field,
                        reconstructed,
                        world,
                    });
                }
            } else if reconstructed > world {
                return Err(ReconcileError::ReconstructionExceedsWorld {
                    field,
                    reconstructed,
                    world,
                });
            }
        }
        Ok(())
    }
}

// --- Encoding ---------------------------------------------------------------

fn encode_header(info: &EventLogInfo) -> Result<Vec<u8>, EventLogError> {
    let build = info.build_version.as_bytes();
    if build.len() > MAX_BUILD_LEN {
        return Err(EventLogError::BuildStringTooLong(build.len()));
    }
    let mut out = Vec::with_capacity(EVENT_LOG_HEADER_LEN + build.len());
    out.extend_from_slice(EVENT_LOG_MAGIC);
    out.extend_from_slice(&EVENT_LOG_FORMAT_VERSION.to_le_bytes());
    out.extend_from_slice(&(EVENT_LOG_HEADER_LEN as u16).to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // flags
    out.extend_from_slice(&info.world_id.to_le_bytes());
    out.extend_from_slice(&info.seed.to_le_bytes());
    out.extend_from_slice(&info.config_hash.to_le_bytes());
    out.extend_from_slice(&info.event_schema_version.to_le_bytes());
    out.extend_from_slice(&info.max_events_per_tick.to_le_bytes());
    out.extend_from_slice(&(build.len() as u16).to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes()); // reserved
    out.extend_from_slice(&info.start_tick.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes()); // reserved
    debug_assert_eq!(out.len(), HEADER_CRC_OFFSET);
    // CRC over the header so far plus the build string that follows it.
    let mut covered = out.clone();
    covered.extend_from_slice(build);
    out.extend_from_slice(&crate::codec::crc32(&covered).to_le_bytes());
    debug_assert_eq!(out.len(), EVENT_LOG_HEADER_LEN);
    out.extend_from_slice(build);
    Ok(out)
}

fn encode_event(out: &mut Vec<u8>, kind: &EventKind) {
    match *kind {
        EventKind::Birth { id, parent_id } => {
            out.push(TAG_BIRTH);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&parent_id.to_le_bytes());
        }
        EventKind::Death { id, cause } => {
            out.push(TAG_DEATH);
            out.extend_from_slice(&id.to_le_bytes());
            out.push(match cause {
                DeathCause::Starvation => 0,
                DeathCause::OldAge => 1,
                DeathCause::Damage => 2,
                // Additive: tags 0..=2 are unchanged, so every Phase 7 log
                // decodes byte-identically.
                DeathCause::Senescence => 3,
                DeathCause::Extrinsic => 4,
            });
        }
        EventKind::CapacityRejected { parent_id } => {
            out.push(TAG_CAPACITY_REJECTED);
            out.extend_from_slice(&parent_id.to_le_bytes());
        }
        EventKind::Extinction => out.push(TAG_EXTINCTION),
        EventKind::PairedBirth {
            id,
            parent_a,
            parent_b,
            genome_hash,
            invest_a_milli,
            invest_b_milli,
            mutated_trait_genes,
            mutated_neural_genes,
        } => {
            out.push(TAG_PAIRED_BIRTH);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&parent_a.to_le_bytes());
            out.extend_from_slice(&parent_b.to_le_bytes());
            out.extend_from_slice(&genome_hash.to_le_bytes());
            out.extend_from_slice(&invest_a_milli.to_le_bytes());
            out.extend_from_slice(&invest_b_milli.to_le_bytes());
            out.extend_from_slice(&mutated_trait_genes.to_le_bytes());
            out.extend_from_slice(&mutated_neural_genes.to_le_bytes());
        }
        EventKind::PairRejected {
            parent_a,
            parent_b,
            reason,
        } => {
            out.push(TAG_PAIR_REJECTED);
            out.extend_from_slice(&parent_a.to_le_bytes());
            out.extend_from_slice(&parent_b.to_le_bytes());
            out.push(match reason {
                PairRejectReason::Capacity => 0,
                PairRejectReason::Placement => 1,
                PairRejectReason::Energy => 2,
                PairRejectReason::Nonviable => 3,
            });
        }
        EventKind::ControllerFault { id, faults } => {
            out.push(TAG_CONTROLLER_FAULT);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&faults.to_le_bytes());
        }
        EventKind::Damage {
            attacker,
            target,
            raw_milli,
            applied_milli,
            health_milli,
        } => {
            out.push(TAG_DAMAGE);
            out.extend_from_slice(&attacker.to_le_bytes());
            out.extend_from_slice(&target.to_le_bytes());
            out.extend_from_slice(&raw_milli.to_le_bytes());
            out.extend_from_slice(&applied_milli.to_le_bytes());
            out.extend_from_slice(&health_milli.to_le_bytes());
        }
        EventKind::DeathByDamage { id, attacker } => {
            out.push(TAG_DEATH_BY_DAMAGE);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&attacker.to_le_bytes());
        }
        EventKind::CarcassCreated {
            id,
            source,
            energy_milli,
        } => {
            out.push(TAG_CARCASS_CREATED);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&source.to_le_bytes());
            out.extend_from_slice(&energy_milli.to_le_bytes());
        }
        EventKind::CarcassConsumed {
            id,
            consumer,
            energy_milli,
        } => {
            out.push(TAG_CARCASS_CONSUMED);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&consumer.to_le_bytes());
            out.extend_from_slice(&energy_milli.to_le_bytes());
        }
        EventKind::StructuralMutationRejected {
            child_id,
            operator,
            reason,
        } => {
            out.push(TAG_STRUCTURAL_MUTATION_REJECTED);
            out.extend_from_slice(&child_id.to_le_bytes());
            out.push(operator);
            // The reason's own stable code, not the discriminant: a variant
            // inserted into the middle of `RejectReason` must not silently
            // change what an already-written log means.
            out.push(reason.code());
        }
        // Same payload shape as `ControllerFault`, because it is the same
        // kind of record: an entity and the number of values neutralized on
        // one tick.
        EventKind::PlasticityFault { id, faults } => {
            out.push(TAG_PLASTICITY_FAULT);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&faults.to_le_bytes());
        }
        // Phase 12. Every enum-valued field is written as its permanent id
        // (`ObjectAction::id`, `RefuseReason::id`, `DestroyCause::id`,
        // `CAUSE_*`), never a discriminant, and the decoder refuses an id
        // this build does not know.
        EventKind::ObjectCreated {
            id,
            material_id,
            cause,
            mass_milli,
            energy_milli,
            parent_id,
        } => {
            out.push(TAG_OBJECT_CREATED);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&material_id.to_le_bytes());
            out.push(cause);
            out.extend_from_slice(&mass_milli.to_le_bytes());
            out.extend_from_slice(&energy_milli.to_le_bytes());
            out.extend_from_slice(&parent_id.to_le_bytes());
        }
        EventKind::ObjectDestroyed { id, cause } => {
            out.push(TAG_OBJECT_DESTROYED);
            out.extend_from_slice(&id.to_le_bytes());
            out.push(cause);
        }
        EventKind::ObjectPickedUp { id, holder, cell } => {
            out.push(TAG_OBJECT_PICKED_UP);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&holder.to_le_bytes());
            out.extend_from_slice(&cell.to_le_bytes());
        }
        EventKind::ObjectReleased {
            id,
            holder,
            placed,
            cell,
        } => {
            out.push(TAG_OBJECT_RELEASED);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&holder.to_le_bytes());
            out.push(u8::from(placed));
            out.extend_from_slice(&cell.to_le_bytes());
        }
        EventKind::ObjectStruck {
            striker,
            target,
            force_q16,
        } => {
            out.push(TAG_OBJECT_STRUCK);
            out.extend_from_slice(&striker.to_le_bytes());
            out.extend_from_slice(&target.to_le_bytes());
            out.extend_from_slice(&force_q16.to_le_bytes());
        }
        EventKind::TerrainStruck {
            striker,
            cell,
            volume_milli,
            material_id,
        } => {
            out.push(TAG_TERRAIN_STRUCK);
            out.extend_from_slice(&striker.to_le_bytes());
            out.extend_from_slice(&cell.to_le_bytes());
            out.extend_from_slice(&volume_milli.to_le_bytes());
            out.extend_from_slice(&material_id.to_le_bytes());
        }
        EventKind::ObjectCombined {
            composite,
            held,
            target,
            combiner,
            depth,
            joint_q16,
        } => {
            out.push(TAG_OBJECT_COMBINED);
            out.extend_from_slice(&composite.to_le_bytes());
            out.extend_from_slice(&held.to_le_bytes());
            out.extend_from_slice(&target.to_le_bytes());
            out.extend_from_slice(&combiner.to_le_bytes());
            out.push(depth);
            out.extend_from_slice(&joint_q16.to_le_bytes());
        }
        EventKind::ObjectConsumed {
            id,
            consumer,
            energy_milli,
        } => {
            out.push(TAG_OBJECT_CONSUMED);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&consumer.to_le_bytes());
            out.extend_from_slice(&energy_milli.to_le_bytes());
        }
        EventKind::ObjectActionRefused { id, action, reason } => {
            out.push(TAG_OBJECT_ACTION_REFUSED);
            out.extend_from_slice(&id.to_le_bytes());
            out.push(action);
            out.push(reason);
        }
        EventKind::ObjectExposure {
            id,
            exposure_ticks,
            carry_ticks,
            age_ticks,
            birth_band,
        } => {
            out.push(TAG_OBJECT_EXPOSURE);
            out.extend_from_slice(&id.to_le_bytes());
            out.extend_from_slice(&exposure_ticks.to_le_bytes());
            out.extend_from_slice(&carry_ticks.to_le_bytes());
            out.extend_from_slice(&age_ticks.to_le_bytes());
            out.push(birth_band);
        }
    }
}

/// Encode one tick's segment. `dropped` is the number of events the kernel's
/// bounded buffer discarded during this tick.
pub fn encode_segment(tick: u64, events: &[Event], dropped: u32) -> Result<Vec<u8>, EventLogError> {
    if events.len() > MAX_EVENTS_PER_TICK {
        return Err(EventLogError::SegmentCountTooLarge(events.len() as u32));
    }
    let mut body = Vec::with_capacity(events.len() * 24);
    for event in events {
        encode_event(&mut body, &event.kind);
    }
    if body.len() as u64 > MAX_SEGMENT_BODY_LEN {
        return Err(EventLogError::SegmentBodyTooLarge(body.len() as u64));
    }
    let mut out = Vec::with_capacity(SEGMENT_HEADER_LEN + body.len() + 4);
    out.extend_from_slice(SEGMENT_MAGIC);
    out.extend_from_slice(&tick.to_le_bytes());
    out.extend_from_slice(&(events.len() as u32).to_le_bytes());
    out.extend_from_slice(&dropped.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    debug_assert_eq!(out.len(), SEGMENT_HEADER_LEN);
    out.extend_from_slice(&body);
    let checksum = crate::codec::crc32(&out);
    out.extend_from_slice(&checksum.to_le_bytes());
    Ok(out)
}

// --- Decoding ---------------------------------------------------------------

struct Cursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, count: usize) -> Option<&'a [u8]> {
        let slice = self
            .bytes
            .get(self.offset..self.offset.checked_add(count)?)?;
        self.offset += count;
        Some(slice)
    }
    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn i64(&mut self) -> Option<i64> {
        Some(i64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
}

/// Parse and validate the header only.
pub fn read_log_info(bytes: &[u8]) -> Result<(EventLogInfo, usize), EventLogError> {
    if bytes.len() < EVENT_LOG_HEADER_LEN {
        return Err(EventLogError::TooShort);
    }
    if &bytes[0..4] != EVENT_LOG_MAGIC {
        return Err(EventLogError::BadMagic);
    }
    let mut cursor = Cursor {
        bytes: &bytes[4..EVENT_LOG_HEADER_LEN],
        offset: 0,
    };
    /// The header slice is length-checked above, so a `None` here means the
    /// fixed layout itself is wrong rather than the input being short.
    fn read<T>(value: Option<T>) -> Result<T, EventLogError> {
        value.ok_or(EventLogError::TooShort)
    }
    let format_version = read(cursor.u16())?;
    if format_version != EVENT_LOG_FORMAT_VERSION {
        return Err(EventLogError::UnsupportedFormat(format_version));
    }
    let header_len = usize::from(read(cursor.u16())?);
    if header_len != EVENT_LOG_HEADER_LEN {
        return Err(EventLogError::BadHeaderLength(header_len));
    }
    let flags = read(cursor.u32())?;
    if flags != 0 {
        return Err(EventLogError::UnknownFlags(flags));
    }
    let world_id = read(cursor.u64())?;
    let seed = read(cursor.u64())?;
    let config_hash = read(cursor.u64())?;
    let event_schema_version = read(cursor.u32())?;
    if event_schema_version != EVENT_SCHEMA_VERSION {
        return Err(EventLogError::UnsupportedEventSchema(event_schema_version));
    }
    let max_events_per_tick = read(cursor.u32())?;
    if max_events_per_tick == 0 || max_events_per_tick as usize > MAX_EVENTS_PER_TICK {
        return Err(EventLogError::SegmentCountTooLarge(max_events_per_tick));
    }
    let build_len = usize::from(read(cursor.u16())?);
    let _reserved = read(cursor.u16())?;
    if build_len > MAX_BUILD_LEN {
        return Err(EventLogError::BuildStringTooLong(build_len));
    }
    let start_tick = read(cursor.u64())?;
    let _reserved = read(cursor.u32())?;
    let recorded_crc = read(cursor.u32())?;

    let build_end = EVENT_LOG_HEADER_LEN + build_len;
    let build_bytes = bytes
        .get(EVENT_LOG_HEADER_LEN..build_end)
        .ok_or(EventLogError::TooShort)?;
    // Verify before trusting any provenance field above.
    let mut covered = bytes[..HEADER_CRC_OFFSET].to_vec();
    covered.extend_from_slice(build_bytes);
    if crate::codec::crc32(&covered) != recorded_crc {
        return Err(EventLogError::HeaderChecksumMismatch);
    }
    let build_version = std::str::from_utf8(build_bytes)
        .map_err(|_| EventLogError::BadBuildString)?
        .to_owned();
    Ok((
        EventLogInfo {
            format_version,
            world_id,
            seed,
            config_hash,
            event_schema_version,
            max_events_per_tick,
            start_tick,
            build_version,
        },
        build_end,
    ))
}

fn decode_events_into(
    body: &[u8],
    count: u32,
    tick: u64,
    counters: &mut ReconstructedCounters,
    collect: Option<&mut Vec<Event>>,
) -> Result<(), EventLogError> {
    let mut cursor = Cursor {
        bytes: body,
        offset: 0,
    };
    let mut sink = collect;
    /// A body that runs out mid-event is a length mismatch, never a panic.
    fn short<T>(value: Option<T>, tick: u64) -> Result<T, EventLogError> {
        value.ok_or(EventLogError::SegmentBodyLengthMismatch { tick })
    }
    macro_rules! short {
        ($value:expr) => {
            short($value, tick)?
        };
    }
    for _ in 0..count {
        let tag = short!(cursor.u8());
        let kind = match tag {
            TAG_BIRTH => EventKind::Birth {
                id: short!(cursor.u64()),
                parent_id: short!(cursor.u64()),
            },
            TAG_DEATH => {
                let id = short!(cursor.u64());
                let cause = match short!(cursor.u8()) {
                    0 => DeathCause::Starvation,
                    1 => DeathCause::OldAge,
                    2 => DeathCause::Damage,
                    3 => DeathCause::Senescence,
                    4 => DeathCause::Extrinsic,
                    _ => return Err(EventLogError::ValueOutOfRange("death cause")),
                };
                EventKind::Death { id, cause }
            }
            TAG_CAPACITY_REJECTED => EventKind::CapacityRejected {
                parent_id: short!(cursor.u64()),
            },
            TAG_EXTINCTION => EventKind::Extinction,
            TAG_PAIRED_BIRTH => EventKind::PairedBirth {
                id: short!(cursor.u64()),
                parent_a: short!(cursor.u64()),
                parent_b: short!(cursor.u64()),
                genome_hash: short!(cursor.u64()),
                invest_a_milli: short!(cursor.i64()),
                invest_b_milli: short!(cursor.i64()),
                mutated_trait_genes: short!(cursor.u32()),
                mutated_neural_genes: short!(cursor.u32()),
            },
            TAG_PAIR_REJECTED => {
                let parent_a = short!(cursor.u64());
                let parent_b = short!(cursor.u64());
                let reason = match short!(cursor.u8()) {
                    0 => PairRejectReason::Capacity,
                    1 => PairRejectReason::Placement,
                    2 => PairRejectReason::Energy,
                    3 => PairRejectReason::Nonviable,
                    _ => return Err(EventLogError::ValueOutOfRange("pair reject reason")),
                };
                EventKind::PairRejected {
                    parent_a,
                    parent_b,
                    reason,
                }
            }
            TAG_CONTROLLER_FAULT => EventKind::ControllerFault {
                id: short!(cursor.u64()),
                faults: short!(cursor.u32()),
            },
            TAG_PLASTICITY_FAULT => EventKind::PlasticityFault {
                id: short!(cursor.u64()),
                faults: short!(cursor.u32()),
            },
            TAG_DAMAGE => EventKind::Damage {
                attacker: short!(cursor.u64()),
                target: short!(cursor.u64()),
                raw_milli: short!(cursor.i64()),
                applied_milli: short!(cursor.i64()),
                health_milli: short!(cursor.i64()),
            },
            TAG_DEATH_BY_DAMAGE => EventKind::DeathByDamage {
                id: short!(cursor.u64()),
                attacker: short!(cursor.u64()),
            },
            TAG_CARCASS_CREATED => EventKind::CarcassCreated {
                id: short!(cursor.u64()),
                source: short!(cursor.u64()),
                energy_milli: short!(cursor.i64()),
            },
            TAG_CARCASS_CONSUMED => EventKind::CarcassConsumed {
                id: short!(cursor.u64()),
                consumer: short!(cursor.u64()),
                energy_milli: short!(cursor.i64()),
            },
            TAG_STRUCTURAL_MUTATION_REJECTED => {
                let child_id = short!(cursor.u64());
                let operator = short!(cursor.u8());
                let code = short!(cursor.u8());
                // Fail closed on both fields. An operator code the build
                // does not know is a log from a different lineage, not a
                // record to guess at.
                if !(OP_POINT..=OP_TRANSPOSITION).contains(&operator) {
                    return Err(EventLogError::ValueOutOfRange("structural operator"));
                }
                let Some(reason) = RejectReason::from_code(code) else {
                    return Err(EventLogError::ValueOutOfRange("structural reject reason"));
                };
                EventKind::StructuralMutationRejected {
                    child_id,
                    operator,
                    reason,
                }
            }
            TAG_OBJECT_CREATED => {
                let id = short!(cursor.u64());
                let material_id = short!(cursor.u16());
                let cause = short!(cursor.u8());
                let mass_milli = short!(cursor.i64());
                let energy_milli = short!(cursor.i64());
                let parent_id = short!(cursor.u64());
                if !sim_core::material_exists(material_id) {
                    return Err(EventLogError::ValueOutOfRange("object material"));
                }
                if !sim_core::cause_is_known(cause) {
                    return Err(EventLogError::ValueOutOfRange("object cause"));
                }
                EventKind::ObjectCreated {
                    id,
                    material_id,
                    cause,
                    mass_milli,
                    energy_milli,
                    parent_id,
                }
            }
            TAG_OBJECT_DESTROYED => {
                let id = short!(cursor.u64());
                let cause = short!(cursor.u8());
                if sim_core::DestroyCause::from_id(cause).is_none() {
                    return Err(EventLogError::ValueOutOfRange("object destroy cause"));
                }
                EventKind::ObjectDestroyed { id, cause }
            }
            TAG_OBJECT_PICKED_UP => EventKind::ObjectPickedUp {
                id: short!(cursor.u64()),
                holder: short!(cursor.u64()),
                cell: short!(cursor.u32()),
            },
            TAG_OBJECT_RELEASED => {
                let id = short!(cursor.u64());
                let holder = short!(cursor.u64());
                let placed = match short!(cursor.u8()) {
                    0 => false,
                    1 => true,
                    _ => return Err(EventLogError::ValueOutOfRange("object placed flag")),
                };
                let cell = short!(cursor.u32());
                EventKind::ObjectReleased {
                    id,
                    holder,
                    placed,
                    cell,
                }
            }
            TAG_OBJECT_STRUCK => EventKind::ObjectStruck {
                striker: short!(cursor.u64()),
                target: short!(cursor.u64()),
                force_q16: short!(cursor.u32()),
            },
            TAG_TERRAIN_STRUCK => {
                let striker = short!(cursor.u64());
                let cell = short!(cursor.u32());
                let volume_milli = short!(cursor.i64());
                let material_id = short!(cursor.u16());
                if !sim_core::material_exists(material_id) {
                    return Err(EventLogError::ValueOutOfRange("terrain material"));
                }
                EventKind::TerrainStruck {
                    striker,
                    cell,
                    volume_milli,
                    material_id,
                }
            }
            TAG_OBJECT_COMBINED => EventKind::ObjectCombined {
                composite: short!(cursor.u64()),
                held: short!(cursor.u64()),
                target: short!(cursor.u64()),
                combiner: short!(cursor.u64()),
                depth: short!(cursor.u8()),
                joint_q16: short!(cursor.u32()),
            },
            TAG_OBJECT_CONSUMED => EventKind::ObjectConsumed {
                id: short!(cursor.u64()),
                consumer: short!(cursor.u64()),
                energy_milli: short!(cursor.i64()),
            },
            TAG_OBJECT_ACTION_REFUSED => {
                let id = short!(cursor.u64());
                let action = short!(cursor.u8());
                let reason = short!(cursor.u8());
                if sim_core::ObjectAction::from_id(action).is_none() {
                    return Err(EventLogError::ValueOutOfRange("object action"));
                }
                if sim_core::RefuseReason::from_id(reason).is_none() {
                    return Err(EventLogError::ValueOutOfRange("object refuse reason"));
                }
                EventKind::ObjectActionRefused { id, action, reason }
            }
            TAG_OBJECT_EXPOSURE => {
                let id = short!(cursor.u64());
                let exposure_ticks = short!(cursor.u64());
                let carry_ticks = short!(cursor.u64());
                let age_ticks = short!(cursor.u64());
                let birth_band = short!(cursor.u8());
                if birth_band > 4 {
                    return Err(EventLogError::ValueOutOfRange("birth band"));
                }
                EventKind::ObjectExposure {
                    id,
                    exposure_ticks,
                    carry_ticks,
                    age_ticks,
                    birth_band,
                }
            }
            // Fail closed. Skipping would corrupt every rate an analysis
            // computes, so an unknown type is never tolerated.
            other => return Err(EventLogError::UnknownEventType { tick, tag: other }),
        };
        counters.observe(&kind);
        if let Some(events) = sink.as_deref_mut() {
            events.push(Event { tick, kind });
        }
    }
    if cursor.offset != body.len() {
        return Err(EventLogError::SegmentBodyLengthMismatch { tick });
    }
    Ok(())
}

/// Replay a whole log strictly: any malformed or truncated byte anywhere is
/// an error, and nothing is returned.
pub fn decode_log(bytes: &[u8]) -> Result<EventLogScan, EventLogError> {
    let (scan, error) = scan_log(bytes, None)?;
    match error {
        Some(error) => Err(error),
        None => Ok(scan),
    }
}

/// Replay a whole log and also materialize every event, in file order.
/// Bounded by the caller's willingness to hold the result in memory; use
/// `decode_log` for counter reconstruction over long runs.
pub fn decode_log_events(bytes: &[u8]) -> Result<(EventLogScan, Vec<Event>), EventLogError> {
    let mut events = Vec::new();
    let (scan, error) = scan_log(bytes, Some(&mut events))?;
    match error {
        Some(error) => Err(error),
        None => Ok((scan, events)),
    }
}

/// Replay the valid prefix of a log and report the typed error that ended
/// it, if any.
///
/// This is a reporting path, not a repair path: it never rewrites the file,
/// never invents a value, and never admits a partially decoded segment. A
/// process killed between `write` and `sync` leaves a torn final segment,
/// and recovery needs to know both how much of the log is trustworthy and
/// exactly why the rest is not.
pub fn decode_log_prefix(
    bytes: &[u8],
) -> Result<(EventLogScan, Option<EventLogError>), EventLogError> {
    scan_log(bytes, None)
}

fn scan_log(
    bytes: &[u8],
    mut collect: Option<&mut Vec<Event>>,
) -> Result<(EventLogScan, Option<EventLogError>), EventLogError> {
    let (info, mut offset) = read_log_info(bytes)?;
    let mut scan = EventLogScan {
        info,
        segments: 0,
        events: 0,
        dropped: 0,
        first_tick: None,
        last_tick: None,
        counters: ReconstructedCounters::default(),
        bytes_consumed: offset,
    };

    loop {
        if offset == bytes.len() {
            return Ok((scan, None));
        }
        let remaining = &bytes[offset..];
        if remaining.len() < SEGMENT_HEADER_LEN + 4 {
            return Ok((scan, Some(EventLogError::TruncatedSegment { offset })));
        }
        if &remaining[0..4] != SEGMENT_MAGIC {
            return Ok((scan, Some(EventLogError::BadSegmentMagic { offset })));
        }
        let mut cursor = Cursor {
            bytes: &remaining[4..SEGMENT_HEADER_LEN],
            offset: 0,
        };
        // The slice length is checked above, so these cannot fail.
        let tick = cursor.u64().ok_or(EventLogError::TooShort)?;
        let count = cursor.u32().ok_or(EventLogError::TooShort)?;
        let dropped = cursor.u32().ok_or(EventLogError::TooShort)?;
        let body_len = cursor.u32().ok_or(EventLogError::TooShort)?;

        // Cap every declared length before it reaches an allocation or a
        // slice index.
        if count as usize > MAX_EVENTS_PER_TICK || count > scan.info.max_events_per_tick {
            return Ok((scan, Some(EventLogError::SegmentCountTooLarge(count))));
        }
        if u64::from(body_len) > MAX_SEGMENT_BODY_LEN {
            return Ok((
                scan,
                Some(EventLogError::SegmentBodyTooLarge(u64::from(body_len))),
            ));
        }
        let total = SEGMENT_HEADER_LEN + body_len as usize + 4;
        if remaining.len() < total {
            return Ok((scan, Some(EventLogError::TruncatedSegment { offset })));
        }
        let framed = &remaining[..SEGMENT_HEADER_LEN + body_len as usize];
        let recorded = u32::from_le_bytes(
            remaining[SEGMENT_HEADER_LEN + body_len as usize..total]
                .try_into()
                .map_err(|_| EventLogError::TruncatedSegment { offset })?,
        );
        if crate::codec::crc32(framed) != recorded {
            return Ok((scan, Some(EventLogError::SegmentChecksumMismatch { tick })));
        }
        if let Some(previous) = scan.last_tick
            && tick <= previous
        {
            return Ok((
                scan,
                Some(EventLogError::TickOutOfOrder {
                    previous,
                    found: tick,
                }),
            ));
        }

        // The frame is intact; only now is the body parsed. Counters are
        // accumulated into a copy so a mid-body failure cannot leave the
        // scan holding half a segment.
        let mut counters = scan.counters;
        let mut staged = collect.as_deref_mut().map(|_| Vec::new());
        let body = &framed[SEGMENT_HEADER_LEN..];
        if let Err(error) = decode_events_into(body, count, tick, &mut counters, staged.as_mut()) {
            return Ok((scan, Some(error)));
        }
        if let (Some(sink), Some(staged)) = (collect.as_deref_mut(), staged) {
            sink.extend(staged);
        }
        scan.counters = counters;
        scan.segments += 1;
        scan.events += u64::from(count);
        scan.dropped += u64::from(dropped);
        scan.first_tick.get_or_insert(tick);
        scan.last_tick = Some(tick);
        offset += total;
        scan.bytes_consumed = offset;
    }
}

// --- Writer -----------------------------------------------------------------

/// Append-only writer. Buffers whole segments and never emits a partial one
/// to the buffer, so the only torn frame possible is one interrupted by a
/// process death mid-`write`, which `decode_log_prefix` reports.
pub struct EventLogWriter {
    file: BufWriter<File>,
    offset: u64,
    segments: u64,
    events: u64,
    dropped: u64,
}

impl EventLogWriter {
    /// Create a new log file, failing if one already exists at `path`. A log
    /// is never reopened for append by this constructor: a new run gets a
    /// new file, so a stale log can never be silently extended.
    pub fn create(path: &Path, info: &EventLogInfo) -> Result<Self, EventLogError> {
        let header = encode_header(info)?;
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| EventLogError::Io(error.to_string()))?;
        let mut writer = BufWriter::new(file);
        writer
            .write_all(&header)
            .map_err(|error| EventLogError::Io(error.to_string()))?;
        Ok(Self {
            file: writer,
            offset: header.len() as u64,
            segments: 0,
            events: 0,
            dropped: 0,
        })
    }

    /// Byte length of the log so far. This is what a snapshot records as its
    /// `event_log_offset`, so a restored world knows exactly where its
    /// history stops.
    pub fn offset(&self) -> u64 {
        self.offset
    }

    pub fn segments(&self) -> u64 {
        self.segments
    }

    pub fn events(&self) -> u64 {
        self.events
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Append one tick's events. A tick with no events and no drops writes
    /// nothing, so an idle world costs no bytes.
    pub fn append(
        &mut self,
        tick: u64,
        events: &[Event],
        dropped: u32,
    ) -> Result<(), EventLogError> {
        if events.is_empty() && dropped == 0 {
            return Ok(());
        }
        let segment = encode_segment(tick, events, dropped)?;
        self.file
            .write_all(&segment)
            .map_err(|error| EventLogError::Io(error.to_string()))?;
        self.offset += segment.len() as u64;
        self.segments += 1;
        self.events += events.len() as u64;
        self.dropped += u64::from(dropped);
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), EventLogError> {
        self.file
            .flush()
            .map_err(|error| EventLogError::Io(error.to_string()))
    }

    /// Flush and fsync. Uses the same durability ordering as the snapshot
    /// writer: bytes reach the device before anything records that they
    /// exist.
    pub fn sync(&mut self) -> Result<(), EventLogError> {
        self.flush()?;
        self.file
            .get_ref()
            .sync_all()
            .map_err(|error| EventLogError::Io(error.to_string()))
    }
}

/// Convenience recorder that turns the kernel's cumulative drop counter into
/// the per-tick drop count the segment format carries.
pub struct EventLogRecorder {
    writer: EventLogWriter,
    last_dropped_total: u64,
}

impl EventLogRecorder {
    pub fn new(writer: EventLogWriter) -> Self {
        Self {
            writer,
            last_dropped_total: 0,
        }
    }

    /// Record the events the world produced during its most recent tick.
    /// Call once immediately after each `step`, before the next one clears
    /// the buffer.
    pub fn record(&mut self, world: &World) -> Result<(), EventLogError> {
        let total = world.counters().dropped_events_total;
        let dropped = total.saturating_sub(self.last_dropped_total);
        self.last_dropped_total = total;
        let dropped = u32::try_from(dropped).unwrap_or(u32::MAX);
        self.writer
            .append(world.tick_number(), world.events(), dropped)
    }

    pub fn writer(&self) -> &EventLogWriter {
        &self.writer
    }

    pub fn writer_mut(&mut self) -> &mut EventLogWriter {
        &mut self.writer
    }

    pub fn into_writer(self) -> EventLogWriter {
        self.writer
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sim_core::OP_DUPLICATION;

    fn info() -> EventLogInfo {
        EventLogInfo {
            format_version: EVENT_LOG_FORMAT_VERSION,
            world_id: 7,
            seed: 0x5eed,
            config_hash: 0xabcd,
            event_schema_version: EVENT_SCHEMA_VERSION,
            max_events_per_tick: MAX_EVENTS_PER_TICK as u32,
            start_tick: 0,
            build_version: "test-build".to_owned(),
        }
    }

    fn sample_events(tick: u64) -> Vec<Event> {
        vec![
            Event {
                tick,
                kind: EventKind::Birth {
                    id: 5,
                    parent_id: 2,
                },
            },
            Event {
                tick,
                kind: EventKind::Death {
                    id: 3,
                    cause: DeathCause::OldAge,
                },
            },
            Event {
                tick,
                kind: EventKind::PairedBirth {
                    id: 9,
                    parent_a: 1,
                    parent_b: 2,
                    genome_hash: 0xfeed,
                    invest_a_milli: 100,
                    invest_b_milli: -50,
                    mutated_trait_genes: 3,
                    mutated_neural_genes: 11,
                },
            },
            Event {
                tick,
                kind: EventKind::PairRejected {
                    parent_a: 4,
                    parent_b: 6,
                    reason: PairRejectReason::Energy,
                },
            },
            Event {
                tick,
                kind: EventKind::ControllerFault { id: 8, faults: 2 },
            },
            Event {
                tick,
                kind: EventKind::CapacityRejected { parent_id: 4 },
            },
            Event {
                tick,
                kind: EventKind::Extinction,
            },
            // Phase 9 C9.6. Two of them, with *different* reasons, so the
            // per-class reconstruction is exercised rather than only the
            // total - a single sample would let two classes be swapped and
            // still round-trip.
            Event {
                tick,
                kind: EventKind::StructuralMutationRejected {
                    child_id: 21,
                    operator: OP_DUPLICATION,
                    reason: RejectReason::Cap,
                },
            },
            Event {
                tick,
                kind: EventKind::StructuralMutationRejected {
                    child_id: 22,
                    operator: OP_TRANSPOSITION,
                    reason: RejectReason::Inapplicable,
                },
            },
        ]
    }

    fn build_log() -> Vec<u8> {
        let mut bytes = encode_header(&info()).unwrap();
        for tick in 1..=3_u64 {
            bytes.extend(encode_segment(tick, &sample_events(tick), 0).unwrap());
        }
        bytes
    }

    #[test]
    fn round_trip_preserves_every_event_and_counter() {
        let bytes = build_log();
        let (scan, events) = decode_log_events(&bytes).unwrap();
        assert_eq!(scan.segments, 3);
        assert_eq!(scan.events, 27);
        assert_eq!(scan.first_tick, Some(1));
        assert_eq!(scan.last_tick, Some(3));
        assert_eq!(scan.bytes_consumed, bytes.len());
        assert_eq!(scan.info.build_version, "test-build");
        // Birth + PairedBirth both advance births_total.
        assert_eq!(scan.counters.births_total, 6);
        assert_eq!(scan.counters.paired_births_total, 3);
        assert_eq!(scan.counters.deaths_old_age_total, 3);
        assert_eq!(scan.counters.deaths_starvation_total, 0);
        assert_eq!(scan.counters.capacity_rejections_total, 3);
        assert_eq!(scan.counters.extinctions_total, 3);
        assert_eq!(scan.counters.pair_rejected_energy_total, 3);
        assert_eq!(scan.counters.controller_faults_total, 6);
        assert_eq!(scan.counters.mutated_trait_genes_total, 9);
        assert_eq!(scan.counters.mutated_neural_genes_total, 33);
        // C9.6: reconstructed per class, not only in total. Three ticks of
        // one `Cap` and one `Inapplicable` each.
        assert_eq!(scan.counters.structural_rejections_total, 6);
        assert_eq!(
            scan.counters.structural_rejections_by_reason
                [usize::from(RejectReason::Cap.code() - 1)],
            3
        );
        assert_eq!(
            scan.counters.structural_rejections_by_reason
                [usize::from(RejectReason::Inapplicable.code() - 1)],
            3
        );
        assert_eq!(
            scan.counters.structural_rejections_by_reason
                [usize::from(RejectReason::Invalid.code() - 1)],
            0,
            "a class that never occurred must stay zero, or the indexing is wrong"
        );

        let expected: Vec<Event> = (1..=3_u64).flat_map(sample_events).collect();
        assert_eq!(events, expected);
    }

    #[test]
    fn empty_log_is_valid_and_carries_no_events() {
        let bytes = encode_header(&info()).unwrap();
        let scan = decode_log(&bytes).unwrap();
        assert_eq!(scan.segments, 0);
        assert_eq!(scan.events, 0);
        assert_eq!(scan.first_tick, None);
    }

    #[test]
    fn header_rejections_are_typed() {
        let valid = build_log();
        assert_eq!(
            read_log_info(&valid[..10]).unwrap_err(),
            EventLogError::TooShort
        );

        let mut bad = valid.clone();
        bad[0] = b'X';
        assert_eq!(read_log_info(&bad).unwrap_err(), EventLogError::BadMagic);

        let mut bad = valid.clone();
        bad[4..6].copy_from_slice(&99_u16.to_le_bytes());
        assert_eq!(
            read_log_info(&bad).unwrap_err(),
            EventLogError::UnsupportedFormat(99)
        );

        let mut bad = valid.clone();
        bad[36..40].copy_from_slice(&99_u32.to_le_bytes());
        assert_eq!(
            read_log_info(&bad).unwrap_err(),
            EventLogError::UnsupportedEventSchema(99)
        );

        let mut bad = valid.clone();
        bad[8..12].copy_from_slice(&1_u32.to_le_bytes());
        assert_eq!(
            read_log_info(&bad).unwrap_err(),
            EventLogError::UnknownFlags(1)
        );
    }

    #[test]
    fn a_corrupted_provenance_field_is_caught_by_the_header_checksum() {
        // Seed, world id, config hash, and the build string are provenance,
        // not framing: nothing downstream would notice a flipped bit in
        // them, so the header's own CRC has to.
        let valid = build_log();
        for offset in [12, 20, 28, 48] {
            let mut bad = valid.clone();
            bad[offset] ^= 0x01;
            assert_eq!(
                read_log_info(&bad).unwrap_err(),
                EventLogError::HeaderChecksumMismatch,
                "corruption at byte {offset} was accepted"
            );
        }
        // The build string is covered too, even though it sits after the
        // fixed header.
        let mut bad = valid.clone();
        bad[EVENT_LOG_HEADER_LEN] ^= 0x01;
        assert_eq!(
            read_log_info(&bad).unwrap_err(),
            EventLogError::HeaderChecksumMismatch
        );
    }

    #[test]
    fn unknown_event_type_fails_closed_rather_than_skipping() {
        let mut bytes = encode_header(&info()).unwrap();
        let mut body = Vec::new();
        body.push(200_u8); // never-allocated tag
        body.extend_from_slice(&0_u64.to_le_bytes());
        let mut segment = Vec::new();
        segment.extend_from_slice(SEGMENT_MAGIC);
        segment.extend_from_slice(&1_u64.to_le_bytes());
        segment.extend_from_slice(&1_u32.to_le_bytes());
        segment.extend_from_slice(&0_u32.to_le_bytes());
        segment.extend_from_slice(&(body.len() as u32).to_le_bytes());
        segment.extend_from_slice(&body);
        let checksum = crate::codec::crc32(&segment);
        segment.extend_from_slice(&checksum.to_le_bytes());
        bytes.extend(segment);

        assert_eq!(
            decode_log(&bytes).unwrap_err(),
            EventLogError::UnknownEventType { tick: 1, tag: 200 }
        );
    }

    #[test]
    fn oversized_declared_lengths_are_rejected_before_allocation() {
        let mut bytes = build_log();
        let header_len = EVENT_LOG_HEADER_LEN + "test-build".len();
        // Declare a segment body far larger than the cap.
        bytes[header_len + 20..header_len + 24].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode_log(&bytes),
            Err(EventLogError::SegmentBodyTooLarge(_))
        ));

        let mut bytes = build_log();
        bytes[header_len + 12..header_len + 16].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(matches!(
            decode_log(&bytes),
            Err(EventLogError::SegmentCountTooLarge(_))
        ));
    }

    #[test]
    fn torn_tail_reports_the_valid_prefix_without_repairing_it() {
        let bytes = build_log();
        let truncated = &bytes[..bytes.len() - 3];
        assert!(matches!(
            decode_log(truncated),
            Err(EventLogError::TruncatedSegment { .. })
        ));
        let (scan, error) = decode_log_prefix(truncated).unwrap();
        // The first two segments are intact and readable.
        assert_eq!(scan.segments, 2);
        assert_eq!(scan.last_tick, Some(2));
        assert!(matches!(
            error,
            Some(EventLogError::TruncatedSegment { .. })
        ));
    }

    #[test]
    fn out_of_order_ticks_are_rejected() {
        let mut bytes = encode_header(&info()).unwrap();
        bytes.extend(encode_segment(5, &sample_events(5), 0).unwrap());
        bytes.extend(encode_segment(2, &sample_events(2), 0).unwrap());
        assert_eq!(
            decode_log(&bytes).unwrap_err(),
            EventLogError::TickOutOfOrder {
                previous: 5,
                found: 2
            }
        );
    }

    #[test]
    fn reconcile_is_exact_without_drops_and_bounded_with_them() {
        let bytes = build_log();
        let scan = decode_log(&bytes).unwrap();
        let counters = Counters {
            births_total: 6,
            deaths_starvation_total: 0,
            deaths_old_age_total: 3,
            capacity_rejections_total: 3,
            dropped_events_total: 0,
        };
        let phase2 = Phase2Counters {
            paired_births_total: 3,
            pair_rejected_capacity_total: 0,
            pair_rejected_placement_total: 0,
            pair_rejected_energy_total: 3,
            pair_rejected_nonviable_total: 0,
            controller_faults_total: 6,
            mutated_trait_genes_total: 9,
            mutated_neural_genes_total: 33,
        };
        scan.reconcile(&counters, Some(&phase2)).unwrap();

        // One counter off with no drops recorded is an error.
        let mut wrong = counters;
        wrong.births_total = 7;
        assert!(matches!(
            scan.reconcile(&wrong, Some(&phase2)),
            Err(ReconcileError::CounterMismatch {
                field: "births_total",
                ..
            })
        ));

        // The drop total the log recorded must match the kernel's.
        let mut dropped = counters;
        dropped.dropped_events_total = 4;
        assert!(matches!(
            scan.reconcile(&dropped, Some(&phase2)),
            Err(ReconcileError::DropCountMismatch {
                recorded: 0,
                world: 4
            })
        ));
    }

    #[test]
    fn reconcile_tolerates_the_gap_a_recorded_drop_explains() {
        // A log whose segments record drops: counters may legitimately fall
        // short of the world's, but never exceed them.
        let mut bytes = encode_header(&info()).unwrap();
        bytes.extend(encode_segment(1, &sample_events(1), 2).unwrap());
        let scan = decode_log(&bytes).unwrap();
        assert_eq!(scan.dropped, 2);

        let counters = Counters {
            births_total: 4, // two more births than the log holds
            deaths_starvation_total: 0,
            deaths_old_age_total: 1,
            capacity_rejections_total: 1,
            dropped_events_total: 2,
        };
        let phase2 = Phase2Counters {
            paired_births_total: 1,
            pair_rejected_capacity_total: 0,
            pair_rejected_placement_total: 0,
            pair_rejected_energy_total: 1,
            pair_rejected_nonviable_total: 0,
            controller_faults_total: 2,
            mutated_trait_genes_total: 3,
            mutated_neural_genes_total: 11,
        };
        scan.reconcile(&counters, Some(&phase2)).unwrap();

        // Reconstructing more than the world counted is always an error.
        let mut fewer = counters;
        fewer.births_total = 1;
        assert!(matches!(
            scan.reconcile(&fewer, Some(&phase2)),
            Err(ReconcileError::ReconstructionExceedsWorld { .. })
        ));
    }

    #[test]
    fn a_phase1_world_may_not_carry_phase2_events() {
        let bytes = build_log();
        let scan = decode_log(&bytes).unwrap();
        let counters = Counters {
            births_total: 6,
            deaths_starvation_total: 0,
            deaths_old_age_total: 3,
            capacity_rejections_total: 3,
            dropped_events_total: 0,
        };
        assert_eq!(
            scan.reconcile(&counters, None),
            Err(ReconcileError::Phase2SectionMismatch)
        );
    }
}
