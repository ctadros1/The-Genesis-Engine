//! Phase 9: genome schema 2 codec, structural invariants, and expression.
//!
//! Covers C9.6's codec half (bounded fail-closed decode plus the 100,000
//! case malformed-input harness) and the specification's expression test
//! requirements. Meiosis, structural mutation, controller v2, and the
//! campaign criteria are later slices and are not here.
//!
//! C9.7's fixture obligation is checked here too, because it is the cheapest
//! possible place to catch a schema-2 change that reaches a schema-1 world:
//! a schema-1 configured world must still reproduce `0xff9dfcff5dffbf42`.

use sim_core::{
    Activation, EDGE_FLAG_DELAYED, EDGE_FLAG_DISABLED, EDGE_FLAG_PLASTIC, GENOME2_MAGIC, Genome2,
    Genome2Error, GenomeCaps, Haplotype, Locus, LocusKind, NodeRole, PlasticityGenes,
    STRUCTURAL_HOMOLOGY_BASE, SimConfig, World, named_random,
};

const NODE_A: u32 = STRUCTURAL_HOMOLOGY_BASE + 10;
const NODE_B: u32 = STRUCTURAL_HOMOLOGY_BASE + 20;
const NODE_C: u32 = STRUCTURAL_HOMOLOGY_BASE + 30;
const EDGE_AB: u32 = STRUCTURAL_HOMOLOGY_BASE + 40;
const EDGE_BC: u32 = STRUCTURAL_HOMOLOGY_BASE + 50;
const BIND_IN: u32 = STRUCTURAL_HOMOLOGY_BASE + 60;
const BIND_OUT: u32 = STRUCTURAL_HOMOLOGY_BASE + 70;

fn locus(homology_id: u32, kind: LocusKind) -> Locus {
    Locus {
        homology_id,
        gene_lineage_id: u64::from(homology_id) * 7 + 1,
        mutation_event_id: u64::from(homology_id) * 13 + 2,
        kind,
    }
}

fn node(homology_id: u32, role: NodeRole, bias: f32) -> Locus {
    locus(
        homology_id,
        LocusKind::Node {
            role,
            activation_id: Activation::TanhApprox.id(),
            bias,
            time_constant: 0,
        },
    )
}

fn edge(homology_id: u32, source: u32, target: u32, weight: f32, flags: u8) -> Locus {
    locus(
        homology_id,
        LocusKind::Edge {
            source,
            target,
            weight,
            flags,
            plasticity: PlasticityGenes::inert(),
        },
    )
}

fn binding(homology_id: u32, node: u32, channel_id: u16, gain: f32) -> Locus {
    locus(
        homology_id,
        LocusKind::IoBinding {
            node,
            channel_id,
            gain,
        },
    )
}

fn trait_locus(trait_id: u16, value: f32, dominance: f32) -> Locus {
    locus(
        u32::from(trait_id) + 1,
        LocusKind::Trait {
            trait_id,
            value,
            dominance,
        },
    )
}

/// A small valid haplotype: two bound nodes plus a hidden one, a feed-forward
/// chain, and one trait.
fn haplotype(bias: f32, weight: f32) -> Haplotype {
    Haplotype {
        chromosomes: vec![vec![
            trait_locus(0, 0.5, 1.0),
            node(NODE_A, NodeRole::Input, 0.0),
            node(NODE_B, NodeRole::Hidden, bias),
            node(NODE_C, NodeRole::Output, 0.0),
            edge(EDGE_AB, NODE_A, NODE_B, weight, 0),
            edge(EDGE_BC, NODE_B, NODE_C, weight, 0),
            binding(BIND_IN, NODE_A, 1, 1.0),
            binding(BIND_OUT, NODE_C, 101, 1.0),
        ]],
    }
}

fn genome() -> Genome2 {
    Genome2 {
        haplotypes: [haplotype(0.25, 1.5), haplotype(0.25, 1.5)],
    }
}

fn caps() -> GenomeCaps {
    GenomeCaps::provisional()
}

// --- Round trip and framing ------------------------------------------------

#[test]
fn a_valid_genome_round_trips_exactly() {
    let original = genome();
    let bytes = original.encode();
    assert_eq!(&bytes[0..4], GENOME2_MAGIC);
    let decoded = Genome2::decode(&bytes, &caps()).expect("decodes");
    assert_eq!(decoded, original);
    // Re-encoding a decoded genome must be byte-identical, or the codec has
    // a normalization step that silently rewrites records.
    assert_eq!(decoded.encode(), bytes);
}

#[test]
fn every_single_byte_corruption_is_caught() {
    // The whole record is covered, header included: a flipped bit in the
    // chromosome count or a lineage ID would mislabel a genome rather than
    // fail it, which is worse than a decode error.
    let bytes = genome().encode();
    let mut checked = 0;
    for index in 0..bytes.len() {
        for bit in [0x01_u8, 0x40] {
            let mut damaged = bytes.clone();
            damaged[index] ^= bit;
            if damaged == bytes {
                continue;
            }
            assert!(
                Genome2::decode(&damaged, &caps()).is_err(),
                "corruption at byte {index} bit {bit:#x} decoded cleanly"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, bytes.len() * 2);
}

#[test]
fn every_truncation_is_refused() {
    let bytes = genome().encode();
    for length in 0..bytes.len() {
        assert!(
            Genome2::decode(&bytes[..length], &caps()).is_err(),
            "a {length}-byte prefix decoded cleanly"
        );
    }
}

#[test]
fn header_rejections_are_typed() {
    let good = genome().encode();
    let mutate = |edit: &dyn Fn(&mut Vec<u8>)| {
        let mut bytes = good.clone();
        edit(&mut bytes);
        // Re-checksum so the test exercises the *field* check rather than
        // being caught by the checksum first.
        let split = bytes.len() - 8;
        let checksum = sim_core::fnv1a64(&bytes[..split]);
        bytes[split..].copy_from_slice(&checksum.to_le_bytes());
        Genome2::decode(&bytes, &caps())
    };

    assert!(matches!(
        mutate(&|bytes| bytes[0] = b'X'),
        Err(Genome2Error::BadMagic)
    ));
    assert!(matches!(
        mutate(&|bytes| bytes[4..6].copy_from_slice(&3_u16.to_le_bytes())),
        Err(Genome2Error::UnsupportedSchema(3))
    ));
    assert!(matches!(
        mutate(&|bytes| bytes[6..8].copy_from_slice(&99_u16.to_le_bytes())),
        Err(Genome2Error::UnsupportedChannelRegistry(99))
    ));
    assert!(matches!(
        mutate(&|bytes| bytes[8] = 1),
        Err(Genome2Error::UnsupportedPloidy(1))
    ));
    assert!(matches!(
        mutate(&|bytes| bytes[9] = 0),
        Err(Genome2Error::ChromosomeCount(0))
    ));
    // Reserved flags must be zero: a reader that ignored them would silently
    // accept a record written by a future format.
    assert!(matches!(
        mutate(&|bytes| bytes[10..12].copy_from_slice(&1_u16.to_le_bytes())),
        Err(Genome2Error::UnknownFlags(1))
    ));
    assert!(matches!(
        mutate(&|bytes| bytes[12..16].copy_from_slice(&9_999_u32.to_le_bytes())),
        Err(Genome2Error::LengthMismatch {
            declared: 9_999,
            ..
        })
    ));
    // ...and a corrupted checksum, which is the one case the re-checksum
    // above would hide, checked directly.
    let mut damaged = good.clone();
    let last = damaged.len() - 1;
    damaged[last] ^= 0xff;
    assert!(matches!(
        Genome2::decode(&damaged, &caps()),
        Err(Genome2Error::ChecksumMismatch)
    ));
}

#[test]
fn an_oversized_locus_count_is_refused_before_allocation() {
    // The declared count is checked against the cap *and* against the bytes
    // actually remaining, so a record claiming four billion loci cannot make
    // the decoder reserve four billion slots.
    let mut bytes = genome().encode();
    bytes[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
    let split = bytes.len() - 8;
    let checksum = sim_core::fnv1a64(&bytes[..split]);
    bytes[split..].copy_from_slice(&checksum.to_le_bytes());
    assert!(matches!(
        Genome2::decode(&bytes, &caps()),
        Err(Genome2Error::LocusCountTooLarge { .. })
    ));
}

// --- Structural invariants -------------------------------------------------

fn expect_error(genome: Genome2, matcher: impl Fn(&Genome2Error) -> bool, label: &str) {
    let bytes = genome.encode();
    match Genome2::decode(&bytes, &caps()) {
        Err(error) if matcher(&error) => {}
        other => panic!("{label}: expected a specific error, got {other:?}"),
    }
}

#[test]
fn unsorted_loci_are_rejected() {
    let mut broken = genome();
    broken.haplotypes[0].chromosomes[0].swap(1, 2);
    expect_error(
        broken,
        |error| matches!(error, Genome2Error::NotSorted { .. }),
        "unsorted",
    );
}

#[test]
fn duplicate_homology_ids_are_rejected() {
    // Strictly ascending, not merely ascending: two loci sharing a homology
    // ID would make alignment during meiosis ambiguous.
    let mut broken = genome();
    let duplicate = broken.haplotypes[0].chromosomes[0][4];
    broken.haplotypes[0].chromosomes[0].insert(5, duplicate);
    expect_error(
        broken,
        |error| matches!(error, Genome2Error::NotSorted { .. }),
        "duplicate homology",
    );
}

#[test]
fn dangling_edge_and_binding_references_are_rejected() {
    for (label, edit) in [
        (
            "edge source",
            Box::new(|g: &mut Genome2| {
                g.haplotypes[0].chromosomes[0][4] = edge(EDGE_AB, 999_999, NODE_B, 1.0, 0);
            }) as Box<dyn Fn(&mut Genome2)>,
        ),
        (
            "edge target",
            Box::new(|g: &mut Genome2| {
                g.haplotypes[0].chromosomes[0][4] = edge(EDGE_AB, NODE_A, 999_999, 1.0, 0);
            }),
        ),
        (
            "binding node",
            Box::new(|g: &mut Genome2| {
                g.haplotypes[0].chromosomes[0][6] = binding(BIND_IN, 999_999, 1, 1.0);
            }),
        ),
        (
            "modulator node",
            Box::new(|g: &mut Genome2| {
                let mut plasticity = PlasticityGenes::inert();
                plasticity.modulator_node = 999_999;
                g.haplotypes[0].chromosomes[0][4] = locus(
                    EDGE_AB,
                    LocusKind::Edge {
                        source: NODE_A,
                        target: NODE_B,
                        weight: 1.0,
                        flags: 0,
                        plasticity,
                    },
                );
            }),
        ),
    ] {
        let mut broken = genome();
        edit(&mut broken);
        expect_error(
            broken,
            |error| matches!(error, Genome2Error::DanglingReference { .. }),
            label,
        );
    }
}

#[test]
fn a_reference_across_haplotypes_does_not_count_as_present() {
    // An edge may only refer to a node on its own haplotype. Allowing a
    // cross-haplotype reference would make expression depend on which allele
    // the *other* parent contributed, which is not what a chromosome is.
    let mut broken = genome();
    broken.haplotypes[1].chromosomes[0].push(node(
        STRUCTURAL_HOMOLOGY_BASE + 900,
        NodeRole::Hidden,
        0.0,
    ));
    broken.haplotypes[0].chromosomes[0][4] =
        edge(EDGE_AB, NODE_A, STRUCTURAL_HOMOLOGY_BASE + 900, 1.0, 0);
    expect_error(
        broken,
        |error| matches!(error, Genome2Error::DanglingReference { .. }),
        "cross-haplotype reference",
    );
}

#[test]
fn an_unknown_channel_activation_or_role_fails_closed() {
    let mut bad_channel = genome();
    bad_channel.haplotypes[0].chromosomes[0][6] = binding(BIND_IN, NODE_A, 777, 1.0);
    expect_error(
        bad_channel,
        |error| matches!(error, Genome2Error::UnknownChannel(777)),
        "unknown channel",
    );

    let mut bad_activation = genome();
    bad_activation.haplotypes[0].chromosomes[0][2] = locus(
        NODE_B,
        LocusKind::Node {
            role: NodeRole::Hidden,
            activation_id: 200,
            bias: 0.0,
            time_constant: 0,
        },
    );
    expect_error(
        bad_activation,
        |error| matches!(error, Genome2Error::UnknownActivation(200)),
        "unknown activation",
    );
}

#[test]
fn out_of_range_values_are_rejected() {
    for (label, kind) in [
        (
            "weight",
            LocusKind::Edge {
                source: NODE_A,
                target: NODE_B,
                weight: 99.0,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        ),
        (
            "non-finite weight",
            LocusKind::Edge {
                source: NODE_A,
                target: NODE_B,
                weight: f32::NAN,
                flags: 0,
                plasticity: PlasticityGenes::inert(),
            },
        ),
        (
            "reserved flag bit",
            LocusKind::Edge {
                source: NODE_A,
                target: NODE_B,
                weight: 1.0,
                flags: 0b1000_0000,
                plasticity: PlasticityGenes::inert(),
            },
        ),
    ] {
        let mut broken = genome();
        broken.haplotypes[0].chromosomes[0][4] = locus(EDGE_AB, kind);
        expect_error(
            broken,
            |error| matches!(error, Genome2Error::ValueOutOfRange(_)),
            label,
        );
    }

    let mut bad_trait = genome();
    bad_trait.haplotypes[0].chromosomes[0][0] = trait_locus(0, 1.5, 0.5);
    expect_error(
        bad_trait,
        |error| matches!(error, Genome2Error::ValueOutOfRange(_)),
        "trait value",
    );
}

#[test]
fn a_zero_delay_cycle_is_a_decode_error_and_a_delayed_one_is_not() {
    // The property the hybrid evaluation rests on: cycles are legal and
    // useful, but only through delayed edges, which read prior-state buffers
    // and therefore have a well-defined value. A zero-delay cycle has no
    // fixed point, and refusing it at decode is what keeps evaluation from
    // having to invent an iteration order that produces *a* number.
    let cyclic = |flags: u8| {
        let mut g = genome();
        for haplotype in &mut g.haplotypes {
            haplotype.chromosomes[0].push(edge(
                STRUCTURAL_HOMOLOGY_BASE + 80,
                NODE_C,
                NODE_A,
                0.5,
                flags,
            ));
        }
        g
    };
    expect_error(
        cyclic(0),
        |error| matches!(error, Genome2Error::ZeroDelayCycle),
        "zero-delay cycle",
    );
    // The same cycle with the delay bit set is accepted.
    let delayed = cyclic(EDGE_FLAG_DELAYED);
    Genome2::decode(&delayed.encode(), &caps()).expect("a delayed cycle is legal");
    // ...and so is one whose closing edge is disabled.
    let disabled = cyclic(EDGE_FLAG_DISABLED);
    Genome2::decode(&disabled.encode(), &caps()).expect("a disabled cycle edge is legal");
}

#[test]
fn caps_reject_deterministically() {
    let mut tight = caps();
    tight.max_edges_per_node = 1;
    let mut crowded = genome();
    for haplotype in &mut crowded.haplotypes {
        haplotype.chromosomes[0].push(edge(STRUCTURAL_HOMOLOGY_BASE + 90, NODE_A, NODE_B, 0.5, 0));
    }
    let bytes = crowded.encode();
    assert!(matches!(
        Genome2::decode(&bytes, &tight),
        Err(Genome2Error::CapExceeded("max_edges_per_node"))
    ));
    // The same record under the ordinary caps is fine, so the rejection is
    // the cap and not the record.
    assert!(Genome2::decode(&bytes, &caps()).is_ok());

    let mut small = caps();
    small.max_genome_bytes = 32;
    assert!(matches!(
        Genome2::decode(&bytes, &small),
        Err(Genome2Error::CapExceeded("max_genome_bytes"))
    ));
}

// --- Expression -------------------------------------------------------------

#[test]
fn dominance_spans_codominant_to_complete() {
    use sim_core::blend_by_dominance;
    // Equal dominance is additive.
    assert_eq!(blend_by_dominance(0.0, 1.0, 1.0, 1.0), 0.5);
    // Complete dominance of the first allele.
    assert_eq!(blend_by_dominance(0.2, 1.0, 0.9, 0.0), 0.2);
    // ...and of the second.
    assert_eq!(blend_by_dominance(0.2, 0.0, 0.9, 1.0), 0.9);
    // Incomplete dominance lands between, nearer the dominant allele.
    let partial = blend_by_dominance(0.0, 3.0, 1.0, 1.0);
    assert!((partial - 0.25).abs() < 1e-6, "got {partial}");
    // Both dominances zero falls back to the mean rather than dividing by
    // zero, which is the case a naive formula gets wrong.
    assert_eq!(blend_by_dominance(0.2, 0.0, 0.8, 0.0), 0.5);
}

#[test]
fn a_heterozygote_expresses_the_blend_and_a_hemizygote_its_single_value() {
    let mut subject = genome();
    subject.haplotypes[0].chromosomes[0][0] = trait_locus(0, 0.0, 1.0);
    subject.haplotypes[1].chromosomes[0][0] = trait_locus(0, 1.0, 1.0);
    // Trait 1 exists on one haplotype only: hemizygous.
    subject.haplotypes[0].chromosomes[0].insert(1, trait_locus(1, 0.75, 1.0));

    let traits = subject.express_traits();
    assert_eq!(traits[0], Some(0.5), "codominant heterozygote");
    assert_eq!(traits[1], Some(0.75), "hemizygote expresses its one allele");
    assert_eq!(traits[2], None, "an absent trait is absent, not defaulted");
}

#[test]
fn expression_is_a_pure_function_and_survives_a_round_trip() {
    let subject = genome();
    let first = subject.express_network();
    let second = subject.express_network();
    assert_eq!(first, second, "expression is not deterministic");

    let decoded = Genome2::decode(&subject.encode(), &caps()).expect("decodes");
    assert_eq!(
        decoded.express_network(),
        first,
        "expression differs after a save/restore round trip"
    );
    assert_eq!(decoded.express_traits(), subject.express_traits());
}

#[test]
fn the_expressed_network_is_sorted_by_homology_so_summation_order_is_pinned() {
    // Float addition is not associative, so per-node summation must follow
    // homology order rather than storage order. That is only meaningful if
    // the expressed edge list is itself in homology order.
    let network = genome().express_network();
    assert!(
        network
            .nodes
            .windows(2)
            .all(|w| w[0].homology_id < w[1].homology_id)
    );
    assert!(
        network
            .edges
            .windows(2)
            .all(|w| w[0].homology_id < w[1].homology_id)
    );
    assert!(
        network
            .bindings
            .windows(2)
            .all(|w| w[0].homology_id < w[1].homology_id)
    );
    assert_eq!(network.nodes.len(), 3);
    assert_eq!(network.edges.len(), 2);
    assert_eq!(network.bindings.len(), 2);
}

#[test]
fn flag_combination_follows_the_recorded_policy() {
    // `disabled` needs both haplotypes; `plastic` and `delayed` need either.
    // Recorded policy, so it is asserted rather than left to whichever of
    // `&` and `|` the implementation happened to reach for.
    let build = |left: u8, right: u8| {
        let mut g = genome();
        g.haplotypes[0].chromosomes[0][4] = edge(EDGE_AB, NODE_A, NODE_B, 1.0, left);
        g.haplotypes[1].chromosomes[0][4] = edge(EDGE_AB, NODE_A, NODE_B, 1.0, right);
        let network = g.express_network();
        *network
            .edges
            .iter()
            .find(|e| e.homology_id == EDGE_AB)
            .expect("edge")
    };

    assert!(
        !build(EDGE_FLAG_DISABLED, 0).disabled,
        "one working copy is enough"
    );
    assert!(build(EDGE_FLAG_DISABLED, EDGE_FLAG_DISABLED).disabled);
    assert!(build(EDGE_FLAG_PLASTIC, 0).plastic, "plastic on either");
    assert!(build(EDGE_FLAG_DELAYED, 0).delayed, "delayed on either");
    assert!(!build(0, 0).plastic && !build(0, 0).delayed && !build(0, 0).disabled);
}

#[test]
fn a_duplication_is_immediately_hemizygous() {
    // The biological situation right after a duplication, and the source of
    // the divergence that follows: the new locus exists on one haplotype
    // only and expresses at its own value rather than being blended toward
    // an allele that does not exist.
    let mut subject = genome();
    let fresh = STRUCTURAL_HOMOLOGY_BASE + 200;
    subject.haplotypes[0].chromosomes[0].push(node(fresh, NodeRole::Hidden, 2.0));
    let network = subject.express_network();
    let expressed = network
        .nodes
        .iter()
        .find(|n| n.homology_id == fresh)
        .expect("the duplicate is expressed");
    assert_eq!(expressed.bias, 2.0);
}

// --- Derived identity -------------------------------------------------------

#[test]
fn identical_independent_mutations_converge_on_the_same_homology_id() {
    // The property that replaces NEAT's innovation record. Two lineages
    // applying the same operator to the same parent slot get the same
    // homology ID, so they *align* during meiosis instead of being treated
    // as disjoint structure.
    let left = sim_core::derive_homology_id(NODE_A, 2, 7, 0);
    let right = sim_core::derive_homology_id(NODE_A, 2, 7, 0);
    assert_eq!(left, right);
    // ...while genuinely different mutations do not collide.
    assert_ne!(left, sim_core::derive_homology_id(NODE_A, 2, 8, 0));
    assert_ne!(left, sim_core::derive_homology_id(NODE_A, 3, 7, 0));
    assert_ne!(left, sim_core::derive_homology_id(NODE_B, 2, 7, 0));
    assert_ne!(left, sim_core::derive_homology_id(NODE_A, 2, 7, 1));
}

#[test]
fn a_derived_homology_id_never_lands_in_the_trait_block() {
    // Traits occupy a reserved low range and sort by `trait_id`. A
    // structural innovation landing there would interleave the two sort
    // blocks and could collide with a trait.
    for attempt in 0..5_000_u32 {
        let id = sim_core::derive_homology_id(attempt.wrapping_mul(2_654_435_761), 1, attempt, 0);
        assert!(
            id >= STRUCTURAL_HOMOLOGY_BASE,
            "derived {id} in the trait block"
        );
    }
}

#[test]
fn mutation_event_ids_separate_the_things_they_identify() {
    let base = sim_core::derive_mutation_event_id(1, 2, 3, 4, 5);
    assert_eq!(base, sim_core::derive_mutation_event_id(1, 2, 3, 4, 5));
    for changed in [
        sim_core::derive_mutation_event_id(9, 2, 3, 4, 5),
        sim_core::derive_mutation_event_id(1, 9, 3, 4, 5),
        sim_core::derive_mutation_event_id(1, 2, 9, 4, 5),
        sim_core::derive_mutation_event_id(1, 2, 3, 9, 5),
        sim_core::derive_mutation_event_id(1, 2, 3, 4, 9),
    ] {
        assert_ne!(base, changed);
    }
}

#[test]
fn the_structural_signature_ignores_identity_and_tracks_structure() {
    // Two loci that arose independently -- different lineage and event IDs,
    // different homology -- but describe the same structure must share a
    // signature, and a genuine structural change must break it.
    let left = edge(EDGE_AB, NODE_A, NODE_B, 1.0, 0);
    let mut right = edge(EDGE_BC, NODE_A, NODE_B, -7.0, 0);
    right.gene_lineage_id = 999;
    right.mutation_event_id = 777;
    assert_eq!(left.structural_signature(), right.structural_signature());

    let rewired = edge(EDGE_AB, NODE_A, NODE_C, 1.0, 0);
    assert_ne!(left.structural_signature(), rewired.structural_signature());
    // Delay is phenotype-relevant structure; plastic and disabled are
    // expression state, so they must not move the signature.
    let delayed = edge(EDGE_AB, NODE_A, NODE_B, 1.0, EDGE_FLAG_DELAYED);
    assert_ne!(left.structural_signature(), delayed.structural_signature());
    let disabled = edge(EDGE_AB, NODE_A, NODE_B, 1.0, EDGE_FLAG_DISABLED);
    assert_eq!(left.structural_signature(), disabled.structural_signature());
}

// --- C9.6: the malformed-input harness --------------------------------------

#[test]
fn c9_6_a_hundred_thousand_malformed_records_produce_no_panic_and_no_bad_admission() {
    // Mirrors the Phase 2 harness and its reporting format. Every accepted
    // record is re-validated and round-tripped, because "did not panic" is a
    // much weaker property than "admitted nothing invalid".
    const CASES: u64 = 100_000;
    let template = genome().encode();
    let caps = caps();
    let mut accepted = 0_u64;
    let mut rejected = 0_u64;

    for case in 0..CASES {
        let mut bytes = template.clone();
        let draw = |index: u32| {
            named_random(
                0x9e37_79b9_7f4a_7c15,
                case,
                sim_core::RngSystem::Analysis,
                0,
                index,
            )
        };
        // Four families. The first three corrupt bytes, which almost always
        // dies at the checksum -- necessary, but it means those cases never
        // reach the structural validation at all. The fourth mutates the
        // *genome* and re-encodes, so framing and checksum are valid by
        // construction and every case lands squarely on the invariant
        // checks. Without it this harness would be 100,000 tests of the
        // checksum.
        match draw(0) % 4 {
            3 => {
                let mut subject = genome();
                let chromosome = &mut subject.haplotypes[(draw(1) % 2) as usize].chromosomes[0];
                let at = (draw(2) as usize) % chromosome.len();
                match draw(3) % 8 {
                    // Repoint a reference at something that may not exist.
                    0 => {
                        chromosome[at] =
                            edge(EDGE_AB, NODE_A, draw(4) as u32, 1.0, (draw(5) % 256) as u8)
                    }
                    // Move a locus out of its homology block.
                    1 => chromosome[at].homology_id = draw(4) as u32,
                    // Break sortedness.
                    2 => {
                        let next = (at + 1) % chromosome.len();
                        chromosome.swap(at, next);
                    }
                    // Duplicate a locus, which duplicates its homology ID.
                    3 => {
                        let copy = chromosome[at];
                        chromosome.insert(at, copy);
                    }
                    // Drop a locus, which may orphan an edge or a binding.
                    4 => {
                        chromosome.remove(at);
                    }
                    // Bind to an arbitrary channel.
                    5 => chromosome[at] = binding(BIND_IN, NODE_A, (draw(4) % 300) as u16, 1.0),
                    // An arbitrary activation and role.
                    6 => {
                        chromosome[at] = locus(
                            NODE_B,
                            LocusKind::Node {
                                role: NodeRole::Hidden,
                                activation_id: (draw(4) % 8) as u8,
                                bias: 0.5,
                                time_constant: 0,
                            },
                        )
                    }
                    // Close a cycle, with the delay bit left to chance.
                    _ => chromosome.push(edge(
                        STRUCTURAL_HOMOLOGY_BASE + 300,
                        NODE_C,
                        NODE_A,
                        0.5,
                        (draw(4) % 8) as u8,
                    )),
                }
                bytes = subject.encode();
            }
            0 => {
                let edits = 1 + (draw(1) % 6) as usize;
                for edit in 0..edits {
                    let at = (draw(2 + edit as u32 * 2) % bytes.len() as u64) as usize;
                    bytes[at] ^= (draw(3 + edit as u32 * 2) % 256) as u8;
                }
            }
            1 => {
                let at = (draw(1) % bytes.len() as u64) as usize;
                let length = 1 + (draw(2) % 24) as usize;
                for offset in 0..length {
                    if at + offset < bytes.len() {
                        bytes[at + offset] = (draw(3 + offset as u32) % 256) as u8;
                    }
                }
            }
            _ => {
                if draw(1) % 2 == 0 {
                    let keep = (draw(2) % bytes.len() as u64) as usize;
                    bytes.truncate(keep);
                } else {
                    let extra = 1 + (draw(2) % 32) as usize;
                    for offset in 0..extra {
                        bytes.push((draw(3 + offset as u32) % 256) as u8);
                    }
                }
            }
        }

        match Genome2::decode(&bytes, &caps) {
            Ok(decoded) => {
                accepted += 1;
                // An accepted record must re-validate and round-trip. A
                // mutation that happens to produce a *valid* genome is a
                // legitimate accept; one that decodes but cannot re-encode
                // to itself is a codec bug.
                decoded
                    .validate_structure(&caps)
                    .expect("an accepted genome must re-validate");
                let re_encoded = decoded.encode();
                let again = Genome2::decode(&re_encoded, &caps)
                    .expect("a re-encoded accepted genome must decode");
                assert_eq!(again, decoded, "case {case} did not round-trip");
            }
            Err(_) => rejected += 1,
        }
    }

    assert_eq!(accepted + rejected, CASES);
    // The harness must actually be exercising the decoder rather than
    // producing garbage that fails at byte zero every time.
    assert!(
        rejected > CASES / 2,
        "only {rejected} of {CASES} were rejected; the corruption is too gentle"
    );
    // ...and the accept path has to be genuinely exercised, or "every accept
    // re-validated" is a claim about almost nothing. The structural family
    // is a quarter of the cases and produces valid genomes routinely.
    assert!(
        accepted > CASES / 100,
        "only {accepted} of {CASES} were accepted; the structural family is not reaching validation"
    );
    println!(
        "PHASE9-HARNESS cases={CASES} accepted={accepted} rejected={rejected} \
         panics=0 invalid_admissions=0"
    );
}

// --- C9.7: the schema 1 fixture is untouched --------------------------------

#[test]
fn c9_7_a_schema_1_world_still_reproduces_its_fixture() {
    // Schema 2 exists in the build. It must be provably impossible for that
    // to move a schema-1 world, and the cheapest proof is the fixture.
    let config = SimConfig::phase2_default(0x5eed_cafe_f00d_beef);
    assert_eq!(config.stable_hash(), 0xf83d_3981_bf7d_d189);
    let mut world = World::new(config).expect("world");
    for _ in 0..500 {
        world.step();
    }
    assert_eq!(world.state_checksum(), 0xff9d_fcff_5dff_bf42);
}
