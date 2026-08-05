# Spatial Sample Format (ALSS 1)

Status: implemented, Phase 7 (D-060). Normative for the artifact any
spatial analysis reads.

## Why This Is Not In The Event Log

Phase 7's primary endpoint needs organism positions over time. They are
deliberately **not** added to ALEV, for two reasons that would both survive
a change of mind about the index:

- ALEV records *what happened*. A per-tick dump of every coordinate is not
  an event, and admitting one makes the file's "one record, one thing that
  occurred" reading false, which is the property every reconstruction in
  `eventlog.rs` depends on.
- Size. Events are sparse; positions are dense. At the standard tier a
  20,000-tick world writes kilobytes of events and would write hundreds of
  megabytes of positions at the same cadence.

So spatial structure gets its own artifact, written by the experiment
harness rather than the kernel. **The kernel is untouched by this file's
existence**: positions are read through the existing read-only
`World::render_entities_in` observer view, which is why both Phase 1 and
Phase 2 fixtures are provably unmovable by any spatial analysis work.

## Layout

Little-endian throughout, matching ALIF and ALEV.

Header, fixed 72 bytes, followed by the build string:

| Offset | Size | Field |
|---:|---:|---|
| 0 | 4 | magic `ALSS` |
| 4 | 2 | format version (currently 1) |
| 6 | 2 | header length (72) |
| 8 | 4 | flags; any non-zero value is refused |
| 12 | 8 | world id |
| 20 | 8 | seed |
| 28 | 8 | config hash |
| 36 | 8 | terrain checksum |
| 44 | 4 | cells x |
| 48 | 4 | cells y |
| 52 | 4 | cell size, metres |
| 56 | 4 | sample interval, ticks |
| 60 | 4 | max organisms (the world's `max_entities`) |
| 64 | 2 | build string length (at most 64) |
| 66 | 2 | reserved |
| 68 | 4 | header CRC32 |

The header CRC covers bytes 0..68 and the build string. As in ALEV, most of
what this header carries is **provenance rather than framing**, and nothing
further down the file would notice a corrupted seed or terrain checksum, so
the header checks itself.

`terrain_checksum` and `config_hash` exist so an analysis cannot silently
measure the wrong world: the analysis regenerates terrain from the
manifest's embedded campaign source and refuses unless the manifest, the
regenerated terrain, and this header all agree. A land mask taken from the
wrong map would renormalize every index computed from it.

Then zero or more segments in strictly ascending tick order:

| Size | Field |
|---:|---|
| 4 | magic `SPL1` |
| 8 | tick |
| 4 | organism count |
| 4 | body length (must equal count * 8) |
| body | `count` records of `x_fp i32`, `y_fp i32` |
| 4 | CRC32 over the segment header and body |

The segment CRC covers the segment header as well as the body. Covering the
body alone would leave `tick` unchecked, and a flipped bit there moves a
sample in time instead of failing the decode.

## Rules

- **An empty world still writes a segment.** "Nobody was alive at tick T" is
  a measurement; skipping it would make an extinct run and an unsampled run
  decode identically and divide every per-sample rate by the wrong
  denominator.
- **Every declared length is capped before allocation**, against both the
  header's `max_organisms` and an absolute ceiling of 1,000,000.
- **Ticks strictly ascend.** A repeated or reversed tick is an error.
- **Appending is the only supported mutation.** There is no rewrite path and
  no repair path. A torn tail -- what a crash between `write` and `sync`
  leaves -- is reported by the prefix reader with the offset where the file
  stops being trustworthy, and is an error to the strict reader.
- **The manifest records the sample count** for each run, and the analysis
  refuses if the file disagrees, so a silently shortened series cannot be
  analysed as a complete one.

## Producing One

`output spatial <ticks>` in a campaign file. Off by default: only analyses
that measure spatial structure need the artifact, and a campaign that does
not ask for one should not pay for it. Measured cost at the standard tier
with a 50-tick cadence is roughly 30 percent of campaign throughput and
about 660 KB per world per 20,000 ticks.
