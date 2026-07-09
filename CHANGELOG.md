# Changelog

All notable user-facing changes are recorded here.

## [Unreleased]

### Fixed

- Changed CAD edge-emphasis rendering to merge coincident imported mesh
  vertices by position before selecting visible edges, so duplicated
  triangulation diagonals no longer render as technical CAD lines.
- Suppressed smooth cylinder side-facet tessellation in CAD edge-emphasis
  rendering while preserving cap-ring feature edges.

## [1.7.2] - 2026-06-27

### Fixed

- Rebuilt the easy-scene chrome showcase cards with the real Studio HDR
  environment and higher sphere tessellation so reflective materials read as
  smooth studio chrome instead of faceted synthetic reflections.
- Removed the experimental `chrome_white_line` environment preset and updated
  recipe validation, docs, and doctor pins to use file-backed environments.
- Added warning-level recipe validation for near-mirror sphere materials whose
  tessellation is too low for smooth reflections, and raised authored recipe
  sphere defaults to a safer baseline.
- Restored public recipe-render reflection verification coverage for chrome
  read failures, including bright/dark fraction diagnostics.
- Hardened browser and platform CI proof for the public demo, WebGL2 software
  lanes, Windows DX12, macOS Metal, and WaterBottle reference-image artifacts.

### Added

- Improved the LLM app-builder path: schema examples and `examples agent`
  templates now include a key/fill/rim light rig, bundled HDR environment,
  presentable backgrounds, and a useful capture-size floor; recipes can now use
  `scene.preset:"product_studio"|"cad_studio"|"industrial_studio"` to apply a
  matching environment, background, floor/grid, floor reflection, and
  contact-shadow SSAO from one field; `validate-recipe` enforces the same local
  path policy as recipe build, recipe bbox-fit expectations measure subject
  bounds instead of floor/grid helpers, and recipe animation verification
  handles imported glTF clips as well as authored clips.
- Added opt-in GPU rendering for `scena render` and `scena recipe render`
  through `--gpu` / `SCENA_USE_GPU=1`; CPU headless rendering remains the
  deterministic default, and reports continue to expose the backend actually
  used.
- Added opt-in hero reconstruction for supersampled recipe renders with
  `render.reconstruction: "box" | "tent" | "gaussian"`; downsampling now filters
  in linear light, `box` remains the default, and supersample factor `8` is
  guarded by target-size capability checks before allocating the internal frame.
- Added opt-in depth-of-field post-processing through
  `DepthOfFieldConfig` and recipe `render.depth_of_field`, with CPU and
  HeadlessGpu depth-source reporting plus an `expect_quality.depth_of_field`
  verifier that compares the native-resolution render against a same-backend
  no-DoF baseline.
- Added conservative GPU prepare auto-instancing for repeated ordinary mesh
  nodes with identical geometry/material and no morph/skin deformation, reducing
  repeated authored-node draw setup while keeping the CPU reference path and
  scene identity unchanged.
- Added allocation-aware M9 performance budgets: benchmark rows now record
  `p95_allocations_per_frame` and `max_allocations_per_frame`, compare them
  against stored baselines, and fail release-readiness when frame-time or
  allocation budgets regress.
- Increased the default/recommended grid-floor stroke width and tightened the
  recipe grid-line quality verifier to inspect the native-resolution
  lower-floor detail crop, so hero floor grids no longer pass from a broad
  full-frame metric while still looking chunky in the visible crop.
- Reduced HDRI specular fireflies in chrome/polished-metal reflections by
  source-mip sampling during GGX environment prefiltering, and added
  `expect_quality.reflection.max_firefly_fraction` so recipe verification can
  fail isolated bright reflection specks instead of treating them as valid
  structure.
- Added fitted-table LTC area-light specular evaluation for rectangular, disc,
  and sphere recipe area lights on the CPU reference path and both GPU PBR
  shader variants, using compact tables generated from the public
  `selfshadow/ltc_code` reference with CPU/lavapipe recipe-render parity proof
  across shape and roughness sweeps.
- Added the repo-hosted `.codex/skills/scena-app-builder` LLM skill and
  `docs/guides/llm-app-builder.md` guide for building and verifying scena
  applications from public schemas, recipes, CLI commands, diagnostics, and
  repair workflows without relying on private renderer internals. The guide is
  also surfaced from `scena --help` and the crate-level docs so installed-crate
  users can find it without browsing hidden repo folders.
- Tightened recipe-authored advanced-PBR validation so `ior` matches the public
  material builder domain exactly and GPU-inert volume texture slots
  (`transmission_texture`, `thickness_texture`) fail closed instead of silently
  rendering differently across backends; scalar KHR volume fields
  (`thickness_factor`, `attenuation_distance`, `attenuation_color`) remain
  supported and now have coupled-scene GPU pixel proof.
- Tightened recipe-authored animation validation and execution: recipe JSON now
  has an operator-owned byte cap, authored animation clips/channels/keyframes
  are capped by `RecipeBuildPolicy`, clip duration must cover every keyframe,
  weight channels fail closed on non-morph targets or wrong morph widths, and
  public authored clip/mixer creation rejects malformed clips instead of
  accepting inert or mis-sampled animation data.
- Added the `scena.visual_patch.v1` SceneHost patch contract for batched
  transform, tint, visibility, camera, eased transition, animation-time,
  selection/hover, material variant, label-anchor, and metadata updates, with
  native and WASM entrypoints plus stable fixtures.
- Added the `scena.host_event.v1` SceneHost event contract for pick, hover,
  selection, load, diagnostic, capture, surface, context, device, and
  capability events, with native drain/sink APIs, sink-only push delivery,
  WASM `drainEventsJson()`, browser context-loss forwarding helpers, and a
  stable fixture.
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
  luminance reports shader-encoded RGBA8 byte-scale values. The report now
  fails closed for camera-frustum failures, alpha-zero drawables, non-finite
  transforms, and active clipping planes that remove all visible content.
  SceneHost native and WASM hosts can now request the same report through
  `render_introspection_json` / `renderIntrospectionJson()`, with browser proof
  covering empty, offscreen, and valid centered frames.
- Added `scena.render_quality.v1`, a native-resolution recipe verification
  report for severe exposure failures, profile-scoped exposure/contrast/noise
  and text-integrity checks, geometry/line/reflection/area-light/grounding/depth-
  of-field checks, and opt-in reference fidelity (`rgba_abs_diff`, Delta E 2000,
  SSIM). Failures carry actionable `fix_hint` strings and are surfaced in the
  compact recipe verification reasons.
- Added `scena.visibility_diagnosis.v1`, an inspection-backed visibility
  diagnoser with stable reason codes and explicit fix suggestions behind the
  `inspection` feature. Whole-scene `all_culled` diagnosis requires every
  inspection-visible drawable to be culled; targeted diagnosis covers subtree
  and SceneHost import roots, hidden ancestors, non-finite transforms, layer
  masks, alpha/material transparency, missing geometry/material evidence,
  camera-frustum diagnostics, active clipping-plane hints, and backend
  capability degradation.
- Added the `scena` schema-discovery CLI (`schema list` and `schema get`) plus
  `scena.schema_catalog.v1` / `scena.schema_entry.v1` contracts for
  agent-readable contract discovery. Schema entries can now include canonical
  invalid examples.
- Added asset-input `render --introspect`, `inspect`, and
  `diagnose --visibility` commands behind the `inspection` feature.
  Agent-facing asset-load failures now emit `scena.asset_doctor.v1` JSON
  instead of prose-only command errors, and the global `--round-floats <0..6>`
  option lets callers reduce floating-point JSON precision without changing
  integer handles or counts.
- Added doctor coverage that requires whole-file feature-gated contract suites
  to publish their exact feature-enabled cargo commands in the roadmap.
- Added `scena.scene_recipe.v1` plus `scena.scene_recipe_validation.v1` for
  fail-closed declarative recipe validation, the `scena validate-recipe`
  command, and recipe input for `render --introspect`, `inspect`, and
  `diagnose --visibility`. Recipe validation now reports missing assets as
  structured JSON errors and expected-extent scale mismatches as warning-level
  diagnostics. Recipes now also support section-box, distance-measurement,
  callout, and exploded-view directives through the SceneHost rendering path;
  remaining future sections such as anchors, connectors, bounds, authored
  planes, and named states fail as `unsupported_feature` until their owner
  features land.
- Expanded `scena.scene_recipe.v1` authored-scene recipes with primitive and
  custom-mesh geometries, unlit/PBR/line/wireframe/edge materials with texture
  slots, directional/point/spot lights, node hierarchy and visual attributes,
  and authored-node section-box/callout targets.
- Added real TrueType/OpenType label fonts through `LabelFontFace`,
  `LabelDesc::truetype`, and recipe `fonts[]` plus label `font` references.
  Font-backed labels use real glyph metrics and kerning for basic Latin glyph
  shapes, preserve glyph coverage through the renderer-owned atlas path, and
  fail closed for missing/oversized fonts or complex-script text that scena does
  not shape. Explicit label text/background/halo colors are now fail-closed
  unless opaque so user alpha is not silently ignored.
- Added authored morph and skin recipe directives plus `Scene::set_skin_binding`
  for deterministic recipe-built deformation data. `scene_recipe.v1` can now
  derive morph and skin geometries, bind authored joint nodes with inverse bind
  matrices, set initial morph weights, and author morph-weight animation on
  morph-capable nodes with fail-closed validation. Deformation support is
  position-focused in this release: morph targets do not author morphed normals,
  and skinned normals use the joint direction transform, so non-uniform joint
  scale is not a lighting-correctness guarantee.
- Added host-supplied particle rendering through `Particle`, `ParticleSet`,
  `Scene::add_particle_set_node()`, and additive `scene_recipe.v1` `particles[]`
  directives. Particles render as opaque camera-facing screen-sized sprites with
  per-particle color, size, rotation, bounds, fail-closed recipe and Rust API
  validation, and headless-GPU proof for color, position, size, rotation, and
  depth behavior; time-stepped particle simulation remains host-owned.
- Added `Scene::frame_all_with_overlays`,
  `SceneHostCore::frame_all_with_overlays`, and browser
  `frameAllWithOverlays()` to frame geometry plus generated overlay label
  anchors with label-derived pixel margin for documentation captures.
- Added `scena.browser_proof_run.v1` and
  `scena browser-proof [scene-host|m6] [--dry-run]`, a machine-readable
  wrapper over the existing wasm-pack + Playwright browser proof lanes.
- Added `scena.placement_result.v1` and `scena place` transform previews for
  recipe imports using `center`, `ground`, `fit_to_size`, `look_at`,
  `align_to_anchor`, and `place_on`, with render-introspection proof that
  center, ground, and align-to-anchor previews produce visible framed output.
- Added `scena.visual_repair_plan.v1`, `scena.agent_loop_result.v1`, and the
  `scena repair --from <report.json>` CLI for conservative repair planning
  over render introspection and visibility diagnosis reports.
- Added `scena.appearance_expectation.v1`,
  `scena.appearance_introspection.v1`, and
  `scena verify appearance --expect <json>` for capture-bound first-time
  material, variant, fallback, alpha, texture, and swatch verification behind
  the `inspection` feature.
- Added `scena.animation_introspection.v1` observed transform values for
  `scena verify animation --expect-translations`, including sampled transforms,
  explicit `--expect-node-handle` binding, expected translations, tolerance
  results, and fail-closed `expected_value_mismatch` diagnostics.
- Added stricter `scena verify interaction` assertions for cleared hover and
  selection state, with native CLI coverage for pick hits, misses, wrong
  handles, hover enter/leave, selection set/clear, and CSS-vs-physical pixel
  mismatch reports.
- Added reusable `CameraState`, `CameraBookmark`, and `OrbitControls::fly_to`
  APIs, viewer bookmark storage helpers, and SceneHost native/WASM bookmark
  easing methods that delegate to the existing host-ticked `camera_eased`
  visual patch channel.
- Added platform-neutral `TransformGizmo` controls for translate/rotate/scale
  manipulation from caller-supplied pointer rays, with world/local/view-aligned
  spaces, axis/plane constraints, SceneHost `VisualPatchV1` emission, helper
  stroke geometry, a stable `scena.scene_host_gizmo_drag.v1` SceneHost request
  contract plus WASM `applyGizmoDragJson()`, and a
  `simple_scene_editor_gizmo.rs` example.
- Added `ViewerProfile` presets for `model_viewer`, `cad_inspection`,
  `product`, `industrial`, and `documentation` viewer setup. The presets
  compose existing renderer profile, render mode, lighting, environment,
  background, grid, picking, and orbit-control helpers without adding a
  separate viewer engine.
- Added `scena.asset_catalog.v1`, `scena.asset_readiness_report.v1`, and
  `Assets::validate_asset_catalog()` for host-owned asset catalog manifests.
  Readiness validation uses real asset loads and reports structured findings
  for missing sources/files, explicit units/coordinate systems, bounds limits,
  authored anchors/connectors/tags, material variants, base-color texture
  requirements, external resources, and material fallbacks.
- Added `scena.connector_browser.v1` SceneHost reports for import, subtree,
  and selection connector browsing with metadata compatibility, snap preview
  distance/tolerance, ghost-transform cues, and stable host handles.
- Added SceneHost material-variant helper reporting: asset-import and
  host-backed inspection JSON now include declared variant names plus the
  active variant, and duplicate source variant names fail closed instead of
  selecting by declaration order.
- Added `scena.product_options.v1` SceneHost product option groups for
  visual-only configurator choices that apply `VisualPatchV1` entries and
  report active options without pricing, compatibility, inventory, or document
  ownership semantics.
- Added `scena.presentation_timeline.v1` SceneHost timelines for host-ticked
  guided tours that flatten visual states, camera bookmarks, labels, tints,
  transforms, and animation mixer sampling into deterministic `VisualPatchV1`
  seeks.
- Added `scena.scene_host_grounding.v1` and SceneHost product grounding
  helpers that compose studio visuals, a floor receiver, SSAO-backed contact
  darkening, and explicit directional-shadow fallback reporting without
  claiming physical shadow correctness before proof closure.
- Promoted directional-shadow capability reporting on GPU-device WebGPU/WebGL2
  and native lanes after visible receiver-darkening proof; CPU/reference and
  unattached factory rows remain degraded with an explicit diagnostic.
- Added deterministic generated catalog previews with
  `render_asset_catalog_preview_png()`, `AssetCatalogPreviewPng`,
  `AssetCatalogPreviewError`, viewer-builder background color support, and the
  `asset_catalog_picker` SceneHost example, plus WebGL2 browser proof for a
  catalog asset preview workflow.
- Added `scena.asset_doctor.v1` runtime asset doctor reports through
  `Assets::doctor_asset_path()`, `Assets::doctor_loaded_asset()`,
  SceneHost native/WASM JSON methods, and the `scena doctor` CLI, with stable
  finding codes, help text, suggested fixes, documented `ok` semantics
  (`ok=true` means no error-severity findings, not warning-free asset
  completeness), and browser proof for a broken asset diagnosis.
- Added measurement overlay primitives for CAD/documentation views:
  distance, angle, bounds-dimension reports, host-supplied `UnitFormat`
  formatting, and line plus optional label rendering through
  `Scene::add_measurement_overlay()`.
- Added CAD-style inspection helpers for scene visibility and context:
  `Scene::isolate`, `show_only`, `hide`, `show`, `toggle_visibility`,
  `ghost`, restoreable visibility/tint snapshots, selected-node framing,
  bounding-box helper overlays, local/world axes triads, and an inspection
  toolkit report, plus a `cad_inspection_viewer` example.
- Added additive `parent` and `children` fields to `scena.subtree.v1` so
  SceneHost subtree reports can directly drive CAD-style part trees from
  stable host handles.
- Added `ExplodedView` and `ExplodedViewPlan` for reversible presentation-only
  assembly exploded views, plus `SceneHostCore::exploded_view_patch_json()` for
  emitting existing `scena.visual_patch.v1` transform or eased-transform
  channels with a metadata-carried restore patch for JSON/WASM hosts.
- Added SceneHost named visual states via
  `scena.scene_host_visual_state.v1` and
  `scena.scene_host_visual_states.v1`, storing host-named `VisualPatchV1`
  presets with opaque metadata and deterministic inventory/application APIs.
- Added embedded 5x7 glyph-cell label text rendering for `LabelDesc::sdf()`
  and `LabelDesc::msdf()`: stable text metrics, screen-sized camera-facing
  billboards, optional background/halo styling, native visual proof, a
  many-label benchmark artifact, and WebGL2 browser proof for a dense label
  helper scene. The SDF/MSDF enum values are preserved as API intent; the 1.7
  renderer path does not yet generate distance-field glyphs.
- Added callout and leader-line helpers for node, world, anchor, and connector
  targets, with SceneHost/WASM node and world callout entrypoints that share
  annotation anchor IDs with the `labels` visual-patch channel.
- Added `<scena-viewer>` annotation layout helpers for viewport clamping,
  behind-camera and optional occlusion hiding, deterministic priority-based
  overlap avoidance, and layout reports that list original and adjusted
  screen-space positions.
- Added `examples/application_builder_lab.rs`, a SceneHost-driven app-builder
  lab that creates one mini-application per roadmap archetype and writes PNG,
  capture/contract JSON, and findings artifacts, plus a checked-in findings
  summary under `docs/checklists/`. Every raw-`Scene` archetype now proves its
  render through the `render_introspection` contract (fail-closed on
  `ok == false`) instead of a capture-buffer-length check that could never
  fail; the model-viewer archetype now uses the same introspection contract via
  high-level viewer helpers.

### Changed

- `scena recipe render <recipe> --introspect` now succeeds without `--verify`
  and emits the same `scena.render_introspection.v1` shape as generic
  `scena render`; adding `--verify` keeps the existing combined
  recipe-render result.
- High-level glTF viewer results now expose render-introspection helpers, and
  section boxes accept zero-thickness planar bounds when a positive margin
  expands them into a usable volume for CAD-style cutaways.
- Render introspection now flags near-edge content as cropped instead of only
  detecting exact outermost-pixel contact, and CAD/documentation agent smoke
  templates now emit runnable CLI load/render/diagnose workflows with explicit
  notes for native-only overlay authoring.
- Hardened agent-facing repair and placement contracts: patch-less
  presentation repairs such as `frame_bounds` now fail closed as
  host-input-required instead of reporting `auto_fixable=true`, placement
  result transforms serialize with three-decimal stable JSON numbers, and
  `doctor --full` now directly pins the `scena.visual_repair_plan.v1` and
  `scena.agent_loop_result.v1` stable fixtures.
- Updated transmission/IOR/volume asset guidance and glass preset docs to
  reflect the proven physical-glass lane: attached GPU-device native, WebGPU,
  and WebGL2 capability rows can report
  `physical_glass_transmission=supported`, while CPU/reference and unattached
  factory lanes remain degraded and should use optional extensions or fallback
  materials for required assets.
- Strengthened the WebGL2 dense source-material proof so the
  `source-gltf-materials` browser artifact records WaterBottle source texture
  roles, camera framing, lighting, comparison lanes, stats, capabilities, and
  screenshot metadata instead of only a texture-binding count.
- Extended `<scena-viewer>` into a thin SceneHost adapter for browser hosts:
  the element now binds to a host, forwards `VisualPatch` JSON, re-emits
  drained `HostEvent` entries as DOM events, and delegates capture/download,
  picking, hover, selection, framing, camera, and studio-lighting helpers to
  the existing Rust/WASM host methods.

### Fixed

- CPU depth buffers now store normalized post-process depth while preserving
  view-depth interpolation for rasterization, so CPU SSAO uses the same
  threshold space as GPU SSAO and no longer turns tiny floor-depth jitter into
  broad noisy contact-shadow bands.
- TrueType label rendering now preserves glyph coverage instead of thresholding
  anti-aliased pixels into hard-edged 1-bit cells. Renderer-owned glyph
  coverage is distinct from user label alpha, so the existing fail-closed
  opaque-label style invariant remains intact while CPU and GPU captures keep
  smooth label edges.
- CPU label backgrounds now use the same flat overlay color transform and
  display-space blending as the GPU label atlas path, so full label regions
  (background pill plus glyphs) match within the documented lavapipe tolerance.

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
