# Lifecycle

`scena` uses an explicit lifecycle so applications know when fallible work can
happen.

```text
load/create assets -> build/mutate scene -> prepare -> render
```

## Prepare

`prepare()` and `prepare_with_assets()` synchronize renderer state with the
current scene and assets.

Preparation can:

- validate scene state,
- resolve camera and target data,
- upload renderer resources,
- update material and texture bindings,
- update environment and lighting state,
- update batching,
- refresh capability-dependent renderer paths.

Because preparation is explicit, the host can handle errors before drawing a
frame.

Routine native preparation polls the device nonblocking for retired-resource
bookkeeping. It blocks only for an API that requires completion, explicit
readback/shutdown, or bounded resource-pressure recovery; the pressure path is
reported separately in `PrepareWorkMetrics`. This avoids serializing the CPU
and GPU on every retained-scene update while keeping pending destruction
bounded.

## Render

`render()` and `render_active()` draw prepared state.

Rendering expects the renderer to be prepared for the current scene, assets,
target, environment, and settings. If the prepared state is stale, `scena`
returns a structured `RenderError`.

Animation mixers share immutable clips through `Arc<AnimationClip>`. Ticking a
warm mixer borrows keyframe channels and does not clone the clip's keyframe
vectors; clip replacement and import rebinding publish a new shared clip.

## When to prepare again

Call `prepare()` again after:

- adding, removing, or moving scene nodes,
- changing cameras or active camera,
- changing lights,
- changing materials, textures, or environments,
- loading or reloading assets,
- changing render target size,
- receiving surface resize or context-loss events,
- changing relevant renderer settings.

## What a prepare costs

The first prepare of a scene bakes it: geometry is resolved into triangles and,
on a GPU backend, uploaded. A later prepare keeps that bake whenever the change
is one the renderer can re-derive from it. It then only refreshes per-draw
placement and tint, visibility, culling, and the back-to-front order of
alpha-blended and transmissive triangles for the active camera.
`PrepareWorkMetrics::prepared_triangle_count` is `0` for such a prepare, and on
a GPU backend no buffer, pipeline, or bind group is created.

| Change | GPU backends | CPU renderer |
|---|---|---|
| Camera moved, re-framed, or another camera made active | keeps the bake | keeps the bake for unlit, untextured materials; lit or textured materials are shaded and tessellated per vertex for the camera and are baked again |
| Opaque node tint | keeps the bake | keeps the bake |
| Node hidden, or shown again | keeps the bake | keeps the bake for unlit materials; lit materials carry baked shadows and are baked again |
| Mesh, light, instance-set node, or an ancestor moved | keeps the bake | bakes again |
| Instance moved, tinted, hidden, or shown | keeps the bake for sets drawn instanced | bakes again |

A prepare always bakes again when the scene structure, the environment, the
render target, or an output setting changed; when a reflection probe was
added, removed, or given another environment; when a node is shown that was
hidden at the last bake; when a transform flips a node between mirrored and
unmirrored or collapses it to zero scale; and when the scene holds an instance
set that is baked per instance instead of drawn instanced (the CPU renderer,
native surfaces, sets with a mirrored instance or a line material, and scenes
with a section box or reflection probes).

Some content is laid out for one view or one placement, so "keeps the bake"
does not apply to a scene that holds it:

- a view change bakes again in a scene with labels, particles, area lights,
  baked ambient occlusion, more punctual lights than the fixed GPU light
  uniform holds, or a mesh that would now select another LOD level;
- moved content bakes again in a scene with a section box, skinned meshes,
  reflection probes, labels, particles, area lights, baked ambient occlusion,
  more punctual lights than the fixed GPU light uniform holds, or materials
  with a physical tile size;
- hiding or showing a node bakes again in a scene with particles, area lights,
  baked ambient occlusion, or materials with a physical tile size, and hiding
  a label always does;
- four or more meshes that share geometry and material are drawn as one
  batch on GPU backends. Hiding the first of them, or so many that fewer than
  four stay visible, bakes again.

The frame after a kept bake is the frame a full prepare produces from the same
scene and camera. It is byte-identical when no draw moved. A draw that moved
keeps the vertices recovered from its first placement, so a shaded pixel can
differ in its last bit from a renderer that baked the draw where it is now.

## Context loss versus device loss

A recoverable surface or browser context loss does not mean the underlying
`wgpu::Device` is dead. With retained CPU-side assets, call
`recover_context()`, then prepare again. Surface replacement uses
`recover_surface()`; for an attached native window that method requests a fresh
adapter Device/Queue before publishing the replacement GPU state.

`SurfaceEvent::DeviceLost` is terminal for the current Device/Queue, regardless
of the host's `recoverable` flag. That flag means the application may rebuild,
not that wgpu permits reuse. `recover_context()` and every later `prepare*()`
return `PrepareError::GpuDeviceRebuildRequired`; `render*()` continues to return
`RenderError::GpuDeviceLost`. Recreate the `Renderer` (or replace an attached
native surface through the fresh-device boundary), then prepare the existing
retained `Scene` and `Assets`. Scena checks the latched device-loss state before
device polling, allocation, upload, or submission.

## GPU resource-retirement evidence

Adapter-optional C09 tests are developer smoke checks. When no adapter can be
created they write a typed `skipped` artifact under
`target/gate-artifacts/c09-gpu-resource-lifecycle/`; that outcome is diagnostic
and is never release evidence.

The required physical-hardware proof is separate and fail closed:

```bash
SCENA_REQUIRE_GPU_RESOURCE_LIFECYCLE=1 cargo test \
  --test c09_gpu_resource_lifecycle \
  required_hardware_gpu_resource_lifecycle_executes_complete_cycle \
  -- --exact --nocapture
```

It prepares a baseline resource set, prepares and renders the larger
MSAA/post-processing set, returns to the baseline retained shape, and polls the
device until every queued destruction is confirmed. The resulting
`scena.q04.required_gpu_resource_lifecycle.v1` artifact records the adapter,
allocation shapes, destruction counters, assertion count, command, commit, and
timestamp. Missing adapters, software adapters, unexecuted assertions, or a
nonzero pending count fail the required lane.

For the final physical Windows proof, build the clean-commit bundle with
`scripts/build_windows_complete_hardware_bundle.sh` and use its `run-proof.ps1`.
The one-shot runner executes this strict lifecycle test alongside attached
PresentOnly/MSAA, resize/loss, WebGPU pixel parity, and shader-cache timing,
then uploads one independently validated archive. Do not substitute optional
adapter smoke files or an uncommitted executable.

## Attached surface acquisition

Native and browser surface churn is handled at the acquisition boundary.
`Outdated` causes one configuration refresh and one acquisition retry; the
refresh re-queries size, format, present mode, alpha mode, and supported usage.
A second `Outdated` result is returned as a structured error rather than
entering a retry loop. wgpu defines `Lost` differently: the surface must be
recreated, so scena latches `SurfaceLost` immediately and the host calls
`recover_surface`/reattaches its canvas instead of pretending `configure()`
revived the old surface. If format or present mode changed, rendering returns
`SurfaceConfigurationChanged`; call `prepare()` so device-bound pipelines match
the refreshed surface before rendering again.

`Timeout` and `Occluded` are diagnostic frame skips: `RenderOutcome::skipped` is
true, no command buffer is submitted, and `RendererStats` increments the
specific timeout or occlusion counter. Acquisition validation and device
out-of-memory signals are hard `GpuValidation`/`GpuOutOfMemory` errors. They are
never folded into a successful black, stale, or unpresented frame.

Native MSAA keeps attachment sample counts explicit: the surface scene pass
uses multisampled scene depth, then resolved stroke/label overlays use their
single-sample overlay depth. An uncaptured native wgpu validation message is
written to stderr before the structured fault is latched, so automatically
uploaded proof logs retain the driver-level cause.

Attached managed auto exposure is deliberately outside synchronous capture.
Each eligible native surface frame can add one fixed 16x16 copy into a two-slot
meter; mapping and polling are nonblocking, and a completed sample changes the
exposure of a later frame. This makes first-frame latency explicit through
`AutoExposureStatus::Pending` and steady state explicit through
`AutoExposureStatus::Converged`, while preserving one scene-color pass per
presented frame. On-change render loops still poll a pending meter before their
skip decision, so convergence cannot stall merely because scene state stopped
changing. Deterministic headless rendering instead meters and applies within
the same render call, which preserves capture/reference sequence semantics.

## Why this design matters

The explicit lifecycle keeps frame rendering predictable:

- asset fetching happens before render,
- parsing happens before render,
- expensive upload work happens before render,
- stale state is reported as a structured error,
- applications decide how to recover.

## SceneHost

`SceneHost` follows the same lifecycle through a browser/WASM facade:

```text
create or attach canvas -> load/instantiate assets -> update scene -> prepare -> render
```

It does not create a second browser-only lifecycle; every host operation maps
back to asset loading, scene mutation, prepare, render, readback, or inspection.
The host page owns scheduling. A typical frame is:

```text
setTransforms or camera input -> prepare -> render -> inspectJson or readPixels when proof is needed
```

Asset fetches happen in `instantiateUrl` and `instantiateUrlUnder`, not inside
`render`. GLB bytes are parsed in `instantiateGlb` and `instantiateGlbUnder`.
Resize and DPR changes are forwarded before the next `prepare`.
Removing a node or import is a structural scene mutation. The host invalidates
the removed `u64` handles immediately, so callers must resolve new handles via
import paths, tags, picking, or inspection after rebuilding that subtree.

When proof needs pixels and metadata in one artifact, call `capture()` after
`render()` for native/CPU or synchronous WebGL2 capture. In a WebGPU browser,
call `await captureAsync()` because GPU-buffer mapping is asynchronous. The
renderer records the scene revision counters and camera that
produced the current RGBA8 frame, and `capture()` writes those rendered values
with viewport/DPR, backend capabilities, and pixel statistics into
`scena.capture.v1`. If the scene or active camera changes after render and
before capture, capture fails closed with `CaptureError::StaleRender` instead
of binding new metadata to old pixels.
The descriptor's `frame` block also binds target/output configuration
revisions and the readback completion timestamp. Renderer-owned capture paths
compare supplied bytes exactly with the retained completed readback, so an old
same-size frame cannot be certified with newer state. Caller-supplied
diagnostic pixels use `capture_unverified_rgba8_from_pixels` and are always
marked `release_evidence: false`.
Use `CaptureRgba8::to_png_bytes`, `Renderer::capture_png_bytes`,
SceneHost `capture_png_bytes`, browser `capturePng()` for synchronous WebGL2,
or `await capturePngAsync()` for WebGPU when the proof artifact should be PNG
bytes; these helpers all delegate to the same descriptor-bound capture object.

## Minimal pattern

```rust
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

If the scene changes:

```rust
scene.set_transform(node, new_transform)?;
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```
