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

Scene recipes expose the same camera helpers. Prefer named lenses and framing
presets instead of hand-tuning a `look_at` distance:

```json
"cameras": [{
  "id": "camera",
  "kind": "perspective",
  "lens": "portrait",
  "framing": { "preset": "three_quarter_front_right", "fill": 0.72, "margin_px": 24 },
  "active": true
}]
```

`lens` routes to `PerspectiveCamera::wide_angle()`, `standard()`,
`portrait()`, or `telephoto()`. `framing` routes to `FramingOptions` and
`Scene::frame_bounds`; `framing.mode:"default_for_bounds"` routes to
`Scene::add_perspective_camera_default_for`. For thin imported CAD parts,
`framing.mode:"principal_face"` frames the largest face instead of letting the
camera land edge-on.

Imported CAD can also request presentation-only material and edge emphasis
directly on the import. This does not mutate the glTF or CAD truth; it only
controls how Scena renders the imported mesh:

```json
"imports": [{
  "id": "terminal",
  "uri": "terminal-block.scene.geometry.gltf",
  "material": {
    "base_color": "#565A60",
    "roughness": 0.86,
    "metallic": 0.0
  },
  "edge_emphasis": {
    "enabled": true,
    "base_color": "#FFB000",
    "stroke_width_px": 2.0,
    "edge_angle_threshold_degrees": 18.0
  }
}]
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

## Authored geometry

Scene recipes can author deterministic primitives and custom meshes for
functional, CAD, dashboard, diagram, chart, and test scenes. Use imported glTF
or GLB assets when the goal is a realistic product or digital twin.

For visible primitive boxes and cylinders in product-style renders, add a small
`bevel` or `fillet` value to catch light on the edge. These fields generate real
flat chamfer geometry; unsupported primitive kinds reject them instead of
silently ignoring an inert knob. The build manifest reports the generated
vertex/index counts so agents can verify the requested geometry was built.

When a recipe uses intentionally repeated high/low geometry variants, attach a
node `lods[]` chain to switch distant or small-on-screen subjects to the cheaper
geometry by projected size. LOD levels reference existing geometry resources and
use `max_screen_fraction` thresholds in `(0, 1]`; scena does not automatically
simplify meshes.

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
For recipe-authored product scenes, prefer `material.preset` before raw PBR
fields:

```json
"materials": [
  { "id": "body", "preset": "chrome", "roughness": 0.06 },
  { "id": "trim", "preset": "plastic", "base_color": "orange" }
]
```

The recipe builder routes these names through the Rust `MaterialDesc` helpers:
`chrome`, `metal`, `rough_metal`, `brushed_steel`, `plastic`,
`clearcoat_plastic`, `satin`, `leather`, `rubber`, `matte`, `clear_glass`, and
`frosted_glass`. `base_color` is optional for presets and acts as a tint where
the helper accepts one; scalar overrides such as `roughness`, `metallic`, and
advanced PBR factors are applied after the preset.
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
Scalar physical-glass controls are supported on the GPU path: positive
`transmission_factor` materials render through the scene-color transmission pass,
and `ior`, `thickness_factor`, `attenuation_distance`, and
`attenuation_color` affect refraction and volume tint when an opaque scene color
exists behind the glass. `transmission_texture` and `thickness_texture` are not
bound by the GPU/WebGL2 material texture layout yet; scene recipes reject those
slots, and the Rust GPU prepare path fails closed instead of silently dropping
them. Use scalar volume fields for portable recipe-authored glass until the
texture-binding budget is expanded or packed.

## Environment

Environment data affects model-viewer lighting and product presentation.
Applications can use bundled defaults for simple scenes or load an explicit
environment for controlled output.

Renderer-managed auto exposure is available through named scenarios such as
`AutoExposureConfig::product_studio()`, `AutoExposureConfig::indoor()`,
`AutoExposureConfig::outdoor()`, and `AutoExposureConfig::mixed()`. Auto
exposure adapts output brightness after a frame is rendered; lighting and
materials still control shape, contrast, and dynamic range.

Recipes can use the same exposure scenarios with `render.auto_exposure`:

```json
"render": { "auto_exposure": "product_studio" }
```

`render.auto_exposure` and fixed `render.exposure_ev` are mutually exclusive in
`scena.scene_recipe.v1`; use auto exposure for product/model scenes and fixed
EV only when a deterministic exposure is part of the specification.

Recipe `scene.preset` values `product_studio`, `cad_studio`, and
`industrial_studio` route through the shared Rust scene-setup preset helper, so
they apply the matching background, bundled environment, grid/floor defaults,
SSAO, and auto-exposure scenario instead of duplicating setup logic in the
recipe layer. `scene.environment:{ "preset":"studio" }` and
`"neutral_studio"` route through `Assets::load_environment_preset` after
`RecipeBuildPolicy` checks the preset asset path. Use
`scene.environment:{ "preset":"studio" }` for low-roughness chrome product
stills; mirror metal mostly reflects its environment, so the real studio HDR
gives structured softbox reflections — pair it with a high-tessellation sphere
(segments>=256, rings>=192) so the mirror does not reveal facets.

Use `Scene::add_grid_floor(&assets, GridFloorOptions::new().under_bounds(bounds))`
when a model needs a matte reference floor. The floor helper derives size from
bounds, renders grid strokes slightly above the slab to avoid coplanar depth
artifacts, and avoids reflective defaults. Use
`GridFloorOptions::line_width_px` or recipe `scene.grid.line_width_px` when the
grid is meant to stay visible in a high-resolution hero render; start around
4.0 px for product-style floor grids and inspect the native-resolution crop.
Recipe `scene.grid.under_bounds` defaults to `true` and explicitly maps to
`GridFloorOptions::under_bounds(bounds)`; set it to `false` only when
`floor_y`/padding must be authored manually.
Recipe `scene.grid.reflection` opts into a deterministic floor-reflection decal
preset for product-style shots. It is verified by `expect_quality.reflection`
and works without requiring material SSR.
Recipe `render.screen_space_reflections` enables opt-in screen-space
reflections. It mirrors the already-rendered upper scene into the floor band and
also lets high-metallic, low-roughness materials such as `MaterialDesc::chrome`
sample visible scene colour in screen space. Material reflections are
roughness-aware and fade back to the environment-lit material at screen edges or
where no screen-space sample exists. Use `expect_quality.reflection` for
floor/reflection-surface checks, or `expect_quality.reflection.target` for a
specific chrome/mirror subject. Chrome-specific gates can add
`min_bright_fraction` and `min_dark_fraction`; failures emit
`reflection_chrome_read_missing` when a mirror subject is flat gray or black
instead of showing white-card highlights and dark edge definition.

## Shadows

Shadow behavior is capability-aware. Applications should query capabilities and
diagnostics when selecting optional shadow-heavy scenes or quality settings.
Directional shadows are supported on GPU-device WebGPU/WebGL2/native lanes
where the renderer renders a shadow map and samples it into visible receiver
pixels. CPU/reference and unattached factory capability rows report `degraded`
instead of claiming the GPU receiver path.

Area lights are evaluated as finite sampled emitters on both CPU and GPU, with
LTC-style specular evaluation for rectangular, disc, and sphere emitter shapes.
The prepare path computes deterministic per-vertex area-light visibility, so a
partially occluded softbox can produce a partial penumbra signal instead of
fully unshadowed radiance. Dedicated area-light shadow maps and clustered/tiled
light assignment are still future rendering work; recipes should not treat area
lights as a path-traced photographic soft-shadow system yet.

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
`OUTPUT_UNIFORM_BYTE_LEN: u64 = 3056`. That buffer contains view, projection,
view-projection, light-space projection, camera/exposure, viewport/depth,
color-management, punctual/area/environment/shadow lighting, and sixteen
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

Depth of field is opt-in for product and documentation hero shots:

```rust
renderer.set_depth_of_field(Some(scena::DepthOfFieldConfig::new(
    2.4, // focus distance from the active camera, in scene units
    2.8, // aperture f-stop; lower values blur more
    6,   // maximum blur radius in output pixels
)));
```

Recipe authors can use `render.depth_of_field` with `focus_distance`,
`aperture_f_stop`, and `radius_px`. The CPU path uses the CPU depth frame, and
HeadlessGpu uses the depth-color post target, both reported through
`PostProcessingReportV1.dof_depth_source`. When DoF is load-bearing, add
`expect_quality.depth_of_field`; recipe verification renders a same-backend
no-DoF baseline at native resolution and checks that the declared background
loses Sobel detail while the focal subject remains sharp.

Medium quality uses FXAA by default. High quality uses sample-based edge AA
(`Msaa4` on the GPU path and a matching CPU supersample resolve) so geometry
silhouettes are actually smoothed instead of only post-filtered. Disable AA
only for visual proof or when a host wants exact unfiltered pixels:

```rust
renderer.set_anti_aliasing(scena::AntiAliasing::None);
renderer.set_anti_aliasing(scena::AntiAliasing::Fxaa);
renderer.set_anti_aliasing(scena::AntiAliasing::Msaa4);
```

For offline or hero captures, use `Renderer::set_supersample_factor(2)`,
`3`, `4`, or a guarded `8`, or
recipe `render.supersample` to render the frame at N× resolution and
downsample. This improves curved silhouettes, thin grid/wire strokes, textures,
and glossy highlights that MSAA alone cannot fully stabilize. Cost grows with
N^2, so keep it opt-in.

Recipes can also opt into `render.reconstruction:"tent"` or `"gaussian"` to
downsample hero supersample captures with a wider positive kernel. The default
`"box"` filter remains the stable choice. Use `"tent"` for grid/wire/line-heavy
captures where stroke contrast matters; `"gaussian"` is a softer silhouette
reconstruction for inspected hero stills. `Renderer::set_supersample_factor(8)`
and `render.supersample:8` are allowed
only for small captures that keep the scaled internal target within renderer
limits.

Headless and descriptor-backed CPU renders can also enable the depth-aware
screen-space ambient occlusion baseline:

```rust
renderer.set_screen_space_ambient_occlusion(Some(
    scena::ScreenSpaceAmbientOcclusionConfig::subtle(),
));
```

The SSAO pass uses the active backend's depth information to darken contact
edges before bloom and FXAA. CPU/headless uses the CPU depth buffer; GPU,
WebGPU, and WebGL2 use the renderer-owned depth-color target reported as
`ssao_depth_source: "depth_color_target"` in post-processing capability reports.

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
