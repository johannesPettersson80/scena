# Changelog

All notable user-facing changes are recorded here.

## [Unreleased]

### Added

- Added the `scena.visual_patch.v1` SceneHost patch contract for batched
  transform, tint, visibility, camera, eased transition, animation-time,
  selection/hover, material variant, label-anchor, and metadata updates, with
  native and WASM entrypoints plus stable fixtures.
- Added the `scena.host_event.v1` SceneHost event contract for pick, hover,
  selection, load, diagnostic, capture, surface, context, device, and
  capability events, with native drain/sink APIs, sink-only push delivery,
  WASM `drainEventsJson()`, and a stable fixture.
- Added shared descriptor-bound PNG capture helpers for `CaptureRgba8`,
  `Renderer`, SceneHost, viewer helpers, and WASM `capturePng()`, plus capture
  contact-sheet and baseline comparison helpers for proof artifacts.
- Added additive `scena.asset_load_report.v1` fields for external-resource
  status rows and material fallback provenance, plus external-image fetch
  progress events for browser/agent diagnostics.
- Added `scena.render_introspection.v1`, a capture-bound inspection report for
  agent-readable frame visibility, luminance, framing, reason, and fix
  summaries behind the `inspection` feature. Content detection is
  background-relative, warning-only framing reasons keep `ok=true`, and
  luminance reports shader-encoded RGBA8 byte-scale values.
- Added `scena.visibility_diagnosis.v1`, an inspection-backed visibility
  diagnoser with stable reason codes and explicit fix suggestions behind the
  `inspection` feature. Whole-scene `all_culled` diagnosis requires every
  inspection-visible drawable to be culled.
- Added the `scena` schema-discovery CLI (`schema list` and `schema get`) plus
  `scena.schema_catalog.v1` / `scena.schema_entry.v1` contracts for
  agent-readable contract discovery, and asset-input `render --introspect`,
  `inspect`, and `diagnose --visibility` commands behind the `inspection`
  feature.
- Added doctor coverage that requires whole-file feature-gated contract suites
  to publish their exact feature-enabled cargo commands in the roadmap.
- Added `scena.scene_recipe.v1` plus `scena.scene_recipe_validation.v1` for
  fail-closed declarative recipe validation, the `scena validate-recipe`
  command, and recipe input for `render --introspect`, `inspect`, and
  `diagnose --visibility`.

## [1.7.1] - 2026-06-12

### Fixed

- Moved the long WaterBottle CPU release proof behind the explicit
  `SCENA_RUN_EXPENSIVE_CPU_RELEASE_TESTS=1` release-lane flag. Default
  `cargo test` now records fail-closed metadata instead of letting GitHub
  hosted Linux jobs kill the native/headless gate during the software render.
- Kept the trust-platform manual glTF-material CPU repro in the default suite
  by switching it from the heavy WaterBottle asset to the small in-repo
  textured-triangle fixture.

## [1.7.0] - 2026-06-12

### Added

- Added SceneHost typed transform/subtree controls for browser and native
  hosts, including typed-array transform batches, visibility, subtree queries,
  subtree tinting with exclusions, and explicit stale-handle errors.
- Added GPU post-processing controls: FXAA, bloom, and screen-space ambient
  occlusion, with SceneHost setters and browser proof artifacts on WebGL2.
- Added world-space stroke rendering for technical overlays and grids, with a
  dedicated retained GPU path.
- Added explicit GPU instancing for `InstanceSet` renderables plus SceneHost
  `instantiateUrlInstanced` / `instantiateUrlInstancedUnder` APIs. Returned
  instance-root handles support transform, visibility, opaque tint, removal,
  picking, and additive inspection metadata without duplicating shared asset
  geometry.
- Added SceneHost animation playback exposure:
  `animation_inventory_json`, `play_animation`, `pause_animation`,
  `stop_animation`, `seek_animation`, `set_animation_speed`, and `advance`,
  plus matching WASM methods.
- Added SceneHost presentation transitions for eased transforms and opaque tint
  fades. These are renderer presentation smoothing helpers driven by the host
  clock, not simulation or domain behavior.
- Added honest renderer submission stats: `RendererStats::gpu_draw_submissions`
  reports actual GPU draw submissions, `RendererStats::instances` reports
  visible per-instance records, and legacy `draw_calls` / `primitives` remain
  deprecated triangle-count aliases.
- Added stable contracts `scena.subtree.v1` and
  `scena.animation_inventory.v1`, with golden fixtures under
  `tests/assets/stable-contracts/`.
- Added `examples/scene_host_release_1_7.rs`, a runnable native SceneHost
  example for post-processing, instancing, visibility/tint, camera presets,
  animation advance, and eased updates.

### Changed

- Deprecated `RendererStats::draw_calls` and `RendererStats::primitives` as
  aliases of `triangles`; removal is planned for 2.0. Use
  `gpu_draw_submissions` for renderer submissions and `instances` for
  per-instance record counts.
- Updated the stable v1 schema docs and fixtures for additive fields:
  `capability_report.v1.post_processing`,
  `scene_inspection.v1.nodes[].tint`,
  `scene_inspection.v1.nodes[].material`,
  `scene_inspection.v1.draw_list[].material`,
  `scene_inspection.v1.draw_list[].instance`,
  `scene_inspection.v1.normal_overlays[].instance`,
  `scene_inspection.v1.instance_sets`,
  `scene_inspection.v1.revisions.appearance`, and
  `capture.v1.revisions.appearance`.
- Added material source evidence to asset-aware scene inspection and
  `material_index` to asset-load material fallback rows, so agents can
  distinguish source-authored materials from generated defaults and optional
  texture fallbacks.
- Documented that `scena.subtree.v1.nodes[].name` is always `null` in 1.7;
  stable identification should use host handles or tags.

### Fixed

- Animation mixer transform tracks now bump transform revisions instead of
  structure revisions, avoiding full primitive re-collection for ordinary
  animated playback.
- Subtree tint writes now cancel active per-node tint fades for the touched
  handles, matching the direct-set cancellation rule.

## [1.6.0] - 2026-06-02

### Added

- Added the renderer charter, stable JSON contract policy, WASM scene-host
  checklist, and browser renderer-fidelity dependency checklist that gate the
  upcoming WASM integration work.
- Added stable v1 serde contracts for capability reports and scene inspection,
  including report-local inspection handles, topology helpers, and schema
  constants for external JSON consumers.
- Added the `scene-host` feature with a native-testable `SceneHostCore`, a
  browser/WASM `SceneHost` facade, host-owned `u64` node/import handles,
  construction primitives, per-frame batch transforms, CSS-pixel picking,
  inspection JSON, and capability/diagnostic/stat JSON helpers.
- Added `scena.capture.v1` with public RGBA8 capture descriptors, FNV-1a
  pixel hashes, revision/camera/viewport/backend metadata, viewer and
  `SceneHost` capture helpers, and deterministic CPU-headless capture tests.
- Added public `Transform::compose` / `Transform * Transform` scene-graph TRS
  composition plus recursive `Scene::remove_node`, `Scene::remove_import`, and
  real `SceneHost` remove APIs with stale-handle invalidation.
- Added per-node tint/highlight render state through `Scene::set_node_tint`,
  `Scene::node_tint`, `Node::tint`, `SceneHostCore::set_node_tint`, and WASM
  `setNodeTint` / `clearNodeTint`; inspection JSON now includes `nodes[].tint`.
- Added `scena.asset_load_report.v1` for serializable scene asset load reports,
  including geometry summaries, progress events, cache-hit warning retention,
  typed missing image/buffer warnings, strict external-resource loading, and
  SceneHost URL instantiate-with-report JSON.
- Added generic `AssetProvenance` metadata on loaded scene assets, textures,
  and environments so asset consumers can inspect source paths, SHA-256 hashes,
  license/generator metadata, and generated derivatives through one serde
  contract. `scena.asset_load_report.v1` and
  `scena.asset_geometry_summary.v1` now include the provenance value.
- Added interactive `SceneHost` camera APIs for saved viewpoints and
  Rust-owned orbit/pan/wheel input, exposed natively through
  `SceneHostCameraState` and over WASM through `setCamera`, `getCameraJson`,
  `cameraPointerDown`, `cameraPointerMove`, `cameraPointerUp`, and
  `cameraWheel`.
- Added stable-contract examples, golden JSON fixtures under
  `tests/assets/stable-contracts/`, and `xtask doctor` evidence checks for the
  shipped SceneHost, inspection, capability, capture, annotation, asset-load,
  geometry-summary, and provenance contracts. The fixtures are also asserted
  against live serde serialization so contract drift requires a visible fixture
  update.
- Added `browser:scene-host-proof`, a Raspberry Pi V3D WebGL2 browser/GPU
  proof harness for `SceneHost` that builds the `scene-host` wasm package,
  constructs a multi-part browser scene, and records
  `scena.scene_host_browser_proof.v1` JSON plus a PNG artifact.

### Fixed

- Hardened capture descriptors so `render -> mutate scene -> capture` fails
  closed with `CaptureError::StaleRender` instead of binding new scene
  revisions to an older framebuffer.
- Fixed WASM `SceneHost.capture()` on browser WebGL2 surfaces so it binds
  revision/camera metadata to canvas RGBA8 readback instead of the CPU-side
  headless frame buffer.

## [1.5.1] - 2026-05-23

### Changed

- Reworked the public showcase material section to use the approved live
  browser-rendered 12-sphere scene instead of PNG proof thumbnails, with
  source-backed material preset code shown for satin, leather, and rubber.
- Optimized the public showcase startup by lazy-loading below-fold static
  images, starting hero/model GLB fetches in parallel with WASM
  initialization, and replacing the material backdrop band stack with one
  gradient mesh.
- Updated the easy scene setup guide so its material preset snippet shows
  every v1.5 `MaterialDesc` preset, not only the earlier subset.
- Corrected stale v1.5 documentation wording in material rustdocs, release
  notes, and historical checklists, and added doctor coverage so the guide
  cannot omit shipped material presets again.
- Renamed the easy scene showcase example and documentation asset folder away
  from the old v1.4-specific names.

### Fixed

- Fixed the public showcase WebGL2 surface lifecycle so the materials, model
  loading, and connector sections detach inactive canvases before activating
  the next live renderer instead of leaving later canvases black.
- Changed `GridFloorOptions::under_bounds` so the default floor plane is placed
  at the supplied bounds' minimum Y, preventing generated showcase floors from
  cutting through imported models.

## [1.5.0] - 2026-05-21

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
  (the v1.4 "honest" four; glass/chrome/leather were postponed until later
  renderer proof).
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
