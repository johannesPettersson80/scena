# Headless rendering

Headless rendering is useful when you need image output without a visible
window.

Use it for:

- CI snapshots,
- generated documentation images,
- regression tests,
- server-side previews,
- deterministic visual checks.

## Example

Run the bundled example:

```bash
cargo run --example headless_ci
```

The example shows the complete lifecycle: create scene, prepare renderer, render
the frame, and write output.

## Basic pattern

```rust
let mut renderer = scena::Renderer::headless(1280, 720)?;
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

Headless GPU `render()` retains synchronous readback, so `frame_rgba8()` is
current when it returns. Advanced native loops can choose explicitly:

```rust,no_run
use scena::RenderReadbackMode;

renderer.render_with_readback_mode(&scene, camera, RenderReadbackMode::PresentOnly)?;
renderer.render_with_readback_mode(&scene, camera, RenderReadbackMode::Synchronous)?;
# Ok::<(), scena::Error>(())
```

`PresentOnly` performs no texture-to-buffer copy, map request, or blocking wait.
`Synchronous` updates `frame_rgba8()` before returning. Output settings that
change GPU resource shape require another `prepare()` before either mode.
Native multi-frame capture can use
`render_batch_with_async_readback(&scene, &cameras)`: it alternates two
prepare-owned buffers, overlaps map submission with the next render, and
returns frames in the exact order of the camera slice.

The exact readback or file-writing helper depends on the output workflow. Start
with `examples/headless_ci.rs` for a runnable reference.

## Semantic ID, depth, and normal output

For machine vision, dataset generation, attributed review, or downstream scene
diffs, use the CPU semantic AOV command:

```bash
scena recipe aov scene.recipe.json --out-dir target/semantic-aov \
  --passes id,depth,normal
```

It writes a paletted ID PNG, linear-depth 16-bit PNG, world-normal PNG, and
`scena.semantic_aov_result.v1` legend/report. Callers must persist authored
recipe/import IDs from the legend, never runtime host handles. CPU v1 excludes
transparent geometry and overlays explicitly; GPU/native/WebGPU/WebGL2 AOV
readback is not claimed until the separate parity lane is complete.

## Why use headless rendering

Headless output is deterministic and easy to automate. It is well suited for:

- checking that an imported asset is visible,
- verifying camera framing,
- detecting blank renders,
- comparing generated image artifacts,
- producing images for documentation.

## CPU and GPU paths

`Renderer::headless` is the deterministic headless path. GPU-capable headless
paths are used when adapter availability and platform support allow it.

GPU-sounding constructors are strict:

```rust,no_run
use scena::SceneHostCore;

let host = SceneHostCore::headless_gpu(640, 480)?;
assert_eq!(host.backend(), scena::Backend::HeadlessGpu);
# Ok::<(), scena::SceneHostError>(())
```

If CPU fallback is acceptable, opt into it by name and retain its report:

```rust,no_run
use scena::SceneHostCore;

let (host, selection) = SceneHostCore::headless_prefer_gpu(640, 480)?;
if selection.fallback_used() {
    eprintln!("GPU unavailable: {:?}", selection.gpu_error());
}
assert_eq!(selection.selected_backend(), host.backend());
# Ok::<(), scena::SceneHostError>(())
```

The high-level viewer follows the same naming contract:
`with_headless_gpu()` is strict, while `with_headless_prefer_gpu()` permits a
fallback and exposes `backend_selection_report()` on the built viewer or first
render. Recipe host construction uses `build_recipe_json_gpu()` for strict GPU
work and `build_recipe_json_prefer_gpu()` for an explicitly reported fallback.

Applications should use capability reports and renderer metadata when they need
to distinguish CPU, native GPU, WebGPU, and WebGL2 output.

Proof/release jobs must use a strict constructor and assert the GPU backend. A
preferred-GPU report that selected `Backend::Headless` is an honest application
fallback, never GPU evidence.

## WaterBottle regression proof

The normal native test lane includes an always-on 256x256 WaterBottle CPU
render. It compares the live deterministic output with the committed
`reference_cpu_256.png` using RGB Chebyshev distance (not DeltaE): at least
99.5% of pixels must be within 4 channel values, full-frame RGB RMSE must be at
most 2.0, and alpha must match everywhere. Both images are opaque RGBA8, sRGB
output, with rows stored top-to-bottom.

The test also writes flattened-chrome, wrong-material, and wrong-camera
mutations and requires the same oracle to reject all three. Release staging
binds those PNGs to the exact Rust test command and observed test log, CPU
backend/adapter label, source commit, timestamp, metrics, and SHA-256 checksums.

The separate macOS GPU lane is not a golden-diff claim. It currently checks
nonblack content, material color-family histograms, and fixed color regions
with RGB Chebyshev tolerance up to 35 on the measured Apple Paravirtual Metal
body sample. `SCENA_REFERENCE_DIFF` remains an opt-in diagnostic and is not set
by required workflows.

## Common mistakes

- Rendering before calling `prepare()`.
- Forgetting to set an active camera.
- Placing the model outside the camera frustum.
- Loading an asset but not instantiating it into the scene.
- Mutating the scene after preparation without preparing again.

See [Troubleshooting](troubleshooting.md).
