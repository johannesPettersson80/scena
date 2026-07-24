# Rendering

`Renderer` turns prepared scene and asset state into frames.

The core rule is:

```text
create or update scene/assets -> prepare -> render
```

## Semantic AOVs

Scena can capture node/instance ID, linear camera-depth, and world-normal AOVs
from prepared state. CPU headless capture is deterministic and allocation-free
after prepare. GPU capture is opt-in so ordinary rendering pays no attachment,
pipeline, or readback cost: set the option before prepare, then use the
SceneHost GPU capture API. Native/headless GPU, WebGPU, and WebGL2 share the
same 24-bit ID/depth packing and exclusion semantics; WebGL2 uses a portable
RGBA8 MRT plus byte-preserving canvas readback. See
[`semantic-aov-v1.md`](specs/semantic-aov-v1.md) for identity, transparency,
sampling, persistence, and proof rules.

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

For aggregate scene or import framing, use the matching option-bearing helper
so the output dimensions, view, padding, depth policy, and inspection-helper
policy are part of the solve:

```rust
let framing = scene.frame_all_with_assets_and_options(
    camera,
    &assets,
    scena::FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .margin_px(32.0)
        .tighten_depth_range(true)
        .include_helpers(false)
        .viewport(output_width, output_height),
)?;
```

Visible hidden nodes are always excluded; tagged inspection helpers are also
excluded by default. Use `include_helpers(true)` only when helper geometry is
part of the intended composition. The legacy `frame_all` and `frame_import`
methods use the camera's current aspect because they do not receive a target
size. Viewers and captures that know the real output size use
`frame_all_with_assets_and_options` or `frame_import_with_options` instead.
Interactive viewers seed `OrbitControls` from that same `FramingOutcome`, so
the first pointer event preserves the selected view instead of snapping to
front.

`Scene::move_origin_to` aligns a node origin. It does not center its geometry.
Use `Scene::center_visible_bounds_on(node, &assets, point)` when imported or
authored content is offset from its node origin. The former ambiguous
`Scene::center_on` name is deprecated.

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

High-level glTF viewer builders promise a presentable default: they preserve
authored lights/environments and otherwise add one neutral directional light
plus a studio background. `FirstRender::diagnostics()` and viewer
`diagnostics()` include a structured warning describing the applied fallback.
The raw `Renderer` API intentionally does not do this; black or transparent
targets and unlit scenes are valid low-level rendering contracts. A low-level
render may therefore return bytes successfully while scene diagnosis reports
`MissingLightingOrEnvironment` or `InvisibleScene`.

## Authored geometry

Scene recipes can author deterministic primitives and custom meshes for
functional, CAD, dashboard, diagram, chart, and test scenes. Use imported glTF
or GLB assets when the goal is a realistic product or digital twin.

For visible primitive boxes and cylinders in product-style renders, add a small
`bevel` or `fillet` value to catch light on the edge. These fields generate real
flat chamfer geometry; unsupported primitive kinds reject them instead of
silently ignoring an inert knob. The build manifest reports the generated
vertex/index counts so agents can verify the requested geometry was built.

Generated cylinder and cone sides duplicate the closing vertex at `u=1`.
Their final side triangles therefore interpolate across the local final UV
interval instead of wrapping backward across the whole texture. Cylinder caps
retain their independent radial UV layout, while cone tips remain face-local.

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

### KHR material visual-proof contract

The deterministic CPU/reference proof for clearcoat, sheen, anisotropy,
iridescence, dispersion, and transmission/volume uses fixed, feature-specific
regions rather than whole-frame maxima. Visible acceptance requires at least a
four-code-value channel change plus per-feature RMSE, changed-pixel, and signed
effect-direction floors. Numerical repeatability is evaluated separately: a
valid feature image with one-LSB noise remains accepted, while a disabled
control, a two-LSB effect nudge, and an inverted-effect direction all fail the
same evaluator. Directional anisotropy is rendered under two light directions.

Run the focused proof with:

```text
cargo test --test m8_visual_proof m8_khr_material_visual_oracle_rejects_disabled_and_wrong_direction_mutations -- --exact
```

It writes the metric and mutation rows to
`target/gate-artifacts/m8-visual/khr-material-feature-proof.json`. This CPU
oracle does not replace the source-bound Round E WebGPU/WebGL2 release lanes;
it prevents extension implementation smoke tests from passing on imperceptible
or directionally wrong output.

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

Attached interactive surfaces meter one frame behind: native rendering copies a
fixed 16x16 sample grid into one of two asynchronous buffers, never performs an
automatic full-frame blocking readback, and never renders the same surface
frame twice. The first frame uses the configured exposure and reports
`AutoExposureStatus::Pending`; a later render call polls without blocking and
reports `Converged` after applying a completed sample. Browser surfaces apply
their completed canvas observation to the next frame. Headless rendering keeps
same-call convergence so repeated capture/reference jobs remain byte
deterministic. For an interactive screenshot whose exposure must already be
settled, wait for `renderer.auto_exposure_status() ==
AutoExposureStatus::Converged` or use a fixed `exposure_ev`. A native surface
without the required copy/format capability reports `Unavailable` instead of
remaining pending forever.

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
pixels. The shipped receiver filter is an explicit 3×3 texel grid: nine
nearest-filtered depth-comparison samples averaged once per shadowed fragment.
`directional_shadow_pcf_kernel: 3` in capabilities and frame stats names that
sample grid, not the implicit 2×2 footprint of one linearly filtered comparison
sample. CPU/reference and unattached factory capability rows report `degraded`
instead of claiming the GPU receiver path. Point/spot shadow maps and cascaded
directional maps are not currently shipped.

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

CPU depth-slab clipping happens before perspective division and row-band
binning. The retained projection cache stores the clipped triangles once per
geometry pass, so parallel bands do not repeat clipping or projection. Pixel
attributes use camera-appropriate interpolation, while post-projection depth
remains screen-linear for the depth buffer. GPU backends continue to use
hardware clip-space clipping for the same near/far contract.

Output color is sRGB unless capability evidence says otherwise.
`Capabilities::wide_gamut_output` and the browser M4 smoke artifact record
Display P3 canvas probe results; scena does not blanket-claim wide-gamut output
on native, headless, or unmeasured browser surfaces.

`Capabilities::color_target_format` reports the selected attachment format.
For `*Srgb` attachments, shaders keep RGB linear and the attachment performs
the transfer. For plain `*Unorm` attachments that carry the sRGB output
contract, the final shader performs the transfer. RGBA8 capture/readback is
therefore sRGB display bytes in either case and does not change interpretation
when post-processing is enabled. See
[`specs/color-contract.md`](specs/color-contract.md).

Subtle postprocess bloom is opt-in:

```rust
renderer.set_bloom(Some(scena::PostBloomConfig::subtle()));
```

The bloom pass samples linear RGB from sRGB post intermediates before FXAA and
is reported through `RendererStats::bloom_passes`.

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

On the current WebGPU/WebGL2 WASM pipelines, automatic or profile-selected
high quality degrades to FXAA and records `MultisampleFallback` with
`fallback_applied:true`. Capability JSON reports color/depth sample matrices of
`[1, 0, 0]`. Calling `set_anti_aliasing(Msaa4)` is an exact request and remains
an actionable `UnsupportedSampleCount { requested: 4, maximum: 1 }` error.

CPU rendering enables a conservative occlusion prepass only when at least 64
prepared primitives have overlapping projected tiles. GPU backends never run
that CPU prepass. A host with its own workload measurements can opt out without
changing pixels:

```rust
renderer.set_cpu_occlusion_culling(false);
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

### Prepare and render performance contracts

GPU triangle shader modules are cached per live device and material texture
binding mode, shared by compatible pipelines, and discarded with that device.
Structural work remains in `prepare()`; `render()` does not compile shaders.
`PrepareWorkMetrics` exposes module creations, cache hits/misses, and routine
nonblocking versus pressure-triggered blocking polls.

On native attached surfaces, `PresentOnly` without post-processing encodes one
scene-color pass and submits once. Requested supported MSAA renders into a
multisampled surface-sized target and resolves into the presentation texture.
Readback and post-processing retain their explicit offscreen paths.

The CPU raster path retains projected row-bin candidates, computes frame-wide
primitive flags once, uses inverse-area multiplication, and performs final
linear-to-display conversion once per finite-depth output pixel where blending
semantics permit. Transparency and transmission keep their required linear or
already-encoded intermediate semantics. The u8-to-linear path uses the shared
bit-identical lookup table. `RenderWorkMetrics` exposes scene passes,
submissions, bounded auto-exposure meter submissions/sample counts, output
encodes, row-bin work, and primitive-flag scans so these contracts can be tested
without timing-only assertions.

Imported animation and skin preparation share one source-node index. Joint
position/normal matrices are computed once per joint update rather than per
vertex influence, including inverse-transpose normal handling for nonuniform
scale.

See [Lifecycle](lifecycle.md).
