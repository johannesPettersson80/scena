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
  - `scena.capture.v1`
  - `scena.asset_load_report.v1`
  - `scena.asset_geometry_summary.v1`
  - `scena.annotation_projection.v1`

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
fixtures and asserts their schema strings or, for nested value contracts, their
required fields.

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

## Shipped v1 contracts

### `scena.capability_report.v1`

Produced by `CapabilityReport::to_schema_json()` and represented by
`CapabilityReportV1`.

Required top-level fields:

- `schema`
- `capabilities`
- `adapter`
- `diagnostics`

Capability enum values use serde names such as `headless`, `supported`,
`degraded`, and `feature_disabled`.

### `scena.scene_inspection.v1`

Produced by `SceneInspectionReport::to_schema_json()` when the `inspection`
feature is enabled and represented by `SceneInspectionReportV1`.

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
`render_group`, `helper_on_top`, and optional `tint`. `tint` is included
because per-node highlight state is part of the render state a host may need
to prove.

Topology helpers on `SceneInspectionReportV1`:

- `node_by_handle(handle)`
- `children_of(handle)`
- `roots()`
- `find_by_tag(tag)`

### `scena.capture.v1`

Produced by `capture_rgba8`, `Renderer::capture_rgba8`, viewer `capture()`
helpers, and `SceneHost.captureJson()`. Represented by `CaptureDescriptor`.

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
the browser `capture().rgba8` typed array. JSON carries byte length, dimensions,
format, and FNV-1a hash metadata.

The descriptor binds to the renderer's last rendered frame state. If scene
revisions or the active camera changed after `render()` and before `capture()`,
capture returns `CaptureError::StaleRender` instead of serializing a false
pixels-to-state binding.

Small example:

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
  "revisions": { "structure": 3, "transform": 2, "interaction": 0 },
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

Each annotation entry contains `id`, `x`, `y`, and `visible`. Native scene
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
    { "id": "left-label", "x": 42.5, "y": 40.0, "visible": true }
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

`provenance` is the loaded asset's `AssetProvenance` value. `geometry` is the
loaded asset's `scena.asset_geometry_summary.v1` report and carries the same
provenance value. Warnings are typed and currently include
`external_image_missing` and `external_buffer_missing`. Cache-hit reports
preserve warnings and external resource counts from the original load, while
`fetched_bytes` remains `0` for the cache-hit call itself.

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
  ]
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
