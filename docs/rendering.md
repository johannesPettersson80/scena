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
- clearcoat factor/roughness plus clearcoat, clearcoat-roughness, and
  clearcoat-normal texture sampling on the CPU/reference path and GPU
  shader/material resource path,
- sheen color/roughness factors plus sheen color and sheen roughness texture
  sampling on the CPU/reference path and GPU shader/material resource path,
- anisotropy strength/rotation factors plus anisotropy direction/strength
  texture sampling on the CPU/reference path and GPU shader/material resource
  path,
- iridescence factor, IOR, thickness-range factors plus iridescence
  factor/thickness texture sampling on the CPU/reference path and GPU
  shader/material resource path,
- dispersion factor parsing and channel-spread specular shading on the
  CPU/reference path and GPU shader/material resource path,
- vertex colors,
- texture slots,
- alpha modes,
- emissive output,
- ACES/sRGB output.

Create materials through `Assets` and attach them to scene renderables.
Optional glTF `KHR_materials_clearcoat` scalar factors and texture slots are
parsed into `MaterialDesc`. The CPU/reference path samples the clearcoat
factor texture's red channel, clearcoat roughness texture's green channel, and
clearcoat normal texture for the clearcoat specular lobe. The WebGPU/WebGL2
shader variants now carry the same factor and texture roles through material
uniforms, bind groups, and punctual-light shading. Approved backend screenshot
or readback proof remains capability-gated release evidence.
Optional glTF `KHR_materials_sheen` color and roughness factors and texture
slots are also parsed into `MaterialDesc`. The CPU/reference path samples the
sheen color texture's RGB channels and the sheen roughness texture's alpha
channel, and the WebGPU/WebGL2 shader variants carry those roles through the
same material uniform and bind-group path.
Optional glTF `KHR_materials_anisotropy` strength, rotation, and texture slots
are parsed into `MaterialDesc`. The CPU/reference path samples the anisotropy
texture's red/green direction channels and blue strength channel, and the
WebGPU/WebGL2 shader variants carry the role through the same material uniform
and bind-group path. Approved backend screenshot or readback proof remains
capability-gated release evidence.
Optional glTF `KHR_materials_iridescence` factor, IOR, thickness range, and
texture slots are parsed into `MaterialDesc`. The CPU/reference path samples
the iridescence factor texture's red channel and thickness texture's green
channel, and the WebGPU/WebGL2 shader variants carry those roles through the
same material uniform and bind-group path. Approved backend screenshot or
readback proof remains capability-gated release evidence.
Optional glTF `KHR_materials_dispersion` factors are parsed into
`MaterialDesc`. The CPU/reference path and WebGPU/WebGL2 shader variants apply
the factor as a channel-spread specular approximation. Required dispersion
assets still report degraded status until approved backend proof and full
transmission/volume glass behavior are promoted.

## Environment

Environment data affects model-viewer lighting and product presentation.
Applications can use bundled defaults for simple scenes or load an explicit
environment for controlled output.

Renderer-managed auto exposure is available through named scenarios such as
`AutoExposureConfig::product_studio()`, `AutoExposureConfig::indoor()`,
`AutoExposureConfig::outdoor()`, and `AutoExposureConfig::mixed()`. Auto
exposure adapts output brightness after a frame is rendered; lighting and
materials still control shape, contrast, and dynamic range.

Use `Scene::add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))`
when a model needs a matte reference floor. The floor helper derives size from
bounds, keeps grid lines on the floor plane, and avoids reflective defaults.

## Shadows

Shadow behavior is capability-aware. Applications should query capabilities and
diagnostics when selecting optional shadow-heavy scenes or quality settings.
Directional shadows are supported on GPU-device WebGPU/WebGL2/native lanes
where the renderer renders a shadow map and samples it into visible receiver
pixels. CPU/reference and unattached factory capability rows report `degraded`
instead of claiming the GPU receiver path.

## Output

Rendering outputs depend on the backend:

- native windows draw to a surface,
- browser paths draw to a canvas,
- headless paths can produce deterministic frame buffers,
- readback paths can write images for CI and docs.

GPU backends share the same wgpu/naga renderer path. Browser WebGL2 keeps a
small material texture binding shim for wgpu 29's GL backend, but it does not
use a separate raw WebGL renderer.

The GPU output uniform layout is pinned by
`OUTPUT_UNIFORM_BYTE_LEN: u64 = 1200`. That buffer contains view, projection,
view-projection, light-space projection, camera/exposure, viewport/depth,
color-management, punctual/environment/shadow lighting, and sixteen
scene-clipping plane uniforms plus clipping control. Per-draw model and normal
matrices live in the draw-uniform bind group instead.

Output color is sRGB unless capability evidence says otherwise.
`Capabilities::wide_gamut_output` and the browser M4 smoke artifact record
Display P3 canvas probe results; scena does not blanket-claim wide-gamut output
on native, headless, or unmeasured browser surfaces.

Subtle postprocess bloom is opt-in:

```rust
renderer.set_bloom(Some(scena::PostBloomConfig::subtle()));
```

The bloom pass runs on the output frame before FXAA and is reported through
`RendererStats::bloom_passes`.

Medium quality uses FXAA by default. High quality uses sample-based edge AA
(`Msaa4` on the GPU path and a matching CPU supersample resolve) so geometry
silhouettes are actually smoothed instead of only post-filtered. Disable AA
only for visual proof or when a host wants exact unfiltered pixels:

```rust
renderer.set_anti_aliasing(scena::AntiAliasing::None);
renderer.set_anti_aliasing(scena::AntiAliasing::Fxaa);
renderer.set_anti_aliasing(scena::AntiAliasing::Msaa4);
```

For offline or hero captures, use `Renderer::set_supersample_factor(2..=4)` or
recipe `render.supersample` to render the frame at N× resolution and
downsample. This improves curved silhouettes, thin grid/wire strokes, textures,
and glossy highlights that MSAA alone cannot fully stabilize. Cost grows with
N^2, so keep it opt-in.

Headless and descriptor-backed CPU renders can also enable the depth-aware
screen-space ambient occlusion baseline:

```rust
renderer.set_screen_space_ambient_occlusion(Some(
    scena::ScreenSpaceAmbientOcclusionConfig::subtle(),
));
```

The SSAO pass uses the CPU depth buffer to darken contact edges before bloom
and FXAA. GPU/WebGPU/WebGL2 SSAO remains a separate capability-gated lane.

Headless and descriptor-backed CPU renders can enable weighted blended
order-independent transparency for overlapping alpha-blended surfaces:

```rust
renderer.set_order_independent_transparency(Some(
    scena::OrderIndependentTransparencyConfig::weighted_blended(),
));
```

This path resolves transparent overlap from a per-pixel accumulator, then
composites the result over opaque pixels. It is reported through
`RendererStats::order_independent_transparency_passes`. GPU/WebGPU/WebGL2
OIT remains a separate capability-gated lane.

For generated images, see [Headless rendering](headless-rendering.md).

## Lifecycle

`prepare()` validates and uploads current scene state. `render()` draws
prepared state.

If you mutate scene graph, assets, surface, target, environment, or relevant
renderer settings, call `prepare()` again.

See [Lifecycle](lifecycle.md).
