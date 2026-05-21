# Changelog

All notable user-facing changes are recorded here.

## [Unreleased]

### Added

- Expanded the honest `MaterialDesc` preset set with `rough_metal`,
  `chrome`, `brushed_steel`, `clearcoat_plastic`, `satin`, `leather`,
  `clear_glass`, and `frosted_glass`, with docs, unit tests, generated
  visual proof, browser proof metadata, and doctor coverage. Glass presets
  are documented as blend/transmission previews rather than full refraction
  claims.

### Changed

- Raised the interactive WebGL2 environment-prefilter sample schedule so
  smooth-metal presets such as `chrome` and `brushed_steel` no longer use
  the old 4/8/16 sample cap that flattened reflections toward mean radiance.

### Fixed

- Browser texture loading now clamps oversized native `ImageBitmap` textures
  to the WebGL2-safe 2048px max dimension before upload, preventing
  uncaptured WebGL2/wgpu validation errors and blank source-material frames
  for dense glTF assets with 4096px textures.

## [1.4.0] - 2026-05-20

### Added

- Added `Color` named constants (`TRANSPARENT`, `BLACK`, `WHITE`, `GRAY`,
  `LIGHT_GRAY`, `DARK_GRAY`, `CHARCOAL`, `STUDIO_BACKDROP`, `WARM_WHITE`,
  `COOL_WHITE`, `RED`, `GREEN`, `BLUE`, `ORANGE`, `YELLOW`, `CYAN`,
  `MAGENTA`) plus `Color::from_hex` and `Color::from_kelvin` so first-path
  scene code can pick named colors instead of raw RGB literals.
- Added `PerspectiveCamera` lens presets `wide_angle`, `standard`,
  `portrait`, `telephoto`, and the explicit `with_fov_degrees` escape
  hatch.
- Added `Transform::looking_at` for facing a node at a target point.
- Added directional light presets `DirectionalLight::sun`, `key_light`,
  `fill_light`, `rim_light`, and point light presets
  `PointLight::softbox`, `bulb_warm`, `bulb_cool`.
- Added `MaterialDesc` PBR presets `matte`, `plastic`, `metal`, `rubber`
  (the "honest" four; glass/chrome/leather remain deferred until the
  renderer can back the visual claim).
- Added `Background` enum (`Studio`, `DarkStudio`, `NeutralGray`,
  `White`, `Black`, `Sky`, `Transparent`, `Custom(Color)`) and
  `Renderer::set_background`.
- Added `OrbitControls` named damping/auto-rotate presets `cinematic`,
  `snappy`, `presentation`, `turntable(rpm)`, plus
  `zoom_limits_bounds_relative` for framing-relative zoom clamps.
- Added `AutoExposureConfig` scenario presets `product_studio`, `indoor`,
  `outdoor`, `mixed`.
- Added bundled `EnvironmentPreset` catalog (`NeutralStudio`, `Studio`)
  with checked license/source/SHA-256 metadata and
  `Assets::load_environment_preset`.
- Added bundled Khronos sample loader (`KhronosSample`, `KhronosSamples`,
  `KhronosSampleMetadata`, `Assets::khronos`) behind the
  `khronos-samples` feature, with shortcut methods for `water_bottle`,
  `transmission_test`, `rigged_simple`.
- Added one-call scene animation playback `Scene::play_animation_by_name`
  and viewer-level sugar `HeadlessGltfViewer::play_clip` /
  `InteractiveGltfViewer::play_clip`.
- Added connector-mating axial gap helper `ConnectOptions::with_axial_gap`
  for editor-style drag-to-assemble workflows.
- Added viewer pointer callbacks `InteractiveGltfViewer::on_click` /
  `on_hover` / `clear_click_callback` / `clear_hover_callback`, plus
  asset-aware `pick_at`, `click_at`, `hover_at`,
  `pick_and_select_at`, `pick_and_hover_at`.
- Added one-liner screenshot capture via
  `HeadlessGltfViewer::capture_png` / `capture_png_bytes`,
  `InteractiveGltfViewer::capture_png` / `capture_png_bytes`,
  `FirstRender::capture_png` / `capture_png_bytes`, and the
  one-shot `HeadlessGltfViewerBuilder::render_png` /
  `render_png_bytes` pipeline. Structured error types
  `ViewerCaptureError` and `ViewerPngError` accompany the API.
- Added native asset hot reload (`Assets::watch_scene_for_hot_reload`,
  `Assets::reload_scene`, `AssetHotReloadWatcher`, `AssetHotReloadError`)
  behind the `hot-reload` feature, backed by `notify-debouncer-full`.
- Added `CameraOrbitUrlState` for sharing camera/orbit state via URL
  query strings without leaking asset URLs or credentials.
- Added `Scene::add_perspective_camera_default_for(bounds, viewport)` so
  the common load → frame → camera path is one call.
- Added scene-owned animation mixer helpers `Scene::create_animation_mixer`,
  `animation_mixer`, `play_animation`, `pause_animation`, `stop_animation`,
  `seek_animation`, `set_animation_speed`, `set_animation_loop_mode`,
  `update_animation`.
- Added `ReferenceImage::from_rgba8`, `regress`, and
  `regress_with_tolerance` so applications can write reference-image
  regression tests against scena RGBA8 frames without depending on a
  specific asset loader, renderer backend, or file layout.
- Added `FollowControls` and `FlyControls` companion control kits with
  named offset and local-motion APIs.
- Added `<scena-viewer>` custom element foundation
  (`defineScenaViewer()`, shadow-canvas custom element, model-viewer
  attribute parsing) plus a host-wirable event surface:
  `ScenaViewerDropDecision`, `ScenaViewerVariantSelection`,
  `ScenaViewerInspectorSnapshot`, `ScenaViewerProgress`,
  `ScenaViewerProgressPhase`, `ScenaViewerAccessibilityDefaults`,
  `ScenaViewerKeyboardAction`, `ScenaViewerGestureAction`,
  `ScenaViewerAnnotationAnchor`.

- Added browser proof for `<scena-viewer>` custom-element surfaces in the M6
  Playwright probe, covering progress UI, drag/drop events, material variants,
  annotation projection, inspector overlay, keyboard events, and mobile/a11y
  DOM defaults.

- Added three-asset `<scena-viewer>` / `<model-viewer>` side-by-side browser
  parity proof using the dev-only `@google/model-viewer` package and the M6
  Playwright screenshot artifact
  `scena-viewer-model-viewer-parity-browser-proof.png`.

- Added M6 browser proof for the camera-control kit, covering Rust/WASM orbit
  pointer input, follow-camera placement, and fly-camera local movement.

- Added a pinned `<scena-viewer>` inspector overlay JSON fixture to the M6
  browser proof so the live overlay is fed from source-controlled renderer
  diagnostics data before screenshot capture.

- Added annotation tracking assertions to the `<scena-viewer>` browser proof,
  verifying that a slotted annotation moves when the host supplies updated
  screen-space projections.

- Added loading progress sequence assertions to the `<scena-viewer>` browser
  proof, verifying both indeterminate and determinate progress UI updates.

- Added `<scena-viewer>` mobile gesture proof and host events for touch orbit,
  pinch zoom, wheel zoom, and keyboard reset handling.

- Added `<scena-viewer>` drag/drop render-after-drop browser proof: accepted
  GLB `File` bytes are loaded through the browser asset pipeline and rendered
  into the element canvas.
- Added custom-element auto-framing proof metadata for dropped GLB renders:
  the M6 browser proof now asserts projected bounds are inside the viewport,
  centered, and fill-correct under `viewer-level-auto-framing`.
- Added custom-element material-variant render proof: selecting the `noon`
  variant now renders `material_variants_scene.gltf` into the viewer canvas
  under `scena-viewer-material-variant-render`.

- Added subtle postprocess bloom via `PostBloomConfig` and
  `Renderer::set_bloom(...)`, with `RendererStats::bloom_passes`,
  supported capability reporting, and an ON/OFF headless visual proof.

- Added connector magnet preview APIs for editor-style drag-to-assemble UIs:
  `Scene::preview_connector_magnet`, `ConnectionMagnetPreview`, and
  `ConnectionMagnetVisualCue`.

- Added M6 browser proof for connector magnet previews, covering
  out-of-range and snap-ready visual cue metadata plus visible rendered
  pixels.

- Added `GltfExtensionDiagnostic::suggested_fix()` so asset import UIs can
  surface actionable extension remediation alongside status and decoder policy.

- Added a headless CPU screen-space ambient occlusion baseline via
  `ScreenSpaceAmbientOcclusionConfig` and
  `Renderer::set_screen_space_ambient_occlusion(...)`, with ON/OFF visual
  proof for depth-contact darkening.

- Added `AntiAliasing` and `Renderer::set_anti_aliasing(...)` so FXAA remains
  the default but can be disabled for exact-pixel or ON/OFF visual proof.

- Added a headless CPU weighted blended order-independent transparency
  baseline via `OrderIndependentTransparencyConfig` and
  `Renderer::set_order_independent_transparency(...)`, with order-invariance
  visual proof for overlapping alpha-blended surfaces.

- Added clearcoat material support across the CPU/reference path and GPU
  shader/material resource path: `MaterialDesc` now exposes clearcoat
  factor/roughness builders plus clearcoat, clearcoat-roughness, and
  clearcoat-normal texture slots. Optional glTF `KHR_materials_clearcoat`
  factors and texture slots are parsed, the CPU preview samples clearcoat,
  roughness, and clearcoat-normal texture channels, and the WebGPU/WebGL2
  shader variants sample the same roles for a punctual-light clearcoat lobe.
  M8 proof records a CPU before/after clearcoat render and a fail-closed
  headless-GPU lane until approved backend screenshots exist.

- Added sheen material support across the CPU/reference path and GPU
  shader/material resource path: `MaterialDesc` exposes sheen color and
  roughness factors plus sheen color/roughness texture slots, optional glTF
  `KHR_materials_sheen` factors and textures are parsed, CPU preview samples
  the RGB and alpha texture channels, and WebGPU/WebGL2 shader variants carry
  the same roles through material uniforms and bind groups.

- Added anisotropy material support across the CPU/reference path and GPU
  shader/material resource path: `MaterialDesc` exposes anisotropy strength,
  rotation, and texture slots, optional glTF `KHR_materials_anisotropy`
  factors and textures are parsed, CPU preview samples the texture direction
  and strength channels, and WebGPU/WebGL2 shader variants carry the same
  role through material uniforms and bind groups.

- Added iridescence material support across the CPU/reference path and GPU
  shader/material resource path: `MaterialDesc` exposes iridescence factor,
  IOR, thickness range, and factor/thickness texture slots, optional glTF
  `KHR_materials_iridescence` factors and textures are parsed, CPU preview
  samples the factor red channel and thickness green channel, and
  WebGPU/WebGL2 shader variants carry the same roles through material
  uniforms and bind groups.

- Added dispersion material support across the CPU/reference path and GPU
  shader/material resource path: `MaterialDesc` exposes a non-negative
  dispersion factor, optional glTF `KHR_materials_dispersion` factors are
  parsed, CPU preview applies channel-spread specular shading, and
  WebGPU/WebGL2 shader variants carry the same scalar through material
  uniforms. Required dispersion remains release-proof guarded until approved
  backend evidence exists.

- Added transmission, IOR, and volume material support on the CPU/reference
  path: `MaterialDesc` exposes transmission and thickness texture slots,
  scalar transmission, IOR, thickness, attenuation distance, and attenuation
  color, optional glTF `KHR_materials_transmission`, `KHR_materials_ior`, and
  `KHR_materials_volume` values are parsed, and CPU preview samples
  transmission and thickness textures. M8 proof records CPU before/after
  transmission-volume artifacts, while full physical GPU/WebGPU/WebGL2 glass
  parity remains a future backend lane.

- Added capability-gated wide-gamut output reporting: capability reports now
  expose `wide_gamut_output`, browser M4 smoke artifacts record Display P3
  canvas color-space probes, and diagnostics keep output treated as sRGB until
  a backend-specific probe proves otherwise.

- Added renderer-owned Display P3 browser output configuration via
  `OutputColorSpace` and
  `RendererOptions::with_output_color_space(OutputColorSpace::DisplayP3)`,
  with M6 WebGL2/WebGPU browser proof that records effective `display-p3`
  canvas presentation and `wide_gamut_output = Supported`.

- Added viewer-level animation playback sugar:
  `HeadlessGltfViewer::play_clip(...)` and
  `InteractiveGltfViewer::play_clip(...)` start a named clip on the loaded
  import while keeping animation update, prepare, and render explicit.

### Changed

- Updated the browser demo showcase so connector mating is the default first
  render with a synced Rust code panel, visible replay action, collapsed
  diagnostics, and a README connector-snap hero GIF.
- Added easy scene setup APIs and docs for projection-based camera framing,
  matte grid floors, studio lighting, renderer-managed auto exposure, projected
  labels, and connector replay framing.

### Fixed

- Browser HDR/IBL diffuse lighting now uses prepared diffuse irradiance instead
  of sampling raw HDR radiance in the surface-normal direction, avoiding dark or
  washed-out connector renders while preserving the specular path.
- Public demo timing logs are quiet by default and remain available through
  `?perf=1` or `?timing=1`.

## [1.2.0] - 2026-05-17

### Added

- Added `AssetLoadOptions` with `with_strict_textures(true)` plus
  `Assets::load_scene_with_options` and `Assets::load_scene_with_report_options`
  so browser hosts can promote missing external glTF image fetches from warnings
  to hard load errors.
- Added `DiagnosticCode::MaterialTextureMissingDecodedPixels` and
  `RendererStats::material_textures_missing_decoded_pixels` so descriptor-only
  material textures are visible during `prepare_with_assets`.
- Added browser WebGL2/WebGPU visual coverage for manual `SceneAsset` source
  material reuse on the dense Khronos WaterBottle glTF with external relative
  PBR textures.

### Fixed

- Depth prepass eligibility now ignores ineligible helper/stroke primitives
  instead of disabling the prepass for the whole scene.
- WebGL2 and WebGPU color/depth passes now use the same
  `clip_from_world * world_position` path, avoiding precision disagreement in
  dense browser scenes.
- Browser asset loading now emits console warnings when optional external
  textures cannot be fetched and the caller did not request strict texture
  loading.

## [1.1.0] - 2026-05-16

### Changed

- WebGL2 now renders through the shared wgpu/naga path instead of the deleted
  hand-written raw WebGL2 renderer. The public `Backend::WebGl2` API remains
  intact.
- WebGL2 material sampling uses a small wgpu shader/layout shim with ordinary
  `texture_2d` bindings because wgpu 29's GL backend rendered material
  `texture_2d_array` samples black in Chromium WebGL2.

### Fixed

- Repeated WebGL2 `Renderer::prepare()` no longer retains the old raw GL
  buffer/texture/program cache, closing the GL out-of-memory and subsequent
  wasm-bindgen mutable-guard poisoning failure family.

### Removed

- Removed the hand-written WebGL2 renderer modules and raw `web_sys`
  render-path bindings.

## [1.0.2] - 2026-05-15

### Fixed

- WebGL2 program link failure on Firefox: the output shader no longer redeclares fragment-only uniforms (`camera_position_exposure`, `color_management`, `base_color_uv_offset_scale`, `base_color_uv_rotation`) in the vertex stage with implicit `highp` precision that conflicts with the fragment stage's `precision mediump float;` directive. Firefox WebGL2 reported `Uniform \`<name>\` is not linkable between attached shaders`; Chromium did not enforce the rule. See `docs/decisions/ADR-0001-webgl2-camera-uniform-precision-mismatch.md`.

### Added

- Unit-level regression test `webgl2_shaders_have_no_cross_stage_uniform_precision_mismatch` in `src/render/gpu/materials.rs::tests` that statically parses both WebGL2 shaders (read via `include_str!` of `webgl2_program.rs`) and fails if any uniform name is declared in both stages with an unresolvable precision mismatch. Catches the bug class in every native `cargo test` run without requiring a browser.

## [1.0.1] - 2026-05-14

### Changed

- Reworked the repository documentation into a user-facing documentation set with guides for getting started, API concepts, assets, rendering, browser use, headless rendering, capabilities, lifecycle, errors, feature flags, and troubleshooting.
- Updated release tooling so documentation gates validate the public documentation surface shipped to users.
- Moved benchmark baseline data out of the public documentation tree and into test fixtures.

## [1.0.0] - 2026-05-14

### Added

- Published the first stable `scena` release.
- Added Rust-native scene graph, asset loading, renderer lifecycle, diagnostics, headless rendering, native platform lanes, browser WebGPU/WebGL2 paths, and public examples.
- Added glTF/GLB workflows for model-viewer, CAD-style inspection, industrial visualization, and digital-twin UI use cases.

### Documentation

- Published README, install instructions, examples, platform notes, and release notes for the stable API.
