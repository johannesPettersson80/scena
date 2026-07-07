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
  - `scena.render_quality.v1`
  - `scena.scene_composition.v1`
  - `scena.visibility_diagnosis.v1`
  - `scena.visual_repair_plan.v1`
  - `scena.agent_loop_result.v1`
  - `scena.agent_smoke_template.v1`
  - `scena.appearance_expectation.v1`
  - `scena.appearance_introspection.v1`
  - `scena.animation_introspection.v1`
  - `scena.interaction_expectation.v1`
  - `scena.interaction_verification.v1`
  - `scena.connector_browser.v1`
  - `scena.scene_recipe.v1`
  - `scena.scene_recipe_validation.v1`
  - `scena.scene_recipe_build.v1`
  - `scena.placement_result.v1`
  - `scena.asset_load_report.v1`
  - `scena.asset_geometry_summary.v1`
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

When a report is emitted by `SceneHost`, the host's generation-checked `u64`
node handle namespace is authoritative. The same handle must be accepted by host
mutation APIs and appear in host-backed inspection reports.

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

The `scena examples agent <template> [--out <dir>]` CLI command writes a small
set of recipe, expectation, and artifact-path files for a named smoke template
and emits a manifest with schema `scena.agent_smoke_template.v1`. The
`scena examples agent get <name> [--out <dir>]` form emits authored-from-scratch
starter snippets such as `primitive_scene`, `cad_plate`, `dashboard_bars`,
`machine_state_viewer`, and `product_configurator`.

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

Additive optional fields:

- `post_processing`: active/available post-processing pass metadata for the
  current renderer configuration.

Capability enum values use serde names such as `headless`, `supported`,
`degraded`, and `feature_disabled`.

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
helpers, and `SceneHost.captureJson()`. Represented by `CaptureDescriptor`.
PNG helpers such as `CaptureRgba8::to_png_bytes`,
`CaptureRgba8::write_png`, `Renderer::capture_png_bytes`,
`Renderer::capture_png`, SceneHost `capture_png_bytes`, and browser
`capturePng()` delegate to the same descriptor-bound capture object.

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
- `auto_frame`
- `pixels`

Large RGBA8 bytes are returned outside JSON through `CaptureRgba8::rgba8` or
the browser `capture().rgba8` typed array. PNG bytes are returned outside JSON
through native byte vectors/files or browser `capturePng().png`. JSON carries
byte length, dimensions, format, and FNV-1a hash metadata.

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

Summary mode omits `nodes_detail`; detail mode includes stable node handles
and draw-derived node state. `nodes_summary.transparent` is computed from
non-opaque prepared draw materials. Failure reasons include stable
`affected_handles` whenever the renderer can identify the node, and fixes that
change scene state carry apply-ready `scena.visual_patch.v1` bodies. The report
rounds floating-point summaries to stable precision and keeps large artifacts
outside JSON, referenced through explicit paths or the nested capture summary.

The stable fixture lives at
`tests/assets/stable-contracts/render_introspection.v1.json`. The `scena`
binary's first CLI transport is `scena render <asset-or-recipe> --introspect
--out <png>` when built with the `inspection` feature. It writes the PNG and
capture descriptor artifacts, then emits this report on stdout.

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
validate-recipe <recipe.json>` command. A recipe is a transient declarative
snapshot input for Scena, not a project file, workflow script, host document
model, or persisted application state.

The current v1 recipe slice supports:

- `schema: "scena.scene_recipe.v1"`
- `imports[]` entries with stable caller `id`, glTF/GLB `uri`, optional
  `optional` skip policy, optional `transform`, and optional
  `expected_extent`
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
- `nodes[]` authored renderables with stable caller `id`, geometry/material
  references, optional manifest `name`, parent hierarchy, tags, visibility,
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
  exposure EV or ergonomic `auto_exposure`, and tonemapper.
  `auto_exposure` accepts `product_studio`, `indoor`, `outdoor`, or `mixed`
  and routes through `Renderer::set_auto_exposure(AutoExposureConfig::*)`.
  `auto_exposure` and fixed `exposure_ev` are mutually exclusive in v1.
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
  GPU/WebGL2 texture-binding budget can support them. `depth_of_field` accepts
  `focus_distance`, `aperture_f_stop`, and `radius_px`; recipe verification can
  pair it with `expect_quality.depth_of_field` so a same-backend no-DoF
  baseline proves the background lost Sobel detail while the focal subject
  remains sharp. The Sobel thresholds are finite non-negative measured values;
  `min_background_sobel_drop_fraction` and `max_focal_mean_delta` are normalized
  fractions.
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
  definition; failures emit `reflection_chrome_read_missing`. `expect_quality.area_light` checks the projected target receiver for
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
`skins`, `morphs`, `particles`, `anchors`, `connectors`, `bounds`, and
`named_states` emit
`unsupported_feature` until the feature slice that owns them implements the
section. Workflow fields such as
`steps`, `sequence`, `loop`, `branch`, `timeline`, and `script` emit
`unsupported_workflow`; recipes must stay snapshots and the host owns cadence
and sequencing.

`scena.scene_recipe_validation.v1` contains `ok` plus deterministic diagnostics
with `code`, `severity`, JSON `path`, `message`, `help`, optional
`suggestion`, and `auto_fixable`. Unknown-field suggestions use bounded string
distance, for example `importe` suggests `imports`.

`scena.scene_recipe_build.v1` is emitted by `SceneHostCore::build_recipe_json`.
It maps caller-authored recipe ids to the stable SceneHost handles that later
patch, overlay, verification, and interaction calls use. Import entries contain
the caller `id`, resolved `uri`, stable `import_handle`, `root_handles`,
`primary_root`, and `nodes_by_path`. Path keys use the shared namespace
`<import_id>:/<path>`; `<import_id>:/` names the primary import root and named
glTF children are included when their authored path is unambiguous. `nodes`,
`cameras`, `lights`, and `animations` are targetable manifest entries with
stable handles.
Authored mesh nodes, instance-set nodes, label nodes, and cameras include their
recipe ids and stable handles in the same targetable lists as imported node
handles. `geometries`, `materials`, and `fonts` are non-targetable resources
without handles; authored geometries report real vertex and index counts, and
font entries report the loaded font resource kind.
`RecipeBuildPolicy` is operator-owned configuration, not part of the authored
recipe schema, and fail-closed policy or required-load failures appear as
deterministic build diagnostics.

`expect` is additive recipe sugar over existing verification contracts. Color
expectations compile to `scena.appearance_expectation.v1`, pick expectations
compile to `scena.interaction_expectation.v1`, and bbox/no-warning checks read
the real `scena.render_introspection.v1` report. Expectation targets use stable
recipe ids resolved through `scena.scene_recipe_build.v1`; missing ids fail
closed as verification reasons.

`scena.recipe_render_result.v1` is emitted by
`scena recipe render <recipe.json> --introspect --verify --out <png>`. It
nests the build manifest, capture descriptor, render-introspection report, and
aggregate verification report. Verification includes a nested
`scena.scene_composition.v1` report for declared-element composition
conformance and a nested `scena.render_quality.v1` report for always-on severe
quality failures and opt-in `expect_quality` / `expect_reference` checks.
Top-level `ok` is true only when build, introspection, and verification are
all true. If build fails before a frame exists, `capture` and `introspection`
are `null` rather than fabricated.

`scena validate-recipe <recipe.json>` first runs shape validation without
rendering, then loads declared assets far enough to validate asset presence and
optional `expected_extent` scale sanity. Missing or unloadable assets emit an
`asset_load_failed` error diagnostic and make `ok=false`. Assets whose maximum
extent falls outside an import's expected range emit warning-level
`extent_out_of_range`; warnings remain visible in JSON but do not make
`ok=false`.

The stable fixtures live at
`tests/assets/stable-contracts/scene_recipe.v1.json` and
`tests/assets/stable-contracts/scene_recipe_validation.v1.json`; the build
manifest fixture lives at
`tests/assets/stable-contracts/scene_recipe_build.v1.json`; the combined
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

Produced by the `scena place <recipe.json> --import <id> --verb <verb>` CLI
command and represented by `ScenePlacementResultV1`. A placement result is a
preview: it proposes a `Transform` for the requested recipe import and does not
mutate a host document or rewrite the recipe file.

The v1 placement result supports bounds-authored recipe import placement:

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
JSON `path`, `message`, `help`, optional `suggestion`, and `auto_fixable`.
Unknown imports, unsupported verbs, missing bounds, invalid size ranges, and
asset load failures return placement JSON on stdout with a non-zero exit.
Missing or ambiguous authored anchors/connectors also fail closed with
placement JSON on stdout and a non-zero exit.

The stable fixture lives at
`tests/assets/stable-contracts/placement_result.v1.json`.

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

The stable fixtures live at
`tests/assets/stable-contracts/schema_catalog.v1.json` and
`tests/assets/stable-contracts/schema_entry.v1.json`.

All `scena` CLI commands write stable JSON to stdout. Human-readable command
errors use stderr only when no contract report can be produced. Asset-loading
failures for agent-facing asset commands emit `scena.asset_doctor.v1` on
stdout with a non-zero exit instead of prose-only command errors. The global
`--round-floats <0..6>` option rounds floating-point JSON numbers after report
generation while preserving integer handles and counts; commands default to
their built-in stable precision when the option is omitted.

### `scena.asset_doctor.v1`

Produced by `Assets::doctor_asset_path`, `Assets::doctor_loaded_asset`,
`SceneHostCore::asset_doctor_json`, the browser `SceneHost.assetDoctorJson()`
method, and the `scena doctor <asset-or-recipe>` CLI. The report is a
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

The stable fixture lives at
`tests/assets/stable-contracts/asset_doctor.v1.json`.

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
`external_image_missing` and `external_buffer_missing`. Cache-hit reports
preserve warnings, external-resource status rows, material fallback provenance,
and external resource counts from the original load, while `fetched_bytes`
remains `0` for the cache-hit call itself.

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
