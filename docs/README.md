# scena documentation

`scena` is a Rust-native 3D scene-graph renderer for glTF/GLB model viewers,
CAD-style inspection, industrial visualization, browser/native applications,
and deterministic headless rendering.

## Start here

- [README](../README.md): install, quick start, features, examples, and platform support.
- [docs.rs API reference](https://docs.rs/scena/latest/scena/): generated Rust API docs.
- [Getting started](getting-started.md): install, first scene, GLB loading, and output paths.
- [Easy scene setup](guides/easy-scene-setup.md): frame models, add studio lighting,
  add a matte grid floor, enable auto exposure, and connect authored anchors.
- [API overview](api.md): the main public types and how they fit together.
- [Renderer charter](RFC-rust-3d-renderer.md): canonical scope, non-goals, and architecture vocabulary.
- [Stable JSON contracts](schema-contracts.md): schema names, compatibility, handle, and fixture policy.
- [v1.5.0 release notes](release-notes/v1.5.0.md): expanded material presets, WebGL2 texture clamping, and smooth-metal browser IBL improvements.
- [v1.4.0 release notes](release-notes/v1.4.0.md): easy-use named primitives, bundled content, viewer ergonomics, `<scena-viewer>` element, and renderer-feature coverage.
- [v1.3.0 release notes](release-notes/v1.3.0.md): easy scene setup API notes and required proof.

## Core documentation

- [Rendering](rendering.md)
- [Lifecycle](lifecycle.md)
- [Assets](assets.md)
- [Platforms](platforms.md)
- [Browser and WASM](browser.md)
- [Headless rendering](headless-rendering.md)
- [Capabilities](capabilities.md)
- [Errors and diagnostics](errors.md)
- [Feature flags](feature-flags.md)
- [Examples](examples.md)
- [Troubleshooting](troubleshooting.md)

## Decisions and checklists

- [ADR-0002: eliminate hand-written rendering paths](decisions/ADR-0002-eliminate-handmade-rendering.md)
- [Eliminate hand-written rendering checklist](checklists/eliminate-handmade-rendering.md)
- [WASM scene host and stable contract checklist](checklists/wasm-scene-host-and-stable-contracts.md)
- [Browser renderer-fidelity dependency checklist](checklists/renderer-fidelity-dependencies.md)

## Guides

- [Easy scene setup](guides/easy-scene-setup.md)
- [Migrating from Three.js](guides/migrating-from-threejs.md)
- [Place and connect objects](guides/place-and-connect-objects.md)
- [Units, axes, and handedness](guides/units-axes-handedness.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)
- [Troubleshooting misplaced assets](guides/troubleshooting-misplaced-assets.md)

## Examples

The `examples/` directory contains runnable Rust examples for first render,
primitive shapes, GLB model viewing, animation, picking, controls, instancing,
labels/helpers, browser canvas setup, native windows, and headless CI output.

Run one:

```bash
cargo run --example glb_model_viewer
```

Compile all public examples:

```bash
cargo check --examples
```
