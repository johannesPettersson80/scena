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
attribute events. The element also owns a shadow DOM progressbar. Hosts can
dispatch a `scena-viewer-progress` event or call `setLoadProgress(detail)` with
`phase`, `ariaText`, and optional `value` / `ratio` / `percent`; the element
updates the visible status text, ARIA progress state, and emits
`scena-viewer-progress-rendered` after the UI changes. Asset loading, rendering,
drag/drop, and annotation overlays build on this surface in follow-up slices.

The element handles browser drag-and-drop ingestion for `.glb` and `.gltf`
files. Valid drops emit `scena-viewer-file-drop` with the accepted `File`
objects and names. Invalid or mixed drops emit `scena-viewer-drop-error` with
rejected names and a user-facing message. The custom element owns validation
and browser events; renderer loading and visual proof are still explicit
follow-up work.

Hosts can expose material variants through the built-in material variant picker
with `setMaterialVariants(variants, activeName)`. The picker accepts string
names or `{ name, label }` objects, emits `scena-viewer-variants-ready` after
population, and emits `scena-viewer-variant-change` with the selected variant
name or `null` for the default material.

The mobile and accessibility defaults are part of the element contract. The host is
keyboard focusable by default, the canvas uses `touch-action: none`, the element
sets role and ARIA labels when the host has not supplied them, and keyboard
navigation emits `scena-viewer-key-control` for arrow-key orbit, `+` / `-` zoom,
and `Escape` / `Home` reset events.

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
