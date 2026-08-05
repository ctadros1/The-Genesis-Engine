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

/// Bumped when an entry is added. Enters the config hash, so a world that
/// declares one registry version can never be confused with another.
pub const CHANNEL_REGISTRY_VERSION: u16 = 1;
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

pub fn channel(id: u16) -> Option<&'static ChannelEntry> {
    CHANNELS.iter().find(|entry| entry.id == id)
}

pub fn channel_exists(id: u16) -> bool {
    channel(id).is_some()
}

pub fn input_channels() -> impl Iterator<Item = &'static ChannelEntry> {
    CHANNELS
        .iter()
        .filter(|entry| entry.direction == ChannelDirection::Input)
}

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
        for unknown in [0_u16, 17, 50, 100, 109, 999, u16::MAX] {
            assert!(
                !channel_exists(unknown),
                "channel {unknown} should not exist"
            );
            assert!(channel(unknown).is_none());
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
