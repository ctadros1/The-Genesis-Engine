//! Versioned channel and activation registries (Phase 9).
//!
//! Schema 1 hard-codes 20 inputs and 12 outputs, so **every new world
//! capability is a genome schema bump performed by a human**. That is the
//! specific thing which makes open-ended complexity impossible: the
//! structure can only change when we change it.
//!
//! A registry replaces that. An `IoBinding` locus connects one network node
//! to one registry channel; an organism binds any subset it likes, and
//! unbound channels are never gathered or requested and cost nothing. Adding
//! a capability later adds a registry entry and bumps
//! `CHANNEL_REGISTRY_VERSION`, which enters the config hash. It does **not**
//! bump the genome schema and does not invalidate a single existing
//! schema-2 genome, which simply has no binding to the new channel.
//!
//! Two rules keep that promise honest, and both are enforced here rather
//! than by convention:
//!
//! - **IDs are permanent.** A channel ID is never reused or renumbered, in
//!   exactly the way an `RngSystem` value or an event tag is never reused.
//!   Renumbering would silently repoint every existing binding at a
//!   different channel.
//! - **Unknown IDs fail closed.** A binding to a channel this build does not
//!   know is a decode error, never a skipped binding.
//!
//! The registry is populated with **exactly the channels that exist after
//! Phase 7** and nothing else, so Phase 9 measures the effect of structural
//! freedom and not the effect of new senses.

/// Registry version 1: the post-Phase-7 channel set. Enters the config hash
/// (as the version a world *offers*, see `SimConfig::channel_registry_version`),
/// so a world that declares one registry version can never be confused with
/// another.
///
/// **This constant did not move when the artifact half added channels, and
/// the reason is recorded rather than left to look like an oversight.** The
/// module doc above promises that growing the registry "does not invalidate
/// a single existing schema-2 genome"; the ALG2 codec stamps the version and
/// refuses a mismatch outright (`genome2.rs`), so bumping this would have
/// made every schema-2 genome on disk undecodable, and hashing it
/// unconditionally inside the genome2 config block would have moved the
/// Phase 9 and 11 fixtures. So the artifact channels are **version 2**, a
/// world's version is 2 only when its artifact section is enabled, and a
/// genome stamps the smallest version that covers its bindings (ADR-0028
/// section 7). Both promises now hold: no genome is invalidated, and a
/// world that offers more channels hashes differently.
pub const CHANNEL_REGISTRY_VERSION: u16 = 1;
/// Registry version 2: version 1 plus the eleven artifact channels.
pub const CHANNEL_REGISTRY_VERSION_ARTIFACT: u16 = 2;
pub const ACTIVATION_REGISTRY_VERSION: u16 = 1;

/// Whether the world supplies a channel to the organism or accepts one from
/// it. A binding's direction is a property of the channel, not of the
/// binding, so an organism cannot write to a sensor or read an action.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelDirection {
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ChannelEntry {
    pub id: u16,
    pub direction: ChannelDirection,
    pub name: &'static str,
}

const fn input(id: u16, name: &'static str) -> ChannelEntry {
    ChannelEntry {
        id,
        direction: ChannelDirection::Input,
        name,
    }
}

const fn output(id: u16, name: &'static str) -> ChannelEntry {
    ChannelEntry {
        id,
        direction: ChannelDirection::Output,
        name,
    }
}

/// Registry version 1: the post-Phase-7 channel set, one entry per channel
/// topology 1 carried, in one ID space so a binding names a channel rather
/// than an index into a direction-specific array.
///
/// IDs 1..=20 are the schema-1 inputs in their documented order and 101..=112
/// the schema-1 outputs, so the correspondence is checkable by eye. The gap
/// is deliberate: it leaves room to append inputs without interleaving them
/// with outputs, and appending is the only permitted change.
///
/// The four memory inputs and four memory outputs of topology 1 are
/// **deliberately absent**. Memory in schema 1 is a fixed-size register file
/// wired output-to-input by the kernel; under schema 2 the same thing is a
/// recurrent edge, which the organism evolves for itself. Keeping the
/// registers as channels would offer both and make it impossible to say
/// which one an organism used.
pub const CHANNELS: &[ChannelEntry] = &[
    input(1, "energy_fraction"),
    input(2, "health_fraction"),
    input(3, "age_fraction"),
    input(4, "food_gradient_x"),
    input(5, "food_gradient_y"),
    input(6, "terrain_suitability"),
    input(7, "nearest_organism_proximity"),
    input(8, "nearest_organism_relative_heading"),
    input(9, "local_crowding"),
    input(10, "local_threat"),
    input(11, "temperature_comfort"),
    input(12, "moisture_comfort"),
    input(13, "speed_fraction"),
    input(14, "turn_rate"),
    input(15, "reproductive_readiness"),
    input(16, "recent_damage_fraction"),
    output(101, "turn"),
    output(102, "throttle"),
    output(103, "eat"),
    output(104, "attack"),
    output(105, "rest"),
    output(106, "mate"),
    output(107, "follow"),
    output(108, "avoid"),
];

/// The channels registry version 2 adds (Phase 12 artifact half). Offered
/// only by a world whose artifact section is enabled; absent, unbindable and
/// undrawable everywhere else, so no world that existed before them can tell
/// they exist.
///
/// Inputs 17-22 are cues, not labels (review 1.3, ADR-0022 A3): heft is the
/// target's mass over the perceiver's own carry capacity and hardness is
/// over the registry maximum; no channel carries a material ID or a
/// composite depth. Outputs 113-117 are the five actions. **109-112 stay
/// unallocated forever**: the doc on `CHANNELS` reserves 101..=112 for
/// schema 1's outputs and excludes its four memory outputs from ever being
/// channels, and an ID that is ambiguous between "reserved" and "free" is
/// not worth four values.
pub const CHANNELS_V2: &[ChannelEntry] = &[
    input(17, "object_present"),
    input(18, "object_distance"),
    input(19, "object_bearing"),
    input(20, "object_heft"),
    input(21, "object_hardness"),
    input(22, "carried_load"),
    output(113, "pick_up"),
    output(114, "drop"),
    output(115, "place"),
    output(116, "strike"),
    output(117, "combine"),
];

pub const CHANNEL_OBJECT_PRESENT: u16 = 17;
pub const CHANNEL_OBJECT_DISTANCE: u16 = 18;
pub const CHANNEL_OBJECT_BEARING: u16 = 19;
pub const CHANNEL_OBJECT_HEFT: u16 = 20;
pub const CHANNEL_OBJECT_HARDNESS: u16 = 21;
pub const CHANNEL_CARRIED_LOAD: u16 = 22;
pub const CHANNEL_PICK_UP: u16 = 113;
pub const CHANNEL_DROP: u16 = 114;
pub const CHANNEL_PLACE: u16 = 115;
pub const CHANNEL_STRIKE: u16 = 116;
pub const CHANNEL_COMBINE: u16 = 117;

/// Registry version 3: version 2 plus the social channels (Phase 13,
/// ADR-0029).
pub const CHANNEL_REGISTRY_VERSION_SOCIAL: u16 = 3;

/// The channels registry version 3 adds. Offered only by a world whose
/// social section is enabled (which requires the artifact section, ADR-0029
/// section 1: the version scheme is a total order because a genome's stamp
/// is "the smallest version covering its bindings").
///
/// Inputs 23..=58 are nine cues for each of four neighbour slots, slot-major
/// - cues, never labels (ADR-0022 A3/A4): no action class, no genotype
/// distance, no material id (`neighbour_carried` is a load fraction, the
/// deviation ADR-0029 section 2 records). Inputs 59..=62 are the committed
/// signal field at the receiver's cell, one per channel. Outputs 118..=121
/// are signal emission amplitudes; the emitted value has no authored
/// meaning anywhere in the kernel.
pub const CHANNELS_V3: &[ChannelEntry] = &[
    input(23, "neighbour_present_0"),
    input(24, "neighbour_distance_0"),
    input(25, "neighbour_bearing_0"),
    input(26, "neighbour_motion_0"),
    input(27, "neighbour_contact_0"),
    input(28, "neighbour_object_delta_0"),
    input(29, "neighbour_carried_0"),
    input(30, "neighbour_scale_0"),
    input(31, "neighbour_health_0"),
    input(32, "neighbour_present_1"),
    input(33, "neighbour_distance_1"),
    input(34, "neighbour_bearing_1"),
    input(35, "neighbour_motion_1"),
    input(36, "neighbour_contact_1"),
    input(37, "neighbour_object_delta_1"),
    input(38, "neighbour_carried_1"),
    input(39, "neighbour_scale_1"),
    input(40, "neighbour_health_1"),
    input(41, "neighbour_present_2"),
    input(42, "neighbour_distance_2"),
    input(43, "neighbour_bearing_2"),
    input(44, "neighbour_motion_2"),
    input(45, "neighbour_contact_2"),
    input(46, "neighbour_object_delta_2"),
    input(47, "neighbour_carried_2"),
    input(48, "neighbour_scale_2"),
    input(49, "neighbour_health_2"),
    input(50, "neighbour_present_3"),
    input(51, "neighbour_distance_3"),
    input(52, "neighbour_bearing_3"),
    input(53, "neighbour_motion_3"),
    input(54, "neighbour_contact_3"),
    input(55, "neighbour_object_delta_3"),
    input(56, "neighbour_carried_3"),
    input(57, "neighbour_scale_3"),
    input(58, "neighbour_health_3"),
    input(59, "signal_in_0"),
    input(60, "signal_in_1"),
    input(61, "signal_in_2"),
    input(62, "signal_in_3"),
    output(118, "signal_emit_0"),
    output(119, "signal_emit_1"),
    output(120, "signal_emit_2"),
    output(121, "signal_emit_3"),
];

/// First neighbour-cue input ID; slot `k` cue `c` is
/// `CHANNEL_NEIGHBOUR_BASE + k * NEIGHBOUR_CUE_COUNT + c`.
pub const CHANNEL_NEIGHBOUR_BASE: u16 = 23;
/// Cues per neighbour slot, in the fixed order present, distance, bearing,
/// motion, contact, object_delta, carried, scale, health.
pub const NEIGHBOUR_CUE_COUNT: u16 = 9;
/// First `signal_in` input ID; channel `c` is this plus `c`.
pub const CHANNEL_SIGNAL_IN_BASE: u16 = 59;
/// First `signal_emit` output ID; channel `c` is this plus `c`.
pub const CHANNEL_SIGNAL_EMIT_BASE: u16 = 118;

/// The smallest registry version that offers `id`, or `None` for an id no
/// version knows.
pub fn channel_version(id: u16) -> Option<u16> {
    if CHANNELS.iter().any(|entry| entry.id == id) {
        Some(CHANNEL_REGISTRY_VERSION)
    } else if CHANNELS_V2.iter().any(|entry| entry.id == id) {
        Some(CHANNEL_REGISTRY_VERSION_ARTIFACT)
    } else if CHANNELS_V3.iter().any(|entry| entry.id == id) {
        Some(CHANNEL_REGISTRY_VERSION_SOCIAL)
    } else {
        None
    }
}

/// Whether registry `version` offers channel `id`. Fails closed on a version
/// this build does not know.
pub fn channel_offered(id: u16, version: u16) -> bool {
    channel_version(id)
        .is_some_and(|needed| needed <= version && version <= CHANNEL_REGISTRY_VERSION_SOCIAL)
}

/// Every channel registry `version` offers, in ID order within each
/// direction's block. `None` for an unknown version.
pub fn channels_for(version: u16) -> Option<impl Iterator<Item = &'static ChannelEntry>> {
    let (v2, v3) = match version {
        CHANNEL_REGISTRY_VERSION => (0, 0),
        CHANNEL_REGISTRY_VERSION_ARTIFACT => (CHANNELS_V2.len(), 0),
        CHANNEL_REGISTRY_VERSION_SOCIAL => (CHANNELS_V2.len(), CHANNELS_V3.len()),
        _ => return None,
    };
    Some(
        CHANNELS
            .iter()
            .chain(CHANNELS_V2[..v2].iter())
            .chain(CHANNELS_V3[..v3].iter()),
    )
}

/// Activation registry version 1.
///
/// Two entries, and the second is not padding. `TanhApprox` is schema 1's
/// rational approximation, unchanged and with no libm call. `Linear` exists
/// because an Input-role node holds an already-normalized channel value in
/// [-1, 1], and squashing it again would distort every sensor before the
/// network ever saw it.
///
/// Deliberately no further entries. Activation choice is a second axis of
/// freedom, and Phase 9's non-goals say the phase measures structural
/// freedom and nothing else.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activation {
    TanhApprox,
    Linear,
}

pub const ACTIVATION_TANH: u8 = 1;
pub const ACTIVATION_LINEAR: u8 = 2;

impl Activation {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            ACTIVATION_TANH => Some(Activation::TanhApprox),
            ACTIVATION_LINEAR => Some(Activation::Linear),
            _ => None,
        }
    }

    pub fn id(self) -> u8 {
        match self {
            Activation::TanhApprox => ACTIVATION_TANH,
            Activation::Linear => ACTIVATION_LINEAR,
        }
    }

    /// Apply the activation. Both paths clamp, so a node can never emit a
    /// value outside [-1, 1] whatever its bias and incoming sum.
    pub fn apply(self, value: f32) -> f32 {
        match self {
            Activation::TanhApprox => crate::controller::tanh_approx(value),
            Activation::Linear => value.clamp(-1.0, 1.0),
        }
    }
}

/// The role a node plays. `Modulatory` is carried and inert until Phase 11
/// gates plastic edges on it, following the precedent thermal preference set
/// in Phase 2: present, inherited, validated, and behaviorally inactive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeRole {
    Input,
    Hidden,
    Output,
    Modulatory,
}

impl NodeRole {
    pub fn from_id(id: u8) -> Option<Self> {
        match id {
            1 => Some(NodeRole::Input),
            2 => Some(NodeRole::Hidden),
            3 => Some(NodeRole::Output),
            4 => Some(NodeRole::Modulatory),
            _ => None,
        }
    }

    pub fn id(self) -> u8 {
        match self {
            NodeRole::Input => 1,
            NodeRole::Hidden => 2,
            NodeRole::Output => 3,
            NodeRole::Modulatory => 4,
        }
    }
}

/// The entry for a channel id in **any** version this build knows. The codec
/// and the compiler use this: a genome that names a channel decodes and
/// compiles regardless of which world it is in; whether the world *offers*
/// the channel is a separate check (`channel_offered`) made at world
/// construction and restore.
pub fn channel(id: u16) -> Option<&'static ChannelEntry> {
    CHANNELS
        .iter()
        .chain(CHANNELS_V2.iter())
        .chain(CHANNELS_V3.iter())
        .find(|entry| entry.id == id)
}

pub fn channel_exists(id: u16) -> bool {
    channel(id).is_some()
}

/// Version-1 input channels.
pub fn input_channels() -> impl Iterator<Item = &'static ChannelEntry> {
    CHANNELS
        .iter()
        .filter(|entry| entry.direction == ChannelDirection::Input)
}

/// Version-1 output channels.
pub fn output_channels() -> impl Iterator<Item = &'static ChannelEntry> {
    CHANNELS
        .iter()
        .filter(|entry| entry.direction == ChannelDirection::Output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn channel_ids_are_unique_and_names_are_too() {
        let ids: BTreeSet<u16> = CHANNELS.iter().map(|entry| entry.id).collect();
        assert_eq!(ids.len(), CHANNELS.len(), "duplicate channel ID");
        let names: BTreeSet<&str> = CHANNELS.iter().map(|entry| entry.name).collect();
        assert_eq!(names.len(), CHANNELS.len(), "duplicate channel name");
    }

    #[test]
    fn the_registry_covers_the_post_phase_7_channel_set_and_no_more() {
        // The non-goal made concrete: 16 sensory channels (topology 1's 20
        // inputs minus its 4 memory registers) and 8 action channels
        // (its 12 outputs minus 4 memory writes). If this count changes,
        // either a capability was added -- which Phase 9 forbids -- or the
        // memory registers crept back in as channels.
        assert_eq!(input_channels().count(), 16);
        assert_eq!(output_channels().count(), 8);
        assert!(
            !CHANNELS.iter().any(|entry| entry.name.contains("memory")),
            "memory registers must be recurrent edges under schema 2, not channels"
        );
    }

    #[test]
    fn an_unknown_channel_is_absent_rather_than_defaulted() {
        assert!(channel_exists(1));
        assert!(channel_exists(101));
        for unknown in [0_u16, 63, 100, 109, 110, 111, 112, 122, 999, u16::MAX] {
            assert!(
                !channel_exists(unknown),
                "channel {unknown} should not exist"
            );
            assert!(channel(unknown).is_none());
        }
    }

    #[test]
    fn version_two_adds_exactly_the_artifact_channels_and_offers_them_only_at_two() {
        // Six cues, five actions, all distinct from version 1 and from each
        // other, and none of them in 109..=112.
        assert_eq!(CHANNELS_V2.len(), 11);
        assert_eq!(
            CHANNELS_V2
                .iter()
                .filter(|entry| entry.direction == ChannelDirection::Input)
                .count(),
            6
        );
        let all: BTreeSet<u16> = CHANNELS
            .iter()
            .chain(CHANNELS_V2.iter())
            .map(|entry| entry.id)
            .collect();
        assert_eq!(
            all.len(),
            CHANNELS.len() + CHANNELS_V2.len(),
            "duplicate id across versions"
        );
        let names: BTreeSet<&str> = CHANNELS
            .iter()
            .chain(CHANNELS_V2.iter())
            .map(|entry| entry.name)
            .collect();
        assert_eq!(names.len(), CHANNELS.len() + CHANNELS_V2.len());
        for entry in CHANNELS_V2 {
            assert!(!(109..=112).contains(&entry.id), "{}", entry.name);
            assert_eq!(
                channel_version(entry.id),
                Some(CHANNEL_REGISTRY_VERSION_ARTIFACT)
            );
            assert!(
                !channel_offered(entry.id, CHANNEL_REGISTRY_VERSION),
                "{}",
                entry.name
            );
            assert!(
                channel_offered(entry.id, CHANNEL_REGISTRY_VERSION_ARTIFACT),
                "{}",
                entry.name
            );
            assert!(channel_exists(entry.id));
        }
        for entry in CHANNELS {
            assert_eq!(channel_version(entry.id), Some(CHANNEL_REGISTRY_VERSION));
            assert!(channel_offered(entry.id, CHANNEL_REGISTRY_VERSION));
            assert!(channel_offered(entry.id, CHANNEL_REGISTRY_VERSION_ARTIFACT));
        }
        // An unknown version offers nothing, even a version-1 channel.
        assert!(!channel_offered(1, 0));
        assert!(!channel_offered(1, 4));
        assert!(channels_for(4).is_none());
        assert_eq!(channels_for(1).unwrap().count(), CHANNELS.len());
        assert_eq!(
            channels_for(2).unwrap().count(),
            CHANNELS.len() + CHANNELS_V2.len()
        );
        // No perception cue names a material or a depth.
        for entry in CHANNELS_V2 {
            assert!(!entry.name.contains("material"), "{}", entry.name);
            assert!(!entry.name.contains("depth"), "{}", entry.name);
        }
    }

    #[test]
    fn version_three_adds_exactly_the_social_channels_and_offers_them_only_at_three() {
        // Nine cues per slot times four slots, four signal inputs, four
        // emission outputs; distinct from every earlier id and from each
        // other, none in the reserved 109..=112.
        assert_eq!(CHANNELS_V3.len(), 44);
        assert_eq!(
            CHANNELS_V3
                .iter()
                .filter(|entry| entry.direction == ChannelDirection::Input)
                .count(),
            40
        );
        let all: BTreeSet<u16> = CHANNELS
            .iter()
            .chain(CHANNELS_V2.iter())
            .chain(CHANNELS_V3.iter())
            .map(|entry| entry.id)
            .collect();
        assert_eq!(
            all.len(),
            CHANNELS.len() + CHANNELS_V2.len() + CHANNELS_V3.len(),
            "duplicate id across versions"
        );
        for entry in CHANNELS_V3 {
            assert!(!(109..=112).contains(&entry.id), "{}", entry.name);
            assert_eq!(
                channel_version(entry.id),
                Some(CHANNEL_REGISTRY_VERSION_SOCIAL)
            );
            assert!(
                !channel_offered(entry.id, CHANNEL_REGISTRY_VERSION),
                "{}",
                entry.name
            );
            assert!(
                !channel_offered(entry.id, CHANNEL_REGISTRY_VERSION_ARTIFACT),
                "{}",
                entry.name
            );
            assert!(
                channel_offered(entry.id, CHANNEL_REGISTRY_VERSION_SOCIAL),
                "{}",
                entry.name
            );
            assert!(channel_exists(entry.id));
        }
        // Everything earlier is still offered at version 3: the scheme is a
        // total order.
        for entry in CHANNELS.iter().chain(CHANNELS_V2.iter()) {
            assert!(
                channel_offered(entry.id, CHANNEL_REGISTRY_VERSION_SOCIAL),
                "{}",
                entry.name
            );
        }
        assert_eq!(
            channels_for(3).unwrap().count(),
            CHANNELS.len() + CHANNELS_V2.len() + CHANNELS_V3.len()
        );
        // The slot/cue arithmetic the sense phase uses matches the table.
        for slot in 0..4_u16 {
            for cue in 0..NEIGHBOUR_CUE_COUNT {
                let id = CHANNEL_NEIGHBOUR_BASE + slot * NEIGHBOUR_CUE_COUNT + cue;
                assert_eq!(channel(id).unwrap().direction, ChannelDirection::Input);
            }
        }
        for c in 0..4_u16 {
            assert_eq!(
                channel(CHANNEL_SIGNAL_IN_BASE + c).unwrap().direction,
                ChannelDirection::Input
            );
            assert_eq!(
                channel(CHANNEL_SIGNAL_EMIT_BASE + c).unwrap().direction,
                ChannelDirection::Output
            );
        }
        // Cues, never labels (ADR-0022 A3/A4, ADR-0029 section 2): no
        // material, depth, action or kin channel; and no signal channel
        // carries a name beyond its number.
        for entry in CHANNELS_V3 {
            for forbidden in ["material", "depth", "action", "kin", "genotype"] {
                assert!(!entry.name.contains(forbidden), "{}", entry.name);
            }
        }
    }

    #[test]
    fn activations_round_trip_and_reject_the_unknown() {
        for activation in [Activation::TanhApprox, Activation::Linear] {
            assert_eq!(Activation::from_id(activation.id()), Some(activation));
        }
        for unknown in [0_u8, 3, 4, 255] {
            assert_eq!(Activation::from_id(unknown), None);
        }
    }

    #[test]
    fn every_activation_clamps_its_output() {
        // A node's output feeds other nodes' pre-activation sums, so an
        // unclamped activation would let one edge weight blow up the whole
        // network. Checked for both, including the linear one, where it is
        // the only thing keeping it bounded.
        for activation in [Activation::TanhApprox, Activation::Linear] {
            for input in [-1e9_f32, -8.0, -1.0, 0.0, 1.0, 8.0, 1e9] {
                let output = activation.apply(input);
                assert!(
                    (-1.0..=1.0).contains(&output),
                    "{activation:?} produced {output} from {input}"
                );
                assert!(output.is_finite());
            }
        }
    }

    #[test]
    fn node_roles_round_trip_and_reject_the_unknown() {
        for role in [
            NodeRole::Input,
            NodeRole::Hidden,
            NodeRole::Output,
            NodeRole::Modulatory,
        ] {
            assert_eq!(NodeRole::from_id(role.id()), Some(role));
        }
        for unknown in [0_u8, 5, 255] {
            assert_eq!(NodeRole::from_id(unknown), None);
        }
    }
}
