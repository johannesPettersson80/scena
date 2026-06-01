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

## Render

`render()` and `render_active()` draw prepared state.

Rendering expects the renderer to be prepared for the current scene, assets,
target, environment, and settings. If the prepared state is stale, `scena`
returns a structured `RenderError`.

## When to prepare again

Call `prepare()` again after:

- adding, removing, or moving scene nodes,
- changing cameras or active camera,
- changing lights,
- changing materials, textures, or environments,
- loading or reloading assets,
- changing render target size,
- receiving surface resize or context-loss events,
- changing debug overlays or relevant renderer settings.

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
setTransforms -> prepare -> render -> inspectJson or readPixels when proof is needed
```

Asset fetches happen in `instantiateUrl` and `instantiateUrlUnder`, not inside
`render`. GLB bytes are parsed in `instantiateGlb` and `instantiateGlbUnder`.
Resize and DPR changes are forwarded before the next `prepare`.
Removing a node or import is a structural scene mutation. The host invalidates
the removed `u64` handles immediately, so callers must resolve new handles via
import paths, tags, picking, or inspection after rebuilding that subtree.

When proof needs pixels and metadata in one artifact, call `capture()` after
`render()`. The renderer records the scene revision counters and camera that
produced the current RGBA8 frame, and `capture()` writes those rendered values
with viewport/DPR, backend capabilities, and pixel statistics into
`scena.capture.v1`. If the scene or active camera changes after render and
before capture, capture fails closed with `CaptureError::StaleRender` instead
of binding new metadata to old pixels.

## Minimal pattern

```rust
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```

If the scene changes:

```rust
scene.node_mut(node).set_transform(new_transform);
renderer.prepare_with_assets(&mut scene, &assets)?;
renderer.render_active(&scene)?;
```
