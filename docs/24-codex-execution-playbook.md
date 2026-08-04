# Codex Execution Playbook

## Before Any Task

Read README.md, AGENTS.md, CODEX.md, the active phase plan, relevant specifications, open questions, and decision log. Inspect the worktree. Run existing targeted tests before editing. State scope, assumptions, validation plan, and what will not be touched.

## During Work

Implement the smallest accepted vertical slice. Keep kernel/UI/storage/infrastructure boundaries intact. Prefer deterministic pure functions. Update tests and docs with behavior. Record new assumptions. Do not make hardware/network/service changes as an implementation shortcut.

## For Rule Changes

Create a proposed ADR, identify config/schema/replay effects, add before/after deterministic fixtures, run a long-run scenario when dynamics may change, and preserve a compatible path for old worlds or explicitly reject them.

## For Performance Work

Capture the baseline first. State hardware/VM/build/config/seed/observer count. Profile. Change one bottleneck class. Rerun correctness and benchmark suites. Record whether the result improves p50/p95, RSS, allocations, or bandwidth and whether it alters determinism.

## Completion Report

Report completed scope, files changed, tests and benchmarks with results, docs/ADR updates, compatibility/deployment effect, remaining risks, and next smallest backlog item. Never report a claim as verified if a test, host audit, or deployment check was not run.

## Safe Stop

Stop when blocked by a decision affecting persistent data, experimental meaning, credentials, external access, Proxmox, firewall, DNS, backups, or a production service. State the exact blocker and a safe next action.
