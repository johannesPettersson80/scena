# Stable JSON contract policy

Status: active policy for new public JSON contracts
Date: 2026-06-01

This document defines how `scena` versioned JSON contracts are named, evolved,
tested, and documented.

## Schema names

Every public JSON report carries a top-level `schema` string:

```json
{ "schema": "scena.<contract>.vN" }
```

Rules:

- Prefix: `scena`.
- Contract name: lowercase snake-case semantic name.
- Version suffix: `vN`, starting at `v1`.
- Example names:
  - `scena.scene_inspection.v1`
  - `scena.capability_report.v1`
  - `scena.schema_catalog.v1`
  - `scena.schema_entry.v1`
  - `scena.capture.v1`
  - `scena.capture_baseline.v1`
  - `scena.render_introspection.v1`
  - `scena.subject_observation.v1`
  - `scena.render_quality.v1`
  - `scena.scene_composition.v1`
  - `scena.visibility_diagnosis.v1`
  - `scena.visual_repair_plan.v1`
  - `scena.agent_loop_result.v1`
  - `scena.agent_smoke_template.v1`
  - `scena.agent_template_catalog.v1`
  - `scena.q01.required_webgpu_pixel_parity.v1`
  - `scena.q04.required_gpu_resource_lifecycle.v1`
  - `scena.appearance_expectation.v1`
  - `scena.appearance_introspection.v1`
  - `scena.animation_introspection.v1`
  - `scena.interaction_expectation.v1`
  - `scena.interaction_verification.v1`
  - `scena.connector_browser.v1`
  - `scena.scene_recipe.v1`
  - `scena.scene_recipe_validation.v1`
  - `scena.scene_recipe_build.v1`
  - `scena.photo_render_result.v1`
  - `scena.photo_plan.v1`
  - `scena.photo_candidate_plan.v1`
  - `scena.photo_shaded_candidate_selection.v1`
  - `scena.photo_report.v1`
  - `scena.placement_result.v1`
  - `scena.asset_load_report.v1`
  - `scena.asset_geometry_summary.v1`
  - `scena.asset_conversion.v1`
  - `scena.annotation_projection.v1`
  - `scena.subtree.v1`
  - `scena.scene_host_visual_state.v1`
  - `scena.scene_host_visual_states.v1`
  - `scena.animation_inventory.v1`
  - `scena.visual_patch.v1`
  - `scena.host_event.v1`

## Compatibility

Compatible within the same `vN`:

- adding optional fields,
- adding enum variants only when consumers are already required to handle
  unknown values,
- adding nested optional metadata with documented defaults,
- relaxing field value constraints without changing field meaning.

Requires a new version:

- renaming or removing a field,
- changing field type,
- changing units, coordinate space, or handle namespace,
- changing enum values in a way that old consumers cannot parse,
- making an optional field required,
- changing the meaning of a field without a new name.

## Stable handles

Wire reports must not serialize raw `NodeKey`, `CameraKey`, `MaterialHandle`,
`GeometryHandle`, or other slotmap/internal handles.

When a report is emitted by `SceneHost`, the host's kind-tagged,
generation-checked `u64` handle namespaces are authoritative. The same node
handle must be accepted by node mutation APIs and appear in host-backed
inspection reports. Import, instance-root, and animation handles have distinct
tags and must be passed only to compatible APIs. All valid values remain exact
JavaScript integers; consumers must still treat their representation as opaque.

Standalone native reports may allocate deterministic report-local handles. Those
IDs are stable only within that report unless a caller supplies an explicit
mapping.

## Field naming and values

- Field names use `snake_case`.
- Numeric vectors use arrays in math order, for example `[x, y, z]` and
  `[x, y, z, w]`.
- Coordinate spaces must be named. Picking and browser viewport inputs use CSS
  pixels unless a contract explicitly says otherwise.
- Large byte payloads should normally be returned outside JSON. JSON carries
  length, format, dimensions, and hash metadata.
- Renderer/backend-specific proof must include backend/capability metadata
  rather than implying cross-machine byte identity.

## Golden fixtures

Each new contract needs:

- a small stable fixture under `tests/assets/` or another documented fixture
  path,
- a serialize/deserialize test when deserialization is public,
- a schema-string test,
- a snapshot/golden JSON test for the smallest representative scene/report,
- a negative or stale-handle test when the contract includes handles.

The shipped v1 fixtures for this track live under
`tests/assets/stable-contracts/`. `tests/stable_contracts.rs` parses those
fixtures, asserts their schema strings or nested value fields, and checks that
each fixture deserializes through the live Rust contract and serializes back to
the same JSON. When a contract shape intentionally changes, regenerate or edit
the matching fixture in the same commit and review the JSON diff as part of the
public API change.

## Doctor coverage plan

`xtask doctor` should enforce these contract surfaces as they land:

- schema strings appear in source and docs,
- docs link to the generated examples and fixture paths,
- feature flags named in docs match `Cargo.toml`,
- golden fixtures exist for every shipped contract,
- public contract docs avoid domain-specific vocabulary except in explicit
  non-goal or denylist sections,
- browser-visible contracts have WASM build/probe evidence.

## Stable serde value contracts

Some public values are embedded in Rust API results rather than emitted as a
top-level JSON report. These do not carry a `schema` field by themselves, but
their serde field names are still external contracts.

### `AssetProvenance`

Returned by `SceneAsset::provenance`, `TextureDesc::provenance`, and
`EnvironmentDesc::provenance`.

Required fields:

- `source_path`
- `source_sha256`
- `license`
- `generator`
- `derivatives`

`source_sha256`, `license`, and `generator` are nullable because not every
asset source has bytes or declared licensing metadata available at load time.
`derivatives` is an array of `{ "path", "sha256" }` entries for generated
assets derived from the source, such as bundled environment cubemaps and BRDF
LUT fixtures.

Small example:

```json
{
  "source_path": "models/cell.glb",
  "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  "license": null,
  "generator": null,
  "derivatives": []
}
```

### `scena.subtree.v1`

Produced by `SceneHostCore::subtree_nodes_json` and the matching WASM
`SceneHost.subtreeNodesJson()` method. Represented by
`SceneHostSubtreeReportV1`.

Required top-level fields:

- `schema`
- `nodes`

Each node entry contains the stable host `handle`, report-local `parent`,
direct `children`, optional `name`, and sorted `tags` for the requested
subtree. The requested subtree root has `parent: null`; child order follows
the scene hierarchy.

In 1.7, subtree node `name` is reserved for a future stable naming policy and
is always serialized as `null`. Use `tags` or host-owned handles for stable
identification in this release.

Small example:

```json
{
  "schema": "scena.subtree.v1",
  "nodes": [
    { "handle": 42, "parent": null, "children": [84], "name": null, "tags": ["frame", "product"] },
    { "handle": 84, "parent": 42, "children": [], "name": null, "tags": ["part"] }
  ]
}
```

### `scena.animation_inventory.v1`

Produced by `SceneHostCore::animation_inventory_json` and the matching WASM
`SceneHost.animationInventoryJson()` method. Represented by
`SceneHostAnimationInventoryV1`.

Required top-level fields:

- `schema`
- `clips`

Each clip entry contains the import-local `name`, `duration_seconds`, and
`channel_count`. Use the returned name with `SceneHostCore::play_animation` or
`SceneHost.playAnimation()`.

Small example:

```json
{
  "schema": "scena.animation_inventory.v1",
  "clips": [
    { "name": "MoveMount", "duration_seconds": 1.0, "channel_count": 1 }
  ]
}
```

### `scena.visual_patch.v1`

Accepted by `SceneHostCore::apply_patch`, `SceneHostCore::apply_patch_json`,
and the matching WASM `SceneHost.applyPatch()` method. Represented by
`VisualPatchV1`; applying it returns `VisualPatchResultV1` with the same
schema string.

Required top-level fields:

- `schema`

The 0.1A envelope fields are additive and default when omitted:

- `transforms`
- `tints`
- `visibility`
- `camera`

The 0.1B easing/time fields are also additive and default when omitted:

- `transforms_eased`
- `tints_eased`
- `camera_eased`
- `animation_time`

The 0.1C app/UI fields are additive and default when omitted:

- `selection`
- `hover`
- `material_variants`
- `labels`
- `section_box`
- `metadata`
- `echo_metadata`

Eased channels schedule presentation transitions. They become visible only when
the host calls `advance(...)`, and their `applied.*_eased` counts mean
"transition scheduled", not "frame pixels already changed"; `animation_time`
entries sample the requested mixer time immediately.

`tints_eased` targets must be `null` or finite opaque colors with `a == 1.0`.
Opacity fades are rejected because they would cross the opaque/transparent
rendering boundary mid-transition.

Node-targeted patch entries use the stable host `u64` node handle namespace.
The same handle values are accepted by existing `SceneHost` mutation APIs and
appear in host-backed `scena.scene_inspection.v1` reports. Import-targeted
entries, such as `material_variants`, use stable SceneHost import handles.

`selection` and `hover` are programmatic host-owned interaction state. They
accept `{ "node": <handle> }` to target a node or `{ "node": null }` to clear
the state. Pointer-driven hover and pick observations still flow out through
`scena.host_event.v1`. In 1.7 these patch fields are node-only; an
`instance_root` hit handle reported by `scena.host_event.v1` is observational
and fails closed if submitted as a `selection` or `hover` node handle.

`material_variants` entries reference a stable import handle and a source
`KHR_materials_variants` name, or `null` to restore default materials. Unknown
variant names and duplicate source variant names fail closed as per-entry
errors; Scena does not choose one duplicate by declaration order.

`labels` entries are host-owned overlay label/annotation anchors. Scena stores
and projects the anchor; the host owns visible text and DOM/native overlay
content. A label target can be `node`, `world`, or `clear`; IDs must be
non-empty strings and node targets use stable host node handles.

`section_box` is the additive section/cutaway channel. It accepts
`{ "mode": "set", "min": [x,y,z], "max": [x,y,z], "margin": n,
"inverted": false, "helper_wireframe": true }`, `{ "mode": "invert",
"inverted": true }`, or `{ "mode": "disable" }`. Bounds are world-space AABBs
with finite `min < max` on every axis. Non-inverted boxes keep the interior and
clip the outside; inverted boxes clip the interior and keep the outside. The
optional helper wireframe is a generated SceneHost node and is removed when a
later section-box update disables the helper or the section box.

A section box sections **model geometry**. It does not remove annotations.
Labels, callout leader lines, and measurement/dimension lines are presentation
overlays that carry the very information a section view exists to communicate,
so they stay legible when the box excludes their anchor. Opt an individual
label into being cut with the model using
`LabelDesc::with_scene_clipping(true)`; generated callout and measurement
overlays are always exempt. Active clipping planes follow the same rule.

`metadata` is caller-owned JSON. It is returned in `VisualPatchResultV1` only
when `echo_metadata` is `true`, so agents can correlate responses without
forcing every result to echo arbitrary host data.
SceneHost helper-generated patches may also use this field for machine-readable
helper data while keeping the top-level contract as `scena.visual_patch.v1`.
`SceneHostCore::exploded_view_patch_json()` stores
`metadata.scena_exploded_view_restore_patch`, an immediate-transform
`VisualPatchV1` that restores the pre-exploded local transforms for JSON/WASM
hosts.

The result includes:

- `applied`: changed-entry counts for `transforms`, `tints`, `visibility`,
  `camera`, `transforms_eased`, `tints_eased`, `camera_eased`,
  `animation_time`, `selection`, `hover`, `material_variants`, `labels`, and
  `section_box`;
- `failed`: per-entry errors with `channel`, `index`, optional `handle`, typed
  `code`, and human-readable `message`;
- `revisions`: scene revision deltas for `structure`, `transform`,
  `appearance`, `visibility`, and `interaction`.
- optional `metadata`, present only when requested with `echo_metadata`.

Invalid or stale handles fail closed in `failed[]` for that entry. Other valid
entries in the same patch may still apply. A patch with no effective changes is
valid and returns zero changed counts and zero revision deltas. A successful
no-op entry is neither counted in `applied` nor listed in `failed`; it is
reported by the absence of changed counts, failures, and revision deltas.

Small input example:

```json
{
  "schema": "scena.visual_patch.v1",
  "transforms": [
    {
      "node": 42,
      "transform": {
        "translation": [1.0, 2.0, 3.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      }
    }
  ],
  "tints": [
    {
      "node": 42,
      "tint": { "r": 0.2, "g": 0.4, "b": 0.6, "a": 1.0 }
    }
  ],
  "visibility": [
    { "node": 42, "visible": true }
  ],
  "camera": {
    "target": [0.0, 0.0, 0.0],
    "distance": 4.0,
    "yaw_radians": 0.7853982,
    "pitch_radians": 0.5235988
  },
  "transforms_eased": [
    {
      "node": 42,
      "transform": {
        "translation": [2.0, 0.0, 0.0],
        "rotation": [0.0, 0.0, 0.0, 1.0],
        "scale": [1.0, 1.0, 1.0]
      },
      "duration_seconds": 0.5,
      "easing": "ease_in_out"
    }
  ],
  "tints_eased": [
    {
      "node": 42,
      "tint": { "r": 0.25, "g": 0.75, "b": 0.5, "a": 1.0 },
      "duration_seconds": 0.25,
      "easing": "linear"
    }
  ],
  "camera_eased": {
    "camera": {
      "target": [1.0, 0.0, 0.0],
      "distance": 5.0,
      "yaw_radians": 1.0,
      "pitch_radians": 0.25
    },
    "duration_seconds": 0.75,
    "easing": "ease_in_out"
  },
  "animation_time": [
    { "mixer": 7, "mode": "seek", "seconds": 0.5 }
  ],
  "selection": { "node": 42 },
  "hover": { "node": null },
  "material_variants": [
    { "import": 9, "variant": "noon" }
  ],
  "labels": [
    {
      "id": "part-label",
      "target": {
        "kind": "node",
        "node": 42,
        "local_offset": [0.0, 0.0, 0.0]
      }
    },
    {
      "id": "origin-label",
      "target": {
        "kind": "world",
        "position": [0.0, 0.0, 0.0]
      }
    }
  ],
  "section_box": {
    "mode": "set",
    "min": [-0.25, -0.5, -0.5],
    "max": [0.25, 0.5, 0.5],
    "margin": 0.05,
    "inverted": false,
    "helper_wireframe": true
  },
  "metadata": { "request_id": "agent-step-42" },
  "echo_metadata": true
}
```

Stable fixtures live at
`tests/assets/stable-contracts/visual_patch.v1.json` and
`tests/assets/stable-contracts/visual_patch_result.v1.json`.

### `scena.scene_host_gizmo_drag.v1`

Consumed by `SceneHostCore::apply_gizmo_drag_json()` and WASM
`SceneHost.applyGizmoDragJson()`. This is a transient interaction request for
the platform-neutral `TransformGizmo`: the host supplies a target stable node
handle separately, plus the starting transform and caller-derived start/current
pointer rays. Scena computes one translate/rotate/scale transform and applies
it through the existing `scena.visual_patch.v1` transform channel. The result
JSON is therefore the normal `VisualPatchResultV1` shape, including no-op,
revision, and stale-handle semantics.

The request supports:

- `mode`: `translate`, `rotate`, or `scale`;
- `space`: `world`, `local`, or `view_aligned` (`world` by default);
- optional `constraint`: `{ "kind": "axis", "axis": "x|y|z" }`,
  `{ "kind": "plane", "axis": "x|y|z" }`, or `{ "kind": "view_plane" }`;
- `start_transform`, using the stable `Transform` JSON shape;
- `start_ray` and `current_ray`, each with finite `origin` and non-zero
  finite `direction`.

Invalid JSON, unsupported schemas, non-finite transforms, invalid rays, and
unresolvable ray/constraint combinations fail closed as `InvalidInput`. Missing
or stale target handles are reported by the returned visual-patch result for
the `transforms` channel; valid requests never create an undo stack, snapping
system, constraint solver, collision check, or document model.

Stable fixture:
`tests/assets/stable-contracts/scene_host_gizmo_drag.v1.json`.

### `scena.scene_host_visual_state.v1` and `scena.scene_host_visual_states.v1`

Returned by `SceneHostCore::store_visual_state_json()`,
`SceneHostCore::visual_state_json()`, and
`SceneHostCore::visual_states_json()`. A visual state stores a named
`VisualPatchV1` plus optional opaque metadata for host-owned presentation
presets. It is not a document model, undo stack, workflow engine, or time
owner.

Host-defined names are allowed. The recommended workflow names are
`assembled`, `exploded`, `service_view`, and `covers_hidden`.

Applying a visual state clones the stored patch and delegates to
`SceneHostCore::apply_patch()`, so normal `scena.visual_patch.v1` defaults,
result counts, metadata echo, stale-handle failures, and no-op semantics apply.
The stored patch remains additive: omitted `VisualPatchV1` fields default
exactly as they do when applying a patch directly.

`scena.scene_host_visual_states.v1` lists stored state names and metadata in
deterministic name order. The list is inspectable state inventory only; it does
not imply history, undo, or ownership of application data.

Stable fixtures live at
`tests/assets/stable-contracts/scene_host_visual_state.v1.json` and
`tests/assets/stable-contracts/scene_host_visual_states.v1.json`.

### `scena.scene_host_section_box.v1`

Returned by `SceneHostCore::set_section_box_json()` and the matching browser
`SceneHost.setSectionBox()` helper. It reports whether the section box is
enabled, the world-space bounds and margin, the exact six generated clipping
planes, and the optional stable helper-node handle when a wireframe helper was
requested. Plane values are reported as data, not as public clipping-plane
handles; hosts mutate the section box through `VisualPatch.section_box` or the
SceneHost section-box helpers.

Stable fixture:
`tests/assets/stable-contracts/scene_host_section_box.v1.json`.

## SceneHost event batch

`SceneHostCore::drain_events_json()` and WASM `drainEventsJson()` return
schema `scena.host_event.v1`, represented by `HostEventBatchV1`.
Native hosts may also call `set_event_sink` for push-style notification.
While a sink is installed, events are delivered to the sink and are not queued
for later drain calls; clear the sink to resume typed batch draining.

Events are emitted by the same SceneHost operations that already mutate or
inspect the live host state. There is no hidden render loop. A batch drains the
current queue and a second drain returns an empty batch until more host calls
emit events. Diagnostic events are edge-triggered by the prepared diagnostic
set, so repeated `prepare()` calls with unchanged diagnostics do not re-emit
the same diagnostic batch.

Event kinds in v1:

- `pick`: CSS pixel coordinates, optional stable hit handle, distance, world
  position, optional normal, optional pointer button, and modifier flags.
- `hover`: CSS pixel coordinates, `entered` / `moved` / `left`, and optional
  stable hit handle.
- `selection_changed`: previous and current stable handles.
- `load_progress`: nested `AssetLoadProgressV1`.
- `asset_loaded`: import handle plus nested `scena.asset_load_report.v1`.
  Instanced URL imports can produce multiple instance-root handles, so they
  emit `load_progress` without a single `asset_loaded` event.
- `diagnostic`: structured diagnostic code, severity, optional stable node
  handle, message, and help text.
- `capture_ready`: capture schema, dimensions, pixel format, payload byte
  length, and payload hash for an out-of-band RGBA8 payload.
- `surface_resized`: CSS pixels, physical pixels, and device pixel ratio.
- `context_lost`, `context_restored`, and `device_lost`. On browser
  SceneHost, pages forward real browser context lifecycle signals through
  `handleSurfaceContextLost(recoverable)` and
  `handleSurfaceContextRestored()`.
- `device_recovered`: reserved in the schema and fixture for platform recovery
  signals; no current `SurfaceEvent` emits it.
- `capability_changed`: capability schema and backend after context recovery.

Browser coordinates are CSS pixels unless a field name explicitly includes
`physical`. Event payload handles are the same stable `u64` handles used by
inspection, visual patches, and direct SceneHost APIs. Removed handles retain
their original generation in queued events and do not alias replacement nodes.

Stable fixture:
`tests/assets/stable-contracts/host_event.v1.json`.

### `scena.interaction_expectation.v1` and `scena.interaction_verification.v1`

The `scena verify interaction <asset-or-recipe> --expect <json>` CLI command
loads the asset through `SceneHostCore`, frames and renders it once, injects the
requested native SceneHost interaction steps, and emits
`scena.interaction_verification.v1`.

The expectation contract is transient input, not a persisted document model.
It contains:

- `schema`
- `viewport`: `width_css_px`, `height_css_px`, and `device_pixel_ratio`
- `steps`: ordered `pick`, `hover`, or `select` actions

Step coordinates use CSS pixels by default. A step may set
`coordinate_space: "physical"` to provide physical pixel coordinates; the
report always echoes both CSS and physical coordinates. This first native slice
does not synthesize browser DOM input, keyboard input, camera-control gestures,
or rendered highlight/outline feedback.

`expect_hit`, `expect_hover`, and `expect_selection` are real boolean
assertions: `true` requires the state to be present, and `false` requires it to
be clear after the step. Omit the field when the step should not assert that
state.

The verification report contains:

- `schema`
- `ok`
- `summary`: step, failure, hit, miss, and event counts
- `steps`: ordered expected/observed interaction results
- `reasons`: stable failure codes such as `hit_mismatch`,
  `handle_mismatch`, `hover_missing`, `hover_unexpected`,
  `selection_missing`, `selection_unexpected`, and `event_sequence_mismatch`
- `fixes`: stable suggested actions
- `artifacts`: viewport and linked `scena.host_event.v1` schema metadata

`ok=false` means at least one requested interaction assertion failed, and the
CLI returns a non-zero exit status while keeping the report on stdout. A native
`select` step validates SceneHost selection state and the emitted
`selection_changed` event; a native `hover` step validates hover state and the
emitted hover event; a native `pick` step validates the picked stable handle and
emitted pick event.

Stable fixtures:
`tests/assets/stable-contracts/interaction_expectation.v1.json` and
`tests/assets/stable-contracts/interaction_verification.v1.json`.

### `scena.agent_smoke_template.v1`

The `scena examples agent get <template> [--out <dir>]` CLI command writes a small
set of recipe, expectation, and artifact-path files for a named smoke template
and emits a manifest with schema `scena.agent_smoke_template.v1`. The
Authored-from-scratch starter snippets have canonical names such as
`primitive-scene`, `cad-plate`, `dashboard-bars`, `machine-state-viewer`, and
`product-configurator-starter`.

The manifest contains:

- `schema`
- `name`
- `status`: usually `ready`; `deferred` is reserved for templates that have no
  honest runnable CLI smoke path yet
- `required_features`: crate features required to run the generated commands
- `files`: generated file paths and their schema names
- `commands`: argv arrays beginning with `scena`, expected output schema,
  expected `ok` value, and artifact paths that should exist after the command
- `notes`: explanatory text for limitations or deferred sub-capabilities

Ready templates are CLI-only acceptance examples over the normal
prepare/render/capture/report path. CAD inspection and documentation rendering
are runnable smoke templates for asset load, render introspection, visibility
diagnosis, recipe-authored section boxes, measurements, callouts, and exploded
views.

### `scena.agent_template_catalog.v1`

Produced by `scena examples agent list`. Each `templates` row includes one
kebab-case canonical `name`, compatibility `aliases`, `status`,
`required_features`, and a summary. Alias generation retains the canonical
manifest name and adds a migration diagnostic; callers never need to parse an
unknown-template error to discover names. The stable fixture is
`tests/assets/stable-contracts/agent_template_catalog.v1.json`.

Stable fixture:
`tests/assets/stable-contracts/agent_smoke_template.v1.json`.

### `scena.browser_proof_run.v1`

`scena browser-proof [scene-host|m6] [--backend webgl2] [--dry-run]` emits
`scena.browser_proof_run.v1`. The wrapper delegates to the existing Playwright
lanes and keeps stdout machine-readable. The `scene-host` lane runs
`browser:scene-host-proof`; the `m6` lane first rebuilds
`target/m6-browser-pkg` with `wasm-pack --features browser-probe`, then runs
`browser:m6`. `--dry-run` reports the exact command, environment, and artifact
paths without launching a browser; real runs return `status: "passed"` or
`"failed"` and exit non-zero on failure. Failed runs include compact stdout and
stderr tails when the underlying command produced them.

Stable fixture:
`tests/assets/stable-contracts/browser_proof_run.v1.json`.

### `scena.q01.required_webgpu_pixel_parity.v1`

The required browser M6 hardware lane emits this source-bound comparison
artifact after evaluating the live renderer-owned WebGPU readback against the
CPU oracle. It records exact frame normalization and thresholds, mask policy,
metric summary, worst-region bounding box, six known-bad mutation outcomes,
adapter identity, reproducing command/environment, image paths and hashes,
source commit, producer, timestamp, and producer-source checksums. A software
adapter may emit the same pixel comparison as conformance evidence, but cannot
claim required hardware parity.

Stable fixture:
`tests/assets/stable-contracts/required_webgpu_pixel_parity.v1.json`.

The aggregate `scena.m6.rust_wasm_renderer_probe.v1` labels evidence scope
separately from status. Nonblack/draw/submission output is `renderer-smoke`; a
software WebGPU run with the Q01 comparison is
`renderer-conformance-with-diagnostic-webgpu-pixel-diff`; and only the strict
physical lane with the complete Q01 oracle is
`renderer-smoke-with-required-webgpu-full-frame-parity`. Smoke-only aggregates
always record `release_evidence: false`. The required class records the exact
`webgpu:m6-identical-unlit-triangle-v1` parity scope, so it cannot imply parity
for WebGL2 or for lit/textured native scenes.

### `scena.q04.required_gpu_resource_lifecycle.v1`

The required native hardware lane emits this artifact only after it creates an
accepted physical adapter, prepares baseline and expanded GPU resource sets,
renders the expanded set, returns retained resources to the baseline shape,
and confirms destruction of every queued object through device polling. The
artifact records adapter provenance, all three resource-counter snapshots,
poll counters/status, the executed assertion count, producer command, source
commit, and timestamp. Optional developer-smoke skip artifacts use a different
proof class and cannot satisfy release readiness.

Stable fixture:
`tests/assets/stable-contracts/required_gpu_resource_lifecycle.v1.json`.

## Renderer stats JSON

`Renderer::stats()` returns the native `RendererStats` struct. `SceneHost`
also exposes the same counters through `statsJson()`.

Release 1.7 adds these counters:

- `gpu_draw_submissions`: actual GPU draw submissions recorded at renderer
  submission sites.
- `instances`: visible per-instance records from explicit instanced imports.

The legacy `draw_calls` and `primitives` fields remain as deprecated aliases of
`triangles` for 1.x compatibility. They do not report GPU submission count and
are planned for removal in 2.0.

## Shipped v1 contracts

### `scena.capability_report.v1`

Produced by `CapabilityReport::to_schema_json()` and represented by
`CapabilityReportV1`.

Required top-level fields:

- `schema`
- `capabilities`
- `adapter`
- `diagnostics`

When this report is emitted by a backend-selecting `scena` CLI command, the CLI
envelope also includes `backend_selection` with `source` (`default` or
`cli_flag`), `requested`, `selected`, `fallback_used`, `reason`, and `remedy`.
Fallback diagnostics stay inside this object; machine-mode stderr remains
empty. The library-produced typed report remains backend-selection-policy
neutral. Recipe render, capture, and CAD inspection CLI envelopes use the same
object.

Additive optional fields:

- `post_processing`: active/available post-processing pass metadata for the
  current renderer configuration.
- `probe`: pre-render discovery provenance emitted by `scena capabilities`.
  `mode` separates `static` from `live_adapter`; `status` is
  `static_no_device`, `measured`, or `unavailable`. It records source/time,
  requested and selected backend, requested-device features/limits, color and
  depth target format/sample-count evidence, readback/presentation constraints,
  and a structured unavailable reason. Old v1 documents without `probe`
  continue to deserialize.

Capability enum values use serde names such as `headless`, `supported`,
`degraded`, and `feature_disabled`.

`capabilities.subject_visible_mask` reports whether exact subject-pixel
visibility can be derived by the active composition path. The CPU headless
recipe verification path reports `supported`; GPU/browser backends report a
degraded static capability until their backend-specific semantic AOV capture is
wired into subject observation/reporting.

Subject observations and recipe composition reasons use the same zero-visible
subject reason-code vocabulary. A declared `render.metering`,
`render.depth_of_field.focus`, or `photo.subject` target that resolves but
contributes no visible pixels must report one of these codes in
`verification.reasons[]`, the matching `subject.*.visible_mask` composition
check, and the relevant `subject_observation.v1.fallback.reason_codes[]`:

| Code | Meaning |
| --- | --- |
| `subject_hidden` | The subject or one of its ancestors is hidden. |
| `subject_outside_viewport` | The subject projects outside the current viewport. |
| `subject_behind_camera` | Drawable subject geometry is behind the active camera. |
| `subject_degenerate_geometry` | The subject has no finite nonzero drawable extent. |
| `subject_clipped_by_section_box` | The active section box removes the subject. |
| `subject_clipped_by_clipping_plane` | An active clipping plane removes the subject. |
| `subject_transparent_unsupported` | Exact subject masks cannot attribute the transparent subject. |
| `subject_occluded` | Other visible geometry fully occludes the subject. |
| `subject_visible_mask_empty` | The subject has semantic identity but no visible pixels in the frame. |
| `stale_subject_observation` | The observation frame key no longer matches the rendered frame. |

`capabilities.auto_exposure_metering_*` reports each public auto-exposure
metering mode separately. A mode that is accepted by recipe validation but not
yet routed into the exposure meter must report `degraded` or
`feature_disabled`; it must not be implied by prose or by the existence of the
recipe field. CPU Headless reports `auto_exposure_metering_subject:"supported"`
because recipe render routes exact semantic subject observations into the
scene-linear exposure meter. GPU/browser lanes remain `degraded` until their
backend-specific subject observations feed metering directly.

`capabilities.color_target_format` is live renderer metadata when a surface is
attached and may be `Rgba8Unorm`, `Rgba8UnormSrgb`, `Bgra8Unorm`, or
`Bgra8UnormSrgb`. It is not a readback-transfer flag: the default RGBA8 output
contract remains sRGB display bytes for all four attachment formats.

`scena capabilities [--live] [--json]` emits this schema before rendering.
The device-free default labels target facts as unmeasured. The live command
uses the selected renderer target and wgpu format features, exits 1 with the
same schema when adapter/device creation is unavailable, and does not claim
surface presentation from its headless probe.

### `scena.scene_inspection.v1`

Produced by `SceneInspectionReport::to_schema_json()` when the `inspection`
feature is enabled and represented by `SceneInspectionReportV1`.
The `scena` binary's first CLI transport is `scena inspect <asset>` when built
with the `inspection` feature; it loads and prepares the asset through the
normal headless viewer path, then emits this report on stdout.

Required top-level fields:

- `schema`
- `nodes`
- `draw_list`
- `camera_frustums`
- `normal_overlays`
- `active_camera`
- `counts`
- `revisions`

Node IDs in standalone native inspection are deterministic report-local `u64`
handles. Host-backed inspection uses the `SceneHost` handle namespace. Raw
slotmap keys and asset handles are intentionally absent from this wire contract.

Native host adapters can pass their own node map with
`SceneInspectionReport::to_schema_report_with_node_handles`. Nodes not present
in that map receive report-local fallback IDs.

Each node entry includes `handle`, `parent`, `kind`, `tags`,
`local_transform`, `world_transform`, `visible`, `bounds`, `layer_mask`,
`render_group`, `helper_on_top`, optional `tint`, and optional `material`
evidence when inspection was produced with assets. `tint` is included because
per-node highlight state is part of the render state a host may need to prove.

Draw-list and normal-overlay entries may include an additive optional
`instance` field. It is `null` or absent for ordinary node drawables and set
for per-instance records.

Node and draw-list `material` evidence is additive on
`scena.scene_inspection.v1`. It reports semantic material data without exposing
raw asset handles: `kind`, `source`, `base_color`, `alpha_mode`, deterministic
texture rows, and fallback rows. `source.kind` is `source_material`,
`generated_default`, `user_created`, or `unknown`; source-backed rows include
the loaded asset path and source material index, while generated rows include a
reason such as a primitive that did not reference a glTF material. Texture rows
include slot name, source path, source format, color space, decoded dimensions,
decoded-pixel availability, texture provenance, and the matching fallback row
when one exists.

Host-backed inspection may also include additive top-level `instance_sets`
entries for explicit instanced URL imports. Each entry contains the
instance-root `root_handle`, `visible`, optional opaque `tint`,
`root_transform`, and per-drawable `entries` with the backing set-node handle,
source `instance_id`, and baked drawable-local transform. This field is
additive on `scena.scene_inspection.v1`; older consumers may ignore it.

Host-backed inspection may also include additive top-level `imports` entries.
Each entry contains the stable SceneHost import `handle`, additive
`root_handles`, declared `material_variants`, and the current `active_variant`.
`root_handles` lets agent-facing diagnosis walk import targets through the same
stable node handles reported in `nodes`. This is the current state report for
the same import handles accepted by the 0.1C `material_variants` visual-patch
channel.

`revisions` includes `structure`, `transform`, additive `appearance`, and
`interaction`. Older `scena.scene_inspection.v1` payloads without
`appearance`, `tint`, `material`, `instance`, `instance_sets`, `imports`, or
`imports[].root_handles` still deserialize with defaults.

Topology helpers on `SceneInspectionReportV1`:

- `node_by_handle(handle)`
- `children_of(handle)`
- `roots()`
- `find_by_tag(tag)`

### `scena.capture.v1`

Produced by `capture_rgba8`, `Renderer::capture_rgba8`, viewer `capture()`
helpers, and `SceneHost.captureJson()` / `SceneHost.captureJsonAsync()`.
Represented by `CaptureDescriptor`.
PNG helpers such as `CaptureRgba8::to_png_bytes`,
`CaptureRgba8::write_png`, `Renderer::capture_png_bytes`,
`Renderer::capture_png`, SceneHost `capture_png_bytes`, and browser
`capturePng()` and `capturePngAsync()` delegate to the same descriptor-bound
capture object. The async browser methods are required for WebGPU buffer
mapping and return the same descriptor/payload shapes as their synchronous
WebGL2-compatible counterparts.

Required top-level fields:

- `schema`
- `width`
- `height`
- `pixel_format`
- `payload`
- `revisions`
- `camera`
- `viewport`
- `backend`
- `capabilities`
- `frame`
- `auto_frame`
- `pixels`

Large RGBA8 bytes are returned outside JSON through `CaptureRgba8::rgba8` or
the browser `capture().rgba8` / awaited `captureAsync().rgba8` typed array. PNG
bytes are returned outside JSON through native byte vectors/files or browser
`capturePng().png` / awaited `capturePngAsync().png`. JSON carries byte length,
dimensions, format, and FNV-1a hash metadata.

`frame` is the provenance binding for the pixel payload. Evidence-bearing
captures use `pixel_source: "renderer_owned_readback"`,
`state_binding: "exact_readback_completion"`, and `release_evidence: true`.
The record carries render/target/output-resource revisions, the exact output
color space, exposure, tonemapper, anti-aliasing and enabled post passes, plus
the readback-completion Unix timestamp. `capture_rgba8_from_pixels` accepts
only bytes identical to the renderer's latest completed readback. The separate
`capture_unverified_rgba8_from_pixels` diagnostic helper labels caller bytes as
`release_evidence: false`; release or trust consumers must reject that form.

The descriptor binds to the renderer's last rendered frame state. If scene
revisions or the active camera changed after `render()` and before `capture()`,
capture returns `CaptureError::StaleRender` instead of serializing a false
pixels-to-state binding.

`scena.capture_baseline.v1` is produced by
`compare_captures_with_tolerance`. It records the actual and expected
`scena.capture.v1` descriptors, tolerance, diff metrics, backend and capability
metadata via the nested descriptors, and `status: "passed"` or `"failed"`.
`capture_contact_sheet_rgba8` produces an RGBA8 proof contact sheet whose
tiles retain the source capture descriptors; it is a proof-artifact helper, not
a replacement capture schema.

### `scena.render_introspection.v1`

Produced by `Renderer::introspect_capture`,
`RenderIntrospectionReportV1::from_capture`, and
`RenderIntrospectionReportV1::from_capture_with_diagnostics` when the
`inspection` feature is enabled. SceneHost also exposes it through native
`SceneHostCore::render_introspection_json(detail)` and browser
`SceneHost.renderIntrospectionJson(detail)` when `scene-host` is enabled. The
report binds a `scena.capture.v1`
descriptor, a `scena.scene_inspection.v1` report, renderer stats, and optional
renderer diagnostics into a small, deterministically ordered agent-readable
summary.

Required top-level fields:

- `schema`
- `ok`
- `reasons`
- `fixes`
- `content_bbox_css_px`
- `content_bbox_fraction`
- `visible_pixel_fraction`
- `luminance`
- `framing`
- `nodes_summary`
- `nodes_detail`
- `artifacts`
- `capabilities`

The v1 report classifies capture-derived and diagnostic-derived visibility
failures: `empty_frame`, `no_visible_drawables`, `all_culled`,
`behind_camera`, `outside_frustum`, `alpha_zero`, `nan_transform`,
`clipped_by_active_clipping_plane`, `tiny_in_frame`, `cropped`, and
`backend_capability_degraded`. `ok` is false only when an `error`-severity
reason is present; warning-only framing and backend capability reasons are
still returned but do not fail the agent loop.
`visible_pixel_fraction`, `content_bbox_css_px`, and
`content_bbox_fraction` are computed from pixels that differ from the
configured shader-encoded background by more than the implementation's byte
tolerance, not from literal non-black pixels. `luminance` values are computed
from shader-encoded RGBA8 bytes on a 0-255 scale. `fixes[]` carries stable
Scena action codes such as `frame_bounds` and `set_visible`; callers decide
whether to apply a suggested action.

#### `nodes_detail[].reason_codes` vocabulary

Detail mode attaches per-node `reason_codes` explaining why a node that is
*visible* still places no pixels on the frame. The complete vocabulary, with the
severity each code carries:

| Code | Severity | Meaning |
|---|---|---|
| `node_hidden` | error | The node itself is hidden. |
| `parent_hidden` | error | An ancestor is hidden. |
| `layer_masked` | error | The active camera's layer mask excludes the node. |
| `stale_handle` | error | The handle no longer resolves to a live node. |
| `nan_transform` | error | The resolved world transform is not finite. |
| `zero_scale` | error | The resolved scale collapses the node to nothing. |
| `missing_geometry` | error | The node has no geometry to draw. |
| `missing_material_upload` | error | The material never reached the backend. |
| `alpha_zero` | error | The material is fully transparent. |
| `behind_camera` | error | The node is entirely behind the active camera. |
| `outside_frustum` | error | The node is outside the active camera frustum. |
| `not_prepared` | error | Nothing has been prepared yet. |
| `missing_camera` | error | The scene has no active camera. |
| `no_visible_drawables` | error | No visible node can rasterize. |
| `all_culled` | error | Every drawable was culled. |
| `import_has_no_roots` | error | The import produced no root nodes. |
| `import_roots_stale` | error | The import's roots no longer resolve. |
| `clipped_by_active_clipping_plane` | warning | An active clipping plane removes the node. |
| `transparent_material` | warning | The material is non-opaque; it may be invisible against the background. |
| `backend_capability_degraded` | warning | The backend lacks a capability the scene relies on. |

Which codes are load-bearing:

- **`error` severity is load-bearing.** `ok` is false whenever any
  `error`-severity reason is present, and a declared template expectation fails.
- **`warning` severity is advisory.** It is reported and must not be filtered
  out, but it does not set `ok` to false on its own.

Codes are only attached to node kinds that can rasterize — `Mesh`, `Label`,
`Renderable`, `Model`, `InstanceSet`, and `ParticleSet`. Cameras, lights, and
empties structure the scene but never place pixels, so a visibility reason on
one of them would be noise rather than a finding.

There is deliberately **no** `clipped_by_section_box` code. A section box
sections model geometry, and geometry it removes is reported through the
existing culling and clipping codes; annotations are not clipped by it at all
(see the section-box semantics above).

Summary mode omits `nodes_detail`; detail mode includes stable node handles
and draw-derived node state. `nodes_summary.transparent` is computed from
non-opaque prepared draw materials.

`nodes_summary.visible` counts **every** visible node, including cameras,
lights, and empties, so it is never comparable to `nodes_summary.drawn`.
`nodes_summary.visible_drawable` (added in 1.9.1, `#[serde(default)]`) counts
visible nodes that can rasterize — the population `drawn` is drawn from. Compare
`drawn` against `visible_drawable`, never against `visible`. Failure reasons include stable
`affected_handles` whenever the renderer can identify the node, and fixes that
change scene state carry apply-ready `scena.visual_patch.v1` bodies. The report
rounds floating-point summaries to stable precision and keeps large artifacts
outside JSON, referenced through explicit paths or the nested capture summary.

The stable fixture lives at
`tests/assets/stable-contracts/render_introspection.v1.json`. The `scena`
binary's first CLI transport is `scena render <asset-or-recipe> --out <png>`
when built with the `inspection` feature. It writes the PNG and capture
descriptor artifacts, then emits this report on stdout. Introspection is the
default; `--introspect` remains an accepted compatibility no-op.

### `scena.render_quality.v1`

Produced by recipe verification when the `inspection` feature is enabled. The
report evaluates native-resolution RGBA8 captures, never downscaled images, and
keeps render quality separate from correctness checks. It is nested under
`SceneRecipeVerificationReportV1.quality` and carries profile-scoped checks for
exposure, contrast, noise, text integrity, line integrity, geometry-edge
integrity, reflection presence, area-light soft-shadow structure, contact-shadow
grounding, depth-of-field blur/focus, and reference fidelity.

Required top-level fields:

- `schema`
- `ok`
- `profile`
- `summary`
- `checks`
- `capabilities`

Each `checks[]` entry has `id`, stable `code`, explicit `status`, `severity`, `region`,
deterministically ordered `observed` and `threshold` maps, and an actionable
`fix_hint`. Status is `checked` for evaluated passing checks and `failed` for
warnings/errors that must be surfaced. Exact failure codes include `severe_black_crush`,
`label_ink_isolation`, `label_missing_antialiasing`,
`line_missing_antialiasing`, `line_not_straight`,
  `geometry_missing_antialiasing`, `reflection_structure_missing`,
  `reflection_firefly_outliers`,
`area_light_soft_shadow_insufficient`, `contact_shadow_missing`,
`depth_of_field_checked`, `depth_of_field_blur_insufficient`,
`depth_of_field_focal_softened`, `reference_delta_e2000_exceeded`, and
`reference_ssim_too_low`.

The stable fixture lives at
`tests/assets/stable-contracts/render_quality.v1.json`.

### `scena.scene_composition.v1`

Produced by recipe verification when the `scene-host` feature is enabled. The
report is nested under `SceneRecipeVerificationReportV1.composition` and
records whether declared recipe elements and generated overlays have explicit,
owned projected output. It is a foundation/spec-conformance layer; explicit
`expect_quality.profile` runs the full object-scoped native-capture checks for
framing, exposure, subject/background salience, and decoded base-color texture
result variation. Product-style verification (`render.profile:"product"` or
`render.auto_exposure:"product_studio"`) also runs the severe subject exposure
gate by default so imported product assets cannot pass verification while
obviously blown out or dead dark.

Required top-level fields:

- `schema`
- `ok`
- `summary`
- `checks`

Each `checks[]` entry has an `id`, `category`, stable `code`, explicit
`status`, `severity`, optional projected `region`, stable `affected_handles`,
deterministically ordered `observed` data, and an actionable `fix_hint`.
Statuses are one of `checked`, `failed`, `skipped_no_declared_intent`,
`skipped_no_backend_support`, `skipped_import_unknown`, `unsupported`, and
`not_applicable`. Failed checks are verification errors. Informational skipped
checks remain visible in the composition block as coverage inventory and are
not promoted to top-level warning reasons. If the selected verification profile
requires a category, missing coverage must fail closed as a `failed`/`error`
check instead of silently reporting `ok:true`.

The foundation report checks declared-node presence, projected bboxes and
screen size when bounds exist, imported-root presence/coverage where import
manifests expose node handles, material base-color intent where draw-material
inspection is available, native-capture visible-pixel coverage in the declared
node or import's viewport-clipped projected region, object-scoped
exposure/salience and texture-result checks when an object quality profile is
declared or inferred from product-style render settings, grid/floor ownership,
callout target ownership, measurement overlay output ownership,
explicit overlay label/line geometry, declared ground contact from
`expect_grounded`, helper-layer occlusion from `expect_helper_occluded`,
object depth order from `expect_occlusion`, and unexpected draw output. Exact
reason codes include
`declared_node_not_drawn`, `unexpected_draw_output`,
`material_base_color_available`, `visible_pixel_coverage_available`,
`visible_pixel_coverage_missing`, `subject_exposure_sane`,
`subject_black_crushed`, `subject_blown_out`, `subject_salience_too_low`,
`subject_fit_sane`, `subject_too_small_in_frame`,
`subject_too_large_in_frame`,
`texture_result_visible`, `texture_result_flat`, `texture_result_missing`,
`grid_floor_output_owned`,
`callout_target_attached`, `callout_overlay_output_projected`,
`measurement_overlay_output_projected`, `overlay_label_clear_of_lines`,
`overlay_label_intersects_line`, `overlay_label_clear_of_labels`,
`overlay_label_intersects_label`, `overlay_label_inside_viewport`,
`overlay_label_clipped_by_viewport`, `ground_contact_present`,
`ground_contact_missing`, `ground_target_unresolved`,
`helper_layer_occluded_by_subject`, `helper_layer_overdraws_subject`,
`helper_occlusion_target_unresolved`, `helper_occlusion_region_unavailable`,
`helper_occlusion_color_unavailable`, `object_depth_order_satisfied`,
`object_depth_order_mismatch`, `object_depth_order_color_ambiguous`,
`object_depth_order_target_unresolved`,
`object_depth_order_region_unavailable`, `object_depth_order_color_unavailable`,
`backend_expectation_satisfied`,
`backend_expectation_mismatch`, `render_antialiasing_active`,
`render_supersample_active`, `render_reconstruction_active`,
`clipping_plane_count_satisfied`, `clipping_plane_count_mismatch`,
`section_box_active`, `section_box_missing`, `section_box_inactive`,
`section_box_unexpected_active`, `section_box_inversion_satisfied`,
`section_box_inversion_mismatch`, `material_variant_state_satisfied`,
`material_variant_state_mismatch`, `transform_conformance_satisfied`,
`transform_conformance_mismatch`, `separation_conformance_satisfied`, and
`separation_conformance_mismatch`. Visible coverage is
computed from foreground pixels relative to the configured render background;
line primitives use their projected stroke region rather than their zero-height
geometric AABB. Grounded expectations compare inspected world-space bounds
against `plane_y` within `tolerance`; use them for content that must touch a
floor/ground plane, not intentionally floating content. Helper occlusion
expectations count helper-coloured pixels inside the occluder's projected
interior; use them for depth-tested helpers, grids, or wireframes that must
stay behind solid subjects. Object occlusion expectations count the expected
back object's colour inside the expected front object's projected interior; use
them for declared object-vs-object depth order such as "part A must occlude
part B." Backend expectations compare the actual render
backend and GPU-device flag with `expect_backend`; use them for GPU/beauty
renders where CPU fallback would invalidate the proof.
Clipping expectations compare active user clipping-plane count and section-box
state with `expect_clipping`; use them when an agent-authored cutaway, section
box, or recipe clipping plane is load-bearing for the visual proof.
Overlay clearance checks compare projected native-resolution label regions
against both line overlays and other label regions; label-vs-label failures use
`overlay_label_intersects_label`. Label viewport-fit checks compare the
unclipped projected label rectangle against the capture viewport; clipped labels
fail with `overlay_label_clipped_by_viewport`.
State expectations compare the actual inspected import material-variant state
with `expect_state`; use them when a configurator/product render depends on
the default import variant or a named material variant being active.
`expect_occlusion` currently uses a native-resolution color-probe in the
front object's projected interior. It fails closed with
`object_depth_order_color_ambiguous` when the expected front/back draw colours
are too similar for that probe to distinguish. Use high-contrast opaque
front/back materials for object-depth expectations.
Object exposure/salience checks operate on each declared object's
viewport-clipped projected region at native capture resolution and compare
foreground pixels against the configured render background. Exact depth/id-mask
occlusion attribution for arbitrary overlapping projected bboxes is a later
precision layer, not implied by this foundation check.
Object framing checks operate on each declared object's projected region when
`expect_quality.profile` is present; use failures such as
`subject_too_small_in_frame` or `subject_too_large_in_frame` as profile-driven
camera/framing defects. `expect_bbox_fit` remains available for explicit
recipe-specific min/max framing contracts.

The stable fixture lives at
`tests/assets/stable-contracts/scene_composition.v1.json`.

### `scena.visibility_diagnosis.v1`

Produced by `Renderer::diagnose_visibility` and
`VisibilityDiagnosisReportV1::from_inspection` /
`VisibilityDiagnosisReportV1::from_inspection_with_diagnostics` when the
`inspection` feature is enabled. The report consumes a
`scena.scene_inspection.v1` report, renderer stats, optional renderer
diagnostics, and an optional stable node handle. It returns ranked reasons and
data fix suggestions without mutating scene state.

Required top-level fields:

- `schema`
- `ok`
- `target`
- `reasons`
- `fixes`
- `summary`
- `evidence`

The v1 contract covers `not_prepared`, `missing_camera`,
`no_visible_drawables`, `all_culled`, `stale_handle`, `node_hidden`,
`parent_hidden`, `zero_scale`, `nan_transform`, `layer_masked`, `alpha_zero`,
`transparent_material`, `missing_material_upload`, `missing_geometry`,
`behind_camera`, `outside_frustum`, `clipped_by_active_clipping_plane`,
`backend_capability_degraded`, and SceneHost import-root diagnosis.
`all_culled` is emitted only when renderer stats show that every
inspection-visible drawable was culled; partial frustum culling of a healthy
scene is not a failure. Each reason includes severity, confidence, whether it
is auto-fixable, affected stable handles when known, and a short message. Fixes
use stable Scena action codes such as `prepare`, `set_camera`, `frame_bounds`,
`set_visible`, `set_transform`, `set_layer_mask`, `set_material_alpha`,
`clear_clipping_planes`, `inspect_capabilities`, and `inspect_assets`.
Content-risk fixes are reported as data and must be applied explicitly by the
host or CLI caller. Summary mode returns only reasons, fixes, and counts;
detail mode may include supporting evidence rows.

The stable fixture lives at
`tests/assets/stable-contracts/visibility_diagnosis.v1.json`. The `scena`
binary's first CLI transport is `scena diagnose <asset-or-recipe> --visibility
[--handle <u64>]` when built with the `inspection` feature. It emits this report
on stdout and exits non-zero when `ok` is false, so shell-driven callers can
branch without parsing JSON first.

### `scena.visual_repair_plan.v1` and `scena.agent_loop_result.v1`

Produced by `VisualRepairPlanV1::from_render_introspection`,
`VisualRepairPlanV1::from_visibility_diagnosis`, and the `scena repair
<asset-or-recipe> --from <report.json>` command when the `inspection` feature
is enabled. Repair planning consumes existing render introspection or
visibility diagnosis reports. It does not inspect images through a separate
agent mode, mutate the scene, rewrite recipe files, or claim that the frame is
fixed.

`scena.visual_repair_plan.v1` contains:

- `schema`
- `status`
- `auto_fixable`
- `confidence`
- `risk`
- `root_cause`
- optional `visual_patch`
- `applied_actions`
- `skipped_actions`
- `remaining_reasons`
- `requires_host_input`
- `rerender_required`

The first v1 repair slice plans non-destructive presentation repairs for
framing-oriented action codes such as `frame_bounds`. It also emits a
reversible content-risk `VisualPatch` for `node_hidden` / `set_visible`
diagnoses, including before/after values and the root-cause reason. Content
changes that cannot be proven reversible from the input report, such as a
generic `set_transform` scale repair, are emitted under `skipped_actions` with
`requires_host_input: true`. Alpha, material override, clipping, and recipe
update repairs are reserved for later feature-owned slices.

`visual_patch` is a proposed patch only. Callers must apply it explicitly,
rerender, and re-run render introspection or visibility diagnosis. A repair
plan's `status: "repairable"` means the plan is safe to try; it is not a visual
success verdict.

`scena.agent_loop_result.v1` is emitted for irreducible or non-converging
repair loops. Its `status` is `"irreducible"`, `ok` is false, and it carries
the iteration budget, remaining reasons, skipped actions, confidence, and
whether host input is required. The `scena repair` command writes this JSON to
stdout and exits non-zero when no safe automatic fix exists.

The positional target is not advisory. Before consuming `--from`, repair loads
a raw asset through `Assets::doctor_asset_path`, or fully validates/builds a
recipe through its effective `RecipeBuildPolicy`. Missing or malformed assets
return `scena.asset_doctor.v1`; invalid recipes return
`scena.scene_recipe_validation.v1`; build/policy rejection returns
`scena.recipe_build_result.v1`. Only a valid target reaches repair planning,
and a second positional target is rejected as an argument error. The report
still owns the proposed changes; this validation does not claim that an older
report is cryptographically bound to the target.

The stable fixtures live at
`tests/assets/stable-contracts/visual_repair_plan.v1.json` and
`tests/assets/stable-contracts/agent_loop_result.v1.json`.

### `scena.appearance_expectation.v1` and `scena.appearance_introspection.v1`

Consumed and produced by `scena verify appearance <asset-or-recipe> --expect
<appearance-expectation.json>` when the `inspection` feature is enabled.
Appearance verification uses the normal load, prepare, render, capture, and
inspection path. There is no separate agent render mode.

`scena.appearance_expectation.v1` is a transient input, not a persisted scene
document. Each target declares one or more first-time appearance assertions:
stable report-local `node`, `tag`, intended `variant`, `color_family`,
`swatch_srgb8`, optional per-target `swatch_tolerance`, `alpha_mode`,
`require_source_material`, and `require_base_color_texture`. glTF
material-name matching is intentionally not part of the first slice because the
stable material inspection report currently exposes source material index and
provenance, not source material names.

`scena.appearance_introspection.v1` contains:

- `schema`
- `ok`
- `active_variant`
- `available_variants`
- `summary`
- `targets`
- `reasons`
- `fixes`
- `artifacts`

The report combines capture-derived frame-content color sampling with
`SceneMaterialInspectionV1` material provenance. It reports active source
material/fallback provenance, alpha mode and base-color alpha, decoded texture
presence, sampled region, dominant color family, swatch distance, and luminance
mean. Matched material-bearing draw targets use a projected `node_bbox` sample
region derived from the capture camera and draw bounds; unmatched or
node-without-draw targets fall back to `frame_content`. Per-node fragment
coverage remains a future additive field.

`ok` is false only for error-severity reasons such as missing intended variant,
variant not active, generated fallback where a source material was required,
hidden alpha, alpha-mode mismatch, missing base-color texture provenance, or
sampled color-family / swatch mismatch. The reason codes distinguish
`color_family_mismatch` from `swatch_mismatch` so agents can branch on family
versus numeric swatch failures. Multiple requested material variants produce a
warning because material variants are applied asset-wide for the rendered
frame. Reports stay machine-readable on stdout; the CLI exits non-zero when
`ok` is false.

The stable fixtures live at
`tests/assets/stable-contracts/appearance_expectation.v1.json` and
`tests/assets/stable-contracts/appearance_introspection.v1.json`.

### `scena.animation_introspection.v1`

Produced by `scena verify animation <asset-or-recipe> --clip <name> --times
<seconds> [--expect-change] [--expect-node-handle <handle>]
[--expect-translations 'x,y,z;...']` when the `inspection` feature is
enabled.
Animation verification uses the normal recipe/asset load, viewer, explicit
`seek_animation`, prepare, render, capture, and inspection path. There is no
hidden playback loop or separate agent render mode.

The v1 report contains:

- `schema`
- `ok`
- optional `clip`
- `summary`
- `samples`
- `reasons`
- `fixes`
- `artifacts`

`clip` records the resolved clip name, duration in seconds, and channel count.
`summary` records sample count, changed/unchanged/invalid channel counts,
whether rendered capture payloads changed across the sampled times, and the
number of capture changes from the first sample. Each `samples[]` entry records
the requested time, scene transform and appearance revisions, capture payload
hash, moving node count compared to the first sample, and invalid node count.
When expected translations are supplied, each sample also includes
`observed_values[]` entries for the selected transform: stable node handle,
full observed transform, expected translation, and `within_tolerance`.
Without `--expect-node-handle`, the verifier reports the first moving node it
can infer from the sampled transforms. Agents that need to verify a specific
part should call `scena inspect` first and pass that stable handle with
`--expect-node-handle`; a bound handle that is missing or static emits
`expected_node_missing` or `expected_node_static`.

When `--expect-change` is supplied, `ok` is false for error-severity reasons
such as missing clip, non-advancing sampled times, frozen channels, invalid
channel values, or unchanged rendered output. The CLI writes the report JSON to
stdout and exits non-zero when `ok` is false. Missing clips also return a
machine-readable report with available clip names in the reason message.
Expected sampled translation mismatches emit `expected_value_mismatch`.

The stable fixture lives at
`tests/assets/stable-contracts/animation_introspection.v1.json`.

### `scena.scene_recipe.v1` and `scena.scene_recipe_validation.v1`

Produced and consumed by `validate_scene_recipe_json`,
`validate_scene_recipe_value`, `parse_valid_scene_recipe_json`, and the `scena
validate-recipe <recipe.json>` command. A recipe is a versioned interchange/build input
for Scena and may be stored or transmitted as a v1
authoring snapshot. It is not the canonical persisted application document,
project file, workflow script, or host state model.

Same-version parsing followed by serialization produces same-version canonical output
for known fields; empty or default fields may normalize. Serde currently
ignores unknown top-level fields, so they are dropped from canonical output,
while many nested structures reject unknown fields with `deny_unknown_fields`.
There is no generic extension-data bag and no cross-version lossless round-trip
guarantee. The host owns migrations, domain persistence, undo, and history.
Schema versioning and deliberate additive fields are the extension mechanism.
Caller-authored recipe IDs are stable for recipe-local build and patch
correlation, not runtime handles or application-persistence identities. URI
access remains constrained by the operator-owned build policy and allowed roots.

The current v1 recipe slice supports:

- `schema: "scena.scene_recipe.v1"`
- `imports[]` entries with stable caller `id`, glTF/GLB `uri`, optional
  `optional` skip policy, optional `transform`, and optional
  `expected_extent`
- Import and node local transforms share the tagged `TransformSpec` grammar.
  Imports accept `kind:"trs"` and `kind:"raw"`; node-only placement kinds
  remain unavailable on imports because they require a built scene. Every
  numeric component must fit finite `f32`. Raw quaternions use `[x,y,z,w]`,
  must be non-zero, and are normalized. TRS rotations are degrees and compose
  by calling X, then Y, then Z before scale is assigned. The published 1.8.0
  untagged import object is accepted only as an exact compatibility alias,
  emits `legacy_transform_shape` with an auto-fix suggestion, and serializes
  back as canonical `kind:"raw"`; an explicit `kind` is never reinterpreted
  through that alias.
- `colors` map entries with stable caller ids and direct `#RRGGBB`, `srgb8`,
  linear RGB, Kelvin, or named `Color` constants. Named constants include the
  public Rust color helpers such as `white`, `black`, `gray`, `light_gray`,
  `dark_gray`, `charcoal`, `studio_backdrop`, `warm_white`, `cool_white`,
  `red`, `green`, `blue`, `orange`, `yellow`, `cyan`, and `magenta`.
- `geometries[]` authored resources with stable caller `id`; primitive kinds
  `box`, `plane`, `sphere`, `cylinder`, `line`, `polyline`, `arrow`, `grid`,
  `axes`, `cone`, `torus`, `disc`, and `wedge`, plus custom `mesh` entries with
  topology, positions, normals, indices, optional colors, and optional UVs.
  `torus` uses explicit `major_radius` and `minor_radius`; `box` and
  `cylinder` accept optional `bevel`/`fillet` aliases that generate real flat
  chamfer geometry and reject unsupported or ambiguous usage; generated primitive
  tessellation is deterministic and build manifests report vertex/index counts
- `materials[]` authored resources with stable caller `id`; either ergonomic
  `preset` (`chrome`, `metal`, `rough_metal`, `brushed_steel`, `plastic`,
  `clearcoat_plastic`, `satin`, `leather`, `rubber`, `matte`, `clear_glass`,
  or `frosted_glass`) or low-level `kind` (`unlit`,
  `pbr_metallic_roughness`, `line`, `wireframe`, and `edge`). Presets route
  through the matching Rust `MaterialDesc::*` helper; optional `base_color`
  tints presets where applicable, and scalar/raw overrides are applied after
  the helper result. Low-level kinds still require `base_color`. All texture
  slots are loaded under `RecipeBuildPolicy`.
  Shared material fields include base color, metallic/roughness,
  double-sided, emissive, alpha mode, stroke width, edge threshold, and texture
  slots loaded under `RecipeBuildPolicy`.
  `pbr_metallic_roughness` also accepts advanced-PBR scalars
  `clearcoat_factor`, `clearcoat_roughness_factor`, `clearcoat_normal_scale`,
  `sheen_color_factor`, `sheen_roughness_factor`,
  `anisotropy_strength_factor`, `anisotropy_rotation_radians`,
  `iridescence_factor`, `iridescence_ior`,
  `iridescence_thickness_minimum_nm`,
  `iridescence_thickness_maximum_nm`, `dispersion_factor`,
  `transmission_factor`, and `ior`, plus texture slots
  `clearcoat_texture`, `clearcoat_roughness_texture`,
  `clearcoat_normal_texture`, `sheen_color_texture`,
  `sheen_roughness_texture`, `anisotropy_texture`,
  `iridescence_texture`, and `iridescence_thickness_texture`.
  Recipe validation rejects out-of-range values before `MaterialDesc` setters
  can clamp or sanitize them. `ior` must be finite and either `0` or `>= 1.0`,
  matching `MaterialDesc`'s sentinel/domain. `transmission_texture` and
  `thickness_texture` remain invalid in recipe-authored materials until the GPU
  path can sample them without exceeding the WebGL2 fragment texture-unit floor.
  Scalar KHR volume fields `thickness_factor`, `attenuation_distance`, and
  `attenuation_color` are valid recipe-authored fields; they are verified with
  a coupled GPU volume scene because absorption only changes pixels when
  transmission, thickness, finite attenuation distance, and attenuation color
  are active together.
- `nodes[]` authored renderables or non-renderable group nodes with stable
  caller `id`. Renderables provide geometry/material together; groups omit
  both. All nodes accept optional manifest `name`, parent hierarchy, tags, visibility,
  layer mask, render group, tint, and optional `raw`, `trs`, `look_at`,
  `center`, `ground`, `fit_to_size`, `place_on`, or `align_to_anchor`
  transform. `place_on` and `look_at` may reference authored nodes declared
  earlier in the recipe or imported node paths of the form `<import_id>:/<path>`;
  `align_to_anchor` resolves `<import_id>.<anchor_name>` against a live imported
  anchor. Forward refs fail closed before build. Mesh nodes may declare
  `lods[]` entries of `{ "geometry", "max_screen_fraction" }`; the renderer
  selects the first sorted LOD whose projected node bounds fit the finite
  `(0, 1]` threshold, otherwise it uses the node's base geometry. This switches
  among explicitly-authored geometry resources and never fabricates or silently
  simplifies meshes.
- `anchors[]`, `connectors[]`, `bounds[]`, and `named_states[]` follow the
  accepted contract in `docs/specs/recipe-spatial-state-v1.md`. Every row has
  a recipe-local stable id. Targets are closed `node`, `import_root`, or exact
  `import_node` objects; authored positions and bounds are scene meters in
  glTF Y-up right-handed axes. Imported anchor/connector aliases preserve
  converted source-unit and coordinate metadata. Connector mates run through
  `Scene::connect_by_key` and fail with structured compatibility, handedness,
  scale, or snap diagnostics. Authored bounds may attach only to empty group
  nodes and never replace geometry/asset bounds. Named states contain only
  transforms, tints, and visibility, use acyclic single inheritance, reject
  animation-transform conflicts, and apply at most one active state after
  mating.
- `instance_sets[]` authored instance-set nodes with stable caller `id`,
  geometry/material references, optional parent and root transform, and
  per-instance stable ids with transform, opaque tint, and visibility. Hidden
  instances are filtered out of render preparation and inspection draw lists.
- `labels[]` free-standing `LabelDesc` nodes with stable caller `id`, text,
  optional parent, transform, color/background/halo colors, and size.
- `clipping_planes[]` arbitrary active clipping planes with stable caller `id`,
  finite non-zero normal, finite distance, and optional `active` flag. Recipes
  fail closed when active planes exceed the renderer's `max_clipping_planes`.
- `animations[]` authored keyframe clips with stable caller `id`, finite
  positive `duration`, and channels targeting authored or imported node ids.
  Channel paths are `translation`, `rotation`, `scale`, or `weights`; `weights`
  channels are valid only for imported morph targets until authored morphs land.
  Times must be finite, non-negative, and strictly increasing; values must match
  the channel arity and interpolation shape.
- `cameras[]` authored perspective cameras with stable caller `id`; at most one
  camera may be `active`, and camera `look_at` transforms may target authored
  nodes, instance sets, labels, or explicit world positions. Ergonomic
  `lens` presets (`wide_angle`, `standard`, `portrait`, `telephoto`) route
  through the matching `PerspectiveCamera` helper and are mutually exclusive
  with raw `fov_degrees`. Ergonomic `framing` routes through
  `FramingOptions` and `Scene::frame_bounds`; `framing.mode:
  "default_for_bounds"` routes through
  `Scene::add_perspective_camera_default_for`; `framing.mode:"principal_face"`
  chooses the imported/rendered bounds' thinnest axis as the view direction so
  thin CAD parts are framed by their broad face rather than their edge.
- `imports[]` may declare presentation-only `material` and `edge_emphasis`
  objects. `material` overrides the imported mesh material with either a named
  material `preset` plus optional tint or a recipe-owned PBR base color,
  roughness, metallic factor, and optional `double_sided:true` for CAD
  inspection views where thin imported surfaces must remain visible from the
  back side. `edge_emphasis` adds renderer edge-material overlay geometry for
  boundary and crease edges above the requested angle threshold. Both are Scena
  rendering controls only; they do not change imported geometry, CAD truth, or
  source glTF bytes.
- `lights[]` authored directional, point, spot, area, or `studio_rig` lights
  with presets, color, intensity/range/cone fields, area shape/size/flux
  fields, and transforms. `kind:"studio_rig"` routes through
  `Scene::add_studio_lighting()` and expands to stable `.key`, `.fill`, and
  `.rim` manifest entries.
- optional `scene` setup with scene presets `product_studio`, `cad_studio`,
  and `industrial_studio`, named or custom background, `default`/`uri`/`none`
  environment IBL, environment presets `studio` and `neutral_studio`, and
  grid-floor options including `line_width_px` and `under_bounds`. Scene
  presets route through the shared Rust scene-setup preset helper, including
  the matching auto-exposure scenario. Environment presets are checked by
  `RecipeBuildPolicy` and then loaded through
  `Assets::load_environment_preset`. Grid `under_bounds` defaults to `true`
  and routes to `GridFloorOptions::under_bounds(bounds)`.
  `scene.grid.reflection` enables a deterministic structured floor-reflection
  preset for product-style shots; material SSR is controlled separately through
  `render.screen_space_reflections`. URI environments are loaded under
  `RecipeBuildPolicy`; missing required environments fail the build.
- optional `render` setup with profile, quality, anti-aliasing, supersample,
  reconstruction filter, screen-space reflections, bloom, SSAO, depth of field,
  exposure EV or ergonomic `auto_exposure`, exposure compensation, and
  tonemapper.
  `auto_exposure` accepts `product_studio`, `indoor`, `outdoor`, or `mixed`
  and routes through `Renderer::set_auto_exposure(AutoExposureConfig::*)`.
  `auto_exposure` and fixed `exposure_ev` are mutually exclusive in v1.
  `exposure_compensation_ev` composes with `auto_exposure` and is rejected
  without it; use fixed `exposure_ev` only for full manual exposure.
  `render.metering` is valid only with `auto_exposure` and accepts
  `mode:"average"`, `center_weighted`, `highlight_weighted`, `subject`, or
  `spot`. Subject mode uses the shared recipe target grammar and accepts
  whole-import targets or authored/imported node targets; spot mode uses a
  normalized viewport
  `rect:{x,y,width,height}` that must be non-empty and contained within
  `[0,1]`. Subject mode accepts `fallback:"error"` (the default) or
  `fallback:"average_metering_with_warning"` for an explicit degraded fallback.
  Headless CPU `scena recipe render` resolves visible subject pixels with a
  semantic-AOV prepass and routes that rect into scene-linear subject-weighted
  auto exposure. Backend strict/degraded execution evidence is tracked in
  `docs/checklists/subject-driven-photo-rendering.md`.
  `anti_aliasing` accepts `none`, `fxaa`, `msaa4`, and `msaa8`;
  `quality:"high"` maps to sample AA. The opt-in `supersample` factor accepts
  `1`, `2`, `3`, `4`, or `8` and renders the capture at N× resolution before
  downsampling; `8` is accepted only when the scaled internal target stays
  within renderer limits. `reconstruction` accepts `box` (default), `tent`, or
  `gaussian`; the wider filters are hero-shot opt-ins because they can soften
  the frame. Supersampling composes with sample AA and should be reserved for
  hero captures because cost grows with N^2. `screen_space_reflections` accepts
  normalized `strength`, `roughness`, `horizon_fraction`, and `fade` and mirrors
  rendered scene content into the configured lower screen band and into
  high-metallic/low-roughness material fragments, fading to the environment-lit
  material at screen edges or where no screen-space sample exists. Bare
  `transmission_texture` and `thickness_texture` slots remain rejected until the
  GPU/WebGL2 texture-binding budget can support them. `depth_of_field` keeps
  the manual renderable form `focus_distance`, `aperture_f_stop`, and
  `radius_px`. It also accepts the subject-focus contract
  `focus:{mode:"subject",target:{kind:"import",id}}` or
  `focus:{mode:"subject",target:{kind:"node",id}}` with `coverage:"all"` and
  `strength:"subtle"`, and rejects ambiguous recipes that combine `focus` with
  `focus_distance`. `scena recipe render` resolves subject focus with a
  semantic-AOV prepass over the visible target pixels, using the median visible
  subject depth as the focal plane. A target with no semantic palette entry or
  no finite visible depth samples fails closed instead of guessing a distance.
  Recipe verification can pair manual DoF with `expect_quality.depth_of_field`
  so a same-backend no-DoF baseline proves the background lost Sobel detail
  while the focal subject remains sharp. The Sobel thresholds are finite
  non-negative measured values; `min_background_sobel_drop_fraction` and
  `max_focal_mean_delta` are normalized fractions.
- `expect.expect_transform[]` compares an authored/imported node target's
  inspected world transform against declared `translation`, `scale`, and/or
  intrinsic X/Y/Z `rotation_degrees` with explicit tolerances. It is a
  composition check, not an animation driver: mismatches fail verification with
  `transform_conformance_mismatch`.
- `expect.expect_separation[]` compares two authored/imported node targets'
  inspected world-space bounds and verifies they do not intersect, or that they
  satisfy an optional `min_gap` with `tolerance`. Use it for assembly,
  documentation, CAD, and industrial viewer scenes where parts must remain clear of each
  other; failures emit `separation_conformance_mismatch`.
  `expect_quality.reflection` checks the floor/reflection-surface region;
  `expect_quality.reflection.target` checks the projected region for a specific
  node. Use `max_firefly_fraction` when polished/metallic IBL reflections must
  reject isolated bright HDR specks separately from missing reflection
  structure. Use `min_bright_fraction` and `min_dark_fraction` when a chrome or
  mirror-metal subject must show white-card highlights plus dark edge
  definition; failures emit `reflection_chrome_read_missing`.
  `expect_quality.exposure.min_mean_luminance_srgb8` and
  `expect_quality.exposure.max_mean_luminance_srgb8` set fixture-specific
  product subject luminance bands when exact subject observations are available;
  they are sRGB8 luminance values in `[0,255]`, while clip-fraction thresholds
  remain normalized `[0,1]`. `expect_quality.area_light` checks the projected target receiver for
  measurable finite-emitter soft-shadow structure and emits
  `area_light_soft_shadow_checked` or
  `area_light_soft_shadow_insufficient` with observed penumbra width, luminance
  levels, shadow contrast, and emitter extent. `expect_quality.grounding`
  checks the projected target's contact band against nearby open floor and
  emits `contact_shadow_checked` or `contact_shadow_missing` with observed
  contact-shadow delta. `expect_quality.depth_of_field` accepts a focal
  `target`, optional `background_target`, and thresholds for background Sobel
  loss and focal-subject preservation; it emits `depth_of_field_checked`,
  `depth_of_field_background_detail_missing`,
  `depth_of_field_blur_insufficient`, or `depth_of_field_focal_softened`.
  Values that
  renderer setters would clamp
  (`bloom.intensity`, `bloom.radius_px`, `ssao.intensity`,
  `ssao.depth_threshold`, `screen_space_reflections.*`) are rejected during
  validation when out of range.
- optional `section_box` directives over an import's bounds or an authored/
  imported node target
- `measurements[]` distance overlays with units and labels
- `callouts[]` anchored to an import root, authored/imported node, or world
  point with label offsets
- optional `exploded_view` directives over an import's root hierarchy
- `fonts[]` TrueType/OpenType font resources with `id`, `uri`, and optional
  `optional`; labels may reference a font by id with `font`
- font-backed labels support basic Latin glyphs, per-glyph metrics, and kerning
  pairs; complex-script text fails closed with `unsupported_feature` rather
  than falling back to broken glyph order
- one optional `capture` directive with `width` and `height`
- opaque caller `metadata`

Unknown fields fail closed. Known future feature sections such as `primitives`,
`viewer_profile`, `environment`, and `placements` emit
`unsupported_feature` until the feature slice that owns them implements the
section. Workflow fields such as
`steps`, `sequence`, `loop`, `branch`, `timeline`, and `script` emit
`unsupported_workflow`; recipes must stay snapshots and the host owns cadence
and sequencing.

`scena.scene_recipe_validation.v1` contains `ok` plus deterministic diagnostics
with `code`, `severity`, JSON `path`, `message`, `help`, optional
`suggestion`, optional structured `candidates`, and `auto_fixable`. Unknown
node, geometry/mesh-resource, material, import, and environment-preset
references use the same normalized, deterministic ranking as runtime lookups,
cap candidates at three, and mirror the first candidate into `suggestion` for
older consumers. Unknown-field suggestions use bounded string distance, for
example `importe` suggests `imports`.

`scena.scene_recipe_build.v1` is emitted by `SceneHostCore::build_recipe_json`.
It maps caller-authored recipe ids to runtime-scoped SceneHost handles that later
patch, overlay, verification, and interaction calls use within that host
generation. These handles are stable for that live host, not persistence IDs.
Import entries contain the caller `id`, resolved `uri`, `import_handle`, `root_handles`,
`primary_root`, and `nodes_by_path`. Path keys use the shared namespace
`<import_id>:/<path>`; `<import_id>:/` names the primary import root and named
glTF children are included when their authored path is unambiguous. `nodes`,
`cameras`, `lights`, and `animations` are targetable manifest entries with
stable handles.
`anchors`, `connectors`, `connections`, `bounds`, and `named_states` map each
persistent feature id to its resolved target, source provenance, units, and
outcome. Runtime node handles are explicitly build-scoped; the manifest marks
feature identity as `persistent_recipe_id`. Connection rows report the source
and target recipe ids plus measured snap distance. Bounds rows report source,
local/world space, finite min/max, and `scene_meters`. Named-state rows report
inheritance, active selection, resolved channel counts, and status.
The additive `instances` rows preserve each authored instance `id` and owning
`set_id` beside its runtime-scoped `set_handle` and `instance_id`; semantic AOV
legends use this mapping without relabeling a runtime handle as durable storage.
Authored mesh nodes, instance-set nodes, label nodes, and cameras include their
recipe ids and stable handles in the same targetable lists as imported node
handles. `geometries`, `materials`, and `fonts` are non-targetable resources
without handles; authored geometries report real vertex and index counts, and
font entries report the loaded font resource kind.
`RecipeBuildPolicy` is operator-owned configuration, not part of the authored
recipe schema, and fail-closed policy or required-load failures appear as
deterministic build diagnostics.
The CLI default root is its current directory. Repeatable
`--allow-root <directory>` options add canonical, existing operator roots
without disabling that sandbox. Resource paths are canonicalized independently
before containment checks: a `..` path or a symlink inside an allowed root
cannot authorize a target outside it. The same effective policy is used by
full validation, recipe build/render, and asset-or-recipe inspect, diagnose,
doctor, and repair paths.
Resolved scene assets retain the semantic `AssetLoadOptions` and evidence used
to populate their cache entry. A cache hit under the same path cannot bypass a
later stricter missing-resource rule or smaller fetch budget; cross-policy reuse
is allowed only when the retained load evidence proves the active operator
policy was already satisfied.

`scena.recipe_build_result.v1` is emitted by
`scena recipe build <recipe.json> [--max-imports <n>] [--allow-root
<directory>]...`, by `scena doctor` for a
recipe, and as the structured rejection result when an asset-or-recipe command
cannot build the complete recipe. It nests the existing
`scena.scene_recipe_build.v1` manifest, the effective
`scena.recipe_policy.v1`, and execution counters. This command constructs only
assets, scene graph state, build budgets, and SceneHost handle tables; it does
not construct a `Renderer`, GPU context, prepared resources, rendered frame, or
capture. Imports, external glTF resources, decoded texture sources, and
explicit/preset environments are resolved under the reported policy; required
environment URIs are checked for real source availability without creating a
renderer. A denied URI is rejected before fetch. `asset_fetches` is a measured
per-store attempt counter: successful and failed source-byte requests count,
including external resources and optional sidecar probes, while cache hits and
embedded bytes do not. Attempted missing resources emit deterministic build
diagnostics. The command is not a render preview and intentionally does not use
ambiguous `--dry-run` wording.

`expect` is additive recipe sugar over existing verification contracts. Color
expectations compile to `scena.appearance_expectation.v1`, pick expectations
compile to `scena.interaction_expectation.v1`, and bbox/no-warning checks read
the real `scena.render_introspection.v1` report. Expectation targets use stable
recipe ids resolved through `scena.scene_recipe_build.v1`; missing ids fail
closed as verification reasons.

`scena.recipe_render_result.v1` is emitted by
`scena recipe render <recipe.json> --verify --out <png>`. It
nests the build manifest, capture descriptor, render-introspection report, and
aggregate verification report. Verification includes a nested
`scena.scene_composition.v1` report for declared-element composition
conformance and a nested `scena.render_quality.v1` report for always-on severe
quality failures and opt-in `expect_quality` / `expect_reference` checks.
Top-level `ok` is true only when build, introspection, and verification are
all true. If build fails before a frame exists, `capture` and `introspection`
are `null` rather than fabricated.

`scena.photo_render_result.v1` is emitted by
`scena photo render <asset-or-recipe> [--intent camera-behavior] --out <png>
--report <json> [--emit-recipe <recipe.json>] [--subject import:<id>|node:<id>]`. It is the stdout command
envelope for the bounded camera-behavior easy path. On success it reports
`ok:true`, the normalized intent, emitted PNG/report/descriptor paths, optional
emitted recipe path, the selected candidate, subject quality metrics, and any
failure codes. Build or acceptance failure remains a domain failure on stdout
with the same top-level schema and `ok:false`; CLI usage, I/O, policy, and
internal failures use `scena.cli_error.v1` on stderr.

`scena.photo_report.v1` is written to the `--report` path by the same command.
It records the input source kind/path, resolved subject target, evaluated
candidates, selected candidate, acceptance bands, subject quality metrics,
bounded retry policy/attempts, bounded work metrics, artifacts, optional
emitted recipe path, and build summary. The first implementation slice supports the
`camera_behavior` intent for imported assets, recipe inputs, and authored-node
recipe subjects through the CLI.
`scena validate` fully validates the required report sections; missing
candidate attempts, selected candidate, exposure report, focus report, or
quality verdict fail closed instead of passing as envelope-only JSON.

`scena.photo_plan.v1` is emitted by
`scena photo plan <asset-or-recipe> [--intent camera-behavior] --out <plan.json>`.
It wraps the deterministic `scena.photo_candidate_plan.v1` candidate list with
input source, resolved subject, optional scorer output, selected candidate id,
rejected-candidate reasons, staging choices, and emitted-recipe artifact path.
It is intentionally render-free so `scena photo plan` can expose the exact plan
without writing the final high-resolution image. Multi-import recipes can pass
`--subject import:<id>` and authored-scene recipes can pass
`--subject node:<id>`; both route through the same recipe target resolver used
by subject metering, subject focus, and target-quality expectations.

Photo reports include a nested
`scena.photo_shaded_candidate_selection.v1` object. It records the bounded
low-resolution shaded candidate pass: candidate render size, candidate budget,
evaluated count, total candidate pixels, selected composition candidate,
per-candidate subject metrics, scorer reasons, the photographic asset-health
report, and the reused
`scena.render_quality.v1` report for each rendered candidate. This pass is an
audit trail for composition/staging selection; it is not a hidden final render
artifact.

The subject metrics are geometry-attributed from the semantic ID, depth, and
world-normal buffers. In addition to framing and exposure they report empty
space, depth/normal variation, highlight fraction/continuity/distribution,
contact-shadow presence/softness, silhouette separation, saturation, color
cast, and reflection washout. Candidate selection scores the combined
photographic result and may perform at most one measured lighting correction
per low-resolution candidate. It does not infer the subject from its color.

`asset_health` uses the classifications `safe_repair`,
`appearance_change_required`, and `unrecoverable`. Safe repairs are applied and
reported. Appearance-changing repairs name the required caller decision.
Unrecoverable missing or malformed geometry/material/texture data rejects the
photo command rather than producing `ok:true`. The supported automatic promise
is coherent visible geometry with sufficient physical material information;
the renderer does not invent components, markings, substances, or texture
content. Topology evidence includes boundary, nonmanifold, folded, and
self-intersecting face counts plus exact duplicate vertices safely removed.
Scene-hierarchy evidence reports hidden, coincident duplicate, microscopic,
detached, and far-outlier components; these are never silently deleted.

Photo reports also include top-level `work_metrics` for the complete
camera-behavior loop. The block records composition candidate budget/count, shaded
candidate budget/renders/pixels, final retry render budget/renders/pixels,
total render, prepare, and capture calls, GPU readback copies, blocking
poll/wait counts, subject-meter sample counts, and the allocation/work policy
(`allocation_policy:"bounded_by_candidate_count_and_frame_pixels"`).
Wall-clock timings are explicitly report-only (`timing_policy:"report_only"`,
`wall_clock_thresholds:"not_used"`) because shared CI/build hosts are not
controlled performance hardware; deterministic counts and allocation/copy
budgets remain the blocking evidence.

Recipes may opt into the same camera-behavior path with an optional top-level
`photo` section:

```json
{
  "schema": "scena.scene_recipe.v1",
  "imports": [{ "id": "subject", "uri": "model.glb" }],
  "photo": {
    "intent": "camera_behavior",
    "subject": { "kind": "import", "id": "subject" }
  }
}
```

`photo.subject` also accepts the explicit subject spec form
`{target:{kind:"import",id:"subject"},fallback:"error"}` or
`{target:{kind:"node",id:"hero"},fallback:"error"}`. The direct target form
remains valid for existing recipes. `fallback:"error"` is the default;
`fallback:"average_metering_with_warning"` permits a deliberate degraded path
and must be reported as such by the rendering surface.

Compatibility: `photo`, `render.metering`, subject-focus
`render.depth_of_field.focus`, and `render.exposure_compensation_ev` are
additive. A recipe that omits `photo.intent` keeps the pre-existing explicit
render contract: fixed `render.exposure_ev` is still the full-manual exposure
path, `render.depth_of_field.focus_distance` is still the manual focus path,
and `render.metering:{mode:"average"}` with `render.auto_exposure` remains
average metering rather than being silently promoted to subject metering.
Use `render.exposure_compensation_ev` when an auto-exposed product/studio shot
needs a small nudge; use fixed `render.exposure_ev` only when intentionally
leaving automatic metering.

An advanced recipe can keep manual camera composition while still using
subject-driven exposure and focus:

```json
{
  "schema": "scena.scene_recipe.v1",
  "imports": [{ "id": "subject", "uri": "model.glb" }],
  "cameras": [{
    "id": "main",
    "kind": "perspective",
    "active": true,
    "framing": { "preset": "three_quarter_front_right", "fill": 0.72 }
  }],
  "render": {
    "auto_exposure": "product_studio",
    "exposure_compensation_ev": 0.3,
    "metering": {
      "mode": "subject",
      "target": { "kind": "import", "id": "subject" },
      "surround_weight": 0.1
    },
    "depth_of_field": {
      "focus": {
        "mode": "subject",
        "target": { "kind": "import", "id": "subject" }
      },
      "coverage": "all",
      "strength": "subtle"
    }
  }
}
```

`scena recipe render <recipe.json> --verify --out <png>` resolves that subject,
applies the camera-behavior composition defaults, runs the bounded candidate
render/exposure loop, and includes product-profile import framing/exposure
checks in the nested `scena.scene_composition.v1` report. Recipes that request
`render.depth_of_field.focus.mode:"subject"` surface a nested
`scena.focus_report.v1` under render introspection. The focus report is bound to
the final capture payload and carries target, mode, coverage, strength, resolved
focus distance, visible-depth percentiles, visible pixel count, confidence, and
frame key. `scena photo render` includes the same report shape in its
`scena.photo_report.v1`; camera-behavior photo renders resolve it from visible
semantic subject-depth samples and report the physical circle-of-confusion
model used by the final camera. Photo reports also
include `scena.exposure_report.v1`, which records the selected EV, measured
subject luminance, subject low/high clip fractions, suggested compensation, and
capture-bound frame key. Ordinary recipe render introspection includes the same
report when `render.auto_exposure` is active, including the metered luminance,
target luminance, base EV, compensation EV, clamp state, highlight guard,
subject sample count, and the metering domain. When
`render.metering.mode:"subject"` is used on the headless CPU backend, the
report's subject sample count is nonzero and the linked
`scena.subject_observation.v1` entries identify the subject rect used by
metering. `metering_domain:"scene_linear_pre_tonemap"` is the strict
camera-behavior evidence domain; `metering_domain:"encoded_output_feedback"` is
reported as degraded because the sample already includes the current exposure.
Remaining backend evidence classes and multi-view candidate planning work are
tracked in `docs/checklists/subject-driven-photo-rendering.md`.

### `scena.subject_observation.v1`

Produced as a capture-bound nested report when recipe verification or photo
rendering has an authored subject target. It records the subject source
(`render.metering`, `render.depth_of_field.focus`, or `photo.subject`), target
kind/id, resolved runtime handles, exact frame key, projected bounds, visible
bounds, visible pixel count, visible fill, visible/projected fraction,
occlusion estimate, optional visible-depth percentiles, optional exact
`pixel_quality` metrics, and explicit fallback flags/reason codes.

`pixel_quality` is present when the subject was measured from exact visible
subject pixels, currently the headless CPU semantic-AOV path. It reports final
sRGB8 subject luminance (`mean_luminance_srgb8`,
`luminance_stddev_srgb8`, `luminance_range_srgb8`), low/high clip fractions,
and sample count. Product-quality verification consumes this block for
subject-specific exposure and material-readability checks instead of relying
on color-difference foreground guesses.

`status:"observed"` means the payload has non-empty projected and visible
bounds for the same completed readback frame. `fallback.degraded:true` is still
allowed on observed payloads when part of the evidence is weaker than exact
semantic attribution, for example current `scena photo render` camera-behavior
reports that use projected/luminance subject metrics while the semantic mask is
not yet routed through the photo path. `status:"degraded"` or
`status:"unavailable"` require reason codes and must not be treated as exact
subject-mask proof.

The frame key follows the same capture-provenance rule as
`scena.focus_report.v1` and `scena.exposure_report.v1`: `state_binding` must be
`exact_readback_completion`, and stale rendered-frame keys are invalid.

The stable fixture lives at
`tests/assets/stable-contracts/subject_observation.v1.json`.

### `scena.capture_sequence_result.v1`

Produced by `scena recipe capture <recipe.json> --out-dir <dir>`. The default
canonical order is front, top, right, isometric in a right-handed, +Y-up world:
front looks from +Z, right from +X, and top uses the closest stable orbit state
to +Y (a one-degree pole offset) with -Z as screen-up. The isometric camera
occupies the +X,+Y,+Z eye octant. `--views none` suppresses
canonical frames. `--turntable <n>` samples evenly spaced yaw angles at a fixed
20-degree pitch. `--clip <name> --frames <n>` resolves an authored recipe
animation or imported clip and samples the inclusive `[0,duration]` interval.
The combined canonical, turntable, and clip frame count is bounded to 360.

One `SceneHostCore` is constructed and reused for the entire sequence. Every
frame follows the normal `set_camera -> prepare -> render -> capture` lifecycle;
clip frames seek through the host animation API before prepare. Each frame row
records its index, kind, label, PNG and descriptor paths, complete camera state,
capture payload hash, and canonical-view, turntable, or clip/time metadata. The
contact-sheet rows retain the same index, label, kind, tile bounds, and payload
hash, so an agent can map a review tile back to the exact source frame.

The stable core output is deterministic full-resolution PNG frames plus a PNG
contact sheet. Contact-sheet tiles use nearest-neighbor thumbnails capped at
192 pixels on their longest edge, so a bounded 360-frame sequence does not
retain 360 full-resolution RGBA buffers merely to compose the sheet; tile rows
still report the original frame payload hash.
GIF and video containers are deliberately outside renderer ownership: use an
external GIF/video encoder over the numbered frames without changing sampling
semantics. `video_encoding.status` and `reason` keep that boundary explicit.
The stable fixture lives at
`tests/assets/stable-contracts/capture_sequence_result.v1.json`.

### `scena.semantic_aov_result.v1`

Produced by `scena recipe aov <recipe.json> --out-dir <dir> [--passes
id,depth,normal]`. CPU v1 emits deterministic paletted RGBA8 node/instance IDs,
16-bit grayscale linear camera-distance depth, and RGBA8 world normals from one
prepared SceneHost state. Palette index zero is transparent background. Every
other palette entry has a legend row containing a runtime-scoped host node
handle, optional runtime instance identity, and a recipe-local stable node or
instance ID when the build manifest owns one. Runtime handles are explicitly not persistence identifiers.

The report pins single-center sampling, nearest-opaque-fragment occlusion,
camera-space depth in scene meters, world-normal encoding, and the absence of
MSAA/post-process resolve. Alpha-blended/transmissive geometry, strokes, labels,
particles, helpers, and overlays remain background and are counted in
`exclusions`; v1 never silently attributes them. The complete normative
contract is `docs/specs/semantic-aov-v1.md`, and the stable fixture is
`tests/assets/stable-contracts/semantic_aov_result.v1.json`.

### `scena.scene_recipe_diff_result.v1`

Produced by `scena diff <before.recipe.json> <after.recipe.json>`. The default
mode is renderer-free and reports typed material, node, camera, and recipe-level
changes with stable IDs, add/remove/modify/reorder kinds, field paths, and an
explicit numeric tolerance. Generic arbitrary-JSON diffing is intentionally
outside renderer ownership.

Inequality is a successful comparison and therefore exits 0 by default with
`exit_policy:"report_only"`. Add `--exit-code` for CI; an unequal report still
stays on stdout but exits 1 with `exit_policy:"difference_is_failure"`. Parse,
policy, build, and I/O failures remain distinct command failures.

`--render --out-dir <dir>` additionally renders both recipes through the same
CPU SceneHost lifecycle, reuses `scena.capture_baseline.v1` for the aggregate
RGBA8 comparison, and writes `before.png`, `after.png`, `diff.png`, and the
complete result JSON. Changed color pixels are sampled against each recipe's
semantic ID AOV and grouped by recipe-local stable node, imported-node, or
authored-instance candidate. The summary is an exact partition:
`changed_pixels = attributed_pixels + ambiguous_pixels + unattributed_pixels`.

Attribution is deliberately conservative: anti-aliased identity edges are ambiguous;
different before/after identity candidates are ambiguous; and zero-ID pixels
are unattributed background or an excluded transparent, transmission, stroke,
label, particle, helper, or overlay surface. The report does not claim causal
attribution for those surfaces or for post-processing. No competitive uniqueness claim
is made without a dated, source-backed cross-product matrix. The stable fixture
is `tests/assets/stable-contracts/scene_recipe_diff_result.v1.json`.

`scena validate-recipe <recipe.json>` defaults to `full_resolution`: it first
runs shape validation without rendering, then resolves the same resource plan
consumed by `recipe build`. The plan inventories imports, environment URI or
builtin preset, fonts, every authored material texture slot, and nested glTF
dependencies reached while loading an import. Each resource row carries its
JSON path, kind, authored URI, normalized URI, required state, and status. The
report also carries the effective `RecipeBuildPolicy` roots and limits.
Resource diagnostics repeat the normalized URI, required state, allowed roots,
and an actionable remedy. Missing required resources make `ok=false`; optional
resources become warnings with `optional_skipped` status. Assets whose maximum
extent falls outside an import's expected range emit `extent_out_of_range`.

`--syntax-only` is the explicit no-I/O alternative. It inventories authored
resources as `not_checked`, sets `validation_mode:"syntax_only"` and
`execution_equivalent:false`, and must not be presented as proof that a build
can start. `--full` spells the default mode explicitly.

The stable fixtures live at
`tests/assets/stable-contracts/scene_recipe.v1.json` and
`tests/assets/stable-contracts/scene_recipe_validation.v1.json`; the build
manifest fixture lives at
`tests/assets/stable-contracts/scene_recipe_build.v1.json`; the combined
renderer-free build fixture lives at
`tests/assets/stable-contracts/recipe_build_result.v1.json`; the combined
render/verify fixture lives at
`tests/assets/stable-contracts/recipe_render_result.v1.json`. `scena
validate-recipe <recipe.json>` emits validation JSON on stdout and exits
non-zero when `ok` is false. When built with `inspection`, `scena render`,
`scena inspect`, and `scena diagnose --visibility` accept either a direct asset
path or a recipe file. Recipes with overlay directives are instantiated through
the same `SceneHostCore` path as native hosts, apply section boxes,
measurements, callouts, and exploded-view directives, frame with
`frame_all_with_overlays`, then use the normal prepare/render/capture path.
Invalid recipe and recipe-adjacent command fixtures live under
`tests/assets/recipe-invalid/`; they pin missing assets, invalid transforms,
oversized assets, unknown placement verbs, and stale handle diagnosis for the
currently landed recipe surface. Future recipe sections must add matching
invalid fixtures for their own enum, material, handle, profile, or verb
diagnostics when they land.

### `scena.placement_result.v1`

Produced by the `scena place <recipe.json> (--import <id>|--node <id>) --verb <verb>` CLI
command and represented by `ScenePlacementResultV1`. A placement result is a
preview: it proposes a `Transform` for the requested recipe import or authored
node and does not
mutate a host document or rewrite the recipe file.

The additive typed `target:{kind:"import"|"node",id}` identifies the target;
the legacy `import_id` string remains populated for v1 readers. Bounds verbs
support imported assets and authored nodes with direct geometry. Anchor and
connector verbs remain explicitly import-only.

The v1 placement result supports bounds-authored placement:

- `center`: translate the import so its transformed bounds center reaches
  `--target x,y,z`, defaulting to the world origin.
- `ground`: translate the import so its transformed bounds minimum Y reaches
  `--ground-y`, defaulting to `0`.
- `fit_to_size`: uniformly scale the import into `--min-size` / `--max-size`.
- `look_at`: orient the import so local `-Z` points at `--target x,y,z` or at
  the bounds center of `--target-import`.
- `align_to_anchor`: align a source authored anchor/connector frame to a target
  authored anchor/connector frame and emit the resulting import transform.
- `place_on`: translate a source authored anchor/connector point onto a target
  authored anchor/connector point while preserving source orientation.

`ok=false` reports include deterministic diagnostics with `code`, `severity`,
JSON `path`, `message`, `help`, optional `suggestion`, namespace-aware
`candidates`, and `auto_fixable`. Unknown imports or nodes, unsupported verbs,
missing bounds, invalid size ranges, and
asset load failures return placement JSON on stdout with a non-zero exit.
Missing or ambiguous authored anchors/connectors also fail closed with
placement JSON on stdout and a non-zero exit.

The stable fixture lives at
`tests/assets/stable-contracts/placement_result.v1.json`.

Placement and recipe-patch transforms use the same canonical raw discriminator
(`kind:"raw"`), quaternion order, and default omission as recipe transforms;
zero translation and unit scale may therefore be absent. Readers keep accepting
the pre-discriminator v1 result shape, but every newly serialized result
includes the discriminator.

### `scena.recipe_patch.v1`

Produced by adding `--apply` to `scena place`. The default placement command
remains a side-effect-free preview. Apply mode is also filesystem-safe: it
emits a complete canonical `updated_recipe` document rather than editing the
source in place. The patch is addressed by a typed recipe-local import or node
ID, never by a transient SceneHost handle, and includes the previous/new
transform plus a semantic JSON-path change summary. `formatting_preserved=false` explicitly
states that canonical JSON output does not promise source whitespace or key
order.

`source_sha256` binds the result to the exact input. Callers that pass
`--expect-source-sha256 <hex>` receive an `ok=false` `stale_source` diagnostic
when the file changed, before any updated document is emitted. Applying the
reported transform and rebuilding must produce the same transform as the
placement preview.

The stable fixture lives at
`tests/assets/stable-contracts/recipe_patch.v1.json`.

### `scena.schema_catalog.v1` and `scena.schema_entry.v1`

Produced by `schema_catalog_v1`, `schema_entry_report_v1`, and the `scena`
binary's `schema list` / `schema get <schema>` commands. The catalog is the
agent-facing discovery surface for public stable JSON contracts; each entry
contains the contract name, owner module, short summary, optional feature flag,
and stable fixture path when one exists.

`scena schema list` writes `scena.schema_catalog.v1` JSON to stdout. `scena
schema get <schema>` writes `scena.schema_entry.v1` JSON containing one catalog
entry, a representative valid example parsed from that contract's fixture, and
`invalid_example` when the contract has a small canonical failing case. Unknown
schema names fail closed with a non-zero exit code and a near-miss suggestion
when one is available.

For `scena.scene_recipe.v1`, `schema get` also includes an authoritative
`scena.field_model.v1` table. Each row names a JSON path and exposes its type,
requiredness, closed enum values, numeric bounds, default when one exists,
deprecation state, examples, owning contract, feature requirements, and
cross-field constraints. The complete path set is generated from the
`SceneRecipeV1` serde/JSON-Schema definition; curated validation metadata may
only enrich those generated rows. A bidirectional parity test and an omitted
SSR-field mutation prevent the public model from drifting behind accepted
camera/framing, lighting/environment, material/texture, animation, post,
capture, import, placement, or expectation fields. Round-trip and invalid
fixtures live under `tests/assets/schema-field-model/`.

The stable fixtures live at
`tests/assets/stable-contracts/schema_catalog.v1.json` and
`tests/assets/stable-contracts/schema_entry.v1.json`.

### `scena.contract_validation.v1` and `scena.json_schema_export.v1`

`scena validate <file>` reads the embedded versioned `schema` field and
dispatches to the owning typed validator for scene recipes, appearance and
interaction expectations, recipe patches, and capability reports. Malformed
JSON, missing or unknown schemas, and typed mismatches return the same
`scena.contract_validation.v1` envelope on stdout with exit 65. Unknown names
include nearest catalog candidates. Other cataloged output/report contracts
receive explicit envelope validation and a limitation directing callers to
their producing workflow for runtime semantics.

`scena schema json <scena.*.vN>` emits `scena.json_schema_export.v1`. The scene
recipe export is generated from the same Rust/serde types used by the runtime.
Contracts without a complete generated schema receive a draft 2020-12 envelope
schema, never invented field constraints. Every export declares that JSON
Schema cannot prove runtime resources, filesystem policy, cross-resource
identity, backend capabilities, or other consuming-workflow semantics.

Use generic validation before invoking a consumer, then use the owning workflow
when its runtime checks matter:

```bash
scena validate expectation.json
scena schema json scena.scene_recipe.v1 > scene-recipe.schema.json
scena validate recipe.json
scena validate-recipe recipe.json --full
```

The stable fixtures live at
`tests/assets/stable-contracts/contract_validation.v1.json` and
`tests/assets/stable-contracts/json_schema_export.v1.json`.

### `scena.field_model.v1`

The versioned nested field-discovery contract returned by `schema get` for
contracts that publish field-level authoring guidance. `contract` identifies
the modeled public schema and `fields[]` carries path-level constraints. Empty
or absent enum/range/default members mean no narrower constraint is promised;
they must not be inferred from the representative example.

The additive `owner`, `feature_requirements`, and `constraints` members default
when omitted, preserving old v1 fixtures. They make capability and cross-field
requirements discoverable without changing the recipe envelope version.

The stable fixture lives at
`tests/assets/stable-contracts/field_model.v1.json`.

### `scena.agent_guide.v1`

Produced by `agent_guide_v1` and `scena guide agent --json`. The contract embeds
the packaged public LLM application-builder guide and indexes its canonical
commands, schemas, policy rules, and template-discovery surface. It is available
in the default CLI build and uses only package-owned public documentation; root
`AGENTS.md`, private builder access, and untracked `.codex` files are never
runtime dependencies. `scena guide agent --markdown` is the explicit raw
Markdown export.

The stable fixture lives at
`tests/assets/stable-contracts/agent_guide.v1.json`.

### `scena.vocab.v1`

Produced by `vocabulary_report_v1` and the `scena vocab list` / `scena vocab
get <name>` commands. Each closed vocabulary has a stable name, integer
version, owning module, ordered value list, and value metadata for aliases,
deprecation, feature requirements, and capability requirements. Renderer
backends, recipe material kinds, material/lens/framing/scene/environment/
exposure/quality presets, named colors, placement verbs, alpha modes, texture
color spaces, tonemappers, easing curves, cameras, light kinds, and per-kind
light presets are discoverable without scraping help text. The value list is
generated from the same registries used by validation; a parity validator and
known-bad omission test prevent advertised and accepted sets from diverging.

The stable fixture lives at `tests/assets/stable-contracts/vocab.v1.json`.

### `scena.recipe_policy.v1`

Produced by `RecipeBuildPolicy::to_schema_report` and `scena policy recipe
[--allow-root <directory>]...`.
The report exposes the effective network switch, URI schemes, canonical local
roots, and every recipe/resource/output limit. Every value carries its source;
the standalone discovery command reports `compiled_default` for defaults and
`operator_override` for each added canonical root. Commands that add operator
roots report the same effective policy in their result rather than implying
that a recipe changed policy. Missing/non-directory roots are argument errors;
there is no global sandbox-disable option.

The stable fixture lives at
`tests/assets/stable-contracts/recipe_policy.v1.json`.

`scena.recipe_patch.v1` returns a complete canonical recipe rather than a
format-preserving text edit. Before serialization, relative import, font,
environment, and material-texture URIs are resolved against the source recipe,
so writing `updated_recipe` at a different path preserves resource identity.
The only reported semantic change is the recipe-ID-selected transform; URI
rebasing is semantic-preserving canonicalization.

### CLI discovery and errors

`scena.cli_help.v1` retains the stable command strings and adds one
`command_contracts` row per command. Each row declares non-empty `emits.success`
and `emits.error` schema sets plus its `failure_exit_classes`. The top-level
`error_taxonomy` binds comparison, usage, input, unsupported, runtime, internal,
I/O, policy, and interruption classes to stable process statuses.
Command-specific reports remain on stdout;
dispatch and argument failures that cannot produce one use
`scena.cli_error.v1` on stderr. Fatal stdout write failures remain the distinct
`scena.cli_io_error.v1` contract.

`scena.cli_error.v1` carries `code`, `exit_class`, `exit_code`, `message`,
optional `path`, command `context`, curated `help`, structured `candidates`, and
an optional machine-applicable `fix`. Unknown schema
and agent-template names populate it from the live catalog using the shared
normalized ranking algorithm; unrelated argument failures emit an empty list.
Consumers must use this field instead of parsing “did you mean” prose.
In short, `scena.cli_error.v1` includes a structured `candidates` array for
machine repair.

The declarations are checked against a command/schema/outcome evidence matrix.
Every non-`cli_error` row names a real CLI integration fixture, while a fast
live matrix executes argument failures for every command and polymorphic
recipe/load failures for the commands that can return them. The matrix also
guards against advertising internal build reports that are actually wrapped by
`recipe_render_result`, or `asset_doctor` results on commands whose real path
returns `cli_error`.

The schema catalog is authoritative for public CLI/library JSON. CI, browser,
benchmark, review, and release proof payloads not listed in the public catalog
are internal gate artifacts, even when they carry a versioned schema string;
their authorities are the owning files under `docs/specs/`, release staging,
or test lanes. Moving one into a public command requires adding a catalog row,
stable fixture, documentation, and compatibility checks.

The `scena.asset_doctor.v1` error shape is intentionally polymorphic across
`inspect`, `render`, and `diagnose`: asset-load failure before the requested
operation emits the same doctor report and a non-zero exit, while successful
operations emit their command-specific inspection/introspection/diagnosis
schemas. Machine help declares both possibilities.

All `scena` CLI commands write stable JSON to stdout. Human-readable command
errors use stderr only when no contract report can be produced. Asset-loading
failures for agent-facing asset commands emit `scena.asset_doctor.v1` on
stdout with a non-zero exit instead of prose-only command errors. The global
`--compact` emits one-line JSON and `--pretty` explicitly selects the
deterministic indented default. These global flags apply to success envelopes,
domain failures on stdout, CLI errors on stderr, and help; they are mutually
exclusive and never change envelope semantics. `--round-floats <0..6>` rounds
floating-point JSON numbers after report generation while preserving integer
handles and counts; commands default to their built-in stable precision when
the option is omitted.

### `scena.asset_doctor.v1`

Produced by `Assets::doctor_asset_path`, `Assets::doctor_loaded_asset`,
`SceneHostCore::asset_doctor_json`, the browser `SceneHost.assetDoctorJson()`
method, and `scena doctor <asset.gltf|asset.glb>`. The report is a
renderer-owned asset readiness diagnosis for agents and host tools that need
actionable load/material/extension findings without compiling Rust in the
loop.

The report has `ok`, `status`, `asset`, `summary`, optional
`asset_load_report`, and `findings`. `ok=true` means the doctor found no
error-severity findings; warning-severity findings such as missing external
image bytes or material fallbacks remain visible in `summary.warning_count` and
`findings[]` but do not by themselves fail the report. Use strict load options
or the catalog readiness gate when an asset must be complete rather than merely
loadable. Each finding has a stable `severity`, `code`, optional
`path`/`field`/`extension`, `message`, `help`, `suggested_fix`, and `source`.
The `scena doctor` CLI writes the report to stdout and exits non-zero when
`ok=false`.

For a parsed scene recipe, `scena doctor <recipe.json>` instead emits
`scena.recipe_build_result.v1`. That path validates and resolves every import
with the same `RecipeBuildPolicy` and manifest builder used by `scena recipe
build`; a rejected later import cannot be hidden behind a successful doctor
result for import 1.

The stable fixture lives at
`tests/assets/stable-contracts/asset_doctor.v1.json`.

### `scena.asset_conversion.v1`

Produced by `scena-convert` for every machine-mode conversion plan, completed
conversion, invalid request, unavailable converter, or converter failure. The
top-level `ok`, `status`, `workflow`, and `message` fields are always present.
When known, `tool`, `input`, `output`, and the exact `command` are included;
`tool_exit_code` records a completed external process, including nonzero
failure. Status is one of `planned`, `converted`, `invalid_request`,
`tool_unavailable`, or `conversion_failed`.

External tool output is never written beside the machine document. Each
non-empty tool line becomes a `diagnostics[]` row with `stream`, `severity`,
and `message`. Stdout is informational; stderr is warning severity after a
successful conversion and error severity after failure. Domain errors keep the
single report on stdout and use exit 2 for invalid requests or exit 1 for tool
startup/conversion failures. Fatal CLI I/O remains
`scena.cli_io_error.v1` on stderr. `--human` is the explicit plain-text and
live-tool-output mode; `--json` explicitly selects the stable machine mode.

The stable fixture lives at
`tests/assets/stable-contracts/asset_conversion.v1.json`.

### `scena.connector_browser.v1`

Produced by `SceneHostCore::connector_browser_json`,
`SceneHostCore::connector_browser_subtree_json`, and
`SceneHostCore::connector_browser_selection_json`. The report lists imported
connectors with stable host node handles, metadata, and optional target-import
candidate previews. Compatibility is metadata-driven: kind/allowed mates,
tags, polarity, units, coordinate system, and the existing connector solver
decide whether a mate is compatible. Scena reports invalid mate reasons and a
snap preview, but it does not compute mesh clearance, collision, or physical
feasibility.

Each report includes a `scope`, `summary`, source `connectors`,
`target_connectors`, pairwise `candidates`, and optional `visual_cues` for
ghost transforms and connection lines. Candidate distances, tolerances, line
points, and ghost transforms are rounded for deterministic JSON. Compatible
out-of-range candidates stay `compatible: true` with `snap_ready: false`; only
metadata or solver rejection increments `invalid_count`.

The stable fixture lives at
`tests/assets/stable-contracts/connector_browser.v1.json`.

### `scena.product_options.v1`

Produced and consumed by `SceneHostCore::store_product_options`,
`SceneHostCore::store_product_options_json`, `SceneHostCore::product_options`,
`SceneHostCore::product_options_json`, `SceneHostCore::apply_product_option`,
and `SceneHostCore::apply_product_option_json`. The contract is a
host-authored option-group model over visual changes only: every option owns a
`VisualPatchV1`, and applying an option delegates to the normal visual patch
path.

Each group has a stable `id`, display `label`, optional `active` option id, and
ordered `options`. Each option has a stable `id`, display `label`,
`patch`, and optional opaque `metadata`. Group and option ids must be
non-empty, contain no control characters, and be unique within their scope.
Stored active ids must reference an option in the same group.

Applying an option returns `VisualPatchResultV1`. The group becomes active only
when that result has an empty `failed[]` array; per-entry visual patch failures
are reported without updating the active option. Scena does not interpret
pricing, compatibility, inventory, persistence, or host document semantics in
this contract.

The stable fixture lives at
`tests/assets/stable-contracts/product_options.v1.json`.

### `scena.presentation_timeline.v1`

Consumed by `SceneHostCore::timeline_patch`,
`SceneHostCore::timeline_patch_json`, `SceneHostCore::seek_timeline`,
`SceneHostCore::seek_timeline_json`, `SceneHostCore::advance_timeline`, and
`SceneHostCore::advance_timeline_json`. The timeline is a host-ticked
presentation helper that emits a normal `VisualPatchV1` for the requested
time; it does not define a separate mutation model or an autonomous render
loop.

Each timeline has optional named `camera_bookmarks` and ordered `actions`.
Actions can apply a direct `VisualPatchV1`, apply a stored
`SceneHostVisualStateV1` by name, apply a named camera bookmark, or sample an
existing host animation mixer as an animation clip. Animation actions use
`VisualPatchV1.animation_time` with `mode: "seek"`; hosts create and own the
mixer before the timeline references it.

Animation segments are resolved and validated against the bound mixer before
any action is emitted. A missing `end_seconds` means the clip duration; an
explicit end beyond the duration is clamped to the duration; and a
`start_seconds` beyond the duration rejects the whole timeline seek before
patch application. A zero-duration imported static clip accepts only start
zero and always samples zero. `Once` mixers sample the inclusive end and hold
that exact terminal pose. `Repeat` mixers sample the half-open `[start,end)`
segment and wrap exact or floating-point-near boundaries to `start`. Invalid
segments are top-level `InvalidInput` errors, not repeated per-entry
`failed[]` rows on every host tick.

`timeline_patch` includes actions whose `at_seconds` is less than or equal to
the requested time and flattens them into one deterministic last-wins patch.
Repeated seeks to the same final state are therefore no-ops when the underlying
scene state is already current. `advance_timeline` is a convenience wrapper
around `seek_timeline(current_seconds + delta_seconds)`; the host still owns
the clock.

The stable fixture lives at
`tests/assets/stable-contracts/presentation_timeline.v1.json`.

### `scena.scene_host_grounding.v1`

Produced by `SceneHostCore::apply_product_grounding_preset()` and
`SceneHostCore::apply_product_grounding_preset_json()`, with the matching
browser `SceneHost.applyProductGroundingPresetJson()` method. The preset
composes existing product-viewer primitives: studio visuals, a floor receiver
under a target node, SSAO when the backend supports it, and lighting defaults.

The report contains the stable target handle, generated `floor_handles`, the
active grounding paths, and explicit fallbacks. `active_paths` may include
`floor_receiver` and `screen_space_ambient_occlusion`. Directional shadow
receiver reporting is not part of this contract until a future proof gate adds
the real path.

SSAO is reported as ambient occlusion only. It may darken depth-contact edges,
but it is not a drop-shadow or physical-shadow substitute. Consumers should
read `fallbacks[]` before making visual-quality claims.

The stable fixture lives at
`tests/assets/stable-contracts/scene_host_grounding.v1.json`.

`scena.capture.v1` small example:

```json
{
  "schema": "scena.capture.v1",
  "width": 64,
  "height": 64,
  "pixel_format": "rgba8",
  "payload": {
    "kind": "rgba8",
    "byte_length": 16384,
    "fnv1a64": "0123456789abcdef"
  },
  "revisions": { "structure": 3, "transform": 2, "appearance": 1, "interaction": 0 },
  "camera": {
    "active": true,
    "world_transform": {
      "translation": [0.0, 0.0, 2.0],
      "rotation": [0.0, 0.0, 0.0, 1.0],
      "scale": [1.0, 1.0, 1.0]
    },
    "projection": {
      "kind": "perspective",
      "vertical_fov_radians": 1.0471976,
      "aspect": 1.0,
      "near": 0.01,
      "far": 1000.0
    }
  },
  "viewport": {
    "width": 64,
    "height": 64,
    "logical_width": 64.0,
    "logical_height": 64.0,
    "device_pixel_ratio": 1.0
  },
  "backend": "headless",
  "auto_frame": null,
  "pixels": {
    "nonblack": 1024,
    "bbox": { "min_x": 16, "min_y": 16, "max_x": 47, "max_y": 47, "width": 32, "height": 32 },
    "center": [255, 255, 255, 255],
    "fnv1a64": "0123456789abcdef"
  }
}
```

CPU-headless capture descriptors and bytes are deterministic for the same
rendered scene state and renderer settings. Browser/GPU capture descriptors
bind pixels to rendered scene revisions, active camera state, viewport/DPR, and
backend capabilities; they are not a cross-machine exact-byte promise.

### `scena.annotation_projection.v1`

Produced by `Scene::annotation_projection_report` and
`SceneHost.annotationProjectionsJson()`. Represented by
`AnnotationProjectionReportV1`.

Required top-level fields:

- `schema`
- `coordinate_space`
- `viewport_width`
- `viewport_height`
- `annotations`

Each annotation entry contains `id`, `node_handle`, `x`, `y`, and `visible`.
`node_handle` is `null` for world anchors and standalone scene reports without a
handle map. `SceneHost` fills it with the same stable host node handle used by
`setTransforms`, `inspectJson`, `pick`, and draw-list inspection. Native scene
reports use the viewport dimensions supplied by the caller. `SceneHost` reports
`coordinate_space: "css_pixels"` and uses the host's logical viewport size, so
browser pages can apply the coordinates directly to HTML overlay elements.

Small example:

```json
{
  "schema": "scena.annotation_projection.v1",
  "coordinate_space": "css_pixels",
  "viewport_width": 120,
  "viewport_height": 80,
  "annotations": [
    {
      "id": "left-label",
      "node_handle": 12,
      "x": 42.5,
      "y": 40.0,
      "visible": true
    }
  ]
}
```

### `scena.asset_geometry_summary.v1`

Produced by `SceneAsset::geometry_summary()` and represented by
`SceneAssetGeometrySummary`.

Required top-level fields:

- `schema`
- `node_count`
- `mesh_count`
- `primitive_count`
- `bounds`
- `provenance`
- `source_units`
- `source_coordinate_systems`

`bounds` is the asset-local AABB after applying the asset's node hierarchy and
instance transforms. `source_units` and `source_coordinate_systems` contain
only metadata stored on the asset itself; import-time options are not folded
into this asset report.

Small example:

```json
{
  "schema": "scena.asset_geometry_summary.v1",
  "node_count": 3,
  "mesh_count": 1,
  "primitive_count": 1,
  "bounds": {
    "min": [-0.5, -0.5, -0.5],
    "max": [0.5, 0.5, 0.5]
  },
  "provenance": {
    "source_path": "models/cell.glb",
    "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "license": null,
    "generator": null,
    "derivatives": []
  },
  "source_units": ["millimeters"],
  "source_coordinate_systems": []
}
```

### `scena.asset_load_report.v1`

Produced by `AssetLoadReport<SceneAsset>::to_schema_json()` and represented by
`AssetLoadReportV1`. `SceneHost.instantiateUrlWithReportJson()` wraps the same
report together with the import handle that was created from it.

Required top-level fields:

- `schema`
- `path`
- `cache_hit`
- `requested_options`
- `cache_entry_options`
- `fetched_bytes`
- `external_buffers`
- `external_images`
- `provenance`
- `geometry`
- `warnings`
- `progress_events`
- `external_resources`
- `material_fallbacks`

`provenance` is the loaded asset's `AssetProvenance` value. `geometry` is the
loaded asset's `scena.asset_geometry_summary.v1` report and carries the same
provenance value. Warnings are typed and currently include
`external_image_missing` and `external_buffer_missing`. `requested_options`
records the semantic policy of this call; `cache_entry_options` records the
policy that produced reused evidence. They differ only for a compatible cache
hit whose retained warnings and fetched-byte total prove the active request was
satisfied. Cache-hit reports preserve warnings, external-resource status rows,
material fallback provenance, and external resource counts from the original
load, while `fetched_bytes` remains `0` for the cache-hit call itself. Both
option fields are additive in v1 and deserialize to lenient, unlimited defaults
when absent.

`external_resources` is a deterministic status table for external buffers and
images discovered from the glTF. Each row has `kind` (`buffer` or `image`),
`path`, nullable `index`, `status` (`fetched`, `missing`, or
`skipped_unsupported_format`), nullable `bytes`, and nullable `reason`.
`progress_events` also includes `external_image_fetched` entries for fetched
image files. `material_fallbacks` records explicit material-source substitutions
such as an optional `KHR_texture_basisu` source falling back to an authored PNG,
or a material texture whose source bytes were missing and therefore binds
`scena.material.fallback_texture` at render time. Fallback rows include
`material_index` when the fallback is tied to a source glTF material. These two
fields are additive in v1 and deserialize as empty arrays when absent.

Small example:

```json
{
  "schema": "scena.asset_load_report.v1",
  "path": "models/cell.glb",
  "cache_hit": false,
  "requested_options": {
    "strict_textures": false,
    "strict_external_resources": false,
    "fetch_byte_limit": null
  },
  "cache_entry_options": {
    "strict_textures": false,
    "strict_external_resources": false,
    "fetch_byte_limit": null
  },
  "fetched_bytes": 4096,
  "external_buffers": 1,
  "external_images": 0,
  "provenance": {
    "source_path": "models/cell.glb",
    "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    "license": null,
    "generator": null,
    "derivatives": []
  },
  "geometry": {
    "schema": "scena.asset_geometry_summary.v1",
    "node_count": 3,
    "mesh_count": 1,
    "primitive_count": 1,
    "bounds": null,
    "provenance": {
      "source_path": "models/cell.glb",
      "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "license": null,
      "generator": null,
      "derivatives": []
    },
    "source_units": [],
    "source_coordinate_systems": []
  },
  "warnings": [
    {
      "kind": "external_image_missing",
      "path": "models/missing.png",
      "reason": "not found"
    }
  ],
  "progress_events": [
    { "kind": "load_started", "path": "models/cell.glb" },
    { "kind": "cached", "path": "models/cell.glb" }
  ],
  "external_resources": [
    {
      "kind": "image",
      "path": "models/missing.png",
      "index": null,
      "status": "missing",
      "bytes": null,
      "reason": "not found"
    }
  ],
  "material_fallbacks": []
}
```

### `scena.asset_catalog.v1`

Consumed by `Assets::validate_asset_catalog()` and represented by
`AssetCatalogV1`. This is a transient host-owned manifest for renderer-relevant
asset readiness. It is not a package database, approval workflow, search index,
versioning system, or persisted document model.

Required top-level fields:

- `schema`
- `assets`

Each asset entry requires `id`, `display_name`, and `source`. Additive v1
fields include `required_files`, `preview`, `declared_units`,
`source_coordinate_system`, `expected_bounds`, `required_anchors`,
`required_connectors`, `required_tags`, `material_requirements`, `license`,
`provenance`, `categories`, and `tags`. `declared_units` and
`source_coordinate_system` are strings so invalid or misspelled values can
round-trip into a readiness finding instead of failing JSON parsing.

`preview.kind` is `image` or `generated`. The current validator checks that an
image preview has a path or that generated preview metadata has positive
dimensions; rendered browser preview proof is a separate checklist item.

### `scena.asset_readiness_report.v1`

Produced by `Assets::validate_asset_catalog()` and represented by
`AssetReadinessReportV1`.

Required top-level fields:

- `schema`
- `ok`
- `summary`
- `assets`

`ok` is false when any asset has an `error` finding. Asset reports include the
host-owned ID/name/source, declared units and coordinate system, preview status,
the loaded `scena.asset_geometry_summary.v1` and nested
`scena.asset_load_report.v1` when loading succeeds, material fallback rows, and
deterministically ordered findings.

Current finding codes include `load_failed`, `required_file_missing`,
`source_units_unknown`, `invalid_source_units`,
`source_coordinate_system_unknown`, `invalid_source_coordinate_system`,
`bounds_missing`, `bounds_not_finite`, `bounds_out_of_range`,
`extent_out_of_range`, `invalid_anchor`, `invalid_connector`,
`required_anchor_missing`, `required_connector_missing`,
`required_tag_missing`, `required_material_variant_missing`,
`base_color_texture_missing`, `material_fallback_used`,
`external_resource_missing`, and `preview_missing`.

Each finding carries `severity`, `code`, `message`, `help`, nullable `path`,
and nullable `field`. `path` is a fetcher path when available; `field` names
the manifest field the host should inspect or repair.

### `scena.scene_host_asset_import.v1`

Produced by `SceneHostCore::instantiate_url_with_report_json`,
`SceneHostCore::instantiate_url_under_with_report_json`, and the matching WASM
`SceneHost` methods. Represented by `SceneHostAssetImportReportV1`.

Required top-level fields:

- `schema`
- `import`
- `asset_load_report`

`import` is a generation-checked host import handle. `asset_load_report` is the
nested `scena.asset_load_report.v1` report for the asset load that produced the
import. The same host owns the import handle, node handle namespace, and
inspection handle namespace.

Additive fields `material_variants` and `active_variant` report the declared
source `KHR_materials_variants` names and current active variant for that
import. Initial import reports normally use `null`/absent `active_variant`;
host-backed `scena.scene_inspection.v1.imports[]` is the current-state report
after visual patches apply or clear variants.

Small example:

```json
{
  "schema": "scena.scene_host_asset_import.v1",
  "import": 7,
  "material_variants": ["midnight", "noon"],
  "active_variant": null,
  "asset_load_report": {
    "schema": "scena.asset_load_report.v1",
    "path": "models/part.glb",
    "cache_hit": false,
    "fetched_bytes": 4096,
    "external_buffers": 0,
    "external_images": 0,
    "provenance": {
      "source_path": "models/part.glb",
      "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
      "license": null,
      "generator": null,
      "derivatives": []
    },
    "geometry": {
      "schema": "scena.asset_geometry_summary.v1",
      "node_count": 1,
      "mesh_count": 1,
      "primitive_count": 1,
      "bounds": null,
      "provenance": {
        "source_path": "models/part.glb",
        "source_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "license": null,
        "generator": null,
        "derivatives": []
      },
      "source_units": [],
      "source_coordinate_systems": []
    },
    "warnings": [],
    "progress_events": []
  }
}
```
