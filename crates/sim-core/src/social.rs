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
