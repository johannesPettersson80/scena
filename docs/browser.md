# Browser and WASM

`scena` supports browser workflows through Rust/WASM and explicit browser
backends.

Use browser support when your application needs:

- WebGPU canvas rendering,
- WebGL2 compatibility rendering,
- shared Rust scene logic between native and web,
- browser-hosted model viewers,
- web-based inspection tools.

## Starting point

Use the browser example:

```bash
cargo run --example browser_canvas
```

For controls in browser-hosted viewers, see:

```bash
cargo run --example orbit_controls_browser_adapter
```

## SceneHost

The `scene-host` feature exposes a generic WASM `SceneHost` facade over the
same `Scene`, `Assets`, and `Renderer` types used natively. It is for hosts that
build scene trees in the browser, push per-frame pose updates from external
data, and then request prepare/render/inspection explicitly.

`SceneHost` is push-driven. It does not own a `requestAnimationFrame` loop:
the embedding page creates or updates nodes, calls `setTransform` or
`setTransforms`, then calls `prepare()` and `render()` at the cadence it owns.
Camera controls may still update camera state when the host wires them, but
scene state is not advanced by an internal loop.

The browser constructor variants are:

```js
import init, { SceneHost } from "./pkg/scena.js";

await init();
const host = await SceneHost.newWebgl2(canvas, width, height, devicePixelRatio);
```

Use `newWebgpu` for WebGPU. `attachCanvasWebgl2`,
`attachCanvasWebgpu`, and `resize(width, height, dpr)` forward surface and DPR
changes through the same renderer lifecycle documented in
[Lifecycle](lifecycle.md).

Construction and lookup stay domain-neutral:

- `addEmpty(parent, translation, rotation, scale, tag)`
- `instantiateGlb(bytes)` and `instantiateGlbUnder(parent, bytes)`
- `instantiateUrl(url)` and `instantiateUrlUnder(parent, url)`
- `instantiateUrlWithReportJson(url)` and
  `instantiateUrlUnderWithReportJson(parent, url)`
- `importRoots(importHandle)`
- `nodeHandle(importHandle, path)`
- `nodeHandleByName(importHandle, name)`
- `setTag`, `clearTag`, and `findByTag`

Frame operations use one host handle namespace:

- `setTransform`
- `setTransforms` with a JSON array of `{ node, translation, rotation, scale }`
- `setNodeTint`
- `clearNodeTint`
- `setNodeAnnotation`
- `setWorldAnnotation`
- `clearAnnotation`
- `removeNode`
- `removeImport`
- `frameNode`
- `frameAll`
- `worldDistance`
- `nodeWorldBoundsJson`
- `pick(x, y)`
- `inspectJson`
- `annotationProjectionsJson`
- `capture()` and `captureJson()`

`pick(x, y)` receives CSS pixels. `SceneHost` stores the current DPR and target
size, applies DPR internally, and returns the same node handle namespace that
`setTransform` and `inspectJson` use.

`removeNode(handle)` recursively removes the node subtree and invalidates every
host handle in that subtree. `removeImport(importHandle)` removes all import
roots, marks the import handle stale, and invalidates the removed node handles.
Subsequent calls with those handles return structured stale-handle errors.

`setNodeTint(handle, r, g, b, a)` applies generic per-node highlight/tint
state without cloning materials. `clearNodeTint(handle)` removes it. The same
state appears in `inspectJson()` as `nodes[].tint` so hosts can prove that a
highlighted node was highlighted in the submitted scene state.

`setNodeAnnotation(id, handle, localOffset)` anchors an overlay point to a
scene node; `setWorldAnnotation(id, position)` anchors one to a world-space
point. `annotationProjectionsJson()` returns schema
`scena.annotation_projection.v1` with `{ id, x, y, visible }` entries in CSS
pixels, so the page can position HTML overlays without reimplementing camera
projection. `clearAnnotation(id)` removes an overlay anchor.

`worldDistance(a, b)` returns the world-space distance between two host node
handles. `nodeWorldBoundsJson(handle)` serializes the node subtree's
world-space bounds, resolving asset-backed mesh and instance geometry through
the host asset store.

`instantiateUrlWithReportJson()` and `instantiateUrlUnderWithReportJson()`
return schema `scena.scene_host_asset_import.v1`: the new import handle plus
the nested `scena.asset_load_report.v1` report. That report includes
node/mesh/primitive counts, asset bounds, fetched byte counts, cache-hit state,
external resource counts, progress events, and typed missing-resource warnings.
Cache-hit reports preserve warnings from the original load.

`capture()` returns a JS object with `descriptorJson` and an `rgba8`
`Uint8Array`. `captureJson()` returns only the descriptor JSON. The descriptor
uses schema `scena.capture.v1` and records dimensions, payload length/hash,
rendered scene revision counters, active camera transform/projection,
viewport/DPR, backend capabilities, auto-frame metadata when available, and
pixel summary. If the embedder mutates scene state after `render()` and before
`capture()`, the host returns a structured capture error instead of serializing
stale proof metadata. CPU-headless captures are deterministic for the same
scene state. Browser GPU captures bind pixels to revision counters and
backend/capability metadata; they do not claim cross-machine byte identity.

## Output Color Space

Browser renderers default to sRGB output. To request wide-gamut presentation,
build the renderer with
`RendererOptions::with_output_color_space(OutputColorSpace::DisplayP3)`.
Scena only reports `Capabilities::wide_gamut_output = Supported` after the
renderer-owned canvas path configures the active browser surface as Display P3:
WebGL2 uses `drawingBufferColorSpace`, and WebGPU uses
`GPUCanvasConfiguration.colorSpace`. Headless, unattached, or unproven browser
surfaces remain sRGB/degraded and emit
`DiagnosticCode::WideGamutOutputUnavailable`.

The M6 browser proof records this under
`scenaM6DisplayP3OutputProbe` in
`target/gate-artifacts/m6-rust-wasm-renderer-probe.json`; both WebGL2 and
WebGPU proof rows must show effective `display-p3` and nonblack rendered
pixels before Display P3 is treated as shipped.

## Custom Element Foundation

The `viewer-element` feature exports a browser registration function for the
drop-in element surface:

```html
<scena-viewer
  src="machine.glb"
  environment="studio"
  tone-mapping="neutral"
  camera-controls
  auto-rotate>
</scena-viewer>
```

```js
import init, { defineScenaViewer } from "./pkg/scena.js";

await init();
defineScenaViewer();
```

The first shipped foundation registers `<scena-viewer>`, creates a shadow DOM
canvas, exposes model-viewer-style attributes, and dispatches structured
attribute events. The browser proof is part of the M6 probe package; run
`SCENA_BROWSER_VIEWER_ELEMENT_ONLY=1 node tests/browser/m6_rust_wasm_renderer_probe.js`
after building `target/m6-browser-pkg` with `--features browser-probe` to
generate `target/gate-artifacts/scena-viewer-element-browser-proof.png` and
`target/gate-artifacts/scena-viewer-model-viewer-parity-browser-proof.png`.
The parity proof uses the dev-only `@google/model-viewer` package locally and
captures a three-asset side-by-side screenshot of `<model-viewer>` reference
panes next to renderer-backed `<scena-viewer>` panes for the same glTF / GLB
assets.

The element also owns a shadow DOM progressbar. Hosts can
dispatch a `scena-viewer-progress` event or call `setLoadProgress(detail)` with
`phase`, `ariaText`, and optional `value` / `ratio` / `percent`; the element
updates the visible status text, ARIA progress state, and emits
`scena-viewer-progress-rendered` after the UI changes. The browser proof
records a `progress_sequence` by dispatching `loading` and `fetching` phases
and asserting that the ARIA value and progressbar transform update between
events.

The element handles browser drag-and-drop ingestion for `.glb` and `.gltf`
files. Valid drops emit `scena-viewer-file-drop` with the accepted `File`
objects and names. Invalid or mixed drops emit `scena-viewer-drop-error` with
rejected names and a user-facing message. The custom element owns validation
and browser events, and the M6 browser proof now renders the accepted dropped
GLB bytes into the element canvas through the renderer-owned
`scena-viewer-drop-render` proof path. This is the render-after-drop
contract for the custom element. The same proof records
`viewer-level-auto-framing` metadata from the rendered asset's projected
bounds, including viewport containment, centering, and fill fraction, so the
browser path proves the model is visible without host-side `frame_bounds()`
calls.

Hosts can expose material variants through the built-in material variant picker
with `setMaterialVariants(variants, activeName)`. The picker accepts string
names or `{ name, label }` objects, emits `scena-viewer-variants-ready` after
population, and emits `scena-viewer-variant-change` with the selected variant
name or `null` for the default material. The M6 browser proof includes a
picker-to-rendered-variant path: the selected `noon` variant from
`material_variants_scene.gltf` is rendered into the element canvas under
`scena-viewer-material-variant-render`, and the proof asserts visible
green-dominant pixels from the selected material.

The mobile and accessibility defaults are part of the element contract. The host is
keyboard focusable by default, the canvas uses `touch-action: none`, the element
sets role and ARIA labels when the host has not supplied them, and keyboard
navigation emits `scena-viewer-key-control` for arrow-key orbit, `+` / `-` zoom,
and `Escape` / `Home` reset events. With `camera-controls`, pointer and wheel
input emits `scena-viewer-gesture-control` with `orbit`, `pinch-zoom`, and
`wheel-zoom` actions so mobile hosts can wire touch gestures into the shared
Rust controls without browser-specific logic in application code. The M6
browser proof records the mobile viewport, overflow check, touch-action
default, keyboard reset, and touch pinch/orbit gestures in
`scena.scena_viewer_mobile_a11y_browser_proof.v1`.

The inspector/dev overlay is host-fed and renderer-neutral. Call
`setInspectorSnapshot({ overlay, diagnostics, stats })` or
`setInspectorDiagnostics(diagnostics, overlay, stats)` to render a shadow-DOM
overlay with the active debug overlay, diagnostic severities, render counters,
and the `scena-viewer-inspector-rendered` event for browser tests. The M6
custom-element proof loads
`/fixtures/viewer/inspector_snapshot.json`, asserts
`scena.scena_viewer_inspector_snapshot.v1`, feeds that JSON through the live
overlay, and captures it in
`target/gate-artifacts/scena-viewer-element-browser-proof.png`.

The annotation overlay uses slotted HTML with `slot="annotation"` and
`data-position`, optional `data-normal`, and optional `data-surface`
attributes. The element emits `scena-viewer-annotations-request` with parsed
anchors and accepts `setAnnotationProjections([{ id, x, y, visible }])`,
then emits `scena-viewer-annotations-rendered` after applying the screen-space
positions. The browser proof records an `annotation_tracking_sequence` by
applying two projection updates to the same slotted label and asserting that
the CSS transform changes while the annotation remains visible.

The same M6 browser probe also records camera-control-kit proof for the shared
Rust control APIs. `scena.m6.camera_control_kit_browser_proof.v1` runs browser
input through `OrbitControls`, applies `FollowControls` and `FlyControls`, and
writes `target/gate-artifacts/camera-control-kit-browser-proof.png`. This proves
the browser input-to-motion contract for the library controls; full
custom-element gesture recordings remain a later `<scena-viewer>` proof.

## Browser responsibilities

The browser host owns:

- HTML layout,
- canvas creation,
- event wiring,
- asset serving,
- requestAnimationFrame scheduling,
- user input routing.

`scena` owns:

- scene and asset state,
- renderer preparation,
- drawing,
- capabilities,
- diagnostics,
- surface events.

## Asset loading

Browser asset paths must be fetchable by the page. Serve `.gltf`, `.glb`,
external `.bin` files, and textures from URLs your application controls.

For glTF files with external buffers or images, keep the relative file layout
intact when deploying.

## WebGPU and WebGL2

Use capability reports instead of assuming a backend:

- WebGPU is the modern browser GPU path.
- WebGL2 is the compatibility path and renders through wgpu's WebGL backend.
- Browser support depends on browser version, OS, GPU, and security context.

Applications should expose clear fallback behavior when a requested backend is
unavailable.

Both browser backends use the shared Rust renderer lifecycle. `prepare()` builds
wgpu resources, and `render()` presents through the configured browser surface.
There is no separate raw WebGL2 render path.

## Surface events

Browser integrations should forward relevant events to the renderer:

- canvas resize,
- device-pixel-ratio changes,
- visibility changes,
- context loss,
- context restore.

After surface changes or recovery, call `prepare()` before rendering again.

See [Lifecycle](lifecycle.md).
