# Easy Scene Setup And Auto-Framing Checklist

Updated: 2026-05-18

Goal: `scena` must make the common model-viewer/showcase workflow easy. A user
should not spend days hand-tuning camera distance, target offsets, lighting,
floor size, exposure, or connector-demo framing. The library must provide
tested primitives for loading a model, lighting it, framing it, placing it on a
simple floor, and connecting authored anchors/connectors without raw matrix
math.

This checklist is the execution contract for turning the connector-demo pain
into reusable library capability. The demo must consume these primitives after
they land; demo-specific constants are not the long-term fix.

Product standard: this is not a temporary demo patch. The goal is a
state-of-the-art Rust 3D library workflow where the first useful render is
pleasant by default, deterministic under test, and still explicit enough for
professional applications to control.

## Current Status

- [x] Review hardening complete in this worktree; ready for user visual review.
- [x] Second review hardening complete:
      - [x] public rustdoc no longer exposes internal phase labels
      - [x] `frame_bounds()` documents aspect overwrite, examples, and errors
      - [x] typed framing error variants have rustdoc
      - [x] README surfaces Easy Scene Setup before the older first-scene path
      - [x] v1.3.0 release notes list the new public API surface
      - [x] Three.js migration guide includes before/after framing code
      - [x] doctor rules enforce the real drift patterns, not dead token names
      - [x] browser probe enforces foreground coverage, mobile height, and
            connector-label projection alignment
      - [x] `TransmissionTest.glb` is a transmissive near/far control asset
      - [x] stale Cloudflare-demo screenshots are removed before each probe run
- [x] Reopened after live verification on 2026-05-18. The previous green
      status was stale: doctor failed, mobile proof was not acceptable, and
      several source-level checklist contracts were not enforced.
- [x] `cargo run -p xtask -- doctor --full` passes without
      `ARCH-KISS-SIZE` or demo/checklist enforcement findings.
- [x] `OrbitControls::focus_on_framing()` adopts the full framed pose
      (target, distance, yaw, pitch) so demos do not re-inject literal camera
      angles after framing.
- [x] Connector demo uses `frame_bounds()` without connector-specific fallback
      camera distance/target constants.
- [x] Mobile connector screenshot is acceptable: model visible, not clipped,
      labels separated/readable, diagnostics hidden, and canvas height not
      collapsed.
- [x] `project_world_point()` and renderer camera projection share one
      canonical world-to-view transform helper.
- [x] Framing failures use structured error variants, not a single
      `InvalidFramingOptions { reason }` catch-all.
- [x] Doctor enforces the real failure families:
      - [x] no post-framing `.with_angles(...)` pose patches in the public demo
      - [x] diagnostics closed by default
      - [x] frame counter text only inside diagnostics
      - [x] easy scene setup guide contains runnable Rust snippets for
            `frame_bounds`, `add_studio_lighting`, and `add_grid_floor`
      - [x] demo build uses the heartbeat wrapper, not direct long-running
            `wasm-pack`
- [x] `frame_bounds()` has deterministic rendered-output proof with image and
      metadata sufficient to catch tiny, clipped, or off-center objects.
- [x] `docs/feature-flags.md` uses the current `scena = "1.3"` snippet.
- [x] M5 public API freeze covers all newly public easy-setup API names.
- [x] `Aabb::union()` is public and examples do not hand-roll bounds union.
- [x] Legacy `Aabb::framing_transform()` and `FramingAngles` are removed from
      the public surface in favor of `Scene::frame_bounds()`.
- [x] Visual handoff authorization updated on 2026-05-18: user authorized
      agent self-review, commit, push, CI monitoring, and cargo version-bump
      check without a separate visual approval round.
- [x] Named camera-view hardening complete:
      - [x] `FramingOptions::azimuth_elevation(-27.5, 17.8)` reproduces the
            previously approved connector view vector within `1e-3`
      - [x] `FramingOptions` has named cardinal and three-quarter view presets
      - [x] connector demo uses the named angle API instead of a raw
            `Vec3::new(...)` direction constant
      - [x] doctor rule `DEMO-CAMERA-VIEWS-NAMED` rejects inline demo camera
            vectors, literal orbit pose patches, `.with_angles(...)`, and
            approved/connector view constants

Final camera-view evidence, captured on 2026-05-18:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo run -p xtask -- doctor --full`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- `SCENA_BUILD_HEARTBEAT_MS=10000 npm run demo:build`
- `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18109/index.html`
- Manual/self visual review of:
  - `target/gate-artifacts/cloudflare-demo/connector-snap-page.png`
  - `target/gate-artifacts/cloudflare-demo/connector-snap-mobile-page.png`
  - `target/gate-artifacts/cloudflare-demo/connector-snap-replay-page.png`
  - `target/gate-artifacts/cloudflare-demo/connector-snap-post-replay-page.png`

Current review-hardening evidence, captured on 2026-05-18:

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `cargo test -p xtask easy_scene_setup_contracts -- --nocapture`
- `cargo test --test easy_scene_setup_framing -- --nocapture`
- `cargo test --test examples_visual_proof frame_bounds_rendered_output_proves_fill_center_and_unclipped_object -- --nocapture`
- `cargo test --test m5_release m5_public_api_baseline_names_frozen_contracts -- --nocapture`
- `cargo run -p xtask -- doctor --full`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- `SCENA_BUILD_HEARTBEAT_MS=10000 npm run demo:build`
- `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18108/index.html`

Second review-hardening evidence, captured on 2026-05-18:

- `cargo test -p xtask easy_scene_setup_contracts -- --nocapture`
- `cargo test --test easy_scene_setup_framing -- --nocapture`
- `cargo test --test examples_visual_proof frame_bounds_rendered_output_proves_fill_center_and_unclipped_object -- --nocapture`
- `cargo test --test m5_release m5_public_api_baseline_names_frozen_contracts -- --nocapture`
- `cargo fmt --check`
- `cargo run -p xtask -- doctor --full`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
- `cargo test`
- `SCENA_BUILD_HEARTBEAT_MS=10000 npm run demo:build`
- `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18108/index.html`

## Previously Completed Work That Must Stay Green

- [x] Test-first contract covered by `tests/easy_scene_setup_framing.rs`
      before migrating the demo.
- [x] Browser proof captured under
      `target/gate-artifacts/cloudflare-demo/`.
- [x] Manual screenshot review completed for connector before/mid/after/orbit,
      Drive unit, Load unit, WaterBottle, ToyCar, and mobile connector page.
- [x] One browser-proof bug was found during manual review: the probe accepted
      stale sample frames while WaterBottle was still parsing. Fixed by
      resetting the demo frame counter before each load and waiting for
      `status-detail == "rendered"` before screenshots.
- [x] Gate evidence:
      - `cargo fmt --check`
      - `cargo check --features demo-page`
      - `cargo clippy --all-targets -- -D warnings`
      - `cargo test` (`CARGO_TEST_EXIT:0`, logged at
        `target/gate-artifacts/cargo-test-final.log`)
      - `cargo run -p xtask -- doctor --full`
      - `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`
      - `npm run demo:build`
      - `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18106/index.html`

## Execution Rules

- [x] Do not solve this by adding more hand-tuned demo camera constants.
- [x] Do not add hard time-box language or a "ship the least-bad constant"
      escape hatch to this checklist. The library primitive is the fix.
- [x] Do not bundle everything into a monolithic `frame_showcase()` API first.
      Build and prove composable primitives, then add a combined helper only
      if repeated examples demand it.
- [x] Every production-code change starts with the narrowest failing test or a
      documented deterministic-proof exception.
- [x] Every browser-visible change needs rendered-output proof; unit tests
      alone are not visual proof.
- [x] The connector demo is the acceptance surface after the library primitives
      land. It must be updated to use the primitives and then tested in the
      actual browser demo.
- [x] Documentation is part of the feature. Do not mark implementation done
      until README, rendering docs, examples, and the dedicated easy-use guide
      explain the workflow.
- [x] User visual approval happens only after our own screenshot review passes.

## Target User Workflow

The final workflow should be close to this shape:

```rust
let mut assets = Assets::new();
let import = assets.load_scene("machine.glb").await?;

let mut scene = Scene::new();
let root = scene.instantiate(&import)?;

scene.add_studio_lighting();
scene.add_grid_floor(&mut assets, GridFloorOptions::under_import(&import))?;

let camera = scene.add_perspective_camera(CameraDesc::default());
scene.frame_bounds(
    camera,
    scene.world_bounds(root)?,
    FramingOptions::new()
        .three_quarter_front_right()
        .fill(0.72)
        .margin_px(48.0)
        .viewport(width, height),
)?;

renderer.set_auto_exposure(AutoExposure::studio_default());
renderer.prepare(&scene, &assets)?;
renderer.render(&scene, &assets, target)?;
```

The connector workflow should remain equally direct:

```rust
let drive = scene.instantiate(&drive_part)?;
let load = scene.instantiate(&load_part)?;
scene.mate(&drive, "shaft", &load, "hub")?;
```

## 0. Root Cause And Scope

- [x] Record the root cause in this checklist before implementation:
      manual demo constants were used for distance, target bias, floor depth,
      connector before/after bounds, and label margins.
- [x] Treat this as a library ergonomics gap, not a one-off demo bug.
- [x] Keep the feature scoped to renderer/viewer authoring helpers:
      camera framing, bounds projection, floor helpers, lighting helpers,
      examples, diagnostics, and docs.
- [x] Do not add simulation, robotics, physics, game engine ECS, or
      domain-specific connector behavior.
- [x] Keep primitives separate:
      - [x] bounds-to-camera framing
      - [x] multi-state bounds union
      - [x] studio lighting
      - [x] grid/floor helper
      - [x] world-to-screen projection
      - [x] connector mate helper/docs
- [x] Do not ship a broad `frame_showcase()` API until multiple demos/examples
      prove the same combined shape is needed.
- [x] Confirm the current governing RFC path before implementation. If
      `docs/RFC-rust-3d-renderer.md` is absent in this worktree, either restore
      the canonical RFC or record the current canonical scope document before
      changing public API.

## 0.5 Implementation Sequence

- [x] Inventory current manual constants and private math:
      - [x] connector camera distance/target/fill constants
      - [x] floor size/depth/horizon constants
      - [x] label projection math
      - [x] replay before/after bounds logic
      - [x] lighting/exposure setup
- [x] Add failing projection and screenshot tests for the framing primitive.
- [x] Implement `frame_bounds()` / `FramingOptions`.
- [x] Add multi-state bounds union support needed by connector replay.
- [x] Expose world-to-screen projection for labels/helpers.
- [x] Add or promote studio lighting, grid floor, and auto-exposure helpers as
      reusable library surfaces.
- [x] Migrate the connector demo to the library primitives.
- [x] Update docs, README, and examples.
- [x] Run focused, repo, browser, visual, and doctor gates.
- [x] Perform our own visual review across every demo page.
- [x] Ask for user visual approval after the above is complete.

## 1. Public API Shape

### 1.1 Projection-Based Camera Framing

- [x] Add a focused public framing primitive, owned by `scene` or a small
      viewer/framing module with a clear owner.
- [x] Preferred API shape:

```rust
let outcome = scene.frame_bounds(
    camera,
    bounds,
    FramingOptions::new()
        .view_direction(Vec3::new(0.8, 0.35, 0.7))
        .fill(0.70)
        .margin_px(48.0)
        .viewport(width, height),
)?;
```

- [x] Use `view_direction: Vec3` as the primary input, not raw orbit angles.
- [x] Add convenience constructors only after the primitive is correct:
      - [x] `FramingOptions::look_from(direction)`
      - [x] `FramingOptions::orbit(yaw, pitch)` as a convenience wrapper
      - [x] `FramingOptions::isometric()`
      - [x] `FramingOptions::front()`
- [x] Support perspective cameras first.
- [x] Decide explicitly whether orthographic support lands in the same patch:
      - Decision: orthographic `frame_bounds()` is deferred for this patch.
      - [x] Return a structured unsupported-camera error and add the follow-up
            item here. `project_world_point()` supports both perspective and
            orthographic projection.
- [x] Keep API naming general and reusable:
      - [x] `frame_bounds`
      - [x] `FramingOptions`
      - [x] `FramingOutcome`
      - [x] `FramingOptions::viewport(width, height)`
      - [x] `ScreenRect`
- [x] Return a structured result:

```rust
pub struct FramingOutcome {
    pub camera_transform: Transform,
    pub target: Vec3,
    pub distance: f32,
    pub projected_rect: ScreenRect,
    pub fill: f32,
    pub margin_px: f32,
}
```

- [x] Expose opt-in near/far tightening from framed bounds and keep it disabled
      by default until the caller has verified it is safe.
- [x] Preserve explicit lifecycle: framing mutates scene camera state and marks
      the scene dirty; it must not prepare, render, fetch assets, or upload GPU
      resources.
- [x] The framing primitive must be callable before the first `prepare()` or
      `render()` call. This is the order every easy-use model-viewer example
      needs: load, instantiate, light, frame, prepare, render.
- [x] After framing, orbit/camera controllers must adopt the computed target as
      their pivot. A user drag after auto-framing must orbit around the framed
      object, not the old/default target.
- [x] Near/far tightening must be conservative and material-aware. It must not
      break environment/IBL contribution, reflections, transmissive materials,
      or canonical control assets such as WaterBottle.
- [x] Use typed errors, not silent fallback:
      - [x] empty bounds
      - [x] invalid view direction
      - [x] missing camera
      - [x] invalid viewport
      - [x] invalid framing options such as `fill <= 0`, `fill > 1`,
            negative `margin_px`, or margins larger than the usable viewport
      - [x] unsupported camera type if orthographic is deferred

### 1.2 Correct Projection Math

- [x] Transform the AABB corners into the candidate view space before solving
      distance.
- [x] Do not use bounding-sphere radius as the core distance solver.
- [x] For perspective cameras, solve distance from both axes:

```text
required_distance = max(
  half_view_height / tan(fov_y / 2),
  half_view_width / (aspect * tan(fov_y / 2))
)
```

- [x] Fold `fill` and `margin_px` into the usable viewport before solving.
- [x] Handle non-square viewports explicitly.
- [x] Add a portrait/mobile regression case because wide-object framing breaks
      there first.
- [x] Preserve the requested `view_direction` and only adjust target/distance.
- [x] If the bounds are off-center in view space, offset the target so the
      projected rectangle centers within tolerance.
- [x] Verify clipped-left-but-zoomed-out failures are impossible under the new
      solver by asserting projected min/max coordinates, not just camera
      distance.
- [x] Clamp only through explicit options; no hidden hard-coded demo constants.
- [x] Write implementation comments only for the projection math that would be
      easy to get wrong.

### 1.3 Multi-State Bounds

- [x] Add a helper to compute union bounds for multiple possible transforms of
      the same node/import.
- [x] The helper must support connector replay use cases:
      - [x] before transform
      - [x] after/mated transform
      - [x] optional intermediate transform samples if interpolation curves
            require them
- [x] Non-goal for the first implementation: automatic skinned-animation,
      morph-target, or full clip-sampled bounds. Add a follow-up before
      claiming `bounds_for_transforms()` covers animated clips.
- [x] The helper must be generic, not connector-specific.
- [x] Candidate API:

```rust
let replay_bounds = scene.bounds_for_transforms(
    drive_root,
    &[before_transform, after_transform],
    &assets,
)?;
let total_bounds = replay_bounds.union(load.bounds_world(&scene)?);
```

- [x] Include imported mesh bounds, direct mesh bounds, and instance bounds if
      already supported by existing bounds APIs.
- [x] Return structured errors for missing bounds instead of silently ignoring
      renderable content.

### 1.4 World-To-Screen Projection

- [x] Expose a public projection helper so demos and applications do not
      duplicate private renderer camera math.
- [x] Candidate API:

```rust
let point = scene.project_world_point(camera, world_point, width, height)?;
```

- [x] Return `None` or a structured enum for points behind the camera/outside
      depth range.
- [x] Use the same math as rendering camera projection.
- [x] Add tests proving browser connector labels can be positioned from real
      connector world points without fixed CSS percentages.

## 2. Easy Scene Setup Primitives

### 2.1 Studio Lighting

- [x] Keep `Scene::add_studio_lighting()` as a reusable helper, not demo-only
      code.
- [x] Document the intended use: product/model-viewer lighting, not physically
      authored scene lighting replacement.
- [x] Treat studio lighting as a broad, balanced default, not a single harsh
      spotlight.
- [x] Ensure the helper creates:
      - [x] key directional light
      - [x] fill light
      - [x] rim/back light if still justified by visual tests
      - [x] key-only shadows unless tests prove another setup is needed
- [x] Keep intensity values tested and documented.
- [x] Add or keep tests that prevent overdriven one-spot lighting and blown
      highlights.
- [x] Ensure auto exposure works with the helper instead of requiring manual
      EV tuning.

### 2.2 Grid/Floor Helper

- [x] Add or promote a reusable floor/grid helper.
- [x] Candidate API:

```rust
scene.add_grid_floor(
    &assets,
    GridFloorOptions::new()
        .under_bounds(bounds)
        .color(Color::from_srgb_u8(54, 59, 69))
        .line_color(Color::from_srgb_u8(69, 75, 87))
        .roughness(0.96),
)?;
```

- [x] Floor must be placed at a known plane, usually `y = 0`, without pushing
      model bases below the floor.
- [x] Grid lines must stay on the floor plane.
- [x] Floor size must derive from bounds and padding, not manual demo constants.
- [x] Add an option to limit/fade grid extent so it does not read as a wall or
      infinite background.
- [x] Floor material must be matte by default and must not mirror the asset.
- [x] Add browser screenshot proof that the floor grounds the object without
      becoming the visual subject.

### 2.3 Auto Exposure

- [x] Keep renderer-managed auto exposure as a library feature, not demo-only
      sampling code.
- [x] Document it as "auto exposure" or "automatic exposure", not vague
      "auto tuning the light". Exposure adapts camera/output brightness;
      lighting still controls scene shape and contrast.
- [x] Document the split:
      - [x] auto exposure prevents global too-dark/too-bright frames
      - [x] lighting/materials still determine dynamic range and visual style
- [x] Add examples showing default auto exposure with studio lighting and HDRI.
- [x] Ensure browser surfaces can enable the same renderer-managed path as
      headless/native surfaces.

### 2.4 Connector Ergonomics

- [x] Keep authored connector mating as a first-class easy-use workflow.
- [x] Documentation must show `scene.mate(&drive, "shaft", &load, "hub")?`
      without raw matrix math.
- [x] Examples must include mismatched authoring conventions:
      - [x] `drive_unit` Y-up, millimeters
      - [x] `load_unit` Z-up, meters
- [x] Connector labels/markers in demos must be driven from real connector
      world positions, not static screen percentages.
- [x] Add projection/browser proof that connector labels move correctly after
      orbit/zoom and during replay.

## 3. Test-First Contract

The screenshot/projection tests are the spec. Do not implement production
framing code before adding failing tests.

### 3.1 Non-GPU Unit/Integration Tests

These tests do not render pixels and do not require a GPU. They cover
projection math and camera-state contracts directly: computed camera state,
AABB corner projection, screen-space rectangles, lifecycle behavior, controller
pivot adoption, and near/far policy.

- [x] Add a failing projection-only test proving bounding-sphere framing
      under-fills or clips a wide object on portrait aspect.
- [x] Add a failing test for a wide AABB in a desktop aspect.
- [x] Add a failing test for a tall AABB in a portrait/mobile aspect.
- [x] Add a failing test for an off-center AABB requiring target offset, not
      only distance changes.
- [x] Add a failing test proving projected min/max coordinates stay within the
      viewport even when the old demo would clip left while still appearing
      zoomed out.
- [x] Add a failing test that invalid `view_direction` returns a structured
      error.
- [x] Add a failing test that missing/empty bounds returns a structured error.
- [x] Add a failing test that framing mutates camera transform and marks scene
      dirty but does not call renderer prepare/render.
- [x] Add a failing test that framing before first `prepare()` / `render()`
      succeeds.
- [x] Add a failing test that orbit/camera controller state adopts the
      `FramingOutcome::target` as its pivot.
- [x] Add a failing test that near/far tightening is skipped or padded for
      reflective/transmissive/environment-dependent content.

### 3.2 Visual/Screenshot Tests

These tests render pixels through headless/browser paths. They verify the
projection math survives the actual rendering pipeline, material behavior, and
backend tolerances.

- [x] Add deterministic rendered-output proof for `frame_bounds()`.
- [x] Required assertions:
      - [x] object is not clipped
      - [x] object is not tiny
      - [x] object projected center is within tolerance of viewport center
      - [x] object occupies requested fill range, e.g. `0.65..0.75`
      - [x] desktop landscape passes
      - [x] mobile/portrait passes
- [x] Use documented per-backend tolerances; do not claim pixel-perfect output.
- [x] Capture failure artifacts when the test fails:
      - [x] rendered image
      - [x] projected screen rect
      - [x] target fill
      - [x] computed distance
      - [x] viewport size/aspect
- [x] Add a regression image for the connector before+after union bounds.
- [x] Add a WaterBottle or equivalent reflective PBR control screenshot proving
      camera near/far handling does not remove environment/IBL contribution or
      regress headline material quality.

### 3.3 Browser Demo Proof

- [x] Extend `scripts/probe_cloudflare_demo.js` or add a focused probe section
      proving the public demo no longer uses hand-tuned camera constants for
      connector framing.
- [x] Probe must assert:
      - [x] connector default before-state is visible and not clipped
      - [x] connector after-state is visible and not clipped
      - [x] model is not tiny
      - [x] public status has no frame-counter text
      - [x] connector labels are projected and visible
      - [x] labels move after orbit/zoom
      - [x] diagnostics stay collapsed
      - [x] no console errors
- [x] Capture desktop screenshots:
      - [x] connector before
      - [x] connector mid-replay
      - [x] connector after
      - [x] orbited connector after
      - [x] WaterBottle control
- [x] Capture mobile screenshot:
      - [x] connector before, no clipping, no overlap

### 3.4 Doctor Coverage

- [x] Add a doctor rule if demo/source drift can be detected mechanically.
- [x] Candidate doctor checks:
      - [x] public demo must not contain connector-specific camera distance
            constants once `frame_bounds()` is available
      - [x] public demo must use the library framing helper for connector mode
      - [x] public demo must keep diagnostics collapsed by default
      - [x] public demo must not expose frame-counter text outside diagnostics
      - [x] `docs/guides/easy-scene-setup.md` must exist
      - [x] the easy scene setup guide must contain runnable snippets for
            `frame_bounds`, `add_studio_lighting`, `add_grid_floor`, and
            renderer-managed auto exposure
      - [x] README/docs navigation must link the easy scene setup guide
- [x] If a doctor rule is not feasible, record the reason in this checklist.

## 4. Demo Adoption

- [x] Do not begin demo migration until the library primitive has focused green
      tests. The demo must consume the library API, not duplicate it.
- [x] Replace connector demo hand-tuned camera target/distance constants with
      `frame_bounds()`.
- [x] Compute connector framing bounds from the union of:
      - [x] load unit bounds
      - [x] drive unit before transform bounds
      - [x] drive unit after/mated transform bounds
      - [x] grid/floor bounds only if the floor must affect composition
- [x] If the replay path arcs through positions outside the before/after AABB
      union, such as rotation or curved interpolation, include sampled
      intermediate transforms per §1.3. The connector mid-replay screenshot in
      §3.3 is the browser-visible check for this failure mode.
- [x] Keep connector before-state separated along the demo's chosen mate-story
      axis and legible.
- [x] Do not use camera target bias constants to paper over clipping.
- [x] Do not reduce connector separation just to make broken framing pass.
- [x] Keep `shaft` and `hub` labels projected from connector world points.
- [x] Keep `scene.mate(...)` code-line highlight on replay.
- [x] Keep public status text free from frame counters.
- [x] Keep frame counters under collapsed diagnostics only.
- [x] Keep grid/floor construction based on bounds and options, not hand-picked
      floor slab constants.
- [x] Keep the model dominant in the hero canvas:
      - [x] no clipping
      - [x] no tiny object in a large empty floor
      - [x] no grid lines climbing into the sky/background
      - [x] before and after states both remain readable
- [x] Re-run every page in the browser probe after adopting the library helper.
- [x] Manually inspect screenshots before asking for user visual approval.

## 5. Documentation Updates

### 5.1 New "Easy Scene Setup" Chapter

- [x] Add a docs chapter dedicated to easy-use workflows, e.g.
      `docs/guides/easy-scene-setup.md`.
- [x] Make this chapter a first-class docs entry, not a hidden appendix.
- [x] Chapter must include:
      - [x] load a GLB/GLTF
      - [x] instantiate it
      - [x] add a camera
      - [x] frame bounds automatically
      - [x] add studio lighting
      - [x] add a matte grid/floor
      - [x] enable/describe auto exposure
      - [x] orbit controls
      - [x] connect authored connectors
      - [x] project labels/helpers to screen
      - [x] prepare/render lifecycle
- [x] The chapter must include a complete minimal example.
- [x] The chapter must include a "good defaults" example:
      - [x] `add_studio_lighting()`
      - [x] renderer-managed auto exposure
      - [x] real HDR/environment setup where applicable
      - [x] matte grid/floor helper
      - [x] `frame_bounds()`
- [x] The chapter must include a connector example:

```rust
let drive = scene.instantiate(&drive_part)?;
let load = scene.instantiate(&load_part)?;
scene.mate(&drive, "shaft", &load, "hub")?;
```

- [x] The chapter must explain why no user-entered coordinates or raw matrices
      are needed for connector mating.
- [x] The chapter must explain what auto-framing does and does not do.
- [x] The chapter must explain that lighting/materials are still real scene
      inputs, but `add_studio_lighting()` gives a strong default.
- [x] The chapter must include troubleshooting notes for:
      - [x] object tiny in viewport
      - [x] object clipped despite being zoomed out
      - [x] floor/grid appears behind the model like a wall
      - [x] labels detached from geometry
      - [x] bright/flat render caused by material or exposure mistakes

### 5.2 README Updates

- [x] Update README "Happy Path" so the first model-viewer workflow uses the
      easy-use primitives.
- [x] Add a short "Easy scene setup" section near the top-level promise.
- [x] Update capability table to mention:
      - [x] projection-based camera framing
      - [x] studio lighting helper
      - [x] grid/floor helper
      - [x] renderer-managed auto exposure
      - [x] authored connector mating
- [x] Keep examples factual and runnable.

### 5.3 Existing Docs

- [x] Update `docs/rendering.md` camera section with `frame_bounds()`.
- [x] Update `docs/rendering.md` lighting section with `add_studio_lighting()`.
- [x] Update `docs/rendering.md` environment/auto-exposure section.
- [x] Update `docs/api.md` public type list and common calls.
- [x] Create or update `docs/guides/migrating-from-threejs.md` with the
      equivalent of Three.js `Box3` + camera fitting + OrbitControls.
- [x] Create or update `docs/guides/place-and-connect-objects.md` to cross-link
      the easy setup chapter.
- [x] Add or update rustdoc examples for public helper APIs.
- [x] Ensure docs do not claim behavior before tests and examples prove it.

## 6. Examples

- [x] Update `examples/camera_framing.rs` to use the new primitive.
- [x] Add or update an easy model-viewer example:
      - [x] load glTF/GLB
      - [x] frame automatically
      - [x] add studio lighting
      - [x] add grid floor
      - [x] enable renderer-managed auto exposure
      - [x] render
- [x] Add or update connector auto-framing example:
      - [x] instantiate drive/load
      - [x] mate connectors
      - [x] compute before+after union bounds
      - [x] frame automatically
      - [x] render screenshot proof
- [x] Ensure every public example compiles.
- [x] Add visual proof for new examples where relevant.

## 7. Release And API Hygiene

- [x] Decide whether this is release-notable; expected answer: yes.
- [x] Treat the public API additions as a minor release. Current baseline is
      `1.2.0`; expected target is `1.3.0` unless the live release plan changes.
- [x] Add changelog entry under `Unreleased` if `CHANGELOG.md` exists.
- [x] Add `docs/release-notes/v1.3.0.md` or the chosen minor-version release
      notes file with the easy-scene setup APIs, examples, and browser proof.
- [x] Run a live pin-site sweep for the chosen version before release. At
      minimum inspect and update:
      - [x] `Cargo.toml`
      - [x] `Cargo.lock`
      - [x] `docs/api.md`
      - [x] `docs/README.md`
      - [x] `docs/release-notes/`
      - [x] `crates/xtask/src/app/doctor_scene_platform/release_contracts.rs`
      - [x] `crates/xtask/src/app/doctor_visual_release/ci_release_lanes.rs`
      - [x] `crates/xtask/src/app/release/lane_artifacts.rs`
      - [x] `crates/xtask/src/app/tests_*.rs`
      - [x] README/getting-started/feature-flag dependency snippets where they
            intentionally track the current minor release
- [x] Update public API baseline if the M5 baseline requires it.
- [x] Add rustdoc for all new public types/methods.
- [x] Avoid naming that sounds demo-only:
      - [x] prefer `frame_bounds`, `FramingOptions`, `GridFloorOptions`
      - [x] avoid `connector_hero_camera_frame` as public API
- [x] Keep public API small and composable.
- [x] Do not expose internals from `render` just to make the demo work.
- [x] Do not make platform/browser code own generic framing logic.

## 8. Required Gates

Focused gates:

- [x] Run focused red tests before implementation and record expected failures.
- [x] Run focused green tests after implementation.
- [x] Run visual/screenshot framing tests.
- [x] Run browser demo probe.

Repo gates:

- [x] `cargo fmt --check`
- [x] `cargo clippy --all-targets -- -D warnings`
- [x] `cargo test`
- [x] `cargo run -p xtask -- doctor --full`
- [x] `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`

Browser/demo gates:

- [x] `npm run demo:build`
- [x] `node scripts/probe_cloudflare_demo.js http://127.0.0.1:<port>/index.html`
- [x] desktop screenshot review
- [x] mobile screenshot review
- [x] connector before/mid/after replay screenshot review
- [x] WaterBottle/Khronos control screenshot review
- [x] console check: no red errors

## 9. Completion Criteria

- [x] A user can load a model and get a good first view without hand-tuned
      camera constants.
- [x] `frame_bounds()` passes projection/screenshot tests on landscape and
      portrait viewports.
- [x] Connector demo uses library framing, not demo-specific camera constants.
- [x] Connector before and after states are both visible and not clipped.
- [x] The model is not tiny in the hero canvas.
- [x] The grid/floor grounds the model without becoming the subject or climbing
      into the background.
- [x] Labels are projected from real world points and remain correct after
      orbit/zoom.
- [x] Docs include a dedicated easy-use chapter.
- [x] README points users to the easy-use workflow.
- [x] Examples compile and at least one easy-use example has visual proof.
- [ ] User visually approves the updated demo after our own screenshot review.

## 10. Explicit Non-Completion Conditions

This work is not done if any of these remain true. Checked items below mean the
condition was reviewed and is false in the current local proof.

- [x] False: the demo still depends on private camera distance/target constants
      for the connector hero shot.
- [x] False: the connector demo looks correct only for one desktop viewport.
- [x] False: the model can be clipped while the camera still appears zoomed out.
- [x] False: labels are positioned by static CSS percentages instead of
      projected world points.
- [x] False: the grid/floor works only because of demo-specific hard-coded
      dimensions.
- [x] False: the docs describe an easy workflow that examples do not compile
      and browser proof does not exercise.
