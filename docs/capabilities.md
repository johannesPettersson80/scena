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

## Backend selection is separate evidence

Strict constructors (`Renderer::headless_gpu`, `SceneHostCore::headless_gpu`,
`SceneHostCore::headless_gpu_with_fetcher`, and viewer
`with_headless_gpu`) either return a GPU-backed object or a structured build
error. They never return a CPU object under a GPU-sounding success result.

Fallback is opt-in through `SceneHostCore::headless_prefer_gpu`,
`headless_prefer_gpu_with_fetcher`, or viewer `with_headless_prefer_gpu`.
Those paths return/expose `HeadlessBackendSelectionReport`, which records the
requested backend, selected backend, whether fallback occurred, and the
original `BuildError`. The selected renderer capability report still describes
only the backend that was actually constructed; keep the selection report
beside it when request/fallback provenance matters.

Release and visual-proof lanes must use strict construction and assert the
selected backend/capability row. A preferred-GPU result is application behavior,
not GPU proof, even when its CPU output is otherwise valid.

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
- Shadow support. Directional shadows are supported only on GPU-device backends
  with visible receiver-darkening proof; CPU/reference and unattached factory
  capability rows report degraded.
- Environment lighting support.
- Material feature support such as clearcoat, sheen, anisotropy, iridescence,
  and dispersion factor handling or texture sampling in CPU/reference and GPU
  shader paths versus backend-gated release proof.
- Physical glass transmission. Attached GPU-device native/WebGPU/WebGL2 rows
  can report `supported` after scene-color transmission, IOR/thickness
  refraction, roughness-blur, and transparency-ordering proof; CPU/reference
  and unattached factory rows report `degraded`.
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

Browser `SceneHost` proof must capture capability JSON beside inspection and
capture JSON. A CPU builder can prove the schema and headless behavior, but it
cannot approve browser-visible rendering claims. WebGPU/WebGL2 claims require a
real browser/GPU run that records the active backend, DPR, rendered pixels, and
capability report for the same scene state.

## Best practice

At startup:

1. Create the renderer.
2. Read capabilities.
3. Select optional features.
4. Prepare the scene.
5. Render.

When a requested capability is unavailable, show a clear message and choose a
known, explicitly named fallback path. Do not infer constructor intent only
from the selected capability row; retain the backend selection report when a
preferred path was used.
