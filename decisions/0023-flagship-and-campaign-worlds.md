# ADR-0023: Flagship Worlds And Campaign Worlds

Status: Proposed
Date: 2026-08-04
Author: Long-horizon scope revision

## Context

The plan built through Phases 5 to 16 optimizes for one thing: a research
instrument. Thirty to fifty short independent worlds per condition, run
headless as fast as the host allows, answering falsifiable questions with
controls and stated effect sizes. Phase 5 delivered exactly that.

The stated product intent is different and was never written down: **one
world, left running for a long time, watched as it develops.** Not a fast
game, and not something that reaches a ceiling in an hour.

These are not in conflict scientifically, but they are in conflict for
compute, for retention policy, and, most importantly, for what a result
means. Without a written boundary the two blur, and the predictable failure
is a screenshot from the long-running world being described as a finding.

There is no risk of the opposite problem. At the measured Phase 2 rate of
roughly 1,575 ticks per ancestry generation, a world at 1x reaches about 23
generations per hour. Nothing is going to be solved in an afternoon; see
`docs/27-time-scale-and-pacing.md`.

## Options Considered

- **Campaign worlds only.** Keeps the evidence discipline pure and declines
  the stated product intent.
- **One long world only.** Delivers the experience and abandons every
  multi-seed acceptance criterion in the plan, since n=1.
- **Two named run modes over one unchanged kernel**, with an explicit
  boundary on what each may claim.

## Proposed Decision

Adopt two run modes. **Neither changes kernel semantics.** They differ in
scheduling, retention, observation cadence, and evidentiary standing.

| | Campaign world | Flagship world |
|---|---|---|
| Count | 30 to 50 per condition | One, or a small named set |
| Horizon | Bounded by the campaign design | Open-ended, months |
| Speed | Maximum headless | Operator's choice, 1x default |
| Lifetime | Disposable after analysis | Preserved indefinitely, backed up |
| Retention | Campaign manifest, then prunable | Full checkpoint history retained |
| Observation | Post-hoc analysis of the whole set | Continuous, plus check-in reports |
| **May support a claim?** | **Yes, that is its purpose** | **No. n=1** |

### The boundary, stated so it cannot erode

A flagship world produces **observations and narrative**. A campaign
produces **evidence**. A flagship world may generate a hypothesis; it may
never confirm one.

This is the same discipline `docs/25-emergence-and-epistemic-position.md`
already applies to the difference between what the simulation makes
possible and what we predict, and the same one ADR-0016 applies to
analysis. The concrete rules:

- Any claim sourced from a flagship world states the run mode and the
  sample size of one, in the same sentence as the claim.
- A flagship observation that looks important becomes a **campaign
  proposal**, not a result. The campaign runs on its own seed set, with the
  flagship excluded from it so the observation that motivated the design is
  not also its evidence.
- Reports from the two modes use different templates and are not merged.
- The era and tradition detection of Phase 16 is available in both modes,
  but a tradition finding in a flagship world is a candidate, not a
  finding, and its report says so.

### Compute is allocated, not assumed

Flagship and campaign worlds draw on the same host. The allocation is an
explicit config decision recorded per period, not an emergent consequence of
whatever is running. A flagship world running continuously is a standing
reservation and reduces campaign throughput by that amount.

This matters more than it sounds: the compute ceiling is already recorded as
unresolved in the risk register, and ADR-0022 A6 raised seed counts to 30
and 50. A permanent flagship reservation makes campaigns slower, and the
honest response is fewer concurrent campaigns rather than a quietly
shortened flagship.

### What a flagship world requires that a campaign does not

- **Long-horizon soak evidence** before it is trusted to run unattended.
  See `specifications/long-horizon-soak.md`. This is the prerequisite, and
  it is currently unmet: the longest verified run is 864,000 ticks.
- **A retention and backup policy** distinct from campaign pruning, since
  checkpoints are the only record of a world that will never be re-run.
  A flagship world cannot be reproduced by re-running its seed once it has
  received any intervention, so its checkpoint chain is the artifact.
- **A check-in report**: what changed since the last observation, derived
  from the event log, so the operator can be away for a week and still
  follow the world. This is Phase 16 machinery pointed at a different
  question.
- **An intervention log with the same audit discipline as Phase 3
  controls**, because a flagship world is the one most likely to be poked.

## Consequences

Positive: the stated product intent gets a home; the evidence discipline
survives contact with it; the compute conflict becomes visible and
schedulable instead of implicit.

Negative and accepted:

- A permanent flagship reservation slows every campaign.
- The soak prerequisite is real work that produces no science.
- Two report templates and two retention policies to maintain.
- **The temptation to claim from the flagship world will be constant**,
  because it is the world with the interesting history and the screenshots.
  The mitigation is a written rule and a report template, which is weaker
  than a structural guarantee. This is the risk most likely to be violated
  by a future session in a hurry.

Compatibility: no kernel, schema, protocol, or fixture impact. Run mode is
config and operational policy.

## Performance Implications

A flagship world at 1x uses a fraction of one core; the cost is that it
occupies a slot continuously and holds its checkpoint history. Campaign
throughput reduces by the reserved share. Both are measured under Phase 5's
existing scheduler instrumentation.

## Operational Implications

Backup becomes load-bearing rather than precautionary: a flagship world's
checkpoint chain is irreplaceable in a way a campaign world's is not. The
existing restore-from-backup discipline applies with a higher stake.

## Revisit Conditions

- The soak criteria fail, meaning unattended long-horizon operation is not
  yet supported and the flagship mode is premature.
- A flagship observation is found in a report as a claim, indicating the
  written boundary is insufficient and needs a structural one.
- Compute pressure makes the standing reservation untenable.

## Evidence Required To Accept

- `specifications/long-horizon-soak.md` criteria passing at the 30-day
  equivalent.
- A check-in report generated from a real multi-week run.
- User approval of the compute split.
