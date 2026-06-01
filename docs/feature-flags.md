# Feature flags

`scena` keeps optional integrations behind Cargo features.

Add features with Cargo:

```bash
cargo add scena --features controls,controls-winit
```

or in `Cargo.toml`:

```toml
[dependencies]
scena = { version = "1.4", features = ["controls", "controls-winit"] }
```

## Features

| Feature | Purpose |
|---|---|
| `controls` | platform-neutral orbit, pan, zoom, and focus controls |
| `controls-winit` | native-host controls adapter support |
| `controls-web` | browser-host controls adapter support |
| `browser-probe` | browser/WASM rendered-output probe entry points; includes `viewer-element` so the browser proof package also verifies `<scena-viewer>` |
| `demo-page` | browser demo page WASM exports |
| `viewer-element` | `<scena-viewer>` custom-element registration surface |
| `hot-reload` | native debounced asset-file watching for explicit reload loops |
| `inspection` | scene inspection metadata and diagnostic output |
| `scene-host` | generic native-testable and browser/WASM `SceneHost` facade over `Scene`, `Assets`, and `Renderer`; enables `inspection` |
| `icc` | ICC/color-management support through `lcms2` |
| `khronos-samples` | checked Khronos glTF sample-asset catalog and loader helpers |
| `ktx2` | KTX2/Basis texture descriptor and decode support for `KHR_texture_basisu` assets |
| `meshopt` | meshopt-compressed glTF buffer decoding support |
| `obj` | OBJ import path |
| `production-assets` | compressed glTF asset profile; enables `ktx2` + `meshopt` without changing defaults |

Stable JSON contracts are exported by their owner surfaces rather than a
separate feature. Capability, capture, asset-load, asset-geometry, and
provenance value contracts are available with the default crate APIs.
Inspection contracts require `inspection`; the browser/native host facade
requires `scene-host`, which enables `inspection`.

## Recommended combinations

Native viewer:

```toml
scena = { version = "1.4", features = ["controls", "controls-winit"] }
```

Browser viewer:

```toml
scena = { version = "1.4", features = ["controls", "controls-web"] }
```

Asset-heavy viewer:

```toml
scena = { version = "1.4", features = ["production-assets"] }
```

Add `obj` separately when the application needs OBJ import in addition to
production glTF compression support.

Sample-driven examples/tests:

```toml
scena = { version = "1.4", features = ["khronos-samples"] }
```

Diagnostic tooling:

```toml
scena = { version = "1.4", features = ["inspection"] }
```

Browser host facade:

```toml
scena = { version = "1.4", features = ["scene-host"] }
```

Browser host with controls:

```toml
scena = { version = "1.4", features = ["scene-host", "controls-web"] }
```

## Default feature set

The default feature set is intentionally small. Add only the integrations your
application needs.
