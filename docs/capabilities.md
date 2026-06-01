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
- Material feature support such as clearcoat, sheen, anisotropy, iridescence,
  and dispersion factor handling or texture sampling in CPU/reference and GPU
  shader paths versus backend-gated release proof.
- Postprocessing support such as subtle bloom, headless CPU SSAO, and
  headless CPU weighted blended OIT.
- Wide-gamut output, which is only claimed when a browser canvas color-space
  probe proves Display P3 support for the active backend.
- Surface and context-loss behavior.
- Readback support.
- WASM/browser support.

For browser WebGL2, capability reports describe the active wgpu GL/WebGL
adapter path. They should not be treated as constants from a separate raw GL
renderer.

## Adapter reports

GPU adapter reports identify backend, adapter name, limits, and related
metadata where available. Use this for diagnostics and bug reports.

## Stable JSON

`CapabilityReport::to_schema_report()` returns the typed report, and
`CapabilityReport::to_schema_json()` returns the same data as versioned JSON:

```json
{
  "schema": "scena.capability_report.v1",
  "capabilities": {
    "backend": "headless",
    "color_target_format": "Rgba8UnormSrgb",
    "forward_pbr": "degraded"
  },
  "adapter": null,
  "diagnostics": [
    {
      "code": "forward_pbr_degraded",
      "severity": "warning",
      "message": "...",
      "help": "..."
    }
  ]
}
```

The typed schema struct is `CapabilityReportV1`, and the schema string is
available as `CAPABILITY_REPORT_SCHEMA_V1`. Enum values in this contract use
serde names such as `supported`, `degraded`, and `feature_disabled`, not Rust
`Debug` formatting.

## Best practice

At startup:

1. Create the renderer.
2. Read capabilities.
3. Select optional features.
4. Prepare the scene.
5. Render.

When a requested capability is unavailable, show a clear message and choose a
known fallback path.
