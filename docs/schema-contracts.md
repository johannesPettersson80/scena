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
  - `scena.visibility_diagnosis.v1`
  - `scena.visual_repair_plan.v1`
  - `scena.agent_loop_result.v1`
  - `scena.agent_smoke_template.v1`
  - `scena.appearance_expectation.v1`
  - `scena.appearance_introspection.v1`
  - `scena.animation_introspection.v1`
  - `scena.interaction_expectation.v1`
  - `scena.interaction_verification.v1`
  - `scena.scene_recipe.v1`
  - `scena.scene_recipe_validation.v1`
  - `scena.placement_result.v1`
  - `scena.asset_load_report.v1`
  - `scena.asset_geometry_summary.v1`
  - `scena.annotation_projection.v1`
  - `scena.subtree.v1`
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

Each node entry contains the stable host `handle`, optional `name`, and sorted
`tags` for the requested subtree.

In 1.7, subtree node `name` is reserved for a future stable naming policy and
is always serialized as `null`. Use `tags` or host-owned handles for stable
identification in this release.

Small example:

```json
{
  "schema": "scena.subtree.v1",
  "nodes": [
    { "handle": 42, "name": null, "tags": ["frame", "product"] },
    { "handle": 84, "name": null, "tags": ["part"] }
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
variant names fail closed as per-entry errors.

`labels` entries are host-owned overlay label/annotation anchors. Scena stores
and projects the anchor; the host owns visible text and DOM/native overlay
content. A label target can be `node`, `world`, or `clear`; IDs must be
non-empty strings and node targets use stable host node handles.

`metadata` is caller-owned JSON. It is returned in `VisualPatchResultV1` only
when `echo_metadata` is `true`, so agents can correlate responses without
forcing every result to echo arbitrary host data.

The result includes:

- `applied`: changed-entry counts for `transforms`, `tints`, `visibility`,
  `camera`, `transforms_eased`, `tints_eased`, `camera_eased`,
  `animation_time`, `selection`, `hover`, `material_variants`, and `labels`;
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
  "metadata": { "request_id": "agent-step-42" },
  "echo_metadata": true
}
```

Stable fixtures live at
`tests/assets/stable-contracts/visual_patch.v1.json` and
`tests/assets/stable-contracts/visual_patch_result.v1.json`.

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
- `context_lost`, `context_restored`, and `device_lost`.
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

The verification report contains:

- `schema`
- `ok`
- `summary`: step, failure, hit, miss, and event counts, plus
  `rendered_feedback_checked`
- `steps`: ordered expected/observed interaction results
- `reasons`: stable failure codes such as `hit_mismatch`,
  `handle_mismatch`, `hover_missing`, `selection_missing`, and
  `event_sequence_mismatch`
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
and emits a manifest with schema `scena.agent_smoke_template.v1`.

The manifest contains:

- `schema`
- `name`
- `status`: `ready` or `deferred`
- `required_features`: crate features required to run the generated commands
- `files`: generated file paths and their schema names
- `commands`: argv arrays beginning with `scena`, expected output schema,
  expected `ok` value, and artifact paths that should exist after the command
- `notes`: explanatory text for deferred templates

Ready templates are CLI-only acceptance examples over the normal
prepare/render/capture/report path. Deferred templates, such as CAD inspection
and documentation rendering, emit no runnable commands until their Phase 2
measurement, section, exploded-view, callout, and annotation dependencies land.

Stable fixture:
`tests/assets/stable-contracts/agent_smoke_template.v1.json`.

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

`revisions` includes `structure`, `transform`, additive `appearance`, and
`interaction`. Older `scena.scene_inspection.v1` payloads without
`appearance`, `tint`, `material`, `instance`, or `instance_sets` still
deserialize with defaults.

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

Produced by `Renderer::introspect_capture` and
`RenderIntrospectionReportV1::from_capture` when the `inspection` feature is
enabled. The report binds a `scena.capture.v1` descriptor, a
`scena.scene_inspection.v1` report, and `RendererStats` into a small,
deterministically ordered agent-readable summary.

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

The first v1 slice classifies capture-derived visibility failures:
`empty_frame`, `no_visible_drawables`, `all_culled`, `tiny_in_frame`, and
`cropped`. `ok` is false only when an `error`-severity reason is present;
warning-only framing reasons are still returned but do not fail the agent loop.
`visible_pixel_fraction`, `content_bbox_css_px`, and
`content_bbox_fraction` are computed from pixels that differ from the
configured shader-encoded background by more than the implementation's byte
tolerance, not from literal non-black pixels. `luminance` values are computed
from shader-encoded RGBA8 bytes on a 0-255 scale. `fixes[]` carries stable
Scena action codes such as `frame_bounds` and `set_visible`; callers decide
whether to apply a suggested action.

Summary mode omits `nodes_detail`; detail mode includes stable node handles
and conservative coverage categories where known. In the first v1 slice,
per-node pixel coverage is not yet attributed, so visible nodes report
`coverage: "unknown"`, `nodes_summary.unknown_coverage` counts those nodes,
and `nodes_summary.clipped` / `nodes_summary.transparent` are reserved
not-yet-computed counters that remain zero. `affected_handles` and
`fix.patch` may be empty in this render-level report; use
`scena.visibility_diagnosis.v1` for targeted actionable patches. The report
rounds floating-point summaries to stable precision and keeps large artifacts
outside JSON, referenced through explicit paths or the nested capture summary.

The stable fixture lives at
`tests/assets/stable-contracts/render_introspection.v1.json`. The `scena`
binary's first CLI transport is `scena render <asset-or-recipe> --introspect
--out <png>` when built with the `inspection` feature. It writes the PNG and
capture descriptor artifacts, then emits this report on stdout.

### `scena.visibility_diagnosis.v1`

Produced by `Renderer::diagnose_visibility` and
`VisibilityDiagnosisReportV1::from_inspection` when the `inspection` feature is
enabled. The report consumes a `scena.scene_inspection.v1` report, renderer
stats, and an optional stable node handle. It returns ranked reasons and data
fix suggestions without mutating scene state.

Required top-level fields:

- `schema`
- `ok`
- `target`
- `reasons`
- `fixes`
- `summary`
- `evidence`

The first v1 slice covers `not_prepared`, `missing_camera`,
`no_visible_drawables`, `all_culled`, `stale_handle`, `node_hidden`, and
`zero_scale`. `all_culled` is emitted only when renderer stats show that every
inspection-visible drawable was culled; partial frustum culling of a healthy
scene is not a failure. Each reason includes severity, confidence, whether it is
auto-fixable, affected stable handles when known, and a short message. Fixes
use stable Scena action codes such as `prepare`, `set_camera`,
`frame_bounds`, `set_visible`, and `set_transform`. Content-risk fixes are
reported as data and must be applied explicitly by the host or CLI caller.
Summary mode returns only reasons, fixes, and counts; detail mode may include
supporting evidence rows.

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
<seconds> [--expect-change]` when the `inspection` feature is enabled.
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

When `--expect-change` is supplied, `ok` is false for error-severity reasons
such as missing clip, non-advancing sampled times, frozen channels, invalid
channel values, or unchanged rendered output. The CLI writes the report JSON to
stdout and exits non-zero when `ok` is false. Missing clips also return a
machine-readable report with available clip names in the reason message.

The stable fixture lives at
`tests/assets/stable-contracts/animation_introspection.v1.json`.

### `scena.scene_recipe.v1` and `scena.scene_recipe_validation.v1`

Produced and consumed by `validate_scene_recipe_json`,
`validate_scene_recipe_value`, `parse_valid_scene_recipe_json`, and the `scena
validate-recipe <recipe.json>` command. A recipe is a transient declarative
snapshot input for Scena, not a project file, workflow script, host document
model, or persisted application state.

The first v1 slice supports:

- `schema: "scena.scene_recipe.v1"`
- `imports[]` entries with stable caller `id`, glTF/GLB `uri`, optional
  `transform`, and optional `expected_extent`
- one optional `capture` directive with `width` and `height`
- opaque caller `metadata`

Unknown fields fail closed. Known future feature sections such as `materials`,
`cameras`, `labels`, `section_box`, and `measurements` emit
`unsupported_feature` until the feature slice that owns them implements the
section. Workflow fields such as `steps`, `sequence`, `loop`, `branch`,
`timeline`, and `script` emit `unsupported_workflow`; recipes must stay
snapshots and the host owns cadence and sequencing.

`scena.scene_recipe_validation.v1` contains `ok` plus deterministic diagnostics
with `code`, `severity`, JSON `path`, `message`, `help`, optional
`suggestion`, and `auto_fixable`. Unknown-field suggestions use bounded string
distance, for example `importe` suggests `imports`.

The stable fixtures live at
`tests/assets/stable-contracts/scene_recipe.v1.json` and
`tests/assets/stable-contracts/scene_recipe_validation.v1.json`. `scena
validate-recipe <recipe.json>` emits validation JSON on stdout and exits
non-zero when `ok` is false. When built with `inspection`, `scena render`,
`scena inspect`, and `scena diagnose --visibility` accept either a direct asset
path or a recipe file and use the first recipe import through the same
load/prepare/render/capture path as direct assets.

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
entry and a representative example parsed from that contract's fixture.
Unknown schema names fail closed with a non-zero exit code and a near-miss
suggestion when one is available.

The stable fixtures live at
`tests/assets/stable-contracts/schema_catalog.v1.json` and
`tests/assets/stable-contracts/schema_entry.v1.json`.

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
such as an optional `KHR_texture_basisu` source falling back to an authored PNG.
Fallback rows include `material_index` when the fallback is tied to a source
glTF material. These two fields are additive in v1 and deserialize as empty
arrays when absent.

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

Small example:

```json
{
  "schema": "scena.scene_host_asset_import.v1",
  "import": 7,
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
