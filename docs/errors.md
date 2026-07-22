# Errors and diagnostics

`scena` uses structured errors so applications can recover predictably and show
useful messages.

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

## Pattern matching

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
| Synchronous capture on WebGPU | await `captureAsync()`, `capturePngAsync()`, `captureJsonAsync()`, or `renderIntrospectionJsonAsync()` so GPU-buffer mapping can complete |
| Capture stale render | render again after mutating the scene or active camera |
| Capture auto-frame projection failure | frame the active camera to the bounds, use valid bounds, or capture without auto-frame metadata |
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
