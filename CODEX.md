# Codex Operating Guide

## Start A Work Session

1. Read README.md, AGENTS.md, and this file.
2. Read the active phase plan and its dependencies.
3. Review docs/22-decision-log.md, docs/21-open-questions.md, and relevant specifications.
4. Inspect the working tree and run existing focused tests.
5. Write a short implementation plan before edits unless the task is a trivial, isolated correction.

## Identify The Active Phase

The current project status is declared in README.md and planning/backlog.md. Do not begin the next phase until the previous phase acceptance criteria, documentation updates, and benchmark gates are satisfied or explicitly waived in a decision record.

## Choosing The Next Task

Choose the smallest backlog item that:

- advances an accepted phase deliverable,
- has explicit acceptance criteria,
- does not require unresolved high-risk decisions,
- can be validated with current tools, and
- does not widen deployment scope accidentally.

Prefer a thin vertical slice over a framework-first buildout. Do not implement later ecosystem, GPU, database, or public-access features early.

## Validation

Run format/lint/type checks, targeted tests, deterministic tests where relevant, and phase-required tests. For observer work, validate desktop and mobile viewports and avoid assuming one rendering backend. For persistence or protocol changes, add compatibility/negative tests.

## Documentation Discipline

Whenever implementation changes behavior or architecture:

1. Update the affected docs/ and specifications/ documents.
2. Add or revise a proposed ADR.
3. Update the decision log and benchmark record.
4. Explain replay, migration, API, and deployment impact in the completion report.

## Safe Stop Conditions

Stop and ask for clarification when a choice affects persistent data compatibility, experimental interpretation, access control, production infrastructure, or requires secrets. Otherwise document a bounded assumption and continue.

## Preventing Context Loss

At the end of a meaningful session, leave the active phase, completed acceptance criteria, test evidence, benchmark evidence, changed invariants, and next smallest task in planning/backlog.md or the decision log. Do not rely on chat history as project memory.
