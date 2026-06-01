# Rust-native 3D renderer charter

Status: canonical charter, owner-ratified 2026-06-01
Last updated: 2026-06-01

`scena` is a Rust-native scene-graph renderer for glTF/GLB model viewing,
industrial visualization, CAD/viewer workflows, browser/native applications,
and deterministic headless rendering. It is the proposed project-level source
of truth for architectural scope and public vocabulary until owner ratification
records it as canonical.

## Mission

`scena` provides renderer-owned scene state, asset loading, camera/light/material
state, explicit prepare/render lifecycles, diagnostics, capability reporting,
browser/native platform adapters, and proof artifacts.

The library is domain-neutral. Host applications own their domain model and map
visual state into `Scene`, `Assets`, and `Renderer`.

## Public vocabulary

Core public concepts stay coherent and small:

- `Scene`: scene graph state, transforms, cameras, lights, labels, imports,
  picking targets, and dirty/revision tracking.
- `Assets`: asset fetching, parsing, cache/deduplication, texture decoding,
  glTF/GLB scene assets, environments, and retain/reload policy.
- `Renderer`: backend resources, prepare/render lifecycle, surfaces, targets,
  stats, diagnostics, capabilities, and readback.
- `SceneImport`: import-local roots, names, paths, anchors, connectors, clips,
  bounds, and stale-import checks.
- `prepare()`: explicit validation, GPU/resource upload, batching, and
  capability-dependent setup.
- `render()`: drawing already-prepared state.

New APIs should extend this vocabulary rather than adding catch-all manager,
engine, world, or application-specific abstractions.

## In scope

- Scene graph mutation, typed handles, stable external handle views, transforms,
  bounds, cameras, lights, labels, annotations, picking, and framing.
- glTF/GLB import, external buffers/images, source units and coordinate-system
  conversion, anchors/connectors, material variants, animation clips, skinning,
  morph targets, and asset cache/reload behavior.
- PBR/material/rendering behavior, WebGPU/WebGL2/native/headless backends,
  prepare/render lifecycle, resource lifetime, readback, and capture metadata.
- Browser/WASM adapters that expose renderer primitives without owning the host
  application loop.
- Structured diagnostics, capability reports, inspection reports, capture
  descriptors, asset-load reports, provenance, release gates, doctor rules, and
  deterministic proof artifacts.

## Non-goals

`scena` is not a simulation engine, not a robotics engine, not a physics engine,
not a PLC/process-logic runtime, and not a game engine. It should not grow
domain-specific motion planning, rigid-body/collision simulation, control-loop
semantics, networking, gameplay ECS, audio, or application business logic.

## Design rules

- Keep renderer logic out of `platform`; platform modules are adapters only.
- Keep asset fetching, parsing, cache ownership, and reload policy in `assets`.
- Keep scene graph state in `scene`; `Renderer` consumes prepared scene/resource
  state.
- Prefer typed handles and structured errors over string contracts or silent
  fallbacks.
- Do not hide async fetches, shader compilation, or GPU upload inside
  `render()`.
- Add abstractions only when they remove real duplication or enforce a current
  contract.
- Claims about renderer quality or Three.js replacement scope must be backed by
  an API, diagnostic, test, rendered-output proof, or release artifact.

## Current execution tracks

- Easy-use and renderer-quality roadmap:
  `docs/checklists/next-release-easy-use-and-state-of-the-art.md`.
- WASM scene host and stable JSON contracts:
  `docs/checklists/wasm-scene-host-and-stable-contracts.md`.
- Browser renderer-fidelity dependencies:
  `docs/checklists/renderer-fidelity-dependencies.md`.
- Stable JSON schema policy:
  `docs/schema-contracts.md`.
