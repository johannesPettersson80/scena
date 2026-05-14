# Troubleshooting

This page lists common problems and the first places to look.

## The output is blank

Check:

- The scene has an active camera.
- The camera is looking at the model.
- The model was instantiated into the scene.
- The model bounds are inside the camera frustum.
- `prepare()` was called after the latest scene or asset change.
- The renderer target size is non-zero.

Useful examples:

- `examples/first_visible_render.rs`
- `examples/camera_framing.rs`
- `examples/headless_ci.rs`

## The model is too large or too small

Check unit metadata and import options.

See [Units, axes, and handedness](guides/units-axes-handedness.md).

## The model is rotated sideways

Check the authored up-axis and coordinate-system metadata.

See:

- [Units, axes, and handedness](guides/units-axes-handedness.md)
- [Troubleshooting misplaced assets](guides/troubleshooting-misplaced-assets.md)

## Textures are missing

Check:

- external image paths are correct,
- browser URLs are fetchable,
- image files are deployed next to the glTF file,
- optional texture features are enabled when required,
- unsupported required extensions are reported in the asset error.

See [Assets](assets.md).

## Rendering fails after a resize

Resize and surface events invalidate prepared renderer state. Forward the event
to the renderer and call `prepare()` again before rendering.

See [Lifecycle](lifecycle.md).

## Browser rendering is unavailable

Check:

- browser support for WebGPU or WebGL2,
- secure context requirements for WebGPU,
- canvas creation,
- requested backend,
- capability report,
- console errors from asset fetching.

See [Browser and WASM](browser.md).

## Picking misses objects

Check:

- camera and viewport dimensions,
- cursor coordinate conversion,
- object visibility,
- layer masks,
- scene preparation after moving objects.

Start with `examples/picking_selection_hover.rs`.

## Anchors or connectors do not align

Check:

- connector forward/up vectors,
- source units,
- coordinate-system conversion,
- left-handed versus right-handed data,
- whether the anchor belongs to the expected imported node.

See:

- [Place and connect objects](guides/place-and-connect-objects.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)
- [Troubleshooting misplaced assets](guides/troubleshooting-misplaced-assets.md)
