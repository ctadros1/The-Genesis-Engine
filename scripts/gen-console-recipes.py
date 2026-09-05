#!/usr/bin/env python3
"""Write apps/console/src/recipes.ts from the campaign files.

The later research phases are not presets: each is a set of sections
switched on inside one configuration, and the experiments turn them on
with `base` lines on top of a preset. A recipe is one campaign's base,
copied verbatim, so the world the console builds from it is the world the
experiment ran. Rerun this script when a campaign base changes; the
console's e2e suite previews every recipe against the server's schema.

    python3 scripts/gen-console-recipes.py
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CAMPAIGNS = ROOT / "experiments"
OUT = ROOT / "apps" / "console" / "src" / "recipes.ts"

# (id, name, description, campaign file or None for a bare preset, preset override)
RECIPES = [
    ("bare-phase2", "Bare phase 2",
     "Phase 2 as shipped: inherited controllers and paired-parent reproduction, nothing else switched on.",
     None, "phase2"),
    ("demography-climate-phase8", "Demography and climate (Phase 8)",
     "Ageing, extrinsic mortality and a climate; 40,000 organisms on the default map. The Phase 8 confirmatory base.",
     "phase8-c81-confirmatory.campaign", None),
    ("variable-genomes-phase9", "Variable genomes (Phase 9)",
     "Genome schema 2 (variable topology) with physiology, 128x128 cells, up to 40,000 organisms: heavy. The Phase 9 confirmatory base.",
     "phase9-c91-confirmatory.campaign", None),
    ("seasons-phase9-probe", "Seasons (Phase 9 climate probe)",
     "Variable genomes under a seasonal environment on a 256x256 map: very heavy. The Phase 9 climate probe base.",
     "phase9-climate-probe.campaign", None),
    ("morphology-phase10", "Morphology (Phase 10)",
     "Bodies of modules on a lattice, grown by development from the genome; 128x128, up to 40,000 organisms. The Phase 10 confirmatory base.",
     "phase10-c103-confirmatory.campaign", None),
    ("plasticity-phase11", "Plasticity and learning (Phase 11)",
     "Lifetime plasticity on the controller, a relocating food patch and the marker probe; 128x128. The Phase 11 confirmatory base.",
     "phase11-c111-confirmatory.campaign", None),
    ("artifacts-phase12", "Artifacts and structures (Phase 12)",
     "Objects, materials, carrying, striking, fracture and terrain yield: the mutable world. Objects do not yet reach the live canvas (ALSP 1.1 is unbuilt); they reach the metrics and analysis routes. 128x128. The Phase 12 confirmatory base.",
     "phase12-artifact-confirmatory.campaign", None),
    ("social-culture-phase13", "Social signals and culture (Phase 13)",
     "The signal channel with observational learning on top of artifacts, plasticity, contest and the food patch: the full stack the culture experiments ran. Signals do not reach the live canvas. 128x128, heavy. The Phase 13 confirmatory base.",
     "phase13-social-confirmatory.campaign", None),
    ("physiology-phase14", "Physiology and ageing (Phase 14)",
     "Morphology with juvenile hazard on the full artifact stack; 128x128, heavy. The Phase 14 confirmatory base.",
     "phase14-physiology-confirmatory.campaign", None),
    ("chemistry-field-phase15", "Chemistry field, scaffold (Phase 15)",
     "The prebiotic field alone on phase 1: substrate, microbial classes and abiogenesis, 40 organisms on 64x64. The Phase 15 field-scaffold base.",
     "phase15-field-scaffold.campaign", None),
    ("transition-phase16", "Transition to life (Phase 16)",
     "A scratch world with no organisms: the field materializes the first bodies. 64x64, 4,000 entities. The Phase 16 confirmatory base.",
     "phase16-transition-confirmatory.campaign", None),
    ("era-tradition-phase17", "Era and tradition (Phase 17)",
     "Variable genomes with plasticity on 64x64, 10,000 entities: the world the era and tradition detectors were validated on. The Phase 17 null-control base.",
     "phase17-era-null.campaign", None),
    ("everything-all-phases", "Everything: all developed phases (128x128)",
     "Every developed mechanism at once: variable genomes, morphology, physiology and ageing, contest, the marker probe, plasticity, the relocating food patch, artifacts and structures, the signal channel with observational learning, the chemistry field as food and the transition. Climate is left off because the validator refuses a capacity-scaled food patch under it. 128x128, up to 40,000 organisms: very heavy - expect a slow tick on the VM.",
     "phase14-physiology-confirmatory.campaign", "everything"),
    ("everything-all-phases-small", "Everything: all developed phases (64x64)",
     "The same full stack on a 64x64 map with 10,000 entities and a 16-cell food patch: the one to spin up interactively.",
     "phase14-physiology-confirmatory.campaign", "everything-small"),
    ("chemistry-field-phase22-base", "Chemistry field (Phase 22 base)",
     "Phase 19 to 23's world: scratch origin, the field as food, the transition, morphology; 64x64, 4,000 entities. The Phase 22 confirmatory base.",
     "phase22-lineages-confirmatory.campaign", None),
    ("youngest-first-phase21-probe", "Youngest first (Phase 21 probe)",
     "The Phase 22 base under the other intake order (the youngest eats first), the probe that lifted born life tenfold.",
     "phase22-lineages-confirmatory.campaign", "descending"),
]


def base_lines(name):
    preset, settings = "phase2", []
    for line in (CAMPAIGNS / name).read_text().splitlines():
        if not line.startswith("base "):
            continue
        parts = line.split(None, 2)
        if parts[1] == "preset":
            preset = parts[2].strip()
        else:
            settings.append((parts[1], parts[2].strip()))
    return preset, settings


def everything(phase14, small):
    """Phase 14's full stack plus the field, the transition, the signal
    channel and plasticity, as Phases 13 and 16 configured them. The
    validator settles what composes: climate stays off (a capacity-scaled
    food patch is inert under it), and the small variant shrinks the patch
    to fit its map."""
    settings = dict(phase14)
    _, phase16 = base_lines("phase16-transition-confirmatory.campaign")
    for field, value in phase16:
        if field.startswith(("chemistry.", "transition.")):
            settings[field] = value
    _, phase13 = base_lines("phase13-social-confirmatory.campaign")
    for field, value in phase13:
        if field.startswith(("social.", "plasticity.")) or field == "genome2.mutation.plasticity_enabled":
            settings[field] = value
    if small:
        settings.update({"cells_x": "64", "cells_y": "64", "max_entities": "10000",
                         "initial_organisms": "120", "worldmod.patch_radius_cells": "16"})
    return list(settings.items())


def ts_string(value):
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def main():
    out = [
        "// GENERATED by scripts/gen-console-recipes.py from experiments/*.campaign on",
        "// 2026-09-04. Do not edit by hand: rerun the script. Each recipe is one",
        "// campaign's `base` lines copied verbatim, so the world it builds is the",
        "// world that experiment ran; the later phases are sections switched on",
        "// inside one configuration, not presets, which is why the preset list has",
        "// two entries and this list has many.",
        "",
        "export interface Recipe {",
        "  id: string;",
        "  name: string;",
        "  description: string;",
        '  preset: "phase1" | "phase2";',
        "  settings: Record<string, string>;",
        "}",
        "",
        "export const RECIPES: Recipe[] = [",
    ]
    for rid, name, description, campaign, override in RECIPES:
        preset, settings = ("phase2", []) if campaign is None else base_lines(campaign)
        if override == "descending":
            settings = settings + [("physiology.intake_order", "descending")]
        elif override in ("everything", "everything-small"):
            settings = everything(settings, small=override == "everything-small")
        elif override:
            preset = override
        out.append("  {")
        out.append(f"    id: {ts_string(rid)},")
        out.append(f"    name: {ts_string(name)},")
        out.append(f"    description: {ts_string(description)},")
        out.append(f"    preset: {ts_string(preset)},")
        out.append("    settings: {" if settings else "    settings: {},")
        for field, value in settings:
            out.append(f"      {ts_string(field)}: {ts_string(value)},")
        if settings:
            out.append("    },")
        out.append("  },")
    out.append("];")
    out.append("")
    OUT.write_text("\n".join(out))
    print(f"wrote {OUT.relative_to(ROOT)}: {len(RECIPES)} recipes")


if __name__ == "__main__":
    main()
