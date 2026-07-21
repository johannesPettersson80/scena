# Feature flags

`scena` keeps optional integrations behind Cargo features.

Add features with Cargo:

```bash
cargo add scena --features controls,controls-winit
```

The `cargo add` form resolves the current compatible release and updates an
existing dependency entry without pinning living documentation to an old
package version.

## Features

| Feature | Purpose |
|---|---|
| `agent` | complete opt-in self-verification surface; enables `scene-host`, which already enables `inspection` |
| `controls` | compatibility marker; platform-neutral orbit, pan, zoom, and focus controls are always compiled |
| `controls-winit` | compatibility alias enabling `controls`; no `winit` dependency or hidden event loop is added |
| `controls-web` | compatibility alias enabling `controls`; browser hosts translate DOM events explicitly |
| `browser-probe` | browser/WASM rendered-output probe entry points; includes `viewer-element` so the browser proof package also verifies `<scena-viewer>` |
| `demo-page` | browser demo page WASM exports |
| `proof-harness` | proof-only demo controls and capture exports; enables `demo-page` |
| `viewer-element` | `<scena-viewer>` custom-element registration surface |
| `hot-reload` | native debounced asset-file watching for explicit reload loops |
| `inspection` | scene inspection metadata and diagnostic output |
| `scene-host` | generic native-testable and browser/WASM `SceneHost` facade over `Scene`, `Assets`, and `Renderer`; enables `inspection` |
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

The `agent` composition is the public one-step choice for recipe authoring,
inspection, rendering, and verification loops. It intentionally names only
`scene-host`; listing `inspection` again would misrepresent the actual feature
graph.

## Recommended combinations

Complete agent/self-verification surface:

```bash
cargo add scena --features agent
```

Native viewer:

```bash
cargo add scena --features controls,controls-winit
```

Browser viewer:

```bash
cargo add scena --features controls,controls-web
```

Asset-heavy viewer:

```bash
cargo add scena --features production-assets
```

Add `obj` separately when the application needs OBJ import in addition to
production glTF compression support.

Sample-driven examples/tests:

```bash
cargo add scena --features khronos-samples
```

Diagnostic tooling:

```bash
cargo add scena --features inspection
```

Browser host facade:

```bash
cargo add scena --features scene-host
```

Browser host with controls:

```bash
cargo add scena --features scene-host,controls-web
```

## Default feature set

The default feature set is exactly empty. `agent` is opt-in and aliases only
existing code, so it adds no dependencies or package files beyond selecting
the already documented `scene-host` -> `inspection` graph. Add only the
integrations your application needs.

PNG, JPEG, and WebP decoding is available without an opt-in feature because
these are the baseline native image paths. KTX2/Basis remains optional behind
`ktx2` (or the grouped `production-assets` profile), as does meshopt decoding.

The machine-readable ownership registry at
[`docs/specs/feature-ownership.json`](specs/feature-ownership.json) maps every
non-default Cargo feature to its owner module, implementation call site,
focused test, and documentation. `xtask doctor --full` rejects unmapped or
unproven features. ICC conversion is not advertised: the former dependency-only
flag was removed because it had no conversion call site, output metadata, or
rendered proof. A future ICC feature must first define an `Assets`-owned,
native/WASM-capable contract with those proofs.
