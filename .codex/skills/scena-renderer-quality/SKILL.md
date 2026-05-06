---
name: scena-renderer-quality
description: Use when adding or reviewing scena tests, visual proof, browser/WASM checks, headless screenshots, color/capability validation, resource lifetime tests, dirty-state tests, allocation gates, or benchmark evidence.
---

# Scena Renderer Quality

## Required Evidence Types

- Test-first proof: add or update the focused unit/integration test before production
  implementation, run it red for the expected reason, then make it green.
- Example compile tests for every public example.
- Headless screenshot/pixel tests with documented per-backend tolerance.
- Browser rendered-output checks for WASM/WebGPU/WebGL2 paths.
- Resource lifetime tests proving counters return to baseline.
- Dirty-state tests for transforms, materials, instances, camera, resize, hover, selection.
- Allocation/steady-state tests for post-`prepare()` common mutations.
- Capability matrix tests where hardware/backend availability permits.
- Doctor checks for known silent-failure families that can be caught from source, docs,
  manifests, or gate artifacts.

## Browser/Visual Rule

Do not declare a browser-visible or WebGL/WebGPU rendering fix from unit tests alone.
Capture rendered output and assert pixels/canvas state or screenshot differences.

## Unit Test First Workflow

1. Identify the contract from the spec/checklist.
2. Add or update the smallest unit or integration test that fails on the missing behavior.
3. Run the focused test and verify the failure is the expected failure.
4. Patch production code.
5. Rerun the focused test, then the required cargo and doctor gates.

Checklist items are not complete until the test-first evidence or a documented exception is
recorded.

## Quality Language

Do not claim "pixel-perfect" across backends. Use deterministic per backend with documented
tolerances.

## Doctor Rule

When a rendering, browser, resource-lifetime, dirty-state, or capability bug exposes a
silent-failure family, add or extend `cargo run -p xtask -- doctor --full` coverage if the
family can be checked mechanically.
