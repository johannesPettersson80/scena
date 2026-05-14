# Platforms

`scena` is designed for native, browser, WASM, and headless rendering
workflows. The same application-level model can drive different targets through
the same `Scene`, `Assets`, and `Renderer` concepts.

## Supported targets

| Target | Use case |
|---|---|
| Linux native/headless | CI rendering, server-side snapshots, native model viewers, Vulkan-capable hosts. |
| macOS Metal | Native model viewers and inspection tools on macOS. |
| Windows DX12 | Native model viewers and inspection tools on Windows. |
| Browser WebGPU | Modern browser rendering through Rust/WASM and WebGPU. |
| Browser WebGL2 | Compatibility browser rendering path. |
| `wasm32-unknown-unknown` | Web packaging and browser integration. |
| Headless CPU | Deterministic software-rendered output for tests, docs, and automation. |

## Native applications

Native hosts own their event loop and windowing integration. `scena` owns
renderer state and accepts surface events from the host.

Start with:

- `examples/native_window.rs`
- `examples/orbit_controls_native_adapter.rs`

## Browser applications

Browser hosts provide a canvas and drive rendering through WASM. WebGPU and
WebGL2 are represented as explicit backend choices so applications can report
or select capabilities.

Start with:

- `examples/browser_canvas.rs`
- `examples/orbit_controls_browser_adapter.rs`
- [Browser guide](browser.md)

## Headless rendering

Headless rendering is useful for:

- CI screenshots,
- generated documentation images,
- regression tests,
- server-side previews,
- deterministic visual checks.

Start with:

- `examples/headless_ci.rs`
- [Headless rendering](headless-rendering.md)

## Feature flags

Optional platform and asset features are controlled through Cargo features. See
[Feature flags](feature-flags.md).

## Capability reports

Use renderer capability reports instead of guessing by platform name. A browser
can support WebGPU or fall back to WebGL2; a native system can expose different
adapter limits depending on hardware and drivers.

See [Capabilities](capabilities.md).
