# M6 browser renderer parity acceptance

Status: active evidence index

The browser package and renderer probe use these commands:

```text
wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe
node tests/browser/m6_rust_wasm_renderer_probe.js
```

The required browser test targets are explicit in both CI and release:

```text
wasm-pack test --headless --chrome --test m1_browser_rendered_output
wasm-pack test --headless --chrome --test m3a_browser_rendered_output
wasm-pack test --headless --chrome --test m3b_browser_rendered_output
wasm-pack test --headless --chrome --test m6_browser_renderer_parity --features browser-probe
```

The M6 headline WebGL2 result includes
`scena.m6.cpu_webgl2_parity.v1`. It renders one identical triangle fixture,
camera, renderer options, background, and dimensions through CPU headless and
an attached WebGL2 canvas. The artifact retains both
`renderer-owned-cpu-frame` and `renderer-owned-gpu-copy` RGBA8 inputs. Frames
are normalized to top-left rows, sRGB8 transfer, straight opaque alpha, and
exact dimensions before bounded RMSE, SSIM, p95/mean channel delta,
foreground-IoU, and foreground-region RMSE evaluation. The same evaluator must
reject the recorded center-channel GPU mutation.

Doctor registers the lane as `VISUAL-BROWSER-M6`. The proof covers
`dirty-transform`, `resource-lifetime`, and `idle-render-skipped` behavior with
renderer-owned readback.

M2 fixture metadata is anchored by
`m2_headless_visual_artifacts_cover_lighting_depth_and_clipping`, the
`m2-headless-core.toml` companion, and `VISUAL-M2-FIXTURE-METADATA`.
Required WebGPU/WebGL2 lanes fail closed when no renderer output is available.
The WebGL2 release headline also fails closed when its parity object, either
frame input, normalization, passing metrics, mutation rejection, or
renderer-owned GPU checksum link is absent.

Required WebGPU uses `SCENA_REQUIRE_PARITY=1`. Its headline triangle must report
an actual WebGPU device, positive draw and submission counts, nonblack
`renderer-owned-gpu-copy` readback, and an adapter classified as discrete,
integrated, or virtual hardware. `NoAdapter`, zero output, missing identity,
SwiftShader/llvmpipe/lavapipe, CPU adapters, and unproven `Other` adapters fail
the producer immediately. `SCENA_BROWSER_ALLOW_UNAVAILABLE` remains available
only for explicitly diagnostic local probes and is forbidden in required CI and
release jobs. The required-parity evaluator has a standalone mutation suite,
and `Q06-REQUIRED-GPU-LANES` prevents workflow or validator drift.

The Linux native lane separately proves strict `HeadlessGpu` Vulkan-path
construction and rendered output; it rejects the `Backend::Headless` CPU
fallback and `host_gpu_available=false`. A software Vulkan adapter proves that
API/backend path, not physical GPU acceleration. Hardware claims therefore
still require a real-GPU artifact, while deterministic CPU fallback remains in
the separately named `headless-cpu` lane.
