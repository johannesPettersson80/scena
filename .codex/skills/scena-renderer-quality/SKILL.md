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

The Hetzner `scena-builder` host is the default CPU compile/test runner, not a real GPU
proof machine. Use it for Rust tests, doctor, headless CPU proof, and compile gates. Use a
real GPU machine when the proof depends on hardware-accelerated WebGPU/WebGL2 behavior.

## Remote Gate Rule

Use `scena-remote-builder` for all cargo compile/test/doctor gates. Keep the remote checkout
matched to the work being validated, then run command shapes such as:

```bash
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo test'
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-<task-slug>" && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-<task-slug>" cargo run -p xtask -- doctor --full'
```

Use the remote-builder validation ladder. Start with the focused visual/test proof that can
fail on the exact bug, then add scoped gates for the touched surface. Full release gates are
required for cross-backend renderer behavior, browser/WASM-visible changes, public API/schema
changes, or release-ready work; they are not the default for every test-only proof update.

Do not run local cargo/build/test/browser/render proof unless the user explicitly permits it.
For visual work, local image inspection is allowed only as inspection of existing artifacts,
not as a substitute for the required remote or real-GPU proof.

## Investigation Circuit Breaker

Classify a failure as product, harness, environment, policy, or provenance before patching.
After two remedies produce the same signature, stop changing code and build a smaller
discriminating probe. At 30 minutes, record the exact signature, elapsed time, attempted
remedies, and missing evidence. Do not broaden test scope to manufacture confidence.

Wall-clock thresholds are strict only on controlled or dedicated performance hardware.
GitHub-hosted runners must set `SCENA_M9_TIMING_POLICY=report-only-hosted`; sample-count and
allocation budgets remain strict. Hardware browser proof requested from a user must be a
single versioned bundle executed by `scripts/run_windows_complete_hardware_proof.ps1` with
automatic artifact upload. A second user-assisted run requires explicit approval after a
root-cause checkpoint.

## Unit Test First Workflow

1. Identify the contract from the spec/checklist.
2. Add or update the smallest unit or integration test that fails on the missing behavior.
3. Run the focused test and verify the failure is the expected failure on `scena-builder`.
4. Patch production code.
5. Rerun the focused test, then the required cargo and doctor gates.

Checklist items are not complete until the test-first evidence or a documented exception is
recorded.

For proof-only hardening, the proof must still be real: it should fail on the old broken
output or missing contract. But once that focused proof passes, do not keep running unrelated
test suites unless the edit touched production behavior or the checklist/release gate
requires it.

For multi-item visual work, keep a per-item proof ledger. Each item should name the exact
artifact or metric that went red, the production change that made it green, and the scoped
gate that protects it. Run broad visual/browser/release gates at the integration checkpoint,
not after every proof-only edit.

## Smarter Render Validation

Use measurement before breadth:

- First locate the failing render path: CPU vs GPU, WebGL2 vs WebGPU, recipe vs Rust API,
  browser page vs headless CLI, overlay vs geometry vs material.
- Add the smallest rendered-output proof that can fail on that path. Examples: one crop
  diff, one pixel movement assertion, one edge metric, one CPU/GPU parity comparison, or one
  browser canvas probe.
- Keep thresholds tied to the defect being fixed. A broad `ok:true`, nonblack, draw-count,
  or before/after DOM change is not enough when the bug is visual.
- If the first proof passes while the human-visible defect remains, the proof is wrong.
  Replace the proof with a measurement of the actual defect before touching production code.
- If the root cause is unclear, do not run more unrelated suites. Split the scene by render
  path or element type until the measurement localizes the failing path.
- After the focused render proof passes, run only the scoped gate that can catch regressions
  in the changed path. Save full cross-backend/browser/release gates for cross-backend
  changes, public behavior changes, or explicit release checkpoints.

## Browser Capture Diagnostics

For an empty, stale, slow, or hanging browser capture/readback, add a disabled-by-default
structured trace before changing lifecycle behavior. Gate it behind an explicit diagnostics
flag and record capture-pass start/completion, backend and readback mode, surface copy
capability/formats, map start/completion, byte length, nonzero byte or pixel count,
deterministic frame hash, and render/drain/map/total elapsed time.

The focused browser harness must enable the trace, preserve it on failure, and assert a real
rendered-frame invariant. A nonblack count alone is insufficient for appearance, mutation,
or parity defects. Once the cause is proven, keep only low-overhead gated diagnostics and add
doctor coverage for mechanically checkable capture-mode or source contracts.

## Quality Language

Do not claim "pixel-perfect" across backends. Use deterministic per backend with documented
tolerances.

## Doctor Rule

When a rendering, browser, resource-lifetime, dirty-state, or capability bug exposes a
silent-failure family, add or extend `cargo run -p xtask -- doctor --full` coverage if the
family can be checked mechanically.
