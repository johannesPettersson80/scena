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
- WebGL2 is the compatibility path.
- Browser support depends on browser version, OS, GPU, and security context.

Applications should expose clear fallback behavior when a requested backend is
unavailable.

## Surface events

Browser integrations should forward relevant events to the renderer:

- canvas resize,
- device-pixel-ratio changes,
- visibility changes,
- context loss,
- context restore.

After surface changes or recovery, call `prepare()` before rendering again.

See [Lifecycle](lifecycle.md).
