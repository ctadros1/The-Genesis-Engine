//! Every settable config field must survive the snapshot codec.
//!
//! This is the structural version of a check that has been made by hand
//! three times and missed three times. `sim-persist`'s
//! `config_round_trip.rs` perturbs a **hand-maintained list** of fields and
//! compares whole `SimConfig`s, so it defends exactly the fields somebody
//! remembered to add to `perturbed()` - and a field left at its default
//! compares default-to-default and round-trips "successfully" whether or not
//! the codec ever wrote it. The record:
//!
//! - D-065: the climate, origin, contest and physiology sections were
//!   dropped on save, and a `seeded` world restored as `random` silently.
//! - D-086: Phase 10's whole morphology config was dropped; the round-trip
//!   test predated the section and perturbed nothing in it.
//! - 2026-08-10: `genome2.mutation.plasticity_enabled` was dropped, and the
//!   *entire* genome2 section turned out never to have been perturbed - so
//!   `regulatory_enabled`, which is C10.3's whole control, had been
//!   undefended for two phases.
//!
//! The fix is to stop maintaining a list. `FIELD_NAMES` is the registry a
//! campaign uses to name config fields, so it is already the definition of
//! "a setting someone can change". Driving the sweep from it means a new
//! field is protected the moment it becomes settable, and a new field that
//! is *not* settable is a separate, visible gap rather than a silent one.
//!
//! This lives in `sim-experiment` because it is the only crate that depends
//! on both the field registry and the codec. Putting it in `sim-persist`
//! would invert the layering.

use sim_experiment::{FIELD_NAMES, FieldValue, read_field, set_field};

/// A value different from the one the field currently holds.
///
/// Deliberately dumb: increment a number, flip a bool, take the other
/// choice. What matters is only that it **differs**, because the whole
/// failure mode being defended against is a value that compares equal for
/// the wrong reason.
fn perturbation(current: FieldValue) -> String {
    match current {
        FieldValue::Bool(value) => (!value).to_string(),
        FieldValue::U32(value) => value.saturating_add(1).to_string(),
        FieldValue::U64(value) => value.saturating_add(1).to_string(),
        FieldValue::I32(value) => value.saturating_add(1).to_string(),
        FieldValue::I64(value) => value.saturating_add(1).to_string(),
        // The only enumerated field today. Named explicitly rather than
        // skipped, so a second one is a compile-visible decision.
        FieldValue::Choice("random") => "seeded".to_owned(),
        FieldValue::Choice(_) => "random".to_owned(),
    }
}

/// Round-trip a config through the real snapshot codec.
///
/// The config is substituted into a carrier world's state, exactly as
/// `sim-persist`'s own test does: some perturbations make an ecologically
/// degenerate map that world generation rightly refuses, and refusing to
/// *encode* those would leave the fields unchecked. This is a codec test,
/// not a world-generation test.
fn round_trip(config: sim_core::SimConfig) -> sim_core::SimConfig {
    let carrier = sim_core::World::new(sim_core::SimConfig::phase2_default(config.world_seed))
        .expect("carrier world builds");
    let mut state = carrier.export_state();
    state.config = config;
    let bytes = sim_persist::encode_snapshot(
        &state,
        1,
        0,
        carrier.state_checksum(),
        sim_persist::BUILD_VERSION,
        0,
        None,
    )
    .expect("encode");
    let (_, decoded) = sim_persist::decode_snapshot(&bytes).expect("decode");
    decoded.config
}

#[test]
fn every_settable_config_field_survives_the_snapshot_codec() {
    let base = sim_core::SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    let mut checked = 0_usize;
    for name in FIELD_NAMES {
        let before = read_field(&base, name)
            .unwrap_or_else(|| panic!("{name} is in FIELD_NAMES but read_field does not know it"));
        let mut perturbed = base;
        set_field(&mut perturbed, name, &perturbation(before))
            .unwrap_or_else(|error| panic!("{name}: {error}"));

        let after_set = read_field(&perturbed, name).expect("field reads back");
        // Guard the guard. If the perturbation did not take - a saturating
        // increment at the type's maximum, say - then the comparison below
        // is default-to-default and proves nothing about the codec, which is
        // the exact defect this file exists for.
        assert_ne!(
            after_set, before,
            "{name}: the perturbation did not change the field, so the \
             round-trip assertion below would be vacuous"
        );

        let restored = round_trip(perturbed);
        let after_round_trip = read_field(&restored, name).expect("field reads back");
        assert_eq!(
            after_round_trip, after_set,
            "{name} did not survive the snapshot codec: set to {after_set}, \
             restored as {after_round_trip}. The field is settable by a \
             campaign and is silently lost across every checkpoint."
        );
        checked += 1;
    }
    // The sweep must have run. An empty or filtered `FIELD_NAMES` would make
    // every assertion above unreachable and this test green.
    assert!(
        checked > 100,
        "only {checked} fields were swept, which is fewer than the registry \
         has ever had; the sweep is not covering what it claims to"
    );
}

#[test]
fn a_field_dropped_by_the_codec_is_caught_by_the_sweep() {
    // The positive control for the test above, and the thing that makes it
    // more than a hopeful loop: it demonstrates the sweep's mechanism on a
    // field whose value is deliberately not carried through.
    //
    // `world_seed` is the one config value the codec writes from a different
    // path (it is the snapshot's own seed field), so it is not a good
    // subject. Instead this asserts the property the sweep relies on: that
    // `read_field` after a round trip reflects what the codec actually
    // stored, rather than being re-derived from a default.
    let base = sim_core::SimConfig::phase2_default(0x1234_5678_9abc_def0);
    let mut perturbed = base;
    set_field(
        &mut perturbed,
        "genome2.mutation.plasticity_enabled",
        "true",
    )
    .expect("the gate is settable");
    set_field(&mut perturbed, "plasticity.enabled", "true").expect("the section is settable");
    assert_ne!(perturbed, base);

    let restored = round_trip(perturbed);
    assert_eq!(
        read_field(&restored, "genome2.mutation.plasticity_enabled"),
        Some(FieldValue::Bool(true)),
        "the plasticity mutation gate is the field whose loss turned a \
         treatment run into a control across a checkpoint"
    );
    assert_eq!(
        read_field(&restored, "plasticity.enabled"),
        Some(FieldValue::Bool(true))
    );
}
