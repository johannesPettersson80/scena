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
| Unsupported GPU sample count | choose a supported anti-aliasing mode; the renderer rejects it during `prepare()` before any render-time fallback |
| WebGPU destruction still `Submitted` | yield to the browser event loop and poll again until `DevicePollStatus::Confirmed`; do not treat submission as completion |
| WebGL2 destruction reports `Automatic` | wgpu retired its logical queue records using the GL lifetime model; do not present this as physical GPU-completion confirmation |
| Scene changed after prepare | call `prepare()` again |
| Surface resized | forward the surface event, then prepare again |
| Missing asset file | fix path or fetcher configuration |
| Missing external glTF buffer or image | inspect `scena.asset_load_report.v1` warnings; serve the referenced resource or enable strict external-resource loading to fail closed |
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
