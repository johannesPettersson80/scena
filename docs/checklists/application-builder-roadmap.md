# Application builder roadmap

Status: proposed implementation checklist
Date: 2026-06-14

Scope: make `scena` easier to use as the visual, interaction, asset, browser,
diagnostic, and proof layer for model viewers, CAD-style inspection,
industrial visualization, product configurators, live-state viewers, and
documentation renderers.

This checklist is intentionally renderer-scoped. Host applications own domain
state, business rules, clocks, persistence, dynamic runtime behavior, CAD
kernels, industrial control logic, and undo/redo document models. `scena` owns
scene graph state, asset loading, camera/light/material state, interaction
primitives, renderer diagnostics, browser/native adapters, and deterministic
proof artifacts.

## Non-negotiable design rules

- [ ] Keep the public vocabulary aligned with `Scene`, `Assets`, `Renderer`,
      `SceneImport`, `prepare()`, and `render()`.
- [ ] Do not add catch-all `Engine`, `World`, `Manager`, or application-domain
      owner types.
- [ ] Do not hide asset fetch, shader compilation, GPU upload, or backend
      capability decisions inside `render()`.
- [ ] Keep browser and native host APIs semantically equivalent. Browser APIs
      may use JSON and CSS pixels, but they must not invent a separate product
      model.
- [ ] Use stable host `u64` handles in public wire contracts. Never serialize
      raw `NodeKey`, `CameraKey`, `MaterialHandle`, `GeometryHandle`, or other
      internal slotmap keys in external JSON.
- [ ] Every public JSON report or command contract uses
      `scena.<contract>.vN`. Additive v1 fields must use `serde(default)` or
      `Option` as appropriate and keep old-fixture deserialization green.
- [ ] Browser-visible rendering changes require rendered-output proof. Unit
      tests alone cannot close visual work.
- [ ] Every implementation item below needs a focused red test or a documented
      deterministic-proof exception before production code changes.
- [ ] Add or extend `xtask doctor` when a new failure family can be detected
      mechanically from source, docs, manifests, fixtures, or gate artifacts.

Feature-gated contract suites must be run explicitly so default `cargo test`
cannot hide them as zero-test successes:

- `cargo test --features inspection --test capture_contracts`
- `cargo test --features inspection --test render_introspection_contracts`
- `cargo test --features inspection --test visibility_diagnosis_contracts`
- `cargo test --features inspection --test visual_repair_contracts`
- `cargo test --features inspection --test appearance_introspection_contracts`
- `cargo test --features inspection --test scena_cli_agent`
- `cargo test --features inspection --test scena_cli_recipe`

## Explicit non-goals

- [ ] No CAD kernel, parametric sketcher, constraint solver, boolean modeling,
      STEP/IGES implementation, or geometry document format.
- [ ] No physics, rigid-body collision, mesh-clearance solver, machine planning,
      industrial process runtime, closed-loop semantics, or dynamic runtime
      loop.
- [ ] No gameplay ECS, networking, audio system, asset-management database, or
      application business rules.
- [ ] No undo/redo stack or host document model. Scena may emit reversible
      visual deltas, but the host owns document history.
- [ ] No hidden requestAnimationFrame loop for scene state. The host owns
      cadence and calls mutation, time advancement, `prepare()`, and `render()`.

## Phase 0 - Host contract foundation

Goal: define one symmetric boundary for host-to-scena visual changes and
scena-to-host events, then make visual proof export a first-class workflow.

### 0.1 Visual patch API

Owner modules: `scene_host`, `scene`, `animation`, `controls`, `diagnostics`.

Foothold: `SceneHostCore::set_transform_eased`,
`set_transforms_eased`, `set_node_tint`, `set_node_tint_eased`,
`set_visible`, `set_camera`, animation playback, material variants, labels,
and stable host handles already exist as separate setters.

Proposed contracts:

- `scena.visual_patch.v1`
- `VisualPatchV1`
- `VisualPatchResultV1`

Implementation slices:

- [x] `0.1A` minimal envelope: schema/version, stable handles, per-entry
      error/result model, changed counts, revision deltas, `transforms`,
      `tints`, `visibility`, and `camera`. This is the first deliverable.
- [x] `0.1B` additive easing/time channels: `transforms_eased`,
      `tints_eased`, `camera_eased`, and `animation_time`.
- [x] `0.1C` additive app/UI channels: `selection`, `hover`,
      `material_variants`, `labels`, and `metadata`.
- [x] All `0.1B` fields are optional or `serde(default)` in v1 and
      old-fixture deserialization remains green.
- [x] All `0.1C` fields are optional or `serde(default)` in v1 and
      old-fixture deserialization remains green when those channels land.

Native API sketch:

```rust
let result = host.apply_patch(&VisualPatchV1 {
    // 0.1A envelope channels.
    transforms: vec![...],
    tints: vec![...],
    visibility: vec![...],
    camera: Some(...),
    // 0.1B additive easing/time channels.
    transforms_eased: vec![...],
    tints_eased: vec![...],
    camera_eased: Some(...),
    animation_time: vec![...],
    // 0.1C additive app/UI channels.
    selection: Some(...),
    hover: Some(...),
    material_variants: vec![...],
    labels: vec![...],
    ..Default::default()
})?;
```

WASM API sketch:

```js
const result = JSON.parse(host.applyPatch(JSON.stringify(patch)));
```

Required fields and behavior:

- [x] `schema: "scena.visual_patch.v1"`.
- [x] Patch entries reference stable `SceneHost` handles.
- [x] `transforms`: immediate node transforms.
- [x] `transforms_eased`: target transforms, duration seconds, easing.
- [x] `tints`: set or clear node tint.
- [x] `tints_eased`: set or clear tint over host-ticked time.
- [x] `visibility`: set a node's visible flag. Hidden parents still affect
      rendered subtree visibility through the existing scene hierarchy.
- [x] `camera`: set camera immediately.
- [x] `camera_eased`: target camera state, duration seconds, easing.
- [x] `animation_time`: set or advance imported clip mixers explicitly.
- [x] `selection`: set, clear, or replace selection state.
- [x] `hover`: set or clear programmatic hover styling/state supplied by the
      host. Pointer-driven hover observations are emitted as `HostEventV1`
      events in 0.2.
- [x] `material_variants`: apply a named source material variant to an import.
- [x] `labels`: create, update, or remove host-owned label/annotation anchors
      where the host supplies stable IDs. Existing label/annotation convenience
      setters and the 2.7/2.8 annotation work delegate to this channel when
      exposed through the host contract; host applications own visible text.
- [x] `metadata`: optional caller-owned object preserved in result logs only
      when explicitly requested.
- [x] Invalid/stale handles fail with structured per-entry errors; no partial
      silent skips.
- [x] One batch reports one result with changed counts, failed entries, and
      revision deltas.
- [x] A patch with no effective changes is valid and reports no changed
      revisions.

Acceptance:

- [x] `0.1A` native test: one patch updates transform, tint, visibility, and
      camera using the same stable handles that inspection reports.
- [x] `0.1B` additive-channel tests: each field can be omitted from old
      fixtures and reports structured per-entry errors when present with
      invalid handles or values.
- [x] `0.1C` additive-channel tests: each field can be omitted from old
      fixtures and reports structured per-entry errors when present with
      invalid handles or values.
- [x] WASM build test: `applyPatch` compiles with `scene-host`.
- [ ] Browser proof: patch-driven transform/tint/visibility/camera changes are
      visible and inspectable for `0.1A`; additive-channel browser proof lands
      with the slice that introduces each channel.
- [x] Stable fixture:
      `tests/assets/stable-contracts/visual_patch.v1.json`.
- [x] Old-fixture deserialization test for additive optional fields.
- [x] Doctor rule: docs, schema constant, fixture, and feature flag references
      stay aligned.

### 0.2 Host event API

Owner modules: `scene_host`, `picking`, `assets`, `render`, `diagnostics`,
`platform`.

Foothold: picking, hover/selection state, load progress, diagnostics JSON,
surface/context/device-loss vocabulary, browser proof capture, and custom
element events already exist in separate places.

Proposed contract:

- `scena.host_event.v1`
- `HostEventV1`
- `HostEventBatchV1`

Native API sketch:

```rust
host.set_event_sink(|event: HostEventV1| {
    // application owns side effects
});
let events = host.drain_events();
```

WASM API sketch:

```js
for (const event of JSON.parse(host.drainEventsJson()).events) {
  handleScenaEvent(event);
}
```

Event kinds:

- [x] `pick`: CSS pixel coordinates, hit result, stable node handle, distance,
      and button/modifier metadata where available.
- [x] `hover`: pointer-driven entered, moved, left observations, stable handle
      if any. Programmatic hover styling/state is host-to-scena patch input in
      0.1.
- [x] `selection_changed`: previous and current stable handles.
- [x] `load_progress`: path, bytes or stage where known, progress kind, status.
- [x] `asset_loaded`: import handle, nested `scena.asset_load_report.v1`.
- [x] `diagnostic`: structured diagnostic code, severity, message, help.
- [x] `capture_ready`: capture descriptor metadata, payload length/hash when
      payload is returned out of band.
- [x] `surface_resized`: CSS pixels, physical pixels, DPR.
- [x] `context_lost` / `context_restored`.
- [x] `device_lost`; `device_recovered` is represented in
      `HostEventV1`/the stable fixture and becomes runtime-active when the
      platform exposes a recovered-device signal.
- [x] `capability_changed`: backend or capability report changed after surface
      recovery.

Acceptance:

- [x] Native and browser events use the same schema and event kind names.
- [x] Browser coordinates are CSS pixels unless a field explicitly says
      physical pixels.
- [x] Removed handles never alias recycled scene nodes in event payloads.
- [x] Event batches are drainable without requiring a hidden render loop.
- [x] Stable fixture:
      `tests/assets/stable-contracts/host_event.v1.json`.
- [ ] Browser proof asserts pick, hover, load progress, diagnostic, and context
      event shapes.

### 0.3 Capture and proof kit

Owner modules: `render`, `capture`, `scene_host`, `viewer`, `diagnostics`,
test/proof harness.

Foothold: `CaptureRgba8`, `capture_rgba8`, `Renderer::capture_rgba8`, viewer
`capture_png` and `capture_png_bytes`, visual regression helpers, and
gate-artifacts already exist.

API additions:

- [x] `CaptureRgba8::to_png_bytes() -> Result<Vec<u8>, CapturePngError>`.
- [x] `CaptureRgba8::write_png(path)`.
- [x] `Renderer::capture_png_bytes(scene, options)`.
- [x] `Renderer::capture_png(scene, options, path)` on native targets.
- [x] `SceneHostCore::capture_png_bytes()`.
- [x] WASM helper returning `Uint8Array` or a JS object with PNG bytes plus
      descriptor JSON.
- [x] Contact-sheet helper for proof artifacts.
- [x] Baseline comparison helper that records threshold, backend, capability
      report, and image metadata.

Acceptance:

- [x] PNG bytes decode as RGBA8 with the same dimensions as
      `CaptureDescriptor`.
- [x] Existing viewer PNG APIs delegate to the shared implementation.
- [x] `scena.capture.v1` remains the metadata source of truth.
- [ ] Browser proof captures PNG and descriptor metadata without relying on
      canvas `toDataURL` as the only path.
- [x] Example: headless documentation renderer writes PNG plus metadata.

### 0.4 External asset typed diagnostics

Owner modules: `assets`, `assets/gltf`, `diagnostics`, `scene_host`,
browser proof harness.

Foothold: `AssetLoadReport`, `AssetLoadWarning`, strict texture behavior,
provenance, material texture diagnostics, and stable asset-load schemas exist.

Required additions:

- [x] Typed missing external-buffer warning/error.
- [x] Preserve missing external-image warnings through cache hits.
- [x] Preserve material-fallback provenance in asset load reports.
- [x] Preserve material-fallback provenance in inspection reports.
- [x] Record generated material override vs source material path explicitly.
- [x] Record fetched/missing/skipped external buffers/images and status in
      `scena.asset_load_report.v1`.
- [x] Surface texture decode failures and fallback texture usage as structured
      diagnostics and report fields.

Acceptance:

- [x] Native tests cover missing buffer, missing image, missing decoded pixels,
      and cache-hit warning retention.
- [ ] Browser proof records all external resources for a `.gltf` with `.bin`
      and texture files.
- [x] Stable `scena.asset_load_report.v1` additions are optional/defaulted.
- [ ] Asset doctor and SceneHost reports expose the same diagnostic family.
      SceneHost report parity is covered for asset reports and inspection
      material evidence; the asset-doctor runtime surface remains open.

## Agent-facing surface track

Goal: expose Scena's normal render, capture, inspection, and diagnostics path
as stable JSON plus artifact paths so shell-driven tooling can run an
act-see-diagnose-correct loop without compiling Rust or relying on subjective
image review.

Governing rules:

- [ ] JSON contracts are the keystone. CLI commands serve those contracts; they
      do not define a second schema family.
- [ ] CLI output is stable `scena.*.vN` JSON to stdout plus artifact paths.
      Human text, if any, goes to stderr.
- [ ] There is no MCP server in this roadmap. The terminal surface is the CLI
      plus stable JSON.
- [ ] No agent-specific render mode. Introspection uses the same
      `prepare()`/`render()`/capture path as human-facing examples and
      applications.
- [ ] Reports are deterministic and token-small by default: stable field order,
      stable item ordering, rounded floats, summary-first shape, and opt-in
      detail for per-node or per-pixel-heavy data.
- [ ] Contract JSON is byte-stable for the same scene, backend class, options,
      and artifact inputs. Any nondeterministic field must be excluded from the
      stable report or isolated under explicit metadata.
- [ ] Shell failure is fail-closed. Commands that render but produce an
      unacceptable frame return structured JSON and a non-zero exit status.
- [ ] Repair loops have an iteration budget. Non-converging cases return a
      structured irreducible result with confidence, reason, and
      `auto_fixable: false` rather than forcing a visual success.
- [ ] Recipes are transient scene snapshots, not persisted documents or
      workflow scripts. No sequences, loops, branching, or hidden time
      ownership.
- [ ] Placement verbs compute one-shot transforms from authored features such
      as anchors, connectors, bounds, or authored planes. They do not infer mesh
      features, solve constraints, compute clearance, or introduce physics.

Minimum loops:

- [ ] Construction loop: emit `scene_recipe.v1` or `visual_patch.v1`, validate,
      render with introspection, diagnose visibility on failure, apply a
      reported fix within budget, rerender, and stop only on `ok=true` or an
      irreducible result.
- [ ] Appearance loop: for material, variant, status-color, or data-color work,
      assert the rendered node/material appearance against intended values, not
      only that pixels changed.
- [ ] Temporal loop: for animation, transitions, or live-state playback, sample
      requested times and assert channel values, visible motion, and no frozen
      or NaN channel.
- [ ] Interaction loop: inject synthetic pointer or keyboard input, then assert
      expected pick/hover/selection events and rendered feedback.

### A.1 Render introspection contract

Owner modules: `render`, `capture`, `scene`, `scene_host`, `diagnostics`,
proof harness.

Foothold: `CaptureRgba8`, `capture_rgba8`, `Renderer::capture_rgba8`,
`SceneHostCore::capture`, renderer stats including `culled_objects`,
auto-exposure luminance measurement, scene inspection JSON, and visual proof
helpers already exist.

Proposed contract:

- `scena.render_introspection.v1`

CLI-shaped API sketch:

```bash
scena render scene.recipe.json --introspect --out target/scena-agent/frame.png
```

Required fields and behavior:

- [x] `schema: "scena.render_introspection.v1"`.
- [x] `ok`: false for implemented error-severity failure families: empty,
      no-visible-drawable, and all-culled frames. Warning-only framing reasons
      such as tiny-in-frame and cropped are reported without failing the agent
      loop.
- [ ] `ok`: false for behind-camera, outside-frustum, alpha-zero, NaN-specific,
      and other visually unacceptable render states. These remain for
      A.2/A.5/A.7 follow-up diagnostics.
- [x] `reasons[]`: stable codes, severity, affected handles where known, and
      short messages.
- [x] `fixes[]`: ranked suggested actions in Scena vocabulary, such as
      `frame_bounds`, `set_visible`, `set_alpha`, `clear_section_box`, or
      `set_transform`. The first slice emits `frame_bounds` and `set_visible`;
      later diagnoser/repair slices add the material, clipping, and transform
      actions.
- [x] `content_bbox_css_px` and `content_bbox_fraction`: screen-space
      non-background content bounds, computed against the configured
      shader-encoded background with byte tolerance rather than literal black.
- [x] `visible_pixel_fraction`: non-background pixel fraction after configured
      background handling.
- [x] `luminance`: min, max, mean, and selected percentiles on the 0-255
      shader-encoded RGBA8 scale, rounded to stable precision.
- [x] `framing`: center offset, fit fraction, crop flags, tiny-in-frame flag,
      and active camera handle.
- [x] `nodes_summary`: counts for visible, hidden, culled, clipped,
      transparent, failed-material, and unknown-coverage nodes.
- [x] `nodes_detail`: optional detail mode only; stable handle, name if
      available, projected bbox where known, coverage category, and reason
      codes. The first slice reports stable handles, kind, visibility, and
      conservative coverage categories; projected bbox detail remains open.
- [x] First-slice MVP fields are explicit: per-node pixel coverage is
      `unknown` for visible nodes, `unknown_coverage` counts those nodes,
      `clipped` and `transparent` are reserved not-yet-computed counters, and
      render-level fixes may omit `target_handle` / `patch`.
- [x] `artifacts`: capture PNG path, capture descriptor path or inline summary,
      and optional contact-sheet path.
- [x] `capabilities`: active backend and capability summary, not a duplicate
      full capability report unless detail mode is requested.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/render_introspection.v1.json`.
- [x] Fixture/test frames cover empty background, all-culled, tiny-in-frame,
      cropped, and valid centered content.
- [ ] Fixture/test frames cover behind-camera, NaN transform, and alpha zero.
- [x] Headless tests prove `ok=false` for the implemented failure frames and
      `ok=true` for the valid frame.
- [x] Headless tests prove warning-only framing reports keep `ok=true`.
- [ ] Browser proof covers at least empty, offscreen, and valid centered
      content.
- [x] Report JSON is deterministic across two consecutive runs.
- [x] Doctor/stable-contract checks keep the schema string and stable fixture
      aligned. CLI help alignment lands with A.3.

### A.2 Visibility diagnoser

Owner modules: `diagnostics`, `scene`, `render`, `material`, `scene_host`.

Foothold: `Renderer::diagnose_scene`, renderer stats, scene inspection,
visibility/subtree APIs, clipping planes, material diagnostics, and structured
errors with help text exist.

Proposed contract:

- `scena.visibility_diagnosis.v1`

API sketch:

```rust
let report = renderer.diagnose_visibility(&scene, handle, camera)?;
```

CLI-shaped API sketch:

```bash
scena diagnose scene.recipe.json --handle 42 --visibility
```

Required behavior:

- [x] Diagnose a node or whole scene from stable inspection handles.
- [ ] Diagnose subtree and import targets.
- [x] Ranked reasons include not prepared, stale handle, missing camera,
      no visible drawables, node hidden, zero scale, and all culled.
- [ ] Ranked reasons include parent hidden, NaN transform, behind camera,
      outside frustum, clipped by active clipping planes, layer masked, alpha
      zero, transparent material, missing material upload, missing geometry,
      and backend capability degradation.
- [x] Each implemented actionable reason includes at least one suggested action
      in Scena vocabulary.
- [x] Suggestions are data, not prose-only: action code, target handle when
      known, optional patch snippet, and human help text.
- [x] Suggestions never mutate the scene directly. The host or CLI caller
      decides whether to apply them.
- [x] Summary mode returns only the ranked reasons and top fixes. Detail mode
      may include supporting projection, bounds, material, and clipping data.

Acceptance:

- [x] Unit tests for the implemented ranked reason families.
- [ ] Unit tests for parent hidden, NaN transform, behind camera,
      outside-frustum, clipping, layer mask, alpha zero, transparent material,
      missing material upload, missing geometry, and backend degradation.
- [x] Tests prove implemented fix suggestions are stable and use public Scena
      vocabulary.
- [ ] Whole-scene diagnosis agrees with render introspection for empty,
      all-culled, and behind-camera frames. The first slice covers all-culled
      and proves partial culling is not reported as all-culled.
- [x] Stable fixture:
      `tests/assets/stable-contracts/visibility_diagnosis.v1.json`.
- [ ] CLI exits non-zero when diagnosis classifies the requested visibility
      target as not visible.

### A.3 Agent-friendly CLI verbs

Owner modules: `bin/scena`, `render`, `scene_host`, `assets`, `diagnostics`,
`capture`, `viewer`.

Foothold: `xtask doctor`, `asset-doctor`, `SceneHostCore` JSON methods,
capture APIs, asset load reports, capability reports, and renderer stats
already expose the needed data in separate surfaces.

Required commands:

- [x] `scena render <asset-or-recipe> --introspect --out <png>`: load,
      prepare, render, capture, introspect, write artifacts, and emit
      `scena.render_introspection.v1`.
- [x] `scena inspect <asset-or-recipe>`: emit `scena.scene_inspection.v1`
      for loaded assets or the first recipe import. Handle/subtree/detail
      filters remain open.
- [x] `scena diagnose <asset-or-recipe> --visibility [--handle <u64>]`: emit
      `scena.visibility_diagnosis.v1`.
- [x] `scena validate-recipe <recipe.json>`: emit validation diagnostics for
      `scena.scene_recipe.v1`.
- [x] `scena place <recipe.json> --verb <verb> ...`: emit a transform or
      `VisualPatch` preview, not a mutated document.
- [x] `scena schema list` and `scena schema get <schema>`: emit schema and
      minimal examples for stable contracts.
- [x] `scena repair <recipe-or-patch> --from <diagnosis.json>`: emit a repair
      plan or irreducible loop result with explicit risk classification.
- [x] `scena verify appearance <recipe-or-asset> --expect <json>`: emit
      `scena.appearance_introspection.v1`.
- [ ] `scena verify animation <recipe-or-asset> --expect <json>`: emit
      `scena.animation_introspection.v1`.
- [ ] `scena verify interaction <recipe-or-asset> --expect <json>`: emit
      `scena.interaction_verification.v1`.
- [ ] `scena doctor <asset-or-recipe>`: expose asset-doctor-style findings
      through the same terminal command family.

CLI behavior:

- [x] JSON to stdout; progress and human text to stderr for the implemented
      schema, render, inspect, diagnose, and repair commands.
- [x] `--json` is the default for the implemented schema, render, inspect, and
      diagnose commands.
- [x] `--detail` opts into larger render-introspection and visibility-diagnosis
      reports.
- [ ] `--round-floats <digits>` defaults to stable, documented precision.
- [x] Exit status `0` means the requested schema operation succeeded.
- [x] Exit status is non-zero for invalid recipes, missing assets, failed
      preparation, failed rendering, `ok=false` introspection, and invisible
      diagnosis targets. Missing assets currently use the command-error path;
      recipe validation, report `ok=false`, and invisible diagnosis targets
      emit machine-readable JSON.
- [x] Artifact paths are explicit for implemented render-introspection output.

Acceptance:

- [ ] Golden stdout fixtures for each command.
- [x] Tests verify stderr/stdout separation for schema commands.
- [x] Tests verify `render --introspect` writes PNG plus capture descriptor
      artifacts and emits `scena.render_introspection.v1`.
- [x] Tests verify `inspect <asset>` emits `scena.scene_inspection.v1`.
- [x] Tests verify `diagnose --visibility --handle <stale>` emits
      `scena.visibility_diagnosis.v1` on stdout with a non-zero exit.
- [x] Tests verify `render --introspect` emits
      `scena.render_introspection.v1` on stdout with a non-zero exit for an
      empty frame.
- [x] Tests verify `repair --from` emits `scena.visual_repair_plan.v1` for a
      reversible diagnosis and emits `scena.agent_loop_result.v1` with a
      non-zero exit for an irreducible diagnosis.
- [x] Tests verify `verify appearance --expect` applies a declared material
      variant, emits `scena.appearance_introspection.v1`, and exits non-zero
      with JSON on stdout when the sampled rendered color does not match the
      expectation.
- [x] Tests verify non-zero exits for invalid recipe and invisible target.
- [ ] Tests verify missing assets emit JSON instead of command-error text.
- [ ] Doctor rule keeps CLI help, schema constants, and stable fixtures aligned.

### A.4 Schema discovery

Owner modules: `diagnostics`, `scene_host`, `assets`, `render`, `capture`,
`bin/scena`, docs.

Foothold: `docs/schema-contracts.md`, stable-contract fixtures, schema strings,
SceneHost JSON methods, and doctor schema/fixture alignment already exist.

Required behavior:

- [x] `scena schema list` emits every public `scena.*.vN` contract name, owner
      module, summary, feature flag if any, and fixture path.
- [x] `scena schema get <schema>` emits the machine-readable schema plus one
      minimal valid example from the stable fixture.
- [ ] `scena schema get <schema>` emits one representative invalid example
      when available.
- [x] Schema output is deterministic and small by default.
- [x] Unknown schema names return diagnostics with near-miss suggestions.
- [x] The schema list includes all currently landed stable fixtures, including
      visual patch, host event, capture, inspection, render introspection, and
      visibility diagnosis.
- [x] The schema list includes scene recipe, scene recipe validation,
      placement result, repair plan, and agent loop result contracts.
- [x] The schema list includes the landed appearance expectation and
      appearance introspection contracts.
- [ ] The schema list includes animation introspection and interaction
      verification contracts as they land.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/schema_catalog.v1.json`.
- [x] Tests prove every listed schema has a fixture or documented exception.
- [x] CLI `schema get` examples deserialize against their contract for the
      tested render-introspection schema.
- [ ] Doctor rule rejects schema names referenced in docs but missing from the
      schema catalog.

### A.5 Declarative scene recipe

Owner modules: `scene_host`, `scene`, `assets`, `material`, `geometry`,
`diagnostics`, `viewer`.

Foothold: stable handles, asset loading, primitives, labels, cameras, lights,
viewer profiles, capture descriptors, and validation diagnostics already exist
as separate APIs.

Proposed contract:

- `scena.scene_recipe.v1`

Scope:

- [x] A recipe is a declarative scene snapshot consumed by Scena.
- [x] The first recipe slice supports imports, per-import transforms and
      expected extents, caller metadata, and at most one optional capture
      directive.
- [ ] Future recipe sections may include primitive nodes, materials, cameras,
      lights, labels, viewer profile references, environment references, and
      feature-owned inspection/annotation helpers as those owner features land.
- [ ] A recipe may reference authored anchors, connectors, bounds, or authored
      planes for placement.
- [ ] Recipe sections are extensible with their owning features. Section boxes,
      measurement overlays, callouts/leader lines, exploded-view directives,
      and named-state references become recipe-expressible as the Phase 2
      features that own them land.
- [x] A recipe is not a project file, document model, workflow script, or
      persisted application state.

Required validation:

- [x] Unknown fields fail closed unless explicitly declared as caller metadata.
- [x] Unknown fields and duplicate caller IDs produce structured diagnostics
      with `help`.
- [x] Near-miss suggestions use a bounded string-distance check for supported
      field names, for example `importe` suggesting `imports`.
- [ ] Unknown verbs, enum variants, handles, materials, assets, or profiles
      produce structured diagnostics as those sections land.
- [ ] Units and scale sanity warnings detect extreme asset extents against an
      expected range when the recipe supplies one.
- [x] Recipe sections whose owning feature is not implemented fail with a
      structured `unsupported_feature` diagnostic instead of being silently
      ignored.
- [x] Validation can run without rendering.
- [x] Rendering a recipe uses the same load, prepare, render, and capture path
      as native examples.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/scene_recipe.v1.json`.
- [x] Stable validation fixture:
      `tests/assets/stable-contracts/scene_recipe_validation.v1.json`.
- [x] Validation tests cover unknown field, duplicate caller ID, and rejected
      workflow fields.
- [ ] Invalid-recipe fixtures cover unknown verb, unknown enum, stale handle,
      missing asset, invalid transform, and oversized asset as those sections
      land.
- [x] Validation diagnostics include deterministic "did you mean" suggestions.
- [x] A valid recipe renders through `scena render --introspect` and produces
      `ok=true`.
- [x] No sequence, loop, branch, timer, or hidden render-loop field is accepted.

### A.6 Semantic placement verbs

Owner modules: `scene`, `assets/gltf`, `scene_host`, `geometry`,
`diagnostics`.

Foothold: anchors, connectors, `connect_objects`, bounds, coordinate-system
repair, `world_distance`, and VisualPatch-compatible transform updates exist.

Required verbs:

- [x] `center`: center a node/import around a target point or bounds.
- [x] `ground`: place a node/import on a ground plane or authored support
      plane using bounds.
- [x] `fit_to_size`: scale uniformly into a requested size range.
- [x] `look_at`: orient a camera or node toward a point or bounds center.
- [x] `align_to_anchor`: align authored anchors/connectors and emit the
      resulting transform.
- [x] `place_on`: place one authored anchor, connector, bounds face, or
      authored plane onto another authored target.

Scope guards:

- [x] No inferred mesh-face detection. A "face" must be an authored plane,
      bounds face, anchor, or connector.
- [x] No continuous constraints, snapping loop, clearance computation,
      collision solving, or domain-specific placement policy.
- [x] Bounds-only recipe import verbs emit a transform preview. They do not
      mutate a host document or rewrite the recipe.
- [x] Anchor/connector verbs emit a transform preview. They do not mutate a
      host document directly.

Acceptance:

- [x] Unit tests for bounds-only recipe import `center`, `ground`, and
      `fit_to_size` with stable numeric tolerances.
- [x] Unit tests for `look_at`, `align_to_anchor`, and `place_on` with stable
      numeric tolerances.
- [x] Tests prove invalid authored-feature references fail with structured
      diagnostics and suggested alternatives where available.
- [x] CLI `scena place` emits deterministic JSON and non-zero exits for invalid
      import references.
- [x] CLI `scena place` emits deterministic JSON and non-zero exits for invalid
      authored anchor/connector/plane references.
- [ ] Visual proof shows `center`, `ground`, and `align_to_anchor` produce a
      visible, framed result when passed through render introspection.

### A.7 Safe visual repair

Owner modules: `diagnostics`, `scene_host`, `scene`, `render`, `material`,
`bin/scena`.

Foothold: visibility diagnosis, render introspection, VisualPatch, camera
framing, placement verbs, structured errors with help, and stable handles exist
or are introduced earlier in this track.

Proposed contracts:

- `scena.visual_repair_plan.v1`
- `scena.agent_loop_result.v1`

Required behavior:

- [x] Repair planning consumes render introspection and visibility diagnosis
      reports and emits a proposed `VisualPatch` or irreducible result. Recipe
      updates remain future feature-owned work.
- [x] Presentation repairs are non-destructive and may be applied freely in the
      first slice for framing/camera-oriented action codes such as
      `frame_bounds`.
- [x] Content repairs are risk-gated and never silent. The first slice emits a
      reversible visibility patch for `node_hidden` and skips unsafe transform
      or scale changes that require host input. Alpha, material override, and
      clipping repair families remain future work.
- [x] Repairs include `auto_fixable`, `confidence`, `risk`, `root_cause`,
      `applied_actions`, `skipped_actions`, `remaining_reasons`, and
      `requires_host_input`.
- [x] Repair never reports success by itself. The caller must rerender and
      re-introspect after applying any repair.
- [x] Non-convergence within the iteration budget emits
      `scena.agent_loop_result.v1` with `status: "irreducible"`.

Acceptance:

- [x] Tests cover presentation repair, content repair, skipped unsafe repair,
      and non-converging repair loop.
- [x] Content repair fixtures prove every landed content change is reversible and
      root-cause-traced.
- [x] CLI exits non-zero for irreducible reports and includes the structured
      reason on stdout.
- [x] Stable fixtures:
      `tests/assets/stable-contracts/visual_repair_plan.v1.json` and
      `tests/assets/stable-contracts/agent_loop_result.v1.json`.

### A.8 Appearance and material introspection

Owner modules: `render`, `material`, `assets/gltf`, `scene_host`,
`diagnostics`, proof harness.

Foothold: material descriptors, material variants, material fallback
provenance, asset load reports, source material import, capture readback,
luminance metrics, and visual proof helpers exist.

Proposed contract:

- `scena.appearance_expectation.v1`
- `scena.appearance_introspection.v1`

Required behavior:

- [x] Assert first-time appearance correctness without requiring a golden image.
- [x] Inputs declare intended appearance per stable node, tag/import-like
      selector, or variant: color family or target swatch, variant name, alpha
      mode, texture presence, and fallback policy. Material-name matching
      remains future work until stable material inspection exposes source
      names.
- [x] Report active source material, active variant, material factors,
      texture/fallback provenance, sampled region stats, dominant color family,
      alpha summary, and luminance summary.
- [x] `ok=false` when an intended variant is missing, a generated fallback is
      used where source material was required, alpha hides the target, sampled
      color family disagrees with the intended swatch, or texture provenance is
      missing.
- [x] Reports include suggested fixes, such as apply variant, load missing
      texture, clear fallback override, set alpha mode, or inspect material
      assignment.
- [x] Summary mode is small and includes capture-bound sampled frame content,
      swatch distance, alpha summary, material source, fallback provenance, and
      luminance mean. The first slice samples the visible frame-content region;
      per-node pixel coverage and glTF material-name checks remain future
      additive fields because current stable material inspection exposes source
      index/provenance but not material names or ID-buffer coverage.

Acceptance:

- [x] Fixtures and tests cover intended green, wrong color, generated fallback,
      missing variant, and valid source-material variant. Intended blue,
      missing texture, alpha zero, and source-material name checks remain
      future hardening cases.
- [x] Product-configurator proof asserts the requested variant rendered with
      the expected appearance, not merely that pixels changed.
- [ ] Data-color proof asserts a known color ramp sample without a golden
      image.
- [ ] Stable fixture:
      `tests/assets/stable-contracts/appearance_expectation.v1.json` and
      `tests/assets/stable-contracts/appearance_introspection.v1.json`.
- [x] CLI exits non-zero when appearance assertions fail.

### A.9 Animation and temporal introspection

Owner modules: `animation`, `scene_host`, `scene`, `render`, `diagnostics`,
proof harness.

Foothold: animation clips, SceneHost animation playback exposure,
host-ticked time advancement, presentation transitions, capture proof, and
scene inspection exist.

Proposed contract:

- `scena.animation_introspection.v1`

Required behavior:

- [ ] Sample requested times for imported clips, presentation transitions, or
      host-ticked visual state playback.
- [ ] Report clip name, channel count, sampled times, changed channel counts,
      unchanged channel counts, NaN/invalid channel counts, and visible-motion
      summary.
- [ ] Report selected transform/tint/camera/visibility values at each sample
      when expected values are supplied.
- [ ] `ok=false` when a required clip is missing, time does not advance,
      channels freeze unexpectedly, values become NaN, expected pose/state at
      time does not match tolerance, or visible output stays unchanged when
      motion was expected.
- [ ] No hidden time loop. The report records the explicit sampled times and
      host advancement calls used to produce it.

Acceptance:

- [ ] Fixtures cover valid motion, frozen channel, NaN transform, missing clip,
      wrong pose at time, and no visible motion.
- [ ] Live-state playback proof samples at least three times and verifies the
      expected visual state at each sample.
- [ ] Animated viewer proof verifies motion happened between samples and the
      final state matches expectation.
- [ ] Stable fixture:
      `tests/assets/stable-contracts/animation_introspection.v1.json`.
- [ ] CLI exits non-zero for temporal assertion failures.

### A.10 Synthetic interaction verification

Owner modules: `platform`, `viewer`, `viewer_element`, `picking`,
`scene_host`, `controls`, `diagnostics`, browser proof harness.

Foothold: picking, hover/selection state, HostEvent, browser proof harness,
viewer callbacks, interaction styles, and custom-element events exist or are
introduced earlier in the roadmap.

Proposed contract:

- `scena.interaction_verification.v1`

Required behavior:

- [ ] Inject synthetic pointer and keyboard input in native viewer tests and
      browser proof harnesses.
- [ ] Inputs use CSS pixels for browser targets and explicitly state physical
      pixel fields when needed.
- [ ] Assertions include expected pick handle, hover state, selection state,
      event sequence, modifier/button metadata, camera-control result, and
      rendered highlight or outline feedback where applicable.
- [ ] `ok=false` when the expected handle is not picked, event order differs,
      hover/selection state is missing, coordinates are interpreted in the
      wrong pixel space, or rendered feedback is absent.
- [ ] Reports include suggested fixes, such as use CSS pixels, frame target,
      enable picking, exclude helper geometry, or update expected handle.

Acceptance:

- [ ] Fixtures cover successful pick, miss, wrong handle, hover enter/leave,
      selection highlight, helper-geometry pass-through, and CSS-vs-physical
      pixel mismatch.
- [ ] Browser proof runs without manual mouse input.
- [ ] Native viewer proof runs without manual input devices.
- [ ] Stable fixture:
      `tests/assets/stable-contracts/interaction_verification.v1.json`.
- [ ] CLI exits non-zero when interaction assertions fail.

### A.11 Agent smoke templates

Owner modules: `bin/scena`, `examples`, `docs`, `scene_host`, `diagnostics`,
proof harness.

Foothold: examples, stable contract fixtures, visual proof artifacts, and
SceneHost host-loop examples exist.

Required templates:

- [ ] `scena examples agent product-configurator`.
- [ ] `scena examples agent live-state-viewer`.
- [ ] `scena examples agent web-viewer`.
- [ ] `scena examples agent data-visualization`.
- [ ] `scena examples agent animated-viewer`.
- [ ] `scena examples agent interaction-proof`.
- [ ] `scena examples agent cad-inspection`, landing incrementally with the
      Phase 2 inspection, measurement, section-box, exploded-view, and
      annotation features it exercises.
- [ ] `scena examples agent documentation-renderer`, landing incrementally
      with the Phase 2 measurement, callout/leader-line, annotation-layout,
      section-box, and exploded-view features it exercises.

Required behavior:

- [ ] Each template emits a recipe, expected assertions, CLI commands, and
      expected artifact paths.
- [ ] Agent-track-only templates can be run through CLI-only steps and produce
      `ok=true` reports for their relevant construction, appearance, temporal,
      or interaction checks when A.11 lands.
- [ ] Phase-2-dependent templates add CLI-only assertions as their owning
      recipe sections and visual helpers land; they must not be marked complete
      before those dependencies are implemented.
- [ ] Templates are examples and acceptance apps, not a hidden application
      framework.

Acceptance:

- [ ] CI or doctor verifies every template command remains documented and
      runnable or records a hardware-specific proof exception.
- [ ] Template outputs include stable JSON reports and capture artifacts.
- [ ] Failing fixture variants exist for each dynamic verification class.

## Phase 1 - Interactive viewer ergonomics

Goal: make the common interactive viewer/editor primitives easy without adding
application state ownership.

### 1.1 Camera fly-to and bookmarks

Owner modules: `controls`, `scene_host`, `scene`, `viewer`.

Foothold: `FramingOutcome`, `OrbitControls::focus_on_framing`,
`SceneHostCameraState`, `set_camera`, framing presets, and `SceneHostEasing`.

API sketch:

```rust
let bookmark = CameraBookmark::from_framing("pump_detail", framing);
host.set_camera_eased(bookmark.state(), 0.35, SceneHostEasing::EaseInOut)?;
```

Required behavior:

- [ ] `CameraBookmark { name, state, target_bounds, description }`.
- [ ] `OrbitControls::fly_to(state, easing, duration)`.
- [ ] `SceneHostCore::set_camera_eased`.
- [ ] WASM `setCameraEased(...)` and `setCameraBookmarkJson(...)`.
- [ ] Browser and native convenience APIs delegate to the 0.1B
      `camera_eased` `VisualPatch` channel instead of defining a parallel
      camera transition wire model.
- [ ] Optional bookmark list on viewer helpers.
- [ ] Camera transitions are advanced only by explicit host time advancement.

Acceptance:

- [ ] Camera interpolation keeps target, distance, yaw, and pitch finite.
- [ ] Zero duration applies immediately.
- [ ] Invalid camera states fail with structured errors.
- [ ] Browser proof shows a framed object remains visible during and after
      fly-to.

### 1.2 Transform gizmo and manipulator

Owner modules: `controls`, `picking`, `scene_host`, `geometry`.

Foothold: picking, interaction styles, technical strokes, camera projection,
and platform-neutral controls exist.

Feature flag: `gizmo` or under `controls` if dependency-free.

API sketch:

```rust
let mut gizmo = TransformGizmo::new(GizmoMode::Translate);
if let Some(delta) = gizmo.pointer_drag(&scene, camera, pointer, &assets)? {
    host.apply_patch(&delta.to_visual_patch(target_handle))?;
}
```

Required behavior:

- [ ] Modes: translate, rotate, scale.
- [ ] Coordinate spaces: world, local, view-aligned where implementable.
- [ ] Axis constraints and plane constraints.
- [ ] Hit testing uses existing picking/ray contracts.
- [ ] Gizmo emits transform deltas or `VisualPatch`; it does not mutate a host
      document directly.
- [ ] Gizmo visuals are helper geometry/strokes and can be hidden from normal
      scene picking except when active.

Scope guards:

- [ ] No undo/redo stack.
- [ ] No collision, constraints solver, snapping-to-mesh, or CAD kernel.
- [ ] No application selection model beyond emitting stable handles/events.

Acceptance:

- [ ] Unit tests for ray-to-axis and ray-to-plane math.
- [ ] Interaction tests for stale target handles.
- [ ] Browser proof for translate drag and rotation drag.
- [ ] Example: simple scene editor moves a selected part with the gizmo.

### 1.3 Viewer profiles

Owner modules: `viewer`, `scene`, `render`, `controls`, `assets`.

Foothold: viewer builders, studio lighting, environment presets, grid floor,
framing, auto exposure, controls, and diagnostics.

API sketch:

```rust
let profile = ViewerProfile::cad_inspection()
    .with_grid(true)
    .with_section_controls(true)
    .with_default_picking(true);
```

Profiles:

- [ ] `ViewerProfile::model_viewer()`.
- [ ] `ViewerProfile::cad_inspection()`.
- [ ] `ViewerProfile::product()`.
- [ ] `ViewerProfile::industrial()`.
- [ ] `ViewerProfile::documentation()`.

Acceptance:

- [ ] Profiles are composable builder presets, not separate viewer engines.
- [ ] Applying a profile does not call `prepare()` or `render()` implicitly.
- [ ] Each profile has a minimal example and a snapshot/proof artifact.
- [ ] Browser and native profile names match.

## Phase 2 - Inspection, CAD view, and annotation

Goal: build the technical viewer tools that CAD-style inspection,
industrial live-state visualization, and documentation need.

### 2.1 Inspection toolkit

Owner modules: `scene`, `scene_host`, `diagnostics`, `geometry`, `viewer`.

Foothold: scene inspection, node hierarchy, visibility, layers, node bounds,
tags, picking, hover/selection, and helper geometry.

Required features:

- [ ] Part tree from imported hierarchy and host handles.
- [ ] `isolate(selection)`: hide all unrelated visible nodes.
- [ ] `ghost(node/subtree, alpha)`: tint/alpha helper for context parts.
- [ ] `show_only`, `hide`, `show`, `toggle`.
- [ ] Fit selected node/subtree.
- [ ] Bounding-box helper overlays.
- [ ] Local/world axes triad widget.
- [ ] Selection set helpers by tag, import path, import name, or handle list.
- [ ] Inspection report records active isolate/ghost/helper state where useful.

Acceptance:

- [ ] Tests for isolate and restore preserving prior visibility.
- [ ] Tests for ghost not mutating source material descriptors.
- [ ] Browser proof for part-tree selection, isolate, ghost, and fit selection.
- [ ] Example: CAD inspection viewer.

### 2.2 Measurement primitives

Owner modules: `scene`, `geometry`, `scene_host`, `diagnostics`.

Foothold: `world_distance`, `node_world_bounds`, lines, labels, annotation
projection.

API sketch:

```rust
let measurement = MeasurementOverlay::distance(a, b)
    .with_units(UnitFormat::millimeters())
    .with_label("shaft offset");
scene.add_measurement_overlay(measurement)?;
```

Required features:

- [ ] Point-to-point distance.
- [ ] Angle between three points or two vectors.
- [ ] Axis-aligned dimension from bounds.
- [ ] Leader line with label.
- [ ] Unit-format hook supplied by the host.
- [ ] Optional screen-space label projection.
- [ ] Measurement report with source handles/points and rendered label ID.

Scope guard:

- [ ] Scena renders measurement visuals and computes simple geometric values.
      The host decides which points matter and owns semantic measurement rules.

Acceptance:

- [ ] Unit tests for distance, angle, and bounds dimensions.
- [ ] Visual proof for dimension line and projected label readability.
- [ ] Browser proof for selecting two points and rendering a distance overlay.

### 2.3 Section box helper

Owner modules: `scene`, `render`, `scene_host`.

Foothold: `ClippingPlane` and `ClippingPlaneSet`.

API sketch:

```rust
let section = SectionBox::from_bounds(bounds).with_margin(0.05);
scene.set_section_box(section)?;
```

Required behavior:

- [ ] Six clipping planes from an `Aabb`.
- [ ] Enable, disable, invert, and update section box.
- [ ] Optional helper wireframe box.
- [ ] Serialization through `VisualPatch` metadata or a later additive patch
      channel; no browser-only or dedicated parallel state model.

Acceptance:

- [ ] Tests verify six planes and stable clipping behavior.
- [ ] Browser proof shows cutaway on an imported asset.

### 2.4 Exploded view helper

Owner modules: `scene`, `scene_host`, `animation`.

Foothold: import hierarchy, bounds, transforms, anchors, and eased transforms.

API sketch:

```rust
let patch = ExplodedView::from_import(import)
    .by_hierarchy_depth()
    .factor(0.65)
    .to_visual_patch(&scene, &assets)?;
host.apply_patch(&patch)?;
```

Required behavior:

- [ ] Offset direct children from assembly center.
- [ ] Offset by hierarchy depth or selected axis.
- [ ] Support factor from `0.0` assembled to `1.0` exploded.
- [ ] Produce reversible transforms.
- [ ] Optional easing through `VisualPatch`.

Acceptance:

- [ ] Tests prove factor `0.0` is identity and factor `1.0` separates parts.
- [ ] Visual proof on a multi-part imported assembly.
- [ ] Example: guided assembly/exploded documentation view.

### 2.5 Named visual states

Owner modules: `scene_host`, `scene`, `diagnostics`.

Foothold: tags, stable handles, patch model, visibility, tint, transforms,
camera state.

Required states:

- [ ] `assembled`.
- [ ] `exploded`.
- [ ] `service_view`.
- [ ] `covers_hidden`.
- [ ] Host-defined names.

Acceptance:

- [ ] A state serializes as a stored `VisualPatch` plus metadata.
- [ ] Stored states preserve omitted additive patch fields by default, following
      the optional/defaulted v1 rule from 0.1.
- [ ] Applying a state is deterministic and inspectable.
- [ ] No document model or undo stack is introduced.

### 2.6 Real SDF/MSDF text

Owner modules: `scene`, `render`, `geometry`, `assets`.

Foothold: `LabelDesc::sdf`, `LabelDesc::msdf`, label nodes, visual proof
fixtures. Current label rendering must be treated as incomplete until actual
crisp text is proven.

Required behavior:

- [ ] Font atlas or embedded default font strategy.
- [ ] SDF/MSDF glyph generation or bundled atlas path.
- [ ] Stable text metrics for layout.
- [ ] Screen-aligned billboards with consistent apparent size.
- [ ] Text color, background/halo option, and DPI-aware scale.
- [ ] Labels remain readable across zoom/orbit.

Acceptance:

- [ ] Unit tests for layout metrics.
- [ ] Headless visual proof for crisp text at multiple sizes.
- [ ] Browser proof for annotation-heavy scene.
- [ ] Performance benchmark for many labels.

### 2.7 Callouts and leader lines

Owner modules: `scene`, `scene_host`, `geometry`, `viewer_element`.

Foothold: annotations, annotation projection reports, line primitives, labels,
technical strokes.

Required behavior:

- [ ] Attach callout to stable node handle, world point, anchor, or connector.
- [ ] Leader line from projected anchor to label.
- [ ] Label stays connected during camera orbit and animation updates.
- [ ] Native and browser report the same projected anchor IDs.
- [ ] Host-facing callout text updates route through the 0.1C `labels`
      `VisualPatch` channel instead of a parallel annotation text model.

Acceptance:

- [ ] Tests for node/world callout cleanup on node removal.
- [ ] Browser proof with moving annotated node.
- [ ] Example: documentation renderer callouts.

### 2.8 Annotation layout helpers

Owner modules: `scene`, `scene_host`, `viewer_element`.

Foothold: `annotation_projection_report`, slotted custom-element annotations,
projected screen coordinates.

Required behavior:

- [ ] Viewport clamping.
- [ ] Behind-camera hiding.
- [ ] Optional occlusion-aware hiding when depth/readback is available.
- [ ] Overlap avoidance with deterministic priority.
- [ ] Layout report listing original and adjusted positions.

Acceptance:

- [ ] Deterministic layout tests for overlapping annotations.
- [ ] Browser proof with crowded labels before/after declutter.

## Phase 3 - Asset and assembly workflows

Goal: make professional assets trustworthy and easy to select, validate, and
assemble without moving domain catalog/search/database logic into Scena.

### 3.1 Asset catalog, manifest, and validation

Owner modules: `assets`, `assets/gltf`, `diagnostics`, `viewer`.

Foothold: `AssetLoadReport`, provenance, source units, coordinate conversion,
bounds, anchors/connectors, texture diagnostics, material variants.

Proposed contracts:

- `scena.asset_catalog.v1`
- `scena.asset_readiness_report.v1`

Manifest fields:

- [ ] Asset ID and display name.
- [ ] Source path or URL.
- [ ] Required files.
- [ ] Preview image path or generated preview metadata.
- [ ] Declared units and source coordinate system.
- [ ] Expected bounds or scale constraints.
- [ ] Required anchors/connectors and tags.
- [ ] Material/texture requirements.
- [ ] License/provenance metadata.
- [ ] Optional categories/tags supplied by the host.

Validation checks:

- [ ] Load succeeds or fails with structured errors.
- [ ] Required external files are present.
- [ ] Bounds are finite and within declared limits.
- [ ] Units and coordinate system are known or explicitly repaired.
- [ ] Required anchors/connectors exist and are well-formed.
- [ ] Required material variants exist.
- [ ] Missing textures and material fallbacks are reported.
- [ ] Preview render can be generated deterministically.

Scope guard:

- [ ] Scena validates and reports. The host owns search, versioning, package
      distribution, database records, approval workflows, and business rules.

Acceptance:

- [ ] Fixture catalog with one valid and several invalid assets.
- [ ] Readiness report stable fixture.
- [ ] Browser preview proof for a catalog asset.
- [ ] Example: asset picker feeding a SceneHost scene.

### 3.2 Asset doctor integration API

Owner modules: `assets`, `diagnostics`, `xtask`, `scene_host`.

Foothold: `xtask doctor`, asset doctor checks, diagnostics JSON, capability
reports.

Required behavior:

- [ ] Rust API returns doctor-style findings for a loaded asset or asset path.
- [ ] WASM API returns JSON findings for browser-hosted assets.
- [ ] CLI and library diagnostics share codes where checks overlap.
- [ ] Findings include severity, code, path, message, help, and suggested fix.

Acceptance:

- [ ] CLI/library parity tests for representative fixtures.
- [ ] Browser proof displays doctor findings for a broken asset.

### 3.3 Connector browser and snap preview

Owner modules: `scene`, `assets/gltf`, `scene_host`, `geometry`.

Foothold: anchors/connectors, connector metadata, connect helpers, distance,
snap preview, ghost transforms.

Required behavior:

- [ ] List connectors for an import, subtree, or selection.
- [ ] Filter compatible connectors by kind, allowed mates, tags, polarity, and
      roll policy from connector metadata.
- [ ] Preview connection with ghost transform.
- [ ] Report snap distance and tolerance.
- [ ] Report invalid mate reasons.
- [ ] Render optional ghost/outline/line cue.

Scope guard:

- [ ] Compatibility is metadata-driven. Scena does not compute mesh clearance,
      collision, or physical feasibility.

Acceptance:

- [ ] Tests for compatible/incompatible connector metadata.
- [ ] Browser proof for snap-ready and snap-invalid states.
- [ ] Example: assembly connector browser.

### 3.4 Material variant helpers

Owner modules: `assets/gltf`, `scene`, `viewer`, `viewer_element`,
`scene_host`.

Foothold: `KHR_materials_variants` parsing, runtime variant selection, viewer
variant picker.

Required behavior:

- [ ] List declared variant names for an import.
- [ ] Apply variant by name through the 0.1C `material_variants`
      `VisualPatch` channel.
- [ ] Clear variant to default through the same patch channel.
- [ ] Report active variant in inspection and asset/import reports.
- [ ] Browser and native APIs have matching behavior and delegate to the same
      patch path.

Acceptance:

- [ ] Tests for missing variant, ambiguous variant, clear-to-default, and stale
      import handles.
- [ ] Browser proof for variant switch with visible pixel change.

### 3.5 Product configurator helpers

Owner modules: `viewer`, `scene_host`, `scene`, `assets/gltf`.

Foothold: material variants, visibility, tint, camera bookmarks, visual patch.

Required behavior:

- [ ] Option group model over visual changes only.
- [ ] Apply option as `VisualPatch`.
- [ ] Report active visual options.
- [ ] Example config can combine variant, tint, visibility, and camera state.

Scope guard:

- [ ] No pricing, compatibility business rules, inventory, persistence, or
      domain-specific configuration logic.

Acceptance:

- [ ] Example: product configurator with material and visibility options.
- [ ] Stable JSON example for option groups if public.

### 3.6 Presentation timeline

Owner modules: `scene_host`, `animation`, `controls`, `viewer`.

Foothold: animation mixers, transform/tint easing, camera state, visual states.

API sketch:

```rust
let timeline = PresentationTimeline::new()
    .at(0.0, TimelineAction::apply_state("assembled"))
    .at(0.5, TimelineAction::camera_bookmark("overview"))
    .at(1.2, TimelineAction::apply_state("exploded"))
    .at(2.0, TimelineAction::play_clip("Open"));
host.seek_timeline(&timeline, 1.5)?;
```

Required behavior:

- [ ] Host-ticked `seek(t)` and `advance(dt)`.
- [ ] Actions compose camera bookmarks, visual states, animation clips,
      transforms, tints, labels, and annotations.
- [ ] Timeline emits `VisualPatch` for the requested time and does not define a
      parallel mutation model.

Scope guard:

- [ ] No autonomous loop. No application workflow engine.

Acceptance:

- [ ] Deterministic seek tests.
- [ ] Browser proof for guided tour.

## Phase 4 - Fidelity and browser reach

Goal: raise output trust and reach web developers without bypassing the host
contracts from Phase 0.

### 4.1 Contact grounding preset

Owner modules: `render`, `scene`, `viewer`, `diagnostics`.

Foothold: SSAO, post-processing, grid/floor helpers, lighting presets,
directional shadow capability tracking.

Required behavior:

- [ ] A product-viewer grounding preset that combines floor/receiver setup,
      SSAO where available, a proven shadow receiver path where supported, and
      lighting defaults where capabilities allow.
- [ ] SSAO alone is ambient occlusion and cannot by itself close
      contact/drop-shadow grounding.
- [ ] Capability report states which grounding path is active.
- [ ] Fallback is explicit when backend capability is unavailable.

Acceptance:

- [ ] Headless and browser visual proof that a product asset is visibly
      grounded.
- [ ] No claim of physical shadow correctness without proof.

### 4.2 Directional shadow proof closure

Owner modules: `render/gpu`, `scene`, `diagnostics`, visual proof harness.

Foothold: shadow map resources, directional light shadow flags, degraded
capability status.

Required behavior:

- [ ] Render shadow casters into a shadow map.
- [ ] Sample shadows into receiver pixels.
- [ ] Validate one shadowed directional light limit or expand it deliberately.
- [ ] Capability status moves from degraded to supported only after visible
      receiver proof.

Acceptance:

- [ ] Unit tests for shadow preparation and single-shadowed-light errors.
- [ ] Rendered proof shows receiver darkening and stable non-shadow regions.
- [ ] Capability report and diagnostics reflect actual backend support.

### 4.3 Physical glass and transmission

Owner modules: `material`, `render/gpu`, `assets/gltf`, diagnostics, visual
proof harness.

Foothold: transmission/IOR/volume parsing, material descriptors, transmission
resources, capability gate.

Required behavior:

- [ ] Scene-color transmission.
- [ ] IOR/thickness refraction approximation.
- [ ] Rough-transmission blur.
- [ ] Transparency ordering strategy documented and proven per backend.
- [ ] Required glTF transmission/volume assets fail or degrade explicitly when
      backend proof is unavailable.

Acceptance:

- [ ] Material tests for parsed factors and textures.
- [ ] GPU/browser proof for clear and frosted glass.
- [ ] Capability report does not overclaim unsupported lanes.

### 4.4 Dense WebGL2 source-material proof

Owner modules: `assets`, `render`, `scene_host`, browser proof harness.

Foothold: source material import, browser proof harness, external resource
loading, renderer-fidelity checklist.

Required behavior:

- [ ] Dense imported glTF/GLB fixture with source materials, external textures,
      normals, metallic/roughness, camera framing, and lighting.
- [ ] Browser WebGL2 render path preserves source materials.
- [ ] Proof distinguishes source material, generated unlit fallback, and PBR
      override paths.

Acceptance:

- [ ] Browser output has non-background pixels and material-specific predicates.
- [ ] Proof records backend, capabilities, resource warnings, stats, and
      screenshot metadata.
- [ ] Capability promotion cites the exact artifact.

### 4.5 `<scena-viewer>` parity

Owner modules: `viewer_element`, `scene_host`, `platform/browser`, `assets`,
`render`.

Foothold: custom element, `SceneHost` JSON APIs, viewer element annotations,
drag/drop, variant picker, browser proof.

Required attributes/APIs:

- [ ] `src`.
- [ ] `environment`.
- [ ] Lighting preset or direct light JSON, mapped to real `Scene` light APIs.
- [ ] Camera/framing attributes and methods.
- [ ] Capture/download methods.
- [ ] Picking, hover, and selection events through `HostEvent`.
- [ ] Material variants.
- [ ] Annotation slots and projection.
- [ ] Drag/drop with load diagnostics.
- [ ] Inspector and diagnostics surfaces.
- [ ] Visual patch application.

Scope guard:

- [ ] The custom element consumes `VisualPatch` and `HostEvent`. It must not
      create a parallel JS-only scene model.

Acceptance:

- [ ] Browser proof for lighting/environment/camera/capture/variant/picking and
      annotation parity.
- [ ] Side-by-side model-viewer style proof for representative assets.
- [ ] Mobile/a11y proof remains green.

## Phase 5 - Demand-driven IO completeness

Goal: add expensive or borderline IO features only when they unblock real
assets or real application workflows.

### 5.1 Draco decode

Owner modules: `assets/gltf`.

Foothold: structured degraded extension diagnostics for
`KHR_draco_mesh_compression`.

Acceptance:

- [ ] Decoder dependency selected and feature-gated.
- [ ] Required Draco assets load or fail with explicit structured error.
- [ ] Browser/native support matrix documented.
- [ ] Real fixture proof only if a real user asset requires it.

### 5.2 glTF or configuration export

Owner modules: `assets`, `scene`, `scene_host`.

Scope:

- [ ] Export visual configuration or a narrow glTF subset for saved viewer
      state.
- [ ] Do not export a CAD document model, product database, or full authoring
      suite.

Acceptance:

- [ ] Export/import round trip for transforms, visibility, variants, camera,
      and annotations where supported.
- [ ] Unsupported fields are reported, not silently dropped.

### 5.3 Hot reload polish

Owner modules: `assets`, `scene`, `scene_host`.

Foothold: `hot-reload` feature and asset retain/reload policy.

Required behavior:

- [ ] Reload assets while preserving stable import/node mappings where source
      identity still matches.
- [ ] Emit reload report describing preserved, replaced, removed, and stale
      handles.
- [ ] Host event reports reload result.

Acceptance:

- [ ] Tests for preserved handles and stale handles after reload.
- [ ] Example: live asset reload in viewer.

## Continuous examples and acceptance apps

These examples are not optional demos. Each phase above should add or update at
least one example that proves the APIs compose into an application workflow.

- [ ] CAD inspection viewer: part tree, picking, fit selection, section box,
      measurements, annotations, isolate/ghost.
- [ ] Industrial dashboard viewer: visual patch stream, event stream,
      diagnostics, labels, stable capture.
- [ ] Product configurator: variants, option groups, camera bookmarks, PNG
      export, appearance assertions, glass/grounding when available.
- [ ] Live-state playback viewer: host-ticked visual patches,
      animation time, temporal assertions, stable proof captures, no domain
      logic in Scena.
- [ ] Headless documentation renderer: deterministic views, callouts,
      contact sheets, baseline comparison.
- [ ] Agent render loop template: recipe, render introspection, visibility
      diagnosis, suggested fix, and rerender from CLI JSON.
- [ ] Data visualization viewer: color-ramp assertions, labels, camera
      bookmarks, and capture proof.
- [ ] Animated viewer: clip inventory, temporal introspection, sampled captures,
      and final-state proof.
- [ ] Interaction proof viewer: synthetic pick, hover, selection, and rendered
      feedback assertions.
- [ ] `<scena-viewer>` browser app: picking, annotations, variants, capture,
      drag/drop, diagnostics.
- [ ] Guided tour: bookmarks, callouts, exploded view, presentation timeline.
- [ ] SceneHost host-loop template: native and browser versions showing
      mutation, prepare, render, event drain, and capture.

## Phase ordering

Recommended order:

1. Phase 0.1A minimal patch envelope.
2. Phase 0.2 Host event API.
3. Phase 0.3 Capture/proof kit.
4. Phase 0.4 External asset diagnostics.
5. Agent track A.1 render introspection contract.
6. Agent track A.2 visibility diagnoser.
7. Agent track A.3 CLI verbs.
8. Agent track A.4 schema discovery.
9. Agent track A.5 declarative scene recipe.
10. Agent track A.6 semantic placement verbs.
11. Agent track A.7 safe visual repair.
12. Agent track A.8 appearance and material introspection.
13. Agent track A.9 animation and temporal introspection.
14. Agent track A.10 synthetic interaction verification.
15. Agent track A.11 core agent smoke templates.
16. Phase 1.1 Camera fly-to and bookmarks.
17. Phase 3.1 Asset catalog and validation.
18. Phase 2.1 and 2.2 Inspection and measurement.
19. Phase 1.2 Transform gizmo.
20. Phase 2.3 through 2.8 CAD/annotation helpers.
21. Phase 3.3 through 3.6 assembly/configuration/presentation workflows.
22. Phase 4 fidelity and `<scena-viewer>` reach.
23. Phase 5 demand-driven IO.

The additive 0.1B/0.1C channels land with the consuming features that need
them, such as `camera_eased` in 1.1, `labels` in 2.7/2.8,
`material_variants` in 3.4, and timeline-emitted patches in 3.6.

The agent-facing track starts after capture and typed diagnostics because it
depends on those surfaces. Its CLI is a transport over stable JSON contracts,
not a competing API layer.

The A.11 CAD-inspection and documentation-renderer templates are
Phase-2-dependent templates. They land incrementally with measurement,
section-box, exploded-view, callout/leader-line, annotation-layout, and
inspection helpers instead of blocking the core agent smoke templates.

Do not start Phase 4 browser reach work by adding ad-hoc custom-element
behavior. The element should inherit the stable patch, event, asset, capture,
and annotation contracts from earlier phases.

## Definition of done for each item

- [ ] Scope guard reviewed against `docs/RFC-rust-3d-renderer.md`.
- [ ] Owner module named.
- [ ] Public API or schema sketched before implementation.
- [ ] Test-first proof or deterministic-proof exception recorded.
- [ ] Native Rust tests pass.
- [ ] WASM build proof exists for browser-exposed APIs.
- [ ] Browser rendered-output proof exists for browser-visible visuals.
- [ ] Stable fixture exists for public JSON contracts.
- [ ] Docs and examples updated.
- [ ] Doctor rule added when source/docs/artifact drift is mechanically
      detectable.
- [ ] Release notes/changelog updated only when the implementation lands.
