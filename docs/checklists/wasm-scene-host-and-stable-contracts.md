# WASM scene host and stable contract checklist

Status: archived historical evidence
Canonical active backlog:
`docs/checklists/full-repo-review-v1.9.0-remediation.md`
Date: 2026-06-01
Scope: expose generic `scena` scene/asset/render primitives to browser/WASM
hosts and ship stable JSON contracts for inspection, capability, capture, and
asset-load proof.

This checklist is for renderer integration work. It must stay domain-neutral:
no simulation, robotics, PLC, process-control, physics, or application-specific
concepts belong in public `scena` APIs, schema names, examples, or docs.

The immediate target is a host-driven browser/webview application that creates a
multi-part visual scene, pushes node transforms from external data, renders on
its own cadence, and proves what was rendered through versioned JSON reports.
The application domain stays outside `scena`; `scena` owns only generic scene
graph, asset, rendering, diagnostics, and proof surfaces.

Required sibling renderer-fidelity work: this checklist makes browser scenes
buildable, inspectable, and provable, but it does not by itself close the
existing dense WebGL2 source-material, depth/prepass, or external texture trust
findings. Those must be fixed as separate renderer-fidelity epics, and the
final browser visual proof from this host/schema work is blocked until each is
closed on current `main` or explicitly reclassified with current evidence; see
the trust-platform investigation docs named in Phase 0.

## Non-negotiable design rules

- [ ] No public no-op stubs. A public method must either work, return a
      structured unsupported error for a real unavailable backend/capability, or
      remain private/unshipped until implemented.
- [ ] One node handle namespace. The `SceneHost` node handle map is the
      authority, and inspection JSON emitted by a host must use the same `u64`
      handles that `set_transform`, `set_transforms`, `pick`, `remove_node`,
      and annotation projection APIs use.
- [ ] Stable wire JSON never serializes raw `NodeKey`, `CameraKey`,
      `MaterialHandle`, `GeometryHandle`, or other slotmap/internal handles.
      Internal reports may keep typed handles; external schema views must map
      them to stable host/report IDs.
- [ ] The browser host is push-driven. `SceneHost` must not own a
      requestAnimationFrame loop for scene state. The embedder pushes scene
      mutations, then calls `prepare()` and `render()`. Camera controls may
      tick only when the host explicitly advances them.
- [ ] `pick(x, y)` uses CSS pixels. `SceneHost` owns DPR and target-size
      conversion internally so picking, resize, and screenshots use one
      coordinate contract.
- [ ] Removed handles are generation-checked. A freed `u64` must return a
      structured stale-handle error and must never alias a recycled slot.
- [ ] Browser inspection is a first-class build target. `inspection` plus the
      host feature must compile for `wasm32-unknown-unknown`, and the browser
      proof must call `inspect_json()`. If wasm inspection exposes native-only
      dependencies, split that porting work into a named Phase 1 subtask rather
      than letting it silently expand the schema work.
- [x] Per-node tint/highlight inspection is decided before implementation.
      `scena.scene_inspection.v1` includes each node's optional `tint` so proof
      JSON can verify highlighted render state.

## Required sibling renderer-fidelity epics

These are not implemented by the host/schema checklist, but they are required
before any final browser visual proof can be claimed.

- [ ] Dense WebGL2 source-material proof:
      add or refresh a dense imported-asset WebGL2 regression that renders
      source glTF/GLB materials, not forced unlit override materials. The proof
      must capture browser output and verify stable visible pixels for textured
      and metallic/roughness materials before any capability is promoted.
- [ ] Depth/prepass robustness:
      prove single-primitive scenes, dense imported meshes, and mixed technical
      line/wire/edge overlays do not disable or corrupt depth for unrelated
      opaque triangles. Any legacy depth sentinel workaround is removed only
      after this proof passes.
- [ ] Browser external-asset and material trust:
      surface missing external images and buffers as typed warnings/errors,
      keep strict load behavior available for release proof, preserve
      cache-hit warning/provenance evidence, and prove oversized browser
      textures are clamped or rejected before upload according to capability
      limits.
- [ ] Source-material path audit:
      prove the browser proof path can retain imported source materials end to
      end when requested. Any host-side or demo-side fallback to generated
      unlit materials must be explicit in metadata and cannot satisfy a
      source-material visual proof.
- [ ] Fidelity gate integration:
      Phase 7 rendered-output proof for `SceneHost` cannot be marked complete
      while the dense WebGL2 source-material, depth/prepass, or external-asset
      epics are still open or untriaged against current `main`.

## Phase 0 - Scope, prior evidence, and contract baseline

Prerequisite before API work.

- [x] Read and reconcile:
      `docs/checklists/trust-platform-digital-twin-webgl-investigation.md`,
      `docs/checklists/trust-platform-1.4.0-verification.md`,
      `docs/checklists/trust-platform-finding-2-webgl-materials.md`, and
      `docs/checklists/next-release-easy-use-and-state-of-the-art.md`.
      Proof: current-main status is summarized in
      `docs/checklists/renderer-fidelity-dependencies.md`.
- [x] Record which prior findings are still open for this checklist:
      dense WebGL2 source-material proof, external asset warning visibility,
      strict asset-load report surfaces, depth/prepass proof, and browser
      capability/readback limitations. Proof:
      `docs/checklists/renderer-fidelity-dependencies.md`.
- [x] Create or update the required sibling renderer-fidelity checklists/issues
      named above, with owners, proof commands, and current-main status. If a
      prior finding was already fixed by later renderer work, record the exact
      current proof and close or reclassify it instead of carrying stale risk.
      Proof: `docs/checklists/renderer-fidelity-dependencies.md`.
- [x] Resolve the missing charter path referenced by AGENTS. `git log --all`
      did not show a prior `docs/RFC-rust-3d-renderer.md`, so Phase 0 authored
      a proposed charter rather than restoring one. The owner ratified it on
      2026-06-01 before Phase 1 public API work continued.
      Proof: `docs/RFC-rust-3d-renderer.md`.
- [x] Define the schema policy in docs: schema string format
      `scena.<contract>.vN`, additive fields, rename/break rules, snapshot
      fixture location, and migration expectations. Proof:
      `docs/schema-contracts.md`.
- [x] Add planned feature flags to `docs/feature-flags.md`, including the
      generic WASM host flag and its relationship to `inspection`,
      `viewer-element`, and `browser-probe`. Proof:
      `docs/feature-flags.md`.
- [x] Add a doctor rule plan for schema/doc drift:
      schema constants, docs links, feature flags, examples, and golden fixtures
      must stay aligned. Proof: `docs/schema-contracts.md`.
- [x] Add a forbidden-vocabulary doctor rule for new public API, schemas,
      examples, and docs touched by this track. Denylist:
      `robot`, `joint`, `urdf`, `plc`, `gripper`, `workpiece`, `weld`,
      `motion`, `trajectory`, `twin`, `simulation`, `controller`.
      The rule may allowlist this denylist declaration and archived
      investigation docs; new renderer contracts should not need these terms.
      The checked surface is prefix-scoped so new `src/scene_host/`,
      `src/capture/`, and track example subfiles are covered automatically.
- [x] Add a failing doctor fixture/test that plants denied vocabulary such as
      `joint` or `urdf` in a new public-contract surface and proves the rule
      fails before the fixture is corrected or allowlisted.
      Proof: `ARCH-PUBLIC-CONTRACT-VOCAB` in `xtask doctor --architecture`,
      covered by
      `doctor_rejects_public_contract_forbidden_vocab_regression`.
- [x] Add a release-hygiene note: each user-visible API/schema phase updates
      `docs/api.md`, `docs/browser.md`, `docs/lifecycle.md`, `docs/errors.md`,
      `docs/capabilities.md`, examples, release notes, and changelog entries as
      appropriate. Proof: this checklist plus `CHANGELOG.md` under
      `[Unreleased]`.

## Phase 1 - Serde foundation and stable wire schemas

Foundation for every later phase. Add focused tests before production changes.

- [x] Enable `glam/serde` so `Vec3`, `Quat`, and matrix values serialize as
      numeric arrays.
- [x] Add serde derives for wire-safe value types such as `Transform`, `Aabb`,
      `Color`, `GeometryTopology`, capability enums, and report structs where
      their public representation is stable.
- [x] Keep raw inspection structs internal-friendly, but add a separate stable
      scene-inspection wire view with schema
      `scena.scene_inspection.v1`.
- [x] `scena.scene_inspection.v1` includes stable `u64` node handles, parent
      handles, tags, local/world transforms, visibility, bounds, node kind,
      draw list, vertex/index/primitive counts, local bounds, topology,
      active camera, and revision counters.
- [x] Host-backed inspection accepts the `SceneHost` node-handle map and emits
      the same handle IDs. Standalone native inspection may allocate
      deterministic traversal IDs, but the contract must state that these are
      report-local when no host map exists. Phase 1 added
      `SceneInspectionReport::to_schema_report_with_node_handles` for the host
      map; the end-to-end host identity test lands with Phase 2.
- [x] Add topology helpers on the stable report view:
      `node_by_handle`, `children_of`, `roots`, and `find_by_tag`.
- [x] Add `scena.capability_report.v1` on `CapabilityReport`, replacing private
      browser-probe ad-hoc JSON for capability fields. Use serde names, not
      `format!("{:?}")`, for stable external values.
- [x] Add browser/wasm build proof for `inspection` plus the planned host
      feature. Phase 1 proof: `cargo build --target wasm32-unknown-unknown
      --features inspection`; the host feature proof lands with Phase 2.
- [x] Tests:
      serialize-deserialize round trips, schema string present, golden snapshot
      for a small scene, and deterministic report-local handle allocation when
      no `SceneHost` handle map is supplied.

## Phase 2 - Generic WASM `SceneHost`

This is the keystone. It must support both scene construction and per-frame
updates, otherwise consumers are forced back to offline merge/bake workflows.

- [x] Add a native-testable host core plus a `wasm_bindgen` wrapper behind a
      feature flag. Keep platform/browser adapters thin.
- [x] Add canvas attach/build APIs for WebGPU and WebGL2, reusing existing
      `PlatformSurface` browser constructors.
- [x] Expose browser lifecycle APIs:
      resize, DPR update, surface/context/device-loss event forwarding,
      active backend, capabilities JSON, diagnostics JSON, renderer stats JSON,
      and structured JS error codes.
- [x] Expose construction primitives:
      `add_empty(parent: Option<u64>, transform, tag: Option<String>) -> u64`,
      `set_tag(node: u64, tag: String)`, `clear_tag` or documented tag-removal
      decision, and `find_by_tag(tag) -> Vec<u64>` or a documented
      first-match API.
- [x] Pull native `Scene::instantiate_under(parent, &SceneAsset, ImportOptions)`
      into this phase and expose it through the host:
      `instantiate_glb_under(parent, bytes) -> import`,
      `instantiate_url_under(parent, url) -> import`, and root-level
      equivalents that delegate to `Scene::root()`.
- [x] Support external-resource loading for `.gltf` plus `.bin` and textures,
      not only monolithic GLB bytes. Report missing external buffers/images
      through typed load warnings. Phase 2 uses existing URL asset loading for
      external resources; the missing-external-buffer typed-warning expansion
      remains in Phase 5.
- [x] Maintain opaque generation-checked `u64` maps for nodes and imports.
      Import handles must expose roots and path/name lookups without leaking
      raw slotmap keys.
- [x] Expose lookup by import path, import name, tag, and stable inspection
      handle.
- [x] Expose per-frame update APIs:
      `set_transform(node, translation[3], rotation[4], scale[3])` and
      `set_transforms(batch)` with one transform revision bump for changed
      batches.
- [x] Expose render/proof APIs:
      `prepare`, `render`, `read_pixels`, `inspect_json`, `pick`,
      `frame_node`, and `frame_all`.
- [x] `pick(x, y)` receives CSS pixels and converts through the host's stored
      DPR/viewport state.
- [x] Do not expose `set_node_tint`, `remove_node`, or `capture` until their
      real implementations land. No stubs.
- [x] Tests:
      native host core builds a multi-part tree with empty frame nodes,
      instantiates assets under those frames, sets tags, batch-poses nodes,
      inspects with the same handles, proves posed node handle `H` appears as
      `H` with the expected transform in `inspect_json()`, picks by CSS-pixel
      coordinates, and frames a node/all.
- [x] Negative handle tests:
      fabricated out-of-range and generation-mismatched handles return
      structured `SceneHostErrorCode` values instead of panicking, silently
      aliasing, or falling back.
- [x] Browser proof:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof` builds
      the `scene-host` wasm package and constructs a multi-part scene in
      Chromium on Raspberry Pi V3D hardware without offline merge. Artifact:
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`.

## Phase 3 - Capture descriptor and public `scena::capture`

Bind pixels to the exact scene state that produced them.

- [x] Add schema `scena.capture.v1`.
- [x] Add `CaptureDescriptor` with width, height, pixel format or PNG sidecar
      metadata, revisions, active camera, camera world transform, projection,
      viewport/DPR, backend/capability summary, auto-frame metadata, pixel
      summary, nonblack count, bounding box, and FNV-1a 64-bit hash.
- [x] Prefer large bytes outside JSON when possible. JSON carries hashes,
      lengths, dimensions, and metadata; APIs may return `{ descriptor, bytes }`
      for PNG/RGBA payloads.
- [x] Snapshot revisions and camera state from the renderer's last rendered
      frame state, not from the current scene by convention.
- [x] Fail closed with `CaptureError::StaleRender` when the scene or active
      camera changes after render but before capture.
- [x] Promote browser-probe-only readback summarizing, hash, report
      serialization, and auto-frame metadata into public renderer/capture
      modules.
- [x] Expose capture natively and through `SceneHost`.
- [x] Document that CPU-headless captures are deterministic per renderer
      contract, while GPU/webview captures bind pixels to revisions and backend
      metadata rather than claiming byte identity across machines.
- [x] Tests:
      capture revisions match subsequent inspection, CPU-headless deterministic
      descriptor/hash proof, viewer capture auto-frame metadata, and host
      capture revision/hash proof.
- [x] Stale-render test:
      `render -> mutate scene -> capture` rejects the stale framebuffer instead
      of silently binding new revisions to old pixels.
- [x] Browser rendered-output proof for WASM paths:
      `SCENA_BROWSER_BACKENDS=webgl2 npm run browser:scene-host-proof` runs on
      Raspberry Pi V3D hardware and writes
      `target/gate-artifacts/scene-host-browser-proof/scene-host-browser-proof.json`
      plus `.png`. The proof verifies `SceneHost.capture()` descriptor JSON,
      RGBA8 payload, DPR metadata, and rendered pixels. The CPU builder still
      does not satisfy browser/GPU proof.

## Phase 4 - Native scene ergonomics and mutation primitives

These are thin additions over existing internals. Keep each item focused and
test-first.

- [x] Promote public `Transform::compose(parent, child)` and
      `impl Mul<Transform> for Transform`; replace duplicated private TRS
      composition copies. Proof: `tests/phase4_native_primitives.rs`.
- [x] Add recursive `Scene::remove_node(node)` and import removal. Remove from
      parent child lists, delete subtree data, invalidate host handles, and bump
      structure/transform revisions as appropriate. Proof:
      `tests/phase4_native_primitives.rs` and `tests/scene_host.rs`.
- [x] Add per-node tint/highlight without material cloning, applied during
      prepare for mesh and instance-set render paths. `Model` nodes remain an
      existing renderer limitation because current prepare rejects model nodes.
      Public APIs: `Scene::set_node_tint`, `Scene::node_tint`, `Node::tint`,
      `SceneHostCore::set_node_tint`, and WASM `setNodeTint` /
      `clearNodeTint`.
- [x] Finalize whether tint/highlight appears in inspection JSON and add the
      corresponding proof. `SceneNodeInspectionV1.tint` is part of
      `scena.scene_inspection.v1`; proof lives in `tests/scene_host.rs`.
- [x] Add node-anchored annotations with optional local offset and engine-owned
      projection output:
      `annotation_projection_report() -> scena.annotation_projection.v1`,
      `SceneHostCore::annotation_projections_json`, and WASM
      `annotationProjectionsJson`. Host output is CSS pixels.
- [x] Add `SceneAsset` geometry summary: node count, mesh count, primitive
      count, local bounds, and source units/coordinate summary where available.
      Public APIs: `SceneAsset::primitive_count`, `SceneAsset::bounds`,
      `SceneAsset::geometry_summary`, and schema
      `scena.asset_geometry_summary.v1`.
- [x] Add minor geometry/scene helpers:
      `world_distance(a, b)`, public `node_world_bounds(node, assets)`, and any
      host-backed convenience required by the Phase 2 browser proof.
- [x] Tests:
      one focused unit/integration test per primitive, plus stale-handle and
      generation-check tests for removal. Transform composition and removal are
      covered; tint is covered by `tests/phase4_native_primitives.rs`,
      `src/render/prepare/tests.rs`, and `tests/scene_host.rs`; annotation
      projection, asset summary, and geometry helper tests are covered by
      `tests/phase4_native_primitives.rs` and `tests/scene_host.rs`.

## Phase 5 - Serializable asset load reports

Make asset-load behavior inspectable and stable.

- [x] Add schema `scena.asset_load_report.v1`.
- [x] Derive or implement serde for `AssetLoadReport`, warnings, and progress
      events through a stable wire view.
- [x] Include asset geometry summary and external-resource summary:
      fetched bytes, cache hit, external buffers, external images, missing
      buffers, missing images, strict/lenient policy, and source URL/path.
- [x] Add typed warning for unresolved external buffers, not only unresolved
      images.
- [x] Ensure cached reports preserve enough warning/provenance information for
      browser proof, or document the cache-hit reporting contract explicitly.
- [x] Expose asset-load report JSON through native loaders and `SceneHost`.
- [x] Tests:
      missing-buffer and missing-image fixtures both produce typed warnings;
      strict mode promotes the right warnings to errors; browser URL loading
      records externally fetched resources.

## Phase 6 - Generic asset provenance

Provenance must be generic asset metadata, not application metadata.

- [x] Add `AssetProvenance` with source hash, license, generator, source URI or
      logical path, derivatives, and optional package/source metadata.
- [x] Attach provenance to loaded scene assets, textures, environments, and
      derived assets where the source is known.
- [x] Include provenance in asset-load report JSON and asset summaries.
- [x] Tests:
      loaded glTF/GLB reports expected source SHA-256; derived texture or
      environment assets record their source/derivative relationship.

## Phase 7 - Documentation, examples, release evidence, and doctor gates

This phase closes the public contract. It is not optional for user-visible API.

- [x] Update `docs/api.md` with the new public APIs and schema names.
- [x] Update `docs/browser.md` with `SceneHost`, host-pushed render cadence,
      canvas attach, resize/DPR, CSS-pixel picking, and browser asset loading.
- [x] Update `docs/lifecycle.md` so `SceneHost` uses the existing
      load/create -> mutate -> prepare -> render lifecycle instead of creating
      a parallel browser lifecycle.
- [x] Update `docs/errors.md` with host stale-handle, unsupported-backend,
      invalid-handle, missing-external-resource, and schema/serialization error
      families.
- [x] Update `docs/capabilities.md` with versioned capability JSON and browser
      backend proof expectations.
- [x] Update `docs/assets.md` with load reports, external buffer/image
      warnings, and provenance.
- [x] Update `docs/feature-flags.md` with host/schema feature combinations.
- [x] Add examples:
      native stable inspection JSON, browser `SceneHost` multi-part assembly,
      capture descriptor, asset-load report JSON, and provenance.
- [x] Add TypeScript/JavaScript snippets for browser host use.
- [x] Add golden JSON fixtures under tests or docs schema fixtures and wire
      them into doctor/source checks and live-serde equality tests so fixture
      drift fails before release.
- [x] Add release notes and changelog entries for each shipped public phase.
- [x] Run remote gates for implementation phases:
      `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`,
      `cargo test --all-features`, `cargo run -p xtask -- doctor --full`,
      `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`, and the
      WASM build/package checks.
- [ ] For browser-visible rendering changes, add rendered-output proof. Unit
      tests alone do not approve browser rendering behavior.

## Phase 8 - Product webview interaction follow-up

This phase is P1 for product webview usability. It is not required for the
deterministic render-proof phases, which must keep using scripted/auto-framed
camera state.

- [x] Expose interactive camera controls on `SceneHost` over WASM using scena's
      existing `OrbitControls` math from `controls.rs`.
- [x] Support orbit from pointer drag, dolly from wheel, and pan, with the
      embedder still owning the render cadence.
- [x] Add `set_camera` / `get_camera` host methods for saved viewpoints and
      scripted viewpoints.
- [x] Keep this generic renderer capability. Do not reimplement orbit math in
      TypeScript and do not add application-domain vocabulary.
- [x] Add native core camera-control tests for saved viewpoints, pointer orbit,
      wheel dolly, and pan.
- [ ] Add browser rendered-output proof on a real browser/GPU machine for the
      interactive webview path.

## Deferred

- [ ] glTF/GLB scene exporter. This is a separate exporter epic with its own
      design, writer dependency decision, binary-buffer assembly, import-export
      round-trip tests, and topology/transform preservation proof.
