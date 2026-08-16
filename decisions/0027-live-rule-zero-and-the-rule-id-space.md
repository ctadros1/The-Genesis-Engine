# ADR-0027: Removing The Dead Rule Value Narrows The Id Space, It Does Not Redirect It

Status: Proposed
Date: 2026-08-15
Author: Coding agent (autonomous session)

## Context

D-107 adopted option A3 for shortening the plasticity conjunction: **remove
the dead value from the rule id space so every `rule_id` names a live rule and
mutation target 2 can no longer be spent on nothing.** It rejected option A1 -
making `rule_id` 0 name a live rule - because "choosing *which* rule sits at 0
authors which learning form evolution starts adjacent to", and ADR-0012's test
is whether you can name the outcome a mechanism makes more likely.

The backlog then specified the implementation as a runtime flag,
`plasticity.live_rule_zero`, with the rule "remapped ONCE in
`compile_with_budget` so `plasticity::step` stays a pure function of the rule
it is handed". ALIF format 5 has since reserved the config byte
(D-108); the flag is encoded, `validate` refuses `true`, and nothing reads it.

**A compile-time remap alone cannot deliver what A3 asks for, and the gap is
not cosmetic.** The relevant machinery:

- `structmut.rs:633` draws a fresh rule as
  `draw(10) % PLASTICITY_RULE_COUNT`, uniform over 5 values.
- `genome2.rs:257` reduces a stored id at **expression** time as
  `rule_id % PLASTICITY_RULE_COUNT`.
- `plasticity.rs` defines `RULE_STATIC = 0` as the dead value and rules 1-4 as
  Hebbian, Oja, modulated Hebbian, and eligibility trace.

A remap confined to `compile_with_budget` receives a value already reduced
mod 5 and must map five equiprobable inputs onto four live rules. **Any such
map makes one rule twice as likely as the others**, which is A1's objection
wearing a different hat: it does not merely make a rule reachable, it makes
one rule the most likely single outcome of a rule mutation.

## Options Considered

- **(a) The flag narrows the id space: the draw and the expression reduction
  both become mod 4, and `compile_with_budget` maps `r` to live rule `r + 1`.**
  Four live rules, 25 percent each.
- **(b) Compile-time remap only, `r -> (r % 4) + 1`.** Narrow, one call site.
  Rule 1 gets 40 percent, rules 2-4 get 20 percent each.
- **(c) Treat id 0 as a fifth live rule duplicating one of the four.**
  Identical distribution to (b), stated more honestly.
- **(d) Reject-and-redraw on id 0.** Uniform, but consumes a variable number
  of RNG draws, which `specifications/determinism-extensions.md` forbids: a
  stream position must not depend on a value.
- **(e) Break the tie with an already-available genome property, e.g.
  `homology_id`.** Deterministic and consumes no new draws, but
  `homology_id` is heritable, so it correlates rule identity with lineage -
  inventing a linkage that does not exist.

## Proposed Decision

**Adopt (a).** When `plasticity.live_rule_zero` is set, the structural
mutation draw is uniform over 4 values instead of 5, and
`compile_with_budget` maps the expressed id `r` to live rule
`(r % 4) + 1`. When the flag is clear, everything is exactly as it is today.

The scope is the id space's **width**, not its contents. No rule is
introduced, removed, reordered, or preferred.

### The expression-time reduction does not need to change, and that was worth
### checking before writing the code

The first draft of this ADR said the effective count had to reach both the
draw and `PlasticityGenes::normalized` - eight call sites across
`structmut.rs`, `genome2.rs`, and `develop.rs`. Tracing where a `rule_id`
can actually come from shows that is unnecessary:

- **`structmut.rs:676` is the only place a fresh `rule_id` is ever produced**
  (`plasticity.rule_id = fresh_rule`, from the draw at line 633). Every other
  construction is a test fixture, a decode, or a crossover that copies an
  allele that already existed.
- The founder's `PlasticityGenes::default` is `rule_id: 0`.
- Duplication, insertion, and transposition copy existing loci. Crossover
  mixes existing alleles. Point mutation on any other field leaves `rule_id`
  alone.

So with the flag set from tick 0, every `rule_id` in circulation is in
`0..4`, and `normalized`'s `% 5` is the identity on all of them. Changing it
would be a no-op that touched three modules.

The `% 4` in the compile-time map is therefore a **clamp for values that
cannot arise under the flag**, not a distribution choice. Two ways one could:
a save written with the flag clear and reloaded with it set, or a `seeded`
origin-mode founder set carrying an arbitrary id. Neither occurs in the 2x2,
which starts fresh worlds with the flag fixed per arm. **Uniformity therefore
holds exactly where the experiment reads it**, and legacy ids fail safe onto
a live rule rather than out of range. If a future campaign does reload across
the flag, the non-uniformity that introduces must be reported, because at
that point the clamp *is* a distribution choice.

`plasticity::step` still receives a rule and stays a pure function of it,
which is the property the original one-call-site instruction was protecting.

### Why (b) is rejected on evidence rather than on taste

The doubled rule under `r -> (r % 4) + 1` is rule 1, plain Hebbian. The
commissioned lifetime-learning review
(`.agents/skills/genesis-neuroevolution/references/`, section 6.3) is explicit:

> A plain unbounded Hebbian rule should not be a production default. It is
> valuable as an ablation and baseline.
> **Recommendation strength: Strongly supported as a baseline; unsupported as
> the sole production rule.**

So (b) does not merely author a preference - it authors a preference for the
one rule the review names as the wrong default, and the one with the worst
runaway-weight failure mode. A 2x2 arm biased toward the least stable rule
could produce or destroy its own effect. AGENTS.md requires consulting this
review before writing a plasticity criterion or ADR; this is what it said.

## Consequences

**Genome validity does not become config-dependent, and this is worth stating
because the obvious worry is wrong.** `PlasticityGenes::normalized` reduces
rather than rejects, deliberately: its doc records that a stored `rule_id = 7`
must stay decodable, because "these fields are *mutation targets*, and a
`rule_id` that had to name a registry entry in order to decode would make most
rule-id mutations lethal for a reason that has nothing to do with learning".
Under a narrower modulus a stored id is still total - every bit pattern still
names some rule. **No genome becomes invalid in either arm.**

**What does change is that the two arms do not share a genotype-to-phenotype
map.** A genome storing `rule_id = 7` expresses as rule 2 with the flag clear
(`7 % 5`) and as rule 3 with it set (`7 % 4`, then `+1`). Consequences:

- A genome may not be transplanted between arms and read as the same
  organism. The 2x2 compares seed-matched *worlds*, not genotypes, so its
  design is unaffected - but any future cross-arm genome comparison is
  meaningless and must be refused rather than interpreted.
- The founder is unaffected. D-107's measured claim is that with `eta == 0`
  the learned state and trace stay at zero under every rule id
  (`with_eta_zero_every_rule_in_the_registry_leaves_the_learned_state_alone`),
  so the founder is inert in both arms and the arms start identical.

**RNG streams are untouched.** The draw stays one call at position 10 in the
same named stream; only the modulus applied to its value changes. No stream is
renumbered, so fixtures `0x1e3158a26afd3b39` and `0xff9dfcff5dffbf42` cannot
move for this reason.

**Blast radius.** `normalized()` has eight call sites across `structmut.rs`,
`genome2.rs`, and `develop.rs`, and the effective count must reach all of
them plus the draw. That is wider than "remap once in `compile_with_budget`"
and is the honest cost of the uniformity the decision buys. `plasticity::step`
still receives a rule and stays a pure function of it, which is the property
the original instruction was protecting.

**The config hash moves when the flag is set**, and only then (D-014 at field
granularity). A world with a live rule 0 is a different experiment and a new
replay lineage. With the flag clear the hash is byte-identical and the Phase 11
fixture does not move.

## Performance Implications

None expected. The change is a modulus and an addition on a path already
executed once per plastic edge at compile time, not per tick. No benchmark
claim is made; if `compile_with_budget` shows in a profile after the change,
that is a finding to record, not an expectation.

## Operational Implications

None. No infrastructure, no public exposure, no stored-data reinterpretation:
existing files decode as they always did, because the flag is absent from
every format-4 file and false in every format-5 file written before it is
implemented.

## Revisit Conditions

- Measured evidence that the four live rules are **not** exchangeable at the
  founder - that is, that `eta == 0` does not make the founder inert under
  some rule id - which would make a uniform draw itself a choice about where
  evolution starts and reopen A1's objection against this option too.
- A fifth live rule, or any change to `RULE_COUNT`, which changes the
  arithmetic this ADR is built on.
- Evidence that the doubled-Hebbian bias of option (b) is immaterial to the
  2x2's outcome, which would make the narrower implementation preferable on
  blast radius alone.

## Evidence Required To Accept

- A test showing the rule distribution is uniform over the four live rules
  with the flag set, and unchanged with it clear, driven from the real
  structural-mutation path rather than from a reimplementation of the draw.
- A test showing the founder compiles to an inert controller under both
  settings, so the arms start identical.
- Mutation testing of both, run by someone who did not write them.
- The Phase 1, Phase 2, Phase 9 and Phase 11 fixtures unmoved with the flag
  clear, asserted rather than argued.
- The full workspace suite and all six verify scripts green on both hosts.
