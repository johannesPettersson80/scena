# scena documentation

`scena` is a Rust-native 3D scene-graph renderer for glTF/GLB model viewers,
CAD-style inspection, industrial visualization, browser/native applications,
and deterministic headless rendering.

## Start here

- [README](../README.md): install, quick start, features, examples, and platform support.
- [docs.rs API reference](https://docs.rs/scena/1.0.2/scena/): generated Rust API docs.
- [Getting started](getting-started.md): install, first scene, GLB loading, and output paths.
- [API overview](api.md): the main public types and how they fit together.
- [v1.0.2 release notes](release-notes/v1.0.2.md): published release evidence and compatibility notes.

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

## Guides

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
