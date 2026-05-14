# Feature flags

`scena` keeps optional integrations behind Cargo features.

Add features with Cargo:

```bash
cargo add scena --features controls,controls-winit
```

or in `Cargo.toml`:

```toml
[dependencies]
scena = { version = "1.0", features = ["controls", "controls-winit"] }
```

## Features

| Feature | Purpose |
|---|---|
| `controls` | platform-neutral orbit, pan, zoom, and focus controls |
| `controls-winit` | native-host controls adapter support |
| `controls-web` | browser-host controls adapter support |
| `browser-probe` | browser/WASM rendered-output probe entry points |
| `inspection` | scene inspection metadata and diagnostic output |
| `icc` | ICC/color-management support through `lcms2` |
| `ktx2` | KTX2/Basis texture descriptor and decode support for `KHR_texture_basisu` assets |
| `meshopt` | meshopt-compressed glTF buffer decoding support |
| `obj` | OBJ import path |

## Recommended combinations

Native viewer:

```toml
scena = { version = "1.0", features = ["controls", "controls-winit"] }
```

Browser viewer:

```toml
scena = { version = "1.0", features = ["controls", "controls-web"] }
```

Asset-heavy viewer:

```toml
scena = { version = "1.0", features = ["ktx2", "meshopt", "obj"] }
```

Diagnostic tooling:

```toml
scena = { version = "1.0", features = ["inspection"] }
```

## Default feature set

The default feature set is intentionally small. Add only the integrations your
application needs.
