# Application builder roadmap

Status: completed historical evidence
Canonical active backlog:
`docs/checklists/full-repo-review-v1.9.0-remediation.md`
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

- [x] Keep the public vocabulary aligned with `Scene`, `Assets`, `Renderer`,
      `SceneImport`, `prepare()`, and `render()`.
- [x] Do not add catch-all `Engine`, `World`, `Manager`, or application-domain
      owner types.
- [x] Do not hide asset fetch, shader compilation, GPU upload, or backend
      capability decisions inside `render()`.
- [x] Keep browser and native host APIs semantically equivalent. Browser APIs
      may use JSON and CSS pixels, but they must not invent a separate product
      model.
- [x] Use stable host `u64` handles in public wire contracts. Never serialize
      raw `NodeKey`, `CameraKey`, `MaterialHandle`, `GeometryHandle`, or other
      internal slotmap keys in external JSON.
- [x] Every public JSON report or command contract uses
      `scena.<contract>.vN`. Additive v1 fields must use `serde(default)` or
      `Option` as appropriate and keep old-fixture deserialization green.
- [x] Browser-visible rendering changes require rendered-output proof. Unit
      tests alone cannot close visual work.
- [x] Every implementation item below needs a focused red test or a documented
      deterministic-proof exception before production code changes.
- [x] Add or extend `xtask doctor` when a new failure family can be detected
      mechanically from source, docs, manifests, fixtures, or gate artifacts.

Evidence: final checklist audit compared this roadmap with
`docs/RFC-rust-3d-renderer.md`; implementation slices below name their owner
modules, stable `scena.*.vN` contracts, focused red/focused-green proof, and
browser/rendered-output proof where applicable. The final docs-only closure
reruns `cargo run -p xtask -- doctor --full` on `scena-builder` before commit.

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

- [x] No CAD kernel, parametric sketcher, constraint solver, boolean modeling,
      STEP/IGES implementation, or geometry document format.
- [x] No physics, rigid-body collision, mesh-clearance solver, machine planning,
      industrial process runtime, closed-loop semantics, or dynamic runtime
      loop.
- [x] No gameplay ECS, networking, audio system, asset-management database, or
      application business rules.
- [x] No undo/redo stack or host document model. Scena may emit reversible
      visual deltas, but the host owns document history.
- [x] No hidden requestAnimationFrame loop for scene state. The host owns
      cadence and calls mutation, time advancement, `prepare()`, and `render()`.

Evidence: the implemented helpers remain visual, diagnostic, interaction, asset,
and proof surfaces. Recipes are versioned interchange/build snapshots rather
than canonical application documents; visual patches and repair reports are
deltas for the host to apply; animation and presentation timeline
proofs are host-ticked and do not add a hidden loop or domain runtime.

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
- [x] Browser proof: patch-driven transform/tint/visibility/camera changes are
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
- [x] Browser proof asserts pick, hover, selection, load progress, asset-loaded,
      diagnostic, capture-ready, and surface-resized event shapes.
- [x] Browser proof asserts context-lost/restored event shapes from a real
      browser signal.

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
- [x] Browser proof captures PNG and descriptor metadata without relying on
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
- [x] Browser proof records all external resources for a `.gltf` with `.bin`
      and texture files. Evidence: `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` was run manually on V3D WebGL2 and loads the
      Khronos WaterBottle `.gltf` in the browser proof, asserting
      `scena.asset_load_report.v1` reports one
      fetched `WaterBottle.bin` buffer plus all four fetched WaterBottle PNG
      texture files.
- [x] Stable `scena.asset_load_report.v1` additions are optional/defaulted.
- [x] Asset doctor and SceneHost reports expose the same diagnostic family.
      Evidence:
      `tests/asset_doctor_contracts.rs::scene_host_asset_doctor_json_uses_same_report_shape`
      verifies `SceneHostCore::asset_doctor_json` emits the same
      `scena.asset_doctor.v1` shape and reason family as `Assets::doctor_asset_path`.

## Agent-facing surface track

Goal: expose Scena's normal render, capture, inspection, and diagnostics path
as stable JSON plus artifact paths so shell-driven tooling can run an
act-see-diagnose-correct loop without compiling Rust or relying on subjective
image review.

Governing rules:

- [x] JSON contracts are the keystone. CLI commands serve those contracts; they
      do not define a second schema family.
- [x] CLI output is stable `scena.*.vN` JSON to stdout plus artifact paths.
      Human text, if any, goes to stderr.
- [x] There is no MCP server in this roadmap. The terminal surface is the CLI
      plus stable JSON.
- [x] No agent-specific render mode. Introspection uses the same
      `prepare()`/`render()`/capture path as human-facing examples and
      applications.
- [x] Reports are deterministic and token-small by default: stable field order,
      stable item ordering, rounded floats, summary-first shape, and opt-in
      detail for per-node or per-pixel-heavy data.
- [x] Contract JSON is byte-stable for the same scene, backend class, options,
      and artifact inputs. Any nondeterministic field must be excluded from the
      stable report or isolated under explicit metadata.
- [x] Shell failure is fail-closed. Commands that render but produce an
      unacceptable frame return structured JSON and a non-zero exit status.
- [x] Repair loops have an iteration budget. Non-converging cases return a
      structured irreducible result with confidence, reason, and
      `auto_fixable: false` rather than forcing a visual success.
- [x] Recipes are transient scene snapshots, not persisted documents or
      workflow scripts. No sequences, loops, branching, or hidden time
      ownership.
- [x] Placement verbs compute one-shot transforms from authored features such
      as anchors, connectors, bounds, or authored planes. They do not infer mesh
      features, solve constraints, compute clearance, or introduce physics.

Evidence: A.1 through A.11 landed as stable `scena.*.vN` JSON contracts served
by `scena render`, `inspect`, `diagnose`, `validate-recipe`, `place`, `repair`,
`verify appearance`, `verify animation`, `verify interaction`, `schema`, and
`examples agent`; stdout golden fixtures pin the shell surface. The CLI tests
assert JSON to stdout, empty stderr on machine-readable success/failure, exit
code `1` for failed visual/diagnostic assertions, and exit code `2` for usage
or validation errors.

Minimum loops:

- [x] Construction loop: emit `scene_recipe.v1` or `visual_patch.v1`, validate,
      render with introspection, diagnose visibility on failure, apply a
      reported fix within budget, rerender, and stop only on `ok=true` or an
      irreducible result.
- [x] Appearance loop: for material, variant, status-color, or data-color work,
      assert the rendered node/material appearance against intended values, not
      only that pixels changed.
- [x] Temporal loop: for animation, transitions, or live-state playback, sample
      requested times and assert channel values, visible changes, and no frozen
      or NaN channel.
- [x] Interaction loop: inject synthetic pointer or keyboard input, then assert
      expected pick/hover/selection events and rendered feedback.

Evidence: `tests/scena_cli_agent_templates.rs` runs the ready agent smoke
templates end-to-end through their CLI commands; `tests/scena_cli_agent.rs`
covers render/diagnose/repair/appearance/animation JSON success and
fail-closed paths; `tests/scena_cli_interaction.rs` and
`tests/browser/scene_host_browser_proof.js` cover synthetic interaction on
native and browser SceneHost surfaces.

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
scena render scene.recipe.json --out target/scena-agent/frame.png
```

Required fields and behavior:

- [x] `schema: "scena.render_introspection.v1"`.
- [x] `ok`: false for implemented error-severity failure families: empty,
      no-visible-drawable, and all-culled frames. Warning-only framing reasons
      such as tiny-in-frame and cropped are reported without failing the agent
      loop.
- [x] `ok`: false for landed behind-camera, outside-frustum, alpha-zero,
      NaN-specific, and active-clipping-plane failure states. Additional
      visually unacceptable states remain future additive reason families.
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
- [x] `nodes_summary`: counts for visible, hidden, culled, transparent, and
      failed-material nodes.
- [x] `nodes_detail`: optional detail mode only; stable handle, name if
      available, reason codes, and draw-derived node state. Placeholder
      coverage and clipped counters were removed from v1 rather than exposed
      as reserved fields.
- [x] `artifacts`: capture PNG path, capture descriptor path or inline summary,
      and optional contact-sheet path.
- [x] `capabilities`: active backend and capability summary, not a duplicate
      full capability report unless detail mode is requested.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/render_introspection.v1.json`.
- [x] Fixture/test frames cover empty background, all-culled, tiny-in-frame,
      cropped, and valid centered content.
- [x] Fixture/test frames cover behind-camera, outside-frustum, NaN transform,
      alpha zero, and active clipping planes.
- [x] Headless tests prove `ok=false` for the implemented failure frames and
      `ok=true` for the valid frame.
- [x] Headless tests prove warning-only framing reports keep `ok=true`.
- [x] Browser proof covers at least empty, offscreen, and valid centered
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
- [x] Diagnose subtree and import targets. Subtree-root handles walk descendant
      inspection rows; SceneHost import rows expose additive `root_handles` so
      import handles diagnose their roots.
- [x] Ranked reasons include not prepared, stale handle, missing camera,
      no visible drawables, node hidden, zero scale, and all culled.
- [x] Ranked reasons include parent hidden, NaN/non-finite transform, layer
      masked, alpha zero, transparent material, missing material upload, and
      missing geometry.
- [x] Ranked reasons include behind camera, outside frustum, clipped by active
      clipping planes, and backend capability degradation.
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
- [x] Unit tests for subtree targets, import targets, parent hidden,
      NaN/non-finite transform, layer mask, alpha zero, transparent material,
      missing material upload, and missing geometry.
- [x] Unit tests for behind-camera, outside-frustum, clipping, and backend
      degradation.
- [x] Tests prove implemented fix suggestions are stable and use public Scena
      vocabulary.
- [x] Whole-scene diagnosis agrees with render introspection for empty,
      all-culled, and behind-camera frames, and partial culling is not reported
      as all-culled.
- [x] Stable fixture:
      `tests/assets/stable-contracts/visibility_diagnosis.v1.json`.
- [x] CLI exits non-zero when diagnosis classifies the requested visibility
      target as not visible
      (`scena_diagnose_cli_emits_json_and_nonzero_for_invisible_target`).

### A.3 Agent-friendly CLI verbs

Owner modules: `bin/scena`, `render`, `scene_host`, `assets`, `diagnostics`,
`capture`, `viewer`.

Foothold: `xtask doctor`, `asset-doctor`, `SceneHostCore` JSON methods,
capture APIs, asset load reports, capability reports, and renderer stats
already expose the needed data in separate surfaces.

Required commands:

- [x] `scena render <asset-or-recipe> --out <png>`: load,
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
- [x] `scena verify animation <recipe-or-asset> --clip <name> --times <csv>
      [--expect-change] [--expect-node-handle <handle>]
      [--expect-translations 'x,y,z;...']`: emit
      `scena.animation_introspection.v1`. Explicit handles come from
      `scena inspect`; without one the verifier reports the first moving node
      it can infer.
- [x] `scena verify interaction <recipe-or-asset> --expect <json>`: emit
      `scena.interaction_verification.v1`. Native CLI verification and
      browser synthetic pick/hover/select proof are complete; rendered
      interaction feedback assertions remain future hardening.
- [x] `scena browser-proof [scene-host|m6] [--dry-run]`: emit
      `scena.browser_proof_run.v1`; delegate to the existing wasm-pack +
      Playwright browser lanes while keeping stdout machine-readable.
- [x] `scena doctor <asset-or-recipe>`: emit `scena.asset_doctor.v1` for a raw
      glTF/GLB asset and the complete policy-aware
      `scena.recipe_build_result.v1` for a parsed recipe. Evidence:
      `tests/scena_cli_agent.rs::scena_doctor_cli_emits_json_and_nonzero_for_broken_asset`
      plus
      `tests/scena_cli_recipe.rs::imports_only_recipe_commands_build_every_import`
      and `tests/assets/cli-golden/doctor_broken_asset_stdout.json`.
- [x] `scena examples agent <template> [--out <dir>]`: emit
      `scena.agent_smoke_template.v1` for core ready templates and structured
      smoke manifests with explicit native-only notes for Phase-2-dependent
      overlay sub-capabilities.

CLI behavior:

- [x] JSON to stdout; progress and human text to stderr for the implemented
      schema, render, inspect, diagnose, and repair commands.
- [x] `--json` is the default for the implemented schema, render, inspect, and
      diagnose commands.
- [x] `--detail` opts into larger render-introspection and visibility-diagnosis
      reports.
- [x] `--round-floats <digits>` defaults to stable, documented precision.
- [x] Exit status `0` means the requested schema operation succeeded.
- [x] Exit status is non-zero for invalid recipes, missing assets, failed
      preparation, failed rendering, `ok=false` introspection, and invisible
      diagnosis targets. Missing assets, recipe validation, report `ok=false`,
      and invisible diagnosis targets emit machine-readable JSON.
- [x] Artifact paths are explicit for implemented render-introspection output.

Acceptance:

- [x] Golden stdout fixtures for each command. Current proof pins
      `schema get`, invalid `validate-recipe`, and broken-asset `doctor`;
      normalized goldens now cover `schema list`, render, inspect, diagnose,
      place, repair, appearance verification, animation verification,
      interaction verification, and `examples agent`.
- [x] Tests verify stderr/stdout separation for schema commands.
- [x] Tests verify `render` writes PNG plus capture descriptor
      artifacts and emits `scena.render_introspection.v1`.
- [x] Tests verify `inspect <asset>` emits `scena.scene_inspection.v1`.
- [x] Tests verify `diagnose --visibility --handle <stale>` emits
      `scena.visibility_diagnosis.v1` on stdout with a non-zero exit.
- [x] Tests verify `render` emits
      `scena.render_introspection.v1` on stdout with a non-zero exit for an
      empty frame.
- [x] Tests verify `repair --from` emits `scena.visual_repair_plan.v1` for a
      reversible diagnosis and emits `scena.agent_loop_result.v1` with a
      non-zero exit for an irreducible diagnosis.
- [x] Tests verify `verify appearance --expect` applies a declared material
      variant, emits `scena.appearance_introspection.v1`, and exits non-zero
      with JSON on stdout when the sampled rendered color does not match the
      expectation.
- [x] Tests verify `verify animation --clip --times --expect-change` emits
      `scena.animation_introspection.v1`, exits non-zero with JSON on stdout
      for a missing clip, and exits non-zero when requested samples do not
      advance in time.
- [x] Tests verify `verify animation --expect-translations` records
      `samples[].observed_values`, exits zero when sampled translations match,
      and exits non-zero with `expected_value_mismatch` when they do not.
- [x] Tests verify `verify animation --expect-node-handle` binds sampled
      translations to the requested stable node handle instead of auto-selecting
      another moving node.
- [x] Tests verify non-zero exits for invalid recipe and invisible target.
- [x] Tests verify missing assets emit JSON instead of command-error text.
- [x] Doctor rule keeps CLI help, schema constants, docs schema references, and
      stable fixtures aligned.

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
- [x] `scena schema get <schema>` emits one representative invalid example
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
- [x] The schema list includes animation introspection as landed.
- [x] The schema list includes interaction expectation and interaction
      verification as landed.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/schema_catalog.v1.json`.
- [x] Tests prove every listed schema has a fixture or documented exception.
- [x] CLI `schema get` examples deserialize against their contract for the
      tested render-introspection schema.
- [x] Doctor rule rejects schema names referenced in docs but missing from the
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
- [x] The recipe surface supports imports, per-import transforms and expected
      extents, caller metadata, at most one optional capture directive,
      section boxes, distance measurements, callouts, and exploded-view
      directives.
- [x] Future recipe sections may include primitive nodes, materials, cameras,
      lights, labels, viewer profile references, environment references, and
      feature-owned inspection/annotation helpers as those owner features land.
      Current validation recognizes these as known future sections and emits
      `unsupported_feature` instead of silently accepting them.
- [x] A recipe may reference authored anchors, connectors, bounds, or authored
      planes for placement. Current placement uses authored features through
      `scena place`; recipe-level authored-feature sections are reserved as
      known future sections and fail closed until their owner slice lands.
- [x] Recipe sections are extensible with their owning features. Section boxes,
      measurement overlays, callouts/leader lines, and exploded-view
      directives are recipe-expressible; named-state references and other
      future sections still fail closed with `unsupported_feature`.
- [x] A recipe is not a project file, document model, workflow script, or
      persisted application state.

Required validation:

- [x] Unknown fields fail closed unless explicitly declared as caller metadata.
- [x] Unknown fields and duplicate caller IDs produce structured diagnostics
      with `help`.
- [x] Near-miss suggestions use a bounded string-distance check for supported
      field names, for example `importe` suggesting `imports`.
- [x] Unknown verbs, enum variants, handles, materials, assets, or profiles
      produce structured diagnostics as those sections land. Current landed
      recipe-adjacent surfaces pin `unknown_verb`, `stale_handle`,
      `asset_load_failed`, `invalid_transform`, and `extent_out_of_range`;
      enum/material/profile diagnostics remain required with their future
      owning recipe sections.
- [x] Units and scale sanity warnings detect extreme asset extents against an
      expected range when the recipe supplies one.
- [x] Recipe sections whose owning feature is not implemented fail with a
      structured `unsupported_feature` diagnostic instead of being silently
      ignored.
- [x] Validation can run without rendering.
- [x] Rendering a recipe uses the same load, prepare, render, and capture path
      as native examples. Recipes with overlay directives go through the
      `SceneHostCore` path and `frame_all_with_overlays`.

Acceptance:

- [x] Stable fixture:
      `tests/assets/stable-contracts/scene_recipe.v1.json`.
- [x] Stable validation fixture:
      `tests/assets/stable-contracts/scene_recipe_validation.v1.json`.
- [x] Validation tests cover unknown field, duplicate caller ID, and rejected
      workflow fields.
- [x] Invalid-recipe fixtures cover unknown verb, stale handle, missing asset,
      invalid transform, and oversized asset for landed sections under
      `tests/assets/recipe-invalid/`. Unknown enum/material/profile fixtures
      are explicitly deferred until those recipe sections land.
- [x] Validation diagnostics include deterministic "did you mean" suggestions.
- [x] A valid recipe renders through `scena render` and produces
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
- [x] Visual proof shows `center`, `ground`, and `align_to_anchor` produce a
      visible, framed result when passed through render introspection.
      Evidence: `cargo test --features inspection --test scena_cli_recipe
      scena_place_cli_previews_render_as_visible_framed_recipes -- --nocapture`
      passed on `scena-builder`; the proof feeds each placement transform back
      into `scena render` and asserts `ok=true`, visible pixels,
      content bbox, and PNG output.

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
      mode, texture presence, fallback policy, and optional target-level
      swatch tolerance. Material-name matching remains future work until stable
      material inspection exposes source names.
- [x] Report active source material, active variant, material factors,
      texture/fallback provenance, sampled region stats, dominant color family,
      alpha summary, and luminance summary.
- [x] `ok=false` when an intended variant is missing, a generated fallback is
      used where source material was required, alpha hides the target, sampled
      color family or numeric swatch distance disagrees with the expectation,
      or texture provenance is missing.
- [x] Reports include suggested fixes, such as apply variant, load missing
      texture, clear fallback override, set alpha mode, or inspect material
      assignment.
- [x] Summary mode is small and includes capture-bound sampled frame content,
      swatch distance, alpha summary, material source, fallback provenance, and
      luminance mean. Matched material-bearing draws use projected `node_bbox`
      color samples; unmatched or node-without-draw targets fall back to
      `frame_content`. Per-node fragment coverage and glTF material-name checks
      remain future additive fields because current stable material inspection
      exposes source index/provenance but not material names or ID-buffer
      coverage.

Acceptance:

- [x] Fixtures and tests cover intended green, wrong color, generated fallback,
      missing variant, valid source-material variant, per-target node-bbox color
      sampling, and strict swatch tolerance. Missing texture, alpha zero, and
      source-material name checks remain future hardening cases.
- [x] Product-configurator proof asserts the requested variant rendered with
      the expected appearance, not merely that pixels changed.
- [x] Data-color proof asserts a known color ramp sample without a golden
      image.
      Evidence: `cargo test --features inspection --test
      appearance_introspection_contracts
      appearance_introspection_verifies_data_color_ramp_without_golden_image
      -- --nocapture` passed on `scena-builder`; the proof samples three
      generated ramp nodes by projected `node_bbox`, checks swatch distance,
      and asserts channel dominance for low/mid/high colors.
- [x] Stable fixture:
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

- [x] Sample requested times for imported clips through explicit
      host-ticked `seek_animation` calls. Presentation transitions and
      arbitrary host-ticked visual state playback remain future additive
      inputs.
- [x] Report clip name, channel count, sampled times, changed channel counts,
      unchanged channel counts, invalid channel counts, and rendered-change
      summary.
- [x] Report selected transform values at each sample when expected
      translations are supplied. Tint/camera/visibility expected-value inputs
      remain future additive fields because the current A.9 surface samples
      imported animation clips, not arbitrary visual patch playback.
- [x] `ok=false` when a required clip is missing, time does not advance,
      channels freeze unexpectedly, channel values become non-finite, or
      visible output stays unchanged when change was expected. Expected
      pose/state-at-time assertions remain future additive fields.
- [x] No hidden time loop. The report records the explicit sampled times and
      host advancement calls used to produce it.

Acceptance:

- [x] Fixtures and CLI tests cover valid change, missing clip, and requested
      samples that do not advance in time. Frozen channel, NaN transform, wrong
      pose at time, and expected final-state fixtures remain future hardening
      cases.
- [x] Live-state playback proof samples at least three times and verifies the
      expected visual state at each sample.
      Evidence: `cargo test --features inspection --test scena_cli_agent
      scena_verify_animation_cli_checks_expected_sampled_translations --
      --nocapture` passed on `scena-builder`; it samples `MoveTriangle` at
      `0,0.5,1.0`, verifies observed translations
      `0,0,0;0.25,0,0;0.5,0,0`, and proves a wrong middle sample fails with
      `expected_value_mismatch`.
- [x] Animated viewer proof verifies rendered changes happened between samples through
      changed capture payloads and moving-node counts. Expected final-state
      matching remains future hardening.
- [x] Stable fixture:
      `tests/assets/stable-contracts/animation_introspection.v1.json`.
- [x] CLI exits non-zero for temporal assertion failures.

### A.10 Synthetic interaction verification

Owner modules: `platform`, `viewer`, `viewer_element`, `picking`,
`scene_host`, `controls`, `diagnostics`, browser proof harness.

Foothold: picking, hover/selection state, HostEvent, browser proof harness,
viewer callbacks, interaction styles, and custom-element events exist or are
introduced earlier in the roadmap.

Proposed contract:

- `scena.interaction_expectation.v1`
- `scena.interaction_verification.v1`

Required behavior:

- [x] Inject synthetic pointer-equivalent pick/hover/select input in native
      SceneHost tests and CLI. Keyboard input, camera-control gestures, and the
      browser proof harness remain open.
- [x] Inputs use CSS pixels by default and explicitly state physical pixel
      fields in reports. Browser target proof remains open.
- [x] Assertions include expected pick handle, hover state, selection state,
      and event sequence for the native SceneHost slice. Modifier/button
      metadata, camera-control result, and rendered highlight or outline
      feedback remain open.
- [x] `ok=false` when the expected handle is not picked, event order differs,
      hover/selection state is missing, hover/selection state is unexpectedly
      present, or CSS-vs-physical coordinate handling misses the expected
      target. Rendered feedback assertions remain open.
- [x] Reports include suggested fixes for the implemented failure families,
      including frame target, CSS-pixel coordinates, event contract, and update
      expected handle.

Acceptance:

- [x] Native fixtures/tests cover successful pick, miss, wrong handle, hover
      enter/leave, selection state set/clear, helper-geometry pass-through, and
      CSS-vs-physical pixel mismatch. Evidence:
      `tests/assets/stable-contracts/interaction_expectation.v1.json`,
      `tests/assets/stable-contracts/interaction_verification.v1.json`,
      `tests/scena_cli_interaction.rs`, and
      `src/picking/tests.rs::asset_picking_skips_stroke_overlay_and_returns_underlying_mesh`.
      Rendered selection/hover highlight proof remains open under the browser
      proof gate.
- [x] Browser proof runs without manual mouse input. Evidence:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof` passed
      manually with V3D WebGL2 and wrote
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`;
      `tests/browser/scene_host_browser_proof.js` asserts synthetic
      `host.pick`, `host.hover`, and `host.select` return the tracked handle
      and that emitted `pick`, `hover`, and `selection_changed` events carry
      the same stable handle.
- [x] Native SceneHost proof runs without manual input devices.
- [x] Stable fixtures:
      `tests/assets/stable-contracts/interaction_expectation.v1.json` and
      `tests/assets/stable-contracts/interaction_verification.v1.json`.
- [x] CLI exits non-zero when interaction assertions fail.

### A.11 Agent smoke templates

Owner modules: `bin/scena`, `examples`, `docs`, `scene_host`, `diagnostics`,
proof harness.

Foothold: examples, stable contract fixtures, visual proof artifacts, and
SceneHost host-loop examples exist.

Required templates:

- [x] `scena examples agent product-configurator`.
- [x] `scena examples agent live-state-viewer`.
- [x] `scena examples agent web-viewer`.
- [x] `scena examples agent data-visualization`.
- [x] `scena examples agent animated-viewer`.
- [x] `scena examples agent interaction-proof`.
- [x] `scena examples agent cad-inspection`, implemented as a runnable CLI
      smoke workflow for asset load, render introspection, visibility
      diagnosis, section boxes, measurements, callouts, and exploded views.
- [x] `scena examples agent documentation-renderer`, implemented as a runnable
      CLI smoke workflow for asset load, render introspection, visibility
      diagnosis, section boxes, measurements, callouts, and exploded views.

Required behavior:

- [x] Each ready template emits a recipe, expected assertions where the
      workflow needs them, CLI commands, and expected artifact paths.
- [x] Agent-track templates can be run through CLI-only steps and produce
      `ok=true` reports for their relevant construction, appearance, temporal,
      or interaction checks when A.11 lands.
- [x] Phase-2-dependent templates do not overclaim domain authoring:
      CAD/documentation templates provide runnable overlay recipe smoke
      commands while CAD kernels, drawing import, page layout, and prose
      generation remain host-side.
- [x] Templates are examples and acceptance apps, not a hidden application
      framework.

Acceptance:

- [x] CI or doctor verifies the six ready core template commands remain
      runnable. CAD/documentation templates also run load/render/diagnose smoke
      commands and inspect generated line/label overlays from recipe
      directives.
- [x] Template outputs include stable JSON reports and capture artifacts.
- [x] Failing fixture variants exist for each dynamic verification class:
      appearance wrong color, animation wrong sampled translation, and
      interaction wrong handle.

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

- [x] `CameraBookmark { name, state, target_bounds, description }`.
- [x] `OrbitControls::fly_to(state, easing, duration)`.
- [x] `SceneHostCore::set_camera_eased`.
- [x] WASM `setCameraEased(...)` and `setCameraBookmarkJson(...)`.
- [x] Browser and native convenience APIs delegate to the 0.1B
      `camera_eased` `VisualPatch` channel instead of defining a parallel
      camera transition wire model.
- [x] Optional bookmark list on viewer helpers.
- [x] Camera transitions are advanced only by explicit host time advancement.

Acceptance:

- [x] Camera interpolation keeps target, distance, yaw, and pitch finite.
- [x] Zero duration applies immediately.
- [x] Invalid camera states fail with structured errors.
- [x] Browser proof shows a framed object remains visible during and after
      fly-to. Evidence: `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` passed manually with V3D WebGL2; the
      SceneHost browser proof asserts `setCameraEased` halfway and final
      captures both have nonzero visible pixels and a nonempty content bbox,
      and that the final camera reaches the requested target state.

### 1.2 Transform gizmo and manipulator

Owner modules: `controls`, `picking`, `scene_host`, `geometry`.

Foothold: picking, interaction styles, technical strokes, camera projection,
and platform-neutral controls exist.

Feature flag: `gizmo` or under `controls` if dependency-free.

API sketch:

```rust
let gizmo = TransformGizmo::new(GizmoMode::Translate)
    .with_constraint(GizmoConstraint::Axis(GizmoAxis::X));
if let Some(transform) = gizmo.drag_transform(start_transform, start_ray, current_ray) {
    host.apply_patch(&gizmo.to_visual_patch(target_handle, transform))?;
}
```

Required behavior:

- [x] Modes: translate, rotate, scale.
- [x] Coordinate spaces: world, local, view-aligned where implementable.
- [x] Axis constraints and plane constraints.
- [x] Hit testing uses existing picking/ray contracts through `Hit` target
      extraction and caller-supplied `GizmoRay` values derived from the active
      camera/pointer pick path.
- [x] Gizmo emits transform deltas or `VisualPatch`; it does not mutate a host
      document directly.
- [x] Gizmo visuals are helper geometry/strokes and can be hidden from normal
      scene picking except when active.

Scope guards:

- [x] No undo/redo stack.
- [x] No collision, constraints solver, snapping-to-mesh, or CAD kernel.
- [x] No application selection model beyond emitting stable handles/events.

Acceptance:

- [x] Unit tests for ray-to-axis and ray-to-plane math. Red proof:
      `CARGO_INCREMENTAL=0 cargo test --test transform_gizmo -- --nocapture`
      failed on the builder with unresolved `Gizmo*` / `TransformGizmo`
      imports before implementation.
- [x] Interaction tests for stale target handles through
      `TransformGizmo::to_visual_patch(...)` + `SceneHostCore::apply_patch`;
      stale handles fail closed as `NodeHandleNotFound`.
- [x] Browser proof for translate drag and rotation drag. Evidence:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof`
      passed manually on V3D WebGL2 and asserts WASM
      `applyGizmoDragJson()` is exported, a
      `scena.scene_host_gizmo_drag.v1` translate drag applies one
      visual-patch transform and moves the tracked mesh along X, a rotate drag
      applies one visual-patch transform and rotates local X to world Y, and
      the rendered/captured frame remains nonblank.
- [x] Example: `simple_scene_editor_gizmo.rs` moves a selected part with the
      gizmo, attaches helper strokes, frames the scene, and renders once.

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

- [x] `ViewerProfile::model_viewer()`.
- [x] `ViewerProfile::cad_inspection()`.
- [x] `ViewerProfile::product()`.
- [x] `ViewerProfile::industrial()`.
- [x] `ViewerProfile::documentation()`.

Acceptance:

- [x] Profiles are composable builder presets, not separate viewer engines.
- [x] Applying a profile does not call `prepare()` or `render()` implicitly.
- [x] Each profile has a minimal example and a snapshot/proof artifact.
- [x] Browser and native profile names match.

Evidence:

- [x] Test-first proof: before production code, `cargo test --test
      m7_interactive_viewer
      viewer_profiles_apply_named_presets_without_parallel_viewer_types
      -- --nocapture` failed on `scena-builder` with unresolved
      `ViewerProfile` and `with_viewer_profile`.
- [x] Focused proof: `cargo test --test m7_interactive_viewer viewer_profile
      -- --nocapture` passes on `scena-builder`, proving the named presets,
      headless builder application, interactive orbit-controls handoff, and
      renderable grid/helper effect.
- [x] Browser/native profile-name proof: `cargo test --test
      scena_viewer_element scena_viewer_profile_attribute_uses_native_viewer_profile_names
      -- --nocapture` passes on `scena-builder`, proving
      `<scena-viewer profile="...">` accepts the same names as
      `ViewerProfile::names()` and rejects browser-only vocabulary.
- [x] Example proof: `cargo run --example viewer_profiles --
      target/gate-artifacts/viewer-profiles` writes one PNG per profile:
      `model_viewer`, `cad_inspection`, `product`, `industrial`, and
      `documentation`.

## Phase 2 - Inspection, CAD view, and annotation

Goal: build the technical viewer tools that CAD-style inspection,
industrial live-state visualization, and documentation need.

### 2.1 Inspection toolkit

Owner modules: `scene`, `scene_host`, `diagnostics`, `geometry`, `viewer`.

Foothold: scene inspection, node hierarchy, visibility, layers, node bounds,
tags, picking, hover/selection, and helper geometry.

Required features:

- [x] Part tree from imported hierarchy and host handles.
- [x] `isolate(selection)`: hide all unrelated visible nodes.
- [x] `ghost(node/subtree, alpha)`: tint/alpha helper for context parts.
- [x] `show_only`, `hide`, `show`, `toggle`.
- [x] Fit selected node/subtree.
- [x] Bounding-box helper overlays.
- [x] Local/world axes triad widget.
- [x] Selection set helpers by handle list. Tag/import path/import name helper
      consolidation remains open for the host-facing slice.
- [x] Inspection report records active isolate/ghost/helper state where useful.

Acceptance:

- [x] Tests for isolate and restore preserving prior visibility.
- [x] Tests for ghost not mutating source material descriptors.
- [x] Tests for fit-selection, bounding-box overlays, axes triads, and toolkit
      state reporting.
- [x] Browser proof for part-tree selection, isolate, ghost, and fit selection.
- [x] Example: CAD inspection viewer.

Evidence:

- [x] Test-first proof: `cargo test --test inspection_tools -- --nocapture`
      failed on the builder before implementation with unresolved
      `SceneVisibilitySnapshot`, `Scene::isolate`, `restore_visibility`,
      `hide`, `toggle_visibility`, `show`, `show_only`, `ghost`, and
      `restore_tints`.
- [x] Focused tests: `cargo test --test inspection_tools -- --nocapture`
      passes locally after implementation.
- [x] Example proof: `cargo run --example cad_inspection_viewer --
      target/gate-artifacts/cad-inspection-viewer` renders a CAD inspection
      artifact using show-only, ghost, selected-node framing, bounding-box
      overlays, axes triads, and measurement overlays.
- [x] Test-first proof: `cargo test --test inspection_tools -- --nocapture`
      failed on the builder before the native helper implementation with
      unresolved `InspectionHelperKind`, `fit_selection_with_assets`,
      `add_bounding_box_overlay`, `add_world_axes_triad`,
      `add_local_axes_triad`, and `inspection_toolkit_report`.
- [x] Focused tests: `cargo test --test inspection_tools -- --nocapture`
      passes locally with seven inspection-tool tests after implementation.
- [x] Test-first proof: `cargo test --features scene-host --test scene_host
      scene_host_subtree_query_is_stable_and_batch_tint_respects_exclusions
      -- --nocapture` failed on the builder before implementation with missing
      `SceneHostSubtreeNodeV1::parent` and `children` fields.
- [x] Focused tests: `cargo test --features scene-host --test scene_host
      scene_host_subtree_query_is_stable_and_batch_tint_respects_exclusions
      -- --nocapture` covers stable host handles plus parent/child tree edges.
- [x] Test-first proof: `cargo test --features scene-host --test scene_host
      scene_host_inspection_tools_drive_part_tree_selection_isolate_ghost_and_fit
      -- --nocapture` failed on the builder before implementation with missing
      `SceneHostCore::show_only`, `ghost`, and `fit_selection`.
- [x] Focused tests: `cargo test --features scene-host --test scene_host
      scene_host_inspection_tools_drive_part_tree_selection_isolate_ghost_and_fit
      -- --nocapture` passes locally.
- [x] Browser proof: `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` passed manually on V3D WebGL2 with artifact
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`
      and screenshot
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.png`.

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

- [x] Point-to-point distance.
- [x] Angle between three points or two vectors.
- [x] Axis-aligned dimension from bounds.
- [x] Leader line with label.
- [x] Unit-format hook supplied by the host.
- [x] Optional screen-space label projection.
- [x] Measurement report with rendered label ID. Source handle/point metadata
      for SceneHost/browser reports remains open for the later host-facing
      slice.

Scope guard:

- [x] Scena renders measurement visuals and computes simple geometric values.
      The host decides which points matter and owns semantic measurement rules.

Acceptance:

- [x] Unit tests for distance, angle, and bounds dimensions.
- [x] Visual proof for dimension line and label rendering.
- [x] Browser proof for selecting two points and rendering a distance overlay.

Evidence:

- [x] Test-first proof: `cargo test --test measurement_overlays -- --nocapture`
      failed on the builder before implementation with unresolved
      `MeasurementAxis`, `MeasurementKind`, `MeasurementOverlay`,
      `UnitFormat`, and `Scene::add_measurement_overlay`.
- [x] Focused tests: `cargo test --test measurement_overlays --test
      measurement_visual_proof -- --nocapture` passes locally after
      implementation.
- [x] Test-first proof: `cargo test --features scene-host --test scene_host
      scene_host_distance_measurement_overlay_reports_stable_line_handle
      -- --nocapture` failed on the builder before implementation with missing
      `SceneHostCore::add_distance_measurement_json`.
- [x] Focused tests: `cargo test --features scene-host --test scene_host
      scene_host_distance_measurement_overlay_reports_stable_line_handle
      -- --nocapture`, `cargo test --features scene-host --test
      stable_contracts
      scene_host_measurement_overlay_golden_matches_live_schema_serialization
      -- --nocapture`, and `cargo test --features scene-host --test
      scena_cli_schema -- --nocapture` pass locally.
- [x] Test-first proof: the same focused SceneHost measurement test failed on
      the builder with `label_projection` absent before the optional
      screen-space label projection was added; the focused SceneHost and stable
      contract tests pass locally after implementation.
- [x] Browser proof: `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` passed manually on V3D WebGL2 with selected point
      centers, `scena.scene_host_measurement_overlay.v1` line-node report,
      draw-list line material, and rendered pixel evidence in
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`.

### 2.3 Section box helper

Owner modules: `scene`, `render`, `scene_host`.

Foothold: `ClippingPlane` and `ClippingPlaneSet`.

API sketch:

```rust
let section = SectionBox::from_bounds(bounds).with_margin(0.05);
scene.set_section_box(section)?;
```

Required behavior:

- [x] Six clipping planes from an `Aabb`.
- [x] Enable, disable, invert, and update section box.
- [x] Optional helper wireframe box.
- [x] Serialization through the additive `VisualPatch.section_box` channel;
      no browser-only or dedicated parallel state model.

Acceptance:

- [x] Tests verify six planes and stable clipping behavior.
- [x] Browser proof shows cutaway on an imported asset.

Evidence:

- [x] Test-first proof: `cargo test --features scene-host --test scene_host
      scene_host_section_box_helper_drives_report_and_visual_patch_channel
      -- --nocapture` failed on the builder before implementation with missing
      `SectionBox`, SceneHost section-box report, and `VisualPatch.section_box`
      symbols.
- [x] Focused local proof: `cargo test --features scene-host --test
      scene_host scene_host_section_box_helper_drives_report_and_visual_patch_channel
      -- --nocapture`, `cargo test --test m2_lighting_depth_clipping
      section_box_clips_rendered_output_and_inverts_inside_region --
      --nocapture`, `cargo test --features scene-host --test
      stable_contracts -- --nocapture`, and `cargo test --lib --
      --nocapture` pass.
- [x] Browser proof: `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` passed manually on V3D WebGL2 with
      `section_box_report_exposes_six_planes_and_helper`,
      `section_box_cutaway_changes_imported_asset_pixels`,
      `section_box_invert_changes_cutaway_pixels`, and
      `section_box_clear_disables_cutaway` assertions in
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`.

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

- [x] Offset direct children from assembly center.
- [x] Offset by hierarchy depth or selected axis.
- [x] Support factor from `0.0` assembled to `1.0` exploded.
- [x] Produce reversible transforms.
      Evidence: `ExplodedViewPlan` stores `original` and exploded transforms;
      `SceneHostCore::exploded_view_patch_json()` keeps the top-level
      `scena.visual_patch.v1` shape and includes
      `metadata.scena_exploded_view_restore_patch` so JSON/WASM hosts can apply
      an immediate restore patch without retaining Rust-only plan state.
- [x] Optional easing through `VisualPatch`.

Acceptance:

- [x] Tests prove factor `0.0` is identity and factor `1.0` separates parts.
- [x] Visual proof on a multi-part imported assembly.
- [x] Example: guided assembly/exploded documentation view.

Implementation evidence:

- [x] Red test first: `cargo test --features scene-host --test exploded_view
      -- --nocapture` failed on `scena-builder` with missing
      `ExplodedView`, `ExplodedViewPlan`, and
      `SceneHostCore::exploded_view_patch_json`.
- [x] Focused proof: `cargo test --features scene-host --test exploded_view
      -- --nocapture` passes on `scena-builder` with direct-child identity/separation,
      selected-axis, hierarchy-depth, SceneHost visual-patch, and imported
      assembly rendered-pixel assertions.
- [x] JSON host restore proof: the SceneHost visual-patch test failed first
      until `metadata.scena_exploded_view_restore_patch` carried immediate
      restore transforms for both immediate and eased exploded-view patches.
- [x] Full remote gate: `cargo fmt --check`, `cargo clippy --all-targets
      --features scene-host -- -D warnings`, `cargo test --features
      scene-host`, `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passed
      on `scena-builder`.
- [x] Example proof: `examples/guided_exploded_view.rs` writes assembled,
      exploded, and restored documentation frames using the same reversible
      `ExplodedViewPlan` updates.

### 2.5 Named visual states

Owner modules: `scene_host`, `scene`, `diagnostics`.

Foothold: tags, stable handles, patch model, visibility, tint, transforms,
camera state.

Required states:

- [x] `assembled`.
- [x] `exploded`.
- [x] `service_view`.
- [x] `covers_hidden`.
- [x] Host-defined names.

Acceptance:

- [x] A state serializes as a stored `VisualPatch` plus metadata.
- [x] Stored states preserve omitted additive patch fields by default, following
      the optional/defaulted v1 rule from 0.1.
- [x] Applying a state is deterministic and inspectable.
- [x] No document model or undo stack is introduced.

Proof:

- [x] Test-first proof: before production code,
      `cargo test --features scene-host --test scene_host
      scene_host_named_visual_states_store_serialize_and_apply_visual_patches
      -- --nocapture` failed on `scena-builder` because
      `SceneHostVisualStateV1`, `SceneHostVisualStatesReportV1`,
      `store_visual_state(_json)`, `visual_states_json`, and
      `apply_visual_state_json` did not exist.
- [x] Focused proof: the same test passed locally after adding
      `SceneHostCore` named visual-state storage and application.
- [x] Stable contract proof:
      `tests/assets/stable-contracts/scene_host_visual_state.v1.json` and
      `tests/assets/stable-contracts/scene_host_visual_states.v1.json` pin the
      stored-state and deterministic inventory wire shapes.
- [x] Remote gate proof: after syncing to `scena-builder`
      `~/projects/scena-visual-patch-impl`, `cargo fmt --check`,
      `cargo clippy --all-targets --features scene-host -- -D warnings`,
      `cargo test --features scene-host`,
      `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passed.

### 2.6 Label text billboards

Owner modules: `scene`, `render`, `geometry`, `assets`.

Foothold: `LabelDesc::new`, label nodes, visual proof fixtures. Labels render
through the embedded/default TrueType atlas path; distance-field rasterization
variants are not part of the public surface.

Required behavior:

- [x] Embedded default TrueType glyph strategy.
- [x] Stable text metrics for layout.
- [x] Screen-aligned billboards with consistent apparent size.
- [x] Text color, background/halo option, and DPI-aware scale.
- [x] Labels remain glyph-shaped and readable across tested zoom/orbit cases.

Acceptance:

- [x] Unit tests for layout metrics.
- [x] Headless visual proof for crisp text at multiple sizes.
- [x] Browser proof for annotation-heavy TrueType-atlas label scene.
- [x] Performance benchmark for many labels.

Proof:

- [x] Test-first proof: before production code,
      `cargo test --test label_text -- --nocapture` failed on
      `scena-builder` because `LabelDesc::with_background()` and
      `LabelDesc::with_halo()` did not exist.
- [x] Focused native proof: `cargo test --test label_text -- --nocapture`
      passed locally, including stable metrics, screen-pixel billboard sizing,
      multi-size rendered text, orbit-view readability, and the many-label
      benchmark artifact.
- [x] Regression proof: `cargo test --test m3a_app_features
      labels_use_truetype_billboard_atlas_render_path --
      --nocapture`, `cargo test --test measurement_visual_proof
      measurement_overlay_renders_line_and_label_pixels -- --nocapture`, and
      `cargo test --test examples_visual_proof
      examples_visual_labels_helpers_renders_axes_bounds_anchor_label_to_ppm --
      --nocapture` passed locally.
- [x] Browser proof: after rebuilding `target/m6-browser-pkg` with
      `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg .
      --features browser-probe`, `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:m6` passed and asserted the `labels-helpers` workflow as
	      `browser-truetype-labels` with 12 labels and visible canvas pixels.
- [x] Remote gate proof: after syncing to `scena-builder`
      `~/projects/scena-visual-patch-impl`, `cargo fmt --check`,
      `cargo clippy --all-targets --features scene-host -- -D warnings`,
      `cargo test --features scene-host`,
      `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passed.

### 2.7 Callouts and leader lines

Owner modules: `scene`, `scene_host`, `geometry`, `viewer_element`.

Foothold: annotations, annotation projection reports, line primitives, labels,
technical strokes.

Required behavior:

- [x] Attach callout to stable node handle, world point, anchor, or connector.
- [x] Leader line from projected anchor to label.
- [x] Label stays connected during camera orbit and animation updates.
- [x] Native and browser report the same projected anchor IDs.
- [x] Host-facing callout text updates route through the 0.1C `labels`
      `VisualPatch` channel instead of a parallel annotation text model.

Acceptance:

- [x] Tests for node/world callout cleanup on node removal.
- [x] Browser proof with moving annotated node.
- [x] Example: documentation renderer callouts.

Proof:

- [x] Test-first proof: before production code,
      `cargo test --test callouts -- --nocapture` failed on
      `scena-builder` because `Callout`, `Scene::add_callout`,
      `Scene::clear_callout`, `Scene::callout`, and the SceneHost callout
      entrypoints did not exist.
- [x] Focused native proof:
      `cargo test --features scene-host --test callouts -- --nocapture` passed
      locally, covering node, world, anchor, and connector targets; generated
      leader-line and label visuals; cleanup; stable SceneHost handles; and
      retargeting through the 0.1C `labels` visual-patch channel.
- [x] Example proof:
      `cargo run --example headless_documentation_renderer --
      target/gate-artifacts/headless-documentation-renderer-callouts` passed
      locally and wrote `documentation-render.png` plus the
      `scena.capture.v1` descriptor.
- [x] Browser proof:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof` passed
      manually on V3D WebGL2 and asserted `addNodeCallout`, a shared
      `browser-callout` annotation anchor ID, and before/after projection
      movement for the callout target node.
- [x] Remote gate proof: after syncing to `scena-builder`
      `~/projects/scena-visual-patch-impl`, `cargo fmt --check`,
      `cargo clippy --all-targets --features scene-host -- -D warnings`,
      `cargo test --features scene-host`,
      `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passed.

### 2.8 Annotation layout helpers

Owner modules: `scene`, `scene_host`, `viewer_element`.

Foothold: `annotation_projection_report`, slotted custom-element annotations,
projected screen coordinates.

Required behavior:

- [x] Viewport clamping.
- [x] Behind-camera hiding.
- [x] Optional occlusion-aware hiding when depth/readback is available.
- [x] Overlap avoidance with deterministic priority.
- [x] Layout report listing original and adjusted positions.

Acceptance:

- [x] Deterministic layout tests for overlapping annotations.
- [x] Browser proof with crowded labels before/after declutter.

Proof:

- [x] Red test first:
      `cargo test --test scena_viewer_element scena_viewer_annotation_layout_clamps_hides_and_declutters_deterministically -- --nocapture`
      failed on `scena-builder` with unresolved imports for
      `ScenaViewerAnnotationLayoutInput`,
      `ScenaViewerAnnotationLayoutOptions`, and
      `layout_scena_viewer_annotations`.
- [x] Focused native proof:
      `cargo test --test scena_viewer_element -- --nocapture` passed locally
      and on `scena-builder` after sync, including the deterministic clamp,
      behind-camera, occlusion, and priority-overlap test.
- [x] Browser proof:
      `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
      and `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` passed locally.
      The `scena.scena_viewer_element_browser_proof.v1` result asserts three
      annotation layout entries, two visible labels after declutter,
      `annotation_clamped_visible=true`, `annotation_overlap_hidden=true`, and
      a changed tracking transform after the second projection update.
- [x] Remote gate proof: after syncing to `scena-builder`
      `~/projects/scena-visual-patch-impl`, `cargo fmt --check`,
      `cargo clippy --all-targets --features scene-host -- -D warnings`,
      `cargo test --features scene-host`,
      `cargo run -p xtask -- doctor --full`, and
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features` passed.

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

- [x] Asset ID and display name.
- [x] Source path or URL.
- [x] Required files.
- [x] Preview image path or generated preview metadata.
- [x] Declared units and source coordinate system.
- [x] Expected bounds or scale constraints.
- [x] Required anchors/connectors and tags.
- [x] Material/texture requirements.
- [x] License/provenance metadata.
- [x] Optional categories/tags supplied by the host.

Validation checks:

- [x] Load succeeds or fails with structured errors.
- [x] Required external files are present.
- [x] Bounds are finite and within declared limits.
- [x] Units and coordinate system are known or explicitly repaired.
- [x] Required anchors/connectors exist and are well-formed.
- [x] Required material variants exist.
- [x] Missing textures and material fallbacks are reported.
- [x] Preview render can be generated deterministically for `generated`
      preview entries through `render_asset_catalog_preview_png()`.

Scope guard:

- [x] Scena validates and reports. The host owns search, versioning, package
      distribution, database records, approval workflows, and business rules.

Acceptance:

- [x] Fixture catalog with one valid and several invalid assets.
- [x] Readiness report stable fixture.
- [x] Browser preview proof for a catalog asset.
- [x] Example: asset picker feeding a SceneHost scene.

Evidence:

- [x] Test-first proof: `cargo test --test asset_catalog_contracts -- --nocapture`
      failed on the builder before implementation with unresolved
      `ASSET_CATALOG_SCHEMA_V1`, `ASSET_READINESS_REPORT_SCHEMA_V1`,
      `AssetCatalogV1`, `AssetReadinessAssetReportV1`, and
      `Assets::validate_asset_catalog`.
- [x] Focused tests: `asset_catalog_contracts`, `stable_contracts`, and
      `scena_cli_schema` pass locally after implementation.
- [x] Preview test-first proof:
      `cargo test --test asset_catalog_preview -- --nocapture` failed on the
      builder before implementation with unresolved
      `AssetCatalogPreviewError` and `render_asset_catalog_preview_png`.
- [x] Focused preview proof: `cargo test --test asset_catalog_preview -- --nocapture`
      passes locally after implementation.
- [x] Example compile/run proof: `cargo run --example asset_catalog_picker
      --features scene-host -- target/gate-artifacts/asset-catalog-picker`
      produces deterministic preview and SceneHost PNG artifact paths.
- [x] Browser preview test-first proof:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` failed after adding
      the required assertion with `unknown M6 browser workflow probe:
      asset-catalog-preview`.
- [x] Browser preview proof:
      `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg .
      --features browser-probe && SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:m6` passes. The `asset-catalog-preview` result records
      `scena.asset_catalog.v1`, `variant-triangle`, generated 256x256 preview
      metadata, active `midnight` variant, one root, `framed=true`, and 838
      nonblack WebGL2 pixels.

### 3.2 Asset doctor integration API

Owner modules: `assets`, `diagnostics`, `xtask`, `scene_host`.

Foothold: `xtask doctor`, asset doctor checks, diagnostics JSON, capability
reports.

Required behavior:

- [x] Rust API returns doctor-style findings for a loaded asset or asset path.
      Evidence: `Assets::doctor_asset_path()` and
      `Assets::doctor_loaded_asset()` both return `scena.asset_doctor.v1`.
- [x] WASM API returns JSON findings for browser-hosted assets.
      Evidence: browser `SceneHost.assetDoctorJson(url)` delegates to
      `SceneHostCore::asset_doctor_json()`.
- [x] CLI and library diagnostics share codes where checks overlap.
      Evidence: runtime and xtask guidance use shared codes such as
      `unsupported_required_extension`, `extension_supported`, and
      `extension_degraded`.
- [x] Findings include severity, code, path, message, help, and suggested fix.
      Evidence: `AssetDoctorFindingV1` pins those fields and
      `tests/assets/stable-contracts/asset_doctor.v1.json` locks the wire
      shape.
- [x] `ok` semantics are explicit.
      Evidence: `docs/schema-contracts.md` and `docs/assets.md` state that
      `ok=true` means no error-severity findings; warning-severity missing
      external images, optional buffers, and material fallbacks remain visible
      and require strict load options or catalog readiness when completeness is
      mandatory.

Acceptance:

- [x] CLI/library parity tests for representative fixtures.
      Evidence: `cargo test --test asset_doctor_contracts --features
      scene-host -- --nocapture`, `cargo test --test scena_cli_agent
      --features inspection -- scena_doctor_cli_emits_json_and_nonzero_for_broken_asset
      --nocapture`, and `cargo test --test stable_contracts
      asset_doctor_golden_matches_live_schema_serialization -- --nocapture`
      pass locally.
- [x] Browser proof displays doctor findings for a broken asset.
      Evidence: `wasm-pack build --dev --target web --out-dir
      target/m6-browser-pkg . --features browser-probe &&
      SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` includes
      `scena.m6.asset_doctor_browser_proof.v1` and writes
      `target/gate-artifacts/asset-doctor-browser-proof.png`.

### 3.3 Connector browser and snap preview

Owner modules: `scene`, `assets/gltf`, `scene_host`, `geometry`.

Foothold: anchors/connectors, connector metadata, connect helpers, distance,
snap preview, ghost transforms.

Required behavior:

- [x] List connectors for an import, subtree, or selection.
      Evidence: `SceneHostCore::connector_browser_json()`,
      `connector_browser_subtree_json()`, and
      `connector_browser_selection_json()` emit
      `scena.connector_browser.v1` with stable host handles.
- [x] Filter compatible connectors by kind, allowed mates, tags, polarity, and
      roll policy from connector metadata.
      Evidence: `ConnectorBrowserCandidateV1` records compatible and invalid
      candidates, including `incompatible_kind`, `polarity_mismatch`, and
      `tag_mismatch`; roll policy is reported for each connector.
- [x] Preview connection with ghost transform.
      Evidence: compatible candidates include rounded `ghost_transform` and
      `connection_line` fields from the existing connector solver.
- [x] Report snap distance and tolerance.
      Evidence: `distance`, `tolerance`, `snap_ready`, and `visual_cue` are
      pinned in `tests/assets/stable-contracts/connector_browser.v1.json`.
- [x] Report invalid mate reasons.
      Evidence: `tests/connector_browser_contracts.rs` covers compatible and
      incompatible metadata candidates.
- [x] Render optional ghost/outline/line cue.
      Evidence: existing M6 browser proof workflow `connector-magnet-preview`
      renders both out-of-range and snap-ready ghost/line cues with visible
      pixels.

Scope guard:

- [x] Compatibility is metadata-driven. Scena does not compute mesh clearance,
      collision, or physical feasibility.
      Evidence: `scena.connector_browser.v1` reports authored metadata,
      snap-distance preview, and invalid mate reasons only; no mesh clearance
      or physics fields exist in the contract.

Acceptance:

- [x] Tests for compatible/incompatible connector metadata.
      Evidence: `cargo test --test connector_browser_contracts --features
      scene-host -- --nocapture` and
      `cargo test --features scene-host --test connector_browser_contracts`.
- [x] Browser proof for snap-ready and snap-invalid states.
      Evidence: `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` includes
      `connector-magnet-preview`, asserting `scena-magnet-ready`,
      `scena-magnet-out-of-range`, and visible browser pixels.
- [x] Example: assembly connector browser.
      Evidence: `examples/assembly_connector_browser.rs` emits
      `scena.connector_browser.v1` for compatible and incompatible authored
      connector targets; the public guide keeps the UI-facing
      `preview_connector_magnet` example.

### 3.4 Material variant helpers

Owner modules: `assets/gltf`, `scene`, `viewer`, `viewer_element`,
`scene_host`.

Foothold: `KHR_materials_variants` parsing, runtime variant selection, viewer
variant picker.

Required behavior:

- [x] List declared variant names for an import.
      Evidence: `SceneHostCore::material_variants`, viewer
      `material_variants()`, and `<scena-viewer>` picker proof all surface
      `midnight`/`noon` from `material_variants_scene.gltf`.
- [x] Apply variant by name through the 0.1C `material_variants`
      `VisualPatch` channel.
      Evidence:
      `scene_host_material_variant_reports_include_available_and_active_state`
      applies `noon` through `VisualPatchV1.material_variants`.
- [x] Clear variant to default through the same patch channel.
      Evidence: the same test clears the channel with `variant: null` and
      verifies inspection returns `active_variant: null`.
- [x] Report active variant in inspection and asset/import reports.
      Evidence: `scena.scene_host_asset_import.v1` now includes
      `material_variants`/`active_variant`, and host-backed
      `scena.scene_inspection.v1` includes an `imports` section with stable
      import handle, available variants, and current active variant.
- [x] Browser and native APIs have matching behavior and delegate to the same
      patch path.
      Evidence: native SceneHost uses `VisualPatchV1.material_variants`;
      browser picker proof continues through the same runtime
      `Scene::set_active_variant` path and reports the selected active variant.

Acceptance:

- [x] Tests for missing variant, ambiguous variant, clear-to-default, and stale
      import handles.
      Evidence: `cargo test --test material_variant_helpers --features
      scene-host -- --nocapture` and
      `cargo test --features scene-host --test material_variant_helpers`;
      0.1C SceneHost tests cover missing variants, and this slice adds
      duplicate-name ambiguity plus stale-import-handle coverage.
- [x] Browser proof for variant switch with visible pixel change.
      Evidence: `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` includes
      `scena-viewer-material-variant-render`, selected/active `noon`, visible
      non-background pixels, and a green-dominant output assertion.

### 3.5 Product configurator helpers

Owner modules: `viewer`, `scene_host`, `scene`, `assets/gltf`.

Foothold: material variants, visibility, tint, camera bookmarks, visual patch.

Required behavior:

- [x] Option group model over visual changes only.
      Evidence: `ProductOptionsV1`, `ProductOptionGroupV1`, and
      `ProductOptionV1` store host-authored groups whose options contain
      `VisualPatchV1` and opaque metadata only.
- [x] Apply option as `VisualPatch`.
      Evidence: `SceneHostCore::apply_product_option` delegates to
      `apply_patch`; the active option updates only when
      `VisualPatchResultV1.failed` is empty.
- [x] Report active visual options.
      Evidence: `SceneHostCore::product_options_json()` returns
      `scena.product_options.v1` with per-group `active` option ids.
- [x] Example config can combine variant, tint, visibility, and camera state.
      Evidence: `examples/product_configurator.rs` applies a material variant,
      tint, camera state, and visibility option through stored product options.

Scope guard:

- [x] No pricing, compatibility business rules, inventory, persistence, or
      domain-specific configuration logic.
      Evidence: product options store only ids, labels, opaque metadata, and
      `VisualPatchV1`; Scena never interprets host business rules.

Acceptance:

- [x] Example: product configurator with material and visibility options.
      Evidence: `cargo run --example product_configurator --features
      scene-host`.
- [x] Stable JSON example for option groups if public.
      Evidence: `tests/assets/stable-contracts/product_options.v1.json` and
      `cargo test --test product_configurator_helpers --features scene-host
      -- --nocapture`.

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

- [x] Host-ticked `seek(t)` and `advance(dt)`.
      Evidence: `SceneHostCore::seek_timeline` and
      `SceneHostCore::advance_timeline` consume
      `scena.presentation_timeline.v1`; `advance_timeline` is a wrapper over
      `seek_timeline(current_seconds + delta_seconds)` and stores no hidden
      player state.
- [x] Actions compose camera bookmarks, visual states, animation clips,
      transforms, tints, labels, and annotations.
      Evidence: `PresentationTimelineActionKindV1` supports direct
      `VisualPatchV1`, stored visual states, named camera bookmarks, and
      animation mixer sampling; labels/annotation anchors travel through the
      existing `VisualPatchV1.labels` channel.
- [x] Timeline emits `VisualPatch` for the requested time and does not define a
      parallel mutation model.
      Evidence: `SceneHostCore::timeline_patch` flattens due actions into a
      deterministic last-wins `VisualPatchV1`; `seek_timeline` applies that
      patch through `apply_patch`.

Scope guard:

- [x] No autonomous loop. No application workflow engine.
      Evidence: the API takes explicit target seconds or
      current/delta seconds from the host and never owns a render loop,
      business workflow, or persistent timeline player.

Acceptance:

- [x] Deterministic seek tests.
      Evidence: `cargo test --test presentation_timeline --features
      scene-host -- --nocapture` covers flattened last-wins seek, idempotent
      replay, and host-ticked animation sampling.
- [x] Browser proof for guided tour.
      Evidence: `rustup run 1.95.0 wasm-pack build . --dev --target web
      --out-dir target/scene-host-browser-pkg --out-name scena --features
      scene-host` and `SCENA_BROWSER_BACKENDS=webgl2 npm run
      browser:scene-host-proof` passed manually on V3D WebGL2. The proof asserts
      `timelinePatchJson` / `seekTimelineJson` emit and apply a guided-tour
      patch with camera, tint, label, and animation sampling, then render
      nonblank pixels.

## Phase 4 - Fidelity and browser reach

Goal: raise output trust and reach web developers without bypassing the host
contracts from Phase 0.

### 4.1 Contact grounding preset

Owner modules: `render`, `scene`, `viewer`, `diagnostics`.

Foothold: SSAO, post-processing, grid/floor helpers, lighting presets,
directional shadow capability tracking.

Required behavior:

- [x] A product-viewer grounding preset that combines floor/receiver setup,
      SSAO where available, a proven shadow receiver path where supported, and
      lighting defaults where capabilities allow.
- [x] SSAO alone is ambient occlusion and cannot by itself close
      contact/drop-shadow grounding.
- [x] Grounding report states which grounding path is active.
- [x] Fallback is explicit when backend capability is unavailable.

Acceptance:

- [x] Headless and browser visual proof that a product asset is visibly
      grounded.
- [x] No claim of physical shadow correctness without proof.

Evidence: test-first proof failed on `scena-builder` with unresolved
`SCENE_HOST_GROUNDING_SCHEMA_V1`, `SceneHostGroundingPathV1`,
`apply_product_grounding_preset`, and `apply_product_grounding_preset_json`
before implementation. `SceneHostCore::apply_product_grounding_preset()` and
the browser `applyProductGroundingPresetJson()` now return
`scena.scene_host_grounding.v1` with `active_paths` containing
`floor_receiver` and `screen_space_ambient_occlusion`, plus an
`ssao_is_ambient_occlusion` fallback that prevents claiming physical shadow
correctness; the stable fixture is
`tests/assets/stable-contracts/scene_host_grounding.v1.json`. Headless proof:
`cargo test --test contact_grounding --features scene-host,inspection -- --nocapture`
writes
`target/gate-artifacts/contact-grounding/headless-product-grounding.png` and
asserts non-background grounded receiver content plus an SSAO pass. Browser
proof: `tests/browser/scene_host_browser_proof.js` asserts the same report
shape, a nonblank WebGL2 render, and `ambient_occlusion_passes >= 1`.

### 4.2 Directional shadow proof closure

Owner modules: `render/gpu`, `scene`, `diagnostics`, visual proof harness.

Foothold: shadow map resources, directional light shadow flags, degraded
capability status.

Required behavior:

- [x] Render shadow casters into a shadow map.
- [x] Sample shadows into receiver pixels.
- [x] Validate one shadowed directional light limit or expand it deliberately.
- [x] Capability status moves from degraded to supported only after visible
      receiver proof.

Acceptance:

- [x] Unit tests for shadow preparation and single-shadowed-light errors.
- [x] Rendered proof shows receiver darkening and stable non-shadow regions.
- [x] Capability report and diagnostics reflect actual backend support.

Evidence: test-first proof
`cargo test --test m2_lighting_depth_clipping directional_shadow_capability_matches_proven_receiver_sampling_backends -- --nocapture`
failed on `scena-builder` because the `HeadlessGpu` GPU-device row still
reported `Degraded` before implementation. `Capabilities::for_gpu_backend()`
and attached GPU rows now report `directional_shadows: supported` for
`HeadlessGpu`, `NativeSurface`, `WebGpu`, and `WebGl2`, while CPU/reference and
unattached factory rows remain `degraded` with a lane-specific diagnostic.
Existing preparation and visual gates cover the rest of the contract:
`shadowed_directional_light_is_opt_in_and_single_owner`,
`single_shadow_map_records_pcf3x3_prepare_stats`,
`directional_shadow_receiver_pixels_are_darkened_by_caster`,
`headless_gpu_directional_shadow_visibility_darkens_receiver_when_available`,
and the browser `pbr-shadow-visibility` assertion from
`wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
plus `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6`.

### 4.3 Physical glass and transmission

Owner modules: `material`, `render/gpu`, `assets/gltf`, diagnostics, visual
proof harness.

Foothold: transmission/IOR/volume parsing, material descriptors, transmission
resources, capability gate.

Required behavior:

- [x] Scene-color transmission.
- [x] IOR/thickness refraction approximation.
- [x] Rough-transmission blur.
- [x] Transparency ordering strategy documented and proven per backend.
- [x] Required glTF transmission/volume assets fail or degrade explicitly when
      backend proof is unavailable.

Acceptance:

- [x] Material tests for parsed factors and textures.
- [x] GPU/browser proof for clear and frosted glass.
- [x] Capability report does not overclaim unsupported lanes.

Evidence: test-first stale-policy proof
`cargo test -p xtask asset_doctor_native_guidance_reports_required_transmission_with_capability_fix -- --nocapture`
failed on `scena-builder` because `asset-doctor` still said GPU/browser proof
was "not release-proven"; `cargo test --test m8_assets_materials_ecosystem m8_optional_real_world_gltf_extensions_report_degradation_metadata -- --nocapture`
failed for the same stale import diagnostic. The guidance now points required
transmission/IOR/volume assets at `physical_glass_transmission=supported`
capability rows and fallback materials for unsupported lanes. Existing shader
and render-path tests prove the physical path:
`transmission_scene_color_does_not_reuse_final_depth_prepass`,
`triangle_shader_applies_scene_color_transmission_in_native_and_webgl2_variants`,
`unlit_pipeline_can_split_opaque_and_transparent_draws_for_transmission`, and
`round_e_glass_ordering_uses_non_overlapping_opaque_target_alternative`.
Material parsing and texture proof are covered by
`m8_transmission_ior_volume_material_factors_are_parsed_from_gltf`,
`m8_transmission_volume_textures_affect_cpu_preview_pixels`, and
`m8_headless_gpu_transmission_volume_ibl_capability_when_available`. Reference
visual proof asserts clear-glass refraction offset and frosted-glass edge
contrast in `round_e_material_reference_docs_image_metrics`; browser
proof is the `pbr-material-presets` WebGL2 workflow from
`wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
plus `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6`, which records
`glass_contract = scene-color-ior-thickness-rough-blur-sorted-transparency`.
Capability proof is pinned by
`capability_matrix_reports_hardware_tier_and_backend_feature_states`: attached
GPU-device `HeadlessGpu`, `NativeSurface`, `WebGpu`, and `WebGl2` rows can
report `physical_glass_transmission: supported`; CPU/reference and unattached
factory rows remain `degraded`.

### 4.4 Dense WebGL2 source-material proof

Owner modules: `assets`, `render`, `scene_host`, browser proof harness.

Foothold: source material import, browser proof harness, external resource
loading, renderer-fidelity checklist.

Required behavior:

- [x] Dense imported glTF/GLB fixture with source materials, external textures,
      normals, metallic/roughness, camera framing, and lighting.
- [x] Browser WebGL2 render path preserves source materials.
- [x] Proof distinguishes source material, generated unlit fallback, and PBR
      override paths.

Acceptance:

- [x] Browser output has non-background pixels and material-specific predicates.
- [x] Proof records backend, capabilities, resource warnings, stats, and
      screenshot metadata.
- [x] Capability promotion cites the exact artifact.

Evidence: test-first browser proof hardening
`SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` failed after
`assertSourceGltfMaterialProof` was tightened to require explicit source
texture roles plus camera-framing and lighting metadata; the old
`source-gltf-materials` artifact only recorded `source_texture_bindings = 5`.
The workflow now emits `source_texture_roles = [base_color, normal,
metallic_roughness, occlusion, emissive]`, `camera_framing = Scene::frame`,
and `lighting = DirectionalLight` from the real WaterBottle
`SceneAsset::nodes mesh.geometry mesh.material` construction. The assertion
now also checks lane-specific rendered pixels: the generated-unlit lane is
cyan/blue, the source glTF material lane is bright and distinct from both
generated lanes, and the generated-PBR lane is warm. Green browser proof:
`wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
plus `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6`; the artifact
`target/gate-artifacts/m6-rust-wasm-renderer-probe.json` has
`workflow = source-gltf-materials`, `proof_class =
browser-source-gltf-material-comparison`, `source =
/fixtures/gltf/khronos/WaterBottle/WaterBottle.gltf`, `load_warnings = 0`,
`material_texture_bindings = 5`, `material_textures_missing_decoded_pixels =
0`, `triangles = 13530`, `screenshot_metadata.backend = webgl2`,
`fixture_sha256 =
0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547`, and
lane-specific left/center/right pixels for the `generated-unlit`,
`source-gltf-material`, and `generated-pbr` comparison lanes. Capability
promotion is the existing attached WebGL2 row in the same artifact with
`forward_pbr = supported`; CPU/reference and unattached factory rows remain
capability-gated elsewhere.

### 4.5 `<scena-viewer>` parity

Owner modules: `viewer_element`, `scene_host`, `platform/browser`, `assets`,
`render`.

Foothold: custom element, `SceneHost` JSON APIs, viewer element annotations,
drag/drop, variant picker, browser proof.

Required attributes/APIs:

- [x] `src`.
- [x] `environment`.
- [x] Lighting preset or direct light JSON, mapped to real `Scene` light APIs.
- [x] Camera/framing attributes and methods.
- [x] Capture/download methods.
- [x] Picking, hover, and selection events through `HostEvent`.
- [x] Material variants.
- [x] Annotation slots and projection.
- [x] Drag/drop with load diagnostics.
- [x] Inspector and diagnostics surfaces.
- [x] Visual patch application.

Scope guard:

- [x] The custom element consumes `VisualPatch` and `HostEvent`. It must not
      create a parallel JS-only scene model.

Acceptance:

- [x] Browser proof for lighting/environment/camera/capture/variant/picking and
      annotation parity.
- [x] Side-by-side model-viewer style proof for representative assets.
- [x] Mobile/a11y proof remains green.

Evidence: test-first browser proof hardening
`SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6` failed after
`assertScenaViewerElementProof` was tightened to require a host-bound custom
element with `VisualPatch` application, `HostEvent` DOM dispatch,
capture/download, studio lighting, framing, and camera delegates; the old
artifact lacked `host_adapter_bound`. The element now keeps the browser layer
thin: `bindHost()` stores a `SceneHost`-shaped object, `applyPatch()` /
`applyVisualPatch()` forward `scena.visual_patch.v1` to `host.applyPatch()`,
`drainHostEvents()` dispatches drained `scena.host_event.v1` entries as
`scena-viewer-host-event` plus specific events, and the capture, picking,
hover, selection, framing, camera, and studio-lighting helpers all delegate to
the existing `SceneHost` method names instead of maintaining JS scene state.
Green proof: `wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe`
plus `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:m6`. The artifact
`target/gate-artifacts/m6-rust-wasm-renderer-probe.json` records
`scena.scena_viewer_element_browser_proof.v1` with
`host_adapter_bound = true`, `visual_patch_applied_visibility = 1`,
`host_event_schema = scena.host_event.v1`, host event kinds
`selection_changed`, `pick`, `hover`, and `capture_ready`, DOM events
`scena-viewer-host-event`, `scena-viewer-pick`, `scena-viewer-hover`,
`scena-viewer-selection-changed`, and `scena-viewer-capture-ready`,
`capture_png_bytes = 8`, `download_file_name = scena-viewer-proof.png`,
`lighting_preset_background = studio`, `frame_method = frameAll`, and
`camera_method = setCameraJson`. The same gate also preserves variant render,
annotation layout, drag/drop render-after-drop, side-by-side
`scena.scena_viewer_model_viewer_parity_proof.v1`, and
`scena.scena_viewer_mobile_a11y_browser_proof.v1`.

## Phase 5 - Demand-driven IO completeness

Goal: add expensive or borderline IO features only when they unblock real
assets or real application workflows.

### 5.1 Draco decode

Owner modules: `assets/gltf`.

Foothold: structured degraded extension diagnostics for
`KHR_draco_mesh_compression`.

Acceptance:

- [x] **[deferred, demand-driven]** Decoder dependency is not selected until
      a real user asset requires Draco. Current policy remains structured
      degradation for optional `KHR_draco_mesh_compression` and explicit
      failure for required usage.
- [x] Required Draco assets load or fail with explicit structured error.
- [x] Browser/native support matrix documented: both native and browser lanes
      use the same no-decoder policy today; optional Draco reports degraded
      metadata, required Draco fails.
- [x] **[deferred, demand-driven]** Real fixture proof is required before a
      decoder can be adopted; no current checklist fixture needs Draco.

Evidence:

- [x] Source policy: `src/assets/gltf/extensions.rs` reports Draco as a
      future external decoder feature and suggests re-exporting uncompressed
      or with `EXT_meshopt_compression`.
- [x] Test proof: `m8_optional_real_world_gltf_extensions_report_degradation_metadata`
      checks optional Draco diagnostics, and
      `m8_real_world_fixture_matrix_covers_asset_edge_cases` checks required
      Draco fails with `UnsupportedRequiredExtension`.
- [x] Docs proof: `docs/assets.md` states the current native/browser Draco
      support matrix and demand gate.

### 5.2 glTF or configuration export

Owner modules: `assets`, `scene`, `scene_host`.

Scope:

- [x] **[deferred, demand-driven]** Export visual configuration or a narrow
      glTF subset only when a real saved-viewer-state workflow requires it.
- [x] Do not export a CAD document model, product database, or full authoring
      suite.

Acceptance:

- [x] **[deferred, demand-driven]** Export/import round trip for transforms,
      visibility, variants, camera, and annotations is not implemented without
      a driving workflow and dedicated exporter design.
- [x] **[deferred, demand-driven]** Unsupported-field reporting is required
      for any future exporter; silent dropping is not allowed.

Evidence:

- [x] Scope proof: `docs/checklists/wasm-scene-host-and-stable-contracts.md`
      keeps a glTF/GLB scene exporter as a separate exporter epic requiring
      writer dependency selection, binary-buffer assembly, import-export
      round-trip tests, and topology/transform preservation proof.
- [x] Existing non-export alternative: visual state is already expressible
      through `scena.visual_patch.v1`, named visual states, camera bookmarks,
      and presentation timelines. Persisting an application document remains
      host-owned.

### 5.3 Hot reload polish

Owner modules: `assets`, `scene`, `scene_host`.

Foothold: `hot-reload` feature and asset retain/reload policy.

Required behavior:

- [x] Existing native reload path reloads retained scene assets explicitly
      through `Assets::reload_scene` and `Scene::replace_import`.
- [x] **[deferred, demand-driven]** Stable SceneHost import/node mapping
      preservation across reload requires a host-level source-identity design
      and is not added without a live workflow that needs it.
- [x] **[deferred, demand-driven]** Reload reports describing preserved,
      replaced, removed, and stale handles land with the host-level mapping
      design.
- [x] **[deferred, demand-driven]** Host reload-result events land with the
      host-level reload report; the current asset watcher remains an asset
      boundary helper and does not hide reload/prepare/render inside
      `render()`.

Acceptance:

- [x] Existing tests cover reload retain-policy, stale import lookups, stale
      connector handles, variant rebinding, retained source bytes, and a
      rendered before/after hot-reload proof.
- [x] Existing example/doc workflow: native hot reload is documented in
      `docs/guides/easy-scene-setup.md`; `tests/round_d_asset_hot_reload.rs`
      writes the live reload visual proof.

Evidence:

- [x] `tests/round_d_asset_hot_reload.rs` writes a retained glTF to disk,
      watches it, edits it, drains a debounced `AssetPath`, reloads through
      `Assets::reload_scene`, replaces the import, prepares, renders, and
      writes
      `target/gate-artifacts/asset-hot-reload/asset-hot-reload-animated-proof.ppm`.
- [x] `tests/m3a_app_features.rs::reload_scene_requires_retain_and_reprepare_after_replace_import`
      proves reload requires `RetainPolicy::Always` and explicit reprepare
      after replacement.
- [x] `tests/m7_threejs_ergonomics.rs::m7_stale_import_connector_handle_after_hot_reload_is_detected`
      and `tests/m8_stale_handle_proof.rs` keep stale-handle behavior
      structured until a host-level preservation report lands.

## Continuous examples and acceptance apps

These examples are not optional demos. Each phase above should add or update at
least one example that proves the APIs compose into an application workflow.

- [x] CAD inspection viewer: part tree, picking, fit selection, section box,
      measurements, annotations, isolate/ghost.
      Evidence: `examples/cad_inspection_viewer.rs` covers show-only,
      isolate/ghost, selected-node framing, helper overlays, and measurement
      overlays; Phase 2 section-box, callout, annotation-layout, and browser
      inspection proof are recorded in the Phase 2 evidence above.
- [x] Industrial dashboard viewer: visual patch stream, event stream,
      diagnostics, labels, stable capture.
      Evidence: `examples/industrial_static_scene.rs` proves the industrial
      profile and labels visually, while `examples/scene_host_contracts.rs`
      exercises the dashboard host loop: visual patch application, labels,
      diagnostics/capabilities/inspection reports, host events, render, and
      stable capture JSON.
- [x] Product configurator: variants, option groups, camera bookmarks, PNG
      export, appearance assertions, glass/grounding when available.
      Evidence: `examples/product_configurator.rs` stores
      `scena.product_options.v1` and applies variants, tints, visibility, and
      camera through `VisualPatchV1`; `scena examples agent
      product-configurator` runs render introspection plus
      `verify appearance`; product grounding and transmission evidence are
      recorded under Phase 4.
- [x] Live-state playback viewer: host-ticked visual patches,
      animation time, temporal assertions, stable proof captures, no domain
      logic in Scena.
      Evidence: `scena examples agent live-state-viewer` validates and renders
      a host-authored recipe through the CLI; SceneHost visual patches,
      animation time, and host-ticked timeline/animation assertions are covered
      by A.9 and Phase 3.6 without adding domain runtime logic.
- [x] Headless documentation renderer: deterministic views, callouts,
      contact sheets, baseline comparison.
      Evidence: `examples/headless_documentation_renderer.rs` writes a
      deterministic PNG plus `scena.capture.v1` descriptor with callouts and
      leader lines; capture proof/baseline utilities and contact-sheet style
      artifacts are covered by Phase 0.3 and the rendered proof harness.
- [x] Agent render loop template: recipe, render introspection, visibility
      diagnosis, suggested fix, and rerender from CLI JSON.
      Evidence: `scena examples agent product-configurator`,
      `live-state-viewer`, `web-viewer`, `data-visualization`,
      `animated-viewer`, and `interaction-proof` generate CLI-only recipes,
      expectations, command manifests, artifacts, and `ok=true` reports; A.2
      and A.7 provide the diagnosis/repair reports and non-converging
      fail-closed shape.
- [x] Data visualization viewer: color-ramp assertions, labels, camera
      bookmarks, and capture proof.
      Evidence: `scena examples agent data-visualization` provides the
      CLI-rendered smoke recipe; A.8 proves generated data-color ramp swatches
      by target `node_bbox` without a golden image; labels, camera bookmarks,
      and capture proof are covered by Phase 2.7 and Phase 1.1/0.3.
- [x] Animated viewer: clip inventory, temporal introspection, sampled captures,
      and final-state proof.
      Evidence: `scena examples agent animated-viewer` emits an animated glTF
      recipe and `verify animation` command over sampled times; A.9 records
      clip inventory, sampled transform assertions, captured frames, frozen
      channel detection, and fail-closed wrong-translation fixtures.
- [x] Interaction proof viewer: synthetic pick, hover, selection, and rendered
      feedback assertions.
      Evidence: `scena examples agent interaction-proof` runs synthetic hover
      and selection assertions from CLI JSON; `examples/picking_selection_hover.rs`
      plus `tests/examples_visual_proof.rs` render the hover/selection feedback
      path to a proof artifact; the browser SceneHost proof asserts matching
      stable handles in browser events.
- [x] `<scena-viewer>` browser app: picking, annotations, variants, capture,
      drag/drop, diagnostics.
      Evidence: Phase 4.5 records the `<scena-viewer>` browser proof for
      host-event dispatch, picking, hover, selection, capture/download,
      material variants, annotations, drag/drop render-after-drop, diagnostics,
      model-viewer parity, and mobile/a11y proof.
- [x] Guided tour: bookmarks, callouts, exploded view, presentation timeline.
      Evidence: `examples/guided_exploded_view.rs` writes assembled, exploded,
      and restored frames; Phase 1.1 records camera bookmark/fly-to proof;
      Phase 2.7 records callout/leader-line proof; Phase 3.6 records
      host-ticked presentation timeline seek/advance and browser proof.
- [x] SceneHost host-loop template: native and browser versions showing
      mutation, prepare, render, event drain, and capture.
      Evidence: `examples/scene_host_contracts.rs` covers the native
      SceneHost loop and JSON contracts; `examples/scene_host_browser_contracts.js`
      and `tests/browser/scene_host_browser_proof.js` cover the browser loop,
      including visual patches, prepare/render, synthetic interaction, event
      drain, and capture-ready events.

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

The A.11 CAD-inspection and documentation-renderer templates are runnable CLI
smoke templates for load, render introspection, visibility diagnosis, section
boxes, measurements, callouts, and exploded views. Their manifests keep CAD
kernels, drawing import, page layout, prose generation, and richer application
state explicitly host-owned.

Do not start Phase 4 browser reach work by adding ad-hoc custom-element
behavior. The element should inherit the stable patch, event, asset, capture,
and annotation contracts from earlier phases.

## Definition of done for each item

- [x] Scope guard reviewed against `docs/RFC-rust-3d-renderer.md`.
- [x] Owner module named.
- [x] Public API or schema sketched before implementation.
- [x] Test-first proof or deterministic-proof exception recorded.
- [x] Native Rust tests pass.
- [x] WASM build proof exists for browser-exposed APIs.
- [x] Browser rendered-output proof exists for browser-visible visuals.
- [x] Stable fixture exists for public JSON contracts.
- [x] Docs and examples updated.
- [x] Doctor rule added when source/docs/artifact drift is mechanically
      detectable.
- [x] Release notes/changelog updated only when the implementation lands.

Evidence: each implementation section names its owner modules, proposed
contracts, and focused proof. Public JSON contracts have stable fixtures under
`tests/assets/stable-contracts/` or CLI golden fixtures under
`tests/assets/cli-golden/`; schema catalog drift is enforced by doctor.
Browser-exposed APIs cite wasm/browser proof artifacts in their sections, while
non-browser or docs-only entries record deterministic-proof exceptions.
`browser:m6` is the CI/release-enforced browser lane; `browser:scene-host-proof`
is manual V3D WebGL2 evidence unless a section explicitly states otherwise. The
final closure gate reruns `cargo run -p xtask -- doctor --full` on
`scena-builder`; code-bearing slices above record their focused and broad
native Rust gates. `CHANGELOG.md`, `docs/schema-contracts.md`,
`docs/api.md`, `docs/examples.md`, and this checklist carry the public surface
updates without adding release-note entries for docs-only evidence changes.
