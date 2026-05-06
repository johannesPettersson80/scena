---
name: scena-rfc-governance
description: Use when editing, reviewing, or executing the scena RFC, including v1.0/v1.x scope, milestones, Three.js replacement claims, non-goals, and whether a feature belongs in the renderer.
---

# Scena RFC Governance

## Workflow

1. Read `docs/RFC-rust-3d-renderer.md` before changing scope or architecture.
2. Keep `scena` scoped to a renderer: scene graph, assets, rendering, authoring helpers,
   diagnostics, tests, native/WASM platform adapters.
3. Reject or move out simulation, robotics, PLC, process logic, physics, game engine ECS,
   or domain-specific behaviors.
4. When changing v1.0 scope, update milestones and acceptance criteria together.
5. Keep claims falsifiable: every "better than Three.js" claim needs an API, diagnostic, or
   test mechanism.

## Acceptance Checks

- Public vocabulary remains coherent: `Scene`, `Assets`, `Renderer`, `SceneImport`,
  `prepare()`, `render()`.
- Non-goals remain explicit.
- v1.0 and v1.x boundaries are consistent across the RFC.
- Any new feature has an owner module and validation surface.
