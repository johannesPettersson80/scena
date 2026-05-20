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
generate `target/gate-artifacts/scena-viewer-element-browser-proof.png`.

The element also owns a shadow DOM progressbar. Hosts can
dispatch a `scena-viewer-progress` event or call `setLoadProgress(detail)` with
`phase`, `ariaText`, and optional `value` / `ratio` / `percent`; the element
updates the visible status text, ARIA progress state, and emits
`scena-viewer-progress-rendered` after the UI changes. The browser proof
records a `progress_sequence` by dispatching `loading` and `fetching` phases
and asserting that the ARIA value and progressbar transform update between
events. Asset loading and rendering remain explicit host responsibilities until
full renderer parity lands under the `<scena-viewer>` roadmap item.

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
