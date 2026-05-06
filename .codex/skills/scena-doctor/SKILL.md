---
name: scena-doctor
description: Use when adding, changing, or reviewing scena doctor checks, validation gates, silent-failure prevention, source-derived architecture rules, or checklist enforcement.
---

# Scena Doctor

## Purpose

The doctor prevents known silent-drift families from returning. It is not a replacement for
unit tests, rendered-output proof, browser checks, or release gates.

## Commands

Run the narrowest relevant doctor during development and the full doctor before handoff:

```bash
cargo run -p xtask -- doctor --docs
cargo run -p xtask -- doctor --architecture
cargo run -p xtask -- doctor --full
```

## Workflow

1. If a review or bug exposes a silent-failure family, ask whether the pattern can be
   checked from source, docs, manifests, or gate artifacts.
2. Add the doctor rule with a known-bad fixture or clearly failing condition when practical.
3. Run the doctor before and after the fix when changing enforcement behavior.
4. Keep doctor findings fail-closed. Waivers need an ADR or release-note entry with owner,
   expiry, affected rule, user-visible risk, and replacement evidence.

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
