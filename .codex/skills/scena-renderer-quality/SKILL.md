---
name: scena-renderer-quality
description: Use when adding or reviewing scena tests, visual proof, browser/WASM checks, headless screenshots, color/capability validation, resource lifetime tests, dirty-state tests, allocation gates, or benchmark evidence.
---

# Scena Renderer Quality

## Required Evidence Types

- Example compile tests for every public example.
- Headless screenshot/pixel tests with documented per-backend tolerance.
- Browser rendered-output checks for WASM/WebGPU/WebGL2 paths.
- Resource lifetime tests proving counters return to baseline.
- Dirty-state tests for transforms, materials, instances, camera, resize, hover, selection.
- Allocation/steady-state tests for post-`prepare()` common mutations.
- Capability matrix tests where hardware/backend availability permits.

## Browser/Visual Rule

Do not declare a browser-visible or WebGL/WebGPU rendering fix from unit tests alone.
Capture rendered output and assert pixels/canvas state or screenshot differences.

## Quality Language

Do not claim "pixel-perfect" across backends. Use deterministic per backend with documented
tolerances.
