# Browser renderer-fidelity dependencies

Status: planned sibling track
Date: 2026-06-01
Scope: renderer-quality dependencies that must be closed before the WASM
`SceneHost` and stable JSON work can claim final browser visual proof.

This checklist tracks fidelity work surfaced by:

- `docs/checklists/trust-platform-digital-twin-webgl-investigation.md`
- `docs/checklists/trust-platform-1.4.0-verification.md`
- `docs/checklists/trust-platform-finding-2-webgl-materials.md`
- `docs/checklists/next-release-easy-use-and-state-of-the-art.md`

It is a sibling to
`docs/checklists/wasm-scene-host-and-stable-contracts.md`. The host/schema work
can proceed independently, but its final browser rendered-output approval is
blocked while the open items here remain unresolved or untriaged against current
`main`.

## Current-main status snapshot

- Oversized browser texture upload failure: **closed on current main by
  existing evidence**. v1.5.0 release notes record WebGL2-safe browser texture
  clamping, and `docs/checklists/trust-platform-1.4.0-verification.md` records
  focused unit and browser proof commands for the oversized-texture reproducer.
- Depth/prepass minimum and ineligible-stroke behavior: **partly closed in
  source, still needs dense-scene visual proof**. Current code has
  `DEPTH_PREPASS_MIN_PRIMITIVES = 1` and focused tests for single-primitives and
  ineligible stroke primitives, but dense imported-scene browser proof remains a
  separate gate.
- Missing external image diagnostics: **partly closed by existing diagnostics
  and strict texture loading, still incomplete for this track**. Existing docs
  and release notes record `AssetLoadWarning::ExternalImageMissing`,
  `with_strict_textures(true)`, and
  `material_textures_missing_decoded_pixels`. Missing external buffers, cache-hit
  warning/provenance retention, and stable JSON load reports remain open for the
  host/schema phases.
- Dense WebGL2 source-material proof: **open**. No current gate proves a dense
  imported source-material scene through browser WebGL2 with representative
  materials.
- Source-material path audit: **open** for the future generic `SceneHost` proof.
  Any proof using generated unlit materials must label itself as such and cannot
  satisfy a source-material visual proof.

## Required epics

### 1. Dense WebGL2 source-material regression

Owner modules: `assets`, `scene`, `render`, browser proof harness.

- [ ] Add or refresh a dense imported glTF/GLB fixture that exercises source
      material handles, external textures, normals, metallic/roughness factors
      or textures, camera framing, and lighting.
- [ ] Render the fixture through browser WebGL2 using the shared wgpu path.
- [ ] Capture browser output and assert non-background pixels plus
      material-specific color/contrast predicates.
- [ ] Record backend, capability report, texture warning counts, renderer stats,
      and screenshot metadata.
- [ ] Compare to a deterministic headless/reference output where practical, or
      document backend-specific tolerance.
- [ ] Keep `forward_pbr` degraded until this proof passes for the claimed
      backend/capability lane.

Proof commands:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test dense_webgl2_source_material --all-features'
wasm-pack build --dev --target web --out-dir target/m6-browser-pkg . --features browser-probe
SCENA_BROWSER_BACKENDS=webgl2 node tests/browser/m6_rust_wasm_renderer_probe.js
```

Use a real GPU/browser machine for final hardware-accelerated proof when the
failure is backend-specific.

### 2. Depth/prepass dense-scene robustness

Owner modules: `render/prepare`, `render/gpu`, visual proof harness.

- [ ] Preserve the current single-primitive and mixed ineligible-stroke unit
      tests.
- [ ] Add a dense imported-scene proof showing opaque triangles keep correct
      depth with no sentinel line.
- [ ] Add a mixed overlay proof showing technical line/wire/edge primitives do
      not disable or corrupt depth for unrelated opaque geometry.
- [ ] Record `depth_prepass_passes`, `depth_prepass_draws`, backend, and
      screenshot metadata in the proof artifact.
- [ ] Remove any downstream depth sentinel workaround only after this proof
      passes.

Proof commands:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test depth_prepass --all-features'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo run -p xtask -- doctor --full'
```

Browser-visible depth fixes also require rendered-output proof.

### 3. Browser external-asset and material trust

Owner modules: `assets`, `render/prepare`, diagnostics, browser proof harness.

- [ ] Keep `ExternalImageMissing` warnings visible through asset-load reports.
- [ ] Add typed missing external-buffer warnings.
- [ ] Preserve strict texture behavior for release proof.
- [ ] Preserve or clearly document warning/provenance behavior on cache hits.
- [ ] Ensure material texture handles without decoded pixels produce structured
      diagnostics/stats.
- [ ] In browser proof, record all externally fetched buffers/images and their
      warning/error status.
- [ ] Keep oversized browser image handling enforced against adapter limits.

Proof commands:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test external_image_missing --all-features'
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test material_textures_missing_decoded_pixels --all-features'
```

### 4. Source-material path audit

Owner modules: `assets/gltf`, `scene/import`, browser proof harness.

- [ ] Prove imported source materials are retained end to end when requested.
- [ ] Ensure any generated unlit/material-override path is explicit in metadata.
- [ ] Add a browser proof row that distinguishes source material, generated
      unlit material, and generated PBR override paths when the same geometry is
      used.
- [ ] Document which path satisfies source-material visual proof.

Proof commands:

```bash
ssh scena-builder 'cd "$HOME/projects/scena" && cargo test source_material --all-features'
SCENA_BROWSER_BACKENDS=webgl2 node tests/browser/m6_rust_wasm_renderer_probe.js
```

## Gate integration

- [ ] `docs/checklists/wasm-scene-host-and-stable-contracts.md` Phase 7 cannot
      be marked complete while this checklist has open or untriaged required
      epics.
- [ ] Any capability promotion from `Degraded` to `Supported` must cite the
      exact rendered-output proof artifact.
- [ ] Any waived fidelity item needs an ADR or release-note entry naming owner,
      expiry, affected backend, user-visible risk, and replacement evidence.

