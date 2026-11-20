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

### Layered surface model

The CPU renderer and the GPU shaders evaluate one layered surface. The shader
side is `src/render/gpu/surface_lighting.wgsl`; the CPU side is
`src/render/prepare/pbr_contract/surface.rs`, which keeps the shader's names so
the two can be read side by side.

- The base layer is Lambert diffuse plus GGX specular. Its dielectric
  reflectance follows the material's index of refraction; the default index of
  1.5 is the usual 0.04.
- Anisotropy reshapes the base specular lobe along the authored tangent
  direction and bends environment reflections toward the grooves. It does not
  add a second highlight, so a brushed surface peaks lower than a plain one.
- Iridescence replaces the base Fresnel term with thin-film interference, for
  direct light and for the reflectance that image-based lighting integrates.
- Sheen is a Charlie lobe on top of the base. The base keeps
  `1 - 0.157 * max(sheen colour)` of its energy, so a sheen layer dims a
  surface seen head-on and brightens its grazing rim.
- Clearcoat is a GGX lobe on top of everything. The layers beneath it, emitted
  light included, keep `1 - clearcoat * F` of their light, where `F` is the
  coat's Fresnel reflectance at the view angle. A coat is not added on top of
  an unchanged base.

Both renderers apply these rules to direct lights and to image-based lighting.
A material that uses no layer and keeps the default index of refraction renders
on the CPU exactly as it did before the layers existed;
`tests/cpu_core_material_bytes.rs` pins those bytes against the last commit
without the layered model. One of its scenes, a glossy cube and chrome sphere,
is re-pinned since glossy triangles are subdivided on screen; the test records
both frames.

`tests/layered_material_parity.rs` renders a sphere per layer on both
renderers under one directional light and the default environment and
compares it with the plain sphere it is built on. Mean absolute difference
between CPU and GPU per channel, worst channel, in 8-bit code values on Mesa
lavapipe:

| Sphere | Layered | Plain base |
|---|---:|---:|
| Clearcoat | 2.3 | 2.9 |
| Sheen | 2.3 | 2.9 |
| Anisotropy | 2.9 | 2.9 |
| Iridescence | 2.5 | 2.9 |
| Clearcoat over sheen | 2.1 | 2.9 |
| Anisotropy on metal | 16.5 | 16.7 |
| Iridescence on metal | 16.4 | 16.7 |

The CPU renderer still differs from the shaders in these places:

- Base-layer image-based lighting. The CPU renderer weights environment light
  with the analytic split-sum fit and single scattering; the shaders read the
  baked split-sum table and add the multiple-scattering term. The difference
  grows with the reflectance at normal incidence: under the default
  environment it is under 3 code values for a dielectric, about 7 for a
  dielectric with an index of refraction of 2.0, and about 16 for a metal of
  roughness 0.38, which is darker on the CPU. Layers inherit it from their
  base. Changing it moves every image-based-lit CPU frame, layered or not.
- Dispersion and the transmission-volume response keep their own CPU
  approximations and are added on top of the layered surface.
- The CPU renderer shades vertices and interpolates, so a highlight narrower
  than the mesh tessellation is resolved by the shaders only.

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
Physical glass is drawn by the CPU renderer and by the native and WebGL2 GPU
shaders with one formulation. A material with a positive `transmission_factor`
is drawn after the opaque scene, in a transmission pass that reads the colour
of the opaque scene behind it:

- **Fresnel.** The share of light the surface reflects follows Schlick's
  approximation with `F0 = ((ior - 1) / (ior + 1))^2`, at the angle between
  the view direction and the normal. The surface's own light, its highlights
  and its reflection of the environment, is added in full. The light from
  behind is weighted by `1 - F`.
- **Refraction.** The view direction is refracted at the surface by the index
  of refraction and travels `thickness_factor` to an exit point. The scene
  colour is read where that point lands on screen, as in the Khronos glTF
  Sample Viewer. Thickness is in model units and follows the scale of the
  model matrix. A thickness of zero is a thin wall that reads the scene
  straight through.
- **Roughness.** The scene colour is blurred by a disc whose radius grows with
  roughness and with the index of refraction: half of
  `width^(roughness * clamp(2 * ior - 2, 0, 1))` pixels, the footprint the
  Sample Viewer reaches through a mip level. The disc has 16 Gaussian-weighted
  taps on a golden-angle spiral, rotated per pixel, and its radius is capped
  at 3 % of the viewport height. Beyond that the taps would be too sparse, so
  very rough glass is blurred less than the formula asks for.
- **Volume.** The volume absorbs along the length of the refracted path by
  Beer-Lambert with `attenuation_color` and `attenuation_distance`, and the
  base colour tints what passes.
- **Share.** `transmission_factor * (1 - metallic)` of the light entering the
  surface is transmitted. The diffuse lobe keeps the rest, so a fully
  transmissive dielectric has no diffuse colour of its own.

This is a screen-space model with known limits. The ray is refracted once, on
entry, so a solid sphere bends the scene like the front half of a lens. The
scene colour holds opaque surfaces only: glass shows neither other glass nor
blended surfaces behind it. A ray that leaves the frame reads the nearest edge
pixel. Glass with nothing behind it transmits the clear colour, so in front of
black it shows only its reflections. Light focused through glass is not
simulated. `Renderer::path_trace` has none of these limits.

The renderers differ in where they evaluate the inputs.
`tests/transmission_parity.rs` holds the CPU and GPU renders of four glass
panes, over the pane interior, to a root-mean-square difference of at most
0.075 and a mean channel difference of at most 16 of 255.

- The CPU renderer bakes surface light at mesh vertices, so a highlight on
  glass is only as fine as the mesh. Its pixel grid sits half a pixel from the
  GPU's, which moves fine refracted detail by up to one pixel.
- The CPU renderer tints transmitted light by the base colour factor. The GPU
  shaders tint by the full base colour, including its texture and vertex
  colour.
- The CPU renderer reads `transmission_texture` and `thickness_texture`,
  averaged per triangle. The GPU/WebGL2 material texture layout does not bind
  them yet; scene recipes reject those slots, and the Rust GPU prepare path
  fails closed instead of silently dropping them. Use scalar volume fields for
  portable recipe-authored glass until the texture-binding budget is expanded
  or packed.
- On GPU-instanced meshes the thickness follows the scale of the draw's model
  matrix but not a per-instance scale.
- Double-sided glass is shaded on its visible side only in every renderer.

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
pixels.

Native targets fit four cascades to the camera frustum each frame and pack them
into one atlas, so a small subject on a large floor still gets most of the
shadow map. Each cascade is a crop of a light frustum fitted to the occluders
that can shadow something; a ground plane with nothing behind it does not
stretch the fit. Receivers are filtered with percentage-closer soft shadows: a
twelve-tap blocker search sizes a rotated sixteen-tap kernel per fragment, so a
shadow is crisp where the caster touches the receiver and softens with
distance. Bias is a normal offset plus a slope term, both measured in shadow
texels.

WebGL2 renders one shadow map and filters it with an explicit 3×3 grid of nine
depth-comparison samples averaged once per shadowed fragment. Both paths use a
linear comparison sampler, so every tap is itself a 2×2 hardware comparison.

`directional_shadow_pcf_kernel: 3` in capabilities and frame stats names the
smallest comparison kernel any GPU backend runs, not the implicit 2×2 footprint
of one linearly filtered comparison sample. CPU/reference and unattached
factory capability rows report `degraded` instead of claiming the GPU receiver
path. Glass-like and blended casters are still opaque to the shadow map;
alpha-masked casters shadow only their visible texels. Point/spot shadow maps
are not currently shipped.

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

Bloom takes the light of pixels brighter than a threshold, blurs it and adds
it back onto the frame, in scene-linear light before FXAA and the display
transform. `PostBloomConfig::new(threshold_srgb, intensity, radius_px)` sets
the luminance threshold as an sRGB byte, the strength of the added light and
how far it spreads, up to 12 pixels. A frame with no pixel above the threshold
comes out unchanged. The stage is reported through
`RendererStats::bloom_passes`.

GPU backends (native, WebGPU and WebGL2) blur with a Gaussian in two
one-dimensional passes, so a small emitter gains a round halo that falls off
steadily with distance. The Gaussian has the variance of a box filter reaching
`radius_px` each way and is cut off at three standard deviations, at most 22
pixels. The threshold has a soft knee: a pixel's contribution rises smoothly
from nothing at the threshold to its whole colour at `threshold * (1 + knee)`
in linear light. The knee defaults to 0.5; `PostBloomConfig::with_knee` sets
it, and 0 is a hard cut. Cost grows with the radius: `subtle()` reads 13 texels
a pixel in each pass, the largest radius 45.

The CPU renderer keeps a box blur of the same radius with a hard threshold,
which the CPU reference images pin. Its halo is square at the corners and
starts abruptly at the threshold; the two agree on how far a highlight spreads
and on its total energy above the knee.

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

On GPU backends (native, WebGPU and WebGL2) FXAA is an edge search in the
manner of FXAA 3.11. A pixel is touched only where its neighbourhood shows a
visible luma step. The filter then walks along the edge in both directions, up
to 26 pixels, to find where the current run of the staircase ends, and blends
the pixel with its neighbour across the edge by its distance to the nearer
end. A second, weaker term softens single-pixel contrast. Flat regions come
out exactly as rendered, and a one-pixel line stays one pixel wide, keeping
about three fifths of its light, instead of spreading into the pixels beside
it.

The pass runs on scene-linear radiance before the display transform. It
therefore measures contrast on exposed luminance, clamped to display white and
taken to gamma 2.0, and it blends two values as the display will show them, so
a highlight brighter than white cannot swamp its neighbour.

Shallow edges, where a staircase is most visible, gain the most: the measured
deviation of a 7 degree edge from a straight line falls by about three
quarters. An edge near 45 degrees has runs of one or two pixels, and FXAA
softens it only slightly. Use `Msaa4` or supersampling when those edges
matter.

The CPU renderer keeps its own FXAA: a 3x3 average on the encoded frame,
applied where a pixel is the darkest or brightest of a contrasting
neighbourhood. The CPU reference images pin it. `AntiAliasing::Fxaa` therefore
produces different edge pixels on the CPU and GPU paths; render with
`AntiAliasing::None` when the two are compared pixel for pixel.

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

Screen-space ambient occlusion is opt-in on every backend:

```rust
renderer.set_screen_space_ambient_occlusion(Some(
    scena::ScreenSpaceAmbientOcclusionConfig::contact(),
));
```

On native GPU targets occlusion is computed before the colour pass. A compute
pass rebuilds view-space geometry from the depth prepass, estimates how much of
the hemisphere above each pixel is blocked within `radius_px` output pixels,
adds a term three times wider for the soft falloff, and blurs the result along
each surface. The colour pass then removes that share of environment light and
most of the light from sources that cast no shadow. Light from the
shadow-casting directional light is left alone, so lit surfaces keep their
colour while a subject gains a contact shadow where it meets its support.
Unlit materials are darkened directly, and blended or transmissive surfaces
are skipped. Capability reports name the source as
`ssao_depth_source: "sampleable_depth_texture"`. The radius scales with the
supersample factor, so a supersampled frame keeps the same apparent reach.

The CPU renderer and browser targets instead apply a depth-difference filter
to the finished frame before bloom and FXAA. CPU/headless uses the CPU depth
buffer (`"cpu_depth_frame"`); WebGPU and WebGL2 use the renderer-owned
depth-color target (`"depth_color_target"`). That filter darkens everything
under it, so `ScreenSpaceAmbientOcclusionConfig::subtle()` is the better fit
there; `contact()` is tuned for the native pass. A frame rendered with
occlusion therefore differs between the CPU reference and native GPU.

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

## Path-traced stills

`Renderer::path_trace` renders a still by simulating light transport instead
of rasterizing. It follows many random light paths through every pixel and
averages them, so soft shadows, bounce light, reflections and refraction come
out of one simulation rather than from separate approximations. It runs on
the CPU, needs no GPU, and takes seconds to minutes per image; it is a mode
for finished stills, not for interaction.

```rust
use scena::PathTraceOptions;

let options = PathTraceOptions::new().with_samples_per_pixel(128);
let image = renderer.path_trace(&scene, &assets, camera, &options)?;
image.write_png("still.png")?;
```

The call reads the scene as it stands: nothing is prepared first and the
renderer is not changed. The image has the renderer's target size and uses
its environment, environment intensity and rotation, background, exposure,
white balance, tone mapping and depth-of-field settings, so one scene can be
rasterized and path traced with the same framing and exposure.
`examples/path_traced_still.rs` lights a subject with three softboxes on a
seamless backdrop and writes the result.

What is simulated:

- **Lights.** Directional, point, spot and area lights in the units the
  rasterizer uses, so brightness matches: a tessellated floor under each
  light type measures within 8 % of the CPU rasterizer. Area lights (rect,
  disc and sphere) are real emitters: their shadows soften with distance and
  they appear in reflections, but the camera does not see them directly. A
  directional light has an angular radius
  (`with_directional_light_angular_radius`, default 0.5 degrees) that
  softens its shadow edges; one set not to cast shadows lights every surface
  that faces it, as in the rasterizer.
- **Environment.** An HDR environment is sampled into a cube of
  `environment_resolution` texels per edge (default 1024, where the
  rasterizer uses 256) and sampled in proportion to its brightness, so small
  bright sources in the image cast clean shadows. Rays that leave the scene
  after a bounce see the environment; the camera sees the renderer's
  background colour.
- **Materials.** Metallic-roughness materials with base-colour,
  metallic-roughness, normal and emissive textures and texture transforms;
  clearcoat; sheen; alpha cut-outs and blending; vertex colours and node
  tints. Emissive surfaces light the scene and are sampled as light sources.
  Unlit materials show their colour whatever the lighting.
- **Glass.** A material with transmission refracts and reflects by its index
  of refraction. With a thickness it is a solid whose attenuation colour
  absorbs along the path inside; without one it is a thin sheet. Roughness
  frosts it.
- **Camera.** Perspective and orthographic cameras with the scene camera's
  near and far planes. With depth of field configured, rays leave a lens of
  the configured focal length and f-stop, which gives true defocus blur.

- **Surface finish and rounded edges.** See "Surface detail" below.

The layers conserve energy: no combination of base, sheen and clearcoat
reflects more light than it receives at any view angle. This makes sheen
slightly brighter face-on and darker at grazing angles than the rasterizer's
constant sheen weight.

### Surface detail

A material's roughness says how sharp its reflections are on average. What
makes a rendered part read as a manufactured object lies between that average
and the shape of the part: the grain of a casting, the orange peel of paint,
the marks a lathe leaves, an edge that is slightly rounded and catches the
light, paint worn off corners, dirt in recesses. The path tracer adds these
from two settings.

**`SurfaceFinish`** is a material attribute that states how the part was
made:

```rust
use scena::{Color, MaterialDesc, PhotographicSurfaceKind, SurfaceFinish};

let cast_iron = MaterialDesc::pbr_metallic_roughness(Color::from_srgb_u8(70, 72, 76), 1.0, 0.6)
    .with_surface_finish(
        SurfaceFinish::new(PhotographicSurfaceKind::CastMetal)
            .with_wear(0.2)
            .with_grime(0.4),
    );
```

The kinds are those of `PhotographicSurfaceKind`: polished, satin, brushed,
machined and cast metal, painted and powder-coated metal, molded and
clearcoat plastic, rubber and fabric. Each brings its own relief, patches of
duller and glossier surface, slight tone variation and edge radius. The
settings are:

| Setting | Effect |
|---|---|
| `with_scale(f)` | Multiplies every feature size. `1.0` is the real size in a scene measured in metres. |
| `with_relief(f)` | Multiplies the depth of the bumps; `0.0` keeps gloss and tone variation only. |
| `with_wear(0..=1)` | Wears the finish off outward edges: coatings wear through to bare metal, bare metal is burnished, plastic and rubber scuff paler. |
| `with_grime(0..=1)` | Darkens and dulls places that nearby surfaces shelter. |
| `with_edge_radius_m(r)` | Rounds the part's edges to this radius; `0.0` keeps them sharp. Without it the kind's typical radius is used (0.2 mm machined, 1.5 mm cast), times the scale. |
| `with_axis(v)` | Direction of brushing, or the axis a machined part was turned about, in the mesh's own space. Turning leaves rings on a face and hoops on a cylinder, about the mesh's origin. |
| `with_seed(n)` | Another instance of the same random pattern. |

A finish is generated in three dimensions from the position on the surface,
in the mesh's own frame. It needs no texture coordinates, has no seams and
never tiles, and it stays put when the mesh is moved, rotated or scaled.
Relief finer than a pixel of the image is not drawn; it widens the highlight
instead, which is what a camera sees. Brushing and turning marks finer than a
pixel still stretch the highlight across the marks only.

A glTF material states its finish in `extras.scena.surface_finish`, as a kind
name or an object with the settings above; see [Assets](assets.md).

**Rounded edges** are a shading effect. At each point it shades, the tracer
looks at the surface of the same mesh within the edge radius and uses the
average normal it finds. On a flat or smoothly curved surface that changes
nothing. Next to an edge the normal leans towards the neighbouring face, so
the edge catches the thin highlight a real one does, and an inside corner
shades as a fillet. Overlapping solids inside one mesh blend where they meet,
which makes a body built from primitives read as one casting. Different
meshes keep a crisp line between them, as separate parts do. The silhouette
keeps its corners, and light that reaches the rounded strip from behind the
face it lies on is not gathered, so a rounded edge is slightly darker than a
modelled bevel when lit only from the far side.

`PathTraceOptions::with_edge_radius_m` rounds the edges of materials that have
no finish, and `with_surface_detail(false)` traces the bare mesh with plain
materials. `PathTraceStats` reports `finished_surfaces` and
`rounded_surfaces`.

Surface detail is applied where the image shows a surface, directly or in a
mirror or through glass, not to the light a surface bounces onto others. The
rasterizer does not draw a `SurfaceFinish`; under `scena photo render` it
keeps the generated texture maps it had.

`demo/samples/gate-valve/gate_valve_dn100.glb` is a flanged gate valve whose
materials all state their finish; `scripts/generate_gate_valve_demo_asset.js`
builds it from its dimensions.

Limits to know about:

- Anisotropy and iridescence are not simulated; such materials render as the
  plain base material.
- Light focused through glass (caustics) is not simulated. Light passes
  through glass in a straight line when shadows are computed, tinted by the
  glass and dimmed by its reflection, so glass objects cast soft tinted
  shadows.
- Lines, wireframes, edges, labels and particles are overlays rather than lit
  surfaces and are left out. `PathTracedImage::stats()` reports how many
  nodes were skipped.
- Model nodes are not traced; instantiate the imported scene instead.
- Every triangle is stored in world space, so instanced geometry counts once
  per instance, up to 16 million triangles.
- The CPU rasterizer places pixel centres on whole coordinates, half a pixel
  from where the path tracer and the GPU place them, so silhouettes can
  differ from a CPU-rasterized frame by one pixel.

Quality is set by `PathTraceOptions`:

| Setting | Default | Effect |
|---|---|---|
| `samples_per_pixel` | 64 | Noise falls with the square root of the sample count; time grows in proportion. `preview()` uses 16, `production()` 256. |
| `max_bounces` | 8 | How often a path may change direction. More bounces brighten interiors and stacked glass. |
| `denoise` | on | An edge-avoiding wavelet filter guided by surface colour, normal and position. It smooths only as far as each pixel's measured noise warrants, so converged detail stays. |
| `radiance_clamp` | 32 | Brightest scattered light one path may contribute, relative to the exposure. It removes isolated bright pixels at the cost of a little energy in rare light paths. |
| `seed` | 0 | The same seed, scene and settings give the same image whatever the thread count. |

`PathTracedImage::rgba8()` is display-ready sRGB.
`PathTracedImage::linear_radiance()` is the scene radiance before tone
mapping, weighted by coverage, for compositing or a different grade.
`Renderer::path_trace_with_progress` reports finished rows for a progress
display.

The `scena` CLI delivers a photo this way with
`scena photo render <asset> --out <png> --report <json> --path-trace`.
`--samples <n>`, `--bounces <n>`, `--no-denoise` and `--no-surface-detail`
tune it. The camera,
exposure, focus and lighting are still chosen by the rasterized
camera-behavior loop; the delivered frame is then traced from that scene and
the quality policy is evaluated on the traced pixels. The report gains a
`path_trace` block with the settings and scene statistics.

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
