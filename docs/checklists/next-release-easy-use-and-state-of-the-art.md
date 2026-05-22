# scena v1.4-v1.5 easy-use + state-of-the-art roadmap and evidence log

Created: 2026-05-18
Reconciled: 2026-05-19 — staleness pass against current main; shipped items
removed, false gaps reframed, four bets promoted, material presets trimmed
to the honest set, auto-framing-default reworded for the viewer level.
Source-truth fix pass: 2026-05-19 — false residual gaps reclassified
against `src/viewer.rs`, `src/scene/mixers.rs`, material variants,
`src/material.rs`, `src/scene/math.rs`, and current feature flags.
Dependency-boundary pass: 2026-05-19 — existing crates preferred for
format / IO / OS / encoding / validation work; renderer public behavior
and visual contracts stay owned by scena modules.
Visual-proof pass: 2026-05-19 — every item declares its visual proof
class so "code compiles, APIs work, but the render is wrong" cannot pass
as success (the v1.3.0 demo failure mode).
v1.5 cleanup: 2026-05-22 — this file is now both the historical execution
log for the v1.4/v1.5 easy-use work and the backlog for still-open proof
lanes. Historical investigation notes below are superseded where they
conflict with the shipped §2.6 material preset contract.

scena's signature is **easy to use**. This document is the gap inventory
between "Rust renderer that works" and "easier than Three.js, more
accurate than `<model-viewer>`." It started as a planning document and is
now kept as an evidence log plus open-proof backlog; items become
contracts as they're picked up, each with its own narrow implementation
checklist (the way
`easy-scene-setup-and-auto-framing.md` was structured for v1.3.0).

## Status legend

Every item carries one tag:

- **[gap]** — genuinely missing; nothing to build on.
- **[ergonomic-gap]** — implementation exists but the user surface is
  raw / opt-in / requires plumbing.
- **[proof-gap]** — implementation exists but lacks rendered-output
  proof, doctor rule, or capability evidence.
- **[deferred]** — real work, but explicitly outside the current
  next-release critical path.
- **[shipped]** — already shipped; listed with version/context where needed.

Each item also names an **owner module** (where the work lives) and a
**proof class** (what would close the item).

---

## Visual proof classes

When a roadmap item produces a frame, declare which visual evidence
closes the spec. Items can combine classes with `+`.

- **none** — text / API / serialization spec; no rendered image required.
- **docs-image** — rendered image embedded in the guide next to the
  example that produced it. **Generated from checked example code or a
  small render harness, never hand-screenshotted** — hand-maintained
  screenshots drift away from the code that produced them. The source
  example / harness, generation command, output path, dimensions, and
  proof class must be recorded so docs images can be regenerated and
  checked like other artifacts.
- **reference-image** — stored reference artifact with a documented
  tolerance: PNG / PPM, sampled-RGBA TOML in `tests/visual/references/`,
  or an asset-specific reference PNG with adjacent metadata / SHA. The
  non-negotiable regression guard when the picture is the spec. Renderer
  features that change visual quality (AA, contact shadows, bloom,
  material extensions) need ON/OFF or before/after pairs, not a single
  image.
- **browser-demo** — live page on the Cloudflare demo or equivalent
  that runs the code and shows the result. This must be an actual
  browser render using the host device GPU/backend (`Backend::WebGl2`
  or `Backend::WebGpu`) through the demo page or browser probe; it must
  not be a static screenshot pasted into the page. Used for integration,
  mobile layout, controls, and WASM/WebGPU/WebGL behavior.
- **animated-proof** — GIF / video / browser recording for motion or
  interaction: damping, hot reload, drag/drop, auto-rotate, picking,
  variant switching.

Mandatory visual proof applies when the feature's value is visual:
`<scena-viewer>`, viewer-level auto-framing, material / light /
background / environment / auto-exposure presets, §3.1 items whose
user-facing value is a changed frame, Khronos sample loader, material
variants, screenshot capture, and anything whose acceptance criterion is
"does it look correct?". Import, compression, capability, and performance
items need their own structured proof first, then rendered proof only
where the render is part of the contract. A unit test alone is not enough
for visual items — the v1.3.0 demo proved that code can compile, APIs can
work, and the rendered result can still be wrong.

For visual features that appear in public documentation, three proof
surfaces are required before the item is fully closed:

1. **reference-image** gate artifact for deterministic regression.
2. **browser-demo** proof rendered live by the host browser/GPU.
3. **public docs/demo media** embedded in the demo page, README, or guide,
   regenerated from the checked harness after the implementation changes.

---

## Dependency boundary

Use existing libraries where they replace hard, spec-heavy, or
platform-specific work. Do not add dependencies for preset tables, thin
API aliases, or renderer passes where the dependency would import another
engine's architecture.

Default split:

- **Lean on proven crates** in `assets`, `diagnostics`, `platform`, and
  `encoding`: glTF parsing, texture / mesh compression, PNG encoding,
  filesystem watching, URL/serde state, browser APIs, and official asset
  validation.
- **Keep ownership in scena** for `scene`, `viewer`, `controls`, and
  `render`: public behavior, typed handles, viewer workflow, camera
  controls, render passes, visual proof, and capability reporting.
- **Use references, not engine dependencies** for render algorithms:
  Khronos Sample Viewer / Sample Renderer for glTF material behavior,
  Filament docs/source for PBR architecture, XeGTAO for ambient occlusion,
  and published OIT / TAA / bloom references. Port the relevant technique
  into scena with focused tests and rendered-output proof.

Dependency admission rule: every new crate needs a named owner module,
feature/default policy, package-size and build-time impact when relevant,
and a proof path. Convenience-only dependencies are rejected unless they
remove meaningful maintenance risk.

---

## 1. The four bets — highest leverage

These four investments move scena from "Rust renderer that works" to
"the library people reach for." Funded first; the convenience-API
rounds in §2 are the floor underneath them, not a substitute.

### 1.1 `<scena-viewer>` custom element with `<model-viewer>` attribute parity

Status: **[shipped]** for the current drop-in renderer parity proof:
custom-element foundation, browser UI proof, and three-asset side-by-side
comparison with `<model-viewer>` are all source-enforced.
Owner: `src/viewer.rs` for shared viewer behavior + a new thin browser
adapter module / WASM package built directly on `web-sys` /
`wasm-bindgen`. Do not add a Rust web-component framework unless a
concrete missing browser API proves one is necessary. The adapter must
delegate asset loading, framing, and rendering to `viewer` / `assets` /
`scene`; it must not become a second renderer owner.
Proof: foundation is `src/viewer_element.rs` with
`defineScenaViewer()`, a shadow-canvas custom element, model-viewer-style
attribute parsing, docs, doctor rule `SCENA-VIEWER-ELEMENT`, and the M6
browser proof artifact
`target/gate-artifacts/scena-viewer-element-browser-proof.png`, plus
`target/gate-artifacts/scena-viewer-model-viewer-parity-browser-proof.png`
from `scena.scena_viewer_model_viewer_parity_proof.v1`.
Three-asset side-by-side `<model-viewer>` parity proof **[shipped]**:
the M6 Playwright lane renders `non_ndc_camera_scene.gltf`,
`AnimatedMorphCube.gltf`, and `WaterBottle.gltf` in renderer-backed
`<scena-viewer>` panes next to actual `@google/model-viewer` reference
panes for the same assets.
Visual proof: reference-image + browser-demo + animated-proof

```html
<scena-viewer
    src="machine.glb"
    environment="studio"
    tone-mapping="neutral"
    camera-controls
    auto-rotate
    ar>
</scena-viewer>
```

This is the single most important item. `<model-viewer>`'s entire value
is exactly this surface — drop a model on a page, get a viewer.
Everything else in this roadmap is a smaller adoption lever than the
custom element.

### 1.2 Auto-framing as the default at the viewer level

Status: **[shipped]** — viewer builders already frame imported assets by
default (`ViewerCommonOptions::frame_import = true`), the scene-level
`Scene::add_perspective_camera_default_for(bounds, viewport)` helper now
exists, and the `<scena-viewer>` browser proof records viewer-level
auto-framing metadata from a real dropped GLB render.
Owner: `src/viewer.rs` (`InteractiveGltfViewer`, `HeadlessGltfViewer`)
and the future `<scena-viewer>`. Not on `Camera::default()` — that
has no bounds or viewport.
Proof: viewer-level integration tests assert "load → render" produces a
centered, fill-correct frame without any `frame_bounds()` call in user
code. The focused M6 custom-element proof drops a GLB `File`, loads the
accepted bytes through `m6RenderDroppedFileProbe`, renders into the
element canvas, and records `viewer-level-auto-framing` projected-bounds
metadata: inside viewport, centered, and fill fraction `0.6991069` on the
96x64 WebGL2 proof path.
Visual proof: reference-image + browser-demo

```rust
// today
let mut scene = Scene::new();
let import = scene.instantiate(&model)?;
let bounds = import.bounds_world(&scene).ok_or(...)?;
let camera = scene.add_perspective_camera(...)?;
let framing = scene.frame_bounds(camera, bounds, FramingOptions::new()...)?;

// already available at viewer-builder level
let mut viewer = interactive_gltf_viewer("machine.glb", surface)
    .with_orbit_controls()
    .build_async()
    .await?;                         // frames imported bounds by default
viewer.render_next_frame()?;

// explicit helper (Scene level)
let camera = scene.add_perspective_camera_default_for(bounds, (w, h))?;
```

### 1.3 Production-grade asset pipeline complete and production-profile ready

Status: **[shipped]** for the production profile — `production-assets`
enables KTX2 / Basis and meshopt together while keeping `default = []`.
KTX2, meshopt, and `EXT_mesh_gpu_instancing` now have feature-gated
local visual proof artifacts. Draco remains deferred and is not a v1.4
critical-path item.
Owner: `Cargo.toml` features + `src/assets/texture.rs` +
`src/assets/gltf/extensions.rs` + a new doctor lane.

Default policy to decide before implementation: keep the crate's default
feature set lean unless package size, build time, and binary-size evidence
support changing it. If KTX2 / meshopt stay optional, ship a documented
`production-assets` profile or example command that enables them together.
Current policy: defaults stay empty, and `production-assets` is the named
profile that enables `ktx2` + `meshopt` together.

Sub-items:

- **PBR Neutral tonemapper as default** — Status: **[shipped]**
  (`src/render/output.rs:62`, `#[default]` on `Tonemapper::PbrNeutral`;
  test at `tests/m1_geometry_materials.rs:719`).
- **KTX2 / Basis textures (`KHR_texture_basisu`)** — Status:
  **[shipped]** for the optional production profile. Feature flag exists
  at `Cargo.toml:45`, documented in `docs/feature-flags.md:28`, decode
  path at `src/assets/texture.rs`, marked `Supported` in extension
  diagnostics, and grouped under `production-assets`. The feature-gated
  proof suite writes material-role visual rows under
  `target/gate-artifacts/m8-compressed-assets`. Native compressed GPU
  upload and browser-lane release proof are not claimed yet.
- **meshopt (`EXT_meshopt_compression`)** — Status: **[shipped]** for
  the optional production profile. Feature flag at `Cargo.toml`, marked
  `Supported` at `src/assets/gltf/extensions.rs`, grouped under
  `production-assets`, and proved by feature-gated rendered fixtures for
  triangle, index-sequence, normal, tangent, and quantized-position paths.
  Native GPU / browser release proof remains a separate lane.
- **Draco (`KHR_draco_mesh_compression`)** — Status: **[deferred]**.
  Not part of the v1.4/v1.5 release closure. Prefer meshopt for the next release;
  revisit Draco only behind an optional feature when a maintained decoder
  path is proven. `draco_decoder` is still 0.0.x, and `draco-oxide`
  decoder support is not ready.
- **GPU instancing (`EXT_mesh_gpu_instancing`)** — Status: **[shipped]**
  for import into scene-owned `InstanceSet` nodes. Scena uses gltf-rs
  raw extension data and a narrow parser for the extension's TRS accessors
  so v1.4 is not blocked on upstream typed support.

Proof: `tests/production_asset_profile.rs` pins the empty-default /
production-profile policy. `tests/m8_compressed_asset_release_proof.rs`
runs with `--features production-assets`, loads KTX2 material-role
textures, meshopt-compressed glTF fixtures, and an instanced glTF, renders
them through the normal CPU headless path, and writes JSON + PPM artifacts
under `target/gate-artifacts/m8-compressed-assets`. The native GPU lane is
now strict when
`SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1` is set: it must render
KTX2 and meshopt through `Renderer::headless_gpu` or fail the run. Browser
production-assets proof is runtime-backed by
`SCENA_BROWSER_COMPRESSED_ASSETS=1 npm run browser:m6`; current WebGL2 and
WebGPU artifacts render meshopt compressed output, but still record
`release_evidence: false` because browser KTX2/Basis remains fail-closed
on the sync wasm texture path. Doctor rules `PRODUCTION-ASSET-PROFILE`
and `ASSETS-M8` pin the profile and proof suite.
Visual proof: reference-image (KTX2-textured render, meshopt-compressed
render, and instanced render artifacts written by the feature-gated
compressed-asset proof suite)

### 1.4 Doctor → official validation + actionable scena guidance

Status: **[shipped]**
Owner: new `xtask` doctor-assets lane under `crates/xtask/src/app/` plus
the existing `src/assets/gltf/extensions.rs` diagnostics infrastructure.
Proof: `cargo run -p xtask -- asset-doctor <asset.gltf|asset.glb>` runs
the official Khronos glTF Validator CLI in stdout mode (`gltf_validator
-o <asset>`, or `SCENA_GLTF_VALIDATOR=<path>`), then emits
`scena.asset_doctor.v1` JSON with scena-native renderer guidance and
`fix` strings. `tests_15` covers command parsing, the official-validator
stdout contract, and required-clearcoat guidance. Doctor rule
`ASSET-VALIDATION-DOCTOR` pins the CLI, docs, tests, checklist, and
fix-string guidance. `GltfExtensionDiagnostic::suggested_fix()` also
surfaces the same remediation class through library diagnostics for importer
and asset-review UIs.
Visual proof: none (structured text errors; no rendered output)

```rust
AssetError::UnsupportedTextureFormat {
    path: "albedo.webp".into(),
    help: "Re-export with PNG/JPEG or use KTX2 through the ktx2 feature",
}
```

The official validator owns glTF spec compliance; scena owns actionable
renderer guidance such as "this asset uses required clearcoat; CPU/reference
and GPU shader paths are wired, but approved GPU/WebGPU/WebGL2 rendered-output
proof is still missing for the target lane." Do not
reimplement a private subset of the glTF Validator against the `gltf`
AST unless the official validator cannot run in CI or `xtask`.
`GltfExtensionDiagnostic` returns `extension`, `status`, `help`,
`suggested_fix`, and `decoder_policy`; the asset doctor combines those
renderer-aware policies with official validator output instead of asking
users to infer remediation from raw extension names.

---

## 2. Tier 1 — "write a name, not a number"

The signature pattern: wherever the library forces users to write raw
coordinates or magic floats, give them a named primitive. Atomic, small,
landable in one PR each.

### 2.1 `Color` named constants + `from_hex` + `from_kelvin`

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-a`: `WHITE`, `BLACK`,
`Color::from_hex_srgb("#rrggbb")`, the wider named palette,
the designer-friendly `from_hex` alias, and the Kelvin helper all exist.
Owner: `src/material.rs` (or a deliberate future split from it).
Proof: `tests/round_a_easy_use.rs` covers every constant plus hex/Kelvin
behavior; rustdoc examples cover `from_hex` and `from_kelvin`; doctor rule
`ROUND-A-EASY-USE-PRIMITIVES` bans raw RGB/RGBA constructors in first-path
examples where a constant would do.
Visual proof: docs-image
`target/gate-artifacts/examples-visual/round-a-named-color-swatch-docs-image.ppm`
generated from the constants.
Dependency note: keep `palette` for color-space conversion plumbing, but
do not rely on it for Kelvin-to-RGB. Implement `from_kelvin` locally as a
small tested 2700-6500K approximation in `src/material.rs`; avoid making
the heavier optional `lcms2` path a default dependency for this helper.

```rust
Color::CHARCOAL                  // = Color::from_hex("#1a1d28")
Color::from_hex("#1a1d28")       // alias for existing from_hex_srgb
Color::from_kelvin(3200.0)       // for light color temperature
```

Constants: `TRANSPARENT`, `WHITE`, `BLACK`, `GRAY`, `LIGHT_GRAY`, `DARK_GRAY`,
`CHARCOAL`, `STUDIO_BACKDROP`, `WARM_WHITE`, `COOL_WHITE`,
`RED`, `GREEN`, `BLUE`, `ORANGE`, `YELLOW`, `CYAN`, `MAGENTA`.

### 2.2 `PerspectiveCamera` lens presets + `with_fov_degrees`

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-a`.
Owner: `src/scene/camera.rs`
Proof: `tests/round_a_easy_use.rs` asserts each preset's FOV in degrees;
rustdoc examples cover each preset plus `with_fov_degrees`; doctor rule
`ROUND-A-EASY-USE-PRIMITIVES` keeps first-path camera examples on named
presets.
Visual proof: docs-image
`target/gate-artifacts/examples-visual/round-a-lens-preset-comparison-docs-image.ppm`
renders one subject with each lens preset side-by-side.

```rust
PerspectiveCamera::wide_angle()    // ~24mm equivalent, ~84° FOV
PerspectiveCamera::standard()      // ~50mm equivalent, ~46° FOV (default)
PerspectiveCamera::portrait()      // ~85mm equivalent, ~28° FOV
PerspectiveCamera::telephoto()     // ~135mm equivalent, ~18° FOV
PerspectiveCamera::standard().with_fov_degrees(60.0)  // escape hatch
```

### 2.3 Drop the `with_aspect` boilerplate

Status: **[shipped]** — first-path docs, examples, and demo camera setup
now use named lens presets; `frame_bounds` continues to write aspect from
`FramingOptions::viewport`.
Owner: `src/scene/framing.rs` (document side effect) + `examples/` (sweep)
Proof: first-path examples and docs drop avoidable `with_aspect(...)`
calls; tests and advanced examples may keep explicit aspect setup where
that is the behavior under test. Doctor rule
`ROUND-A-EASY-USE-PRIMITIVES` rejects `PerspectiveCamera::default().with_aspect(`
in first-path files.
Visual proof: none (no visual change; behavior-preserving cleanup)

### 2.4 `Transform` rotations in degrees + `looking_at`

Status: **[shipped]** — degree rotations already exist as
`rotate_x_deg`, `rotate_y_deg`, and `rotate_z_deg`; Round A deliberately
keeps those names and adds `looking_at` rather than adding alias churn.
Owner: `src/scene/math.rs`
Proof: `tests/round_a_easy_use.rs` asserts `looking_at` against known
forward vectors; rustdoc example covers the public call.
Visual proof: docs-image (optional — a small "node rotated 45° around Y" render is a useful tutorial illustration but the spec is the math)

```rust
Transform::at(Vec3::new(1.0, 0.0, 0.0)).rotate_y_deg(45.0)
Transform::default().rotate_x_deg(-90.0)            // glTF Y-up → CAD Z-up
Transform::looking_at(target_position, Vec3::Y)     // node faces a point
```

Implementation decision: keep the existing `rotate_*_deg` names and only
add `looking_at`; do not add beginner aliases unless a later docs/usability
pass proves the current names are a real obstacle.

### 2.5 Light presets

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`.
Owner: `src/scene/lights.rs`
Proof: `tests/round_b_light_presets.rs` asserts preset colors,
intensities, shadow ownership, and point-light ranges; rustdoc examples
cover each preset; doctor rule `NAMED-LIGHT-PRESETS` keeps the API,
tests, and visual proof present.
Visual proof: reference-image + docs-image
`target/gate-artifacts/examples-visual/round-b-light-preset-reference-docs-image.ppm`
renders one subject under each preset side-by-side.

```rust
DirectionalLight::sun()
DirectionalLight::key_light()
DirectionalLight::fill_light()
DirectionalLight::rim_light()
PointLight::softbox()
PointLight::bulb_warm()      // 2700K
PointLight::bulb_cool()      // 5600K
```

### 2.6 `MaterialDesc` PBR presets — honest expanded set

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`; 2026-05-21 follow-up expands the set
behind WebGL2 IBL-quality and material-extension proof.
Owner: `src/material/presets.rs` extending `MaterialDesc`
Proof: `tests/round_b_material_presets.rs` asserts each preset's
PBR material kind, base color, metallic factor, roughness factor, and
extension-specific lanes; rustdoc examples cover the preset
constructors; doctor rule
`HONEST-MATERIAL-PRESETS` keeps preset names, tests, checklist text, and
visual proof aligned.
Visual proof: reference-image + docs-image
`target/gate-artifacts/examples-visual/round-b-material-preset-reference-docs-image.ppm`
renders the presets side-by-side on the same subject; M6 browser proof
adds a WebGL2/WebGPU material-preset workflow so the browser path cannot
fall behind the docs image.

```rust
MaterialDesc::matte(Color)
MaterialDesc::plastic(Color)
MaterialDesc::metal(Color)
MaterialDesc::rough_metal(Color)
MaterialDesc::chrome()
MaterialDesc::brushed_steel()
MaterialDesc::clearcoat_plastic(Color)
MaterialDesc::satin(Color)
MaterialDesc::leather(Color)
MaterialDesc::clear_glass(Color)
MaterialDesc::frosted_glass(Color)
MaterialDesc::rubber()
```

Preset contract:

- `chrome()` and `brushed_steel()` are backed by metallic roughness,
  anisotropy where appropriate, and the raised WebGL2 environment
  prefilter sample floor. They do not claim SSR floor reflections.
- `clear_glass(Color)` and `frosted_glass(Color)` are transparent
  transmission/IOR/volume presets with blend-mode browser previews. They
  do not claim full refractive caustics or physical WebGPU/WebGL2 glass
  parity.
- `leather(Color)` is a smooth leather-like sheen preset, not a
  procedural grain or normal-map material.

### 2.7 `Background` named scheme

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`.
Owner: `src/render/background.rs` (new) + `Renderer::set_background`
Proof: `tests/round_b_background_presets.rs` asserts every named
scheme maps to the intended clear color and that
`Renderer::set_background(...)` drives the existing renderer clear path;
doctor rule `NAMED-BACKGROUND-PRESETS` keeps the API, tests, and visual
proof present.
Visual proof: reference-image + docs-image
`target/gate-artifacts/examples-visual/round-b-background-preset-reference-docs-image.ppm`
renders the same subject over every named background.

```rust
renderer.set_background(Background::DarkStudio);
// variants: Studio, DarkStudio, NeutralGray, White, Black, Sky, Transparent, Custom(Color)
```

### 2.8 `OrbitControls` named damping presets

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`.
Owner: `src/controls.rs`
Proof: `tests/round_b_orbit_controls_presets.rs` asserts named damping
values, `presentation()`'s slow turntable behavior, `turntable(rpm)`'s
explicit speed, and `advance(delta_seconds)` frame-advance semantics;
doctor rule `NAMED-ORBIT-CONTROL-PRESETS` keeps the API, tests, visual
proof, and demo sweep present.
Visual proof: animated-proof + docs-image
`target/gate-artifacts/examples-visual/round-b-orbit-control-preset-animated-docs-image.ppm`
plus its generated frame sequence shows `presentation()` and
`turntable(6.0)` changing camera pose over time.

```rust
OrbitControls::from_framing(framing).cinematic()      // heavy damping
OrbitControls::from_framing(framing).snappy()         // light damping
OrbitControls::from_framing(framing).presentation()   // medium + slow auto-rotate
OrbitControls::from_framing(framing).turntable(6.0)   // auto-rotate, 6 RPM
```

### 2.9 `AutoExposureConfig` scenario presets

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`.
Owner: `src/render/exposure.rs`
Proof: `tests/round_c_auto_exposure_presets.rs` asserts the scenario
settings and their different EV behavior; doctor rule
`NAMED-AUTO-EXPOSURE-PRESETS` keeps the API, docs, demo sweep, and
visual proof present.
Visual proof: reference-image + docs-image
`target/gate-artifacts/examples-visual/round-c-auto-exposure-preset-reference-docs-image.ppm`
renders one matched scene under each scenario and records the solved EVs
in adjacent metadata.

```rust
AutoExposureConfig::product_studio()    // tight EV range, clean highlights
AutoExposureConfig::indoor()
AutoExposureConfig::outdoor()
AutoExposureConfig::mixed()             // default, conservative
```

---

## 3. Renderer state-of-the-art — three buckets

Not all "state of the art" items are greenfield. Each is tagged by the
bucket it actually sits in.

### 3.1 Renderer feature roadmap

Proof rule for this bucket: visual renderer features need
**reference-image with ON/OFF, before/after, or order-invariance pairs**.
A single "pretty render" only proves that something rendered — it does
not prove the feature is doing anything. Pipeline / compression /
capability / performance items need structural or measured proof first
(decode/import assertions, capability artifacts, package-size/build-time
data, allocation/performance gates), plus a rendered reference only when
the rendered result is part of the contract.

- **Anti-aliasing.** Status: **[shipped]** for the FXAA output-space
  baseline. `AntiAliasing::Fxaa` is the default and
  `Renderer::set_anti_aliasing(AntiAliasing::None)` gives deterministic
  unfiltered output for exact-pixel and ON/OFF proof. MSAA/TAA remain
  future quality lanes, not part of this claim. Proof:
  `anti_aliasing_can_be_disabled_for_on_off_visual_proof` asserts the
  aliased edge stays hard with AA disabled and becomes visibly smoothed
  with FXAA, while `tests/m2_visual_proof.rs` writes the
  `anti-aliasing-on-off` reference artifact.
  Visual proof: reference-image ON/OFF shipped via
  `target/gate-artifacts/m2-visual/anti-aliasing-on-off.ppm`.
- **Contact shadows / SSAO.** Status: **[shipped]** for the headless
  CPU / descriptor-backed depth-buffer baseline. `ScreenSpaceAmbientOcclusionConfig::subtle()`
  and `Renderer::set_screen_space_ambient_occlusion(...)` run a
  depth-aware contact-darkening pass before bloom and FXAA, with
  `RendererStats::ambient_occlusion_passes` and headless
  `Capabilities::screen_space_ambient_occlusion = Supported`. GPU,
  WebGPU, and WebGL2 SSAO remain future backend lanes and are not claimed.
  Proof: `tests/m2_lighting_depth_clipping.rs` asserts a depth-contact
  edge darkens while distant floor pixels stay lighter, and
  `tests/m2_visual_proof.rs` writes the `ssao-contact-on-off` ON/OFF
  reference artifact.
  Visual proof: reference-image ON/OFF shipped via
  `target/gate-artifacts/m2-visual/ssao-contact-on-off.ppm`.
- **Subtle bloom in post.** Status: **[shipped]** for the output-space
  baseline. `PostBloomConfig::subtle()` and `Renderer::set_bloom(...)`
  run a threshold / blur / composite postprocess before FXAA, with
  `RendererStats::bloom_passes` and `Capabilities::bloom = Supported`.
  Proof: `tests/m2_lighting_depth_clipping.rs` asserts the halo is
  visible without a second tonemap, `tests/m2_visual_proof.rs` writes the
  `bloom-on-off` reference artifact, and doctor rule
  `ARCH-FXAA-OUTPUT` keeps the post-output proof wired.
  Visual proof: reference-image ON/OFF at a fixed exposure shipped via
  `target/gate-artifacts/m2-visual/bloom-on-off.ppm`.
- **Material features**: Status: **[shipped]** for clearcoat, sheen,
  anisotropy, iridescence, and dispersion scalar factors plus clearcoat,
  clearcoat-roughness, clearcoat-normal, sheen-color, sheen-roughness,
  anisotropy direction/strength, and iridescence factor/thickness texture
  sampling on the CPU/reference path; transmission, IOR, and volume factors
  plus transmission/thickness texture slots are also parsed and used by the
  CPU/reference path. WebGPU/WebGL2 shader/material resource wiring carries
  punctual-light clearcoat, sheen, anisotropy, and iridescence lobes plus
  dispersion channel-spread shading, and the browser proof records a
  material-extension composite readback. Full physical backend
  transmission/volume glass parity remains a future backend lane and is not
  claimed here.
  Owner: `src/material.rs`, `src/assets/gltf/materials.rs`,
  `src/render/prepare/lighting.rs`, `src/render/gpu/materials.rs`,
  `src/render/gpu/material_uniform.rs`, and
  `src/render/gpu/output_shader*.wgsl`. Proof:
  `m8_clearcoat_material_factors_are_parsed_from_gltf` asserts optional
  `KHR_materials_clearcoat` scalar factors propagate into `MaterialDesc`,
  `m8_clearcoat_texture_slots_are_parsed_from_gltf` asserts clearcoat,
  roughness, normal texture slots, transforms, and normal scale propagate,
  `m8_clearcoat_png_textures_affect_cpu_preview_pixels` proves the CPU
  preview samples the clearcoat texture red channel and roughness texture
  green channel,
  `m8_clearcoat_normal_texture_affects_cpu_preview_pixels` proves the CPU
  preview samples clearcoat normal textures for the clearcoat lobe,
  `m8_sheen_material_factors_are_parsed_from_gltf` asserts optional
  `KHR_materials_sheen` factors propagate into `MaterialDesc`,
  `m8_sheen_texture_slots_are_parsed_from_gltf` asserts sheen color and
  roughness texture slots plus transforms propagate,
  `m8_sheen_png_textures_affect_cpu_preview_pixels` proves the CPU preview
  samples sheen color RGB and roughness alpha channels,
  `m8_anisotropy_material_factors_are_parsed_from_gltf` asserts optional
  `KHR_materials_anisotropy` strength and rotation factors propagate into
  `MaterialDesc`,
  `m8_anisotropy_texture_slot_is_parsed_from_gltf` asserts anisotropy
  texture slots plus transforms propagate,
  `m8_anisotropy_png_texture_affects_cpu_preview_pixels` proves the CPU
  preview samples anisotropy texture direction and strength channels,
  `m8_iridescence_material_factors_are_parsed_from_gltf` asserts optional
  `KHR_materials_iridescence` factor, IOR, and thickness range propagate
  into `MaterialDesc`,
  `m8_iridescence_texture_slots_are_parsed_from_gltf` asserts iridescence
  factor/thickness texture slots plus transforms propagate,
  `m8_iridescence_png_textures_affect_cpu_preview_pixels` proves the CPU
  preview samples the iridescence factor red channel and thickness texture
  green channel,
  `m8_dispersion_material_factor_is_parsed_from_gltf` asserts optional
  `KHR_materials_dispersion` factors propagate into `MaterialDesc`,
  `m8_dispersion_factor_affects_cpu_preview_pixels` proves the CPU preview
  applies dispersion instead of silently ignoring the factor,
  `m8_transmission_ior_volume_material_factors_are_parsed_from_gltf`
  asserts optional `KHR_materials_transmission`, `KHR_materials_ior`, and
  `KHR_materials_volume` factors propagate into `MaterialDesc`,
  `m8_transmission_volume_textures_affect_cpu_preview_pixels` proves the
  CPU preview samples transmission red-channel and thickness green-channel
  texture data,
  `clearcoat_light_contribution_adds_dielectric_lobe` keeps the PBR
  math owned by `pbr_contract`,
  `sheen_light_contribution_adds_colored_lobe` keeps the sheen lobe owned
  by `pbr_contract`,
  `anisotropy_light_contribution_uses_strength_texture_and_direction`
  keeps the anisotropy lobe owned by `pbr_contract`,
  `iridescence_light_contribution_uses_factor_thickness_and_textures`
  keeps the iridescence lobe owned by `pbr_contract`, and
  `dispersion_light_contribution_uses_factor_and_ior_spread` keeps the
  dispersion lobe owned by `pbr_contract`,
  `transmission_volume_uses_factor_ior_thickness_and_attenuation` keeps
  the transmission/volume contribution owned by `pbr_contract`,
  `material_uniform_upload_encodes_material_factors`,
  `triangle_shader_applies_clearcoat_lobe_in_native_and_webgl2_variants`,
  `triangle_shader_applies_sheen_lobe_in_native_and_webgl2_variants`,
  `triangle_shader_applies_anisotropy_lobe_in_native_and_webgl2_variants`,
  `triangle_shader_applies_iridescence_lobe_in_native_and_webgl2_variants`,
  `triangle_shader_applies_dispersion_lobe_in_native_and_webgl2_variants`,
  `material_resources_define_shader_visible_texture_bindings`, and
  `backend_material_slots_preserve_all_texture_roles_and_material_only_slots`
  pin GPU uniform, shader, bind-group, and prepare-resource contracts.
  `m8_headless_gpu_clearcoat_texture_lobe_brightens_pbr_output_when_available`
  is fail-closed by default and records the unapproved GPU release lane until
  `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1` is set on an approved
  backend proof lane. Browser proof: the M6 `pbr-material-extensions`
  workflow records nonblack WebGPU/WebGL2 readback, extension metadata, and
  material texture binding stats for the clearcoat/sheen/anisotropy/
  iridescence/dispersion composite. CPU visual proof:
  `m8_headless_visual_artifacts_cover_material_texture_environment_paths`
  writes the `m8-clearcoat-material-feature` and
  `m8-sheen-material-feature`, `m8-anisotropy-material-feature`, and
  `m8-iridescence-material-feature`, `m8-dispersion-material-feature`, and
  `m8-transmission-volume-material-feature` before/after artifacts.
  Visual proof: reference-image before/after shipped for scalar clearcoat
  and sheen via `target/gate-artifacts/m8-visual/m8-clearcoat-material-feature.ppm`
  and `target/gate-artifacts/m8-visual/m8-sheen-material-feature.ppm`,
  and for anisotropy via
  `target/gate-artifacts/m8-visual/m8-anisotropy-material-feature.ppm`,
  and for iridescence via
  `target/gate-artifacts/m8-visual/m8-iridescence-material-feature.ppm`,
  and for dispersion via
  `target/gate-artifacts/m8-visual/m8-dispersion-material-feature.ppm`,
  and for transmission/volume via
  `target/gate-artifacts/m8-visual/m8-transmission-volume-material-feature.ppm`;
  physical GPU/WebGPU/WebGL2 transmission/volume glass parity remains a
  future backend lane.
- **WebGL2 IBL prefilter quality for smooth metals.** Status:
  **[shipped]** for the raised interactive sample schedule used by
  chrome/brushed-metal presets. `InteractiveWebGl2` keeps a bounded
  first-frame budget but no longer uses the old 4/8/16 sample table that
  flattened smooth-metal reflections toward mean radiance.
  Proof: `interactive_prefilter_profile_caps_browser_runtime_work`
  pins the WebGL2 sample floor below the Reference profile but high
  enough for roughness-0.28 metal, and M6 material-preset browser proof
  renders the expanded preset set through WebGL2/WebGPU.
- **Clustered / tiled light culling.** Status: **[deferred]**. Not a
  v1.4/v1.5 release-closure item; keep it as future GPU/backend scale work.
  Babylon 9 made this baseline.
  Proof: many-light stress scene proves correct light selection,
  stable frame time / allocation behavior, and no dropped-light fallback.
  Visual proof: reference-image of the stress scene; not an ON/OFF gate.
- **Area lights with LTC** (rect/disc/sphere). Status:
  **[deferred]**. Not a v1.4/v1.5 release-closure item; keep it as a future
  physically richer lighting lane.
  Visual proof: reference-image before/after per light shape.
- **Screen-space reflections (SSR).** Status: **[deferred]**. Not a
  v1.4/v1.5 release-closure item; keep it as a future reflective-floor backend
  lane.
  Visual proof: reference-image ON/OFF on a reflective-floor control.
- **Order-independent transparency (OIT).** Status: **[shipped]** for
  the headless CPU / descriptor-backed weighted-blended baseline.
  `OrderIndependentTransparencyConfig::weighted_blended()` and
  `Renderer::set_order_independent_transparency(...)` resolve transparent
  overlap from a per-pixel accumulator, with
  `RendererStats::order_independent_transparency_passes` and headless
  `Capabilities::order_independent_transparency = Supported`. GPU,
  WebGPU, and WebGL2 OIT remain future backend lanes and are not claimed.
  Proof: `weighted_blended_transparency_is_order_independent_for_overlaps`
  asserts opposite insertion orders produce the same overlap pixel, and
  `tests/m2_visual_proof.rs` writes the
  `oit-overlap-order-invariance` reference artifact.
  Visual proof: reference-image order-invariance pair shipped via
  `target/gate-artifacts/m2-visual/oit-overlap-order-invariance.ppm`.
- **Wide-gamut output (Display P3)** — Status: **[shipped]** for
  capability-gated reporting, renderer-owned browser output configuration,
  and browser proof evidence. PBR Neutral targets sRGB by default;
  `RendererOptions::with_output_color_space(OutputColorSpace::DisplayP3)`
  requests the wide-gamut path, `Capabilities::wide_gamut_output` remains
  disabled for headless/unattached reports and degraded until the active
  browser canvas is configured as Display P3, and
  `DiagnosticCode::WideGamutOutputUnavailable` is emitted when unavailable.
  Proof: `display_p3_output_requires_explicit_canvas_configuration_proof`
  asserts the public option/capability contract, the M6 browser probe records
  `scenaM6DisplayP3OutputProbe` for WebGL2 `drawingBufferColorSpace` and
  WebGPU `GPUCanvasConfiguration.colorSpace` with effective `display-p3`,
  and `doctor --full` source-enforces the option, capability, browser output
  configuration, and proof artifact shape.
- **Draco mesh compression** (`KHR_draco_mesh_compression`). Status:
  **[deferred]**. Mirrors the §1.3 Draco decision; not a v1.4
  critical-path item.
  Proof: decode/import assertions against a known compressed fixture,
  package-size/build-time impact for any optional feature, and a rendered
  reference proving the decoded asset survives the normal pipeline.
  Visual proof: reference-image compared to the uncompressed control, not
  an ON/OFF renderer-feature gate.
- **GPU instancing import** (`EXT_mesh_gpu_instancing`) — Status:
  **[shipped]** for local parsing and import into scene-owned
  `InstanceSet` nodes. Upstream `gltf-rs` typed support is still worth
  contributing, but v1.4 no longer blocks on it.
  Proof: extension parse/import assertions for instance count and
  transforms, plus a rendered repeated-part fixture.
  Visual proof: reference-image of the repeated-part fixture; not an
  ON/OFF renderer-feature gate.

### 3.2 Ergonomics closed out

These started as source-level features with missing user surfaces or
default stories. The active rows below are now status-tagged explicitly.

Visual proof for this bucket: **reference-image of a known asset rendered
through each feature path** (KTX2-textured asset render, meshopt
asset render, animation clip at fixed timestamps). For animation
specifically, add **animated-proof** of the clip playing back.

- **Animation update flow.** Status: **[shipped]** for scene-owned
  mixer creation, one-call named playback, and rendered-output proof.
  `src/scene/mixers.rs` has `create_animation_mixer`, `play_animation`,
  `pause`, `stop`, `seek`, `set_speed`, `set_loop_mode`,
  `update_animation`, and `Scene::play_animation_by_name`, which creates
  the mixer, starts it, and returns the typed mixer handle. Viewer sugar
  is also shipped through `HeadlessGltfViewer::play_clip(...)` and
  `InteractiveGltfViewer::play_clip(...)`. Proof:
  `tests/round_c_animation_playback.rs` proves the named helper starts a
  mixer and moves an imported animated node, while
  `round_c_animation_playback_reference_animated_docs_image` renders a
  generated frame sequence at fixed timestamps and asserts the rendered
  frames change.
- **KTX2 / Basis textures.** Status: **[shipped]** for the optional
  production profile. The decode path and feature flag remain opt-in, and
  feature-gated rendered proof covers material texture roles. Native GPU
  upload and browser release lanes remain future proof work.
- **meshopt compression.** Status: **[shipped]** for the optional
  production profile. The feature-gated proof suite renders decoded
  compressed fixtures for the supported bufferView modes and metadata
  paths. Native GPU and browser release lanes remain future proof work.
- **glTF extension diagnostics.** Status: **[shipped]**.
  `GltfExtensionDiagnostic` exposes typed `extension`, `status`, `help`,
  `suggested_fix`, and `decoder_policy` metadata for importer UIs. The
  `asset-doctor` lane combines official Khronos glTF Validator output with
  scena-native `fix` guidance for renderer-specific extension policy.
  Proof: `m8_optional_real_world_gltf_extensions_report_degradation_metadata`
  asserts actionable fixes for clearcoat and Draco, `tests_15` covers
  official-validator mode and required-clearcoat guidance, and
  `ASSET-VALIDATION-DOCTOR` source-enforces the CLI/docs/library surface.

### 3.3 Visual proof work

The pipeline runs but no stored reference asserts the visual is right.
This section IS the reference-image work; closing every item below
produces a stored PNG with a CI diff threshold.

- Animation clip rendered-output regression test — **[shipped]** through
  `round_c_animation_playback_reference_animated_docs_image`, which
  renders a generated frame sequence at fixed timestamps and asserts a
  visible frame change.
- KTX2-textured asset rendered-output regression test — **[shipped]**
  locally through `tests/m8_compressed_asset_release_proof.rs` with
  `--features production-assets`.
- meshopt-compressed asset rendered-output regression test — **[shipped]**
  locally through `tests/m8_compressed_asset_release_proof.rs` with
  `--features production-assets`.
- Transmission + IBL combo capability evidence on the headless GPU lane —
  **[proof-gap]**. CPU/reference transmission + IBL proof exists, and
  browser preset proof records blend-mode glass previews with IBL. A
  dedicated headless-GPU gate now exists at
  `m8_headless_gpu_transmission_volume_ibl_capability_when_available`;
  the current local artifact is fail-closed until an approved GPU lane
  runs it with `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1`.
- Per-backend capability matrix evidence (Vulkan / Metal / DX12 / WebGPU
  / WebGL2 fallback) — **[proof-gap]** for non-Linux host lanes only.
  The local `m9-capability-matrix.json` now folds the M6 browser proof
  into measured WebGL2/WebGPU rows and folds the wasm-size gate into the
  wasm lane when those artifacts exist; it still reports
  `status: incomplete` until macOS Metal and Windows DX12 hosts upload
  measured lane artifacts.

---

## 4. Tier 2 — ease-of-use ergonomics

### 4.1 Bundled Khronos sample loader

Status: **[shipped]** for the checked fixture catalog — implemented in
`src/assets/khronos.rs` behind the `khronos-samples` feature. The shipped
API exposes the current audited fixtures (`WaterBottle`, `TransmissionTest`,
animation/skin/morph samples, texture samples, unlit/alpha samples) through
`assets.khronos()`. Do not claim `DamagedHelmet` or `DragonAttenuation`
until those specific assets are vendored or fetched through a checked cache.
Proof: `tests/round_c_khronos_samples.rs` loads every catalog entry without
user-supplied local file paths, checks source/license/checksum/file-list
metadata and package-size budget, and generates
`target/gate-artifacts/khronos-samples/rigged-simple-sample-loader-reference.ppm`.
No sample bytes are embedded into the library binary; the default feature set
remains unchanged.
Visual proof: reference-image shipped for one catalog path; browser-demo and
per-sample reference grid remain follow-up proof work.

```rust
assets.khronos().water_bottle().await?
assets.khronos().rigged_simple().await?
assets.khronos().transmission_test().await?       // transmissive control
```

### 4.2 `OrbitControls` bounds-relative zoom

Status: **[shipped]**
Owner: `src/controls.rs`
Proof: `tests/round_d_orbit_zoom_limits.rs` asserts limits are derived
from the current framed distance and that wheel / pinch input clamps at
both extremes; doctor rule `ORBIT-ZOOM-LIMITS` keeps the API, tests,
docs, and generated proof present.
Visual proof: animated-proof via
`target/gate-artifacts/examples-visual/round-d-orbit-zoom-limit-animated-docs-image.ppm`
(contact sheet shows start, clamped minimum, repeated minimum, and
clamped maximum).

```rust
OrbitControls::from_framing(framing).zoom_limits_bounds_relative(0.5, 4.0)
```

### 4.3 `ConnectOptions::with_axial_gap` / unit-aware clearance helper

Status: **[shipped]** for scene-unit axial gaps; `with_clearance_mm`
remains intentionally absent until imported source-unit metadata can make
it fail closed.
Owner: `src/scene/connectors/`
Proof: `tests/round_d_connector_axial_gap.rs` asserts that
`with_axial_gap` offsets along the target connector's forward axis and
sanitizes invalid gaps to zero; doctor rule `CONNECTOR-AXIAL-GAP` keeps
the API, test, docs, and `with_clearance_mm` absence present.
Visual proof: docs-image optional — the behavior is placement math and
covered by deterministic connector tests.

```rust
options.with_axial_gap(0.4)
options.with_clearance_mm(2.5)        // only when source units are known
```

`with_clearance_mm` must fail closed or be absent when imported source
units are unknown. Otherwise it suggests physical precision the connector
system cannot prove.

### 4.4 One-call animation playback by clip name

Status: **[shipped]** — `Scene::play_animation_by_name` is the primary
scene surface, and `HeadlessGltfViewer::play_clip(...)` /
`InteractiveGltfViewer::play_clip(...)` are thin viewer conveniences for
the loaded import.
Owner: `src/scene/mixers.rs` for scene-owned mixer state and
`src/viewer/animation.rs` for viewer sugar.
Proof: `tests/round_c_animation_playback.rs` proves the scene helper
creates and starts a mixer, returns the typed handle for update/loop/speed
control, and that a headless viewer can start the same named clip without
manually reaching through the import. Doctor rule
`ONE-CALL-ANIMATION-PLAYBACK` keeps the API, example, docs, and rendered
proof present.
Visual proof: reference-image + animated-proof
`target/gate-artifacts/examples-visual/round-c-animation-playback-reference-animated-docs-image.ppm`
and its generated frame sequence render a visible child of an imported
animated node at fixed timestamps.

```rust
// preferred primary surface: Scene owns mixer creation and playback state
let idle = scene.play_animation_by_name(&import, "idle")?;

scene.set_animation_loop_mode(idle, AnimationLoopMode::Once)?;
```

Implementation decision: `Scene::play_animation_by_name` remains the
owner API because the scene owns mixer handles. Viewer `play_clip` is
deliberately thin sugar; it does not hide animation update, prepare, or
render inside the call.

### 4.5 Viewer pointer callbacks

Status: **[shipped]** — `InteractiveGltfViewer::pick_at(x, y)`
exists in `src/viewer.rs`, and `on_click` / `on_hover` callbacks now
route through the same typed picking path as `click_at` / `hover_at`.
Owner: `src/viewer.rs`
Proof: `tests/round_d_viewer_pointer_callbacks.rs` asserts that click
and hover callbacks receive hit and no-hit results without bypassing the
existing picking API; `xtask doctor --full` rule
`VIEWER-POINTER-CALLBACKS` pins the API, test, docs, and visual proof.
Visual proof: animated-proof + docs-image via
`target/gate-artifacts/examples-visual/round-d-viewer-pointer-callback-animated-docs-image.ppm`;
the frame sequence renders the imported viewer scene plus callback-state
markers generated from the actual callback results. Browser-demo remains
deferred to the `<scena-viewer>` / live demo surface.

```rust
viewer.on_click(|result| ...);
viewer.on_hover(|result| ...);
viewer.click_at(x, y)?;
viewer.hover_at(x, y)?;
```

### 4.6 Screenshot one-liner

Status: **[shipped]** — viewer capture APIs now encode the current
RGBA8 frame through the existing `png` crate. `capture_png_bytes()` is
available on `FirstRender`, `HeadlessGltfViewer`, and
`InteractiveGltfViewer`; `capture_png(path)` is native-only file output.
`HeadlessGltfViewerBuilder::render_png_bytes()` and native
`render_png(path)` cover the no-GPU one-shot asset-pipeline path.
Owner: `src/viewer/capture.rs`
Dependency note: use the already-present `png` crate directly for
`capture_png_bytes`; keep `image` only where broader format support is
actually exercised. Add `gif` only behind an optional stretch feature.
Proof: `tests/round_d_viewer_capture_png.rs` asserts that
`capture_png_bytes()` decodes as an RGBA8 PNG with the same dimensions
and bytes as `snapshot_rgba8`, and that `capture_png(path)` writes the
same bytes behind native-only filesystem support. `xtask doctor --full`
rule `VIEWER-CAPTURE-PNG` pins the API, test, docs, and direct `png`
crate usage. GIF remains stretch.
Visual proof: reference-image via
`target/gate-artifacts/viewer-capture/viewer-capture-png-reference.png`;
the captured PNG is itself the proof.

```rust
viewer.capture_png("frame.png")?;
viewer.capture_png_bytes()?;
headless_gltf_viewer("machine.glb").render_png_bytes().await?;
viewer.capture_gif("turntable.gif", Duration::from_secs(4))?;   // stretch
```

### 4.7 Asset hot-reload during dev

Status: **[shipped]** — native `hot-reload` feature adds
`Assets::watch_scene_for_hot_reload`, backed by
`notify-debouncer-full`, so a save drains as a debounced asset-path
reload decision. WASM mechanism remains a separate explicit contract.
Owner: `src/assets/hot_reload.rs`
Proof: `tests/round_d_asset_hot_reload.rs` writes a retained glTF asset
to disk, starts a debounced watcher, edits the asset, drains exactly one
changed `AssetPath`, reloads through `Assets::reload_scene`, replaces the
existing scene import, prepares, and rerenders. `xtask doctor --full`
rule `ASSET-HOT-RELOAD` pins the feature, dependency, API, test, docs,
and proof artifact. Browser/WASM remains separate; this does not hide
fetch/reload/prepare inside `render()`.
Visual proof: animated-proof via
`target/gate-artifacts/asset-hot-reload/asset-hot-reload-animated-proof.ppm`;
the generated before/after strip shows the watched glTF color edit
rendering without rebuilding the scene by hand.

### 4.8 Drag-and-drop in the WASM viewer

Status: **[shipped]** — `<scena-viewer>` now owns the browser
drag/drop ingestion surface, validates dropped `.glb` / `.gltf`
filenames through `ScenaViewerDropDecision`, and has Playwright
render-after-drop proof **[shipped]** for accepted/rejected drop events
plus the rendered accepted result.
Owner: `<scena-viewer>` (bet 1.1).
Proof: native drop-decision test in `tests/scena_viewer_element.rs`;
custom element dispatches `scena-viewer-file-drop` for accepted `File`
objects and `scena-viewer-drop-error` for rejected drops. The focused M6
browser proof writes
`target/gate-artifacts/scena-viewer-element-browser-proof.png` and records
`scena.scena_viewer_element_browser_proof.v1`; it drops a GLB `File`,
loads the dropped bytes through the browser asset pipeline, renders the
accepted result into the element canvas with proof class
`scena-viewer-drop-render`, asserts visible pixels, and records
viewer-level auto-framing browser proof **[shipped]** under
`viewer-level-auto-framing` with projected-bounds containment, centering,
and fill-fraction checks.
Visual proof: browser-demo shipped through the generated custom-element
browser proof artifact. Animated recording remains optional polish for
the public Cloudflare demo, not a blocker for the library contract.

### 4.9 State-via-URL serializer

Status: **[shipped]** — `CameraOrbitUrlState` lives in
`src/controls/url_state.rs` with helpers on `FramingOutcome` and
`OrbitControls`.
Dependency note: direct `serde` derives are used for structured camera/orbit
state and `urlencoding` is used for percent-encoding. Serialized output
contains only camera state; asset URLs with credentials, tokens, and other
application query parameters are ignored on parse and never emitted.
Proof: `tests/round_d_viewer_url_state.rs` covers camera/orbit round-trip,
compact checklist query compatibility, `FramingOutcome` export, `serde`
round-trip, and a privacy test proving credentialed asset URLs and tokens
are not serialized.
Visual proof: none (URL serialization is text spec)

```rust
?camera-orbit=-28deg%2018deg%202.5m // model-viewer-style query value
```

---

## 5. Ease-of-use signature opportunities

Cross-cutting features that delight users; each corresponds to a
specific competitor primitive.

- **Bundled studio environments as a Rust enum.** Status: **[shipped]**
  for the checked HDR/fixture catalog; KTX2 cubemap presets remain future
  work because the environment loader does not yet own a KTX2 cubemap
  decode path. Owner: `src/assets/environment_preset.rs`. The shipped
  `EnvironmentPreset` catalog exposes `NeutralStudio` and `Studio` with
  license/checksum/file-list/source metadata and a package-size budget.
  Proof: `tests/round_c_environment_presets.rs` loads each preset without
  user-supplied paths and writes
  `target/gate-artifacts/environment-presets/environment-preset-reference-docs-image.ppm`.
  Visual proof: reference-image + docs-image shipped for the checked catalog; browser-demo and KTX2 cubemap preset grid remain follow-up proof work.
  ```rust
  let environment = assets.load_environment_preset(EnvironmentPreset::Studio).await?;
  renderer.set_environment(environment);
  // checked variants today: NeutralStudio, Studio
  ```
- **Camera control kit.** Status: **[shipped]** for library primitives,
  Rust/WASM browser input-to-motion proof, and custom-element gesture
  event proof. Owner:
  `src/controls.rs`. `OrbitControls` covers orbit, turntable, and
  presentation movement; `FollowControls` tracks a scene node from a named
  offset; `FlyControls` exposes host-driven local movement and look deltas
  without platform coupling. `<scena-viewer>` now emits host-wirable
  `scena-viewer-gesture-control` events for orbit, pinch zoom, and wheel
  zoom when `camera-controls` is enabled.
  Proof: `tests/camera_control_kit.rs` covers Follow/Fly scene application;
  the M6 Playwright probe exports
  `scena.m6.camera_control_kit_browser_proof.v1`, runs real
  `PointerEvent` input through `OrbitControls`, applies `FollowControls`
  and `FlyControls` in Rust/WASM, and writes
  `target/gate-artifacts/camera-control-kit-browser-proof.png`. The
  `CAMERA-CONTROL-KIT` and `VISUAL-BROWSER-M6` doctor rules pin the public
  API, guide, checklist, test, and browser-proof contract. The M6 mobile
  proof also records the custom-element touch orbit, pinch zoom, wheel
  zoom, and keyboard reset event surface.
  Visual proof: browser-demo shipped for browser input-to-motion via the
  generated Playwright artifact plus
  `target/gate-artifacts/scena-viewer-mobile-a11y-browser-proof.png`.
- **Picking + outline + hover.** Status: **[shipped]** for the library
  renderer surface. Owner: `src/picking.rs` + `src/render/`.
  `Scene::pick_and_select_with_assets` and
  `Scene::pick_and_hover_with_assets` update typed interaction state;
  `InteractionStyle::outline` plus `Renderer::set_hover_style` and
  `Renderer::set_selection_style` make hover/selection visible in
  rendered output; viewer callbacks already route through the same
  picking path. Proof:
  `examples_visual_picking_selection_hover_renders_styled_pick_to_ppm`
  renders the styled pick path, and doctor rule `PICKING-OUTLINE-HOVER`
  pins the source API, guide, checklist, and visual proof. The future
  `<scena-viewer>` live demo can still add a richer browser recording,
  but the renderer/library contract is closed.
  Visual proof: reference-image shipped via the generated
  `picking_selection_hover` artifact; custom-element animated browser
  demo remains follow-up polish.
- **HTML/CSS annotation overlay anchored to 3D points.** Status:
  **[shipped]** for the slotted overlay, projection contract, and
  annotation tracking proof **[shipped]**. Owner: `<scena-viewer>` (bet 1.1).
  `ScenaViewerAnnotationAnchor` parses `data-position`,
  `data-normal`, and `data-surface`; the element emits
  `scena-viewer-annotations-request` with parsed anchors and accepts
  `setAnnotationProjections([{ id, x, y, visible }])` before emitting
  `scena-viewer-annotations-rendered`. The M6 browser proof verifies the
  rendered projection, records an `annotation_tracking_sequence` with two
  different screen transforms, asserts `annotation_update_visible`, and
  includes the custom-element screenshot artifact.
  Visual proof: browser-demo + animated-proof shipped through
  `target/gate-artifacts/scena-viewer-element-browser-proof.png`.
- **Variant switching for `KHR_materials_variants`.** Status:
  **[shipped]** overall; Viewer primitive, reference/docs-image proof,
  `<scena-viewer>` picker surface, and picker-to-rendered-variant proof
  **[shipped]**.
  Extension diagnostics mark it supported and
  `Scene::set_active_variant(&import, Some(name))` exists. Viewers now
  expose `material_variants()`, `active_material_variant()`, and
  `set_active_material_variant(name)`; the setter delegates to the scene
  API and re-prepares before the next render. `<scena-viewer>` exposes
  `ScenaViewerVariantSelection`, `setMaterialVariants(...)`, and
  `scena-viewer-variant-change` for a host-owned picker-to-renderer
  binding. The M6 custom-element proof selects `noon`, renders
  `tests/assets/gltf/material_variants_scene.gltf` into the element
  canvas through `scena-viewer-material-variant-render`, and asserts
  visible green-dominant pixels from the selected material. Owner:
  `src/viewer.rs` + `src/viewer_element.rs` +
  `src/browser_probe/material_variant.rs`.
  Visual proof: reference-image + docs-image shipped through
  `target/gate-artifacts/examples-visual/viewer-material-variant-reference-docs-image.ppm`;
  custom-element browser-demo proof for picker events shipped through
  `target/gate-artifacts/scena-viewer-element-browser-proof.png`;
  picker-to-rendered-variant proof **[shipped]** through the same artifact
  and proof class `scena-viewer-material-variant-render`.
- **Loading progress primitives.** Status: **[shipped]** for loader,
  viewer, and `<scena-viewer>` progress sequencing.
  `AssetLoadProgress` exists in `src/lib.rs` and is now surfaced
  through `HeadlessGltfViewerBuilder::build_with_progress`,
  `HeadlessGltfViewerBuilder::render_with_progress`,
  `InteractiveGltfViewerBuilder::build_with_progress`,
  `InteractiveGltfViewerBuilder::build_async_with_progress`, plus
  `load_progress_events()` accessors on built viewers and `FirstRender`.
  `<scena-viewer>` now exposes `ScenaViewerProgress` /
  `ScenaViewerProgressPhase`, a shadow DOM `progressbar`, structured
  `scena-viewer-progress` ingestion, and a
  `scena-viewer-progress-rendered` event.
  Proof: loader progress test over cache hit, external buffer, texture
  decode, and cancellation paths; viewer progress tests in
  `tests/first_render_api.rs` and `tests/m7_interactive_viewer.rs`;
  custom-element progress mapping in `tests/scena_viewer_element.rs`;
  focused Playwright proof in the M6 browser probe records
  `progress_sequence` across `loading` and `fetching` phases; doctor rule
  `SCENA-VIEWER-ELEMENT` pins the browser UI surface and
  loading progress sequence proof **[shipped]**.
  Visual proof: browser-demo proof shipped through
  `target/gate-artifacts/scena-viewer-element-browser-proof.png`.
- **Mobile-first + a11y defaults.** Status: **[shipped]** for
  mobile/ARIA/keyboard defaults and touch gesture browser proof.
  Owner: `<scena-viewer>` (bet 1.1). `ScenaViewerAccessibilityDefaults`
  `ScenaViewerKeyboardAction`, and `ScenaViewerGestureAction` define the
  source contract; the element sets host role/label/tabindex defaults,
  keeps the canvas touch-safe, emits `scena-viewer-key-control` for
  keyboard orbit/zoom/reset events, and emits
  `scena-viewer-gesture-control` for `orbit`, `pinch-zoom`, and
  `wheel-zoom` host wiring.
  Browser proof now covers host role/label/tabindex, roledescription,
  canvas `touch-action: none`, keyboard event dispatch, mobile viewport
  overflow behavior, and mobile/a11y gesture proof **[shipped]** through
  `scena.scena_viewer_mobile_a11y_browser_proof.v1`.
  Visual proof: browser-demo shipped through
  `target/gate-artifacts/scena-viewer-mobile-a11y-browser-proof.png`.
- **Inspector / dev overlay.** Status: **[shipped]** for the host-fed
  overlay and browser snapshot fixture.
  Owner: `crates/xtask/` doctor integration + `<scena-viewer>`.
  `ScenaViewerInspectorSnapshot` turns renderer debug overlay,
  diagnostics, and render stats into a testable snapshot; the element
  exposes `setInspectorSnapshot(...)`, `setInspectorDiagnostics(...)`,
  `clearInspectorSnapshot()`, and emits
  `scena-viewer-inspector-rendered`. Browser overlay snapshot proof now
  ships through the M6 custom-element proof artifact and is fed by
  `tests/assets/viewer/inspector_snapshot.json` with schema
  `scena.scena_viewer_inspector_snapshot.v1`; doctor rules
  `SCENA-VIEWER-ELEMENT` and `VISUAL-BROWSER-M6` pin the fixture, page
  fetch, schema assertion, and screenshot proof.
  Visual proof: browser-demo + reference-image shipped through the live
  overlay screenshot in
  `target/gate-artifacts/scena-viewer-element-browser-proof.png`.

---

## 6. Differentiators scena could uniquely own

These are not blanket "no competitor has this" claims. They are places
scena could own a distinct Rust / digital-twin workflow if implemented
with proof and a clean public surface.

- **Connector "magnet" snapping with visual cues.** Status:
  **[shipped]** for the library-level magnetic preview and browser
  rendered cue proof. Owner:
  `src/scene/connectors/`. `Scene::preview_connector_magnet` reuses the
  existing connection solver and returns `ConnectionMagnetPreview` with
  distance, tolerance, ghost transform, connection line, and
  `ConnectionMagnetVisualCue` styling (`scena-magnet-ready` /
  `scena-magnet-out-of-range`). The M6 browser workflow
  `connector-magnet-preview` renders both the out-of-range and snap-ready
  states, records the measured distance/tolerance sequence, and asserts
  visible browser pixels through `assertConnectorMagnetPreviewProof`.
  Literal pointer-driven drag interaction remains part of the broader
  `<scena-viewer>` renderer parity bet.
  Visual proof: browser-demo + reference-image via
  `target/gate-artifacts/m6-rust-wasm-renderer-probe.json`
  (`connector-magnet-preview` workflow with `magnet_sequence`).
- **CPU rasterizer fallback for no-GPU screenshots.** Status:
  **[shipped]**. Owner: `src/viewer/capture.rs`.
  `HeadlessGltfViewerBuilder::render_png_bytes()` and native
  `render_png(path)` load, frame, render, and encode through the CPU
  headless renderer without requesting a GPU adapter. The structured
  error type is `ViewerPngError`.
  Proof: `tests/round_d_viewer_capture_png.rs` asserts that one-shot
  bytes and file PNGs decode to the requested dimensions and contain visible
  CPU-rendered pixels; doctor rule `VIEWER-HEADLESS-PNG` pins the API,
  docs, test, and checklist entry.
  Visual proof: reference-image via the same viewer-capture PNG artifact;
  a dedicated CPU/no-GPU diff fixture can be added when public
  reference-image tooling lands.
- **Reference-image regression as a public API.** Status:
  **[shipped]** for owned RGBA8 images. Owner: `src/reference_image.rs`.
  `ReferenceImage::from_rgba8`, `regress`, and
  `regress_with_tolerance` expose exact and tolerance-based comparison
  without tying the API to one asset loader, renderer backend, or file
  layout. PNG/file decoding can stay in user code or test helpers; the
  public contract starts at deterministic RGBA8 frames.
  Proof: `tests/reference_image_regression_api.rs` covers exact match,
  tolerance failure reports, invalid RGBA lengths, and dimension
  mismatches. Doctor rule `REFERENCE-IMAGE-REGRESSION` pins the API,
  docs, test, and checklist entry.
  Visual proof: reference-image (self-referential — this feature is the
  public reference-image comparison primitive for end users)

---

## 7. Doctor enforcement pattern

For every Tier-1 named primitive that lands, add a doctor rule in the
same shape — but with an **allowlist clause** so escape hatches stay
teachable.

- [x] Ban inline raw RGB/RGBA constructors such as
      `Color::from_linear_rgba(<lit>, ...)` or
      `Color::from_srgb(<lit>, ...)` / `Color::from_srgb_u8(<lit>, ...)`
      in first-path examples and `src/demo_page*` **except** in the
      dedicated color escape-hatch example. Do not ban
      `Color::from_kelvin`; that is one of the named conveniences this
      roadmap wants.
- [x] Ban first-path camera FOV literals once lens presets land: direct
      `vertical_fov: Angle::from_degrees(<lit>)`, raw FOV setter calls, or
      equivalent. Do not key the rule to dead API names like
      `with_fov(<float>)`.
- [x] Ban inline `with_damping(<float>)` in `src/demo_page*` if a named
      damping preset would do.
- [x] Ban inline `Quat::from_*(<float>, ...)` in `examples/` **except**
      in the dedicated transform escape-hatch example.
- [x] Ban inline `look_from(Vec3::new(<lit>, <lit>, <lit>))` and
      `orbit(<lit>, <lit>)` in `src/demo_page*` (already in v1.3.0).

The rule shape: **wherever the library ships a name, the first-path
examples and the demo must use the name; one dedicated example per area
demonstrates the escape hatch**. Without the allowlist, the docs become
artificially clean and users cannot see how to go beyond presets.

Rule shape lesson from v1.3.0: bind rules to the **residue pattern**
(inline-float-literal in a setter call), not to dead API names.
Before enabling a doctor rule, either create the allowlist teaching
example named by the rule or point the allowlist at an existing file; the
doctor fixture should fail before the implementation sweep and pass after.

---

## 8. Shipping rounds

Sized to land independently. Each round closes a coherent slice rather
than spreading work horizontally. Bets 1.1–1.4 are funded **alongside**
the rounds, not after — they're the strategic arc.

### Round A — name, not number (atomic, low risk)

1. - [x] `Color` constants + `from_hex` + `from_kelvin` (§2.1)
2. - [x] `PerspectiveCamera` lens presets + drop `with_aspect` (§2.2, §2.3)
3. - [x] `Transform` alias decision + `looking_at` (§2.4)

### Round B — easy by name, continued

4. - [x] Light presets (§2.5)
5. - [x] `MaterialDesc` honest PBR presets (§2.6 — expanded set with WebGL2 IBL proof)
6. - [x] `Background` enum (§2.7)
7. - [x] `OrbitControls` named damping presets (§2.8)

### Round C — bundled content + feature shortcuts

8. - [x] `EnvironmentPreset::*` checked environment presets (§5)
9. - [x] `Assets::khronos::*` sample loaders (§4.1)
10. - [x] `AutoExposureConfig` scenario presets (§2.9)
11. - [x] Scene / Viewer one-call animation playback by clip name (§4.4)

### Round D — Tier 2 ergonomics

12. - [x] `ConnectOptions::with_axial_gap` (§4.3)
13. - [x] `OrbitControls` bounds-relative zoom (§4.2)
14. - [x] `Viewer::on_click` / `on_hover` callbacks (§4.5)
15. - [x] `Viewer::capture_png` + headless `render_png_bytes` (§4.6, §6)
16. - [x] Asset hot-reload (§4.7)
17. - [x] State-via-URL (§4.9)

### Strategic arc (parallel with rounds)

- **Bet 1.1** — `<scena-viewer>` custom element (large; phased delivery)
- **Bet 1.2** — auto-framing default at viewer level (medium; mostly
  custom-element / helper API + proof because viewer builders already frame)
- **Bet 1.3** — production-grade asset pipeline (medium; mostly proof +
  production-profile/default policy)
- **Bet 1.4** — doctor → per-asset validation (medium)

### Prioritized remaining work after the material/WebGL2 pass

Execute these in order. Items at the top close user-visible proof gaps or
release-evidence gaps; larger renderer research lanes follow.

1. - [x] **Public demo material proof**. The demo page renders the
       expanded material-preset scene live in the browser using the host
       device GPU/backend, not a static screenshot. Proof:
       `demo/?sample=material-presets` calls the Rust
       `load_material_presets_scene(...)` path and attaches it to the
       browser canvas; browser screenshot artifact:
       `target/gate-artifacts/demo-material-presets-browser.png`;
       regenerated public showcase media:
       `docs/assets/easy-scene-showcase/material-presets.jpg`. The image shows
       all current presets (`matte`, `plastic`, `metal`, `rough_metal`,
       `chrome`, `brushed_steel`, `clearcoat_plastic`, `satin`,
       `leather`, `clear_glass`, `frosted_glass`, `rubber`).
2. - [x] **M9 browser/capability matrix reconciliation**. The local
       `m9_capability_matrix_artifact_covers_required_lanes` writer now
       folds `target/gate-artifacts/m6-rust-wasm-renderer-probe.json`
       into measured `linux-webgl2-chromium` and
       `linux-webgpu-chromium` rows when browser proof exists, and folds
       `target/gate-artifacts/m9-wasm-size.json` into the wasm lane when
       present. Current proof:
       `target/gate-artifacts/m9-platform/m9-capability-matrix.json`
       reports WebGL2/WebGPU as `measurement_source:
       browser-probe-runtime`; macOS Metal and Windows DX12 remain
       explicit `missing-lane-artifact` rows until those hosts produce
       artifacts.
3. - [x] **WebGL2 oversized source-material texture regression** —
       **[shipped]**. Trust-platform's Firefox/WebGL2 live
       source-material frames were blank because two external YCB PNG
       base-color textures are `4096x4096`, while the WebGL2/downlevel
       wgpu device limit on that lane is `2048`. The Scena 1.4.0
       browser upload path attempts to create `4096x4096`
       `scena.material.base_color` textures, emits uncaptured wgpu
       validation errors, and does not surface a structured render
       failure to the consumer. Scena now clamps oversized browser
       `ImageBitmap` textures to `2048` while preserving aspect ratio
       before WebGL2 upload. Proof:
       `cargo test browser_texture_resize_dimensions --lib` covers
       `4096x4096` and aspect-preserving resize behavior; the host-safe
       browser proof runs with `SCENA_BROWSER_BACKENDS=webgl2
       SCENA_BROWSER_OVERSIZED_TEXTURE=1 npm run browser:m6` and writes
       `target/gate-artifacts/m6-oversized-browser-texture-probe.json`.
       That proof records source texture `2049x2049`, browser texture
       `2048x2048`, one material texture binding, visible pixels, and
       no wgpu validation errors. The default
       `target/gate-artifacts/m6-rust-wasm-renderer-probe.json` remains
       the WebGL2+WebGPU M9 browser matrix input. Downstream proof:
       trust-platform's package gate rejects packaged PNGs above
       `2048`, the YCB packaged textures were capped to `2048x2048`,
       and `node scripts/trust_twin_robot_cell_playwright.mjs` now
       passes with `renderer_origin: "scena_webgl"`,
       `pixel_difference_count: 602108`, non-background ratios around
       `0.28-0.29`, and `evidence_blockers: []`. The proof set includes
       live browser screenshots, deterministic JSON gate artifacts, and
       `target/gate-artifacts/trust-twin-robot-cell-picture-proof.html`
       displaying the browser-rendered PNGs.
4. - [ ] **Transmission + IBL headless GPU capability evidence** —
       **[proof-gap]**. Gate added:
       `m8_headless_gpu_transmission_volume_ibl_capability_when_available`
       records a dedicated headless-GPU transmission/volume-under-IBL
       assertion and writes
       `target/gate-artifacts/gpu-release-gaps/m8_headless_gpu_transmission_volume_ibl_capability_when_available.json`
       as fail-closed by default. This remains open until an approved
       GPU lane runs the gate with
       `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1` and produces a
       `release_evidence: true` artifact. Current local approved attempt:
       the gate failed correctly with `Renderer::headless_gpu unavailable:
       RequestDevice { backend: HeadlessGpu }`, so no release evidence was
       produced on this host.
5. - [ ] **Compressed asset native-GPU/browser release proof** —
       **[proof-gap]**. KTX2/Basis and meshopt are shipped for the
       optional `production-assets` profile with local CPU rendered proof,
       but full native GPU/browser release proof remains open. The native
       GPU lane now fails approved runs unless KTX2 and meshopt both render
       through `Renderer::headless_gpu`; the current host reports
       `RequestDevice { backend: HeadlessGpu }`. Browser WebGL2/WebGPU
       runtime proof now renders `EXT_meshopt_compression` under
       `SCENA_BROWSER_COMPRESSED_ASSETS=1`, but the artifacts stay
       `release_evidence: false` because KTX2/Basis browser decode is still
       fail-closed.
6. - [ ] **GPU/WebGPU/WebGL2 SSAO and OIT parity** — **[deferred]**.
       CPU/headless baselines are shipped; backend parity needs separate
       render passes, capability reporting, and browser visual proof.
7. - [ ] **Physical GPU/WebGPU/WebGL2 transmission/volume glass parity** —
       **[deferred]**. Current glass presets are blend/transmission
       previews with honest no-refraction/no-caustics wording. Full
       physical glass needs backend shading/proof work.
8. - [ ] **KTX2 cubemap environment presets/grid** — **[deferred]**.
       Existing environment presets use checked HDR/fixture paths; KTX2
       cubemap presets need a real decode/upload/prefilter path plus a
       browser/demo grid.
9. - [ ] **MSAA/TAA beyond current FXAA** — **[deferred]**. FXAA is the
       shipped default. MSAA/TAA are future quality lanes with ON/OFF
       proof requirements.
10. - [ ] **Area lights with LTC** — **[deferred]**. Requires lighting
       model, LUT/shader work, and before/after visual proof for
       rect/disc/sphere lights.
11. - [ ] **Clustered/tiled light culling** — **[deferred]**. Useful for
       many-light scaling, but it is GPU architecture/performance work
       rather than a blocking user-proof gap.
12. - [ ] **Screen-space reflections** — **[deferred]**. Needs a robust
       depth/normal-backed backend pass and reflective-floor ON/OFF proof;
       keep it behind stronger browser proof because SSR can fail
       visually while passing compile/unit checks.
13. - [ ] **Draco mesh compression** — **[deferred]**. Meshopt is the
       v1.4 compression path. Revisit Draco only behind an optional
       feature when a maintained decoder path and package-size/build-time
       evidence are proven.

---

## 9. Explicit non-goals

Skip these — they're game-engine / simulation territory and would
dilute scena's renderer-only positioning:

- Physics, collision detection, rigid bodies (Rapier territory).
- Game loop / ECS as a public API (Bevy territory).
- Audio, positional audio.
- Particle systems beyond simple sprites.
- AI navigation, pathfinding, character controllers.
- Networking / multiplayer state sync.
- Geometry-creation asset editor (keep import-only).
- Visual node editor / scripting language for materials.
- Animation **authoring** (import + playback is renderer; authoring is not).

---

## 10. Positioning verdict

For scena's actual positioning — a Rust renderer for trust-platform /
digital-twin applications — Rounds A–D + bets 1.1–1.4 get scena to
**credibly competitive with Three.js for static product viewing**.
That's a defensible "state-of-the-art static product / digital-twin
viewer" claim.

The unqualified **"state-of-the-art 3D library"** claim needs at least
the §3.1 list (genuinely missing) cleared — particularly animation
visual proof, contact shadows, AA, and material coverage — before it
survives someone running the same glTF through `<model-viewer>` or
Three.js side by side. This roadmap is a necessary step, not a
sufficient one.

---

## 11. Reconciliation notes (2026-05-19)

Items removed from the prior draft because they're already in v1.3.0:

- **Named camera views** (`front`/`back`/`left`/`right`/`top`/`bottom`/
  `three_quarter_*`/`azimuth_elevation`) — shipped per
  `docs/release-notes/v1.3.0.md:22` and `docs/api.md:25`.
- **PBR Neutral default tonemapper** — shipped per
  `src/render/output.rs:62` (`#[default]` on `Tonemapper::PbrNeutral`),
  test at `tests/m1_geometry_materials.rs:719`.

Items reframed from "missing" to "implemented but [ergonomic|proof]-gap":

- **Animation playback** — update flow exists at `src/scene/mixers.rs:10-104`.
  `create_animation_mixer` already accepts a clip name; the remaining
  ergonomic gap is a one-call scene/viewer helper that creates, starts,
  and documents update-loop wiring. The proof gap remains no rendered
  regression of a clip.
- **KTX2 / meshopt compression** — feature flags + decode paths exist at
  `Cargo.toml:45`, documented at `docs/feature-flags.md:28`. This was
  first reframed as an optional-profile proof gap; the later compressed
  asset proof pass below closes the local production-profile proof.
- **glTF extension diagnostics** — exist at `src/assets/gltf/extensions.rs`.
  Initially reframed as an ergonomic gap; the later asset-validation and
  fix-hint passes close the user-facing `fix`/validator combination.

Items originally trimmed for honesty, then reopened once the backing
lanes landed:

- **Material presets**: `clear_glass`, `frosted_glass`, `chrome`,
  `brushed_steel`, and `leather` were deferred until the material
  feature lanes could back the names. The 2026-05-21 follow-up reopens
  them with an explicit narrowed contract: chrome/brushed steel rely on
  raised WebGL2 IBL quality and do not claim SSR floor reflections;
  glass presets are blend/transmission previews and do not claim full
  physical refraction.

Item reworded:

- **Auto-framing as default**: no longer "`Camera::default()` computes a
  good view" (impossible — no bounds, no viewport). Now "viewer-level
  default" via existing `InteractiveGltfViewer` / `HeadlessGltfViewer`
  builder framing, future `<scena-viewer>`, and a proposed
  `Scene::add_perspective_camera_default_for(bounds, viewport)` helper.
  Status is therefore an ergonomic/proof gap, not a greenfield gap.

Additional source-truth fixes in this pass:

- **Material variants**: `Scene::set_active_variant(&import, Some(name))`
  already exists. The remaining gap is Viewer / `<scena-viewer>` binding
  and rendered-output proof.
- **Color and transform owners**: color lives in `src/material.rs`; degree
  rotations live in `src/scene/math.rs`. The checklist no longer points at
  nonexistent `src/material/color.rs` or `src/scene/transform.rs` paths.
  The transform alias question is now an explicit API decision, not an
  implied rename.
- **GPU instancing**: `EXT_mesh_gpu_instancing` now has a local narrow
  parser and imports into existing scene-owned `InstanceSet` nodes.
  Procedural/internal instancing remains separate from the glTF extension
  import path.
- **Asset pipeline defaults**: KTX2 / meshopt remain optional features today.
  The roadmap now requires package-size, build-time, and profile/default
  policy evidence before changing defaults.
- **Dependency boundary**: format / IO / OS / encoding / validation work
  should use proven crates, while `scene`, `viewer`, `controls`, and
  `render` keep ownership of scena's public behavior and visual proof.
  Specific accepted choices: local tested Kelvin approximation rather than
  a heavy default dependency, `notify-debouncer-full` for hot reload, `png`
  for screenshot encoding, official Khronos Validator in `xtask doctor`,
  and no Draco critical path until a maintained decoder route is proven.

Visual-proof pass (2026-05-19):

- Added a **Visual proof classes** legend (`none` / `docs-image` /
  `reference-image` / `browser-demo` / `animated-proof`) plus a per-item
  **Visual proof:** field so the "looks correct?" half of the spec is
  declared up front. Rule: docs-images are generated from checked
  example code or a small render harness — never hand-screenshotted —
  with generation metadata so they can be regenerated. Reference images
  may be PNG / PPM, sampled-RGBA TOML, or asset-specific PNGs with
  adjacent metadata. Renderer features that change visual quality need
  ON/OFF, before/after, or order-invariance pairs, not a single image;
  import / compression / capability / performance features keep their
  own structured proof gates and add render proof only where the render
  is part of the contract.

Doctor section: added an **allowlist clause** so escape-hatch teaching
examples stay demonstrable rather than artificially banned. The rules now
target actual residue patterns (`Color::from_linear_rgba`,
`Color::from_srgb`, `Color::from_srgb_u8`, direct camera FOV literals,
quaternion literals), not dead or desired API names.

Wide-gamut output: marked **capability-gated**, not blanket-claimed —
PBR Neutral targets sRGB; Display P3 is a per-backend
`drawingBufferColorSpace` / `GPUCanvasConfiguration.colorSpace`
capability. The first implementation slice should ship reporting and probe
evidence before any visual-output claim.

Round A implementation pass (2026-05-19):

- Landed the "name, not number" slice on branch
  `easy-use-state-art/round-a`: `Color` constants / `from_hex` /
  `from_kelvin`, `PerspectiveCamera` lens presets / `with_fov_degrees`,
  and `Transform::looking_at`.

Viewer callback implementation pass (2026-05-19):

- Landed `InteractiveGltfViewer::on_click` / `on_hover` callback
  registration and `click_at` / `hover_at` helpers on branch
  `easy-use-state-art/round-b`. The closure receives the same typed
  `Result<Option<Hit>, LookupError>` returned by direct picking, so hit,
  miss, and structured errors remain observable. Proof is pinned by
  `tests/round_d_viewer_pointer_callbacks.rs`, generated animated docs
  image `round-d-viewer-pointer-callback-animated-docs-image`, and doctor
  rule `VIEWER-POINTER-CALLBACKS`.

Viewer PNG capture implementation pass (2026-05-19):

- Landed `capture_png_bytes()` and native-only `capture_png(path)` on
  `FirstRender`, `HeadlessGltfViewer`, and `InteractiveGltfViewer`.
  Encoding uses the existing `png` crate directly and returns structured
  `ViewerCaptureError` values for invalid frame buffers, encoding errors,
  or file-write errors. Proof is pinned by
  `tests/round_d_viewer_capture_png.rs`, generated reference image
  `viewer-capture-png-reference.png`, and doctor rule
  `VIEWER-CAPTURE-PNG`.
- Extended the same capture module with the no-GPU one-shot path:
  `HeadlessGltfViewerBuilder::render_png_bytes()` plus native
  `render_png(path)`. The helper uses the CPU headless renderer, returns
  `ViewerPngError`, and is pinned by
  `headless_viewer_builder_renders_gltf_to_png_bytes_without_gpu_setup`
  and `headless_viewer_builder_renders_gltf_to_png_file_without_gpu_setup`
  plus doctor rule `VIEWER-HEADLESS-PNG`.
- Added public reference-image regression primitives:
  `ReferenceImage::from_rgba8`, `ReferenceImageTolerance`, `regress`,
  and `regress_with_tolerance`. The first public surface is deliberately
  RGBA8-frame based so it works with viewer screenshots, readback, and
  user-managed references without owning asset loading or filesystem
  policy. Proof is pinned by `tests/reference_image_regression_api.rs`
  and doctor rule `REFERENCE-IMAGE-REGRESSION`.

Native hot-reload implementation pass (2026-05-19):

- Landed the native `hot-reload` feature with
  `Assets::watch_scene_for_hot_reload`, `AssetHotReloadWatcher`, and
  `AssetHotReloadError`. The watcher uses `notify-debouncer-full` rather
  than raw `notify`, drains debounced changed `AssetPath`s, and leaves
  `reload_scene`, `Scene::replace_import`, `prepare`, and `render` as
  explicit host operations. Proof is pinned by
  `tests/round_d_asset_hot_reload.rs`, generated before/after artifact
  `asset-hot-reload-animated-proof.ppm`, and doctor rule
  `ASSET-HOT-RELOAD`.
- Swept first-path docs, examples, and demo setup away from avoidable
  `PerspectiveCamera::default().with_aspect(...)` and raw color literals.
- Added generated docs-image proof artifacts for the color swatch panel
  and lens-preset comparison under `target/gate-artifacts/examples-visual/`.
- Added `ROUND-A-EASY-USE-PRIMITIVES` doctor coverage so the source,
  tests, visual proof, and first-path API style remain enforced.

State-via-URL implementation pass (2026-05-19):

- Landed `CameraOrbitUrlState` plus helpers on `OrbitControls` and
  `FramingOutcome`. The canonical query uses model-viewer-style concrete
  units for `camera-orbit` / `camera-target`, while the parser also accepts
  the earlier compact checklist form `?camera-orbit=-28,18,2.5`.
- The serializer intentionally emits only camera state. Parsed `src`,
  token, credentialed asset URL, and other application query parameters are
  not preserved. Proof is pinned by `tests/round_d_viewer_url_state.rs` and
  doctor rule `STATE-VIA-URL`.

Khronos sample-loader implementation pass (2026-05-19):

- Landed the feature-gated `khronos-samples` catalog and
  `assets.khronos()` loader in `src/assets/khronos.rs`. The catalog uses the
  already checked fixture set from `tests/assets/gltf/khronos`, carries source
  commit/license/checksum/file-list metadata, and keeps the default feature
  set unchanged.
- Proof is pinned by `tests/round_c_khronos_samples.rs`, including
  all-catalog load coverage, package-size budget, named shortcuts, and the
  generated reference artifact
  `target/gate-artifacts/khronos-samples/rigged-simple-sample-loader-reference.ppm`.
  `DamagedHelmet` and `DragonAttenuation` remain intentionally unclaimed
  until those exact fixtures are vendored or fetched through a checked cache.

Environment preset implementation pass (2026-05-19):

- Landed the public `EnvironmentPreset` catalog and
  `Assets::load_environment_preset` in `src/assets/environment_preset.rs`.
  The current catalog is deliberately limited to checked environment inputs
  the renderer can already load: the neutral studio fixture and Poly Haven
  `studio_small_03` HDR. KTX2 cubemap presets remain unclaimed until the
  environment loader owns that decode path.
- Proof is pinned by `tests/round_c_environment_presets.rs`, including
  metadata/package-budget checks, all-preset load coverage, and generated
  reference artifact
  `target/gate-artifacts/environment-presets/environment-preset-reference-docs-image.ppm`.

Doctor residue-rule closure (2026-05-19):

- Extended `ROUND-A-EASY-USE-PRIMITIVES` so the first-path rule catches
  raw demo color constructors, raw camera FOV setters/literals, and raw
  `Quat::from_*` example literals. `DEMO-CAMERA-VIEWS-NAMED` already
  catches inline `look_from(Vec3::new(...))` and `.orbit(<float>, <float>)`
  in `src/demo_page*`.
- Added `Color::TRANSPARENT` and swept the demo background away from the
  raw `Color::from_linear_rgba(0.0, 0.0, 0.0, 0.0)` residue so the new
  rule can fail closed.

Auto-framing helper implementation pass (2026-05-19):

- Added `Scene::add_perspective_camera_default_for(bounds, viewport)` as
  the explicit scene-level helper promised by §1.2. It inserts a
  `PerspectiveCamera::standard()`, frames it through `frame_bounds`, and
  makes it the active camera without preparing or rendering.
- Added focused integration tests proving the helper creates a centered,
  fill-correct active camera and rejects a zero-sized viewport before
  making a camera active. The remaining §1.2 gap is the
  `<scena-viewer>` browser-demo/reference-image path.

Production asset profile implementation pass (2026-05-19):

- Added the `production-assets` Cargo feature as the named compressed
  glTF profile that enables `ktx2` + `meshopt` while keeping
  `default = []`.
- Updated `docs/feature-flags.md` so asset-heavy users can opt into the
  profile without guessing the decoder feature pair.

Compressed asset proof closure pass (2026-05-19):

- Verified `cargo test --features production-assets --test
  m8_compressed_asset_release_proof`: KTX2 material-role visual rows,
  meshopt visual rows, EXT_mesh_gpu_instancing visual row, and fail-closed
  backend-lane artifacts all passed.
- Reclassified §1.3 KTX2 and meshopt from `[proof-gap]` to `[shipped]`
  for the optional production profile. The claim remains deliberately
  local: native compressed GPU upload and browser-lane release proof are
  not claimed until those lanes have their own artifacts.
- Extended `PRODUCTION-ASSET-PROFILE` doctor coverage so the production
  profile cannot exist without the compressed-asset visual proof suite and
  checklist evidence.

EXT_mesh_gpu_instancing implementation pass (2026-05-19):

- Added a built-in parser for `EXT_mesh_gpu_instancing` node attributes
  (`TRANSLATION`, `ROTATION`, `SCALE`) and import mapping into existing
  scene-owned `InstanceSet` nodes.
- Added focused import proof and a release-artifact visual row for an
  instanced glTF fixture, with doctor coverage pinning the parser,
  importer mapping, and proof tests.

Viewer loading-progress implementation pass (2026-05-19):

- Surfaced `AssetLoadProgress` through headless and interactive viewer
  builders and preserved the emitted events on built viewers / first
  renders for status UIs.
- Added focused headless and interactive viewer tests plus a
  `VIEWER-LOAD-PROGRESS` doctor rule so docs, tests, library re-export,
  and viewer APIs stay aligned.

Viewer material-variant proof pass (2026-05-19):

- Added generated reference/docs-image proof for the shipped viewer
  `KHR_materials_variants` surface:
  `viewer-material-variant-reference-docs-image.ppm` renders the default,
  `midnight`, and `noon` variants from the same glTF fixture and asserts
  the expected red / blue / green color families.
- Extended `VIEWER-MATERIAL-VARIANTS` doctor coverage so the viewer API
  cannot remain marked shipped without the generated visual proof.

Viewer material-variants implementation pass (2026-05-19):

- Surfaced `KHR_materials_variants` names and active-variant switching on
  headless and interactive viewers, with automatic re-prepare after a
  switch.
- Added a real glTF variant fixture, focused viewer tests, and a
  `VIEWER-MATERIAL-VARIANTS` doctor rule. Remaining work is the
  `<scena-viewer>` picker plus rendered reference/docs images.

Camera-control kit implementation pass (2026-05-19):

- Added library-owned `FollowControls` and `FlyControls` alongside the
  existing orbit, turntable, and presentation modes. The new controls stay
  platform-neutral: hosts pass input deltas explicitly and apply the
  resulting camera pose through `Scene`.
- Added focused Follow/Fly scene-application tests and a
  `CAMERA-CONTROL-KIT` doctor rule so the guide, checklist, public
  re-exports, and test proof stay aligned. Remaining proof is the
  browser-demo/animated custom-element interaction surface.

Picking/outline/hover reconciliation pass (2026-05-19):

- Reclassified the signature picking item from `[gap]` to `[shipped]`
  for the library renderer surface after verifying the existing typed
  pick-and-select / pick-and-hover APIs, outline interaction styles, viewer
  callbacks, and generated visual proof.
- Added a `PICKING-OUTLINE-HOVER` doctor rule so this remains a
  source-enforced shipped claim instead of a stale checklist note.

`<scena-viewer>` foundation implementation pass (2026-05-19):

- Added `src/viewer_element.rs` with native-tested attribute parsing and a
  wasm `defineScenaViewer()` export that registers `<scena-viewer>`,
  creates a shadow DOM canvas, and emits structured ready/attribute events.
- Added `SCENA-VIEWER-ELEMENT` doctor coverage for the feature flag,
  public re-exports, browser docs, tests, and checklist evidence. Full
  asset loading/rendering parity remains open under bet 1.1.

`<scena-viewer>` progress UI implementation pass (2026-05-19):

- Added `ScenaViewerProgress` / `ScenaViewerProgressPhase` as the typed
  bridge from `AssetLoadProgress` events to accessible custom-element UI
  labels.
- Extended the custom element with a shadow DOM progressbar,
  `setLoadProgress(detail)`, `scena-viewer-progress` ingestion, and
  `scena-viewer-progress-rendered` notification.
- Reclassified loading progress from ergonomic gap to proof gap: the UI
  surface is shipped and source-enforced; a throttled browser recording is
  still required as visual proof.

`<scena-viewer>` drag/drop ingestion pass (2026-05-19):

- Added `ScenaViewerDropDecision`, `ScenaViewerDropKind`, and
  `ScenaViewerDroppedFile` so supported dropped asset names are validated
  without duplicating string checks in every host.
- Extended the custom element with dragover/drop handling and structured
  `scena-viewer-file-drop` / `scena-viewer-drop-error` events.
- Added M6 render-after-drop proof: the Playwright fixture drops a GLB
  `File`, passes the accepted bytes to `m6RenderDroppedFileProbe`, renders
  the parsed asset into the `<scena-viewer>` canvas, and asserts visible
  pixels under proof class `scena-viewer-drop-render`.
- Added viewer-level auto-framing browser proof **[shipped]** to the same
  path: the proof records `viewer-level-auto-framing` projected-bounds
  metadata and asserts the dropped GLB is inside the viewport, centered,
  and fill-correct without host-side `frame_bounds()` calls.
- Reclassified WASM drag-and-drop from proof gap to shipped for
  ingestion, validation, and render-after-drop browser proof.

`<scena-viewer>` material-variant picker pass (2026-05-19):

- Added `ScenaViewerVariantSelection` / `ScenaViewerVariantOption` as the
  typed picker model for available and active `KHR_materials_variants`
  names.
- Extended the custom element with `setMaterialVariants(...)`,
  `scena-viewer-variants-ready`, and `scena-viewer-variant-change` so hosts
  can bind the picker to the existing viewer/scene variant setter.
- Added M6 picker-to-rendered-variant proof **[shipped]**: selecting
  `noon` in the custom-element picker renders the real
  `material_variants_scene.gltf` fixture through
  `scena-viewer-material-variant-render` and asserts green-dominant pixels
  from the active material.
- Reclassified variant switching from proof gap to shipped for viewer API,
  docs/reference image, custom-element picker events, and rendered selected
  variant proof.

`<scena-viewer>` mobile/a11y defaults pass (2026-05-19):

- Added `ScenaViewerAccessibilityDefaults` and
  `ScenaViewerKeyboardAction` to make role, label, touch-action, minimum
  size, focusability, and keyboard control mapping testable on native
  targets.
- Extended the custom element with default host `tabIndex`,
  `aria-roledescription`, and `scena-viewer-key-control` events for
  keyboard orbit/zoom/reset actions.
- Reclassified mobile-first + a11y from gap to proof gap: defaults are
  shipped and source-enforced; mobile viewport and touch/pinch browser
  proof remains open.

`<scena-viewer>` inspector overlay pass (2026-05-19):

- Added `ScenaViewerInspectorSnapshot` and
  `ScenaViewerInspectorDiagnostic` so renderer debug overlay,
  diagnostics, and render counters can feed a browser overlay through a
  typed native-tested surface.
- Extended the custom element with `setInspectorSnapshot(...)`,
  `setInspectorDiagnostics(...)`, `clearInspectorSnapshot()`, and
  `scena-viewer-inspector-rendered` for host-driven browser proof.
- Reclassified inspector/dev overlay from ergonomic gap to proof gap:
  overlay data plumbing and source enforcement are shipped; live browser
  snapshot proof remains open.

`<scena-viewer>` inspector fixture proof pass (2026-05-19):

- Added `tests/assets/viewer/inspector_snapshot.json` with schema
  `scena.scena_viewer_inspector_snapshot.v1` as the pinned inspector
  overlay fixture for browser proof.
- Updated the M6 custom-element browser probe to fetch the JSON fixture,
  assert the schema in-page, feed it through `setInspectorSnapshot(...)`,
  and verify the rendered overlay details before capturing the screenshot.
- Reclassified inspector/dev overlay from proof gap to shipped for the
  host-fed overlay: the fixture, live overlay, screenshot artifact, and
  doctor source enforcement now close the planned proof.

`<scena-viewer>` annotation overlay pass (2026-05-19):

- Added `ScenaViewerAnnotationAnchor` and
  `ScenaViewerAnnotationError` to parse `data-position`, `data-normal`,
  and `data-surface` from annotation elements with structured errors.
- Extended the custom element with a slotted annotation layer,
  `annotationAnchors()`, `requestAnnotationProjections()`,
  `setAnnotationProjections(...)`, and the
  `scena-viewer-annotations-request` /
  `scena-viewer-annotations-rendered` events.
- Reclassified the annotation overlay from gap to proof gap: the HTML
  surface and host projection contract are shipped and source-enforced;
  animated browser proof remains open.

`<scena-viewer>` annotation tracking proof pass (2026-05-19):

- Extended the M6 custom-element browser proof so one annotation receives
  two projection updates and the probe records an
  `annotation_tracking_sequence` with distinct CSS transforms.
- The proof now asserts `annotation_update_visible` and fails if the
  slotted label stops moving when the host supplies a new screen-space
  projection.
- Reclassified the annotation overlay from proof gap to shipped for the
  host-fed projection contract: parsed anchors, projection request,
  projection update, visible DOM movement, screenshot proof, and doctor
  source enforcement are now all present.

`<scena-viewer>` loading progress sequence proof pass (2026-05-19):

- Extended the M6 custom-element browser proof so it dispatches multiple
  progress updates and records a `progress_sequence` with the DOM phase,
  ARIA value, progressbar transform, and visibility after each update.
- The browser runner now fails if the proof does not cover both the
  indeterminate `loading` state and determinate `fetching` state.
- Reclassified loading progress primitives from proof gap to shipped for
  loader events, viewer event capture, accessible custom-element progress
  UI, browser sequence proof, and doctor source enforcement.

`<scena-viewer>` mobile/a11y gesture proof pass (2026-05-19):

- Added `ScenaViewerGestureAction` and a browser custom-element gesture
  bridge for `orbit`, `pinch-zoom`, and `wheel-zoom` events when
  `camera-controls` is enabled.
- Added `scena.scena_viewer_mobile_a11y_browser_proof.v1`, which runs in
  a mobile-sized Playwright viewport, checks no horizontal overflow,
  asserts touch-action and ARIA defaults, and records touch pinch/orbit
  plus wheel and keyboard reset events.
- Reclassified mobile-first + a11y defaults from proof gap to shipped for
  defaults, keyboard surface, mobile viewport proof, gesture event proof,
  screenshot artifact, and doctor source enforcement.

`<scena-viewer>` browser-proof pass (2026-05-19):

- Extended the M6 Playwright browser probe with
  `scena.scena_viewer_element_browser_proof.v1`, exercising the real
  wasm-exported `defineScenaViewer()` package path.
- The proof covers shadow canvas defaults, ARIA/focus/touch-action
  defaults, progressbar rendering, drag/drop accepted/rejected events,
  material-variant picker events, annotation projection, inspector
  overlay rendering, and keyboard control events.
- The probe writes
  `target/gate-artifacts/scena-viewer-element-browser-proof.png` and is
  source-enforced by `SCENA-VIEWER-ELEMENT` / `VISUAL-BROWSER-M6`.
  Full asset load/render parity and animated gesture proofs remain open
  under the relevant checklist items.

`<scena-viewer>` model-viewer parity proof pass (2026-05-20):

- Added the dev-only `@google/model-viewer` reference package to the M6
  browser proof lane so parity screenshots use the actual custom element
  locally rather than a CDN.
- Added `scena.scena_viewer_model_viewer_parity_proof.v1`, which renders
  `non_ndc_camera_scene.gltf`, `AnimatedMorphCube.gltf`, and
  `WaterBottle.gltf` through renderer-backed `<scena-viewer>` panes next
  to `<model-viewer>` panes for the same assets.
- The proof writes
  `target/gate-artifacts/scena-viewer-model-viewer-parity-browser-proof.png`,
  asserts visible renderer pixels for all three assets, and is now
  source-enforced by `SCENA-VIEWER-ELEMENT`.
- Reclassified bet 1.1 from remaining proof gap to shipped for the current
  drop-in renderer parity proof: custom element foundation, browser UI
  proof, and three-asset side-by-side `<model-viewer>` comparison are all
  present.

Roadmap status closeout pass (2026-05-20):

- Marked the remaining untagged §3.1 research lanes as `[deferred]`
  instead of leaving them as implicit active gaps: clustered/tiled light
  culling, LTC area lights, SSR, and the duplicate Draco row now point to
  future backend or decoder lanes.
- Renamed the §3.2 heading so `[ergonomic-gap]` appears only in legend or
  history text, not as an active section status.

Camera-control browser-proof pass (2026-05-19):

- Added `scena.m6.camera_control_kit_browser_proof.v1` to the M6
  Playwright browser probe, exercising real Rust/WASM `OrbitControls`
  pointer input, bounds-relative zoom limits, `FollowControls`, and
  `FlyControls`.
- The proof writes
  `target/gate-artifacts/camera-control-kit-browser-proof.png` and records
  orbit/follow/fly camera translations so browser proof fails if input no
  longer produces motion.
- Reclassified the camera control kit from ergonomic gap to shipped:
  library APIs, browser input-to-motion proof, and custom-element gesture
  event proof are shipped and source-enforced. Direct camera-motion binding
  inside `<scena-viewer>` remains part of the full renderer parity bet.

Connector magnet preview pass (2026-05-19):

- Added `Scene::preview_connector_magnet` plus
  `ConnectionMagnetPreview` and `ConnectionMagnetVisualCue` so editor
  UIs can draw ghost placement and snap/out-of-range cues without
  mutating the scene.
- The magnet path reuses the existing connector validation and
  `preview_connection` solver, then reports distance against the
  connector-authored snap tolerance and exposes stable CSS cue names.
- Added the M6 `connector-magnet-preview` browser workflow and
  `assertConnectorMagnetPreviewProof`, which render out-of-range and
  snap-ready states, record the measured distance/tolerance sequence, and
  fail if the cue metadata or visible pixels disappear.
- Reclassified connector magnet snapping from proof gap to shipped for
  the library contract plus generated browser proof. Literal
  pointer-driven drag interaction remains scoped to the broader
  `<scena-viewer>` parity bet.

Asset-validation doctor implementation pass (2026-05-19):

- Added `cargo run -p xtask -- asset-doctor <asset.gltf|asset.glb>` as the
  official-validation lane: it shells out to the Khronos glTF Validator in
  stdout mode and fails closed if the executable is missing or does not
  produce parseable JSON.
- Added scena-native renderer guidance with structured `fix` strings for
  required/degraded extensions such as clearcoat, Draco, KTX2, meshopt, and
  WebP extension rebinding, plus `ASSET-VALIDATION-DOCTOR` source/doc/test
  enforcement.

Ease-of-use implementation continuation (2026-05-19):

- Landed named background presets, orbit-control presets, auto-exposure
  scenarios, one-call animation playback, connector axial gaps, and
  bounds-relative orbit zoom as small independent slices.
- Added doctor rules for each shipped slice so source APIs, focused
  tests, docs snippets, and generated visual-proof artifacts remain
  source-enforced instead of checklist-only claims.

Subtle bloom implementation pass (2026-05-19):

- Added `PostBloomConfig`, `Renderer::set_bloom(...)`,
  `Renderer::clear_bloom()`, `RendererStats::bloom_passes`, and supported
  bloom capability reporting for the output-space postprocess baseline.
- Added focused proof that the halo appears outside the source highlight
  without a second tonemap and an ON/OFF visual fixture at
  `target/gate-artifacts/m2-visual/bloom-on-off.ppm`.
- Reclassified subtle bloom from a genuine renderer gap to shipped for
  the threshold / blur / composite baseline; HDR pre-tonemap bloom can
  still replace this later if the renderer grows an HDR postprocess chain.

glTF extension diagnostic fix-hint pass (2026-05-19):

- Added `GltfExtensionDiagnostic::suggested_fix()` so library consumers can
  surface actionable remediation without shelling out to `asset-doctor`.
- Extended the M8 extension diagnostics proof to assert clearcoat fallback
  guidance for required assets, Draco-to-meshopt guidance, then
  source-enforced the library method through `ASSET-VALIDATION-DOCTOR`.
- Reclassified glTF extension diagnostics from an ergonomic gap to shipped
  for typed status, help, decoder policy, suggested fix hints, and official
  validator combination through the asset doctor.

Headless SSAO/contact-shadow baseline pass (2026-05-19):

- Added `ScreenSpaceAmbientOcclusionConfig`, renderer setters/clearers,
  `RendererStats::ambient_occlusion_passes`, and headless/described CPU
  capability reporting for the depth-aware contact-darkening baseline.
- Added a focused depth-contact test and an ON/OFF M2 visual fixture
  `ssao-contact-on-off` so the checklist closes on rendered evidence, not
  on a pretty single render.
- Reclassified contact shadows / SSAO from a genuine renderer gap to
  shipped for the headless CPU baseline while keeping GPU/WebGPU/WebGL2
  SSAO as explicit future backend work.

Anti-aliasing control and ON/OFF proof pass (2026-05-19):

- Added `AntiAliasing` and `Renderer::set_anti_aliasing(...)` so FXAA is
  explicit and can be disabled for exact-pixel captures and visual proof.
- Added a focused ON/OFF test plus the M2 `anti-aliasing-on-off`
  reference artifact to prove edge smoothing against an unfiltered
  baseline.
- Reclassified anti-aliasing from a genuine gap to shipped for the FXAA
  baseline while leaving MSAA/TAA as future renderer-quality lanes.

Weighted blended OIT baseline pass (2026-05-19):

- Added `OrderIndependentTransparencyConfig`,
  `Renderer::set_order_independent_transparency(...)`,
  `Renderer::clear_order_independent_transparency()`,
  `RendererStats::order_independent_transparency_passes`, and
  headless/described CPU capability reporting for the weighted-blended
  transparency baseline.
- Added a focused opposite-insertion-order test plus the M2
  `oit-overlap-order-invariance` reference artifact to prove overlapping
  transparent surfaces resolve independent of scene insertion order.
- Reclassified OIT from a genuine renderer gap to shipped for the
  headless CPU baseline while leaving GPU/WebGPU/WebGL2 OIT as explicit
  future backend work.

Scalar clearcoat material baseline pass (2026-05-19):

- Added `MaterialDesc::with_clearcoat_factor(...)`,
  `MaterialDesc::with_clearcoat_roughness_factor(...)`, and matching
  getters for the scalar `KHR_materials_clearcoat` factors.
- Parsed optional glTF clearcoat scalar factors into `MaterialDesc` and
  added a CPU/reference PBR clearcoat lobe owned by
  `src/render/prepare/pbr_contract.rs`.
- Added parser proof, PBR math proof, and the generated M8 before/after
  visual artifact `m8-clearcoat-material-feature`. Reclassified material
  features from fully missing to shipped for scalar CPU clearcoat only;
  texture slots, GPU/WebGPU/WebGL2 clearcoat, sheen, anisotropy,
  iridescence, and dispersion were still open at that point.

Wide-gamut capability probe pass (2026-05-19):

- Added `Capabilities::wide_gamut_output` and
  `DiagnosticCode::WideGamutOutputUnavailable` so scena reports sRGB as the
  default and only treats Display P3 as possible when an attached browser
  surface has probe evidence.
- Extended the M4 browser smoke to record WebGL2 `drawingBufferColorSpace`
  and WebGPU `GPUCanvasConfiguration.colorSpace` probe results in the
  capability artifact.
- Reclassified wide-gamut output to shipped for capability-gated reporting
  and browser probe evidence only; renderer-level Display P3 presentation
  remains an explicit gap until it has backend visual proof.

Display P3 renderer output pass (2026-05-20):

- Added `OutputColorSpace` and
  `RendererOptions::with_output_color_space(OutputColorSpace::DisplayP3)`
  so Display P3 is requested through the renderer options path instead of
  host-side probe-only code.
- Added renderer-owned browser canvas configuration for WebGL2
  `drawingBufferColorSpace` and WebGPU `GPUCanvasConfiguration.colorSpace`;
  capabilities now switch to `wide_gamut_output = Supported`,
  `OutputStageStatus::PbrNeutralDisplayP3`, and
  `Rgba8UnormSrgb+DisplayP3Canvas` only after the active canvas reports
  effective `display-p3`.
- Extended the M6 browser proof with `scenaM6DisplayP3OutputProbe`.
  `target/gate-artifacts/m6-rust-wasm-renderer-probe.json` records passed
  WebGL2 and WebGPU `display-p3-output` results with
  `RendererOptions::with_output_color_space` as the injection path and
  nonblack rendered pixels.

Animation proof reconciliation pass (2026-05-19):

- Reconciled §3.2 / §3.3 with the already-shipped one-call animation
  playback proof: the focused API test starts a named mixer and the
  generated docs-image proof renders fixed-timestamp frames that visibly
  change.

Clearcoat texture-slot pass (2026-05-19):

- Added `MaterialDesc` clearcoat, clearcoat-roughness, and clearcoat-normal
  texture slots, including `KHR_texture_transform` preservation and
  clearcoat normal scale.
- Parsed optional glTF `KHR_materials_clearcoat` texture slots into
  `MaterialDesc`, retained them through asset GC and scene inspection, and
  included them in material texture resource accounting.
- Added CPU/reference sampling for the clearcoat texture red channel,
  clearcoat roughness texture green channel, and clearcoat normal texture.
  Reclassified material features from scalar-only shipped to CPU/reference
  clearcoat texture shipped while keeping GPU/WebGPU/WebGL2 clearcoat as an
  open backend gap.

GPU clearcoat shader/resource pass (2026-05-20):

- Extended prepared material slots, material batching, GPU texture uploads,
  material bind groups, and material uniforms to carry clearcoat,
  clearcoat-roughness, and clearcoat-normal texture roles.
- Updated the WebGPU/native `texture_2d_array` shader and WebGL2
  `texture_2d` shader variant to sample those roles and add a
  punctual-light clearcoat lobe from `clearcoat_factors`.
- Reclassified GPU/WebGPU/WebGL2 clearcoat from a pure implementation gap to
  shipped shader/resource wiring with a proof gap: focused source tests and
  doctor rules pin the contract, while the headless-GPU release lane remains
  fail-closed until approved backend screenshot or readback proof is run.

Sheen material pass (2026-05-20):

- Added `MaterialDesc` sheen color/roughness factors and sheen color plus
  sheen roughness texture slots, including `KHR_texture_transform`
  preservation.
- Parsed optional glTF `KHR_materials_sheen` factors and texture slots into
  `MaterialDesc`, retained them through asset GC and scene inspection, and
  included them in material texture resource accounting.
- Added CPU/reference sampling for the sheen color texture RGB channels and
  sheen roughness texture alpha channel, with the sheen lobe owned by
  `src/render/prepare/pbr_contract.rs`.
- Extended prepared material slots, material batching, GPU texture uploads,
  material bind groups, material uniforms, and both shader variants to carry
  sheen color and sheen roughness roles.
- Reclassified sheen from a pure implementation gap to shipped
  CPU/reference plus shader/resource wiring with a proof gap: focused tests,
  generated M8 visual proof, and doctor rules pin the contract, while
  approved backend screenshot or readback proof remains required for
  release-grade GPU/WebGPU/WebGL2 parity.

Anisotropy material pass (2026-05-20):

- Added `MaterialDesc` anisotropy strength/rotation factors and anisotropy
  texture slots, including `KHR_texture_transform` preservation.
- Parsed optional glTF `KHR_materials_anisotropy` factors and texture slots
  into `MaterialDesc`, retained them through asset GC and scene inspection,
  and included them in material texture resource accounting.
- Added CPU/reference sampling for the anisotropy texture red/green direction
  channels and blue strength channel, with the anisotropy lobe owned by
  `src/render/prepare/pbr_contract.rs`.
- Extended prepared material slots, material batching, GPU texture uploads,
  material bind groups, material uniforms, and both shader variants to carry
  the anisotropy role.
- Reclassified anisotropy from a pure implementation gap to shipped
  CPU/reference plus shader/resource wiring with a proof gap: focused tests,
  generated M8 visual proof, and doctor rules pin the contract, while
  approved backend screenshot or readback proof remains required for
  release-grade GPU/WebGPU/WebGL2 parity.

Iridescence material pass (2026-05-20):

- Added `MaterialDesc` iridescence factor, IOR, thickness-range factors, and
  iridescence factor/thickness texture slots, including `KHR_texture_transform`
  preservation.
- Parsed optional glTF `KHR_materials_iridescence` factors and texture slots
  into `MaterialDesc`, retained them through asset GC and scene inspection,
  and included them in material texture resource accounting.
- Added CPU/reference sampling for the iridescence texture red channel and
  iridescence thickness texture green channel, with the iridescence lobe owned
  by `src/render/prepare/pbr_contract.rs`.
- Extended prepared material slots, material batching, GPU texture uploads,
  material bind groups, material uniforms, and both shader variants to carry
  the iridescence factor and thickness roles.
- Reclassified iridescence from a pure implementation gap to shipped
  CPU/reference plus shader/resource wiring with a proof gap: focused tests,
  generated M8 visual proof, and doctor rules pin the contract, while
  approved backend screenshot or readback proof remains required for
  release-grade GPU/WebGPU/WebGL2 parity.

Dispersion material pass (2026-05-20):

- Added `MaterialDesc::with_dispersion_factor(...)` and
  `MaterialDesc::dispersion_factor()` for the scalar
  `KHR_materials_dispersion.dispersion` factor.
- Parsed optional glTF `KHR_materials_dispersion` factors into
  `MaterialDesc` while keeping required dispersion assets guarded as degraded
  until approved backend proof exists.
- Added CPU/reference channel-spread specular shading owned by
  `src/render/prepare/pbr_contract/dispersion.rs` and wired the same scalar
  through GPU material uniforms plus both WebGPU/WebGL2 shader variants.
- Added focused parse, CPU pixel, PBR math, shader-source, generated M8 visual
  proof, and doctor rules so dispersion is no longer a pure implementation
  gap; release-grade backend parity remains a proof gap.

Transmission/volume material pass (2026-05-20):

- Added `KHR_materials_transmission`, `KHR_materials_ior`, and
  `KHR_materials_volume` parsing into `MaterialDesc`, including transmission
  and thickness texture slots, transmission factor, IOR, thickness,
  attenuation distance, and attenuation color.
- Added CPU/reference transmission-volume shading owned by
  `src/render/prepare/pbr_contract/transmission.rs` and kept full physical
  GPU/WebGPU/WebGL2 glass parity as a future backend lane rather than a
  claimed v1.4 blocker.
- Added focused parse, CPU pixel, PBR math, generated M8 visual proof,
  M6 browser material-extension composite proof, asset guidance, and doctor
  rules so the material row no longer carries an actionable proof-gap marker.

Viewer animation sugar pass (2026-05-19):

- Added `HeadlessGltfViewer::play_clip(...)` and
  `InteractiveGltfViewer::play_clip(...)` as thin convenience surfaces over
  `Scene::play_animation_by_name` for the viewer's loaded import.
- Kept animation update, prepare, and render explicit by returning the
  scene-owned mixer key; the host still drives `update_animation(...)`.

## Live-demo smooth-metal investigation handoff (2026-05-21)

Open question after a long debugging session on `scena-demo.pages.dev`:
the `material-presets` sample's `metal` sphere reads as light-gray plastic
with one bright spot in iPhone Safari and Windows Chrome, not as
polished chrome. The session reverted off `main` (force-push back to
`15b623b`); these notes record what was learned so the next pass does
not relitigate the same dead ends.

The reverted attempt lived across commits `2a853e7..218881d` on `main`
(now gone — recover from reflog if needed).

### Superseded source of truth check

Current v1.5 status: `MaterialDesc::metal(Color::LIGHT_GRAY)` remains a
generic polished-metal preset (roughness 0.28), not the chrome shortcut.
`MaterialDesc::chrome()` and `MaterialDesc::brushed_steel()` are now
shipped under the narrower §2.6 contract: they are backed by metallic
roughness, anisotropy where appropriate, and the raised WebGL2
environment-prefilter sample floor, but they do not claim SSR floor
reflections. The pre-v1.5 wording below is retained only as debugging
history.

### Five contributing factors found, in order of bite

1. **WebGL2 prefilter quality is heavily downsampled.**
   `src/render/prepare/environment_prefilter.rs::sample_count_for_roughness`
   chooses 4/8/16 GGX importance samples for the `InteractiveWebGl2`
   profile vs 32/96/192/384/768 for `Reference`. At metal roughness
   0.28 (`stepped = 2`), WebGL2 runs 8 samples per direction. Eight
   samples is not enough to resolve a 1k HDR's hot softbox on a
   smooth-metal mip — the integral collapses to ~mean radiance.
   Backend → profile mapping is in
   `src/render/prepare/environment.rs:49-57`:
   `Backend::WebGl2 => Self::InteractiveWebGl2`; everything else
   (including `Backend::WebGpu` and the headless paths) routes to
   `Reference`.

2. **Local lavapipe and headless renders lie about the browser look.**
   `examples/easy_scene_showcase.rs::render_material_sphere` uses
   `Renderer::headless(...)` → `Backend::Headless` → `Reference`
   profile. The local `Renderer::headless_gpu(...)` lavapipe path also
   resolves to `Reference`. The shipped browser demo went through
   `Backend::WebGl2` and got the 8-sample profile. So screenshots
   produced via the showcase example or any `headless_gpu` preview
   tool **will show chrome character even when the production demo
   does not**. Verify any future fix against the actual WebGL2 path
   (real iPhone Safari, or Chrome forced to WebGL2 fallback) before
   declaring it shipped.

3. **The bundled demo HDR is the wrong Polyhaven file.**
   `demo/samples/environment/white_studio_03_1k.hdr` (Polyhaven
   `white_studio_03`) is *not* the file `EnvironmentPreset::Studio`
   names (`tests/assets/environment/polyhaven/studio_small_03_1k.hdr`,
   Polyhaven `studio_small_03`). The two are different studios.
   ImageMagick mean/std on each (range clamped to 0..1 in 16-bit
   sRGB):

   | HDR | min | max | mean | std | std/mean |
   | --- | --- | --- | --- | --- | --- |
   | `white_studio_03_1k.hdr` (demo) | 0.003 | 1.000 | 0.443 | 0.258 | 0.58 |
   | `studio_small_03_1k.hdr` (preset) | 0.000 | 1.000 | 0.183 | 0.256 | 1.40 |

   The bundled demo HDR has no true blacks and ~2.4× the mean radiance;
   relative contrast is roughly half of the polyhaven preset's. With
   the existing prefilter on a smooth metallic sphere, the brighter
   uniformity pre-filters to a near-flat specular cubemap. Side-by-side
   A/B/C/D matrix (different HDR × different exposure setup) isolated
   the HDR file as the only variable that changed the chrome ball's
   reading. Demo loader is `src/demo_page.rs:31` (`DEMO_HDR_ENVIRONMENT`)
   and `attach_to_canvas` at line 220.

4. **WebGPU-first attach broke Windows Chrome.** Tried changing
   `attach_to_canvas` to attempt
   `PlatformSurface::browser_webgpu_canvas_element` first and fall back
   to the existing WebGL2 surface. WebGPU attach succeeded on Windows
   Chrome but the canvas rendered blank — no console errors, no
   pageerror, status read `rendered`, the surface just produced no
   visible pixels. iPhone Safari was unaffected (no WebGPU there).
   This is the path forward if Codex wants Windows/Mac browsers on the
   Reference IBL profile, but it needs a real root-cause pass on the
   `browser_webgpu_canvas_element` surface + first-render lifecycle in
   Chrome before reintroducing the attach.

5. **`add_studio_lighting()` interferes with IBL specular on metallic
   materials.** Three directional lights at 13.5 + 4.5 + 3.5 klx (the
   key + fill + rim composite) flood smooth metallic surfaces with
   diffuse white. Metal's PBR character comes almost entirely from IBL
   specular reflection of the env. The directional flood masks the env
   reflection until the metal sphere just reads as bright. `examples/
   easy_scene_showcase.rs::render_material_sphere` uses IBL only with a
   manual `set_exposure_ev(0.5)` and no `add_studio_lighting`.

### Historical knobs evaluated before the v1.5 material pass

This list records the options considered during the reverted debugging
session. Do not treat it as the current backlog unless the item also
appears in the prioritized remaining-work section above.

1. **Swap the demo HDR** — bundle `studio_small_03_1k.hdr` (the file
   `EnvironmentPreset::Studio` already names) into the demo and point
   `DEMO_HDR_ENVIRONMENT` at it. Update `demo/samples/SOURCES.md` and
   the m8 test in `tests/m8_real_asset_proof.rs` that pins the old
   demo HDR's path / SHA-256. This was tested in the reverted session
   and visibly changed the chrome ball reading under `Reference`
   quality. Effect under `InteractiveWebGl2` quality is less dramatic
   (still gated by item 2).

2. **Raise the WebGL2 prefilter sample count** in
   `src/render/prepare/environment_prefilter.rs`. The 4/8/16 schedule
   was set for first-frame budget on Khronos textured PBR samples
   (whose surfaces have their own normal-map variation); it underserves
   smooth metallic surfaces. Bumping to 16/48/96/128/192 still stays
   well under the `Reference` 32/96/192/384/768 schedule and is plenty
   for a smooth chrome ball. Adds ~500 ms to the first env upload on
   a desktop browser; no per-frame cost. The `interactive_prefilter_
   profile_caps_browser_runtime_work` test pins the old numbers — it
   will need to relax to "stay below the Reference numbers" rather
   than equal specific samples.

3. **Investigate and reintroduce the WebGPU-first surface attach** in
   `src/demo_page.rs::attach_to_canvas`. Goal: Windows / Mac / Linux
   Chrome users get `Backend::WebGpu` → `Reference` quality. Required
   prerequisite: trace why
   `PlatformSurface::browser_webgpu_canvas_element` renders blank on
   the current Chrome stable in scena 1.4.0. Likely candidates: surface
   configure format/usage mismatch, missing pre-multiplied alpha, or
   the WGSL pipeline not picking up the swapchain texture format. The
   reverted attempt is in commits `f5398f1` and `218881d` for a
   reference of where it lived.

4. **Resolved in v1.5: add `MaterialDesc::chrome()` and
   `MaterialDesc::brushed_steel()` presets.** The shipped contract is the
   narrowed §2.6 contract, not the older "wait for SSR" condition:
   chrome/brushed steel rely on metallic roughness, anisotropy where
   appropriate, and raised WebGL2 IBL quality. SSR and reflected floors
   remain separate deferred renderer work.

5. **Bundle a higher-contrast studio HDR** specifically tuned for
   smooth-metal showcase (dark backdrop, hot point lights against
   matte walls). Polyhaven has CC0 options. Would help the chrome
   ball reading even under `InteractiveWebGl2` quality because the
   prefilter's integral picks up actual blacks instead of uniform
   mid-gray.

### What was tried in the reverted session and why each failed

For posterity, so the next pass does not retry these dead ends:

- **Adding `PointLight::softbox` grids** to the synthetic
  material-presets / lens-presets / auto-exposure scenes. Either
  drowned out the IBL further (at 60k candela) or didn't show
  distinct highlights (at 900 candela default).
- **`MaterialDesc::metal(Color::CHARCOAL)`** for the chrome sphere.
  Darkened the base so highlights were more visible but the user
  rejected the dark-iron look — and rightly so per the checklist's
  `metal(LIGHT_GRAY)` doc example.
- **Lowering the metal sphere's roughness to 0.05 via
  `pbr_metallic_roughness` directly.** Made no visible difference at
  the WebGL2 prefilter quality — the source env content the prefilter
  averaged was already smooth, so sharper sampling did not surface
  new detail.
- **Auto-exposure preset sweeps and manual `set_exposure_ev`** from
  `-2.0` to `+1.0`. Exposure scales the whole image uniformly; it
  cannot create the bright/dark contrast that makes chrome read as
  chrome.
- **Removing `add_studio_lighting()` entirely.** Mostly correct (the
  Codex showcase example does the same) but does not by itself make
  smooth metal read as chrome under the WebGL2 prefilter.

### Suggested handoff checklist for Codex

- [x] Decide whether to ship aluminum honestly (label the sphere
      "polished metal" + leave the visual as it is) or extend the
      checklist to allow chrome. Decision: extend the checklist with a
      bounded chrome/brushed-metal contract and raised WebGL2 IBL
      sample floor.
- [x] If extending: pick one path from the knob list above. Run the
      release-skill gate chain (fmt + clippy + test + doctor + cargo
      doc) before pushing — the existing
      `interactive_prefilter_profile_caps_browser_runtime_work` and
      m8 test will block reworks of the prefilter schedule and HDR
      bundle respectively until updated in lockstep.
- [x] Verify against a real `Backend::WebGl2` browser render (iPhone
      Safari or desktop Chrome with WebGPU disabled). Proof:
      `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg
      . --features browser-probe` and
      `node tests/browser/m6_rust_wasm_renderer_probe.js` passed with the
      `pbr-material-presets` workflow and
      `browser-pbr-material-preset-expanded-set` metadata.
