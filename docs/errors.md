# Errors and diagnostics

`scena` uses structured errors so applications can recover predictably and show
useful messages.

## CLI error and exit taxonomy

Machine-mode dispatch failures are one `scena.cli_error.v1` JSON document on
stderr. The document carries a stable `code`, `exit_class`, numeric
`exit_code`, `message`, optional `path`, command `context`, curated `help`,
structured `candidates`, and an optional machine-applicable `fix`. Runtime
failures are never mislabeled as `invalid_arguments`.

| Exit class | Status | Meaning |
|---|---:|---|
| `comparison` | 1 | a valid comparison found inequality |
| `usage` | 2 | unknown command or invalid arguments |
| `input` | 65 | missing, malformed, or unknown input contract |
| `unsupported` | 69 | required feature, capability, or backend unavailable |
| `runtime` | 70 | execution failed after valid dispatch |
| `internal` | 70 | serialization or invariant failure |
| `io` | 74 | output or filesystem I/O failure |
| `policy` | 77 | sandbox or operator policy rejected the request |
| `interrupted` | 130 | process interrupted or cancelled |

### `runtime` and `internal` share exit 70 — branch on the JSON, not `$?`

Two classes map to the same numeric exit code, so **exit status alone cannot
tell them apart**. A caller that only reads `$?` will see 70 for both.

Read the error document instead. `exit_class` and `code` are always distinct:

| Exit class | `code` | What it means for the caller |
|---|---|---|
| `runtime` | `runtime_error` | Your request was valid and dispatched; the operation itself failed. Inspect `message` and `help`, adjust the inputs or environment, and retry. |
| `internal` | `internal_error` | A scena invariant broke. Nothing you change will fix it — preserve the JSON document and file an issue. |

This is a deliberate, documented limitation rather than a defect: neither class
is fixable by changing arguments, so a shell-only consumer would take the same
action for both. Exit codes are part of the published contract and are pinned by
`tests/a01_cli_error_taxonomy.rs`; separating them would be a breaking change.

Both fields are set from a typed error kind, not inferred from the message text,
so rewording an error can never move it between these two classes.

## Camera-behavior and subject-observation failures

Camera-behavior acceptance failures are domain-result JSON, not CLI dispatch
errors. `scena photo render` writes `scena.photo_render_result.v1` on stdout
with `ok:false`, `failure_codes[]`, and a `scena.photo_report.v1` report path
when the command is valid but the image fails the camera-behavior gate. Recipe
verification uses the same stdout domain-failure pattern through
`scena.recipe_render_result.v1`. Use `exit_class` and `code` only for
`scena.cli_error.v1` dispatch/input/policy/runtime errors on stderr; do not
infer error taxonomy from prose.

Common subject and photo reason codes:

| Code | Meaning | Recovery |
|---|---|---|
| `subject_luminance_below_min` | the measured subject is too dark for the active product-quality band | keep auto exposure and apply `exposure_report.suggested_compensation_ev`, or let the bounded camera-behavior retry do it |
| `subject_low_clip_above_max` | too much of the subject is crushed near black | keep subject metering; improve lighting/materials or exposure compensation instead of hard-coding `exposure_ev` |
| `subject_fill_below_min` / `subject_too_small_in_frame` | the subject occupies too little of the intended frame | use `photo.intent` composition or adjust fill constraints |
| `subject_fill_above_max` | the subject is too large for the camera-behavior band | loosen fill or use a wider candidate constraint |
| `subject_center_offset_above_max` | the selected subject is off-center for the intent | check subject target and composition constraints |
| `subject_luminance_structure_below_min` | material/steel readability is too flat or reflection structure is missing | use a reflection-capable staging/environment or material preset |
| `subject_visible_pixels_missing` / `subject_visible_mask_empty` | the declared subject resolved but contributed no visible pixels | inspect visibility, target, clipping, occlusion, and semantic attribution |
| `subject_visible_mask_backend_unsupported` | the backend cannot provide exact subject-mask evidence | choose a supported backend or accept an explicit degraded result |
| `subject_transparent_unsupported` | exact subject identity cannot be proven for the transparent subject | use opaque/masked fallback geometry for strict evidence |
| `stale_subject_observation` | the subject-observation frame key no longer matches the rendered pixels | render again after the camera, viewport, scene, or render-output change |

`scena --help` is the machine-authoritative command table: each
`command_contracts[]` row declares its success/error schemas and applicable
`failure_exit_classes`. A closed stdout pipe is quiet success; another stdout
write failure emits `scena.cli_io_error.v1` with the same `io`/74 classification.

## Complete CLI process contract

`scena --help` is also the complete per-command process table; automation does
not need to inspect Rust source or join undocumented conventions. Every
`command_contracts[]` row contains:

- `emits.success[]` and `emits.error[]`: every possible top-level versioned
  result family for that command;
- `streams`: success and domain-failure JSON use stdout; CLI dispatch and runtime errors use stderr;
- `failure_exits[]`: the applicable typed class, numeric code, schema, and
  stream together in one row, including I/O 74;
- `feature_requirements[]`: the installable Cargo feature set, explicitly `[]`
  for core commands and `["agent"]` for the one-step application-builder
  surface.

The older `failure_exit_classes[]` and top-level `error_taxonomy[]` remain in
the v1 help envelope for compatibility. `failure_exits[]` is the direct form
new automation should consume. Domain-result envelopes can report a failed
validation or comparison on stdout; `scena.cli_error.v1` is reserved for
dispatch, input, unsupported, policy, runtime, internal, I/O, and interruption
errors on stderr.

## Error families

| Error | Typical cause |
|---|---|
| `BuildError` | renderer or platform construction failed |
| `AssetError` | asset loading, fetching, decoding, or extension handling failed |
| `ImportError` | imported scene data could not be interpreted |
| `InstantiateError` | asset instantiation into a scene failed |
| `LookupError` | a named node, path, anchor, connector, or handle lookup failed |
| `PrepareError` | renderer preparation failed |
| `RenderError` | rendering failed, often because prepared state is stale |
| `CaptureError` | capture descriptor/readback metadata failed, usually no rendered frame, stale rendered frame metadata, invalid RGBA length, invalid DPR, or auto-frame projection failure |
| `AnimationError` | clip, mixer, channel, skin, or morph target operation failed |
| `ConnectionError` | anchor or connector placement failed |
| `ColorParseError` | color parsing failed |
| `SceneHostError` | browser/native host facade operation failed; carries a stable `SceneHostErrorCode` |

The eight top-level renderer/asset error families expose uniform recovery
methods. Call `.help()` for curated guidance or `.diagnostic()` for an owned,
serializable `ErrorDiagnostic { code, message, help, context }`. Converting a
family into top-level `scena::Error` preserves and delegates the same remedy;
`Display` intentionally remains the concise failure message.

## Pattern matching

CLI JSON uses deterministic pretty formatting by default across success and
failure streams. Global `--compact` selects one-line JSON and `--pretty`
explicitly selects the default; the flags may appear anywhere, are mutually
exclusive, and do not alter schemas, exit codes, or field values.

Use Rust pattern matching for application logic:

```rust
match renderer.render_active(&scene) {
    Ok(frame) => frame,
    Err(err) => {
        eprintln!("{err}");
        return Err(err.into());
    }
}
```

Use richer matching when an application needs specific recovery behavior, such
as preparing again after stale renderer state.

## Common recoveries

| Problem | Recovery |
|---|---|
| Render called before prepare | call `prepare()` and render again |
| Unsupported GPU sample count | inspect `render_sample_counts` / `depth_sample_counts`; automatic browser quality uses a reported FXAA fallback, while an exact MSAA request is rejected during `prepare()` |
| WebGPU destruction still `Submitted` | yield to the browser event loop and poll again until `DevicePollStatus::Confirmed`; do not treat submission as completion |
| WebGL2 destruction reports `Automatic` | wgpu retired its logical queue records using the GL lifetime model; do not present this as physical GPU-completion confirmation |
| Scene changed after prepare | call `prepare()` again |
| `RenderError::NoActiveCamera` | call `Scene::add_default_camera` or `Scene::set_active_camera`; the same remedy survives `SceneHostError` and JSON serialization |
| Surface resized | forward the surface event, then prepare again |
| `RenderError::SurfaceLost` | recreate/reattach the surface with `recover_surface` (or the async browser attachment API), then prepare again; wgpu does not permit reviving a lost surface with `configure()` alone |
| `RenderError::SurfaceOutdated` | scena already refreshed configuration and retried once; wait for the next resize/monitor event or replace the surface |
| `RenderError::SurfaceConfigurationChanged` | call `prepare()` so surface pipelines match the refreshed format/present mode, then render again |
| surface timeout or occlusion | accept the `RenderOutcome::skipped` frame and inspect `surface_timeout_skips` / `surface_occluded_skips`; schedule another frame when appropriate |
| `RenderError::GpuValidation` | inspect the wgpu diagnostic and fix the renderer/surface contract; do not treat it as transient surface churn |
| `RenderError::GpuOutOfMemory` | release resources or lower GPU memory demand; rebuild if device loss follows |
| Recoverable context loss | keep CPU-side assets, forward context restoration, call `recover_context`, then prepare again |
| `PrepareError::GpuDeviceRebuildRequired` / `RenderError::GpuDeviceLost` | recreate `Renderer` and prepare the retained scene/assets; a lost wgpu Device/Queue cannot be cleared by `recover_context` |
| Missing asset file | fix path or fetcher configuration |
| Missing external glTF buffer or image | inspect `scena.asset_load_report.v1` warnings; serve the referenced resource or enable strict external-resource loading to fail closed |
| Missing glTF `NORMAL` | valid nondegenerate triangles load with flat normals and a `computed_flat_normals` report row; repair the named degenerate triangle or author normals when loading fails |
| glTF texture requests `texCoord` 1 or higher | export that texture against `TEXCOORD_0`; Scena rejects the material slot/path instead of silently sampling UV0 |
| glTF skin has more than four nonzero influences | inspect `skin_influences_truncated`, or author at most four influences per vertex for exact cross-tool parity; sets 0 and 1 are combined before selection |
| Selected glTF joint index exceeds the bound skin | correct `JOINTS_0/1` or the node's skin joint list; prepare reports the selected index and available binding width |
| Node morph-weight count differs from primitive targets | author one node weight per morph target on every primitive of the referenced mesh |
| Invalid glTF anchor/connector transform extras | repair the exact `nodes[n].extras.scena.anchors[n]` or `connectors[n]` path in `AssetError::Parse`; author finite nonzero scale, a normalized quaternion or paired nonparallel forward/up vectors, or a finite affine shear-free 16-value matrix |
| Scene reload dependency missing or malformed | inspect `AssetReloadError::error()` and `previous_asset_preserved()`, keep using the last complete cached scene, repair the named path/bytes, then call `reload_scene_with_report` again; explicit reload never publishes a partial dependency set |
| Unsupported required glTF extension | enable the relevant feature or choose an asset variant without that required extension |
| Missing named node or anchor | inspect imported names and paths |
| Removing the scene root | remove child nodes instead; the root is the permanent scene anchor |
| `LookupError::InvalidTransform` | provide only finite translation, rotation, and scale components; the rejected mutation is an atomic no-op |
| Stale host node handle | refresh the handle through import path, tag lookup, picking, or inspection |
| Stale host import handle | instantiate or load the asset again, then resolve fresh roots and node handles |
| Wrong SceneHost handle namespace | pass the handle back to the API family that created it; do not pass import, instance-root, or animation handles to node/import/animation-only APIs interchangeably |
| Browser backend unavailable | choose another backend or show a capability message |
| Capture invalid DPR | update the stored viewport/DPR before capture |
| Capture before render | call `prepare()` and `render()` before `capture()` |
| Synchronous browser GPU capture without an accessible completed readback | await `captureAsync()`, `capturePngAsync()`, `captureJsonAsync()`, or `renderIntrospectionJsonAsync()` so renderer-owned WebGPU/WebGL2 readback can complete and bind pixels to frame provenance |
| Capture stale render | render again after mutating the scene or active camera |
| Capture auto-frame projection failure | frame the active camera to the bounds, use valid bounds, or capture without auto-frame metadata |
| `AnimationError::InvalidClip` | use `AnimationSourceClip::try_new` / `try_rebind`; fix the named channel time, output shape, finite value, or duration before creating a mixer |
| `MissingLightingOrEnvironment` | on a high-level glTF viewer, inspect `setting` and `fallback_applied`; author a light/environment to replace the neutral fallback. On a low-level renderer, add lighting/environment explicitly |
| `InvisibleScene` | inspect camera/frustum, visibility, material opacity, and lighting diagnostics before accepting captured bytes as a successful visual result |

Named lookup failures expose a capped `candidates` array ordered by one shared,
case- and separator-normalized edit-distance algorithm. `LookupError` carries
candidates for imported nodes, animation clips, variants, anchors, and
connectors; `AnimationError::ClipNotFound` preserves the clip candidates rather
than collapsing them to prose. Recipe diagnostics add the same field for
geometry/mesh-resource, material, node, import, and environment-preset
references. `SceneHostError::candidates()` and serialized SceneHost errors keep
the structured list across host conversions. Treat the first candidate as a
suggestion, not an automatic mutation, unless the application has separately
proved that correction is unambiguous.

`SceneHostErrorCode::Capture` wraps capture descriptor/readback failures from
the browser/native host facade. The host still returns structured errors; it
does not silently drop capture metadata.

SceneHost handles carry an explicit node, import, instance-root, or animation
kind tag plus a per-kind slot generation. `SceneHostErrorCode::WrongHandleNamespace`
means a structurally valid handle was passed to a resolver for a different
kind. `NodeHandleNotFound`, `ImportHandleNotFound`, and
`AnimationHandleNotFound` mean that the requested kind is malformed or has no
allocated slot. `StaleNodeHandle`, `StaleImportHandle`, and
`StaleAnimationHandle` mean that the slot existed but the supplied generation
was removed or superseded. Instance roots use the node stale/not-found codes
because the documented node-target mutation APIs accept both ordinary nodes
and instance roots, but their encoded kind remains distinct.

Handle values are opaque: applications must not decode tags, slots, or
generations. All valid encodings stay within the exact JavaScript integer range
(`2^53 - 1`) for browser and JSON transport. A slot whose generation space is
exhausted is retired rather than issuing a previously used handle again.

## Diagnostics

Renderer diagnostics and capability reports are designed for user-facing error
messages and bug reports. Include them when reporting platform-specific issues.

Useful diagnostic information:

- backend,
- adapter name,
- active feature flags,
- asset path,
- glTF extension name,
- scene/import handle,
- renderer capability report,
- renderer stats.

`FirstRender::diagnostics()` and both high-level viewer `diagnostics()` methods
combine setup warnings, renderer diagnostics, and scene diagnosis. Applied
defaults are machine-readable: `setting` identifies the affected configuration
and `fallback_applied` distinguishes a recovered first render from an authored
presentation. Low-level `Renderer::render*` can still return valid bytes for an
intentionally black target; call `diagnose_scene_with_assets` when using that
explicit API directly. Verification and introspection commands may reject a
provably invisible image even when capture itself produced bytes.
