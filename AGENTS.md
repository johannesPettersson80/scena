# AGENTS

Repo-specific instructions for agents working on `scena`.

## Mission

`scena` is a Rust-native Three.js replacement for scene-graph, glTF, model-viewer,
industrial visualization, CAD/viewer, and digital-twin style applications. It is not a
simulation engine, not a robotics engine, not physics, not PLC/domain logic, and not a game
engine.

The canonical charter is `docs/RFC-rust-3d-renderer.md`. Start architectural, API, scope, or
milestone work from that RFC.

## Required Skills

- Use `scena-rfc-governance` when editing the RFC, changing v1.0/v1.x scope, changing
  milestones, or reviewing whether a feature belongs in the renderer.
- Use `scena-renderer-architecture` when implementing or refactoring scene/assets/render
  module ownership, typed handles, prepare/render lifecycle, resource lifetime, or public
  API.
- Use `scena-renderer-quality` when adding tests, visual proof, browser/WASM checks,
  headless screenshots, leak tests, allocation tests, or capability gates.
- Use `scena-gltf-assets` when touching glTF/GLB loading, extensions, animation, skinning,
  morph targets, anchors, units, or hot reload.

## Architecture Rules

- Keep renderer logic out of `platform`; platform modules are adapters only.
- Keep asset fetching/parsing/cache ownership in `assets`; `Renderer` must not fetch assets.
- Keep scene graph state in `scene`; `Renderer` consumes prepared scene/resource state.
- Do not add simulation, robotics, PLC, process, or physics concepts to `scena`.
- Prefer typed handles and structured errors over stringly contracts or silent fallbacks.
- Do not hide async fetches, shader compilation, or GPU upload inside `render()`.

## Validation

For any code change, run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

For browser, WebGPU/WebGL2, visual, or 3D rendering changes, add rendered-output proof.
Prefer Playwright or a deterministic headless harness. Do not declare a visual fix from
unit tests alone.

## Subagents

Claude Code subagents live in `.claude/agents/`. Routing guidance lives in
`docs/agents/subagents.md`.
