//! ALIF world snapshot format, version 6.
//!
//! Layout (all little-endian, matching the kernel's canonical hashing):
//!
//! header (fixed 112 bytes):
//!   magic "ALIF" | format u16 | header_len u16 | flags u32 (bit0 zstd)
//!   world_id u64 | parent_world_id u64 | tick u64 | seed u64
//!   config_hash u64 | save_state_version u16 | genome_schema u16
//!   build_len u16 | reserved u16 | event_log_offset u64
//!   uncompressed_len u64 | stored_len u64 | payload_crc32 u32
//!   state_checksum u64 | terrain_checksum u64
//! then: build version string (build_len <= 64 bytes)
//! then: payload (zstd-compressed when flagged), which is a sequence of
//! sections: tag u16 | flags u16 | length u64 | body | crc32 u32.
//!
//! Every decoder treats input as hostile: lengths are capped before any
//! allocation or decompression, checksums verify before parsing, and
//! unknown versions/sections fail closed with typed errors. Loaders never
//! repair data.
//!
//! # Where the two terrain checksums live, and why they are not together
//!
//! `specifications/mutable-world-state.md` says "both checksums are recorded
//! in the header". The **baseline** one is, exactly where it has always been
//! and with its fail-closed check untouched - that check is the property the
//! whole design exists to preserve. The **composed** one is in
//! `SECTION_WORLD_META` instead, and the deviation is deliberate:
//!
//! - The header is a fixed 112 bytes and `read_info` asserts that length
//!   exactly, so a tenth field means a longer header, and a longer header
//!   means every existing reader's `BadHeaderLength` fires on every new file.
//!   The version bump would cover that, but it would also mean the cheap
//!   header-only provenance read stops being comparable across formats.
//! - The composed check cannot run at header time anyway. It is meaningful
//!   only after the modification section has been decoded and applied, which
//!   is step 5 of the restore order; `SECTION_WORLD_META` is where the rest of
//!   the state the restore order needs already lives.
//!
//! The word is written **only when the snapshot carries a modification
//! section**, so a world with the section disabled produces a payload that is
//! byte-identical to the one format 3 wrote for it. That is not a
//! micro-optimisation: it is what makes the format 3 to format 4 migration
//! testable against a real legacy file, and what lets C12.8's "a disabled
//! world encodes as it always did" be asserted rather than argued.

use sim_core::{
    GENOME_SCHEMA_VERSION, LAYER_COUNT, Ledger, Phase2SaveState, SAVE_STATE_VERSION, SaveState,
    TRAIT_COUNT, TerrainModCounters, TerrainModState,
};
use std::fmt;

pub const SNAPSHOT_MAGIC: &[u8; 4] = b"ALIF";
/// Format 5 reserves one byte in the config block for
/// `plasticity.live_rule_zero`.
///
/// **The smallest format change there has been, and the first one forced by
/// the config block rather than by a section.** Every optional *section*
/// added since format 3 - learn, morphology, worldmod, action census - is
/// absent from a world that does not have it, so a build that predates one
/// reads a file that lacks it unchanged. The config block has no such
/// property: `encode_config` is positional and unconditional, so one new
/// field shifts `worldmod` and `probe` by a byte and every existing format-4
/// file decodes as garbage or, more often, as
/// `ValueOutOfRange("section trailing bytes")`.
///
/// That is why this is a version bump and not an append. The 120 format-4
/// campaign artifacts are still read for re-analysis, and the alternative -
/// appending the field and letting old files fail - is the mistake format 3
/// already made silently: Phase 11 grew the config block by seventeen bytes
/// *within* format 3, so the retained format-3 reader can only read format-3
/// files this build's own writer produces. That is survivable there because
/// no pre-Phase-11 format-3 file exists. It would not be survivable here.
///
/// The two registered migrations are **3 to current** and **4 to current**.
/// `encode_snapshot_format4` and `decode_snapshot_format4` stay in the build
/// permanently, on the same grounds as their format-3 counterparts: the
/// acceptance requirement is byte identity against what the format-4 reader
/// produces, and a comparison you have deleted one side of is not a
/// comparison.
pub const FORMAT_VERSION: u16 = FORMAT_VERSION_12;
/// Format 8 appends the Phase 13 social config block (ADR-0029 section 6).
///
/// The fourth config-block bump, block-shaped like format 7's rather than
/// byte-shaped like 5's and 6's, and appended in one piece after format 7's
/// block so the format-7 body stays a byte prefix of the format-8 body;
/// every appended byte is its field's default in a world without the
/// section. The chain test in `format7.rs` gains one row declaring
/// `FORMAT8_CONFIG_BYTES`.
///
/// **Named as its own constant from the day it shipped**, and every guard
/// that introduces something at format 8 says `FORMAT_VERSION_8`, never
/// `FORMAT_VERSION` (D-108's trap, closed structurally).
pub const FORMAT_VERSION_8: u16 = 8;

/// Format 9 (Phase 14, ADR-0030): appends the physiology-v2 ontogeny
/// config fields to the config body and introduces `SECTION_ONTOGENY`.
/// Guarded by name wherever it gates, permanently (D-108).
pub const FORMAT_VERSION_9: u16 = 9;

/// Format 10 (Phase 14, ADR-0030 decision 2): appends the two mate-choice
/// gates to the config body and introduces `SECTION_MATECHOICE`. Guarded
/// by name wherever it gates, permanently (D-108).
pub const FORMAT_VERSION_10: u16 = 10;

/// Format 11 (Phase 15, ADR-0031): appends the chemistry config block and
/// introduces `SECTION_CHEMISTRY`. Guarded by name wherever it gates,
/// permanently (D-108).
pub const FORMAT_VERSION_11: u16 = 11;

/// Format 12 (Phase 15, ADR-0031, increment 2): appends the microbial
/// config fields to the config body and introduces `SECTION_MICROBIAL`.
/// Guarded by name wherever it gates, permanently (D-108).
pub const FORMAT_VERSION_12: u16 = 12;
/// Format 7 appends the Phase 12 artifact config block and
/// `genome2.mutation.binding_q16` to the config block, adds one counter word
/// to the schema-2 section, and introduces `SECTION_OBJECTS` (ADR-0028
/// section 13).
///
/// **The third config-block bump, and the first that adds a block rather
/// than a byte.** Format 5's and format 6's notes both said a third field
/// should expect the same cost; here it is, thirty-odd fields at once, and
/// the chain test in `format7.rs` asserts the byte delta per adjacent pair
/// rather than "exactly one" (D-112 anticipated a non-one-byte extension
/// would need "its own reasoning": the block is one section's config, it is
/// appended in one piece so the format-6 body stays a byte prefix, and every
/// appended byte is the field's default in a default world).
///
/// **Named as its own constant from the day it shipped**, and every guard
/// that introduces something at format 7 says `FORMAT_VERSION_7`, never
/// `FORMAT_VERSION`. D-108 recorded the trap of writing `format <
/// FORMAT_VERSION` for a section that arrived at the then-current format;
/// format 6's own byte was guarded `>= FORMAT_VERSION` and would have been
/// misread the day this constant moved. Naming the introducing format at
/// introduction time closes that trap structurally.
pub const FORMAT_VERSION_7: u16 = 7;
/// Format 6 reserves one config byte for `plasticity.price_moved_edges_only`.
///
/// Retained as a reader and a writer for the reason every earlier format is.
/// `encode_snapshot_format6` refuses a state that carries anything format 7
/// added - the artifact section, a nonzero `binding_q16`, a nonzero
/// `binding_applied`, an object table - with `FieldNotInFormat` or
/// `SectionNotInFormat`, so a format-6 file can never describe a world it has
/// no bytes for.
pub const FORMAT_VERSION_6: u16 = 6;
/// Format 5 reserves one config byte for `plasticity.live_rule_zero`.
///
/// **The second config-block bump in as many increments, and the repetition
/// is the point rather than an accident.** Format 5's note argues that a
/// positional, unconditional config block cannot grow without a version bump.
/// That argument does not weaken with use: `plasticity.price_moved_edges_only`
/// is one more byte, one more format, one more retained reader and writer.
/// Anyone adding a third field should expect the same cost and budget for it,
/// or propose a self-describing config block in an ADR - which would have to
/// explain how an absent trailing field avoids being "altered meaning on
/// load", the rule that makes defaulting one unacceptable.
pub const FORMAT_VERSION_5: u16 = 5;
/// Format 4 stores terrain modifications and the composed terrain checksum.
///
/// Retained as a reader and a writer, not as history. It was the current
/// format for the whole Phase 12 mutable-world half and every campaign
/// artifact on disk is one.
///
/// **It is 4 and not 2, and the difference is not bookkeeping.** Every
/// sentence in `specifications/mutable-world-state.md`, in
/// `specifications/world-save-format.md`, and in ADR-0015 that calls the
/// mutable-world successor "format 2" was written before formats 2 and 3
/// existed; both shipped for reasons that have nothing to do with terrain,
/// and both are documented below. Writing a migration registry against a
/// version number that is already taken is the specific mistake this
/// paragraph exists to have prevented.
///
/// Formats 1 and 2 have no migration, by design and by physics rather than
/// by neglect - see their notes below. `decode_snapshot_format3` and
/// `encode_snapshot_format3` stay in the build permanently, because the
/// acceptance requirement for the migration is byte identity against what the
/// format 3 reader produces, and a comparison you have deleted one side of is
/// not a comparison.
pub const FORMAT_VERSION_4: u16 = 4;
/// Format 3 makes the Phase 2 section describe its own two counts.
///
/// Format 2 wrote one count and drove the per-organism loop from
/// `traits.len()`. That is the organism count in a schema-1 world and zero
/// in a schema-2 world, which carries no flat genome - so a schema-2
/// snapshot encoded no per-organism records at all and dropped heading,
/// speed, turn, parents, depth, child count, birth tick, and memory. It
/// failed closed on restore rather than corrupting silently, but it failed:
/// **a schema-2 world could not be checkpointed.** Format 3 writes the
/// organism count and the flat-genome count separately, so the two are never
/// again assumed equal. No migration from 2, on the same grounds as 1 to 2:
/// a format-2 schema-2 file does not contain the dropped state, and a
/// format-2 schema-1 file would have to be re-framed on a guess.
///
/// Format 2 added the config sections that Phase 6, 7, and 8 introduced.
///
/// Format 1 encoded the config only as far as Phase 2, so every section
/// added afterwards was silently dropped on save and restored at its
/// default on load. For climate and contest the presence check in
/// `World::from_state` turned that into a confusing restore failure; for
/// the origin section, which has no presence check, it would have restored
/// a *different config* with no error at all. There is no migration from 1:
/// a format-1 file cannot say what its climate settings were, so inventing
/// them is exactly the "never alter meaning during load" rule this crate
/// exists to keep.
pub const FORMAT_VERSION_3: u16 = 3;
/// The logical state version format 3 pairs with. Pinned as its own constant
/// rather than read as `SAVE_STATE_VERSION - 1`: the two axes move
/// independently, and format 3 has been paired with logical version 1 for its
/// whole life whatever the current version becomes.
pub const SAVE_STATE_VERSION_3: u16 = 1;
pub const FLAG_ZSTD: u32 = 1;
const HEADER_LEN: usize = 112;
const MAX_BUILD_LEN: usize = 64;
/// Absolute caps applied before allocation/decompression.
pub const MAX_STORED_LEN: u64 = 256 * 1024 * 1024;
pub const MAX_UNCOMPRESSED_LEN: u64 = 1024 * 1024 * 1024;
const MAX_SECTION_LEN: u64 = MAX_UNCOMPRESSED_LEN;

const SECTION_CONFIG: u16 = 1;
const SECTION_WORLD_META: u16 = 2;
const SECTION_ORGANISMS: u16 = 3;
const SECTION_BIOMASS: u16 = 4;
const SECTION_LEDGER: u16 = 5;
const SECTION_PHASE2: u16 = 6;
/// Phase 6 climate. Optional exactly as the Phase 2 section is: present
/// only when the subsystem exists, absent otherwise, so a world without
/// climate encodes byte-identically to the way it always did. Section tags
/// are permanent and never reused.
const SECTION_CLIMATE: u16 = 7;
/// Phase 7 contest. Optional on the same terms as Phase 2 and climate.
const SECTION_CONTEST: u16 = 8;
/// Phase 8 demography. Optional on the same terms. Tags are permanent and
/// never reused, so a Phase 7 snapshot decodes unchanged.
const SECTION_PHYSIOLOGY: u16 = 9;
/// Phase 9 genome schema 2. Optional on the same terms.
const SECTION_SCHEMA2: u16 = 10;
/// Phase 10 morphology. Optional on the same terms, and deliberately tiny:
/// bodies are derived and never stored, so this section carries only the
/// developmental counters.
const SECTION_MORPHOLOGY: u16 = 11;
/// Phase 11 learned state. Optional on the same terms as every section
/// above, and the format version stays 3: an absent optional section is
/// readable by every existing build, so a pre-Phase-11 snapshot decodes
/// unchanged and a snapshot from a world with plasticity disabled is
/// byte-identical to the one that build would have written.
///
/// Sparse: only plastic edges are stored, each naming the edge it belongs to
/// rather than a slot index. See `LearnSaveState`.
const SECTION_LEARN: u16 = 12;
/// Phase 12 terrain modification. Optional on the same terms as every
/// section above - present exactly when `config.worldmod.enabled` - which is
/// what makes a snapshot of a disabled world byte-identical to a format 3
/// snapshot of the same world.
///
/// Unlike every section above, this one uses the section **flags** word, and
/// it is the reason that word stopped being ignored; see
/// `SECTION_FLAG_DENSE_LAYER0`.
const SECTION_WORLDMOD: u16 = 13;
/// Phase 11 action census. Optional on the same terms as every section above,
/// present exactly when `config.probe.action_census_enabled`, so a snapshot of
/// a world without the probe is byte-identical to the one this build would
/// have written before the section existed and all five fixtures are
/// untouched.
///
/// **The format version stays 4**, following `SECTION_LEARN`'s precedent
/// rather than `SECTION_WORLDMOD`'s: format 4 was bumped because the *logical
/// state* gained a meaning (a composed terrain checksum in the header), not
/// because a section was appended. An absent optional section needs no bump.
/// The decoder still refuses this tag in a format 3 file, so a legacy file
/// carrying it is a typed error rather than a section read under a framing
/// that never defined it.
///
/// Dense: `ACTION_CLASS_COUNT` u32 columns per organism. The learn section
/// beside it is sparse because most edges are not plastic; every living
/// organism has an action every tick, so a sparse histogram would carry an
/// index next to almost every entry and save nothing.
const SECTION_ACTION_CENSUS: u16 = 14;
/// Phase 12 object table (ADR-0028). Optional, present exactly when
/// `config.artifact.enabled`, guarded on `FORMAT_VERSION_7` by name because
/// that is the format that introduced it, permanently.
///
/// Layout: object count, then per object the seventeen fields in the order
/// `ObjectRecord` declares them followed by the composition length and its
/// ids; then `objects_allocated_total`, the ledger's ten `i128` terms, and
/// the counters' thirty `u64` words. Every declared count is bounded by
/// `allocation_fits` before its allocation (D-075).
const SECTION_OBJECTS: u16 = 15;
/// Phase 13 social state (ADR-0029 section 6): the committed signal field,
/// the per-organism one-tick cue records, the emission cost remainders, and
/// the social counters. Guarded on `FORMAT_VERSION_8` by name, permanently
/// (D-108).
const SECTION_SOCIAL: u16 = 16;
/// Phase 14 ontogeny progress (format 9): per-organism grown-prefix
/// lengths and payments toward the next module, plus the section's two
/// counters. Present exactly when the config's ontogeny gate is on.
const SECTION_ONTOGENY: u16 = 17;
/// Phase 14 mate-choice counters (format 10): the section is counters
/// only - the weights cache is expressed from genomes on load.
const SECTION_MATECHOICE: u16 = 18;
/// Phase 15 chemistry field (format 11): per-cell substrate
/// concentrations plus the chemistry ledger. Stored, never recomputed.
const SECTION_CHEMISTRY: u16 = 19;
/// Phase 15 microbial field (format 12): per-cell per-class densities
/// plus the attribution counters. Stored, never recomputed.
const SECTION_MICROBIAL: u16 = 20;
/// Bytes one object's fixed fields occupy: the bound a declared object count
/// implies before any composition list is read.
const OBJECT_FIXED_BYTES: u64 =
    8 + 2 + 4 + 4 + 4 + 8 + 8 + 4 + 4 + 4 + 8 + 8 + 1 + 8 + 8 + 1 + 8 + 8;
/// Bytes one organism's action row occupies. Used to bound the allocation a
/// declared organism count implies, never to assert an exact length (D-075).
const ACTION_CENSUS_BYTES_PER_ORGANISM: u64 = 4 * sim_core::ACTION_CLASS_COUNT as u64;
/// Bytes per sparse override inside the modification section: cell index and
/// value. The layer id is not stored, because the layer is implied by which
/// per-layer block the entry sits in. Used to bound an allocation, never to
/// assert an exact length (D-075).
const WORLDMOD_SPARSE_BYTES_PER_ENTRY: u64 = 4 + 8;
/// Bytes per cell in a dense layer block.
const WORLDMOD_DENSE_BYTES_PER_CELL: u64 = 8;
/// The dense sentinel: "this cell has no override on this layer".
///
/// `-1` is unambiguous for every layer that exists or is reserved, because
/// `sim_core::value_in_domain` requires a non-negative value on all three -
/// traversability is `0..=1`, a capacity scale is a non-negative Q16
/// multiplier, and a material yield is a non-negative quantity. A layer with
/// a signed domain would need a presence bitmap instead, and adding one is a
/// format change, which is exactly the kind of thing a format version is for.
const WORLDMOD_DENSE_ABSENT: i64 = -1;

/// Section flags bit 0: layer 0 is stored densely rather than sparsely. Bits
/// 1 and 2 say the same of layers 1 and 2.
///
/// **This is the field that closed a fail-open.** Every section has carried a
/// 16-bit flags word since format 1; every writer wrote a literal zero and
/// the reader bound it to `_flags` and never looked at it. Any value at all
/// was accepted and silently ignored, so a section could have claimed any
/// property it liked and the loader would have agreed. Now each tag declares
/// the bits it understands (`section_flags_allowed`) and anything else is
/// refused with a typed error, on the same pattern the header's `FLAG_ZSTD`
/// has used since format 1.
///
/// One bit per layer rather than one bit for the whole section, because
/// `specifications/mutable-world-state.md` selects the representation "when
/// the modified cell count exceeds `dense_threshold_q16` of the cell count
/// **for that layer**" - and the layers have wildly different occupancies. A
/// world with a dense capacity patch and three blocked cells would otherwise
/// store a full-map traversability field to say almost nothing.
const SECTION_FLAG_DENSE_LAYER0: u16 = 1;

/// The flag bits a given section tag understands. Everything else is refused.
///
/// A whitelist rather than a blacklist, and per tag rather than global: a bit
/// that means "dense layer 0" in the modification section must not quietly
/// mean anything at all in the organism table.
fn section_flags_allowed(tag: u16) -> u16 {
    match tag {
        SECTION_WORLDMOD => {
            // One bit per reserved layer.
            (1_u16 << LAYER_COUNT) - 1
        }
        _ => 0,
    }
}
/// Smallest number of bytes one organism's learn record can occupy: its
/// plastic-edge count word and its fault word, with no plastic edges at all.
/// Used to bound the allocation a declared organism count implies, never to
/// assert an exact length (D-075).
const LEARN_MIN_PER_ORGANISM: u64 = 4 + 4 + 4;
/// Bytes per stored plastic edge: homology id, learned delta, trace.
const LEARN_BYTES_PER_EDGE: u64 = 4 + 4 + 4;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CodecError {
    TooShort,
    BadMagic,
    UnsupportedFormat(u16),
    UnsupportedSaveState(u16),
    UnsupportedGenomeSchema(u16),
    BadHeaderLength(usize),
    UnknownFlags(u32),
    BuildStringTooLong(usize),
    StoredTooLarge(u64),
    UncompressedTooLarge(u64),
    LengthMismatch {
        expected: usize,
        actual: usize,
    },
    PayloadChecksumMismatch,
    SectionChecksumMismatch(u16),
    DecompressionFailed,
    DecompressedLengthMismatch {
        declared: u64,
        actual: usize,
    },
    TruncatedSection,
    UnknownSection(u16),
    MissingSection(u16),
    DuplicateSection(u16),
    ValueOutOfRange(&'static str),
    /// A section's flags word carried a bit the tag does not define.
    UnknownSectionFlags {
        tag: u16,
        flags: u16,
    },
    /// A section that does not exist in the format version the file claims.
    /// A format 3 file carrying a format 4 section is lying about one of the
    /// two, and either way it is not a format 3 file.
    SectionNotInFormat {
        tag: u16,
        format: u16,
    },
    /// A **config field** that does not exist in the format version being
    /// written, holding a value that version cannot express.
    ///
    /// Distinct from `SectionNotInFormat` because a config field is not a
    /// section: it has no tag, it is never optional, and the refusal is on
    /// the *write* side only. `encode_snapshot_format4` returns this rather
    /// than dropping `plasticity.live_rule_zero`, on the same grounds the
    /// format 3 writer refuses a state carrying a worldmod section - silently
    /// writing a file that describes a different world is the "never alter
    /// meaning during load" rule broken one step earlier, where it is harder
    /// to notice.
    FieldNotInFormat {
        field: &'static str,
        format: u16,
    },
}

impl fmt::Display for CodecError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CodecError {}

/// Decoded header metadata (available without touching the payload).
#[derive(Clone, Debug)]
pub struct SnapshotInfo {
    pub format_version: u16,
    pub compressed: bool,
    pub world_id: u64,
    pub parent_world_id: u64,
    pub tick: u64,
    pub seed: u64,
    pub config_hash: u64,
    pub save_state_version: u16,
    pub genome_schema_version: u16,
    pub build_version: String,
    pub event_log_offset: u64,
    pub uncompressed_len: u64,
    pub stored_len: u64,
    pub state_checksum: u64,
    pub terrain_checksum: u64,
}

// --- primitive writers/readers ---------------------------------------------

struct Writer(Vec<u8>);

impl Writer {
    fn u8(&mut self, value: u8) {
        self.0.push(value);
    }
    fn u16(&mut self, value: u16) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u32(&mut self, value: u32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i32(&mut self, value: i32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i64(&mut self, value: i64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn i128(&mut self, value: i128) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
    fn f32(&mut self, value: f32) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }
}

struct Reader<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8], CodecError> {
        let slice = self
            .bytes
            .get(self.offset..self.offset + count)
            .ok_or(CodecError::TruncatedSection)?;
        self.offset += count;
        Ok(slice)
    }
    fn u8(&mut self) -> Result<u8, CodecError> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, CodecError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, CodecError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, CodecError> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i32(&mut self) -> Result<i32, CodecError> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn i64(&mut self) -> Result<i64, CodecError> {
        Ok(i64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
    fn i128(&mut self) -> Result<i128, CodecError> {
        Ok(i128::from_le_bytes(self.take(16)?.try_into().unwrap()))
    }
    fn f32(&mut self) -> Result<f32, CodecError> {
        Ok(f32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn done(&self) -> bool {
        self.offset == self.bytes.len()
    }
    /// Bytes left in this section body. Used by exactly one caller, the
    /// world-metadata section's optional trailing composed checksum, and
    /// safe there only because `done()` is asserted at the end of every
    /// section: a body with 4 or 12 spare bytes is a decode failure, so
    /// "8 bytes remain" cannot mean anything but "the word is present".
    fn remaining(&self) -> usize {
        self.bytes.len() - self.offset
    }
}

pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = 0_u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

// --- sections ----------------------------------------------------------------

/// Whether a declared count's minimum encoded size fits inside the section
/// body it was read from.
///
/// **The overflow case is the hostile case, and the obvious spelling admits
/// it.** Every section used to write its bound as
/// `count.checked_mul(size) > Some(body.len() as u64)`, which reads correctly
/// and is wrong: `checked_mul` returns `None` for exactly the counts that are
/// too large to multiply, and `None > Some(_)` is **false** under `Option`'s
/// ordering, so those counts passed the guard and went straight into
/// `Vec::with_capacity`, which aborts with a capacity overflow. A loader that
/// panics on hostile input has failed open into a crash - it never reaches
/// the typed error the caller is supposed to see.
///
/// Found by the Phase 11 decode-bound test patching an organism count to
/// `u64::MAX`. The climate, contest, physiology and schema-2 sections all had
/// it and are all routed through here now. `persistence.rs`'s 2,000-flip
/// corruption sweep never found it because a handful of flipped bits does not
/// produce a count near 2^61, and because a panic aborts the sweep rather
/// than counting as a rejection.
fn allocation_fits(count: u64, per_item: u64, extra: u64, body_len: usize) -> bool {
    match count
        .checked_mul(per_item)
        .and_then(|bytes| bytes.checked_add(extra))
    {
        Some(bytes) => bytes <= body_len as u64,
        None => false,
    }
}

/// Write one section with an explicit flags word.
///
/// `flags` was a hard-coded zero here until Phase 12; it is a parameter now
/// so that a writer has to state what it means, and the reader refuses
/// anything `section_flags_allowed` does not define. Callers that carry no
/// flags pass `0` and are byte-identical to what they always wrote.
fn write_section(out: &mut Vec<u8>, tag: u16, flags: u16, body: Vec<u8>) {
    debug_assert_eq!(
        flags & !section_flags_allowed(tag),
        0,
        "section {tag} wrote a flag bit its own reader will refuse"
    );
    out.extend_from_slice(&tag.to_le_bytes());
    out.extend_from_slice(&flags.to_le_bytes());
    out.extend_from_slice(&(body.len() as u64).to_le_bytes());
    let checksum = crc32(&body);
    out.extend_from_slice(&body);
    out.extend_from_slice(&checksum.to_le_bytes());
}

/// Encode the config section for a given framing version.
///
/// `format` is a parameter because format 5's only difference from format 4
/// is one byte in this block, and unlike every optional *section* that
/// difference cannot be gated on the state: a config is written
/// unconditionally, so the byte is present in every format-5 file and absent
/// from every format-4 one. `decode_config` takes the same parameter and the
/// two must be read together - they are the definition of the format
/// difference, and there is nowhere else it is expressed.
fn encode_config(config: &sim_core::SimConfig, format: u16) -> Vec<u8> {
    let mut writer = Writer(Vec::new());
    writer.u64(config.world_seed);
    writer.u32(config.cells_x);
    writer.u32(config.cells_y);
    writer.u32(config.cell_size_m);
    writer.u32(config.initial_organisms);
    writer.u32(config.max_entities);
    writer.u32(config.dt_ms);
    writer.u32(config.growth_rate_q16_per_s);
    writer.i64(config.cell_capacity_milli);
    writer.u32(config.initial_biomass_q16);
    writer.i64(config.energy_max_milli);
    writer.i64(config.initial_energy_milli);
    writer.i64(config.basal_cost_milli_per_s);
    writer.i64(config.move_cost_milli_per_s);
    writer.i64(config.intake_rate_milli_per_s);
    writer.u32(config.assimilation_q16);
    writer.u32(config.speed_mps_q16);
    writer.u32(config.crowding_radius_m);
    writer.u32(config.crowding_threshold);
    writer.i64(config.crowding_cost_milli_per_s);
    writer.u64(config.maturity_age_ticks);
    writer.u64(config.max_age_ticks);
    writer.u8(u8::from(config.reproduction_enabled));
    writer.i64(config.repro_threshold_milli);
    writer.i64(config.offspring_energy_milli);
    writer.i64(config.repro_overhead_milli);
    writer.u64(config.repro_cooldown_ticks);
    writer.u32(config.land_threshold_q16);
    writer.u32(config.min_land_fraction_q16);
    writer.u32(config.max_land_fraction_q16);
    let phase2 = &config.phase2;
    writer.u8(u8::from(phase2.enabled));
    writer.u32(phase2.variation_probability_q16);
    writer.u32(phase2.variation_trait_sigma_q16);
    writer.u32(phase2.variation_neural_sigma_q16);
    writer.u32(phase2.pairing_range_m);
    writer.u32(phase2.compatibility_threshold_q16);
    writer.i64(phase2.pairing_energy_threshold_milli);
    writer.i64(phase2.pairing_overhead_milli);
    writer.i32(phase2.eat_threshold_q16);
    writer.i32(phase2.mate_threshold_q16);
    writer.i32(phase2.rest_threshold_q16);
    writer.u32(phase2.max_turn_per_tick_bam);
    writer.u32(phase2.cluster_threshold_q16);
    writer.u32(phase2.cluster_sample_max);
    writer.u32(phase2.cluster_neural_weight_q16);

    // Phase 6 climate. Written unconditionally, unlike the *state*
    // sections: a config is not optional, and a disabled section still has
    // parameters that have to survive a round trip.
    let climate = &config.climate;
    writer.u8(u8::from(climate.enabled));
    writer.u8(match climate.worldgen_version {
        sim_core::WorldgenVersion::V1 => 1,
        sim_core::WorldgenVersion::V2 => 2,
    });
    writer.i32(climate.base_temperature_milli);
    writer.i32(climate.lapse_milli_per_full_elevation);
    writer.i32(climate.latitude_amplitude_milli);
    writer.u64(climate.season_period_ticks);
    writer.i32(climate.season_amplitude_milli);
    for value in climate.drift_period_ticks {
        writer.u64(value);
    }
    for value in climate.drift_amplitude_milli {
        writer.i32(value);
    }
    writer.i32(climate.temperature_min_milli);
    writer.i32(climate.temperature_max_milli);
    writer.i64(climate.initial_moisture_milli);
    writer.i64(climate.coastal_moisture_bonus_milli);
    writer.i64(climate.moisture_max_milli);
    writer.i64(climate.moisture_ceiling_milli);
    writer.u32(climate.sea_proximity_weight_q16);
    writer.u32(climate.moisture_diffusion_q16);
    writer.u32(climate.moisture_drain_weight);
    writer.u32(climate.highland_elevation_q16);
    writer.i64(climate.wetland_moisture_milli);
    writer.i64(climate.arid_moisture_milli);
    writer.i64(climate.forest_moisture_milli);
    writer.i32(climate.forest_min_temperature_milli);
    for value in climate.biome_capacity_q16 {
        writer.u32(value);
    }
    writer.u64(climate.reclassify_interval_ticks);

    // Phase 6 origin.
    let origin = &config.origin;
    writer.u8(match origin.mode {
        sim_core::OriginMode::Random => 1,
        sim_core::OriginMode::Seeded => 2,
    });
    writer.u32(origin.trait_low_q16);
    writer.u32(origin.trait_span_q16);
    writer.u32(origin.neural_span_q16);
    writer.u32(origin.deme_count);
    writer.u32(origin.deme_radius_m);
    writer.u32(origin.deme_min_separation_m);
    writer.u32(origin.deme_trait_spread_q16);
    writer.u32(origin.archetype_count);
    for archetype in &origin.archetypes {
        writer.u32(u32::from(archetype.id));
        for mean in archetype.trait_mean_q16 {
            writer.u32(u32::from(mean));
        }
        writer.u32(u32::from(archetype.trait_spread_q16));
        writer.u32(u32::from(archetype.neural_spread_q16));
        writer.u32(u32::from(archetype.biome_affinity));
    }

    // Phase 7 contest.
    let contest = &config.contest;
    writer.u8(u8::from(contest.enabled));
    writer.i64(contest.base_health_milli);
    writer.i64(contest.damage_base_milli);
    writer.u32(contest.damage_variance_q16);
    writer.i64(contest.attack_cost_milli);
    writer.u32(contest.attack_range_m);
    writer.i32(contest.attack_threshold_q16);
    writer.u64(contest.attack_cooldown_ticks);
    writer.i64(contest.heal_milli_per_s);
    writer.u32(contest.heal_energy_cost_q16);
    writer.u32(contest.heal_energy_floor_q16);
    writer.u32(contest.damage_decay_q16_per_s);
    writer.u32(contest.carcass_energy_q16);
    writer.u32(contest.carcass_decay_q16_per_s);
    writer.u32(contest.carcass_reach_m);
    writer.u32(contest.max_carcasses);
    writer.i64(contest.local_depletion_milli);

    // Phase 8 physiology.
    let physiology = &config.physiology;
    writer.u8(u8::from(physiology.enabled));
    writer.u8(u8::from(physiology.allometry_enabled));
    writer.u32(physiology.basal_exponent_quarters);
    writer.u8(u8::from(physiology.thermoregulation_enabled));
    writer.i32(physiology.thermal_pref_low_milli);
    writer.i32(physiology.thermal_pref_high_milli);
    writer.i32(physiology.thermal_neutral_band_milli);
    writer.i64(physiology.thermal_cost_milli_per_s_per_degree);
    writer.u8(u8::from(physiology.senescence_enabled));
    writer.u64(physiology.senescence_onset_ticks);
    writer.u64(physiology.senescence_scale_ticks);
    writer.u32(physiology.senescence_power);
    writer.u32(physiology.senescence_hazard_q16_per_s);
    writer.u32(physiology.extrinsic_hazard_q16_per_s);
    writer.u32(physiology.juvenile_hazard_multiplier_q16);

    // Phase 9 genome schema 2.
    let genome2 = &config.genome2;
    writer.u8(u8::from(genome2.enabled));
    writer.u8(genome2.caps.max_chromosomes);
    writer.u32(genome2.caps.max_loci_per_chromosome);
    writer.u32(genome2.caps.max_nodes);
    writer.u32(genome2.caps.max_edges);
    writer.u32(genome2.caps.max_edges_per_node);
    writer.u32(genome2.caps.max_genome_bytes);
    writer.u32(genome2.caps.min_nodes);
    writer.u8(genome2.meiosis.mode.id());
    writer.u32(genome2.meiosis.max_extra_crossovers);
    writer.u32(genome2.mutation.point_q16);
    writer.u32(genome2.mutation.duplication_q16);
    writer.u32(genome2.mutation.deletion_q16);
    writer.u32(genome2.mutation.insertion_q16);
    writer.u32(genome2.mutation.transposition_q16);
    writer.u32(genome2.mutation.max_run);
    writer.u32(genome2.mutation.point_delta_q16);
    writer.u8(u8::from(genome2.mutation.regulatory_enabled));
    // Phase 11's mutation gate, and the **third** time a config flag has been
    // added without this function (D-065, then Phase 10's morphology block
    // immediately below). Its absence is worse than the others: a plasticity
    // treatment run that is checkpointed and resumed comes back with
    // plasticity mutation silently **off**, which turns condition A into
    // condition B mid-run and would be reported as "plasticity was not
    // selected for".
    writer.u8(u8::from(genome2.mutation.plasticity_enabled));
    // Phase 10 morphology config. Absent from the first cut of this
    // function, which meant a restored world had morphology **disabled**:
    // it rebuilt no bodies, its census came back empty, and the analysis
    // read that as "no organism was mature" rather than as "the config did
    // not survive". Exactly the defect D-065 introduced this whole function
    // to prevent, one phase later.
    let morphology = &config.morphology;
    writer.u8(u8::from(morphology.enabled));
    writer.u8(morphology.lattice.id());
    writer.u32(morphology.base_node_budget);
    writer.u32(u32::from(morphology.caps.max_modules));
    writer.i32(i32::from(morphology.caps.lattice_radius));
    writer.u32(u32::from(morphology.caps.max_growth_steps));
    writer.u8(morphology.caps.required_types_mask);
    // Phase 11 plasticity config. **The fourth time a config section has been
    // added without this function**, after D-065's climate/contest/origin, the
    // Phase 10 morphology block above, and `mutation.plasticity_enabled`
    // immediately above that - and it was caught here by
    // `config_round_trip.rs` only because that test was extended in the same
    // change, which is the whole argument for extending it.
    //
    // The consequence is the worst of the four. `World::from_state` checks
    // that the learn section's presence matches the configuration, so a
    // snapshot of a plasticity world decoded with `enabled` back at its
    // `false` default does not restore a world with plasticity quietly off -
    // it **refuses to restore at all**, with a message about section presence
    // that names neither the config field nor the codec. A campaign that
    // checkpoints could not resume.
    let plasticity = &config.plasticity;
    writer.u8(u8::from(plasticity.enabled));
    writer.i64(plasticity.plastic_edge_cost_milli_per_s);
    writer.u32(plasticity.max_plastic_edges);
    writer.u32(plasticity.lamarckian_fraction_q16);
    // Phase 12 mutable world. **The fifth config section to need this
    // function, and the first one whose absence was caught before it
    // shipped** - by `config_field_coverage.rs`, which walks `FIELD_NAMES`
    // rather than a hand-maintained list, and which failed the moment the
    // fields were registered and left unencoded. Every previous instance
    // (D-065's climate/origin/contest, Phase 10's morphology,
    // `mutation.plasticity_enabled`, Phase 11's plasticity) was found after
    // the fact, two of them phases later.
    //
    // The consequence would have been the same class as the plasticity one
    // and one step worse. `World::from_state` refuses a save whose worldmod
    // section presence does not match the configuration, so a snapshot of a
    // mutable world decoded with `enabled` back at its `false` default does
    // not restore a quietly static world - it **refuses to restore at all**,
    // and the message names a section rather than the config field that was
    // dropped.
    let worldmod = &config.worldmod;
    writer.u8(u8::from(worldmod.enabled));
    writer.u32(worldmod.dense_threshold_q16);
    writer.u32(worldmod.max_traversable_overrides);
    writer.u32(worldmod.max_capacity_overrides);
    writer.u32(worldmod.max_material_overrides);
    writer.u8(u8::from(worldmod.patch_enabled));
    writer.u64(worldmod.relocate_interval_ticks);
    writer.u32(worldmod.patch_radius_cells);
    writer.u32(worldmod.patch_capacity_scale_q16);
    // Phase 11 measurement section, appended last. `config_field_coverage.rs`
    // is what makes this line's absence a failing test rather than a silent
    // loss - and the loss would be the worst-behaved of the six, because
    // `probe.marker_locus_enabled` does not merely change what is stored: a
    // restore that decoded it as `false` would keep the marker loci in the
    // genomes (they are in the schema-2 section) while the world believed it
    // had no marker, so `marker_census` would keep reporting them and the
    // config hash would say the run had no drift control.
    let probe = &config.probe;
    writer.u8(u8::from(probe.enabled));
    writer.u8(u8::from(probe.action_census_enabled));
    writer.u8(u8::from(probe.marker_locus_enabled));
    // Format 5's one byte, **appended at the end rather than filed with the
    // plasticity block it belongs to**, and the reason is a property worth
    // more than the grouping: appended, the format-4 config body is a byte
    // *prefix* of the format-5 body for the same world. That is a single
    // assertion a test can make and a reader can check by eye
    // (`the_format_4_config_body_is_a_prefix_of_the_format_5_body`). Filed
    // next to `plasticity.lamarckian_fraction_q16`, the two bodies would
    // instead differ from that point on, `worldmod` and `probe` would sit at
    // different offsets in the two formats, and the only way to state the
    // difference would be to re-describe the layout.
    //
    // The order here is not the config *hash* order and the two must not be
    // conflated: `SimConfig::stable_hash` appends for a different reason (its
    // order is the definition of every hash already issued) and gates each
    // section on being enabled. This block is unconditional.
    if format >= FORMAT_VERSION_5 {
        writer.u8(u8::from(config.plasticity.live_rule_zero));
    }
    // Format 6's byte, appended after format 5's for the same reason format
    // 5's was appended after `probe`: each format's config body stays a byte
    // prefix of the next, so the difference between any two adjacent formats
    // is one assertion rather than a re-description of the layout.
    if format >= FORMAT_VERSION_6 {
        writer.u8(u8::from(config.plasticity.price_moved_edges_only));
    }
    // Format 7's block: the artifact section and `binding_q16`, appended in
    // one piece after format 6's byte for the reason format 6's was appended
    // after format 5's - the format-6 body stays a byte prefix of the
    // format-7 body, and every byte here is its field's default in a world
    // that has no artifact section (`enabled` false, `binding_q16` 0), so
    // the delta between the two formats is a fixed number of default bytes.
    // Guarded on `FORMAT_VERSION_7` by name, permanently.
    if format >= FORMAT_VERSION_7 {
        encode_artifact_config(&mut writer, config);
    }
    // Format 8's block: the social section, appended after format 7's block
    // for the reason format 7's was appended after format 6's byte. Guarded
    // on `FORMAT_VERSION_8` by name, permanently.
    if format >= FORMAT_VERSION_8 {
        encode_social_config(&mut writer, config);
    }
    // Format 9's block: the physiology-v2 ontogeny fields, appended after
    // format 8's block for the reason format 8's was appended after format
    // 7's. Guarded on `FORMAT_VERSION_9` by name, permanently.
    if format >= FORMAT_VERSION_9 {
        encode_physiology_v2_config(&mut writer, config);
    }
    // Format 10's block, appended after format 9's on the same terms.
    if format >= FORMAT_VERSION_10 {
        encode_matechoice_config(&mut writer, config);
    }
    // Format 11's block, on the same terms again.
    if format >= FORMAT_VERSION_11 {
        encode_chemistry_config(&mut writer, config);
    }
    // Format 12's microbial fields. Guarded on `FORMAT_VERSION_12` by
    // name, permanently.
    if format >= FORMAT_VERSION_12 {
        encode_microbial_config(&mut writer, config);
    }
    writer.0
}

/// The format-7 artifact block, one field per line in the order the
/// specification's configuration table lists them, then `binding_q16`.
/// `config_field_coverage.rs` sweeps every one of these through the real
/// codec, so a field added to `ArtifactConfig` and not written here fails a
/// test rather than restoring at its default.
fn encode_artifact_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    let artifact = &config.artifact;
    writer.u8(u8::from(artifact.enabled));
    writer.u8(u8::from(artifact.inert));
    writer.u8(u8::from(artifact.ephemeral));
    writer.u32(artifact.max_objects);
    writer.u32(artifact.max_objects_per_cell);
    writer.u32(artifact.max_composition_depth);
    writer.u32(artifact.max_composition_breadth);
    writer.u32(artifact.max_held_objects);
    writer.u32(artifact.max_candidates);
    writer.i64(artifact.carry_capacity_milli);
    writer.u32(artifact.carry_move_cost_q16);
    writer.i64(artifact.hold_cost_milli_per_s);
    writer.i64(artifact.action_cost_milli);
    writer.i64(artifact.strike_cost_milli);
    writer.i32(artifact.action_threshold_q16);
    writer.u32(artifact.reach_m);
    writer.u32(artifact.consume_reach_m);
    writer.u32(artifact.perception_range_m);
    writer.u32(artifact.strike_force_q16);
    writer.i64(artifact.strike_mass_reference_milli);
    writer.u32(artifact.fracture_margin_q16);
    writer.u32(artifact.max_fragments);
    writer.i64(artifact.min_fragment_mass_milli);
    writer.u32(artifact.joint_floor_q16);
    writer.i64(artifact.blocking_mass_milli);
    writer.i64(artifact.terrain_yield_milli);
    writer.i64(artifact.extraction_milli);
    writer.i64(artifact.yield_regen_milli);
    writer.u64(artifact.yield_regen_interval_ticks);
    writer.u32(artifact.stone_relative_q16);
    writer.u32(artifact.wood_relative_q16);
    writer.u32(config.genome2.mutation.binding_q16);
}

/// Bytes the format-7 block adds to a config body. Asserted by the chain
/// test rather than trusted: 3 + 6*4 + 8 + 4 + 8*3 + 4 + 3*4 + 4 + 8 + 4 + 4 + 8 + 4 + 8*4 + 8 + 4*2 + 4.
pub const FORMAT7_CONFIG_BYTES: usize = 3 // enabled, inert, ephemeral
    + 6 * 4 // six caps
    + 8 + 4 + 8 // carry
    + 8 + 8 + 4 // costs, threshold
    + 3 * 4 // reach
    + 4 + 8 + 4 + 4 + 8 // strike/fracture
    + 4 // joint floor
    + 8 // blocking mass
    + 8 + 8 + 8 + 8 // yield
    + 4 + 4 // elevation bands
    + 4; // binding_q16

fn decode_artifact_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    let artifact = &mut config.artifact;
    artifact.enabled = reader.u8()? != 0;
    artifact.inert = reader.u8()? != 0;
    artifact.ephemeral = reader.u8()? != 0;
    artifact.max_objects = reader.u32()?;
    artifact.max_objects_per_cell = reader.u32()?;
    artifact.max_composition_depth = reader.u32()?;
    artifact.max_composition_breadth = reader.u32()?;
    artifact.max_held_objects = reader.u32()?;
    artifact.max_candidates = reader.u32()?;
    artifact.carry_capacity_milli = reader.i64()?;
    artifact.carry_move_cost_q16 = reader.u32()?;
    artifact.hold_cost_milli_per_s = reader.i64()?;
    artifact.action_cost_milli = reader.i64()?;
    artifact.strike_cost_milli = reader.i64()?;
    artifact.action_threshold_q16 = reader.i32()?;
    artifact.reach_m = reader.u32()?;
    artifact.consume_reach_m = reader.u32()?;
    artifact.perception_range_m = reader.u32()?;
    artifact.strike_force_q16 = reader.u32()?;
    artifact.strike_mass_reference_milli = reader.i64()?;
    artifact.fracture_margin_q16 = reader.u32()?;
    artifact.max_fragments = reader.u32()?;
    artifact.min_fragment_mass_milli = reader.i64()?;
    artifact.joint_floor_q16 = reader.u32()?;
    artifact.blocking_mass_milli = reader.i64()?;
    artifact.terrain_yield_milli = reader.i64()?;
    artifact.extraction_milli = reader.i64()?;
    artifact.yield_regen_milli = reader.i64()?;
    artifact.yield_regen_interval_ticks = reader.u64()?;
    artifact.stone_relative_q16 = reader.u32()?;
    artifact.wood_relative_q16 = reader.u32()?;
    config.genome2.mutation.binding_q16 = reader.u32()?;
    Ok(())
}

/// The format-8 social block, one field per line in declaration order.
/// `config_field_coverage.rs` sweeps every one of these through the real
/// codec, so a field added to `SocialConfig` and not written here fails a
/// test rather than restoring at its default.
fn encode_social_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    let social = &config.social;
    writer.u8(u8::from(social.enabled));
    writer.u8(u8::from(social.perception_enabled));
    writer.u8(u8::from(social.signal_enabled));
    writer.u8(u8::from(social.scramble_delivery));
    writer.u8(u8::from(social.observational_enabled));
    writer.u32(social.perception_k);
    writer.u32(social.perception_radius_m);
    writer.u32(social.signal_channels);
    writer.u32(social.signal_base_range_m);
    writer.i64(social.signal_cost_milli);
    writer.u32(social.signal_retain_q16);
    writer.u32(social.signal_corruption_q16);
}

/// Bytes the format-8 block adds to a config body. Asserted by the chain
/// test rather than trusted.
pub const FORMAT8_CONFIG_BYTES: usize = 5 // the five gates
    + 4 * 4 // k, radius, channels, base range
    + 8 // signal cost
    + 4 + 4; // retain, corruption

fn decode_social_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    let social = &mut config.social;
    social.enabled = reader.u8()? != 0;
    social.perception_enabled = reader.u8()? != 0;
    social.signal_enabled = reader.u8()? != 0;
    social.scramble_delivery = reader.u8()? != 0;
    social.observational_enabled = reader.u8()? != 0;
    social.perception_k = reader.u32()?;
    social.perception_radius_m = reader.u32()?;
    social.signal_channels = reader.u32()?;
    social.signal_base_range_m = reader.u32()?;
    social.signal_cost_milli = reader.i64()?;
    social.signal_retain_q16 = reader.u32()?;
    social.signal_corruption_q16 = reader.u32()?;
    Ok(())
}

/// The format-9 physiology-v2 block, one field per line in declaration
/// order. `config_field_coverage.rs` sweeps every one of these through the
/// real codec, so a field added to the ontogeny group and not written here
/// fails a test rather than restoring at its default.
fn encode_physiology_v2_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    let physiology = &config.physiology;
    writer.u8(u8::from(physiology.ontogeny_enabled));
    writer.u32(physiology.birth_modules_min);
    writer.i64(physiology.growth_cost_milli_per_mass_milli);
    writer.i64(physiology.growth_rate_milli_per_s);
}

/// Bytes the format-9 block adds to a config body. Asserted by the chain
/// test rather than trusted.
pub const FORMAT9_CONFIG_BYTES: usize = 1 // the gate
    + 4 // birth_modules_min
    + 8 + 8; // growth cost, growth rate

fn decode_physiology_v2_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    let physiology = &mut config.physiology;
    physiology.ontogeny_enabled = reader.u8()? != 0;
    physiology.birth_modules_min = reader.u32()?;
    physiology.growth_cost_milli_per_mass_milli = reader.i64()?;
    physiology.growth_rate_milli_per_s = reader.i64()?;
    Ok(())
}

/// The format-10 mate-choice block: the two gates, one field per line in
/// declaration order, swept by `config_field_coverage.rs` like every
/// config field before them.
fn encode_matechoice_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    writer.u8(u8::from(config.physiology.mate_choice_enabled));
    writer.u8(u8::from(config.physiology.mate_choice_scramble));
}

/// Bytes the format-10 block adds to a config body. Asserted by the chain
/// test rather than trusted.
pub const FORMAT10_CONFIG_BYTES: usize = 2;

fn decode_matechoice_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    config.physiology.mate_choice_enabled = reader.u8()? != 0;
    config.physiology.mate_choice_scramble = reader.u8()? != 0;
    Ok(())
}

/// The format-11 chemistry block, one field per line in declaration
/// order, swept by `config_field_coverage.rs` like every block before it.
fn encode_chemistry_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    let chemistry = &config.chemistry;
    writer.u8(u8::from(chemistry.enabled));
    writer.u32(chemistry.field_steps_per_tick);
    writer.u32(chemistry.diffusion_q16);
    writer.u32(chemistry.reaction_monomer_q16);
    writer.u32(chemistry.reaction_recycle_q16);
    writer.i64(chemistry.production_milli_per_step);
    writer.u32(chemistry.scaffold_patch_radius_cells);
    writer.u32(chemistry.scaffold_patch_contrast_q16);
    writer.u8(u8::from(chemistry.abiogenesis_enabled));
    writer.u32(chemistry.abiogenesis_weight_primordial_q16);
    writer.u32(chemistry.abiogenesis_weight_monomer_q16);
    writer.u32(chemistry.abiogenesis_weight_polymer_q16);
    writer.u32(chemistry.abiogenesis_cap_q16);
    writer.i64(chemistry.abiogenesis_seed_milli);
}

/// Bytes the format-11 block adds to a config body. Asserted by the chain
/// test rather than trusted.
pub const FORMAT11_CONFIG_BYTES: usize = 1 + 4 * 4 + 8 + 4 * 2 + 1 + 4 * 4 + 8;

fn decode_chemistry_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    let chemistry = &mut config.chemistry;
    chemistry.enabled = reader.u8()? != 0;
    chemistry.field_steps_per_tick = reader.u32()?;
    chemistry.diffusion_q16 = reader.u32()?;
    chemistry.reaction_monomer_q16 = reader.u32()?;
    chemistry.reaction_recycle_q16 = reader.u32()?;
    chemistry.production_milli_per_step = reader.i64()?;
    chemistry.scaffold_patch_radius_cells = reader.u32()?;
    chemistry.scaffold_patch_contrast_q16 = reader.u32()?;
    chemistry.abiogenesis_enabled = reader.u8()? != 0;
    chemistry.abiogenesis_weight_primordial_q16 = reader.u32()?;
    chemistry.abiogenesis_weight_monomer_q16 = reader.u32()?;
    chemistry.abiogenesis_weight_polymer_q16 = reader.u32()?;
    chemistry.abiogenesis_cap_q16 = reader.u32()?;
    chemistry.abiogenesis_seed_milli = reader.i64()?;
    Ok(())
}

/// The format-12 microbial block, one field per line in declaration
/// order, swept by `config_field_coverage.rs` like every block before it.
fn encode_microbial_config(writer: &mut Writer, config: &sim_core::SimConfig) {
    let chemistry = &config.chemistry;
    writer.u8(u8::from(chemistry.microbial_enabled));
    writer.u32(chemistry.replication_axis);
    writer.u32(chemistry.aggregation_axis);
    writer.u32(chemistry.growth_rate_low_q16);
    writer.u32(chemistry.growth_rate_high_q16);
    writer.u32(chemistry.growth_yield_q16);
    writer.u32(chemistry.death_q16);
    writer.u32(chemistry.death_waste_fraction_q16);
    writer.u32(chemistry.mutation_q16);
}

/// Bytes the format-12 block adds to a config body. Asserted by the chain
/// test rather than trusted.
pub const FORMAT12_CONFIG_BYTES: usize = 1 + 4 * 8;

fn decode_microbial_config(
    reader: &mut Reader,
    config: &mut sim_core::SimConfig,
) -> Result<(), CodecError> {
    let chemistry = &mut config.chemistry;
    chemistry.microbial_enabled = reader.u8()? != 0;
    chemistry.replication_axis = reader.u32()?;
    chemistry.aggregation_axis = reader.u32()?;
    chemistry.growth_rate_low_q16 = reader.u32()?;
    chemistry.growth_rate_high_q16 = reader.u32()?;
    chemistry.growth_yield_q16 = reader.u32()?;
    chemistry.death_q16 = reader.u32()?;
    chemistry.death_waste_fraction_q16 = reader.u32()?;
    chemistry.mutation_q16 = reader.u32()?;
    Ok(())
}

/// Decode the config section written by `encode_config` at the same version.
///
/// Read the two together. A format-4 body reaching this at format 5 runs out
/// of bytes on the last field and fails `TruncatedSection`; a format-5 body
/// reaching it at format 4 leaves one byte over and fails the trailing-bytes
/// check every section runs. Both directions fail closed on the *body* alone,
/// before the header's version word is consulted at all - which is what makes
/// a file with a forged version word fail too.
fn decode_config(reader: &mut Reader, format: u16) -> Result<sim_core::SimConfig, CodecError> {
    let mut config = sim_core::SimConfig::phase1_default(reader.u64()?);
    config.cells_x = reader.u32()?;
    config.cells_y = reader.u32()?;
    config.cell_size_m = reader.u32()?;
    config.initial_organisms = reader.u32()?;
    config.max_entities = reader.u32()?;
    config.dt_ms = reader.u32()?;
    config.growth_rate_q16_per_s = reader.u32()?;
    config.cell_capacity_milli = reader.i64()?;
    config.initial_biomass_q16 = reader.u32()?;
    config.energy_max_milli = reader.i64()?;
    config.initial_energy_milli = reader.i64()?;
    config.basal_cost_milli_per_s = reader.i64()?;
    config.move_cost_milli_per_s = reader.i64()?;
    config.intake_rate_milli_per_s = reader.i64()?;
    config.assimilation_q16 = reader.u32()?;
    config.speed_mps_q16 = reader.u32()?;
    config.crowding_radius_m = reader.u32()?;
    config.crowding_threshold = reader.u32()?;
    config.crowding_cost_milli_per_s = reader.i64()?;
    config.maturity_age_ticks = reader.u64()?;
    config.max_age_ticks = reader.u64()?;
    config.reproduction_enabled = reader.u8()? != 0;
    config.repro_threshold_milli = reader.i64()?;
    config.offspring_energy_milli = reader.i64()?;
    config.repro_overhead_milli = reader.i64()?;
    config.repro_cooldown_ticks = reader.u64()?;
    config.land_threshold_q16 = reader.u32()?;
    config.min_land_fraction_q16 = reader.u32()?;
    config.max_land_fraction_q16 = reader.u32()?;
    config.phase2.enabled = reader.u8()? != 0;
    config.phase2.variation_probability_q16 = reader.u32()?;
    config.phase2.variation_trait_sigma_q16 = reader.u32()?;
    config.phase2.variation_neural_sigma_q16 = reader.u32()?;
    config.phase2.pairing_range_m = reader.u32()?;
    config.phase2.compatibility_threshold_q16 = reader.u32()?;
    config.phase2.pairing_energy_threshold_milli = reader.i64()?;
    config.phase2.pairing_overhead_milli = reader.i64()?;
    config.phase2.eat_threshold_q16 = reader.i32()?;
    config.phase2.mate_threshold_q16 = reader.i32()?;
    config.phase2.rest_threshold_q16 = reader.i32()?;
    config.phase2.max_turn_per_tick_bam = reader.u32()?;
    config.phase2.cluster_threshold_q16 = reader.u32()?;
    config.phase2.cluster_sample_max = reader.u32()?;
    config.phase2.cluster_neural_weight_q16 = reader.u32()?;

    config.climate.enabled = reader.u8()? != 0;
    config.climate.worldgen_version = match reader.u8()? {
        1 => sim_core::WorldgenVersion::V1,
        2 => sim_core::WorldgenVersion::V2,
        _ => return Err(CodecError::ValueOutOfRange("worldgen_version")),
    };
    config.climate.base_temperature_milli = reader.i32()?;
    config.climate.lapse_milli_per_full_elevation = reader.i32()?;
    config.climate.latitude_amplitude_milli = reader.i32()?;
    config.climate.season_period_ticks = reader.u64()?;
    config.climate.season_amplitude_milli = reader.i32()?;
    for index in 0..config.climate.drift_period_ticks.len() {
        config.climate.drift_period_ticks[index] = reader.u64()?;
    }
    for index in 0..config.climate.drift_amplitude_milli.len() {
        config.climate.drift_amplitude_milli[index] = reader.i32()?;
    }
    config.climate.temperature_min_milli = reader.i32()?;
    config.climate.temperature_max_milli = reader.i32()?;
    config.climate.initial_moisture_milli = reader.i64()?;
    config.climate.coastal_moisture_bonus_milli = reader.i64()?;
    config.climate.moisture_max_milli = reader.i64()?;
    config.climate.moisture_ceiling_milli = reader.i64()?;
    config.climate.sea_proximity_weight_q16 = reader.u32()?;
    config.climate.moisture_diffusion_q16 = reader.u32()?;
    config.climate.moisture_drain_weight = reader.u32()?;
    config.climate.highland_elevation_q16 = reader.u32()?;
    config.climate.wetland_moisture_milli = reader.i64()?;
    config.climate.arid_moisture_milli = reader.i64()?;
    config.climate.forest_moisture_milli = reader.i64()?;
    config.climate.forest_min_temperature_milli = reader.i32()?;
    for index in 0..config.climate.biome_capacity_q16.len() {
        config.climate.biome_capacity_q16[index] = reader.u32()?;
    }
    config.climate.reclassify_interval_ticks = reader.u64()?;

    config.origin.mode = match reader.u8()? {
        1 => sim_core::OriginMode::Random,
        2 => sim_core::OriginMode::Seeded,
        _ => return Err(CodecError::ValueOutOfRange("origin_mode")),
    };
    config.origin.trait_low_q16 = reader.u32()?;
    config.origin.trait_span_q16 = reader.u32()?;
    config.origin.neural_span_q16 = reader.u32()?;
    config.origin.deme_count = reader.u32()?;
    config.origin.deme_radius_m = reader.u32()?;
    config.origin.deme_min_separation_m = reader.u32()?;
    config.origin.deme_trait_spread_q16 = reader.u32()?;
    config.origin.archetype_count = reader.u32()?;
    for index in 0..config.origin.archetypes.len() {
        let archetype = &mut config.origin.archetypes[index];
        archetype.id = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype id"))?;
        for gene in 0..archetype.trait_mean_q16.len() {
            archetype.trait_mean_q16[gene] = u16::try_from(reader.u32()?)
                .map_err(|_| CodecError::ValueOutOfRange("archetype trait mean"))?;
        }
        archetype.trait_spread_q16 = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype trait spread"))?;
        archetype.neural_spread_q16 = u16::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype neural spread"))?;
        archetype.biome_affinity = u8::try_from(reader.u32()?)
            .map_err(|_| CodecError::ValueOutOfRange("archetype biome affinity"))?;
    }

    config.contest.enabled = reader.u8()? != 0;
    config.contest.base_health_milli = reader.i64()?;
    config.contest.damage_base_milli = reader.i64()?;
    config.contest.damage_variance_q16 = reader.u32()?;
    config.contest.attack_cost_milli = reader.i64()?;
    config.contest.attack_range_m = reader.u32()?;
    config.contest.attack_threshold_q16 = reader.i32()?;
    config.contest.attack_cooldown_ticks = reader.u64()?;
    config.contest.heal_milli_per_s = reader.i64()?;
    config.contest.heal_energy_cost_q16 = reader.u32()?;
    config.contest.heal_energy_floor_q16 = reader.u32()?;
    config.contest.damage_decay_q16_per_s = reader.u32()?;
    config.contest.carcass_energy_q16 = reader.u32()?;
    config.contest.carcass_decay_q16_per_s = reader.u32()?;
    config.contest.carcass_reach_m = reader.u32()?;
    config.contest.max_carcasses = reader.u32()?;
    config.contest.local_depletion_milli = reader.i64()?;

    config.physiology.enabled = reader.u8()? != 0;
    config.physiology.allometry_enabled = reader.u8()? != 0;
    config.physiology.basal_exponent_quarters = reader.u32()?;
    config.physiology.thermoregulation_enabled = reader.u8()? != 0;
    config.physiology.thermal_pref_low_milli = reader.i32()?;
    config.physiology.thermal_pref_high_milli = reader.i32()?;
    config.physiology.thermal_neutral_band_milli = reader.i32()?;
    config.physiology.thermal_cost_milli_per_s_per_degree = reader.i64()?;
    config.physiology.senescence_enabled = reader.u8()? != 0;
    config.physiology.senescence_onset_ticks = reader.u64()?;
    config.physiology.senescence_scale_ticks = reader.u64()?;
    config.physiology.senescence_power = reader.u32()?;
    config.physiology.senescence_hazard_q16_per_s = reader.u32()?;
    config.physiology.extrinsic_hazard_q16_per_s = reader.u32()?;
    config.physiology.juvenile_hazard_multiplier_q16 = reader.u32()?;

    config.genome2.enabled = reader.u8()? != 0;
    config.genome2.caps.max_chromosomes = reader.u8()?;
    config.genome2.caps.max_loci_per_chromosome = reader.u32()?;
    config.genome2.caps.max_nodes = reader.u32()?;
    config.genome2.caps.max_edges = reader.u32()?;
    config.genome2.caps.max_edges_per_node = reader.u32()?;
    config.genome2.caps.max_genome_bytes = reader.u32()?;
    config.genome2.caps.min_nodes = reader.u32()?;
    config.genome2.meiosis.mode = sim_core::InheritanceMode::from_id(reader.u8()?)
        .ok_or(CodecError::ValueOutOfRange("inheritance_mode"))?;
    config.genome2.meiosis.max_extra_crossovers = reader.u32()?;
    config.genome2.mutation.point_q16 = reader.u32()?;
    config.genome2.mutation.duplication_q16 = reader.u32()?;
    config.genome2.mutation.deletion_q16 = reader.u32()?;
    config.genome2.mutation.insertion_q16 = reader.u32()?;
    config.genome2.mutation.transposition_q16 = reader.u32()?;
    config.genome2.mutation.max_run = reader.u32()?;
    config.genome2.mutation.point_delta_q16 = reader.u32()?;
    config.genome2.mutation.regulatory_enabled = reader.u8()? != 0;
    config.genome2.mutation.plasticity_enabled = reader.u8()? != 0;
    config.morphology.enabled = reader.u8()? != 0;
    let lattice_id = reader.u8()?;
    config.morphology.lattice = sim_core::LatticeKind::from_id(lattice_id)
        .ok_or(CodecError::ValueOutOfRange("morphology lattice"))?;
    config.morphology.base_node_budget = reader.u32()?;
    config.morphology.caps.max_modules = u16::try_from(reader.u32()?)
        .map_err(|_| CodecError::ValueOutOfRange("morphology max_modules"))?;
    config.morphology.caps.lattice_radius = i16::try_from(reader.i32()?)
        .map_err(|_| CodecError::ValueOutOfRange("morphology lattice_radius"))?;
    config.morphology.caps.max_growth_steps = u16::try_from(reader.u32()?)
        .map_err(|_| CodecError::ValueOutOfRange("morphology max_growth_steps"))?;
    config.morphology.caps.required_types_mask = reader.u8()?;
    config.plasticity.enabled = reader.u8()? != 0;
    config.plasticity.plastic_edge_cost_milli_per_s = reader.i64()?;
    config.plasticity.max_plastic_edges = reader.u32()?;
    config.plasticity.lamarckian_fraction_q16 = reader.u32()?;
    config.worldmod.enabled = reader.u8()? != 0;
    config.worldmod.dense_threshold_q16 = reader.u32()?;
    config.worldmod.max_traversable_overrides = reader.u32()?;
    config.worldmod.max_capacity_overrides = reader.u32()?;
    config.worldmod.max_material_overrides = reader.u32()?;
    config.worldmod.patch_enabled = reader.u8()? != 0;
    config.worldmod.relocate_interval_ticks = reader.u64()?;
    config.worldmod.patch_radius_cells = reader.u32()?;
    config.worldmod.patch_capacity_scale_q16 = reader.u32()?;
    config.probe.enabled = reader.u8()? != 0;
    config.probe.action_census_enabled = reader.u8()? != 0;
    config.probe.marker_locus_enabled = reader.u8()? != 0;
    // Format 5's byte. Left at its `false` default for a format-4 body, and
    // that is a resolution rather than an invention: rule 0 was a no-op in
    // every build that could write a format-4 file, so `false` is what the
    // world the file describes actually ran with. It is the same kind of
    // identity the 3-to-4 transform uses for the composed terrain checksum,
    // and it is why the migration's `expected_loss` is still the empty string.
    if format >= FORMAT_VERSION_5 {
        config.plasticity.live_rule_zero = reader.u8()? != 0;
    }
    // Format 6's byte. Left at `false` for an older body, and that is a
    // resolution rather than an invention on the same terms as format 5's:
    // every world that could write a format-5 or format-4 file priced every
    // flagged edge, because no other pricing existed.
    if format >= FORMAT_VERSION_6 {
        config.plasticity.price_moved_edges_only = reader.u8()? != 0;
    }
    // Format 7's block. Left at its defaults for an older body - no artifact
    // section, `binding_q16` zero - and that is a resolution rather than an
    // invention on the same terms as formats 5 and 6: no build that could
    // write a format-6 file had an object in it or a `bind` operator to run.
    if format >= FORMAT_VERSION_7 {
        decode_artifact_config(&mut *reader, &mut config)?;
    }
    if format >= FORMAT_VERSION_8 {
        decode_social_config(&mut *reader, &mut config)?;
    }
    // Format 9's block. Left at its defaults for an older body - ontogeny
    // off, its knobs at their documented defaults - and that is a
    // resolution rather than an invention on the same terms as formats 5
    // through 8: no build that could write a format-8 file grew a body
    // over a lifetime.
    if format >= FORMAT_VERSION_9 {
        decode_physiology_v2_config(&mut *reader, &mut config)?;
    }
    // Format 10's block. Left at its defaults for an older body - both
    // gates off - the same resolution-not-invention every earlier appended
    // block states: no build that could write a format-9 file chose a mate
    // by anything but distance.
    if format >= FORMAT_VERSION_10 {
        decode_matechoice_config(&mut *reader, &mut config)?;
    }
    // Format 11's block. Left at its defaults for an older body - the
    // whole section off - the same resolution-not-invention as always: no
    // build that could write a format-10 file held a chemistry field.
    if format >= FORMAT_VERSION_11 {
        decode_chemistry_config(&mut *reader, &mut config)?;
    }
    if format >= FORMAT_VERSION_12 {
        decode_microbial_config(&mut *reader, &mut config)?;
    }
    Ok(config)
}

/// Encode the payload for a given framing version.
///
/// A retained writer for every format that has a retained reader, because the
/// acceptance requirement for each migration is byte identity against a real
/// legacy file, and a comparison against a reimplementation of the old writer
/// would test the reimplementation.
///
/// **The parameter became real at format 5.** Until then it was documented as
/// a parameter and was not one: the format 3-to-4 differences were the
/// worldmod section and the composed checksum, both gated on the *state*
/// rather than the version, so a format 4 file for a world without the
/// section is byte-identical to the format 3 file for the same world and the
/// function needed no version at all. Format 5's difference is a config byte
/// that is written unconditionally, so it can only be gated on the version -
/// which is why `encode_config` now takes one and this function has to pass
/// it down.
fn encode_payload(state: &SaveState, format: u16) -> Vec<u8> {
    let mut payload = Vec::new();
    write_section(
        &mut payload,
        SECTION_CONFIG,
        0,
        encode_config(&state.config, format),
    );

    let mut meta = Writer(Vec::new());
    meta.u64(state.tick);
    meta.u8(u8::from(state.paused));
    meta.u8(u8::from(state.extinct));
    meta.u64(state.next_entity_id);
    meta.u64(state.terrain_checksum);
    if state.worldmod.is_some() {
        // Written only alongside a modification section, so a world without
        // one produces the metadata section it always produced. A reader that
        // finds no word takes the baseline as the composed value, which is
        // the identity `TerrainModState::composed_checksum` guarantees for an
        // empty set - so the field is never *inferred*, only omitted where
        // the two numbers are provably the same.
        meta.u64(state.composed_terrain_checksum);
    }
    write_section(&mut payload, SECTION_WORLD_META, 0, meta.0);

    let mut organisms = Writer(Vec::new());
    organisms.u64(state.ids.len() as u64);
    for index in 0..state.ids.len() {
        organisms.u64(state.ids[index]);
        organisms.i32(state.x_fp[index]);
        organisms.i32(state.y_fp[index]);
        organisms.i64(state.energy_milli[index]);
        organisms.u64(state.age_ticks[index]);
        organisms.u64(state.cooldown_ticks[index]);
    }
    write_section(&mut payload, SECTION_ORGANISMS, 0, organisms.0);

    let mut biomass = Writer(Vec::new());
    biomass.u64(state.biomass_milli.len() as u64);
    for &value in &state.biomass_milli {
        biomass.i64(value);
    }
    write_section(&mut payload, SECTION_BIOMASS, 0, biomass.0);

    let mut ledger = Writer(Vec::new());
    ledger.i128(state.ledger.initial_energy_milli);
    ledger.i128(state.ledger.assimilated_milli);
    ledger.i128(state.ledger.spent_milli);
    ledger.i128(state.ledger.removed_at_death_milli);
    ledger.i128(state.ledger.initial_biomass_milli);
    ledger.i128(state.ledger.grown_milli);
    ledger.i128(state.ledger.consumed_biomass_milli);
    ledger.u64(state.counters.births_total);
    ledger.u64(state.counters.deaths_starvation_total);
    ledger.u64(state.counters.deaths_old_age_total);
    ledger.u64(state.counters.capacity_rejections_total);
    ledger.u64(state.counters.dropped_events_total);
    write_section(&mut payload, SECTION_LEDGER, 0, ledger.0);

    if let Some(phase2) = &state.phase2 {
        let mut section = Writer(Vec::new());
        // **Two counts, because they are not the same number.** A schema-2
        // world carries no flat genome, so `traits` and `neural` are empty
        // by construction while every other per-organism array is full
        // length. This loop used to be driven by `traits.len()`, which meant
        // a schema-2 snapshot encoded zero per-organism records and silently
        // dropped heading, speed, turn, parents, depth, child count, birth
        // tick, and memory - all state a schema-2 world uses. It failed
        // closed on restore rather than corrupting, but it failed: a
        // schema-2 world could not be checkpointed at all.
        let organisms = phase2.heading_bam.len() as u64;
        let flat_genomes = phase2.traits.len() as u64;
        section.u64(organisms);
        section.u64(flat_genomes);
        for index in 0..phase2.traits.len() {
            for &gene in &phase2.traits[index] {
                section.f32(gene);
            }
            for &gene in &phase2.neural[index] {
                section.f32(gene);
            }
        }
        for index in 0..phase2.heading_bam.len() {
            for &value in &phase2.memory[index] {
                section.f32(value);
            }
            section.u16(phase2.heading_bam[index]);
            section.i64(phase2.speed_milli[index]);
            section.f32(phase2.last_turn[index]);
            section.u64(phase2.parents[index][0]);
            section.u64(phase2.parents[index][1]);
            section.u32(phase2.depth[index]);
            section.u32(phase2.child_count[index]);
            section.u64(phase2.birth_tick[index]);
        }
        section.u64(phase2.counters.paired_births_total);
        section.u64(phase2.counters.pair_rejected_capacity_total);
        section.u64(phase2.counters.pair_rejected_placement_total);
        section.u64(phase2.counters.pair_rejected_energy_total);
        section.u64(phase2.counters.pair_rejected_nonviable_total);
        section.u64(phase2.counters.controller_faults_total);
        section.u64(phase2.counters.mutated_trait_genes_total);
        section.u64(phase2.counters.mutated_neural_genes_total);
        write_section(&mut payload, SECTION_PHASE2, 0, section.0);
    }
    if let Some(climate) = state.climate.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(climate.moisture_milli.len() as u64);
        for &value in &climate.moisture_milli {
            section.i64(value);
        }
        // The biome map is stored state, not a derived field: it is a
        // classification cached on a cadence, so recomputing it on load
        // gives a different map and the restored world diverges.
        section.u64(climate.biome.len() as u64);
        for biome in &climate.biome {
            section.u8(*biome as u8);
        }
        section.i128(climate.capacity_loss_milli);
        write_section(&mut payload, SECTION_CLIMATE, 0, section.0);
    }
    if let Some(contest) = state.contest.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(contest.health_milli.len() as u64);
        for index in 0..contest.health_milli.len() {
            section.i64(contest.health_milli[index]);
            section.i64(contest.recent_damage_milli[index]);
        }
        section.u64(contest.carcasses.len() as u64);
        for carcass in &contest.carcasses {
            section.u64(carcass.id);
            section.i32(carcass.x_fp);
            section.i32(carcass.y_fp);
            section.i64(carcass.energy_milli);
            section.u64(carcass.created_tick);
        }
        section.i128(contest.carcass_created_milli);
        section.i128(contest.carcass_consumed_milli);
        section.i128(contest.carcass_decayed_milli);
        section.u64(contest.attacks_total);
        section.i128(contest.damage_dealt_milli);
        section.u64(contest.deaths_by_damage_total);
        section.i128(contest.healed_milli);
        write_section(&mut payload, SECTION_CONTEST, 0, section.0);
    }
    if let Some(physiology) = state.physiology.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(physiology.cumulative_hazard_q16.len() as u64);
        for &hazard in &physiology.cumulative_hazard_q16 {
            section.i64(hazard);
        }
        section.u64(physiology.deaths_senescence_total);
        section.u64(physiology.deaths_extrinsic_total);
        section.u64(physiology.deaths_juvenile_total);
        section.i128(physiology.thermal_cost_milli);
        section.i128(physiology.allometric_cost_milli);
        write_section(&mut payload, SECTION_PHYSIOLOGY, 0, section.0);
    }
    if let Some(schema2) = state.schema2.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(schema2.genomes.len() as u64);
        for index in 0..schema2.genomes.len() {
            let genome = &schema2.genomes[index];
            section.u32(genome.len() as u32);
            section.0.extend_from_slice(genome);
            let values = &schema2.activation_values[index];
            let prior = &schema2.activation_prior[index];
            section.u32(values.len() as u32);
            for value in values {
                section.u32(value.to_bits());
            }
            for value in prior {
                section.u32(value.to_bits());
            }
            section.u32(schema2.activation_faults[index]);
        }
        // **Destructured rather than field-accessed, so the compiler fails
        // this when a counter is added.** The previous form was a list of
        // eleven `counters.x` reads, and two counters were added without it:
        // they were dropped on save, and since the counters are hashed into
        // the state checksum, a restored world's checksum silently differed
        // from the one it was saved from. An exhaustive destructuring with
        // no `..` cannot be left behind that way.
        let sim_core::MutationCounters {
            point_applied,
            duplication_applied,
            deletion_applied,
            insertion_applied,
            transposition_applied,
            binding_applied,
            rejected_homology_collision,
            rejected_orphaned,
            rejected_min_nodes,
            rejected_no_bindings,
            rejected_cap,
            rejected_inapplicable,
            rejected_cycle,
            rejected_invalid,
        } = schema2.counters;
        for value in [
            point_applied,
            duplication_applied,
            deletion_applied,
            insertion_applied,
            transposition_applied,
            rejected_homology_collision,
            rejected_orphaned,
            rejected_min_nodes,
            rejected_no_bindings,
            rejected_cap,
            rejected_inapplicable,
            rejected_cycle,
            rejected_invalid,
        ] {
            section.u64(value);
        }
        // Format 7's counter, written after the thirteen that came before it
        // and only at format 7 or later, so a format-6 body is byte-identical
        // to what format 6 wrote. The retained format-6 writer refuses a
        // nonzero value before reaching here.
        if format >= FORMAT_VERSION_7 {
            section.u64(binding_applied);
        }
        write_section(&mut payload, SECTION_SCHEMA2, 0, section.0);
    }
    if let Some(morphology) = state.morphology.as_ref() {
        let mut section = Writer(Vec::new());
        // Exhaustive destructuring with no `..`, so adding a counter fails
        // this line rather than silently dropping it on save (D-077).
        let sim_core::DevelopCounters {
            bodies_grown,
            modules_placed,
            differentiations,
            scale_changes,
            refused_occupied,
            refused_out_of_bounds,
            refused_max_modules,
            refused_node_budget,
            nonviable_empty,
            nonviable_disconnected,
            nonviable_missing_type,
            nonviable_other,
        } = morphology.counters;
        for value in [
            bodies_grown,
            modules_placed,
            differentiations,
            scale_changes,
            refused_occupied,
            refused_out_of_bounds,
            refused_max_modules,
            refused_node_budget,
            nonviable_empty,
            nonviable_disconnected,
            nonviable_missing_type,
            nonviable_other,
        ] {
            section.u64(value);
        }
        write_section(&mut payload, SECTION_MORPHOLOGY, 0, section.0);
    }
    if let Some(learn) = state.learn.as_ref() {
        let mut section = Writer(Vec::new());
        // **The organism count, never the plastic-edge count.** This is
        // D-076's trap and the Phase 2 section carries the scar of it: that
        // loop was driven by `traits.len()`, which is the organism count in a
        // schema-1 world and zero in a schema-2 world, so a schema-2 snapshot
        // encoded no per-organism records at all. The mirror image here is a
        // low-plasticity world - the *expected* outcome of the phase under
        // `E-stationary`, where plasticity is predicted to be selected down -
        // in which every organism has zero plastic edges. Framing that by
        // edges would write a section that says "no organisms" rather than
        // "no plastic edges", and the per-organism fault counts would go with
        // it. Each organism writes its own count instead.
        let organisms = learn.edges.len() as u64;
        section.u64(organisms);
        for index in 0..learn.edges.len() {
            let row = &learn.edges[index];
            section.u32(row.len() as u32);
            for edge in row {
                // Destructured with no `..` so a field added to the record
                // fails here rather than being dropped on save (D-077).
                let sim_core::LearnedEdgeSave {
                    edge_homology_id,
                    learned_q16,
                    trace_q16,
                } = *edge;
                section.u32(edge_homology_id);
                section.i32(learned_q16);
                section.i32(trace_q16);
            }
            section.u32(learn.faults[index]);
            section.u32(learn.cost_remainder[index]);
        }
        // Exhaustive destructuring with no `..`, for the reason the schema-2
        // block states: these counters are hashed into the state checksum, so
        // a counter dropped on save makes a restored world's checksum differ
        // from the one it was saved from with nothing to point at.
        let sim_core::PlasticityCounters {
            updates_applied,
            updates_static,
            updates_refused,
            faults,
            clamped,
            trace_clamped,
        } = learn.counters;
        for value in [
            updates_applied,
            updates_static,
            updates_refused,
            faults,
            clamped,
            trace_clamped,
        ] {
            section.u64(value);
        }
        section.i128(learn.cost_milli);
        write_section(&mut payload, SECTION_LEARN, 0, section.0);
    }
    if let Some(worldmod) = state.worldmod.as_ref() {
        let (flags, body) = encode_worldmod(worldmod, state);
        write_section(&mut payload, SECTION_WORLDMOD, flags, body);
    }
    if let Some(census) = state.action_census.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(census.counts.len() as u64);
        for row in &census.counts {
            for value in row {
                section.u32(*value);
            }
        }
        // Exhaustive destructuring with no `..`, for the reason every
        // counter block in this function states: these are hashed into the
        // state checksum, so a counter dropped on save makes a restored
        // world's checksum differ from the one it was saved from with
        // nothing to point at. `resets_total` is the one that would be
        // easiest to lose and the most damaging to lose, because a world
        // whose rows are all zero looks identical either way.
        let sim_core::ActionCensusCounters {
            classified_total,
            resets_total,
        } = census.counters;
        section.u64(classified_total);
        section.u64(resets_total);
        write_section(&mut payload, SECTION_ACTION_CENSUS, 0, section.0);
    }
    if let Some(objects) = state.objects.as_ref() {
        write_section(&mut payload, SECTION_OBJECTS, 0, encode_objects(objects));
    }
    if let Some(social) = state.social.as_ref() {
        write_section(&mut payload, SECTION_SOCIAL, 0, encode_social(social));
    }
    if let Some(ontogeny) = state.ontogeny.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(ontogeny.grown_modules.len() as u64);
        for &grown in &ontogeny.grown_modules {
            section.u32(grown);
        }
        for &paid in &ontogeny.growth_paid_milli {
            section.i64(paid);
        }
        section.u64(ontogeny.modules_grown_total);
        section.i128(ontogeny.growth_spent_milli_total);
        write_section(&mut payload, SECTION_ONTOGENY, 0, section.0);
    }
    if let Some(matechoice) = state.matechoice.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(matechoice.choices_total);
        section.u64(matechoice.scrambled_choices_total);
        write_section(&mut payload, SECTION_MATECHOICE, 0, section.0);
    }
    if let Some(chemistry) = state.chemistry.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(chemistry.concentrations.len() as u64);
        for &value in &chemistry.concentrations {
            section.i64(value);
        }
        section.i128(chemistry.produced_milli);
        section.i128(chemistry.deposited_milli);
        section.i128(chemistry.seeded_out_milli);
        section.u64(chemistry.abiogenesis_fired_total);
        write_section(&mut payload, SECTION_CHEMISTRY, 0, section.0);
    }
    if let Some(microbial) = state.microbial.as_ref() {
        let mut section = Writer(Vec::new());
        section.u64(microbial.densities.len() as u64);
        for &value in &microbial.densities {
            section.i64(value);
        }
        section.i128(microbial.grown_milli_total);
        section.i128(microbial.died_milli_total);
        section.i128(microbial.mutated_milli_total);
        write_section(&mut payload, SECTION_MICROBIAL, 0, section.0);
    }
    payload
}

/// Encode the social table. Exhaustive destructuring with no `..` (D-077):
/// every field here is hashed into the state checksum, so a field dropped on
/// save makes a restored world's checksum differ with nothing to point at.
fn encode_social(table: &sim_core::SocialTable) -> Vec<u8> {
    let mut section = Writer(Vec::new());
    let sim_core::SocialTable {
        committed_field_q16,
        prior_contact,
        prior_object_delta_q16,
        emission_remainder_milli,
        counters,
    } = table;
    section.u64(committed_field_q16.len() as u64);
    for &value in committed_field_q16 {
        section.i32(value);
    }
    section.u64(prior_contact.len() as u64);
    for &value in prior_contact {
        section.u8(u8::from(value));
    }
    for &value in prior_object_delta_q16 {
        section.i32(value);
    }
    for &value in emission_remainder_milli {
        section.i64(value);
    }
    let sim_core::SocialCounters {
        signals_emitted_total,
        signal_cost_milli_total,
        perception_faults_total,
        corruption_draws_total,
        scrambled_deliveries_total,
        rule5_updates_total,
    } = *counters;
    section.u64(signals_emitted_total);
    section.u64(signal_cost_milli_total);
    section.u64(perception_faults_total);
    section.u64(corruption_draws_total);
    section.u64(scrambled_deliveries_total);
    section.u64(rule5_updates_total);
    section.0
}

fn decode_social(reader: &mut Reader) -> Result<sim_core::SocialTable, CodecError> {
    let field_len = reader.u64()?;
    // Four bytes per field value; the bound is a floor on what the body must
    // hold, never a guess at what it does hold (D-075, D-091).
    if !allocation_fits(field_len, 4, 0, reader.remaining()) {
        return Err(CodecError::ValueOutOfRange("social field count"));
    }
    let mut committed_field_q16 = Vec::with_capacity(field_len as usize);
    for _ in 0..field_len {
        committed_field_q16.push(reader.i32()?);
    }
    let population = reader.u64()?;
    // One contact byte, one delta word, one remainder word per organism.
    if !allocation_fits(population, 1 + 4 + 8, 0, reader.remaining()) {
        return Err(CodecError::ValueOutOfRange("social organism count"));
    }
    let mut prior_contact = Vec::with_capacity(population as usize);
    for _ in 0..population {
        let value = reader.u8()?;
        if value > 1 {
            return Err(CodecError::ValueOutOfRange("social contact flag"));
        }
        prior_contact.push(value != 0);
    }
    let mut prior_object_delta_q16 = Vec::with_capacity(population as usize);
    for _ in 0..population {
        prior_object_delta_q16.push(reader.i32()?);
    }
    let mut emission_remainder_milli = Vec::with_capacity(population as usize);
    for _ in 0..population {
        emission_remainder_milli.push(reader.i64()?);
    }
    let counters = sim_core::SocialCounters {
        signals_emitted_total: reader.u64()?,
        signal_cost_milli_total: reader.u64()?,
        perception_faults_total: reader.u64()?,
        corruption_draws_total: reader.u64()?,
        scrambled_deliveries_total: reader.u64()?,
        rule5_updates_total: reader.u64()?,
    };
    Ok(sim_core::SocialTable {
        committed_field_q16,
        prior_contact,
        prior_object_delta_q16,
        emission_remainder_milli,
        counters,
    })
}

/// Encode the object table. Reads every field through `ObjectTable::record`,
/// whose exhaustive `ObjectRecord` literal is what makes a field added to the
/// table fail to compile here rather than fall out of the save (D-077).
fn encode_objects(table: &sim_core::ObjectTable) -> Vec<u8> {
    let mut section = Writer(Vec::new());
    section.u64(table.len() as u64);
    for index in 0..table.len() {
        let sim_core::ObjectRecord {
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
        } = table.record(index);
        section.u64(id);
        section.u16(material_id);
        section.i32(x_fp);
        section.i32(y_fp);
        section.i32(integrity_q16);
        section.i64(mass_milli);
        section.i64(energy_milli);
        section.u32(hardness_q16);
        section.u32(durability_q16);
        section.u32(decay_q16);
        section.u64(holder_id);
        section.u64(owner_id);
        section.u8(depth);
        section.u64(created_tick);
        section.u64(creator_id);
        section.u8(cause);
        section.u64(parent_id);
        section.u64(composition.len() as u64);
        for constituent in composition {
            section.u64(constituent);
        }
    }
    section.u64(table.objects_allocated_total);
    for value in table.ledger.to_array() {
        section.i128(value);
    }
    for value in table.counters.to_array() {
        section.u64(value);
    }
    // Per-organism observations, population-long, after the table proper.
    section.u64(table.exposure_ticks.len() as u64);
    for index in 0..table.exposure_ticks.len() {
        section.u64(table.exposure_ticks[index]);
        section.u64(table.carry_ticks[index]);
        section.u8(table.birth_band[index]);
    }
    section.0
}

/// Decode the object table, every count bounded before its allocation.
fn decode_objects(reader: &mut Reader) -> Result<sim_core::ObjectTable, CodecError> {
    let count = reader.u64()?;
    // `OBJECT_FIXED_BYTES` already includes the composition-length word, so
    // it is the exact minimum one object occupies; a per-item bound above the
    // real minimum would refuse a *legitimate* table once its trailer (408
    // bytes) could no longer cover the overcount, which a first draft did at
    // 51 objects. The bound is a floor on what the body must hold, never a
    // guess at what it does hold (D-075).
    if !allocation_fits(count, OBJECT_FIXED_BYTES, 0, reader.remaining()) {
        return Err(CodecError::ValueOutOfRange("object count"));
    }
    let mut table = sim_core::ObjectTable::default();
    let mut last_id = 0_u64;
    for _ in 0..count {
        let id = reader.u64()?;
        // Ascending is a decode-time invariant: `ObjectTable::push` asserts
        // it in debug builds, and a table that arrives out of order is
        // refused here by name rather than tripping that assertion.
        if id <= last_id {
            return Err(CodecError::ValueOutOfRange("object ids not ascending"));
        }
        last_id = id;
        let material_id = reader.u16()?;
        let x_fp = reader.i32()?;
        let y_fp = reader.i32()?;
        let integrity_q16 = reader.i32()?;
        let mass_milli = reader.i64()?;
        let energy_milli = reader.i64()?;
        let hardness_q16 = reader.u32()?;
        let durability_q16 = reader.u32()?;
        let decay_q16 = reader.u32()?;
        let holder_id = reader.u64()?;
        let owner_id = reader.u64()?;
        let depth = reader.u8()?;
        let created_tick = reader.u64()?;
        let creator_id = reader.u64()?;
        let cause = reader.u8()?;
        let parent_id = reader.u64()?;
        let breadth = reader.u64()?;
        if !allocation_fits(breadth, 8, 0, reader.remaining()) {
            return Err(CodecError::ValueOutOfRange("object composition length"));
        }
        let mut composition = Vec::with_capacity(breadth as usize);
        for _ in 0..breadth {
            composition.push(reader.u64()?);
        }
        table.push(sim_core::ObjectRecord {
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
        });
    }
    table.objects_allocated_total = reader.u64()?;
    let mut ledger = [0_i128; sim_core::ObjectLedger::FIELD_COUNT];
    for slot in &mut ledger {
        *slot = reader.i128()?;
    }
    table.ledger = sim_core::ObjectLedger::from_array(ledger);
    let mut counters = [0_u64; sim_core::ObjectCounters::FIELD_COUNT];
    for slot in &mut counters {
        *slot = reader.u64()?;
    }
    table.counters = sim_core::ObjectCounters::from_array(counters);
    let organisms = reader.u64()?;
    if !allocation_fits(organisms, 8 + 8 + 1, 0, reader.remaining()) {
        return Err(CodecError::ValueOutOfRange("object observation rows"));
    }
    for _ in 0..organisms {
        table.exposure_ticks.push(reader.u64()?);
        table.carry_ticks.push(reader.u64()?);
        let band = reader.u8()?;
        if band > 4 {
            return Err(CodecError::ValueOutOfRange("birth band"));
        }
        table.birth_band.push(band);
    }
    Ok(table)
}

/// Encode the terrain modification section, choosing sparse or dense per
/// layer, and return the flags word that says which was chosen.
///
/// # Both representations exist because the spec requires both, and they are
/// required to be indistinguishable after a restore
///
/// The sparse form is a sorted `(cell, value)` list per layer; the dense form
/// is one `i64` per cell with `WORLDMOD_DENSE_ABSENT` marking "no override".
/// They decode to the same `TerrainModState` and therefore to the same world
/// and the same composed checksum - that is C12.5's representation-
/// equivalence clause, and it is asserted rather than assumed.
///
/// # The threshold, and a measured caveat on its default
///
/// The choice is `layer_len / cell_count > dense_threshold_q16 / 65536`,
/// evaluated exactly in `u128` so a large map cannot overflow the numerator.
/// It is versioned config rather than a constant here because the
/// specification requires the representation to be recorded rather than
/// guessed, and a threshold living in the encoder would be a silent format
/// parameter.
///
/// **The shipped default is below the byte-for-byte crossover and that is
/// worth stating rather than hiding.** A sparse entry costs 12 bytes and a
/// dense cell costs 8, so dense is smaller only past 8/12 = 2/3 occupancy,
/// while `worldmod_default()` sets the threshold at 1/2 with a comment
/// computing the crossover from a 13-byte entry - the flat layout that
/// carried its own layer id, before the per-layer blocks made that byte
/// redundant. At the default a layer between 1/2 and 2/3 occupancy therefore
/// encodes dense and slightly larger. It is a size choice and never a
/// correctness one, both arms round-trip identically, and the number belongs
/// to `sim-core`'s config rather than to this crate; measured figures are in
/// `tests/bench_phase12_snapshot.rs`.
fn encode_worldmod(worldmod: &TerrainModState, state: &SaveState) -> (u16, Vec<u8>) {
    // The map's cell count, taken from the biomass field rather than from the
    // config's `cells_x * cells_y`: `from_state` already validates that array
    // against the regenerated terrain, so this is the one length in the save
    // that is checked against the world rather than declared by it.
    let cell_count = state.biomass_milli.len() as u64;
    let threshold = u128::from(state.config.worldmod.dense_threshold_q16);
    let mut flags = 0_u16;
    let mut writer = Writer(Vec::new());
    for layer in 0..LAYER_COUNT {
        let range = worldmod.layer_range(layer);
        let dense = cell_count > 0
            && u128::from(range.len() as u64) * 65_536 > threshold * u128::from(cell_count);
        if dense {
            flags |= SECTION_FLAG_DENSE_LAYER0 << layer;
            writer.u64(cell_count);
            let mut cursor = range.start;
            for cell in 0..cell_count as u32 {
                while cursor < range.end && worldmod.cells[cursor] < cell {
                    cursor += 1;
                }
                if cursor < range.end && worldmod.cells[cursor] == cell {
                    writer.i64(worldmod.values[cursor]);
                } else {
                    writer.i64(WORLDMOD_DENSE_ABSENT);
                }
            }
        } else {
            writer.u64(range.len() as u64);
            for index in range {
                writer.u32(worldmod.cells[index]);
                writer.i64(worldmod.values[index]);
            }
        }
    }
    writer.i128(worldmod.capacity_loss_milli);
    // Exhaustive destructuring with no `..`, for the reason the schema-2 and
    // learn blocks give: these counters are hashed into the state checksum,
    // so one dropped on save makes a restored world's checksum differ from
    // the one it was saved from with nothing in the file to point at.
    let TerrainModCounters {
        writes_inserted,
        writes_replaced,
        writes_cleared,
        writes_no_change,
        refused_cap,
        refused_occupied,
        refused_invalid,
        relocations,
        cells_trimmed,
    } = worldmod.counters;
    for value in [
        writes_inserted,
        writes_replaced,
        writes_cleared,
        writes_no_change,
        refused_cap,
        refused_occupied,
        refused_invalid,
        relocations,
        cells_trimmed,
    ] {
        writer.u64(value);
    }
    (flags, writer.0)
}

/// Decode the terrain modification section into the flat sorted arrays the
/// kernel keeps it in.
///
/// Cross-layer ordering is a property of this loop - layers are read in
/// ascending id and appended in order - so only ordering *within* a layer can
/// come from the file. It is not checked here: `World::from_state` runs
/// `order_violation` and `bounds_violation` over the whole set before it
/// reaches a world, and duplicating the check would be a second copy of the
/// ordering rule to keep in step with `terrainmod.rs`. What is checked here
/// is framing: every declared count is capped against the section body before
/// anything is allocated (D-075), and the count is never asserted to equal a
/// field count.
fn decode_worldmod(
    reader: &mut Reader,
    flags: u16,
    body_len: usize,
) -> Result<TerrainModState, CodecError> {
    let mut state = TerrainModState::default();
    for layer in 0..LAYER_COUNT {
        let dense = flags & (SECTION_FLAG_DENSE_LAYER0 << layer) != 0;
        let declared = reader.u64()?;
        if dense {
            if !allocation_fits(declared, WORLDMOD_DENSE_BYTES_PER_CELL, 0, body_len) {
                return Err(CodecError::ValueOutOfRange("worldmod dense cells"));
            }
            if declared > u64::from(u32::MAX) {
                return Err(CodecError::ValueOutOfRange("worldmod dense cell index"));
            }
            for cell in 0..declared as u32 {
                let value = reader.i64()?;
                // The sentinel is checked before the domain, because "absent"
                // is deliberately outside every layer's domain and would
                // otherwise be refused as an illegal value.
                if value == WORLDMOD_DENSE_ABSENT {
                    continue;
                }
                state.layers.push(layer);
                state.cells.push(cell);
                state.values.push(value);
            }
        } else {
            if !allocation_fits(declared, WORLDMOD_SPARSE_BYTES_PER_ENTRY, 0, body_len) {
                return Err(CodecError::ValueOutOfRange("worldmod sparse entries"));
            }
            state.layers.reserve(declared as usize);
            state.cells.reserve(declared as usize);
            state.values.reserve(declared as usize);
            for _ in 0..declared {
                state.layers.push(layer);
                state.cells.push(reader.u32()?);
                state.values.push(reader.i64()?);
            }
        }
    }
    state.capacity_loss_milli = reader.i128()?;
    let mut counters = TerrainModCounters::default();
    for slot in [
        &mut counters.writes_inserted,
        &mut counters.writes_replaced,
        &mut counters.writes_cleared,
        &mut counters.writes_no_change,
        &mut counters.refused_cap,
        &mut counters.refused_occupied,
        &mut counters.refused_invalid,
        &mut counters.relocations,
        &mut counters.cells_trimmed,
    ] {
        *slot = reader.u64()?;
    }
    state.counters = counters;
    Ok(state)
}

/// Decode a payload written under framing version `format`.
///
/// The version gates *which sections may appear*, not how any of them is
/// parsed. A format 3 file that carries a format 4 section is refused rather
/// than read leniently: leniency there would mean a file whose header says 3
/// and whose body says 4 loads as whichever the reader felt like, and the
/// whole point of a registry is that nothing migrates implicitly.
fn decode_payload(bytes: &[u8], format: u16, state_checksum: u64) -> Result<SaveState, CodecError> {
    let mut offset = 0_usize;
    let mut config = None;
    let mut climate: Option<sim_core::ClimateSaveState> = None;
    let mut physiology: Option<sim_core::PhysiologySaveState> = None;
    let mut schema2: Option<sim_core::Schema2SaveState> = None;
    let mut morphology: Option<sim_core::MorphologySaveState> = None;
    let mut learn: Option<sim_core::LearnSaveState> = None;
    let mut contest: Option<sim_core::ContestSaveState> = None;
    let mut worldmod: Option<TerrainModState> = None;
    let mut action_census: Option<sim_core::ActionCensusSaveState> = None;
    let mut objects: Option<sim_core::ObjectTable> = None;
    let mut social: Option<sim_core::SocialTable> = None;
    let mut ontogeny: Option<sim_core::OntogenySave> = None;
    let mut matechoice: Option<sim_core::MateChoiceSave> = None;
    let mut chemistry: Option<sim_core::ChemistrySave> = None;
    let mut microbial: Option<sim_core::MicrobialSave> = None;
    type WorldMeta = (u64, bool, bool, u64, u64, Option<u64>);
    let mut meta: Option<WorldMeta> = None;
    type OrganismColumns = (Vec<u64>, Vec<i32>, Vec<i32>, Vec<i64>, Vec<u64>, Vec<u64>);
    let mut organisms: Option<OrganismColumns> = None;
    let mut biomass = None;
    let mut ledger_counters: Option<(Ledger, sim_core::Counters)> = None;
    let mut phase2 = None;

    while offset < bytes.len() {
        if bytes.len() - offset < 12 {
            return Err(CodecError::TruncatedSection);
        }
        let tag = u16::from_le_bytes(bytes[offset..offset + 2].try_into().unwrap());
        // **Validated, not bound to `_` and forgotten.** Every section has
        // carried this word since format 1 and no reader had ever looked at
        // it, so any value was silently accepted - a fail-open that cost
        // nothing only because nothing had ever written one. It carries the
        // modification section's per-layer representation now, so it has to
        // mean exactly what the tag says it can mean.
        let section_flags = u16::from_le_bytes(bytes[offset + 2..offset + 4].try_into().unwrap());
        if section_flags & !section_flags_allowed(tag) != 0 {
            return Err(CodecError::UnknownSectionFlags {
                tag,
                flags: section_flags,
            });
        }
        let length = u64::from_le_bytes(bytes[offset + 4..offset + 12].try_into().unwrap());
        if length > MAX_SECTION_LEN {
            return Err(CodecError::ValueOutOfRange("section length"));
        }
        let length = length as usize;
        let body_start = offset + 12;
        let body_end = body_start
            .checked_add(length)
            .ok_or(CodecError::TruncatedSection)?;
        if bytes.len() < body_end + 4 {
            return Err(CodecError::TruncatedSection);
        }
        let body = &bytes[body_start..body_end];
        let declared = u32::from_le_bytes(bytes[body_end..body_end + 4].try_into().unwrap());
        if declared != crc32(body) {
            return Err(CodecError::SectionChecksumMismatch(tag));
        }
        offset = body_end + 4;

        let mut reader = Reader {
            bytes: body,
            offset: 0,
        };
        match tag {
            SECTION_CONFIG => {
                if config.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                config = Some(decode_config(&mut reader, format)?);
            }
            SECTION_WORLD_META => {
                if meta.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let tick = reader.u64()?;
                let paused = reader.u8()? != 0;
                let extinct = reader.u8()? != 0;
                let next_entity_id = reader.u64()?;
                let terrain_checksum = reader.u64()?;
                // The composed terrain checksum, present only in a snapshot
                // that carries a modification section. Absent is not a
                // default: it is resolved to the baseline below, which is
                // provably the composed value of an empty set, and the
                // resolution is then verified by `World::from_state` like any
                // other. A body with any other number of spare bytes is
                // rejected by the trailing-bytes check every section runs.
                let composed = if reader.remaining() >= 8 {
                    Some(reader.u64()?)
                } else {
                    None
                };
                meta = Some((
                    tick,
                    paused,
                    extinct,
                    next_entity_id,
                    terrain_checksum,
                    composed,
                ));
            }
            SECTION_ORGANISMS => {
                if organisms.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                const RECORD: u64 = 8 + 4 + 4 + 8 + 8 + 8;
                // Exact-size check before allocation.
                if count
                    .checked_mul(RECORD)
                    .and_then(|body_len| body_len.checked_add(8))
                    != Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("organism count"));
                }
                let count = count as usize;
                let mut ids = Vec::with_capacity(count);
                let mut x_fp = Vec::with_capacity(count);
                let mut y_fp = Vec::with_capacity(count);
                let mut energy = Vec::with_capacity(count);
                let mut ages = Vec::with_capacity(count);
                let mut cooldowns = Vec::with_capacity(count);
                for _ in 0..count {
                    ids.push(reader.u64()?);
                    x_fp.push(reader.i32()?);
                    y_fp.push(reader.i32()?);
                    energy.push(reader.i64()?);
                    ages.push(reader.u64()?);
                    cooldowns.push(reader.u64()?);
                }
                organisms = Some((ids, x_fp, y_fp, energy, ages, cooldowns));
            }
            SECTION_BIOMASS => {
                if biomass.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                if count
                    .checked_mul(8)
                    .and_then(|body_len| body_len.checked_add(8))
                    != Some(body.len() as u64)
                {
                    return Err(CodecError::ValueOutOfRange("biomass count"));
                }
                let count = count as usize;
                let mut cells = Vec::with_capacity(count);
                for _ in 0..count {
                    cells.push(reader.i64()?);
                }
                biomass = Some(cells);
            }
            SECTION_LEDGER => {
                if ledger_counters.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let ledger = Ledger {
                    initial_energy_milli: reader.i128()?,
                    assimilated_milli: reader.i128()?,
                    spent_milli: reader.i128()?,
                    removed_at_death_milli: reader.i128()?,
                    initial_biomass_milli: reader.i128()?,
                    grown_milli: reader.i128()?,
                    consumed_biomass_milli: reader.i128()?,
                };
                let counters = sim_core::Counters {
                    births_total: reader.u64()?,
                    deaths_starvation_total: reader.u64()?,
                    deaths_old_age_total: reader.u64()?,
                    capacity_rejections_total: reader.u64()?,
                    dropped_events_total: reader.u64()?,
                };
                ledger_counters = Some((ledger, counters));
            }
            SECTION_PHASE2 => {
                if phase2.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                let flat_genomes = reader.u64()?;
                // Cap the declared counts against the body before allocating,
                // exactly as the climate section does. This was an equality
                // against `8 + 7 * 8` - the count word plus one word per
                // Phase 2 counter - and adding an eighth counter broke every
                // snapshot in the build, which is the same way the climate
                // check broke when its section grew. A bound is what the
                // fail-closed rule actually needs; exactness is still
                // enforced, by the trailing-bytes check every section runs at
                // the end, and that check needs no editing when a field is
                // added.
                let organism_len = (4 * 4 + 2 + 8 + 4 + 16 + 4 + 4 + 8) as u64;
                let flat_len = ((TRAIT_COUNT + sim_core::NEURAL_COUNT) * 4) as u64;
                let declared = organisms
                    .checked_mul(organism_len)
                    .and_then(|bytes| bytes.checked_add(flat_genomes.checked_mul(flat_len)?));
                if declared > Some(body.len() as u64) || declared.is_none() {
                    return Err(CodecError::ValueOutOfRange("phase2 count"));
                }
                // A flat-genome count that is neither zero nor the organism
                // count cannot describe any world: schema 1 carries one per
                // organism and schema 2 carries none.
                if flat_genomes != 0 && flat_genomes != organisms {
                    return Err(CodecError::ValueOutOfRange("phase2 flat genome count"));
                }
                let organisms = organisms as usize;
                let flat_genomes = flat_genomes as usize;
                let mut section = Phase2SaveState {
                    traits: Vec::with_capacity(flat_genomes),
                    neural: Vec::with_capacity(flat_genomes),
                    memory: Vec::with_capacity(organisms),
                    heading_bam: Vec::with_capacity(organisms),
                    speed_milli: Vec::with_capacity(organisms),
                    last_turn: Vec::with_capacity(organisms),
                    parents: Vec::with_capacity(organisms),
                    depth: Vec::with_capacity(organisms),
                    child_count: Vec::with_capacity(organisms),
                    birth_tick: Vec::with_capacity(organisms),
                    counters: Default::default(),
                };
                for _ in 0..flat_genomes {
                    let mut traits = [0.0_f32; TRAIT_COUNT];
                    for gene in traits.iter_mut() {
                        *gene = reader.f32()?;
                    }
                    let mut neural = Vec::with_capacity(sim_core::NEURAL_COUNT);
                    for _ in 0..sim_core::NEURAL_COUNT {
                        neural.push(reader.f32()?);
                    }
                    section.traits.push(traits);
                    section.neural.push(neural);
                }
                for _ in 0..organisms {
                    let mut memory = [0.0_f32; 4];
                    for value in memory.iter_mut() {
                        *value = reader.f32()?;
                    }
                    section.memory.push(memory);
                    section.heading_bam.push(reader.u16()?);
                    section.speed_milli.push(reader.i64()?);
                    section.last_turn.push(reader.f32()?);
                    section.parents.push([reader.u64()?, reader.u64()?]);
                    section.depth.push(reader.u32()?);
                    section.child_count.push(reader.u32()?);
                    section.birth_tick.push(reader.u64()?);
                }
                section.counters = sim_core::Phase2Counters {
                    paired_births_total: reader.u64()?,
                    pair_rejected_capacity_total: reader.u64()?,
                    pair_rejected_placement_total: reader.u64()?,
                    pair_rejected_energy_total: reader.u64()?,
                    pair_rejected_nonviable_total: reader.u64()?,
                    controller_faults_total: reader.u64()?,
                    mutated_trait_genes_total: reader.u64()?,
                    mutated_neural_genes_total: reader.u64()?,
                };
                phase2 = Some(section);
            }
            SECTION_CLIMATE => {
                if climate.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let count = reader.u64()?;
                // Cap the declared length against the section body before
                // any allocation. A *cap* rather than an exact-length
                // equality: the biome map that follows also contributes
                // bytes, and an exact check here would have to be edited
                // every time the section gains a field -- which is how it
                // broke when it did. Exactness is still enforced, by the
                // trailing-bytes check every section runs at the end.
                if !allocation_fits(count, 8, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("climate count"));
                }
                let mut moisture_milli = Vec::with_capacity(count as usize);
                for _ in 0..count {
                    moisture_milli.push(reader.i64()?);
                }
                let biome_count = reader.u64()?;
                if biome_count > body.len() as u64 {
                    return Err(CodecError::ValueOutOfRange("climate biome cells"));
                }
                let mut biome = Vec::with_capacity(biome_count as usize);
                for _ in 0..biome_count {
                    biome.push(
                        sim_core::Biome::from_id(reader.u8()?)
                            .ok_or(CodecError::ValueOutOfRange("climate biome id"))?,
                    );
                }
                climate = Some(sim_core::ClimateSaveState {
                    moisture_milli,
                    biome,
                    capacity_loss_milli: reader.i128()?,
                });
            }
            SECTION_CONTEST => {
                if contest.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Cap before allocating: 16 bytes per organism plus the
                // carcass count that follows.
                if !allocation_fits(organisms, 16, 8, body.len()) {
                    return Err(CodecError::ValueOutOfRange("contest organisms"));
                }
                let mut health_milli = Vec::with_capacity(organisms as usize);
                let mut recent_damage_milli = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    health_milli.push(reader.i64()?);
                    recent_damage_milli.push(reader.i64()?);
                }
                let carcass_count = reader.u64()?;
                if !allocation_fits(carcass_count, 32, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("contest carcasses"));
                }
                let mut carcasses = Vec::with_capacity(carcass_count as usize);
                for _ in 0..carcass_count {
                    carcasses.push(sim_core::Carcass {
                        id: reader.u64()?,
                        x_fp: reader.i32()?,
                        y_fp: reader.i32()?,
                        energy_milli: reader.i64()?,
                        created_tick: reader.u64()?,
                    });
                }
                contest = Some(sim_core::ContestSaveState {
                    health_milli,
                    recent_damage_milli,
                    carcasses,
                    carcass_created_milli: reader.i128()?,
                    carcass_consumed_milli: reader.i128()?,
                    carcass_decayed_milli: reader.i128()?,
                    attacks_total: reader.u64()?,
                    damage_dealt_milli: reader.i128()?,
                    deaths_by_damage_total: reader.u64()?,
                    healed_milli: reader.i128()?,
                });
            }
            SECTION_PHYSIOLOGY => {
                if physiology.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Cap before allocating: 8 bytes per organism.
                if !allocation_fits(organisms, 8, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("physiology organisms"));
                }
                let mut cumulative_hazard_q16 = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    cumulative_hazard_q16.push(reader.i64()?);
                }
                physiology = Some(sim_core::PhysiologySaveState {
                    cumulative_hazard_q16,
                    deaths_senescence_total: reader.u64()?,
                    deaths_extrinsic_total: reader.u64()?,
                    deaths_juvenile_total: reader.u64()?,
                    thermal_cost_milli: reader.i128()?,
                    allometric_cost_milli: reader.i128()?,
                });
            }
            SECTION_SCHEMA2 => {
                if schema2.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Each organism contributes at least a length word, so a
                // count beyond the section body is refused before anything
                // is allocated.
                if !allocation_fits(organisms, 4, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("schema2 organisms"));
                }
                let mut genomes = Vec::with_capacity(organisms as usize);
                let mut activation_values = Vec::with_capacity(organisms as usize);
                let mut activation_prior = Vec::with_capacity(organisms as usize);
                let mut activation_faults = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    let length = reader.u32()?;
                    if length as u64 > body.len() as u64 {
                        return Err(CodecError::ValueOutOfRange("schema2 genome length"));
                    }
                    let mut genome = Vec::with_capacity(length as usize);
                    for _ in 0..length {
                        genome.push(reader.u8()?);
                    }
                    genomes.push(genome);
                    let nodes = reader.u32()?;
                    if !allocation_fits(u64::from(nodes), 8, 0, body.len()) {
                        return Err(CodecError::ValueOutOfRange("schema2 activation length"));
                    }
                    let mut values = Vec::with_capacity(nodes as usize);
                    for _ in 0..nodes {
                        values.push(f32::from_bits(reader.u32()?));
                    }
                    let mut prior = Vec::with_capacity(nodes as usize);
                    for _ in 0..nodes {
                        prior.push(f32::from_bits(reader.u32()?));
                    }
                    activation_values.push(values);
                    activation_prior.push(prior);
                    activation_faults.push(reader.u32()?);
                }
                let mut counters = sim_core::MutationCounters::default();
                for slot in [
                    &mut counters.point_applied,
                    &mut counters.duplication_applied,
                    &mut counters.deletion_applied,
                    &mut counters.insertion_applied,
                    &mut counters.transposition_applied,
                    &mut counters.rejected_homology_collision,
                    &mut counters.rejected_orphaned,
                    &mut counters.rejected_min_nodes,
                    &mut counters.rejected_no_bindings,
                    &mut counters.rejected_cap,
                    &mut counters.rejected_inapplicable,
                    &mut counters.rejected_cycle,
                    &mut counters.rejected_invalid,
                ] {
                    *slot = reader.u64()?;
                }
                if format >= FORMAT_VERSION_7 {
                    counters.binding_applied = reader.u64()?;
                }
                schema2 = Some(sim_core::Schema2SaveState {
                    genomes,
                    activation_values,
                    activation_prior,
                    activation_faults,
                    counters,
                });
            }
            SECTION_MORPHOLOGY => {
                if morphology.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let mut counters = sim_core::DevelopCounters::default();
                for slot in [
                    &mut counters.bodies_grown,
                    &mut counters.modules_placed,
                    &mut counters.differentiations,
                    &mut counters.scale_changes,
                    &mut counters.refused_occupied,
                    &mut counters.refused_out_of_bounds,
                    &mut counters.refused_max_modules,
                    &mut counters.refused_node_budget,
                    &mut counters.nonviable_empty,
                    &mut counters.nonviable_disconnected,
                    &mut counters.nonviable_missing_type,
                    &mut counters.nonviable_other,
                ] {
                    *slot = reader.u64()?;
                }
                morphology = Some(sim_core::MorphologySaveState { counters });
            }
            SECTION_LEARN => {
                if learn.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // **A cap, never an equality, and never an encoded field
                // count** (D-075). Every organism contributes at least its
                // count word and its fault word, so a declared count beyond
                // what the body could hold is refused before anything is
                // allocated. Exactness is enforced by the trailing-bytes
                // check every section runs at the end, which needs no editing
                // when this section gains a field - which is how the climate
                // and Phase 2 equality checks broke when theirs did.
                if !allocation_fits(organisms, LEARN_MIN_PER_ORGANISM, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("learn organisms"));
                }
                let mut edges = Vec::with_capacity(organisms as usize);
                let mut faults = Vec::with_capacity(organisms as usize);
                let mut cost_remainder = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    let plastic = u64::from(reader.u32()?);
                    if !allocation_fits(plastic, LEARN_BYTES_PER_EDGE, 0, body.len()) {
                        return Err(CodecError::ValueOutOfRange("learn plastic edges"));
                    }
                    let mut row = Vec::with_capacity(plastic as usize);
                    for _ in 0..plastic {
                        row.push(sim_core::LearnedEdgeSave {
                            edge_homology_id: reader.u32()?,
                            learned_q16: reader.i32()?,
                            trace_q16: reader.i32()?,
                        });
                    }
                    edges.push(row);
                    faults.push(reader.u32()?);
                    cost_remainder.push(reader.u32()?);
                }
                let mut counters = sim_core::PlasticityCounters::default();
                for slot in [
                    &mut counters.updates_applied,
                    &mut counters.updates_static,
                    &mut counters.updates_refused,
                    &mut counters.faults,
                    &mut counters.clamped,
                    &mut counters.trace_clamped,
                ] {
                    *slot = reader.u64()?;
                }
                // Nothing here checks that a learned value is inside its
                // clamp or that an edge id matches a plan. That is deliberate
                // and not an omission: the codec's job is framing, and the
                // meaning of these numbers lives in `World::from_state`,
                // which recompiles the plans and refuses a mismatch. A bounds
                // check duplicated here would be a second copy of the range
                // to keep in step with `plasticity.rs`.
                learn = Some(sim_core::LearnSaveState {
                    edges,
                    faults,
                    cost_remainder,
                    counters,
                    cost_milli: reader.i128()?,
                });
            }
            SECTION_WORLDMOD => {
                // **`FORMAT_VERSION_4`, not `FORMAT_VERSION`.** This section
                // arrived *in* format 4, so the version it must be compared
                // against is the one that introduced it and not whichever
                // version happens to be current. Written as `< FORMAT_VERSION`
                // it read correctly for exactly as long as format 4 was
                // current, and the moment format 5 landed it would have
                // refused this section in every format-4 file - which is every
                // campaign artifact on disk - with an error naming the section
                // rather than the comparison.
                if format < FORMAT_VERSION_4 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if worldmod.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                worldmod = Some(decode_worldmod(&mut reader, section_flags, body.len())?);
            }
            SECTION_ACTION_CENSUS => {
                // `FORMAT_VERSION_4` for the reason the worldmod arm above
                // gives: the constant names the format that introduced the
                // section, permanently, not the format that is current.
                if format < FORMAT_VERSION_4 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if action_census.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // **A cap, never an equality, and never an encoded field
                // count** (D-075). A declared count beyond what the body
                // could hold is refused before anything is allocated;
                // exactness is enforced by the trailing-bytes check every
                // section runs at the end, which needs no editing when this
                // section gains a field.
                if !allocation_fits(organisms, ACTION_CENSUS_BYTES_PER_ORGANISM, 0, body.len()) {
                    return Err(CodecError::ValueOutOfRange("action census organisms"));
                }
                let mut counts = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    let mut row = [0_u32; sim_core::ACTION_CLASS_COUNT];
                    for slot in &mut row {
                        *slot = reader.u32()?;
                    }
                    counts.push(row);
                }
                action_census = Some(sim_core::ActionCensusSaveState {
                    counts,
                    counters: sim_core::ActionCensusCounters {
                        classified_total: reader.u64()?,
                        resets_total: reader.u64()?,
                    },
                });
            }
            SECTION_OBJECTS => {
                // `FORMAT_VERSION_7` by name: the format that introduced the
                // section, permanently (D-108).
                if format < FORMAT_VERSION_7 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if objects.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                objects = Some(decode_objects(&mut reader)?);
            }
            SECTION_SOCIAL => {
                // `FORMAT_VERSION_8` by name: the format that introduced the
                // section, permanently (D-108).
                if format < FORMAT_VERSION_8 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if social.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                social = Some(decode_social(&mut reader)?);
            }
            SECTION_ONTOGENY => {
                // `FORMAT_VERSION_9` by name: the format that introduced the
                // section, permanently (D-108).
                if format < FORMAT_VERSION_9 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if ontogeny.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let organisms = reader.u64()?;
                // Cap before allocating: 12 bytes per organism plus the two
                // trailing counters (D-091's discipline).
                if !allocation_fits(organisms, 12, 24, body.len()) {
                    return Err(CodecError::ValueOutOfRange("ontogeny organisms"));
                }
                let mut grown_modules = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    grown_modules.push(reader.u32()?);
                }
                let mut growth_paid_milli = Vec::with_capacity(organisms as usize);
                for _ in 0..organisms {
                    growth_paid_milli.push(reader.i64()?);
                }
                ontogeny = Some(sim_core::OntogenySave {
                    grown_modules,
                    growth_paid_milli,
                    modules_grown_total: reader.u64()?,
                    growth_spent_milli_total: reader.i128()?,
                });
            }
            SECTION_MATECHOICE => {
                // `FORMAT_VERSION_10` by name: the format that introduced
                // the section, permanently (D-108).
                if format < FORMAT_VERSION_10 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if matechoice.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                matechoice = Some(sim_core::MateChoiceSave {
                    choices_total: reader.u64()?,
                    scrambled_choices_total: reader.u64()?,
                });
            }
            SECTION_CHEMISTRY => {
                // `FORMAT_VERSION_11` by name (D-108).
                if format < FORMAT_VERSION_11 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if chemistry.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let values = reader.u64()?;
                // Cap before allocating: 8 bytes per value plus the four
                // trailing ledger terms (D-091's discipline).
                if !allocation_fits(values, 8, 56, body.len()) {
                    return Err(CodecError::ValueOutOfRange("chemistry values"));
                }
                let mut concentrations = Vec::with_capacity(values as usize);
                for _ in 0..values {
                    concentrations.push(reader.i64()?);
                }
                chemistry = Some(sim_core::ChemistrySave {
                    concentrations,
                    produced_milli: reader.i128()?,
                    deposited_milli: reader.i128()?,
                    seeded_out_milli: reader.i128()?,
                    abiogenesis_fired_total: reader.u64()?,
                });
            }
            SECTION_MICROBIAL => {
                // `FORMAT_VERSION_12` by name (D-108).
                if format < FORMAT_VERSION_12 {
                    return Err(CodecError::SectionNotInFormat { tag, format });
                }
                if microbial.is_some() {
                    return Err(CodecError::DuplicateSection(tag));
                }
                let values = reader.u64()?;
                // Cap before allocating: 8 bytes per value plus the three
                // trailing counter terms (D-091's discipline).
                if !allocation_fits(values, 8, 48, body.len()) {
                    return Err(CodecError::ValueOutOfRange("microbial values"));
                }
                let mut densities = Vec::with_capacity(values as usize);
                for _ in 0..values {
                    densities.push(reader.i64()?);
                }
                microbial = Some(sim_core::MicrobialSave {
                    densities,
                    grown_milli_total: reader.i128()?,
                    died_milli_total: reader.i128()?,
                    mutated_milli_total: reader.i128()?,
                });
            }
            unknown => return Err(CodecError::UnknownSection(unknown)),
        }
        if !reader.done() {
            return Err(CodecError::ValueOutOfRange("section trailing bytes"));
        }
    }

    let config = config.ok_or(CodecError::MissingSection(SECTION_CONFIG))?;
    let (tick, paused, extinct, next_entity_id, terrain_checksum, composed) =
        meta.ok_or(CodecError::MissingSection(SECTION_WORLD_META))?;
    let (ids, x_fp, y_fp, energy_milli, age_ticks, cooldown_ticks) =
        organisms.ok_or(CodecError::MissingSection(SECTION_ORGANISMS))?;
    let biomass_milli = biomass.ok_or(CodecError::MissingSection(SECTION_BIOMASS))?;
    let (ledger, counters) = ledger_counters.ok_or(CodecError::MissingSection(SECTION_LEDGER))?;

    let _ = state_checksum; // verified by the restore path via World::from_state
    Ok(SaveState {
        config,
        tick,
        paused,
        extinct,
        next_entity_id,
        terrain_checksum,
        composed_terrain_checksum: composed.unwrap_or(terrain_checksum),
        worldmod,
        action_census,
        objects,
        social,
        ontogeny,
        matechoice,
        chemistry,
        microbial,
        ids,
        x_fp,
        y_fp,
        energy_milli,
        age_ticks,
        cooldown_ticks,
        biomass_milli,
        ledger,
        counters,
        phase2,
        climate,
        contest,
        physiology,
        schema2,
        morphology,
        learn,
    })
}

/// Encode a snapshot. `compression_level`: None = uncompressed, Some(level)
/// = zstd at that level.
pub fn encode_snapshot(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 3** snapshot.
///
/// Kept in the build permanently, alongside `decode_snapshot_format3`, for
/// the reason `specifications/world-save-format.md` gives: the acceptance
/// requirement for the 3-to-4 migration is byte identity against a real
/// legacy file, so a legacy file has to be constructible. Nothing in the
/// engine calls this outside migration tests, and it refuses to write a state
/// that carries a modification section, because a format 3 file cannot
/// express one and silently dropping it would be the "never alter meaning"
/// rule broken on the write side.
pub fn encode_snapshot_format3(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    if state.worldmod.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_WORLDMOD,
            format: FORMAT_VERSION_3,
        });
    }
    // Same refusal, same reason: a format 3 file cannot express an action
    // census, and silently dropping one on the way out is the "never alter
    // meaning" rule broken on the write side rather than the read side.
    if state.action_census.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_ACTION_CENSUS,
            format: FORMAT_VERSION_3,
        });
    }
    refuse_format8_state(state, FORMAT_VERSION_3)?;
    refuse_format9_state(state, FORMAT_VERSION_3)?;
    refuse_format10_state(state, FORMAT_VERSION_3)?;
    refuse_format12_state(state, FORMAT_VERSION_3)?;
    refuse_format11_state(state, FORMAT_VERSION_3)?;
    refuse_format7_state(state, FORMAT_VERSION_3)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_3,
        SAVE_STATE_VERSION_3,
    )
}

/// Encode a **format 4** snapshot.
///
/// Kept in the build permanently alongside `decode_snapshot_format4`, for the
/// reason `encode_snapshot_format3` is: the acceptance requirement for the
/// 4-to-5 migration is byte identity against a real legacy file, so a legacy
/// file has to be constructible, and the 120 format-4 campaign artifacts are
/// not in the repository.
///
/// It refuses a state whose `plasticity.live_rule_zero` is set, because a
/// format 4 file has no byte for it and writing one anyway would produce a
/// file describing a world with rule 0 dead - the same class of silent
/// meaning change the format 3 writer refuses for the worldmod section, and
/// the one this whole format bump exists to prevent.
///
/// The save-state version is `SAVE_STATE_VERSION`, not a `_4` constant, and
/// that is deliberate: format 5 adds a config *field* and changes no existing
/// field's meaning, so the logical state version does not move. Version 2
/// was bumped when terrain stopped being a pure function of `(seed, config)`,
/// which is a change of meaning; this is not one. Phase 11 set the precedent
/// in the other direction by adding four config fields and a whole optional
/// section without moving it.
pub fn encode_snapshot_format4(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    if state.config.plasticity.live_rule_zero {
        return Err(CodecError::FieldNotInFormat {
            field: "plasticity.live_rule_zero",
            format: FORMAT_VERSION_4,
        });
    }
    refuse_format8_state(state, FORMAT_VERSION_4)?;
    refuse_format9_state(state, FORMAT_VERSION_4)?;
    refuse_format10_state(state, FORMAT_VERSION_4)?;
    refuse_format12_state(state, FORMAT_VERSION_4)?;
    refuse_format11_state(state, FORMAT_VERSION_4)?;
    refuse_format7_state(state, FORMAT_VERSION_4)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_4,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 5** snapshot.
///
/// Retained on the same terms as `encode_snapshot_format3` and
/// `encode_snapshot_format4`: the acceptance requirement for the 5-to-6
/// migration is byte identity against a real legacy file, and no `.alif` file
/// exists in the repository, so a legacy file has to be constructible.
///
/// It refuses a state whose `plasticity.price_moved_edges_only` is set,
/// because a format 5 file has no byte for it and writing one anyway would
/// produce a file describing a world that prices every flagged edge - a
/// different experiment, silently.
pub fn encode_snapshot_format5(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    if state.config.plasticity.price_moved_edges_only {
        return Err(CodecError::FieldNotInFormat {
            field: "plasticity.price_moved_edges_only",
            format: FORMAT_VERSION_5,
        });
    }
    refuse_format8_state(state, FORMAT_VERSION_5)?;
    refuse_format9_state(state, FORMAT_VERSION_5)?;
    refuse_format10_state(state, FORMAT_VERSION_5)?;
    refuse_format12_state(state, FORMAT_VERSION_5)?;
    refuse_format11_state(state, FORMAT_VERSION_5)?;
    refuse_format7_state(state, FORMAT_VERSION_5)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_5,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 6** snapshot.
///
/// Retained on the same terms as every earlier writer: the acceptance
/// requirement for the 6-to-7 migration is byte identity against a real
/// legacy file, and a legacy file has to be constructible.
///
/// It refuses a state carrying anything format 7 added, because a format-6
/// file has no bytes for any of it and writing one anyway would describe a
/// world without objects, without a `bind` operator, and without the
/// bindings that operator inserted - a different experiment, silently.
pub fn encode_snapshot_format6(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format8_state(state, FORMAT_VERSION_6)?;
    refuse_format9_state(state, FORMAT_VERSION_6)?;
    refuse_format10_state(state, FORMAT_VERSION_6)?;
    refuse_format12_state(state, FORMAT_VERSION_6)?;
    refuse_format11_state(state, FORMAT_VERSION_6)?;
    refuse_format7_state(state, FORMAT_VERSION_6)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_6,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 7** snapshot.
///
/// Retained on the same terms as every earlier writer: the acceptance
/// requirement for the 7-to-8 migration is byte identity against a real
/// legacy file, and a legacy file has to be constructible. It refuses a
/// state carrying anything format 8 added, because a format-7 file has no
/// bytes for the social section and writing one anyway would describe a
/// world whose organisms cannot perceive one another - a different
/// experiment, silently.
pub fn encode_snapshot_format7(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format8_state(state, FORMAT_VERSION_7)?;
    refuse_format9_state(state, FORMAT_VERSION_7)?;
    refuse_format10_state(state, FORMAT_VERSION_7)?;
    refuse_format12_state(state, FORMAT_VERSION_7)?;
    refuse_format11_state(state, FORMAT_VERSION_7)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_7,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 8** snapshot.
///
/// Retained on the same terms as every earlier writer: the acceptance
/// requirement for the 8-to-9 migration is byte identity against a real
/// legacy file, and a legacy file has to be constructible. It refuses a
/// state carrying anything format 9 added, because a format-8 file has no
/// bytes for the ontogeny section and writing one anyway would describe a
/// world whose juveniles were never juveniles - a different experiment,
/// silently.
pub fn encode_snapshot_format8(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format9_state(state, FORMAT_VERSION_8)?;
    refuse_format10_state(state, FORMAT_VERSION_8)?;
    refuse_format12_state(state, FORMAT_VERSION_8)?;
    refuse_format11_state(state, FORMAT_VERSION_8)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_8,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 9** snapshot, retained on the terms every earlier
/// writer is: the 9-to-10 migration's byte-identity requirement is stated
/// against it. It refuses a state carrying anything format 10 added.
pub fn encode_snapshot_format9(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format10_state(state, FORMAT_VERSION_9)?;
    refuse_format12_state(state, FORMAT_VERSION_9)?;
    refuse_format11_state(state, FORMAT_VERSION_9)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_9,
        SAVE_STATE_VERSION,
    )
}

/// Encode a **format 10** snapshot, retained on the terms every earlier
/// writer is: the 10-to-11 migration's byte-identity requirement is
/// stated against it.
pub fn encode_snapshot_format10(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format12_state(state, FORMAT_VERSION_10)?;
    refuse_format11_state(state, FORMAT_VERSION_10)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_10,
        SAVE_STATE_VERSION,
    )
}

/// The write-side refusal every retained pre-8 writer shares: a state that
/// carries what only format 8 can express is refused with the field named,
/// before a byte is written. The whole struct is compared against its
/// default rather than only the gate, because a knob moved off its default
/// with the section disabled is still a value the format has no bytes for,
/// and restoring it at the default would alter meaning on load.
/// Encode a **format 11** snapshot, retained on the terms every earlier
/// writer is: the 11-to-12 migration's byte-identity requirement is
/// stated against it.
pub fn encode_snapshot_format11(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
) -> Result<Vec<u8>, CodecError> {
    refuse_format12_state(state, FORMAT_VERSION_11)?;
    encode_snapshot_versioned(
        state,
        world_id,
        parent_world_id,
        state_checksum,
        build_version,
        event_log_offset,
        compression_level,
        FORMAT_VERSION_11,
        SAVE_STATE_VERSION,
    )
}

/// The write-side refusal every retained pre-12 writer shares, on the
/// terms `refuse_format9_state` is: the microbial fields compared field
/// by field against their defaults (the rest of the chemistry struct is
/// format 11's and legitimately non-default there), and the densities
/// section refused by name.
fn refuse_format12_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    let defaults = sim_core::ChemistryConfig::chemistry_default();
    let chemistry = &state.config.chemistry;
    if chemistry.microbial_enabled != defaults.microbial_enabled
        || chemistry.replication_axis != defaults.replication_axis
        || chemistry.aggregation_axis != defaults.aggregation_axis
        || chemistry.growth_rate_low_q16 != defaults.growth_rate_low_q16
        || chemistry.growth_rate_high_q16 != defaults.growth_rate_high_q16
        || chemistry.growth_yield_q16 != defaults.growth_yield_q16
        || chemistry.death_q16 != defaults.death_q16
        || chemistry.death_waste_fraction_q16 != defaults.death_waste_fraction_q16
        || chemistry.mutation_q16 != defaults.mutation_q16
    {
        return Err(CodecError::FieldNotInFormat {
            field: "chemistry microbial",
            format,
        });
    }
    if state.microbial.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_MICROBIAL,
            format,
        });
    }
    Ok(())
}

/// The write-side refusal every retained pre-11 writer shares, on the
/// terms its predecessors are. The whole-struct comparison predates the
/// format-12 microbial fields; `refuse_format12_state` names those first
/// wherever both run, so this arm answers for format 11's own fields.
fn refuse_format11_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    if state.config.chemistry != sim_core::ChemistryConfig::chemistry_default() {
        return Err(CodecError::FieldNotInFormat {
            field: "chemistry",
            format,
        });
    }
    if state.chemistry.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_CHEMISTRY,
            format,
        });
    }
    Ok(())
}

/// The write-side refusal every retained pre-10 writer shares, on the
/// terms `refuse_format9_state` is: the two mate-choice gates compared
/// field by field, and the counters section refused by name.
fn refuse_format10_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    if state.config.physiology.mate_choice_enabled || state.config.physiology.mate_choice_scramble
    {
        return Err(CodecError::FieldNotInFormat {
            field: "physiology mate choice",
            format,
        });
    }
    if state.matechoice.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_MATECHOICE,
            format,
        });
    }
    Ok(())
}

/// The write-side refusal every retained pre-9 writer shares: a state that
/// carries what only format 9 can express is refused with the field or
/// section named, before a byte is written. The ontogeny knobs are compared
/// field by field against their documented defaults rather than the whole
/// physiology struct, because the rest of that struct existed long before
/// format 9 and is legitimately non-default in older files.
fn refuse_format9_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    let defaults = sim_core::PhysiologyConfig::physiology_default();
    let physiology = &state.config.physiology;
    if physiology.ontogeny_enabled != defaults.ontogeny_enabled
        || physiology.birth_modules_min != defaults.birth_modules_min
        || physiology.growth_cost_milli_per_mass_milli
            != defaults.growth_cost_milli_per_mass_milli
        || physiology.growth_rate_milli_per_s != defaults.growth_rate_milli_per_s
    {
        return Err(CodecError::FieldNotInFormat {
            field: "physiology ontogeny",
            format,
        });
    }
    if state.ontogeny.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_ONTOGENY,
            format,
        });
    }
    Ok(())
}

fn refuse_format8_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    if state.config.social != sim_core::SocialConfig::social_default() {
        return Err(CodecError::FieldNotInFormat {
            field: "social",
            format,
        });
    }
    if state.social.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_SOCIAL,
            format,
        });
    }
    Ok(())
}

/// The write-side refusals every retained pre-7 writer shares: a state that
/// carries what only format 7 can express is refused with the field or
/// section named, before a byte is written.
fn refuse_format7_state(state: &SaveState, format: u16) -> Result<(), CodecError> {
    if state.config.artifact.enabled
        || state.config.artifact.inert
        || state.config.artifact.ephemeral
    {
        return Err(CodecError::FieldNotInFormat {
            field: "artifact.enabled",
            format,
        });
    }
    if state.config.genome2.mutation.binding_q16 != 0 {
        return Err(CodecError::FieldNotInFormat {
            field: "genome2.mutation.binding_q16",
            format,
        });
    }
    if state
        .schema2
        .as_ref()
        .is_some_and(|schema2| schema2.counters.binding_applied != 0)
    {
        return Err(CodecError::FieldNotInFormat {
            field: "schema2.counters.binding_applied",
            format,
        });
    }
    if state.objects.is_some() {
        return Err(CodecError::SectionNotInFormat {
            tag: SECTION_OBJECTS,
            format,
        });
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn encode_snapshot_versioned(
    state: &SaveState,
    world_id: u64,
    parent_world_id: u64,
    state_checksum: u64,
    build_version: &str,
    event_log_offset: u64,
    compression_level: Option<i32>,
    format_version: u16,
    save_state_version: u16,
) -> Result<Vec<u8>, CodecError> {
    let build = build_version.as_bytes();
    if build.len() > MAX_BUILD_LEN {
        return Err(CodecError::BuildStringTooLong(build.len()));
    }
    let payload = encode_payload(state, format_version);
    let uncompressed_len = payload.len() as u64;
    let (stored, flags) = match compression_level {
        Some(level) => (
            zstd::bulk::compress(&payload, level).map_err(|_| CodecError::DecompressionFailed)?,
            FLAG_ZSTD,
        ),
        None => (payload, 0),
    };
    let payload_crc = crc32(&stored);

    let mut out = Writer(Vec::with_capacity(HEADER_LEN + build.len() + stored.len()));
    out.0.extend_from_slice(SNAPSHOT_MAGIC);
    out.u16(format_version);
    out.u16(HEADER_LEN as u16);
    out.u32(flags);
    out.u64(world_id);
    out.u64(parent_world_id);
    out.u64(state.tick);
    out.u64(state.config.world_seed);
    out.u64(state.config.stable_hash());
    out.u16(save_state_version);
    out.u16(GENOME_SCHEMA_VERSION);
    out.u16(build.len() as u16);
    out.u16(0);
    out.u64(event_log_offset);
    out.u64(uncompressed_len);
    out.u64(stored.len() as u64);
    out.u32(payload_crc);
    out.u64(state_checksum);
    out.u64(state.terrain_checksum);
    // Pad the fixed header to HEADER_LEN.
    while out.0.len() < HEADER_LEN {
        out.u8(0);
    }
    debug_assert_eq!(out.0.len(), HEADER_LEN);
    out.0.extend_from_slice(build);
    out.0.extend_from_slice(&stored);
    Ok(out.0)
}

/// The framing version a file claims, without validating it.
///
/// The one read that is allowed to see a version this build does not decode,
/// because it is what the migration registry is consulted with. `read_info`
/// refuses an unsupported version by design, which is correct and is also why
/// `migration_for` was **unreachable** from `lifesim verify-save` before this
/// existed: the CLI called `read_info` first, so an old file failed with
/// `UnsupportedFormat` and the registry it was about to consult never ran.
pub fn peek_format_version(bytes: &[u8]) -> Result<u16, CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::TooShort);
    }
    if &bytes[0..4] != SNAPSHOT_MAGIC {
        return Err(CodecError::BadMagic);
    }
    Ok(u16::from_le_bytes(bytes[4..6].try_into().unwrap()))
}

/// Parse and validate only the header (cheap integrity/provenance check).
pub fn read_info(bytes: &[u8]) -> Result<SnapshotInfo, CodecError> {
    read_info_versioned(bytes, FORMAT_VERSION, SAVE_STATE_VERSION)
}

fn read_info_versioned(
    bytes: &[u8],
    expected_format: u16,
    expected_save_state: u16,
) -> Result<SnapshotInfo, CodecError> {
    if bytes.len() < HEADER_LEN {
        return Err(CodecError::TooShort);
    }
    if &bytes[0..4] != SNAPSHOT_MAGIC {
        return Err(CodecError::BadMagic);
    }
    let mut reader = Reader {
        bytes: &bytes[4..HEADER_LEN],
        offset: 0,
    };
    let format_version = reader.u16()?;
    if format_version != expected_format {
        return Err(CodecError::UnsupportedFormat(format_version));
    }
    let header_len = usize::from(reader.u16()?);
    if header_len != HEADER_LEN {
        return Err(CodecError::BadHeaderLength(header_len));
    }
    let flags = reader.u32()?;
    if flags & !FLAG_ZSTD != 0 {
        return Err(CodecError::UnknownFlags(flags));
    }
    let world_id = reader.u64()?;
    let parent_world_id = reader.u64()?;
    let tick = reader.u64()?;
    let seed = reader.u64()?;
    let config_hash = reader.u64()?;
    let save_state_version = reader.u16()?;
    if save_state_version != expected_save_state {
        return Err(CodecError::UnsupportedSaveState(save_state_version));
    }
    let genome_schema_version = reader.u16()?;
    if genome_schema_version != GENOME_SCHEMA_VERSION {
        return Err(CodecError::UnsupportedGenomeSchema(genome_schema_version));
    }
    let build_len = usize::from(reader.u16()?);
    let _reserved = reader.u16()?;
    if build_len > MAX_BUILD_LEN {
        return Err(CodecError::BuildStringTooLong(build_len));
    }
    let event_log_offset = reader.u64()?;
    let uncompressed_len = reader.u64()?;
    if uncompressed_len > MAX_UNCOMPRESSED_LEN {
        return Err(CodecError::UncompressedTooLarge(uncompressed_len));
    }
    let stored_len = reader.u64()?;
    if stored_len > MAX_STORED_LEN {
        return Err(CodecError::StoredTooLarge(stored_len));
    }
    let payload_crc = reader.u32()?;
    let state_checksum = reader.u64()?;
    let terrain_checksum = reader.u64()?;

    let expected_total = HEADER_LEN + build_len + stored_len as usize;
    if bytes.len() != expected_total {
        return Err(CodecError::LengthMismatch {
            expected: expected_total,
            actual: bytes.len(),
        });
    }
    let build_version = std::str::from_utf8(&bytes[HEADER_LEN..HEADER_LEN + build_len])
        .map_err(|_| CodecError::ValueOutOfRange("build string"))?
        .to_owned();
    let stored = &bytes[HEADER_LEN + build_len..];
    if crc32(stored) != payload_crc {
        return Err(CodecError::PayloadChecksumMismatch);
    }
    Ok(SnapshotInfo {
        format_version,
        compressed: flags & FLAG_ZSTD != 0,
        world_id,
        parent_world_id,
        tick,
        seed,
        config_hash,
        save_state_version,
        genome_schema_version,
        build_version,
        event_log_offset,
        uncompressed_len,
        stored_len,
        state_checksum,
        terrain_checksum,
    })
}

/// Full decode to logical state (header validation included).
pub fn decode_snapshot(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION, SAVE_STATE_VERSION)
}

/// Full decode of a **format 3** snapshot.
///
/// Permanent, not transitional. This is the reader the 3-to-4 migration's
/// byte-identity requirement is stated against
/// (`specifications/world-save-format.md`), and it stays in the build for as
/// long as that requirement does - which is forever, because the alternative
/// is a migration whose correctness rests on the assertion that it was
/// correct on the day it was written.
///
/// It produces the current `SaveState` type with the two Phase 12 fields
/// resolved the only way a format 3 file allows: no modification section, and
/// a composed checksum equal to the baseline. That is not an invention of
/// missing data - it is the identity an empty modification set satisfies, and
/// `World::from_state` re-derives and verifies it rather than trusting it.
pub fn decode_snapshot_format3(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_3, SAVE_STATE_VERSION_3)
}

/// Full decode of a **format 4** snapshot.
///
/// Permanent, not transitional, for the reason `decode_snapshot_format3` is:
/// this is the reader the 4-to-5 migration's byte-identity requirement is
/// stated against, and it stays in the build for as long as that requirement
/// does.
///
/// It is a real reader rather than a wrapper that strips a byte. The single
/// difference is threaded through `decode_payload` to `decode_config`, which
/// is the one place the format is expressed, so this function cannot drift
/// away from what a format-4 file actually contains. What it produces is the
/// current `SaveState` with `plasticity.live_rule_zero` at `false` - not an
/// invention, but the value every world that could write a format-4 file
/// actually ran with, since rule 0 was a no-op in all of them.
pub fn decode_snapshot_format4(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_4, SAVE_STATE_VERSION)
}

/// Full decode of a **format 5** snapshot.
///
/// Permanent, for the reason `decode_snapshot_format4` is: it is the reader
/// the 5-to-6 migration's byte-identity requirement is stated against. What
/// it produces is the current `SaveState` with
/// `plasticity.price_moved_edges_only` at `false` - the value every world
/// that could write a format-5 file actually ran with.
pub fn decode_snapshot_format5(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_5, SAVE_STATE_VERSION)
}

/// Full decode of a **format 6** snapshot.
///
/// Permanent, for the reason `decode_snapshot_format5` is: it is the reader
/// the 6-to-7 migration's byte-identity requirement is stated against. What
/// it produces is the current `SaveState` with no artifact section, no
/// object table, `binding_q16` zero and `binding_applied` zero - the values
/// every world that could write a format-6 file actually ran with.
pub fn decode_snapshot_format6(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_6, SAVE_STATE_VERSION)
}

/// Decode a **format 7** snapshot. Retained for the 7-to-8 migration, on the
/// terms every earlier retained reader is.
pub fn decode_snapshot_format7(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_7, SAVE_STATE_VERSION)
}

/// Decode a **format 8** snapshot. Retained for the 8-to-9 migration, on the
/// terms every earlier retained reader is.
pub fn decode_snapshot_format8(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_8, SAVE_STATE_VERSION)
}

/// Decode a **format 9** snapshot. Retained for the 9-to-10 migration, on
/// the terms every earlier retained reader is.
pub fn decode_snapshot_format9(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_9, SAVE_STATE_VERSION)
}

/// Decode a **format 10** snapshot. Retained for the 10-to-11 migration,
/// on the terms every earlier retained reader is.
pub fn decode_snapshot_format10(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_10, SAVE_STATE_VERSION)
}

pub fn decode_snapshot_format11(bytes: &[u8]) -> Result<(SnapshotInfo, SaveState), CodecError> {
    decode_snapshot_versioned(bytes, FORMAT_VERSION_11, SAVE_STATE_VERSION)
}

fn decode_snapshot_versioned(
    bytes: &[u8],
    expected_format: u16,
    expected_save_state: u16,
) -> Result<(SnapshotInfo, SaveState), CodecError> {
    let info = read_info_versioned(bytes, expected_format, expected_save_state)?;
    let stored = &bytes[HEADER_LEN + info.build_version.len()..];
    let payload = if info.compressed {
        let decompressed = zstd::bulk::decompress(stored, info.uncompressed_len as usize)
            .map_err(|_| CodecError::DecompressionFailed)?;
        if decompressed.len() as u64 != info.uncompressed_len {
            return Err(CodecError::DecompressedLengthMismatch {
                declared: info.uncompressed_len,
                actual: decompressed.len(),
            });
        }
        decompressed
    } else {
        if stored.len() as u64 != info.uncompressed_len {
            return Err(CodecError::DecompressedLengthMismatch {
                declared: info.uncompressed_len,
                actual: stored.len(),
            });
        }
        stored.to_vec()
    };
    let state = decode_payload(&payload, info.format_version, info.state_checksum)?;
    Ok((info, state))
}
