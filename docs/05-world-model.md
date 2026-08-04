# World Model

## Phase 1 Implementation Status

`sim-core`'s generator (`lifesim-worldgen-v1`) implements the Phase 1 subset:
low-frequency lattice noise with bilinear interpolation, radial falloff to a
single bounded continent, a forced water rim, largest-connected-component
masking, an elevation-suitability food-capacity field, and validation of land
fraction, habitable-cell count, and a reproducible terrain checksum. Biome
classification, moisture, temperature, drainage, day/night, seasons, hazard
fields, and world editing are not implemented yet and remain specified below
for later phases.

## World Representation

The world is a bounded continent surrounded by water. It combines static generation layers with dynamic environmental layers:

| Layer | Resolution | Persistence | Notes |
|---|---|---|---|
| Elevation | Raster | Static after generation unless edited | Drives coastlines, drainage, temperature lapse |
| Land/water mask | Raster | Derived/static | Coast is a collision and resource boundary |
| Biome | Raster | Derived, may slowly transition later | Initial classification from elevation/temp/moisture |
| Temperature | Raster | Dynamic | Seasonal and weather-influenced |
| Moisture | Raster | Dynamic | Rainfall, evaporation, drainage approximation |
| Food biomass | Raster | Dynamic | Renewable vegetation/resource layer |
| Hazard fields | Raster | Dynamic/optional | Fire, storm, drought, or pollution variants |
| Spatial buckets | Derived | Rebuilt/updated | Organism neighbor/proximity lookup |

## Procedural Generation

Generation uses a world seed and versioned generator config:

1. Generate low-frequency continental elevation with deterministic noise and masks.
2. Select a guaranteed connected landmass and surround it with ocean.
3. Derive water, coast, slope, and drainage directions.
4. Generate climate baseline from latitude proxy, elevation, coast distance, and deterministic variation.
5. Derive initial moisture, biomes, food carrying capacity, and spawn zones.
6. Validate invariants: land percentage, connected usable region, water boundaries, non-empty habitable cells, and reproducible checksum.

Do not use ambient random libraries in generation. Store generator version and all effective parameters in world metadata.

## Biomes And Resources

Initial biomes should be intentionally few: coast, grassland, forest, wetland, arid land, highland, and water. Each biome supplies constraints and resource behavior rather than scripted organism roles. A cell's food capacity is a function of biome, moisture, temperature, and disturbance. Expand biome richness only after baseline ecology is stable.

## Day, Night, And Seasons

Day/night primarily affects visibility, temperature, and activity opportunity; it is not a separate physics engine. Seasons affect temperature, rainfall probability, food growth, and reproductive readiness. All periods and amplitudes are per-world config. The observer may render a day/night tint but must display simulation phase clearly in scientific overlay mode.

## Disasters

Rare configurable disasters are event-driven, scoped, and auditable. Examples: drought, wildfire, storm, cold snap, and food blight. Each has a seed, spatial extent, duration, intensity, and start/end event. The product-direction default is rare, bounded disasters enabled in the mature high-realism world profile. They remain disabled because the implementation does not exist yet, and they are not scheduled in Phases 5 through 16. A future disaster slice requires long-run recovery and extinction evidence first, and its own plan entry.

## World Editing By Organisms (Phase 11)

Distinct from sandbox intervention: from Phase 11 organisms themselves
modify the world, and the modification persists after they die. Design in
`specifications/mutable-world-state.md`; decision in ADR-0015.

Mutable layers in the first slice, deliberately few:

| Layer | Effect |
|---|---|
| Traversability override | Blocks or permits movement through a cell |
| Food capacity override | Raises or lowers a cell's carrying capacity |
| Material yield | Remaining extractable material; depleted by striking, regenerating on a configured schedule |

**Elevation stays immutable.** It feeds coastline derivation, drainage, and
the temperature lapse term, and the generator validates land fraction and
connectivity against it. Making it mutable means revalidating those
invariants every tick or accepting that a world can be modified into an
invalid configuration. Deferred, not permanently excluded.

The acceptance criterion "same generator config and seed produces identical
terrain-layer checksums in strict mode" still holds, and it now applies to
the **baseline**. The composed world carries a second checksum over
baseline plus the stored modification delta, and both are verified on
restore.

Generator invariants (land fraction, connected habitable region, water
boundary) are validated against the baseline at generation time only. They
are not revalidated against the composed world, because organisms are
permitted to make the world worse for themselves. An organism population
that renders its region uninhabitable and goes locally extinct is a
legitimate outcome; extinction is already a valid, savable, observable,
latched state.

## World Editing By Operators

Sandbox edits are server-validated interventions. A terrain edit, weather change, food injection, or organism spawn records actor, tick, request, accepted effect, config version, and replay effect. An intervention cannot mutate a loaded save in-place without creating a new branch/world lineage.

## Acceptance Criteria

- Same generator config and seed produces identical terrain-layer checksums in strict mode.
- Invalid maps fail validation with actionable errors.
- Coast and world bounds cannot produce out-of-range accesses.
- Resource fields remain within configured bounds across a long-run test.
