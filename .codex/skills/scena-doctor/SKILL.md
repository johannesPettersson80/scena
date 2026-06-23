---
name: scena-doctor
description: Use when adding, changing, or reviewing scena doctor checks, validation gates, silent-failure prevention, source-derived architecture rules, or checklist enforcement.
---

# Scena Doctor

## Purpose

The doctor prevents known silent-drift families from returning. It is not a replacement for
unit tests, rendered-output proof, browser checks, or release gates.

## Commands

Run the narrowest relevant doctor during development and the full doctor before handoff on
`scena-builder`. Do not use `doctor --full` as a substitute for a focused reproducer: first
prove the bug or drift with the smallest test/check that can fail, then run the doctor gate
that enforces the recurring family.

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --docs'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --architecture'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --full'
```

Use `scena-remote-builder` before running doctor if local work must be synced to the remote
checkout.

## Workflow

1. If a review or bug exposes a silent-failure family, ask whether the pattern can be
   checked from source, docs, manifests, or gate artifacts.
2. Add the doctor rule with a known-bad fixture or clearly failing condition when practical.
3. Run the doctor before and after the fix when changing enforcement behavior.
4. Keep doctor findings fail-closed. Waivers need an ADR or release-note entry with owner,
   expiry, affected rule, user-visible risk, and replacement evidence.

If only a doctor pin or checklist guard changed, the normal scoped gate is the relevant
doctor command plus any formatting check required by Rust edits. Do not run unrelated cargo
test suites unless the doctor change also touched production behavior.

For a multi-finding cleanup, add doctor coverage only after the focused proof for that
finding exists or the finding is source-checkable by doctor alone. Run the relevant doctor
gate for the new rule, then defer full release validation to the batch checkpoint unless the
doctor edit also changes public behavior.

## Current Rule Families

- `doctor --docs`: required docs, local links, stale contract names, and canonical contract
  anchors.
- `doctor --architecture`: required module files, renderer scope boundaries, module
  boundary drift, backend vocabulary, SOLID/KISS guardrails, unit-test-first governance, and
  AGENTS validation.

## Expansion Targets

- Lifecycle: no hidden fetch, first GPU upload, or shader compile inside `render()`.
- Errors: required fallbacks use structured diagnostics or errors.
- glTF: extension matrix, anchors, stale imports, reload, and animation mixer contracts.
- Visual: screenshot metadata, tolerance files, default environment hashes, reference
  artifacts.
- Platform: capability JSON, WASM size, surface/context-loss artifact shape.
- API: public API diff and semver checks once the M5 baseline exists.
- SOLID/KISS: generated dependency graph checks, fan-in/fan-out thresholds, abstraction
  allowlists, and module-size reports once real implementation exists.
- Unit-test-first: source-to-test ownership mapping and red/green evidence artifacts once
  implementation checklists start producing gate artifacts.
