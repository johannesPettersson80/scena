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
- [v1.9.0 release notes](release-notes/v1.9.0.md): correctness, portability, agent workflows, proof quality, and interactive performance.
- [v1.9.1 draft release notes](release-notes/v1.9.1.md): subject-driven camera-behavior rendering, photo reports, demo-hero recipe migration, and compatibility notes.
- [v1.8.0 release notes](release-notes/v1.8.0.md): deterministic authoring workflows, renderer correctness, cross-backend GPU proof, and enforceable release evidence.
- [v1.7.2 release notes](release-notes/v1.7.2.md): chrome showcase reflections, recipe tessellation validation, and CI proof hardening.
- [v1.7.1 release notes](release-notes/v1.7.1.md): explicit WaterBottle CPU release proof lane.
- [v1.7.0 release notes](release-notes/v1.7.0.md): post-processing, instanced SceneHost imports, strokes, animation playback, and presentation transitions.
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

### Draft RFCs

- [Subject-driven photographic rendering RFC](RFC-subject-driven-photographic-rendering.md)
- [Subject-driven photographic rendering checklist](checklists/subject-driven-photo-rendering.md)

### Active open backlog

- [v1.9.0 full-repo review remediation checklist](checklists/full-repo-review-v1.9.0-remediation.md)
- [Deferred pre-render lint profile design](specs/lint-profile-v1.md)

### Historical evidence

- [v1.8.0 full-repo review remediation checklist](checklists/full-repo-review-v1.8.0-remediation.md)
- [ADR-0002: eliminate hand-written rendering paths](decisions/ADR-0002-eliminate-handmade-rendering.md)
- [Eliminate hand-written rendering checklist](checklists/eliminate-handmade-rendering.md)
- [WASM scene host and stable contract checklist](checklists/wasm-scene-host-and-stable-contracts.md)
- [Browser renderer-fidelity dependency checklist](checklists/renderer-fidelity-dependencies.md)
- [Application builder roadmap](checklists/application-builder-roadmap.md)

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
cargo check --examples --all-features
```
