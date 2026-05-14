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
