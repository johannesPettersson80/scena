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

Status: **[gap]**
Owner: `src/viewer.rs` for shared viewer behavior + a new thin browser
adapter module / WASM package built directly on `web-sys` /
`wasm-bindgen`. Do not add a Rust web-component framework unless a
concrete missing browser API proves one is necessary. The adapter must
delegate asset loading, framing, and rendering to `viewer` / `assets` /
`scene`; it must not become a second renderer owner.
Proof: WASM browser test rendering against three sample assets +
side-by-side screenshot comparison with `<model-viewer>` on the same assets.
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
assets by default (`ViewerCommonOptions::frame_import = true`); the gap is
the frictionless scene/helper/custom-element surface plus stored
browser-rendered proof.
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

// desired explicit helper (Scene level)
let camera = scene.add_perspective_camera_default_for(bounds, (w, h))?;
```

### 1.3 Production-grade asset pipeline complete and production-profile ready

Status: **[gap]** overall — KTX2 / meshopt are implemented but not
ergonomic/proven, while Draco and `EXT_mesh_gpu_instancing` import support
remain genuinely missing.
Owner: `Cargo.toml` features + `src/assets/texture.rs` +
`src/assets/gltf/extensions.rs` + a new doctor lane.

Default policy to decide before implementation: keep the crate's default
feature set lean unless package size, build time, and binary-size evidence
support changing it. If KTX2 / meshopt stay optional, ship a documented
`production-assets` profile or example command that enables them together.

Sub-items:

- **PBR Neutral tonemapper as default** — Status: **[shipped]**
  (`src/render/output.rs:62`, `#[default]` on `Tonemapper::PbrNeutral`;
  test at `tests/m1_geometry_materials.rs:719`).
- **KTX2 / Basis textures (`KHR_texture_basisu`)** — Status:
  **[ergonomic-gap]**. Feature flag exists at `Cargo.toml:45`, documented
  in `docs/feature-flags.md:28`, decode path at `src/assets/texture.rs`,
  marked `Supported` in extension diagnostics. Not on by default; no
  benchmark proving the GPU memory win; no rendered-output regression
  image of a KTX2-textured asset.
- **meshopt (`EXT_meshopt_compression`)** — Status: **[ergonomic-gap]**.
  Feature flag at `Cargo.toml`, marked `Supported` at
  `src/assets/gltf/extensions.rs`. Not default; no proof artifact.
- **Draco (`KHR_draco_mesh_compression`)** — Status: **[gap]**.
  Not a v1.4 critical-path item. Prefer meshopt for the next release;
  revisit Draco only behind an optional feature when a maintained decoder
  path is proven. `draco_decoder` is still 0.0.x, and `draco-oxide`
  decoder support is not ready.
- **GPU instancing (`EXT_mesh_gpu_instancing`)** — Status: **[gap]**.
  Source-truth pass found no `EXT_mesh_gpu_instancing` import support;
  procedural/internal instancing is separate. Preferred path: file /
  contribute upstream `gltf-rs` support. Contingency: a narrow
  scena-side parser for this extension so v1.4 is not blocked on upstream
  merge timing.

Proof: a doctor lane that loads a KTX2-textured + meshopt-compressed +
instanced glTF, renders it, diffs against a stored reference, and records
package-size / build-time impact for any default-feature change.
Visual proof: reference-image (per format: KTX2-textured render, meshopt-compressed render, instanced render — each diffed against a stored reference)

### 1.4 Doctor → official validation + actionable scena guidance

Status: **[ergonomic-gap]**
Owner: new `xtask` doctor-assets lane under `crates/xtask/src/app/` plus
the existing `src/assets/gltf/extensions.rs` diagnostics infrastructure.
Proof: doctor lane that runs the official Khronos glTF Validator for
spec-compliance validation, then runs scena-native checks for
renderer-specific guidance and produces structured errors with `fix`
strings; rustdoc example for each scena error variant.
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

Constants: `WHITE`, `BLACK`, `GRAY`, `LIGHT_GRAY`, `DARK_GRAY`,
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
- **GPU instancing import** (`EXT_mesh_gpu_instancing`). Procedural/internal
  instancing is separate; this item is the glTF extension import path.
  Upstream `gltf-rs` support is preferred; local narrow parsing is the
  release-timing contingency.
  Proof: extension parse/import assertions for instance count, transforms,
  bounds, and resource sharing, plus a rendered repeated-part fixture.
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
- **KTX2 / Basis textures.** Feature flag at `Cargo.toml:45`, documented
  at `docs/feature-flags.md:28`, decode path at `src/assets/texture.rs`,
  marked `Supported` in extension diagnostics. **Gap**: not in default
  features; no rendered-output proof of a KTX2-textured asset; no
  benchmark vs uncompressed.
- **meshopt compression.** Feature flag in `Cargo.toml`, marked
  `Supported` at `src/assets/gltf/extensions.rs`. **Gap**: not default;
  no proof artifact.
- **glTF extension diagnostics.** `GltfExtensionDiagnostic` exists at
  `src/assets/gltf/extensions.rs`. **Gap**: not surfaced as typed
  user-facing errors with `fix` hints and not yet combined with official
  Khronos glTF Validator output (covered by bet 1.4).

### 3.3 Implemented but not visually/proof complete — [proof-gap]

The pipeline runs but no stored reference asserts the visual is right.
This section IS the reference-image work; closing every item below
produces a stored PNG with a CI diff threshold.

- Animation clip rendered-output regression test.
- KTX2-textured asset rendered-output regression test.
- meshopt-compressed asset rendered-output regression test.
- Transmission + IBL combo capability evidence on the headless GPU lane.
- Per-backend capability matrix evidence (Vulkan / Metal / DX12 / WebGPU
  / WebGL2 fallback).

---

## 4. Tier 2 — ease-of-use ergonomics

### 4.1 Bundled Khronos sample loader

Status: **[gap]**
Owner: `src/assets/khronos.rs` (new) behind `khronos-samples` feature.
Proof: each sample loads and renders without user-supplied local file
paths, with license/checksum metadata and a package-size budget. Prefer
checked download/cache or dev-fixture resolution; do not silently bloat
the default published crate with large binaries.
Visual proof: reference-image + browser-demo (one reference image per sample; the cloudflare demo lists them for users to click through)

```rust
Assets::khronos::water_bottle().await?
Assets::khronos::damaged_helmet().await?
Assets::khronos::dragon_attenuation().await?       // transmissive control
```

### 4.2 `OrbitControls` bounds-relative zoom

Status: **[gap]**
Owner: `src/controls.rs`
Proof: unit test deriving limits from a known AABB and browser interaction
test proving wheel / pinch input cannot zoom inside or outside the bounds.
Visual proof: animated-proof (short interaction recording showing zoom clamped at both extremes)

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

Status: **[ergonomic-gap]** — `InteractiveGltfViewer::pick_at(x, y)`
exists in `src/viewer.rs`. Missing: callback registration.
Owner: `src/viewer.rs`
Proof: browser test that click / hover callbacks receive hit, no-hit, and
stale-scene cases without bypassing the existing picking API.
Visual proof: animated-proof + browser-demo (recording shows click → callback fires; demo page lets users see hover/click feedback live)

```rust
viewer.on_click(|hit| ...);
viewer.on_hover(|hit| ...);
```

### 4.6 Screenshot one-liner

Status: **[ergonomic-gap]** — internal screenshot plumbing exists in
`src/viewer.rs` ("convenience for screenshots and visual-proof
artifacts"); not surfaced cleanly on the public viewer.
Owner: `src/viewer.rs`
Dependency note: use the already-present `png` crate directly for
`capture_png_bytes`; keep `image` only where broader format support is
actually exercised. Add `gif` only behind an optional stretch feature.
Proof: unit/integration test that `capture_png_bytes()` decodes as a PNG
with the same dimensions as `snapshot_rgba8`; file-writing helper tested
behind native-only filesystem support. GIF remains stretch.
Visual proof: reference-image (the captured PNG is itself the proof; CI diffs the captured bytes against a stored reference)

```rust
viewer.capture_png("frame.png")?;
viewer.capture_png_bytes()?;
viewer.capture_gif("turntable.gif", Duration::from_secs(4))?;   // stretch
```

### 4.7 Asset hot-reload during dev

Status: **[gap]**
Owner: `src/assets.rs` behind `hot-reload` feature on native; WASM
mechanism separate. Use `notify-debouncer-full`, not raw `notify`, so a
single editor save becomes one reload decision instead of several raw
filesystem events.
Proof: native integration test reloads a retained asset after file change;
browser/WASM path is a separate explicit contract, not a hidden fetch in
`render()`.
Visual proof: animated-proof (recording shows edit → save → render reflects the change without page reload)

### 4.8 Drag-and-drop in the WASM viewer

Status: **[gap]**
Owner: `<scena-viewer>` (bet 1.1).
Proof: Playwright test drops GLB/glTF files onto the custom element,
renders the result, and surfaces structured validation errors for rejected
files.
Visual proof: animated-proof + browser-demo (recording shows drag-drop ingestion; the cloudflare demo accepts dropped files)

### 4.9 State-via-URL serializer

Status: **[gap]**
Owner: new helper on `FramingOutcome` + `OrbitControls`.
Dependency note: use direct `serde` derives for structured camera/orbit
state and `urlencoding` for percent-encoding. Do not serialize asset URLs
with credentials or other secrets.
Proof: round-trip test for camera/orbit state plus a privacy test proving
serialized URLs do not include credentialed asset URLs or other secrets.
Visual proof: none (URL serialization is text spec)

```rust
?camera-orbit=-28,18,2.5         // round-trip-compatible with model-viewer
```

---

## 5. Ease-of-use signature opportunities

Cross-cutting features that delight users; each corresponds to a
specific competitor primitive.

- **Bundled studio environments as a Rust enum.** Status: **[gap]**.
  Owner: `src/assets/environment_preset.rs`. Ship a small manifest of
  curated KTX2 cubemaps with license/checksum metadata; embed only if
  package-size evidence says it is acceptable, otherwise use checked
  download/cache. Proof: rendered reference per preset plus package-size
  budget.
  Visual proof: reference-image + docs-image + browser-demo (per-environment reference on the same subject; tutorial picker; demo page environment selector)
  ```rust
  scene.set_environment(Environment::Studio)?;
  // variants: Studio, Apartment, City, Sunset, Warehouse, Park, Dawn, Lobby
  ```
- **Camera control kit.** Status: **[gap]**. Owner: `src/controls.rs`.
  Minimum: Orbit, Turntable/Presentation, Follow, Fly. Proof: browser
  interaction test per mode.
  Visual proof: animated-proof + browser-demo (one short recording per mode showing the input → motion mapping)
- **Picking + outline + hover.** Status: **[gap]** overall. Owner:
  `src/picking.rs` + `src/render/`. Picking exists at `src/picking.rs`;
  outline rendering is missing. Proof: browser hit-test plus rendered
  outline reference.
  Visual proof: reference-image + animated-proof (reference image of an outlined selection on a known asset; recording shows hover/click highlight)
- **HTML/CSS annotation overlay anchored to 3D points.** Status:
  **[gap]**. Owner: `<scena-viewer>` (bet 1.1). `data-position` /
  `data-normal` / `data-surface` attribute pattern; the `data-surface`
  trick (label sticks to a deforming surface) is the killer feature.
  Proof: Playwright test showing labels track projected 3D points across
  camera movement.
  Visual proof: browser-demo + animated-proof (labels visible in the demo; recording shows them tracking through camera orbit and animation)
- **Variant switching for `KHR_materials_variants`.** Status:
  **[ergonomic-gap]** — extension diagnostics mark it supported and
  `Scene::set_active_variant(&import, Some(name))` already exists.
  Surface it on Viewer / `<scena-viewer>` and add rendered-output proof
  as the closing evidence. Owner: `src/viewer.rs` + future custom element.
  Visual proof: reference-image + docs-image (one reference per variant on the same asset; tutorial shows the variant picker output)
- **Loading progress primitives.** Status: **[ergonomic-gap]**.
  `AssetLoadProgress` exists in `src/lib.rs`; surface it as a Viewer /
  `<scena-viewer>` primitive. Proof: loader progress test over cache hit,
  external buffer, texture decode, and cancellation paths.
  Visual proof: animated-proof + browser-demo (progress bar advancing on a throttled connection)
- **Mobile-first + a11y defaults.** Status: **[gap]**. Owner:
  `<scena-viewer>` (bet 1.1). Proof: Playwright mobile viewport tests for
  touch/pinch plus keyboard/ARIA smoke checks.
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
- **CPU rasterizer fallback for no-GPU screenshots.** Owner: existing CPU
  path. Status: **[ergonomic-gap]** — the path exists; the public "render
  a glTF to PNG on native/headless or WASI-like hosts with no GPU" surface
  doesn't.
  Visual proof: reference-image (CPU-rendered output of a known asset diffed against a stored reference)
- **Reference-image regression as a public API.** Status: **[ergonomic-gap]**
  — `SCENA_REFERENCE_DIFF` already exists internally; surface as
  `scena::regress(asset, expected)` for end users.
  Visual proof: reference-image (self-referential — this feature *is* reference-image tooling for end users)

---

## 7. Doctor enforcement pattern

For every Tier-1 named primitive that lands, add a doctor rule in the
same shape — but with an **allowlist clause** so escape hatches stay
teachable.

- [ ] Ban inline raw RGB/RGBA constructors such as
      `Color::from_linear_rgba(<lit>, ...)` or
      `Color::from_srgb(<lit>, ...)` / `Color::from_srgb_u8(<lit>, ...)`
      in first-path examples and `src/demo_page*` **except** in the
      dedicated color escape-hatch example. Do not ban
      `Color::from_kelvin`; that is one of the named conveniences this
      roadmap wants.
- [ ] Ban first-path camera FOV literals once lens presets land: direct
      `vertical_fov: Angle::from_degrees(<lit>)`, raw FOV setter calls, or
      equivalent. Do not key the rule to dead API names like
      `with_fov(<float>)`.
- [x] Ban inline `with_damping(<float>)` in `src/demo_page*` if a named
      damping preset would do.
- [ ] Ban inline `Quat::from_*(<float>, ...)` in `examples/` **except**
      in the dedicated transform escape-hatch example.
- [ ] Ban inline `look_from(Vec3::new(<lit>, <lit>, <lit>))` and
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

8. - [ ] `Environment::*` curated KTX2 environment presets (§5)
9. - [ ] `Assets::khronos::*` sample loaders (§4.1)
10. - [x] `AutoExposureConfig` scenario presets (§2.9)
11. - [x] Scene / Viewer one-call animation playback by clip name (§4.4)

### Round D — Tier 2 ergonomics

12. - [x] `ConnectOptions::with_axial_gap` (§4.3)
13. - [ ] `OrbitControls` bounds-relative zoom (§4.2)
14. - [ ] `Viewer::on_click` / `on_hover` callbacks (§4.5)
15. - [ ] `Viewer::capture_png` (§4.6)
16. - [ ] Asset hot-reload (§4.7)
17. - [ ] State-via-URL (§4.9)

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
  `Cargo.toml:45`, documented at `docs/feature-flags.md:28`. Now an
  ergonomic gap (not default) and a proof gap (no rendered reference).
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
- **GPU instancing**: `EXT_mesh_gpu_instancing` import support is not wired;
  marked as a genuine glTF extension gap. Procedural/internal instancing is
  separate. Preferred path is upstream `gltf-rs` support with a local
  narrow parser as the release-timing contingency.
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
- Swept first-path docs, examples, and demo setup away from avoidable
  `PerspectiveCamera::default().with_aspect(...)` and raw color literals.
- Added generated docs-image proof artifacts for the color swatch panel
  and lens-preset comparison under `target/gate-artifacts/examples-visual/`.
- Added `ROUND-A-EASY-USE-PRIMITIVES` doctor coverage so the source,
  tests, visual proof, and first-path API style remain enforced.
