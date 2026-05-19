# scena post-1.3.0 — Easy-use + state-of-the-art roadmap

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

scena's signature is **easy to use**. This document is the gap inventory
between "Rust renderer that works" and "easier than Three.js, more
accurate than `<model-viewer>`." It is a planning document, not a
contract; items become contracts as they're picked up, each with its own
narrow implementation checklist (the way
`easy-scene-setup-and-auto-framing.md` was structured for v1.3.0).

## Status legend

Every item carries one tag:

- **[gap]** — genuinely missing; nothing to build on.
- **[ergonomic-gap]** — implementation exists but the user surface is
  raw / opt-in / requires plumbing.
- **[proof-gap]** — implementation exists but lacks rendered-output
  proof, doctor rule, or capability evidence.
- **[shipped]** — already in v1.3.0; listed only for context.

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
  that runs the code and shows the result. Used for integration, mobile
  layout, controls, and WASM/WebGPU/WebGL behavior.
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

Status: **[gap]** for full drop-in renderer parity; custom-element
foundation **[shipped]**.
Owner: `src/viewer.rs` for shared viewer behavior + a new thin browser
adapter module / WASM package built directly on `web-sys` /
`wasm-bindgen`. Do not add a Rust web-component framework unless a
concrete missing browser API proves one is necessary. The adapter must
delegate asset loading, framing, and rendering to `viewer` / `assets` /
`scene`; it must not become a second renderer owner.
Proof: foundation is `src/viewer_element.rs` with
`defineScenaViewer()`, a shadow-canvas custom element, model-viewer-style
attribute parsing, docs, and doctor rule `SCENA-VIEWER-ELEMENT`.
Remaining proof for full parity: WASM browser test rendering against
three sample assets + side-by-side screenshot comparison with
`<model-viewer>` on the same assets.
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

Status: **[ergonomic-gap]** — viewer builders already frame imported
assets by default (`ViewerCommonOptions::frame_import = true`), and the
scene-level `Scene::add_perspective_camera_default_for(bounds, viewport)`
helper now exists. The remaining gap is the custom-element/browser-demo
surface plus stored browser-rendered proof.
Owner: `src/viewer.rs` (`InteractiveGltfViewer`, `HeadlessGltfViewer`)
and the future `<scena-viewer>`. Not on `Camera::default()` — that
has no bounds or viewport.
Proof: viewer-level integration test asserting "load → render" produces
a centered, fill-correct frame without any `frame_bounds()` call in user
code, plus a browser screenshot artifact for the custom element path.
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
- **Draco (`KHR_draco_mesh_compression`)** — Status: **[gap]**.
  Not a v1.4 critical-path item. Prefer meshopt for the next release;
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
under `target/gate-artifacts/m8-compressed-assets`. The native GPU and
browser compressed-asset lanes write fail-closed unavailable artifacts
instead of pretending local unit tests are release proof. Doctor rules
`PRODUCTION-ASSET-PROFILE` and `ASSETS-M8` pin the profile and proof
suite.
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
fix-string guidance.
Visual proof: none (structured text errors; no rendered output)

```rust
AssetError::UnsupportedTextureFormat {
    path: "albedo.webp".into(),
    help: "Re-export with PNG/JPEG or use KTX2 through the ktx2 feature",
}
```

The official validator owns glTF spec compliance; scena owns actionable
renderer guidance such as "this asset uses clearcoat, but this pipeline
will render the matte fallback until clearcoat support lands." Do not
reimplement a private subset of the glTF Validator against the `gltf`
AST unless the official validator cannot run in CI or `xtask`.
`GltfExtensionDiagnostic` already returns `extension`, `status`, `help`,
`decoder_policy` (`src/assets/gltf/extensions.rs:24-45`). The ergonomic
gap is surfacing this as user-facing typed errors with `fix` hints on
every `Assets::load_*` path, not just internal diagnostics.

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

### 2.6 `MaterialDesc` PBR presets — honest set only

Status: **[shipped]** — implemented on branch
`easy-use-state-art/round-b`.
Owner: `src/material/presets.rs` extending `MaterialDesc`
Proof: `tests/round_b_material_presets.rs` asserts each preset's
PBR material kind, base color, metallic factor, and roughness factor;
rustdoc examples cover all four constructors; doctor rule
`HONEST-MATERIAL-PRESETS` keeps overpromising glass/chrome/leather names
out until the renderer can back them.
Visual proof: reference-image + docs-image
`target/gate-artifacts/examples-visual/round-b-material-preset-reference-docs-image.ppm`
renders the four presets side-by-side on the same subject.

```rust
MaterialDesc::matte(Color)
MaterialDesc::plastic(Color)
MaterialDesc::metal(Color)
MaterialDesc::rubber()
```

**Deferred until the renderer can back the visual claim**:
`brushed_steel`, `chrome` (need sharp environment reflections + SSR for
floor reflection), `clear_glass` / `frosted_glass` (need transmission +
IOR + OIT), `leather` (needs sheen). These names overpromise without
the underlying material features in §3.1.

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

### 3.1 Genuinely missing — [gap]

Proof rule for this bucket: visual renderer features need
**reference-image with ON/OFF, before/after, or order-invariance pairs**.
A single "pretty render" only proves that something rendered — it does
not prove the feature is doing anything. Pipeline / compression /
capability / performance items need structural or measured proof first
(decode/import assertions, capability artifacts, package-size/build-time
data, allocation/performance gates), plus a rendered reference only when
the rendered result is part of the contract.

- **Anti-aliasing.** MSAA at minimum; TAA preferred. Owner: `src/render/`.
  Proof: rendered-output diff against a non-AA reference showing edge
  quality.
  Visual proof: reference-image ON/OFF.
- **Contact shadows / SSAO.** Single biggest "pro vs amateur" tell beyond
  framing. Owner: `src/render/`. Proof: reference image of the grid floor
  + model with and without contact shadows.
  Visual proof: reference-image ON/OFF.
- **Subtle bloom in post.** One low-threshold pass; the difference between
  "rendered" and "photographed."
  Visual proof: reference-image ON/OFF at a fixed exposure.
- **Material features**: clearcoat, sheen, anisotropy, iridescence,
  dispersion on top of the existing metal-rough + transmission. Owner:
  `src/render/prepare/material_batch.rs` + shaders.
  Visual proof: reference-image before/after per feature using Khronos
  sample controls where available.
- **Clustered / tiled light culling.** Babylon 9 made this baseline.
  Proof: many-light stress scene proves correct light selection,
  stable frame time / allocation behavior, and no dropped-light fallback.
  Visual proof: reference-image of the stress scene; not an ON/OFF gate.
- **Area lights with LTC** (rect/disc/sphere).
  Visual proof: reference-image before/after per light shape.
- **Screen-space reflections (SSR).**
  Visual proof: reference-image ON/OFF on a reflective-floor control.
- **Order-independent transparency (OIT).** Weighted-blended is the cheap
  baseline.
  Visual proof: reference-image order-invariance pair for overlapping
  transparent surfaces.
- **Wide-gamut output (Display P3)** — capability-gated. PBR Neutral
  targets sRGB; Display P3 is a `drawingBufferColorSpace` capability on
  WebGL/WebGPU. Needs measured proof per backend, not a blanket claim.
  Proof: capability-matrix artifact per backend plus a color-space probe;
  no blanket visual claim for unavailable backends.
- **Draco mesh compression** (`KHR_draco_mesh_compression`).
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

### 3.2 Implemented but not ergonomic — [ergonomic-gap]

These exist in the source but the user surface or default story isn't there.

Visual proof for this bucket: **reference-image of a known asset rendered
through each feature path** (KTX2-textured asset render, meshopt
asset render, animation clip at fixed timestamps). For animation
specifically, add **animated-proof** of the clip playing back.

- **Animation update flow.** `src/scene/mixers.rs:10-104` has
  `create_animation_mixer`, `play_animation`, `pause`, `stop`, `seek`,
  `set_speed`, `set_loop_mode`, `update_animation`. `create_animation_mixer`
  already takes a clip name. **Scene helper shipped**:
  `Scene::play_animation_by_name` creates the mixer, starts it, and returns
  the typed mixer handle. Viewer sugar remains deferred. Proof:
  rendered-output proof of a known animation clip playing back at fixed
  timestamps.
- **KTX2 / Basis textures.** Status: **[shipped]** for the optional
  production profile. The decode path and feature flag remain opt-in, and
  feature-gated rendered proof covers material texture roles. Native GPU
  upload and browser release lanes remain future proof work.
- **meshopt compression.** Status: **[shipped]** for the optional
  production profile. The feature-gated proof suite renders decoded
  compressed fixtures for the supported bufferView modes and metadata
  paths. Native GPU and browser release lanes remain future proof work.
- **glTF extension diagnostics.** `GltfExtensionDiagnostic` exists at
  `src/assets/gltf/extensions.rs`. **Gap**: not surfaced as typed
  user-facing errors with `fix` hints and not yet combined with official
  Khronos glTF Validator output (covered by bet 1.4).

### 3.3 Implemented but not visually/proof complete — [proof-gap]

The pipeline runs but no stored reference asserts the visual is right.
This section IS the reference-image work; closing every item below
produces a stored PNG with a CI diff threshold.

- Animation clip rendered-output regression test.
- KTX2-textured asset rendered-output regression test — **[shipped]**
  locally through `tests/m8_compressed_asset_release_proof.rs` with
  `--features production-assets`.
- meshopt-compressed asset rendered-output regression test — **[shipped]**
  locally through `tests/m8_compressed_asset_release_proof.rs` with
  `--features production-assets`.
- Transmission + IBL combo capability evidence on the headless GPU lane.
- Per-backend capability matrix evidence (Vulkan / Metal / DX12 / WebGPU
  / WebGL2 fallback).

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
surface; viewer sugar remains deferred until a concrete viewer workflow
needs it.
Owner: `src/scene/mixers.rs`
Proof: `tests/round_c_animation_playback.rs` proves the helper creates
and starts a mixer and returns the typed handle for update/loop/speed
control; doctor rule `ONE-CALL-ANIMATION-PLAYBACK` keeps the API,
example, docs, and rendered proof present.
Visual proof: reference-image + animated-proof
`target/gate-artifacts/examples-visual/round-c-animation-playback-reference-animated-docs-image.ppm`
and its generated frame sequence render a visible child of an imported
animated node at fixed timestamps.

```rust
// preferred primary surface: Scene owns mixer creation and playback state
let idle = scene.play_animation_by_name(&import, "idle")?;

scene.set_animation_loop_mode(idle, AnimationLoopMode::Once)?;
```

Implementation decision: `Scene::play_animation_by_name` is the shipped
primary API because the scene already owns mixer handles. `Viewer::play_clip`
is deferred; add it only as a thin convenience if a viewer-specific
workflow needs to expose animations directly.

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

Status: **[proof-gap]** — `<scena-viewer>` now owns the browser
drag/drop ingestion surface and validates dropped `.glb` / `.gltf`
filenames through `ScenaViewerDropDecision`; full render-after-drop proof
is still missing.
Owner: `<scena-viewer>` (bet 1.1).
Proof: native drop-decision test in `tests/scena_viewer_element.rs`;
custom element dispatches `scena-viewer-file-drop` for accepted `File`
objects and `scena-viewer-drop-error` for rejected drops. Remaining proof:
Playwright test drops GLB/glTF files onto the custom element, renders the
result, and surfaces structured validation errors for rejected files.
Visual proof: animated-proof + browser-demo (recording shows drag-drop ingestion; the cloudflare demo accepts dropped files)

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
- **Camera control kit.** Status: **[ergonomic-gap]** overall;
  Orbit/Turntable/Presentation plus Follow/Fly library primitives
  **[shipped]**, browser interaction proofs remain future custom-element
  work. Owner: `src/controls.rs`. `OrbitControls` covers orbit,
  turntable, and presentation movement; `FollowControls` tracks a scene
  node from a named offset; `FlyControls` exposes host-driven local
  movement and look deltas without platform coupling.
  Proof: `tests/camera_control_kit.rs` covers Follow/Fly scene application
  and the `CAMERA-CONTROL-KIT` doctor rule pins the public API, guide,
  checklist, and test contract.
  Visual proof: animated-proof + browser-demo still required for the
  future `<scena-viewer>` interaction surface (one short recording per
  mode showing the input → motion mapping)
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
  **[gap]**. Owner: `<scena-viewer>` (bet 1.1). `data-position` /
  `data-normal` / `data-surface` attribute pattern; the `data-surface`
  trick (label sticks to a deforming surface) is the killer feature.
  Proof: Playwright test showing labels track projected 3D points across
  camera movement.
  Visual proof: browser-demo + animated-proof (labels visible in the demo; recording shows them tracking through camera orbit and animation)
- **Variant switching for `KHR_materials_variants`.** Status:
  **[proof-gap]** overall; Viewer primitive, reference/docs-image proof,
  and `<scena-viewer>` picker surface **[shipped]**.
  Extension diagnostics mark it supported and
  `Scene::set_active_variant(&import, Some(name))` exists. Viewers now
  expose `material_variants()`, `active_material_variant()`, and
  `set_active_material_variant(name)`; the setter delegates to the scene
  API and re-prepares before the next render. `<scena-viewer>` exposes
  `ScenaViewerVariantSelection`, `setMaterialVariants(...)`, and
  `scena-viewer-variant-change` for a host-owned picker-to-renderer
  binding. Owner: `src/viewer.rs` + `src/viewer_element.rs`.
  Visual proof: reference-image + docs-image shipped through
  `target/gate-artifacts/examples-visual/viewer-material-variant-reference-docs-image.ppm`;
  browser-demo remains future custom-element proof.
- **Loading progress primitives.** Status: **[proof-gap]** overall;
  Viewer primitive and `<scena-viewer>` progress UI surface **[shipped]**.
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
  doctor rule `SCENA-VIEWER-ELEMENT` pins the browser UI surface.
  Visual proof: animated-proof + browser-demo still required for a
  throttled-connection custom-element recording.
- **Mobile-first + a11y defaults.** Status: **[proof-gap]** —
  `<scena-viewer>` now ships explicit mobile/ARIA/keyboard defaults.
  Owner: `<scena-viewer>` (bet 1.1). `ScenaViewerAccessibilityDefaults`
  and `ScenaViewerKeyboardAction` define the source contract; the element
  sets host role/label/tabindex defaults, keeps the canvas touch-safe, and
  emits `scena-viewer-key-control` for keyboard orbit/zoom/reset events.
  Remaining proof: Playwright mobile viewport tests for touch/pinch plus
  keyboard/ARIA smoke checks.
  Visual proof: browser-demo + animated-proof (mobile-viewport demo capture; touch-pinch recording)
- **Inspector / dev overlay.** Status: **[ergonomic-gap]**. Owner:
  `crates/xtask/` doctor integration + an in-viewer overlay. Doctor is
  already half of this. Proof: browser overlay snapshot plus doctor JSON
  fixture feeding the overlay.
  Visual proof: browser-demo + reference-image (live overlay in the demo; reference snapshot of the overlay state for CI diff)

---

## 6. Differentiators scena could uniquely own

These are not blanket "no competitor has this" claims. They are places
scena could own a distinct Rust / digital-twin workflow if implemented
with proof and a clean public surface.

- **Connector "magnet" snapping with visual cues.** Status: **[gap]**.
  Owner: builds on `src/scene/connectors/`. Triggered when an interactive
  drag-to-assemble workflow has a concrete consumer; not needed for
  read-only viewing.
  Visual proof: animated-proof + browser-demo (recording shows ghost + green outline as a part approaches a valid mate within tolerance)
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
5. - [x] `MaterialDesc` honest PBR presets (§2.6 — matte/plastic/metal/rubber only)
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
  Now an ergonomic gap (not surfaced as user-facing typed errors with
  `fix` hints).

Items trimmed for honesty:

- **Material presets**: `clear_glass`, `frosted_glass`, `chrome`,
  `brushed_steel`, `leather` deferred until clearcoat / sheen /
  anisotropy / OIT / SSR land — otherwise the names overpromise the
  renderer.

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
`drawingBufferColorSpace` capability. Needs measured proof per backend.

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
- Reclassified WASM drag-and-drop from gap to proof gap: ingestion and
  validation are shipped and source-enforced; render-after-drop browser
  proof remains open.

`<scena-viewer>` material-variant picker pass (2026-05-19):

- Added `ScenaViewerVariantSelection` / `ScenaViewerVariantOption` as the
  typed picker model for available and active `KHR_materials_variants`
  names.
- Extended the custom element with `setMaterialVariants(...)`,
  `scena-viewer-variants-ready`, and `scena-viewer-variant-change` so hosts
  can bind the picker to the existing viewer/scene variant setter.
- Reclassified variant switching from ergonomic gap to proof gap: the
  picker surface is shipped and source-enforced; browser-demo proof remains
  open.

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
