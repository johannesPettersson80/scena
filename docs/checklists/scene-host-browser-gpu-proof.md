# SceneHost browser/GPU proof plan

Status: CI/release-enforced on the Linux WebGL2 browser lane, with prior
Raspberry Pi V3D WebGL2 evidence retained for hardware-specific review.
Renderer fidelity dependencies remain open.
Date: 2026-06-02
Scope: real browser/GPU rendered-output proof for the generic `SceneHost`
contracts.

Real browser/GPU machine required. The Hetzner CPU builder is valid for Rust
compile, test, doc, doctor, WASM compile, and headless capture proof. It is not
valid evidence for WebGPU/WebGL2 rendered output.
Run this plan explicitly on V3D hardware when changing SceneHost browser
contracts that require hardware-specific proof; GitHub CI also runs the
same harness on the Linux WebGL2 browser lane.

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

Hardware-specific browser proof is blocked from final release approval while the current
renderer-fidelity dependencies remain open or untriaged against current `main`:

- dense WebGL2 source-material proof,
- depth/prepass robustness,
- browser external-asset and material trust,
- source-material path audit.

See `renderer-fidelity-dependencies.md` for the current evidence and required
follow-up work.

## Run evidence

Run on Raspberry Pi 5 hardware using system Chromium and ANGLE/GLES:

```bash
SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof
```

The harness builds the SceneHost package with the configured Rust toolchain:

```bash
rustup run "$RUST_TOOLCHAIN" wasm-pack build . --dev --target web --out-dir target/scene-host-browser-pkg --out-name scena --features scene-host
```

Artifacts:

- `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`
- `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.png`

Recorded renderer:

```text
ANGLE (Broadcom, V3D 7.1.10.2, OpenGL ES 3.1 Mesa 25.0.7-2+rpt4)
```

The hardware guard asserts the renderer contains `V3D` and does not contain
`SwiftShader` or `llvmpipe`. The run recorded `hardware_tier: "low"` and
`forward_pbr: "supported"`; the harness also accepts low-tier
`forward_pbr: "degraded"` as expected-on-tier for this proof. This gate proves
SceneHost browser construction, render, inspection, annotation projection,
CSS-pixel picking, and capture contracts. It does not close the dense PBR or
source-material fidelity epics.

## Current status

CPU/headless validation is covered by the Rust tests and doctor gates in this
branch. The SceneHost browser/GPU proof above has V3D hardware evidence and is
also run by the WebGL2 CI/release lane. Final hardware-specific browser visual
approval still waits on the renderer-fidelity dependencies listed above.
