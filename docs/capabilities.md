# Capabilities

Capabilities describe what the active renderer path can do on the current
platform.

Use capabilities to decide:

- which backend path is active,
- whether a feature is supported,
- whether a fallback is in use,
- which optional effects to enable,
- what diagnostics to show to users.

## Why capability reports matter

Platform names are not enough. Two machines with the same OS can expose
different GPU adapters, driver limits, browser support, and texture limits.

Query the renderer instead of hard-coding assumptions.

## Capability states

Capabilities are structured so applications can distinguish:

- supported features,
- degraded features,
- disabled features,
- unsupported features,
- unavailable backend resources.

This lets applications present clear UI instead of failing silently.

## Common capability-dependent areas

- WebGPU versus WebGL2.
- Native GPU versus headless software rendering.
- Texture arrays and texture limits.
- Shadow support.
- Environment lighting support.
- Surface and context-loss behavior.
- Readback support.
- WASM/browser support.

## Adapter reports

GPU adapter reports identify backend, adapter name, limits, and related
metadata where available. Use this for diagnostics and bug reports.

## Best practice

At startup:

1. Create the renderer.
2. Read capabilities.
3. Select optional features.
4. Prepare the scene.
5. Render.

When a requested capability is unavailable, show a clear message and choose a
known fallback path.
