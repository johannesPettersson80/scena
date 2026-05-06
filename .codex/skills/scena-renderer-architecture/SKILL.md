---
name: scena-renderer-architecture
description: Use when implementing or refactoring scena renderer architecture, public API, module ownership, typed handles, resource lifetime, prepare/render lifecycle, surface/context recovery, or native/WASM platform boundaries.
---

# Scena Renderer Architecture

## Ownership Rules

- `scene`: scene graph, typed keys, transforms, bounds, anchors, clipping, queries.
- `assets`: fetchers, caches, glTF/GLB parsing, texture decoding, asset handles, retain policy.
- `geometry`: primitives, technical lines, helpers, labels metadata.
- `material`: material descriptors, texture slots, color space, alpha modes.
- `render`: wgpu device/surface, prepare lifecycle, pipelines, passes, stats, GPU resources.
- `animation`: glTF clips, mixer state, skinning, morph targets.
- `controls`: platform-neutral camera controls.
- `picking`: rays, acceleration, typed hit results.
- `diagnostics`: structured errors, debug overlays, capabilities.
- `platform`: thin winit/browser adapters only.

## Implementation Rules

- Do not hide asset fetch, shader compile, or GPU upload inside `render()`.
- Use typed handles and structured errors; avoid stringly runtime contracts.
- Keep renderer internals independent of domain simulation logic.
- Preserve native/WASM separation: platform adapters call into renderer logic, not the other
  way around.
- Add tests for public contract changes before broadening implementation.

## Required Checks

Run:

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```
