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
- GPU post-processing as renderer-owned presentation quality controls,
  including FXAA, bloom, and screen-space ambient occlusion where backend
  capabilities allow them.
- GPU instancing for repeated renderable imports without duplicating shared
  geometry, while keeping host-owned instance handles for transforms,
  visibility, tinting, picking, and inspection.
- World-space technical stroke rendering for CAD/viewer overlays and grids.
- SceneHost animation playback exposure for imported clips, driven by an
  explicit host clock rather than a hidden render loop.
- Presentation transitions for transforms and tint fades. These are renderer
  smoothing for visual state pushed by the host application; they are not
  simulation, robotics, PLC/process logic, physics, or gameplay behavior.
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

### One active implementation backlog

- Full-repo correctness, agent-contract, proof, performance, and documentation
  remediation:
  `docs/checklists/full-repo-review-v1.8.0-remediation.md`.

### Historical evidence tracks

The following completed or archived plans retain acceptance evidence; their
status tags are historical records, not current prioritization:

- Easy-use and renderer-quality evidence log:
  `docs/checklists/next-release-easy-use-and-state-of-the-art.md`.
- WASM scene host and stable JSON contracts:
  `docs/checklists/wasm-scene-host-and-stable-contracts.md`.
- Browser renderer-fidelity dependencies:
  `docs/checklists/renderer-fidelity-dependencies.md`.
- Stable JSON schema policy:
  `docs/schema-contracts.md`.

## Recipe document boundary

`SceneRecipeV1` is a versioned interchange/build input. It can be stored or
transmitted as a v1 authoring snapshot, but the host owns application persistence,
migrations, domain state, undo, and history. The contract makes
no cross-version lossless round-trip guarantee and does not preserve arbitrary
unknown extension data. Recipe IDs are stable within the recipe/build workflow;
they are not runtime handles or application-persistence identities.
