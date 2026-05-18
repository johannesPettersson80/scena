# Rendering

`Renderer` turns prepared scene and asset state into frames.

The core rule is:

```text
create or update scene/assets -> prepare -> render
```

## Cameras

Scenes can contain perspective and orthographic cameras. Applications select an
active camera or pass a camera explicitly when rendering.

Useful workflows:

- create a default camera,
- frame imported bounds with `Scene::frame_bounds()` and `FramingOptions`,
- focus on a selected node,
- keep camera state in the host and write it into `Scene`.

Start with `examples/camera_framing.rs`.

`frame_bounds()` projects the supplied AABB through the candidate camera and
solves distance from both viewport axes. This is the helper to use when a model
must stay centered and unclipped on both desktop and portrait/mobile canvases:

```rust
let framing = scene.frame_bounds(
    camera,
    bounds,
    scena::FramingOptions::new()
        .isometric()
        .fill(0.72)
        .margin_px(48.0)
        .viewport(width, height),
)?;
let controls = scena::OrbitControls::from_framing(framing);
```

## Lights

`scena` supports directional, point, and spot lighting concepts for common
model-viewer and visualization scenes.

Typical setup:

- one key directional light,
- optional fill or point lights,
- a neutral environment,
- explicit shadow selection when needed.

For a product/model-viewer default, call `Scene::add_studio_lighting()`. It
adds a balanced three-directional rig with one shadowed key light and softer
fill/rim lights. It is a convenient default, not a replacement for an authored
scene-specific light rig.

Start with `examples/industrial_static_scene.rs`.

## Materials

Material workflows include:

- unlit materials,
- metallic-roughness materials,
- vertex colors,
- texture slots,
- alpha modes,
- emissive output,
- ACES/sRGB output.

Create materials through `Assets` and attach them to scene renderables.

## Environment

Environment data affects model-viewer lighting and product presentation.
Applications can use bundled defaults for simple scenes or load an explicit
environment for controlled output.

Renderer-managed auto exposure is available through
`Renderer::set_auto_exposure(AutoExposureConfig::default())`. Auto exposure
adapts output brightness after a frame is rendered; lighting and materials
still control shape, contrast, and dynamic range.

Use `Scene::add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))`
when a model needs a matte reference floor. The floor helper derives size from
bounds, keeps grid lines on the floor plane, and avoids reflective defaults.

## Shadows

Shadow behavior is capability-aware. Applications should query capabilities and
diagnostics when selecting optional shadow-heavy scenes or quality settings.

## Output

Rendering outputs depend on the backend:

- native windows draw to a surface,
- browser paths draw to a canvas,
- headless paths can produce deterministic frame buffers,
- readback paths can write images for CI and docs.

GPU backends share the same wgpu/naga renderer path. Browser WebGL2 keeps a
small material texture binding shim for wgpu 29's GL backend, but it does not
use a separate raw WebGL renderer.

For generated images, see [Headless rendering](headless-rendering.md).

## Lifecycle

`prepare()` validates and uploads current scene state. `render()` draws
prepared state.

If you mutate scene graph, assets, surface, target, environment, debug overlay,
or relevant renderer settings, call `prepare()` again.

See [Lifecycle](lifecycle.md).
