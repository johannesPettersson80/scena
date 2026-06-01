# SceneHost browser/GPU proof plan

Status: open
Date: 2026-06-01
Scope: real browser/GPU rendered-output proof for the generic `SceneHost`
contracts.

Real browser/GPU machine required. The Hetzner CPU builder is valid for Rust
compile, test, doc, doctor, WASM compile, and headless capture proof. It is not
valid evidence for WebGPU/WebGL2 rendered output.

## Contract

The proof artifact schema is `scena.scene_host_browser_proof.v1`.

Required output artifacts:

- `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`
- `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.png`

The JSON artifact records:

- backend and capability report JSON,
- browser name/version and operating system,
- viewport size and device-pixel ratio,
- asset URLs and the `scena.scene_host_asset_import.v1` reports,
- `inspectJson()` output,
- `annotationProjectionsJson()` output,
- `SceneHost.capture()` descriptor and RGBA8 byte length/hash,
- pick result for a CSS-pixel coordinate at device-pixel ratio other than 1,
- screenshot path and SHA-256.

## Acceptance

- Build the WASM package with the `scene-host` feature.
- Open a real browser canvas backed by WebGPU or WebGL2.
- Construct a multi-part scene through `addEmpty`,
  `instantiateUrlUnderWithReportJson`, and `instantiateUrlUnder`.
- Push transforms through `setTransforms`; the host must not own the render
  cadence.
- Call `prepare()` and `render()`.
- Assert nonblank rendered pixels in the canvas screenshot and in the
  `SceneHost.capture()` RGBA8 payload.
- Assert the same host node handle appears in `setTransforms`, `inspectJson()`,
  `annotationProjectionsJson()`, `pick()`, and the draw list.
- Assert `pick(x, y)` accepts CSS pixels and internally applies device-pixel
  ratio conversion.
- Assert capture descriptor revisions and active camera match the inspection
  report for the rendered frame.

## Dependencies

This browser proof is blocked from final release approval while the current
renderer-fidelity dependencies remain open or untriaged against current `main`:

- dense WebGL2 source-material proof,
- depth/prepass robustness,
- browser external-asset and material trust,
- source-material path audit.

See `renderer-fidelity-dependencies.md` for the current evidence and required
follow-up work.

## Current status

CPU/headless validation is covered by the Rust tests and doctor gates in this
branch. The real browser/GPU proof above remains open and must be run on a
machine with hardware-accelerated browser rendering before browser-visible
SceneHost behavior is treated as release evidence.
