# Full-repository review v1.9.0 remediation checklist

Created: 2026-07-22

Status: **implementation in progress — mandatory implementation, the single
CPU/WASM full chain, and exact Windows hardware proof are complete; final
GitHub provenance and public-version follow-through remain**

Source baseline: `main@a28f2149c39290aac7a059232b4e21de266ea88c`,
package version `1.9.0`

Audit: `docs/reviews/full-repo-review-v1.9.0.md`

Canonical charter: `docs/RFC-rust-3d-renderer.md`

This is the one active remediation backlog for the v1.9.0 full-repository
review. It separates mandatory correctness/proof/documentation closure from
optional feature development. “Perfect” means zero known release-blocking
defects at the frozen release candidate, fail-closed contracts, truthful docs,
and green required evidence. It does not mean that every possible renderer
feature must ship in this batch.

## 0. Execution contract

### 0.1 Test cadence — focused per fix, full tests once at the end

Do not run the complete test/release chain after each small change.

For every implementation row:

1. Add the narrowest deterministic regression that fails on the baseline for
   the claimed reason.
2. Run that focused proof on the bootstrapped remote builder or required GPU
   host and preserve the command/output/artifact.
3. Classify the failure as product, harness, environment, policy, or provenance.
4. Make the smallest production change that closes the tested contract.
5. Rerun the identical focused proof until green.
6. Run only the scoped gate implied by touched files.
7. Update the row's validation ledger before starting the next logical item.

Scoped gates:

- Rust change: focused Rust test, then one `cargo fmt --all --check` for the
  logical group. Add the affected integration target or clippy package only
  when the changed surface warrants it.
- glTF/import change: focused fixture test plus the glTF conformance/import
  target; do not run unrelated browser suites.
- JavaScript/WASM/browser-visible change: focused evaluator first, then the one
  affected browser lane. Real hardware is required only for a hardware claim.
- Shader/output change: offline variant validation plus the smallest rendered
  comparison on each affected backend. Unit tests alone cannot close it.
- CLI/schema/docs/doctor/workflow change: focused contract or mutation test;
  run `xtask doctor --full` once after the related group freezes.
- Performance change: benchmark distribution and rendered-output parity on a
  controlled host. GitHub-hosted timing remains report-only.
- Public API change: compile/API fixture in the item; rustdoc, semver, and
  publish dry-run wait for the final checkpoint.

Run the **full chain exactly once** in section 10 after all mandatory rows are
green and the release-candidate diff is frozen. If that chain finds a failure,
reduce it to a focused reproducer and rerun only affected gates until the fix
changes a cross-cutting surface. Do not repeatedly rerun already-valid broad
gates for a newer timestamp.

### 0.2 Investigation circuit breakers

- [x] Preserve the exact failing command, source commit, fixture/artifact hash,
  complete output, backend/adapter, and relevant environment before editing.
- [x] After two failed remediation attempts with the same signature, freeze
  product and harness edits until a smaller probe falsifies a competing cause.
- [x] After 30 minutes without a proven cause, record the checkpoint and next
  single discriminating probe; do not compensate with a broader suite.
- [x] For a failed GitHub run, collect all failed jobs with
  `scripts/collect_ci_failure_evidence.sh <run-id>` before the next RC push.
- [x] Never weaken an oracle or widen a threshold merely to obtain green.
- [x] Never convert unavailable hardware, missing evidence, or unknown
  provenance into a pass.

Every completed item records:

- `focused red`: exact command, failure, source, host/backend.
- `classification`: product, harness, environment, policy, or provenance.
- `implementation`: files and contract changed.
- `focused green`: identical command and result.
- `scoped`: additional narrow gates and why.
- `full`: `deferred to section 10` until the frozen checkpoint.
- `skipped`: broader gates intentionally omitted and why.
- `counts`: elapsed investigation time, remediation attempts, RC pushes,
  full-matrix runs, user actions.

### 0.3 Checkout and remote-builder bootstrap

Before editing after a branch switch, every remote sync, and every cargo gate:

- [x] Record canonical source `/home/johannes/projects/scena`, destination,
  branch, and HEAD.
- [x] Run
  `ssh scena-builder 'bash -s -- full-review-1-9-audit' < scripts/scena_remote_builder_preflight.sh`.
- [x] Use the isolated `validation_path` and `CARGO_TARGET_DIR` printed by the
  preflight; never assume the old shared checkout exists.
- [x] Mirror the exact local tree excluding `.git` and `target`.
- [x] Manually copy root `AGENTS.md` and complete `.codex/skills/**` after the
  mirror, then compare canonical/destination hashes.
- [x] Read the destination instructions and required skills before a gate.
- [ ] Clean only the task-scoped remote snapshot/cache when the batch ends.

### 0.4 Checkpoints

- [x] **A — release blockers:** C01-C04 focused red/green and scoped gates.
- [x] **B — remaining correctness:** C05-C23 focused red/green and scoped gates.
- [x] **C — proof, CLI, performance, docs:** Q01-Q12, A01-A15, P01-P08,
  D01-D06, and H01-H02 focused/scoped evidence.
- [ ] **D — final:** section 10 once on one frozen source commit.

Optional features F01-F09 are not prerequisites for checkpoints A-D.

## 1. Release-blocking correctness

### C01 — Preserve raw glTF texture index identity

Owner: `src/assets/gltf/textures.rs`, glTF material import tests.

- [x] Add a fixture with at least three glTF texture entries: an unresolvable
  early entry and a later valid image referenced by a material.
- [x] Prove the baseline binds the later material slot to the wrong decoded
  texture or loses it because `filter_map` compacted the vector.
- [x] Replace the compact vector contract with index-aligned optional entries
  or an explicit raw-index map.
- [x] Return a structured error naming material, slot, raw texture index, image
  source, and reason when a referenced entry cannot resolve.
- [x] Prove invalid unreferenced entries do not shift valid entries.
- [x] Cover base-color, metallic/roughness, normal, occlusion, emissive,
  transmission, and extension-owned texture slots.
- [x] Cover URI, data URI, buffer-view/GLB, duplicate image, and sampler cases.
- [x] Add a doctor/source rule preventing `document.textures().filter_map(...)`
  from feeding raw-index material resolution, or encode the invariant in a
  shared indexed-collection type that cannot compact.
- [x] Update importer diagnostics and supported-input docs if failure wording
  changes.
- [x] Acceptance: every resolvable material slot binds the texture at its glTF
  index; a referenced invalid entry fails closed without rebinding neighbors.

### C02 — Decode all accepted quantized tangent and morph accessors

Owner: `src/assets/gltf/meshes.rs`, extension/conformance fixtures.

- [x] Add minimal fixtures for BYTE/UNSIGNED_BYTE/SHORT/UNSIGNED_SHORT and F32
  tangent accessors under `KHR_mesh_quantization`.
- [x] Add morph POSITION, NORMAL, and TANGENT delta fixtures for normalized and
  permitted non-normalized component representations.
- [x] Prove the baseline debug failure and release-mode corruption/zeroing with
  a value-level oracle, not merely successful loading.
- [x] Implement one bounds-checked, component-aware accessor decoder with glTF
  normalization semantics; do not duplicate unsafe byte interpretation.
- [x] Validate accessor type, component type, stride, offset, count, sparse
  overrides, finite decoded values, and extension declaration.
- [x] Preserve tangent handedness and morph-target index identity.
- [x] Reject a quantized encoding that the glTF contract does not allow instead
  of silently filling zero/default vectors.
- [x] Add malformed/truncated/overflow known-bad fixtures.
- [x] Update extension capability docs so “supported” means every accepted
  semantic has a value-level fixture.
- [x] Acceptance: debug and release builds produce the same expected decoded
  values and rendered mutation proof for each supported component type.

### C03 — Bind capture pixels to exact readback-frame provenance

Owner: `src/capture.rs`, `src/render/frame.rs`, capture contract tests.

- [x] Add a focused sequence: render/capture A synchronously, mutate scene,
  render B present-only, then request capture without a B readback.
- [x] Prove the baseline can return A pixels certified with B rendered state.
- [x] Make returned bytes, frame state, scene revision, camera, output config,
  dimensions, backend, and timestamp come from one readback completion record.
- [x] Reject missing/stale readback with a structured remedy; never substitute
  `last_rendered_frame_state` for pixel provenance.
- [x] Preserve asynchronous two-slot readback behavior and row-padding rules.
- [x] Cover resize, camera switch, output-toggle, device/surface recovery, and
  skipped-frame sequences.
- [x] Update `capture_descriptor` schema/goldens and trust-consumer docs if the
  provenance shape changes.
- [x] Add a known-bad mutation that swaps rendered/readback state and must fail.
- [x] Acceptance: no API can label pixels with state that did not produce them;
  stale requests fail closed and explain how to obtain a fresh capture.

### C04 — Correct color transfer for every native surface format

Owner: native GPU surface/output path and color-management tests.

- [x] Add a pure target-format contract test showing when shader sRGB encoding
  is required for sRGB and non-sRGB surface formats.
- [x] Add a rendered reference with midtone patches that clearly distinguishes
  linear bytes from sRGB-encoded bytes.
- [x] Prove native non-sRGB output currently reuses offscreen sRGB state.
- [x] Build output uniforms/bind groups from the actual target format for each
  pass, or define an explicit linear intermediate plus format-correct blit.
- [x] Keep browser, offscreen capture, post-enabled, post-disabled, MSAA, and
  surface-present paths consistent.
- [x] Prove sRGB surfaces do not receive double encoding.
- [x] Run the focused physical target-format proof and record unsupported
  native target classes as unavailable rather than passed.
- [x] Update capability output to report the actual selected surface format and
  transfer contract.
- [x] Acceptance: native/browser readback and presentation match the reference
  within the established color oracle for every supported target class.

## 2. High-priority correctness and interaction

### C05 — Honor glTF scene selection

- [x] Add a multi-scene fixture containing shared nodes and a node reachable
  only from a non-default scene.
- [x] Specify no-default behavior, default-scene behavior, explicit index/name
  selection, shared-node ownership, animation/skin references, and empty scenes.
- [x] Import only roots reachable from the selected scene while retaining
  required dependency data.
- [x] Expose selected source scene in the load report/provenance.
- [x] Reject invalid/ambiguous selection with available candidates.
- [x] Document backward compatibility and provide an explicit “all scenes”
  option only if a real host use case requires it.
- [x] Acceptance: scene contents and root order follow glTF scene semantics
  deterministically on native and WASM loaders.

### C06 — Preserve host placement during hot reload

- [x] Add nested host-parent fixtures with multiple import roots, non-identity
  root locals, hidden state, and an external texture change.
- [x] Prove replacement currently returns roots to the scene root.
- [x] Capture old root-parent/local-placement mapping before mutation.
- [x] Stage and validate the replacement before removing the live import; make
  replacement atomic on error.
- [x] Reattach corresponding new roots deterministically and define behavior
  for added/removed/renamed roots.
- [x] Preserve selected host-owned state without retaining stale asset-owned
  handles.
- [x] Cover repeated reloads, cache identity, texture edits, and rollback.
- [x] Acceptance: successful reload changes asset data without changing host
  placement; failed reload leaves the original live import intact.

### C07 — Normalize browser wheel zoom

- [x] Add JS tests for `deltaMode` pixel/line/page values and representative
  mouse-wheel and trackpad deltas.
- [x] Add Rust controller tests for finite values, direction, symmetry, bounds,
  and multiple small deltas versus one aggregate delta.
- [x] Normalize at the browser boundary and use bounded exponential scaling.
- [x] Reject or ignore NaN/infinite wheel input without corrupting camera state.
- [x] Prevent page scrolling only while the viewer intentionally consumes zoom.
- [x] Document zoom sensitivity if exposed publicly.
- [x] Acceptance: one ordinary wheel event changes distance modestly, trackpad
  zoom is smooth, and zoom-in/out are approximately reciprocal.

### C08 — Make camera descriptors validated, revisioned scene state

- [x] Add failing tests for NaN, infinity, non-positive aspect, invalid
  near/far, degenerate orthographic extents, FOV <= 0 and FOV >= 180 degrees.
- [x] Add render-on-change proof where only aspect/FOV/near/far changes and the
  next frame must not be skipped.
- [x] Centralize projection validation and route constructors, framing, resize,
  recipe build, import, and public mutation through it.
- [x] Prefer checked setters/private invariants; if public fields must remain
  for compatibility, validate at prepare and return a structured hard error.
- [x] Add camera-descriptor revision to frame/prepared/capture identity.
- [x] Make resize update viewport, camera aspect according to documented policy,
  picking normalization, and frame dirtiness coherently.
- [x] Surface remedy text for invalid/no active camera in CLI and host APIs.
- [x] Acceptance: invalid active cameras never yield a blank successful render,
  and intrinsic-only changes always invalidate the correct work.

### C09 — Validate authored and imported animation through one contract

- [x] Add public `AnimationSourceClip` tests for NaN/Inf time/value/tangent,
  decreasing/duplicate times, zero duration, invalid interpolation shape, and
  channel/weight dimensional mismatch.
- [x] Prove unchecked source construction/rebinding can currently reach mixer
  transforms.
- [x] Make construction and rebinding fallible with channel/keyframe paths and
  remedies, or introduce a checked builder while deprecating unchecked public
  entry points.
- [x] Reuse the same validator for imported and authored clips.
- [x] Ensure sampling/mixing cannot write a non-finite transform even if an
  internal invariant is violated; return diagnostics rather than poisoning the
  scene.
- [x] Update public API, examples, JSON errors, and semver assessment.
- [x] Acceptance: no public clip path can create a non-finite runtime transform.

### C10 — Correct hierarchical exploded-view transforms

- [x] Add exact-world-position tests for a three-level hierarchy, siblings,
  rotated/scaled parents, zero-size bounds, and nested imported roots.
- [x] Prove the baseline double-counts ancestor displacement in the failing
  hierarchy case.
- [x] Define hierarchy, radial, and axis mode semantics in world space.
- [x] Compute children against planned final parent worlds in topological order,
  or apply a mathematically equivalent local delta exactly once.
- [x] Keep apply/revert idempotent and preserve original locals exactly.
- [x] Add an image/semantic-AOV proof showing separation without unintended
  subtree distortion.
- [x] Update exploded-view docs and examples.
- [x] Acceptance: target world transforms match the specification at every
  hierarchy level and exact revert restores the original scene.

### C11 — Keep SceneHost viewport, resize, DPR, and picking synchronized

- [x] Add the focused reproducer: construct a 640x480 host, send only
  `SurfaceEvent::Resize { width:1280, height:960 }`, and pick known geometry at
  coordinates valid under the new target. Prove the baseline ray still uses
  640x480 NDC and misses or hits the wrong target.
- [x] Add the control case using `ViewportChanged`, which must pick correctly;
  this distinguishes stale host viewport state from geometry/camera defects.
- [x] Update `self.viewport` in the `Resize` arm using the retained DPR before
  forwarding/announcing the event.
- [x] Apply `ScaleFactorChanged` to host DPR and physical/logical dimensions;
  define what dimensions are authoritative when the platform reports scale and
  size as separate events.
- [x] Ensure emitted `SurfaceResized` CSS/physical fields are computed from the
  new viewport, never stale DPR.
- [x] Cover Resize→Scale, Scale→Resize, zero/minimized size, non-1/fractional
  DPR, repeated events, and direct `resize()` API parity.
- [x] Run browser/interactive picking proof after the Rust contract test; a unit
  test alone cannot close event-coordinate behavior.
- [x] Rerun C08 intrinsic-revision tests so camera aspect, viewport, renderer
  target, and picking cannot diverge.
- [x] Acceptance: natural winit-style 1:1 event forwarding keeps render target,
  host viewport, emitted event, camera policy, and pick ray synchronized.

### C12 — Retain unculled source templates for dynamic preparation (B14/R1)

- [x] Add a node contributing both a mesh and retained stroke/overlay; cull only
  the mesh during full prepare, then move the camera so it must re-enter.
- [x] Prove the other retained representation satisfies the source-coverage
  guard while the missing mesh remains invisible, and separately prove an
  entirely culled node forces full prepare on every pan.
- [x] Store a source-complete retained template before frustum/occlusion
  decisions and perform view-dependent culling during dynamic encode.
- [x] Preserve structural invalidation for topology/material/resource changes.
- [x] Cover mesh, stroke, label, instance, LOD, transparency order, clipping,
  occlusion, and objects entering/leaving the view.
- [x] Acceptance: dynamic output matches a forced full prepare and camera-only
  changes no longer become full prepares merely because a node is off-frustum.

### C13 — Make CPU occlusion culling conservative for thin/high-resolution detail

- [x] First create the missing rendered reproducer: a high-resolution visible
  sliver or gap that becomes covered in the 256px occlusion buffer and is
  falsely culled.
- [x] Record output at occlusion off/on and multiple target sizes; do not change
  constants unless the fixture fails for the predicted downsampling reason.
- [x] Select a documented conservative policy: depth bias/dilation, minimum
  projected size exemption, hierarchical resolution, or a higher bounded cap.
- [x] Prove true large occluded geometry still culls and thin visible geometry
  never disappears.
- [x] Benchmark added occlusion work on a controlled host.
- [x] Acceptance: occlusion may miss an optimization but may not remove visible
  geometry relative to the non-occlusion reference.

### C14 — Refresh surface configuration after `Suboptimal`

- [x] Add a surface-policy test proving `PresentThenReconfigure` currently
  reapplies the identical cached config.
- [x] Route post-present recovery through `refresh_surface_configuration` with
  adapter/device/current target.
- [x] If format or present mode changes, return the structured
  configuration-changed result and require prepare before rendering again.
- [x] Cover unchanged configuration, resize, format change, lost/outdated, and
  repeated suboptimal signals without a reconfiguration loop.
- [x] Acceptance: a suboptimal signal performs one meaningful refresh and never
  hides a pipeline/target format mismatch.

### C15 — Restore visibility completely when snapshot nodes went stale

- [x] Add a snapshot/isolate test, remove one recorded node, then restore.
- [x] Prove the baseline aborts before restoring later live entries and leaves
  isolate bookkeeping populated.
- [x] Restore every live entry, skip stale entries deterministically, and clear
  toolkit state in all completion paths.
- [x] Return a structured result/report listing skipped stale nodes if callers
  need to distinguish exact from partial restoration.
- [x] Cover removed parent/subtree, repeated restore, empty snapshot, and error
  rollback semantics.
- [x] Acceptance: no stale snapshot entry can strand the scene in isolate mode.

### C16 — Emit complete, meaningful hover transitions

- [x] Specify the event sequence for none→A, A→A, A→B, A→none, and none→none.
- [x] Add host event tests proving A→B emits `Left(A)` followed by `Entered(B)`.
- [x] Stop emitting a meaningless empty `Moved`, unless a versioned contract
  explicitly retains pointer movement without a hit.
- [x] Preserve event ordering, coordinates, typed target handles, and WASM JSON.
- [x] Version/migrate the host-event contract if one callback currently assumes
  exactly one event per move.
- [x] Acceptance: consumers can maintain hover state from events alone without
  synthesizing a missing leave.

### C17 — Pick with the camera that actually rendered

- [x] Build an interactive viewer with camera A, switch scene active camera to
  B, render B, and prove baseline `pick_at` still casts from A.
- [x] Resolve `scene.active_camera()` for picking or make the viewer-owned
  camera and scene active camera one revisioned invariant.
- [x] Return the standard no-active/invalid-camera error when appropriate.
- [x] Cover camera removal, replacement, builder framing, and selection helper.
- [x] Acceptance: a visible pixel and its pick ray always use the same active
  camera state.

### C18 — Preserve generations in retired anchor/connector diagnostics

- [x] Add slot-reuse tests that retire a handle, allocate a replacement in the
  same slot, retire again, and query both stale handles.
- [x] Prove the `u32` slot key overwrites or misassociates generation metadata.
- [x] Key retirement records by full typed key/`KeyData` or a generation-safe
  external identity.
- [x] Bound retirement retention and define pruning without breaking stale
  diagnostics.
- [x] Cover transaction rollback and import replacement.
- [x] Acceptance: stale diagnostics identify the exact retired generation and
  never a newer object that reused its slot.

### C19 — Track pointer-button gesture ownership

- [x] Add sequences with simultaneous primary/secondary pointers/buttons,
  unrelated release, matching release, and cancellation.
- [x] Prove releasing one button currently clears both orbit and pan.
- [x] Track the initiating pointer/button per active gesture and end only that
  gesture; cancellation clears the affected pointer's state safely.
- [x] Keep touch/mouse/pointer-capture behavior consistent with viewer-element.
- [x] Acceptance: unrelated release cannot terminate another active gesture or
  leave a gesture stuck.

### C20 — Diagnose invalid material-variant mappings

- [x] Add an asset whose optional mapping references an out-of-range material.
- [x] Prove it disappears without an `AssetLoadWarning`.
- [x] Preserve primitive/mapping/material/variant indices while parsing and
  emit a structured optional-extension warning, or fail when policy requires
  strict extension fidelity.
- [x] Ensure valid mappings retain raw order and are not shifted by invalid ones.
- [x] Acceptance: no malformed variant mapping is silently discarded.

### C21 — Preserve warnings for `load_scene_from_bytes` cache hits

- [x] Load warning-producing bytes twice under the same cache key and compare
  first/cached reports with the disk loader behavior.
- [x] Include `scene.load_warnings()` in byte-loader telemetry before caching.
- [x] Preserve fetched/decode/cache-hit counters without duplicating warnings.
- [x] Cover retain policy, changed bytes/same path, and option-sensitive cache
  identity.
- [x] Acceptance: cache hits report the same semantic warnings as the original
  parse.

### C22 — Validate animation morph-weight width against target geometry

- [x] Add a mesh with N morph targets and a bound animation channel with N-1
  and N+1 weights; prove baseline playback silently truncates through `zip`.
- [x] Validate width during import/rebind/instantiation while both channel and
  target geometry metadata are available.
- [x] Return a structured clip/channel/node expected-vs-actual error before
  playback; do not resize or pad silently.
- [x] Cover multi-primitive meshes, per-node weight overrides, cubic spline,
  and changing assets on hot reload.
- [x] Acceptance: every bound weight sample has exactly the target width.

### C23 — Keep GPU fallback inside versioned JSON output

- [x] Add a CLI capture test requesting GPU on an unavailable host and parse
  every stdout/stderr line as the declared machine contract.
- [x] Prove baseline `warn_gpu_fallback` emits prose.
- [x] Put requested/selected backend, fallback status, reason, and remedy in the
  existing result envelope or a versioned warning entry.
- [x] Keep human formatting an explicit mode, not an unsolicited stderr side
  channel.
- [x] Update help/errors/schema fixtures and agent guide.
- [x] Acceptance: fallback is actionable and no machine-mode stream contains
  unversioned prose.

Validation ledger (C01-C23):

- `focused red`: each family was pinned against `a28f2149` before production
  changes. The principal failing targets are the indexed-texture unit fixtures,
  `c04_gltf_deformation_contracts`, `capture_contracts`, native target-transfer
  units, `gltf_validation_contracts`, import transaction tests, browser wheel
  evaluator, `camera_projection_validation`, authored animation validation,
  `exploded_view`, `scene_host`, phase-5 retained-state tests, conservative
  culling tests, surface policy units, `inspection_tools`, interactive viewer,
  pointer ownership, material-variant units, and machine GPU-fallback tests.
- `classification`: C01-C23 were product correctness or silent-diagnostic
  defects. C04 also has a physical-output proof obligation; implementation
  does not turn adapter unavailability into a pass.
- `implementation`: raw identity, checked decode/validation, readback
  provenance, target-format transfer, scene selection, atomic reload, input and
  viewport state, finite/revisioned animation/cameras, world-space exploded
  planning, source-complete retained state, conservative occlusion, meaningful
  surface refresh, generation-safe diagnostics, and versioned CLI fallback now
  have one owner and structured failures.
- `focused green`: all named focused targets passed on the isolated remote
  builder during their slices; rendered CPU references, browser evaluator
  tests, and applicable lavapipe comparisons passed. The exact
  `4e84ccf05ad7d525c39e5d13091019e31a793984` Windows candidate passed the
  native DX12 and browser WebGPU/WebGL2 output proofs. The browser WebGPU target
  reported non-sRGB `Bgra8Unorm`; no separate native non-sRGB surface class was
  exposed, so none is claimed.
- `scoped`: Rust formatting and the affected glTF, SceneHost, CLI, schema,
  browser, doctor, and rendered-output targets were run per slice. The current
  cumulative diff has not yet run the one final broad chain.
- `full`: recorded in section 10. `skipped`: no repeated full suite; a physical
  non-sRGB surface is recorded unavailable if the exact Windows adapter does
  not expose one.
- `counts`: zero release-candidate pushes, one full matrix, one user hardware
  action for this candidate.

## 3. Proof and release-gate integrity

### Q01 — Validate every generated WGSL variant offline

- [x] Inventory all assembled vertex/fragment/compute sources and feature axes:
  binding mode, material extensions, shadows, post, labels, strokes, depth,
  picking, semantic AOVs, instancing, skinning, morphing, and backend profile.
- [x] Generate a deterministic shader-variant manifest from production assembly
  metadata; do not hand-maintain an unrelated list.
- [x] Naga parse and validate every legal variant for its target capabilities.
- [x] Add known-bad syntax, binding, entry-point, location, and capability
  mutations that the gate rejects.
- [x] Keep semantic substring/source rules only where Naga cannot prove the
  renderer contract; delete redundant brittle pins after equivalent negative
  proof exists.
- [x] Wire the manifest gate into CI before browser/hardware work.
- [x] Acceptance: every production variant is compiled offline and omission of
  a new variant fails a doctor/manifest coverage test.

Validation ledger (2026-07-22):

- `focused red`: remote
  `cargo test production_shader_modules_are_created_only_by_manifest_owner --lib -- --nocapture`
  failed with the seven production bypasses in `depth.rs`, `labels.rs`,
  `pipeline.rs`, `post/mod.rs`, `semantic_aov/webgl2.rs`, `shadow.rs`, and
  `strokes.rs`.
- `classification`: product/architecture enforcement defect; the offline list
  could validate its own entries while production created an omitted shader
  directly.
- `implementation`: `src/render/gpu/shader_manifest.rs` now owns a macro-derived
  typed registry, feature-axis inventory, source, profile, entry points, and the
  only `Device::create_shader_module` call. All production pipeline builders use
  that owner. CI, release, and hardware workflows run the offline gate before
  browser work. Doctor rejects a raw module-creation bypass and obsolete raw
  shader-source pins were replaced with typed routing pins.
- `focused green`: all four `shader_manifest::tests` pass, including Naga
  validation, syntax/binding/location/entry/capability mutations, omitted-entry
  mutation, and the production-bypass scan. The executed doctor mutation
  `app::tests_02::doctor_rejects_shader_module_creation_outside_generated_manifest`
  and real-tree `renderer_truth_contracts_are_source_enforced` both pass.
- `scoped`: remote `cargo fmt --all --check` passed; lavapipe
  `headless_gpu_clipping_plane_set_discards_fragments` rendered successfully;
  a `wasm-pack build --dev ... --features browser-probe` completed with no Rust
  warnings after the WASM-only dead-code annotations were corrected.
- `environment checkpoint`: the first remote Chromium render attempt failed
  before loading scena with `ERR_INSUFFICIENT_RESOURCES`; a task-local `TMPDIR`
  retry passed navigation but hung without probe output and was stopped after
  the bounded second observation. Host memory/disk/inodes were available, but
  an unrelated five-day-old orphan Chromium tree exists on the shared builder.
  No production/harness patch was made. Browser hardware execution remains part
  of the exact-candidate Windows gate before push.
- `full`: deferred to section 10. `skipped`: no broad suite; production shader
  bytes are unchanged and the exact final Windows candidate must still pass all
  browser/native hardware proofs.
- `counts`: about 35 minutes; one product remediation; two browser-environment
  attempts; zero RC pushes; zero full matrices; zero user actions.

### Q02 — Compile every public example with required features

- [x] Generate or validate a manifest mapping every Cargo example to its
  `required-features` combination.
- [x] Add CI checks for `scene_host_contracts`, `scene_host_release_1_7`,
  `asset_catalog_picker`, `product_configurator`, and
  `application_builder_lab`, plus future gated examples automatically.
- [x] Add a known-bad fixture proving an omitted required-feature example fails
  the doctor/CI coverage rule.
- [x] Correct README wording so its command either truly compiles every public
  example or points to the maintained script that does.
- [x] Acceptance: adding a public example without compile coverage fails CI.

Validation ledger (2026-07-22):

- `focused red`: the audited baseline's CI/release command was
  `cargo check --examples`; the executed doctor mutation
  `doctor_rejects_public_example_gate_without_all_features` rejects that exact
  command because Cargo skips targets whose `required-features` are disabled.
- `classification`: proof/CI coverage defect; the examples existed and Cargo
  declared their requirements, but the claimed all-example gate did not enable
  them.
- `implementation`: `tests/public_examples_manifest.rs` derives the complete
  example/path/required-feature map from Cargo metadata, compares it to every
  `examples/*.rs` source, pins the six explicitly gated examples, and rejects an
  omitted target mutation. CI/release/macOS/Windows/publish commands and public
  docs use `cargo check --examples --all-features`; the existing doctor owner
  now also pins the metadata test, scripts, and docs without a duplicate rule.
- `focused green`: remote
  `cargo test --test public_examples_manifest -- --nocapture` passed and remote
  `cargo check --examples --all-features` compiled every public target. The
  doctor mutation and real-tree coverage tests both pass.
- `scoped`: remote `cargo fmt --all --check` passed. The first metadata-test run
  exposed a harness assumption that Cargo omits the `required-features` JSON
  key for empty lists; the parser was corrected to treat omission as an empty
  set and the identical proof passed.
- `full`: deferred to section 10. `skipped`: no broad suite because this slice
  changes coverage metadata/workflow/docs, not example runtime behavior.
- `counts`: about 20 minutes; one test-harness remediation; zero RC pushes; zero
  full matrices; zero user actions.

### Q03 — Add CI-issued provenance to release evidence

- [x] Specify versioned provenance fields: repository, workflow/ref, workflow
  SHA, run id, run attempt, job, source commit, artifact digest, issuer, and
  attestation/verification status.
- [x] Populate fields only from trusted CI context; do not allow arbitrary local
  environment values to mint release-grade evidence.
- [x] Keep local diagnostic artifacts useful but force
  `release_evidence=false` with an explicit reason.
- [x] Validate source commit reachability and artifact digest at staging.
- [x] Reject replay, wrong repository/ref, missing job, and tampered artifact
  known-bad fixtures.
- [x] Document trust boundary and retention requirements.
- [x] Acceptance: a self-reported exact commit is insufficient for publication;
  only CI-issued verified provenance satisfies the release gate.

Validation ledger:

- `focused red`: remote Node and xtask focused tests initially failed because
  the CI provenance producer/consumer did not exist; the readiness test then
  demonstrated that local staging metadata could not satisfy the new gate.
- `classification`: policy/provenance defect. Existing artifacts named a source
  commit but had no independently verifiable CI identity.
- `implementation`: `scripts/ci_provenance.js` emits the versioned pending
  manifest from GitHub-owned context and hashes the complete downloaded tree.
  Both release-producing workflows use commit-pinned `actions/attest` with OIDC
  permissions. Strict staging verifies repository/workflow/ref/run/job/source,
  commit reachability, tree digest, and the live SLSA attestation; it retains the
  signed manifest. Readiness independently invokes `gh attestation verify`
  rather than trusting the recorded receipt. Local staging is explicitly
  non-release. `RELEASE-CI-PROVENANCE` doctor coverage rejects missing wiring.
- `focused green`: remote `node tests/release/ci_provenance_test.js`; remote
  `cargo test -p xtask q03_`; canonical staging test; and the live M9 workflow
  contract test all passed.
- `scoped`: remote `cargo fmt --all --check` passed. The documentation now
  records the trust boundary and retention contract.
- `full`: deferred to section 10. Live GitHub OIDC issuance is deliberately not
  self-simulated before push; the official release remains blocked until the
  authorized exact-commit workflow verifies it.
- `skipped`: no workspace suite or release workflow was run during this slice.
- `counts`: about 45 minutes; two test-harness corrections, zero RC pushes,
  zero full matrices, zero user actions.

### Q04 — Keep smoke evidence distinct from image parity

- [x] Rename/schema-label nonblack/draw/submission checks as renderer smoke or
  conformance, never parity.
- [x] Ensure every release workflow that claims GPU parity requires current
  full-frame Q01 and its known-bad mutation set.
- [x] Keep vertical-flip/mirror rejection, thresholds, diff PNG, worst-region
  bbox, and source provenance pinned.
- [x] Add a doctor rule rejecting `release_evidence=true` for smoke-only output.
- [x] Acceptance: no user-facing headline or release manifest can equate
  nonblack output with parity.

Validation ledger:

- `focused red`: remote
  `node tests/browser/browser_evidence_classification_test.js` failed because
  the aggregate producer had no evidence classifier.
- `classification`: proof-labeling/policy defect; Q01 pixel evaluation was
  already rigorous, but aggregate smoke status did not state its scope.
- `implementation`: browser aggregates now distinguish `renderer-smoke`,
  diagnostic software WebGPU pixel comparison, and the exact strict Q01
  full-frame class. Only the latter can set `release_evidence:true`, and its
  scope is explicitly the unlit WebGPU triangle. Staging labels its merged
  multi-backend output as non-release conformance. The Rust release consumer
  rejects mislabeled smoke, and doctor owns the classifier/consumer/docs.
- `focused green`: remote browser classification and required-GPU evaluator
  tests passed; the focused xtask hardware/conformance consumer tests, doctor
  mutation, and canonical staging test passed.
- `scoped`: remote `cargo fmt --all --check` passed. The existing Q01 contract
  continues to pin six mutation rejections, thresholds, heatmap, worst-region
  bbox, exact frame sources, and provenance.
- `full`: deferred to section 10. `skipped`: no live browser rerender because
  this slice changes evidence metadata/consumption, not pixels; exact browser
  hardware runs in the final Windows gate.
- `counts`: about 25 minutes; one Rust compile correction and one doctor-fixture
  correction; zero RC pushes; zero full matrices; zero user actions.

### Q05 — Expand proof coverage for corrected correctness families

- [x] Add rendered/value-level fixtures for C01/C02 rather than load-only tests.
- [x] Add capture-provenance mutation proof for C03.
- [x] Add native color reference proof for C04 and browser transfer parity.
- [x] Add scene-selection/hot-reload semantic AOV proof for C05/C06.
- [x] Add camera invalidity and intrinsic-revision negative proof for C08.
- [x] Add hierarchy visual/semantic proof for C10.
- [x] Acceptance: each high-severity family has at least one rejected known-bad
  implementation, not only a green happy path.

Validation ledger:

- `focused`: C01 preserves raw texture indices through decoded-slot and
  rendered-material tests; C02's accepted component matrix is exercised by
  rendered tangent/morph fixtures and malformed-accessor rejection; C03 rejects
  a swapped readback/rendered-state mutation; C08 projection validation and
  render-on-intrinsics-change tests are green; C10 exact world-transform and
  semantic-AOV placement tests are green.
- `focused Q05 addition`: C05's multi-scene fixture now proves only the selected
  source identity contributes semantic pixels. C06's replacement transaction
  preserves identity mask, depth, and normals and rejects a lost-placement
  mutation. Remote commands were `cargo test --features scene-host --test
  gltf_validation_contracts
  gltf_default_scene_selection_is_pinned_by_semantic_aov_pixels -- --exact` and
  `cargo test --lib
  replacement_parent_and_placement_are_pinned_by_semantic_aov_pixels`; both
  passed.
- `physical green`: the maintained complete Windows runner passed native DX12
  output, browser WebGPU/WebGL2 output toggles and transfer, Q01 full-frame
  parity, and the independent artifact validator for exact candidate
  `4e84ccf05ad7d525c39e5d13091019e31a793984`.
- `full`: recorded in section 10. `counts`: zero RC pushes, one full matrix, one
  user hardware action for this candidate.

### Q06 — Make native m8 WaterBottle release evidence full-frame

- [x] Pin the existing macOS command as the focused baseline and prove it writes
  `release_evidence:true` with `metrics.reference_diff:"not-claimed"` when
  `SCENA_REFERENCE_DIFF` is absent.
- [x] Add a horizontal mirror/asymmetric-region mutation that passes the seven
  point samples plus histograms but fails a full-frame/asymmetry oracle.
- [x] Require the approved full-frame reference diff in every workflow that
  consumes m8 as release evidence, or emit `release_evidence:false` with a
  precise missing-evidence code.
- [x] Keep browser Q01 triangle parity as its own rigorous, narrow oracle; do
  not use it as evidence for native lit/textured/mipped/tonemapped WaterBottle.
- [x] Produce native frame, reference, heatmap/diff, worst-region bbox, adapter
  key, thresholds, and reference provenance.
- [x] Acceptance: sparse samples/histograms can remain diagnostics but cannot
  independently certify native m8 release output.

Validation ledger:

- `focused red`: source audit pinned the old macOS command and confirmed that
  it unconditionally wrote release evidence while the full-frame branch was
  optional. The new pure oracle test initially failed to compile because the
  full-frame evaluator/mirror owner did not exist.
- `classification`: proof-integrity defect. The rigorous browser Q01 unlit
  triangle oracle is intentionally unchanged and does not certify the native
  lit/textured WaterBottle path.
- `implementation`: the native producer now emits diagnostic-only
  `FULL_FRAME_REFERENCE_DIFF_NOT_RUN` evidence unless the complete 512x512
  reference oracle ran and rejected a horizontal mirror. CI/release workflows
  set `SCENA_REFERENCE_DIFF=1`; staging requires the render/reference/diff
  hashes, thresholds, worst-region bbox, structured adapter key, and mutation
  result. The Windows complete-hardware bundle runs the same test on DX12 and
  independently validates its artifacts.
- `focused green`: remote pure mirror-oracle test, release finalizer test,
  canonical staging mutation test, doctor mutation test, and Node Windows
  bundle-validator mutations passed.
- `scoped`: Rust formatting and the focused compilation surface passed. The
  release-gate, headless-rendering, contributor, and reference metadata docs
  now describe the fail-closed contract.
- `full`: deferred to section 10. `skipped`: the real Metal/DX12 rerender is a
  final exact-candidate hardware gate; it is not simulated on the CPU builder.
- `counts`: about 40 minutes; one test-harness correction after the diff became
  a required artifact; zero RC pushes; zero full matrices; zero user actions.

### Q07 — Prove anti-aliasing effect, especially MSAA, at pixel level

- [x] Preserve the existing FXAA hard-edge and m2 on/off pixel tests.
- [x] Add a deterministic diagonal/high-contrast geometry fixture for None,
  FXAA, MSAA4, and supported MSAA8.
- [x] Assert intermediate-luma boundary coverage and lower alias/edge-energy
  metric versus None; output difference or timing alone is insufficient.
- [x] Run native/browser variants where the backend claims the AA mode and
  record explicit degradation where it cannot.
- [x] Add known-bad no-op AA and blur-everything mutations.
- [x] Acceptance: every advertised AA mode has an effect-specific pixel oracle,
  not only lifecycle/resource evidence.

Validation ledger:

- `focused red`: the new native diagonal proof initially failed to compile on
  an untyped edge-energy accumulator, then the release staging mutation showed
  the result lacked producer/source-checksum provenance. Both failures were
  classified and corrected without broad-suite reruns.
- `implementation`: a pinned asymmetric diagonal measures intermediate-luma
  coverage, hard transitions, squared edge energy, contrast, and edge-local
  spread for None/FXAA/MSAA4 and supported MSAA8. Unsupported MSAA8 is explicit
  structured degradation. PF01 WebGPU/WebGL2 captures now record normalized
  edge metrics and reject hash-only FXAA evidence. Native and browser oracles
  reject no-op and whole-frame-blur mutations. Release staging validates the
  metrics, hardware adapter, exact source, and frame checksums.
- `focused green`: remote synthetic diagonal mutation test, PF01 evaluator
  test, Windows complete-proof validator test, canonical release-staging
  mutation test, and doctor mutation test passed.
- `scoped`: remote `cargo fmt --all --check` passed. Metal workflows and the
  exact-candidate Windows runner own the live native invocation; browser PF01
  owns the required live WebGPU/WebGL2 invocation.
- `second frozen Windows observation`: commit
  `f7e67ee2ccf41007c877a393a3e7535dacaf0257` passed the browser WebGPU/WebGL2
  proofs, attached DX12 surface proof, native FR06, resource lifecycle,
  shader-cache distribution, and WaterBottle full-frame proof before Q07
  rejected legitimate FXAA coverage: 4,831 intermediate pixels exceeded an
  incorrectly absolute 4,574 ceiling. The ceiling used
  `max(baseline + 20, hard_edges * 6)` even though the baseline already
  contained 4,554 intermediate-tone interior pixels. This is classified as a
  test-harness defect. The final archive upload then failed independently
  because the Windows host could not resolve `filebin.net`; the complete
  console failure was retained.
- `focused repair`: a captured-metric regression failed under the old formula.
  The producer and both independent release consumers now bound growth by
  `baseline_intermediate + hard_edges * 6`, while the existing no-op and
  whole-frame-blur mutations remain rejected. The focused Rust oracle, Windows
  archive validator, canonical staging validator, and doctor mutation are
  green.
- `third frozen Windows observation`: commit
  `9da1e44269df22acc5306ae38862e12535daebc3` again passed the browser
  WebGPU/WebGL2 proofs, attached DX12 surface proof, native FR06, resource
  lifecycle, shader-cache distribution, and WaterBottle full-frame proof.
  Q07 rendered and uploaded all four frames. Recomputed archive metrics for
  None/FXAA/MSAA4/MSAA8 were respectively
  `4554/384/22118400`, `4831/0/9642940`, `4667/129/14671152`, and
  `4675/152/14202618` for intermediate pixels/hard transitions/squared edge
  energy, so every mode satisfied the corrected oracle. The executable then
  exited before writing `result.json` because its runtime source checksum read
  required `tests/q07_antialiasing_effect.rs`, which the one-shot bundle had
  omitted. This is a test-harness packaging defect, not a renderer or pixel
  oracle failure.
- `focused packaging repair`: the bundle-source contract test failed against
  the omission, then passed after the builder copied the Q07 producer source
  beside the packaged tests. Doctor now pins that runtime dependency and its
  omission mutation passes.
- `fourth frozen Windows observation`: commit
  `88e21cdce78388b849ed148f6f0102ad2b6efb0b` contained the manifest-bound Q07
  source in the downloaded ZIP, but the PowerShell runner installed a
  hard-coded subset of bundle files and omitted that source before calling
  `Assert-ManifestAtRoot`. The run therefore failed before executing any proof
  with `Installed proof workspace is missing manifest file:
  tests/q07_antialiasing_effect.rs`. The package/install contract test now
  fails if either half is omitted, the runner explicitly installs the source,
  and doctor rejects both package and installer omission mutations. A fresh
  exact-candidate hardware run remains mandatory.
- `fifth frozen Windows confirmation`: commit
  `49bc86b7df50d6221fe022f223512cafef555901` installed all manifest files and
  passed Q07 with a complete `result.json` before the later Q08 failure below.
  The next exact candidate must still repeat the maintained one-shot run
  because its manifest changes.
- `full`: deferred to section 10. `skipped`: live GPU pixels are deferred to the
  frozen Metal/Windows/browser hardware checkpoint; the CPU builder cannot mint
  this evidence.
- `counts`: about 75 minutes; one compile correction, one provenance-contract
  correction, one `/tmp` environment retry, one Windows-derived oracle
  correction, one Windows bundle-content correction, one Windows
  bundle-installer correction; zero RC pushes, one completed CPU full chain,
  five complete Windows attempts, five user actions.

### Q08 — Require transmission/clipping/dynamic parity on intended hardware lanes

- [x] Inventory parity tests using `require_cpu_gpu_parity_adapter_or_skip` and
  map each claimed backend/workflow to a required command.
- [x] Prove ordinary macOS/Windows all-target tests return before assertions
  when no forcing variable/lavapipe path is present.
- [x] Add explicit strict invocations on real hardware or emit structured
  non-release artifacts that the release consumer rejects.
- [x] Keep lavapipe as CPU-hosted GPU-path conformance, not a substitute for
  physical Metal/DX12/browser evidence.
- [x] Acceptance: every required parity artifact records executed assertions,
  adapter/backend, exact commit, and cannot be green after an early return.

Validation ledger (Q08):

- `focused`: the pure `q08_required_lane_policy` tests prove ordinary no-adapter runs
  select a diagnostic skip while strict mode cannot downgrade; all six parity
  targets compiled and then passed together on lavapipe with nonzero assertion
  counts of 5/8/10/12/12/24. Their generated artifacts recorded Vulkan/CPU,
  `diagnostic-gpu-conformance`, and `release_evidence:false`. The canonical
  release-staging mutation rejected a zero-assertion result with
  `RELEASE-PHYSICAL-PARITY`; the doctor mutation and Windows complete-proof
  validator mutation also passed.
- `test-first exception`: the shared parity producer refactor had already been
  partially staged during continuation of the audit before the isolated remote
  runner was restored. Closure therefore uses the closest deterministic proof:
  pure policy tests plus in-test zero-assertion mutations of both independent
  release consumers. No claim is made that a pre-patch red run was captured.
- `scoped`: remote compilation covered transmission, clipping, both dynamic
  transform tests, PBR, and PF08; remote `cargo fmt --all --check` passed. The
  macOS workflows and Windows one-shot runner now invoke each exact test with
  `SCENA_REQUIRE_GPU_PARITY=1`; staging and the archive validator require a
  physical adapter, exact commit, source checksum, and executed assertions.
- `first frozen Windows physical-parity observation`: commit
  `49bc86b7df50d6221fe022f223512cafef555901` passed every preceding hardware
  gate, then rendered all four transmission CPU/GPU pairs. Their RMSE values
  were `0.04980/0.04142/0.03860/0.01885`, mean channel deltas were
  `2.31863/2.04841/2.50919/1.63725`, and both CPU/GPU Sobel energies remained
  above the required `0.010`; the pixel oracle therefore passed. The
  finalizer then failed while checksumming the omitted runtime producer source
  `tests/transmission_parity.rs`. The same finalizer owns all six Q08 commands,
  so the repair packages all five Q08 test-source files, installs all bundled
  `tests/*.rs`, and pins the complete package/install set in focused and doctor
  omission tests rather than advancing one source at a time. The complete
  remaining-executable audit also found PBR and PF08 artifact paths compiled
  from the Linux builder's `CARGO_MANIFEST_DIR`; focused portability tests and
  doctor mutations now require runtime-workspace-relative `target/` paths
  before another Windows run.
- `full`: deferred to section 10. `skipped`: live Metal/DX12 evidence is
  intentionally deferred to the frozen exact-candidate hardware checkpoint;
  lavapipe cannot satisfy it.
- `counts`: about 45 minutes; one formatting correction, one test-fixture
  visibility compile correction, one Windows provenance-family correction,
  one pre-rerun cross-builder path correction, zero release-candidate pushes,
  zero full matrices, one user hardware action.

### Q09 — Replace adapter-display-string tolerance policy

- [x] Capture current macOS adapter/backend/device/driver facts structurally.
- [x] Replace substring detection of `Apple Paravirtual device`/`Metal` with a
  versioned adapter expectation key and reviewed expected samples/thresholds.
- [x] Target the ordinary tolerance of 25; any exception needs measured frames,
  owner, expiry, and a separate expected-value set—not a global widening.
- [x] Add mutation tests for renamed/empty adapter strings and unknown devices.
- [x] Acceptance: free-form adapter display text cannot silently change visual
  acceptance policy.

Validation ledger (Q09):

- `focused`: before implementation, the two `waterbottle_adapter_*` tests failed
  to compile because `waterbottle_region_profile` did not exist. After the
  patch they passed for renamed, empty, spoofed, and unknown display-name cases
  and pinned every reviewed region to Chebyshev 25. Retained macOS workflow run
  `29918670156` at audited commit `a28f2149` supplied the structured
  Metal/0/0/IntegratedGpu/empty-driver facts and measured samples from PNG
  SHA-256 `2239bbb25313877e32dd5431fdae14660608257a4c11c60c383804fecbf6285f`.
- `scoped`: canonical release staging rejected an unknown adapter profile; the
  WaterBottle result finalizer, Q09 doctor mutation, and Windows complete-proof
  validator mutation passed. The producer, staging consumer, Windows consumer,
  reference metadata, and release-gate docs all bind the versioned structured
  profile; the human-readable adapter name is diagnostic only.
- `first frozen Windows observation`: the maintained complete proof for commit
  `685557327576046a4bce87e0de5bfdff08533dfc` passed every browser/native step
  through the controlled shader-cache distribution, then the WaterBottle lane
  rejected the Intel Arc Vulkan frame at the obsolete portable
  `body_olive_mid` sparse sample. Independent comparison of the captured frame
  against the pinned reference found 99.6128% of pixels within RGB Chebyshev
  16 (95% required; 1,015 outliers of 262,144; max channel delta 31), proving
  the full-frame oracle passed while the older diagnostic contradicted it.
  This is classified as an oracle defect, not accepted renderer drift.
- `focused repair`: a new deterministic regression failed because the portable
  mid-body expectation was 25 levels from its own pinned reference and
  therefore left zero headroom for the full-frame tolerance. The portable
  expectation now uses the referenced pixel `[163,143,53]`, retains
  Chebyshev 25, and a permanent invariant requires every portable sparse sample
  to leave all 16 levels of full-frame tolerance headroom. Producer, Rust
  staging consumer, Windows validator, fixture, and reference metadata use the
  same reviewed 2026-07-23 profile.
- `Windows backend repair`: the first archive proved that every native artifact
  used Intel Arc through Vulkan even though the maintained runner exported
  `WGPU_BACKEND=dx12`. The first repair applied
  `wgpu::Backends::all().with_env()` to attached-surface instance construction,
  and the second frozen run proved that surface path selected DX12. It also
  exposed a separate `Instance::default()` in `request_headless_gpu`: the
  WaterBottle diagnostic still selected Vulkan. The headless constructor now
  uses the same filtered `instance_for_backend` path, a focused regression
  pins that call site, and doctor rejects removal of either binding. The strict
  Windows archive validator now requires DX12 for every structured native
  adapter artifact; its policy was not weakened.
- `full`: deferred to section 10. `skipped`: a fresh Metal frame is deferred to
  the frozen exact-candidate hardware checkpoint; the reviewed baseline frame
  is retained as profile evidence and the final Metal lane must reproduce it.
- `counts`: about 100 minutes; one rejected weak test, one doctor-helper compile
  correction, one formatting correction, three focused remediation attempts,
  zero release-candidate pushes, two complete Windows attempts, two user
  actions.

### Q10 — Add real rerendered known-bad mutations

- [x] Keep cheap post-hoc pixel mutations as oracle sensitivity tests.
- [x] Add at least one wrong-camera and one wrong-material scene mutation that
  reruns prepare/render and is rejected by the same WaterBottle oracle.
- [x] Ensure mutations exercise importer, resources, shader/CPU material path,
  camera, tone mapping, and output transfer as applicable.
- [x] Record mutation artifact hashes and failure metrics.
- [x] Acceptance: the oracle rejects both synthetic output corruption and real
  renderer-state regressions.

Validation ledger (Q10):

- `focused`: before implementation, the exact Q01 WaterBottle test failed to
  compile because `render_wrong_material_scene` and
  `render_wrong_camera_scene` did not exist. After implementation, the
  always-on 256x256 CPU proof passed in 16.5 seconds. The fresh wrong-material
  render was rejected at RGB RMSE `82.60` with artifact SHA-256
  `b6406fb5e2a2195670cfffd4876eb054d6c8665905a391b320f7e7501b58cb6d`;
  the fresh wrong-camera render was rejected at RMSE `89.64` with SHA-256
  `bb6f0b053bddf7d7ce2899a08fe4b784038acaa7e3727e86064b5e489a23fb18`.
- `scoped`: the CPU result finalizer rejected a wrong-material result relabeled
  `post-hoc-pixel`; canonical release staging rejected the equivalent
  wrong-camera lie. The Q10 doctor mutation and the existing Q01 doctor command
  mutation passed. Each rendered mutation records a fresh import, loaded
  resources, the scene-state change before prepare, CPU material or active
  camera path, render, PBR-neutral tonemapping, and sRGB8 output. The retained
  flattened-chrome mutation remains explicitly `post-hoc-pixel`.
- `full`: deferred to section 10. `skipped`: no extra GPU rerender was added;
  this finding concerned Q01's falsely described CPU mutations, and the CPU
  path deterministically exercises the applicable importer/resource/material/
  camera/output contracts without spending two additional native GPU frames.
- `counts`: about 35 minutes; one staging-fixture ordering correction, one
  formatting correction, zero release-candidate pushes, zero full matrices,
  zero user hardware actions.

### Q11 — Govern reference regeneration and cross-architecture stability

- [x] Add repeated in-process deterministic renders and compare bytes/metrics
  before consulting the committed reference.
- [x] Record x86_64 Linux, arm64 macOS, Windows, and relevant backend metric
  distributions for the same exact source/reference.
- [x] Define reference regeneration command, clean environment, generator
  version, source asset hash, reviewer/approval, before/after diff, and external
  anchor requirements.
- [x] Do not loosen the shared Chebyshev/RMSE tolerance without this evidence;
  use per-architecture references only if stable measured differences require
  them.
- [x] Align `reference_metadata.toml` claims with the workflows that actually
  run the diff.
- [x] Acceptance: a code change cannot regenerate its own oracle and certify
  itself without an independent ratchet.

Validation ledger (Q11):

- `focused red`: the exact Q11 test first failed to compile because the new
  independent `render_baseline_repeat` contract had no implementation. The
  release-staging mutation then demonstrated that a non-deterministic Q11 JSON
  was initially copied without semantic validation; staging was changed to
  reject it explicitly.
- `focused green`: two fresh Linux x86_64 WaterBottle renders were byte
  identical (RGBA SHA-256
  `7494647775d8cb30bbc989530d3a903f931b7e97d20030d20acb94eef5e3a5be`),
  and each independently measured `0.999664306640625` within Chebyshev 4 with
  RGB RMSE `0.2009220485570316`. The exact Q01 test also passed with its new
  pre-reference repeat. The canonical staging mutation, Q11 doctor mutation,
  and Windows complete-proof validator mutation passed.
- `implementation`: CI/release and the exact-candidate Windows bundle run the
  same Q11 test and require separate Linux x86_64, macOS aarch64, and Windows
  x86_64 metric artifacts. Candidate generation is clean-checkout-only,
  non-release, target-local, and cannot overwrite the reference. Promotion
  requires a separately authored named-human approval binding the candidate,
  prior reference, diff heatmap, generator commit, and Blender anchor; it
  refuses threshold changes.
- `scoped`: remote `cargo fmt --all` passed. One first validator invocation was
  classified as an environment failure because `/tmp` returned quota error
  `-122`; the same test passed unchanged with the required task-local `TMPDIR`.
  No full suite was run because Q11 is a proof/governance slice and the frozen
  final checkpoint owns the single full chain.
- `full`: deferred to section 10. `skipped`: live macOS arm64 and Windows
  x86_64 records are deferred to their exact-candidate lanes; the CPU builder
  cannot mint either platform identity.
- `counts`: about 70 minutes; two staging/harness corrections, one environment
  retry, zero release-candidate pushes, zero full matrices, zero user hardware
  actions.

### Q12 — Replace brittle/fail-open doctor contracts with semantic ownership

- [x] Remove redundant literal `MATERIAL_UNIFORM_BYTE_LEN = 224` pins after a
  single layout/encode/bind-size semantic test and mutation own the invariant.
- [x] Add current v1.9.0 review/checklist/release docs to required-document
  governance and derive versioned release-note references from one source.
- [x] Make missing generated artifacts fail only in the explicit release mode
  that claims them; ordinary source doctor may report unavailable but must not
  synthesize evidence.
- [x] Complete Q01's generated WGSL variant manifest, then remove only shader
  substring pins made redundant by parse/validation plus semantic mutations.
- [x] Add known-bad missing-doc, missing-release-artifact, stale-version,
  uniform-layout, and omitted-shader-variant fixtures.
- [x] Acceptance: doctor fails closed on claimed evidence and does not encode
  implementation trivia already proven by the compiler/semantic tests.

Validation ledger (Q12):

- `focused red`: the material-uniform contract initially failed to compile
  because layout and binding code had no shared semantic size owner. The Q12
  doctor mutation then caught its own test literal and a wrapped documentation
  phrase; those harness defects were isolated before changing the production
  checks.
- `focused green`: all eight material-uniform tests passed, including Naga
  layout reflection and an omitted-lane mutation; all four production-derived
  shader-manifest tests passed, including an omitted-variant mutation. The Q12
  doctor tests passed with stale-version and missing-current-document fixtures,
  as did the renderer-truth ownership contract.
- `implementation`: material layout, encode length, and bind range now share one
  typed size function. Nineteen production WGSL variants are enumerated from
  the renderer-owned manifest and parsed/validated with Naga; the doctor no
  longer duplicates raw shader substrings or the numeric uniform size. Current
  release notes, review report, and remediation checklist are owned by one
  v1.9.0 constant set. Generated WASM absence is non-evidence in ordinary
  source doctor and a hard error only when explicit release-artifact mode is
  selected by lanes that actually build those artifacts.
- `scoped`: remote `cargo fmt --all` passed. Focused xtask Q12, renderer-truth,
  material-uniform, shader-manifest, and required-document tests passed. No
  broad suite was run because the frozen final checkpoint owns the single full
  chain.
- `full`: deferred to section 10. `skipped`: live generated-artifact validation
  remains in the final browser/release lanes, where the artifacts exist; source
  doctor intentionally cannot claim them.
- `counts`: about 45 minutes; two focused harness corrections, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

## 4. CLI and agent on-ramp

### A01 — Introduce a typed CLI error and exit taxonomy

- [x] Inventory every command's input, policy, I/O, asset parse, unsupported
  feature/capability, backend, inequality, internal, and interrupted errors.
- [x] Replace top-level `Err(String)` classification with a structured error
  carrying stable code, exit class, message, path/context, help, and optional
  candidates/fix.
- [x] Preserve versioned JSON envelopes on stdout/stderr as documented.
- [x] Ensure runtime failures no longer report `invalid_arguments`.
- [x] Define and test broken pipe behavior without panic.
- [x] Add per-command table tests for code/schema/exit status.
- [x] Update `docs/errors.md`, CLI help, agent guide, and schema catalog.
- [x] Acceptance: automation can distinguish repairable input, policy, runtime,
  comparison inequality, and internal failures without parsing prose.

Validation ledger (A01):

- `focused red`: the new real-CLI taxonomy test observed no `exit_class` on an
  unknown command and observed a feature/runtime failure incorrectly exiting 2
  as `invalid_arguments`.
- `focused green`: three real dispatch classes now emit typed
  `scena.cli_error.v1` reports; unknown schema and agent-template lookup tests
  use input/65 with candidates; feature/runtime failures are not usage errors.
  Every machine-help command declares error schemas and known exit classes.
- `implementation`: the top-level boundary now owns a typed `CliError` with
  stable code/class/status, message, optional path, command context, help,
  candidates, and optional fix. The shared writer preserves quiet BrokenPipe
  success and adds io/74 fields to non-BrokenPipe output errors. Machine help,
  stable fixture, schema docs, error docs, and the agent guide share the same
  nine-class taxonomy; a doctor mutation prevents its removal.
- `scoped`: the A01 integration tests, typed-error unit tests, name-candidate
  integration test under `agent`, CLI output/BrokenPipe contracts, stable JSON
  fixture test, doctor mutation, and remote formatting passed.
- `full`: deferred to section 10. `skipped`: unrelated CLI workflows and the
  full feature matrix remain for the frozen checkpoint; the focused command
  table covers their declared error surface without rendering assets.
- `counts`: about 55 minutes; one classifier correction for the exact template
  error wording, zero release-candidate pushes, zero full matrices, zero user
  hardware actions.

### A02 — Make the complete recipe field model authoritative

- [x] Enumerate every accepted recipe path, type, required/default status,
  enum, numeric range, feature requirement, and cross-field constraint.
- [x] Generate field-model output from validation/authoring metadata or enforce
  exhaustive bidirectional parity in tests.
- [x] Cover camera/framing, lighting/environment, materials/textures, animation,
  bloom/SSAO/SSR/DoF, output/capture, imports, placement, and policies promoted
  by public guides.
- [x] Add a known-bad omission test.
- [x] Version the schema if shape changes and update golden pins/examples.
- [x] Acceptance: every public guide field is discoverable before an agent
  attempts a build, and every advertised field is accepted under its stated
  feature/capability conditions.

Validation ledger (A02):

- `focused red`: the new field-model test failed to compile because no recipe
  JSON Schema or path-parity surface existed. The first generated-schema run
  then exposed a test-harness mistake: auto exposure is an untagged preset
  string or `{preset,...}` object, not a fictitious `kind` field.
- `focused green`: the generated model contains more than 350 accepted paths
  and covers promoted import/transform, material/texture, LOD, animation,
  camera/framing, lighting/environment, bloom/SSAO/SSR/DoF/exposure, and capture
  fields. Its path set is byte-for-byte equal to the `SceneRecipeV1` JSON-Schema
  traversal, and deleting the SSR property makes parity fail.
- `implementation`: all recipe input types derive JSON Schema from the same
  serde definitions used for decoding. The field model is generated from that
  graph, then enriched—never expanded—by validation-owned enum/range/default,
  feature, owner, and cross-field metadata. The new metadata members are
  additive v1 fields with serde defaults, so no incompatible envelope shape
  change or version bump was required; the stable fixture documents them.
- `scoped`: A02 path/coverage/known-bad tests, the existing live schema-get
  field-model contract, round-trip/invalid fixture test, doctor mutation, and
  remote formatting passed.
- `full`: deferred to section 10. `skipped`: the complete schema catalog suite
  and packaged CLI matrix remain at the frozen checkpoint; no runtime recipe
  execution behavior changed in this slice.
- `counts`: about 65 minutes; one test-harness correction, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

### A03 — Make canonical guide commands executable from a clean directory

- [x] Fix the missing `target/scena-agent` directory creation before shell
  redirection.
- [x] Use one consistent `primitive-scene` output path; remove the accidental
  underscore variant.
- [x] Extract and execute canonical shell blocks in a clean temporary CWD using
  packaged/public assets only.
- [x] Test both repository install and documented `cargo install` workflow.
- [x] Assert expected JSON schema, files, image dimensions/nonblack result, and
  exit status rather than command success alone.
- [x] Add doctor coverage that prevents canonical command drift.
- [x] Acceptance: a new user can copy the documented sequence verbatim.

Validation ledger (A03):

- `focused red`: the extracted canonical guide block failed because the output
  directory was not created and the documented template path alternated
  between `primitive-scene` and `primitive_scene`.
- `focused green`: the exact marked shell block passes from a clean temporary
  directory with both the repository `agent` binary and the release-mode
  binary installed from the current `.crate` archive. It asserts six output
  schemas, successful validation/build/render reports, a decodable image, and
  nonblack pixels.
- `implementation`: the guide creates its output directory, uses one canonical
  template path, and exposes stable block markers consumed by the smoke test.
  The feature-discoverability doctor check pins the markers, command spelling,
  packaged test, and known-bad underscore mutation.
- `scoped`: repository and packaged guide smokes, A05 contract integration,
  doctor mutation, and remote formatting passed.
- `full`: deferred to section 10. `skipped`: unrelated guide examples and the
  full CLI matrix remain for the frozen checkpoint.
- `counts`: about 35 minutes; two identical package-extraction path harness
  failures were stopped and resolved with one archive-layout probe, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

### A04 — Decide and enforce the installed CLI feature contract

- [x] Record size/build-time/API tradeoffs for: default `agent` feature,
  always-enabled CLI-only dependencies, or a separate `scena-cli` package.
- [x] Select one owner-approved contract; do not silently change library default
  features as an incidental fix.
- [x] Make `cargo install scena` behavior, `--features agent`, recovery hints,
  README, crates.io metadata, and CI installation test agree.
- [x] Add a clean packaged-crate smoke test for help, schema discovery,
  validate, render, inspect, diagnose, repair, and examples discovery.
- [x] Acceptance: the executable's default installation behavior is explicit,
  tested, and every unavailable command gives one correct installation remedy.

Validation ledger (A04):

- `focused red`: the packaged default binary proved that render, inspect,
  diagnose, and repair named the internal `inspection` feature instead of the
  documented installable `agent` composition.
- `focused green`: release-mode binaries installed from the current package
  archive pass both contracts outside the repository. The default install
  exposes core discovery/validation and returns typed unsupported/69 reports
  with exactly `cargo install scena --features agent`; the agent install runs
  template discovery, render, inspect, diagnose, doctor, and repair.
- `implementation`: package metadata and the dedicated install-contract spec
  preserve empty library defaults and document why a default agent feature,
  binary-only feature, or second CLI package was not selected. README,
  getting-started guidance, help, and all command fallbacks now agree.
- `scoped`: both packaged install modes, CLI taxonomy/help evidence, the
  feature-discoverability mutation, and remote formatting passed.
- `full`: deferred to section 10. `skipped`: publish dry-run and the complete
  feature matrix remain for the frozen release checkpoint.
- `counts`: about 25 minutes plus eight minutes of optimized package builds;
  one product remedy correction, zero release-candidate pushes, zero full
  matrices, zero user hardware actions.

### A05 — Package/discover the application-builder guidance

- [x] Keep root `AGENTS.md` as contributor-only repository governance.
- [x] Add a public `scena guide agent --json|--markdown` or equivalent export
  for the application-builder workflow, schemas, commands, and policies.
- [x] Ensure packaged crate/installed CLI contains the source of that guidance;
  do not depend on untracked/private `.codex` files.
- [x] Add `examples agent list` and machine-readable template metadata if not
  already available in the selected surface.
- [x] Test from a temp CWD outside the repository.
- [x] Acceptance: an application-building agent never needs the private builder
  or contributor instructions to discover public scena usage.

Validation ledger (A05):

- `focused red`: a real CLI test from a clean directory failed with unknown
  command for `guide agent`.
- `focused green`: both default and agent binaries installed from the current
  package emit `scena.agent_guide.v1` JSON and the embedded Markdown guide from
  outside the checkout. The schema catalog, stable fixture, help contract, and
  feature-gated real-CLI evidence matrix all execute and pass.
- `implementation`: a library-owned, versioned agent-guide contract embeds the
  public guide and structured command/schema/policy/template indexes. The CLI
  exports JSON or explicit raw Markdown; existing `examples agent list`
  remains the live machine-readable template inventory. Private `.codex` and
  contributor governance are not runtime dependencies.
- `scoped`: A05 package smokes in both install modes, schema catalog golden,
  stable fixture validation, CLI output-evidence and argument-error fixtures,
  doctor mutation, and remote formatting passed.
- `full`: deferred to section 10. `skipped`: all-schema and documentation suites
  remain for the frozen checkpoint.
- `counts`: about 30 minutes; one deterministic catalog-order fixture correction
  and one zero-test selector rerun with the required features, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

### A06 — Provide one fail-closed pre-render health profile

- [x] Design `scena lint` as a versioned composition/profile over existing
  schema validation, recipe resolution, capability planning, and scene
  diagnostics; do not fork their rules.
- [x] Define offline versus backend/live checks and disclose skipped evidence.
- [x] Include missing camera/lighting, invisible scene, unresolved assets,
  sandbox/policy, invalid animation/camera, unsupported features, and suggested
  fixes/candidates.
- [x] Keep lint optional to this remediation release unless required to expose a
  fixed defect; its design must not delay C/Q/A closure.
- [x] Release selection: implementation/acceptance is explicitly deferred; the
  v1.9.0 CLI and docs make no `scena lint` availability claim.

Validation ledger (A06):

- `focused`: design review found no corrected C/Q/A defect that requires a new
  lint command; existing owner validators remain the tested release surface.
- `implementation`: `docs/specs/lint-profile-v1.md` defines the future stable
  composition envelope, offline/live evidence policy, required finding
  families, fail-closed skipped checks, and implementation tests without
  forking rules or hiding a render.
- `scoped`: documentation linkage and explicit no-command/no-claim selection;
  link and doctor checks are deferred to the grouped documentation gate.
- `full`: deferred to section 10. `skipped`: command/schema implementation is
  an optional feature and is not selected for v1.9.0.
- `counts`: about 10 minutes; zero remediation attempts, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

### A07 — Generate complete preset vocabularies

- [x] Inventory material, lens, framing, named color, light, scene,
  environment, quality, tonemapper, easing, and placement preset registries.
- [x] Generate `scena.vocab.v1` entries from their authoritative definitions or
  enforce exact parity with them.
- [x] Include feature/capability requirements, aliases/deprecations, and owner.
- [x] Add a known-bad omitted-preset mutation.
- [x] Acceptance: every preset promoted by public guides is discoverable without
  provoking a validation error.

Validation ledger (A07):

- `focused red`: the new parity target failed to compile because
  `VocabularyV1` had no value metadata or omission validator, and the live
  report contained none of the material/lens/framing/color/environment/
  exposure/scene/quality/tonemapper/easing/light preset registries.
- `focused green`: default and `scene-host` runs prove exact set parity with
  public material, lens, framing, named-color, environment, exposure, and scene
  registries plus validation-owned render and per-light-kind constants. Removing
  `chrome` from a report is rejected by the known-bad invariant.
- `implementation`: the additive v1 `entries` metadata records aliases,
  deprecation, feature, and capability requirements while preserving the
  existing ordered `values`. Scene/light constants are shared with validators;
  placement uses the exported placement authority. Docs and the stable fixture
  describe the complete inventory.
- `scoped`: A07 default and scene-host parity tests, live CLI vocabulary test,
  stable fixture test, and executed doctor mutation passed.
- `full`: deferred to section 10. `skipped`: unrelated schema and CLI suites
  remain for the frozen checkpoint.
- `counts`: about 35 minutes; zero post-implementation remediation attempts,
  zero release-candidate pushes, zero full matrices, zero user hardware
  actions.

### A08 — Make render introspection the default

- [x] Add CLI tests showing render without `--introspect` currently returns a
  usage error.
- [x] Emit the versioned introspection/result envelope by default.
- [x] Keep `--introspect` accepted as a compatibility no-op with a migration
  note; provide an explicit human/minimal mode only if a real use case exists.
- [x] Update command help, templates, guides, and golden outputs.
- [x] Acceptance: the safest machine-verifiable output requires no ceremony.

Validation ledger (A08):

- `focused red`: the real `render` command without `--introspect` returned a
  typed usage error and exit 2 before loading the valid recipe.
- `focused green`: both `render` and `recipe render` now succeed without the
  flag, write PNGs, and emit `scena.render_introspection.v1`; a second test
  proves the legacy flag remains accepted with identical envelope semantics.
- `implementation`: both parsers treat `--introspect` as a no-op and machine
  help declares introspection as the default. Generated templates, canonical
  guide block, README/getting-started/API/schema/troubleshooting text, historic
  command claims, and the agent golden omit the ceremony while documenting the
  migration.
- `scoped`: A08 red/green target, canonical guide execution, template golden,
  FR04 observed-contract/evidence tests, A01 help inventory, and the executed
  doctor mutation passed.
- `full`: deferred to section 10. `skipped`: unrelated render-quality and GPU
  lanes were not needed because frame generation/output behavior is unchanged.
- `counts`: about 30 minutes; one doctor-fixture harness correction, zero
  product remediation retries, zero release-candidate pushes, zero full
  matrices, zero user hardware actions.

### A09 — Add generic schema-based validation and JSON Schema export

- [x] Add `scena validate <file>` dispatching on the embedded versioned `schema`
  field with nearest-schema suggestions.
- [x] Reuse expectation/recipe/patch validators; do not duplicate constraints in
  the dispatcher.
- [x] Export versioned JSON Schema with declared limits where JSON Schema cannot
  express runtime/cross-field semantics.
- [x] Cover unknown schema, malformed JSON, schema mismatch, expectations,
  recipes, patches, and capability reports.
- [x] Acceptance: agents can validate any public contract before invoking its
  consuming workflow.

Validation ledger (A09):

- `focused red`: the new real-CLI target first proved that `validate` and
  `schema json` were absent. A separate known-bad recipe-patch mutation then
  showed that a malformed source digest passed before the owner gained and the
  dispatcher reused its invariant validator.
- `focused green`: four tests now cover typed recipe, appearance expectation,
  interaction expectation, recipe patch, and capability validation; malformed
  JSON, unknown schema suggestions, typed mismatch, patch-owner invariants, and
  generated recipe JSON Schema all pass with the declared exit/envelope rules.
- `implementation`: cataloged contracts dispatch to their owner validator when
  its feature is compiled and otherwise receive explicit envelope-only
  validation. Recipe JSON Schema comes from the serde/schemars owner type;
  other catalog entries export only a draft 2020-12 envelope plus limitations,
  never invented field constraints. Help, README, getting started, the packaged
  agent guide, schema docs, stable fixtures, catalog goldens, and doctor pins
  expose the same contract.
- `scoped`: A09 real-CLI tests, schema CLI/catalog golden, FR04 observed output
  and evidence matrix, default-feature A01 taxonomy, 62 stable-contract tests,
  canonical clean-directory guide execution, and the executed doctor mutation
  passed. Formatting was applied remotely; `git diff --check` is clean.
- `full`: deferred to section 10. `skipped`: renderer/browser/GPU lanes are not
  implicated because this slice changes contract parsing and CLI discovery
  only.
- `counts`: about 50 minutes; two implementation corrections (owned schema
  lifetime and default-feature gating), two test-invocation corrections, zero
  release-candidate pushes, zero full matrices, zero user hardware actions.

### A10 — Publish one complete CLI contract and exit table

- [x] Generate/document every command's success/failure schema, stdout/stderr
  mode, exit codes—including I/O 74—and feature requirements.
- [x] Tie the table to A01's typed error taxonomy and help metadata.
- [x] Add golden/doctor parity so source and docs cannot diverge.
- [x] Acceptance: automation never needs to search Rust source for process
  semantics.

Validation ledger (A10):

- `focused red`: the new real-help target showed every command row lacked
  streams, numeric exit rows, and feature requirements; the errors guide also
  lacked a complete process-contract section.
- `focused green`: every `command_contracts[]` row now carries success/error
  schemas, explicit stdout/stderr roles, applicable class/code/schema/stream
  failure rows (including I/O 74), and an explicit `[]` or `["agent"]` feature
  requirement. Core and agent-only command inventories pass independently.
- `implementation`: `failure_exits[]` is derived from A01's one typed taxonomy,
  while legacy `failure_exit_classes[]` remains for v1 compatibility. Domain
  failures remain declared stdout envelopes; dispatch/runtime CLI errors remain
  `scena.cli_error.v1` on stderr. `docs/errors.md` documents the exact machine
  contract and the one-step install feature.
- `scoped`: A10 real-help tests, deterministic command-table SHA-256 golden,
  A01 default-feature taxonomy, FR04 agent output/evidence matrix, and the
  executed doctor mutation passed. Remote formatting was applied and local
  `git diff --check` is clean.
- `full`: deferred to section 10. `skipped`: renderer/browser/GPU lanes are
  unchanged by additive CLI help metadata.
- `counts`: about 25 minutes; one test-only digest-encoding correction and one
  doctor-needle correction, zero release-candidate pushes, zero full matrices,
  zero user hardware actions.

### A11 — Apply bounds placement verbs to authored nodes

- [x] Generalize place target identity to typed import or authored-node targets.
- [x] Support bounds verbs (`center`, `ground`, `fit_to_size`, `look_at`) for
  authored nodes while keeping import-only anchor/connector verbs explicit.
- [x] Return candidates for wrong/unknown target namespace.
- [x] Add starter-template place/apply examples and recipe round-trip proof.
- [x] Acceptance: the default authored templates can use the same placement loop
  the guide recommends.

Validation ledger (A11):

- `focused red`: the three-test real-CLI target showed `--node` was rejected as
  an unknown flag, authored starters contained no place/apply commands, and an
  authored node passed through `--import` produced the wrong diagnostic family.
- `focused green`: authored box nodes now preview and apply grounded transforms,
  the resulting recipe passes syntax validation, wrong/near namespace lookups
  return actionable candidates, and anchor/connector verbs remain import-only.
- `implementation`: additive typed `target:{kind,id}` metadata preserves the
  legacy `import_id` field and old-v1 deserialization. Node placement resolves
  authored primitive/mesh bounds, parent-world transforms, and writes the
  resulting local transform at `$.nodes[index].transform`. Every authored
  starter advertises preview and apply commands for a concrete node.
- `scoped`: A11 (3/3), A10 contract-table golden (4/4), FR04 output/evidence
  matrix (6/6), stable contracts (62/62), and six existing import-placement CLI
  tests passed. The contract-discovery doctor mutation rejects removal of typed
  target emission. Remote `cargo fmt --all --check` passed.
- `full`: deferred to section 10. `skipped`: renderer/browser/GPU lanes are
  unchanged; exact-candidate Windows proof remains mandatory before any push.
- `counts`: about 35 minutes; one invalid test-fixture correction and two
  intentional contract-golden updates, zero release-candidate pushes, zero full
  matrices, zero user hardware actions.

### A12 — Make JSON formatting consistent and controllable

- [x] Inventory every CLI JSON writer, including help and subprocess errors.
- [x] Choose one machine default and add global `--compact`/`--pretty` behavior
  consistently across commands.
- [x] Preserve byte-deterministic goldens for each explicit mode.
- [x] Acceptance: formatting never changes unexpectedly by command family and
  never changes envelope semantics.

Validation ledger (A12):

- `focused red`: all three real-CLI tests failed because the format flags were
  parsed as command-specific/unknown arguments, help defaulted to compact JSON,
  and CLI errors had no shared formatting policy.
- `focused green`: global flags now work before or after commands for help,
  successes, domain failures, and stderr CLI errors; default and explicit
  pretty bytes match, compact is one line, and both parse to identical values.
- `implementation`: one `CliJsonStyle` serializer owns JSON presentation;
  pretty is the compatibility default, compact/pretty are mutually exclusive,
  and float rounding composes with either style. Non-JSON guide output is not
  reformatted unless a JSON-format flag is explicitly requested.
- `scoped`: A12 (3/3), exact serializer goldens, A10 (4/4), FR04 (6/6), schema
  CLI (8/8), scena binary units (11/11), default-feature A01 (3/3), and the
  executed doctor mutation passed. Remote formatting was applied.
- `full`: deferred to section 10. `skipped`: renderer/browser/GPU lanes are not
  implicated; exact-candidate Windows proof remains mandatory before push.
- `counts`: about 30 minutes; one compile-API correction and one test-invocation
  correction, zero release-candidate pushes, zero full matrices, zero user
  hardware actions.

### A13 — Complete Rust error remedies

- [x] Add help coverage for Build, Import, Instantiate, Animation, and top-level
  `Error` delegation alongside existing Asset/Prepare/Render/Lookup help.
- [x] Decide whether `Display` remains concise; if so, expose one structured
  diagnostic adapter carrying code/message/help/context for every error.
- [x] Add exhaustive enum-variant coverage tests so a new error cannot omit a
  remedy unintentionally.
- [x] Acceptance: callers can obtain curated recovery guidance uniformly from
  any public scena error.

Validation ledger (A13):

- `test-first exception`: the exhaustive target was authored before production
  code, and baseline inspection proved the referenced methods/types did not
  exist, but the remote red invocation was accidentally skipped before the
  implementation patch. This process miss is recorded rather than claiming a
  red run that did not happen.
- `focused green`: two tests enumerate every Build/Instantiate/Animation
  variant, both Import branches, and all eight top-level Error branches; every
  path has non-empty help and a structured diagnostic whose message matches
  concise `Display`.
- `implementation`: `Display` remains concise. All eight error families expose
  `.diagnostic() -> ErrorDiagnostic { code, message, help, context }`; top-level
  `Error` delegates the exact underlying remedy and records its wrapper.
- `scoped`: A13 (2/2), the executed contract-discovery doctor mutation, and
  remote formatting passed. No renderer behavior changed.
- `full`: deferred to section 10. `skipped`: renderer/browser/GPU lanes are not
  implicated; exact-candidate Windows proof remains mandatory before push.
- `counts`: about 20 minutes; one documented test-process exception, zero
  implementation retries, zero release-candidate pushes, zero full matrices,
  zero user hardware actions.

### A14 — Add typed in-memory texture APIs and structured size diagnostics

- [x] Design checked RGBA8/linear-float in-memory texture constructors with
  dimensions, color space, sampler, mip policy, and stable cache identity.
- [x] Add slot-typed loading helpers or material-slot APIs that choose/validate
  sRGB versus linear interpretation.
- [x] Report browser downscaling through structured asset telemetry in addition
  to optional console logging.
- [x] Classify native dimension/allocation-limit failures as a dedicated error
  with actual/maximum dimensions and remedy, not generic parse.
- [x] Cover native/WASM parity and malformed/overflow inputs.
- [x] Acceptance: application-generated textures need no fake filesystem path,
  and no implicit resize/color-space choice is silent.

Validation ledger (A14):

- `focused red`: the new public-contract integration test failed with 22
  missing-type/method/error diagnostics on the pre-change API. A separate mip
  contract test then proved that a mipmap sampler was accepted while mip
  generation was disabled.
- `focused green`: five integration tests cover stable deduplication/collision,
  RGBA8 and finite HDR float input, half-float storage, slot color space,
  malformed/zero/overflow/NaN input, and explicit mip policy. Three library
  tests cover structured browser resize telemetry, dedicated native oversize
  errors, and HDR-preserving float mip generation.
- `implementation`: `TextureMemoryDesc` accepts checked RGBA8 or linear f32
  source pixels without a caller-supplied path, stores finite HDR data as
  filterable RGBA16Float, and binds immutable `TextureMemoryId` cache identity.
  `TextureSlot` drives sRGB/linear validation for generated and path-backed
  textures. Browser resize warnings flow into load reports and
  `Assets::texture_warnings`; PNG/JPEG/WebP/KTX2 limits share
  `AssetError::TextureSizeLimit`.
- `scoped`: wasm32 `scene-host` check passed; the A14 doctor mutation test
  passed; remote formatting was applied. The first native oversize fixture
  omitted PNG image data and was correctly classified as malformed; adding a
  valid IDAT made the intended size-limit proof pass (test-harness defect, one
  harness correction).
- `docs`: README capability summary, API usage/semantics, troubleshooting, and
  changelog now describe identity, slots, mip policy, limits, and telemetry.
- `full`: deferred to section 10. `skipped`: no broad cargo/browser/GPU suite
  was run; the exact frozen candidate still requires the final remote chain
  and zero-error Windows complete-hardware proof before any push.
- `counts`: about 70 minutes; one implementation integration correction, one
  test-harness correction, zero release-candidate pushes, zero full matrices,
  zero user hardware actions.

### A15 — Curate Rust imports and consolidate ergonomics deliberately

- [x] Add a small `scena::prelude` containing stable everyday scene/assets/
  render types; keep versioned schema types opt-in by module/root import.
- [x] Map native file-not-found to curated `AssetError::NotFound` with path/help.
- [x] Audit framing overloads and consolidate around `FramingOptions` without a
  breaking convenience purge unless migration evidence supports it.
- [x] Keep `controls*` compatibility aliases with an explicit no-code/alias test
  and docs, or deprecate/remove them through normal semver hygiene; do not call
  the current documented alias behavior a product defect.
- [x] Acceptance: common Rust usage is discoverable and concise while feature
  metadata remains truthful.

Validation ledger (A15):

- `focused red`: the ergonomics contract failed because `scena::prelude` and
  option-bearing node-subtree framing did not exist; the native fetch path was
  still mapped only after those compile blockers.
- `focused green`: three integration tests compile a procedural scene entirely
  from the curated prelude, frame one asset-backed subtree through
  `FramingOptions`, distinguish native NotFound, and pin `controls`,
  `controls-winit`, and `controls-web` as documented metadata-only aliases.
- `implementation`: the prelude exports stable everyday types but no versioned
  report/schema catalog. New node option methods join the existing bounds/all/
  import option family; legacy no-options conveniences remain and delegate to
  that model. Missing native files now preserve typed NotFound while other I/O
  failures remain Io.
- `scoped`: the contract-discovery mutation and feature-ownership mutation
  tests passed; remote formatting was applied. No renderer backend changed.
- `docs`: API/README explain prelude boundaries and the canonical option-bearing
  framing family; troubleshooting explains NotFound versus Io; feature docs
  and ownership metadata pin compatibility aliases; changelog is aligned.
- `full`: deferred to section 10. `skipped`: broad cargo/browser/GPU lanes are
  unchanged and intentionally deferred; final zero-error Windows proof remains
  mandatory before push.
- `counts`: about 30 minutes; zero implementation retries, zero release-
  candidate pushes, zero full matrices, zero user hardware actions.

## 5. Measure-first performance work

### P01 — Restore the retained dynamic path under culling

- [x] Benchmark a scene with visible and off-frustum objects while panning the
  camera and moving one object; record full/dynamic prepare counts, allocations,
  CPU time distribution, draw set, and output hash.
- [x] Prove the baseline coverage guard forces full prepare.
- [x] Retain a source-complete template and refresh culling/draw membership on
  the dynamic path without stale visibility.
- [x] Cover objects entering/leaving frustum, LOD, occlusion state, instances,
  transparency ordering, and removal/addition structural invalidation.
- [x] Acceptance: camera/transform-only changes use dynamic preparation when
  structurally safe and render identically to full prepare.

Validation ledger:

- `focused red`: the retained source template was populated after view-dependent
  culling, so camera re-entry hit the coverage guard and recollected full
  geometry. Classified as a product defect.
- `implementation`: full prepare now retains source-complete primitive/stroke/
  instance templates, while each dynamic encode refreshes culling and draw
  membership. Structural/material, LOD, occlusion, clipping, instance, and
  transparency safety checks still force full preparation when required.
- `focused green`: `off_frustum_source_stays_in_retained_template_across_camera_motion`
  proves enter/exit behavior and exact forced-full pixel parity;
  `off_frustum_transparency_prevents_unsafe_dynamic_reentry` pins the safe
  fallback; existing retained-path tests cover the other structural families.
- `benchmark`: remote `SCENA_USE_GPU=1 cargo test --test
  m4_performance_platform
  retained_dynamic_culling_benchmark_records_work_allocations_and_output_hash
  -- --exact --nocapture` passed and wrote the v1 report with observational
  timing, allocation count, prepare work, draw set, and FNV-1a output identity.
- `full`: deferred to section 10. One initial unlit benchmark fixture selected
  the intentional fallback; changing it to the already-proven PBR retained path
  resolved the harness mismatch. Zero RC pushes/full matrices/user actions.

### P02 — Make automatic exposure asynchronous and bounded

- [x] Benchmark current readback stalls, render count, convergence frames,
  allocation, and output sequence on controlled native hardware.
- [x] Specify exposure latency, adaptation, first-frame, capture, and
  deterministic-headless semantics.
- [x] Meter a downsampled luminance source asynchronously; avoid mandatory
  full-frame synchronous readback.
- [x] Avoid rendering the same surface frame twice; apply exposure from a prior
  completed meter sample or an explicit prepass according to the specification.
- [x] Keep capture determinism and expose pending/converged status.
- [x] Acceptance: no unconditional synchronous full-frame readback or second
  scene render in the steady surface loop, with reference-sequence parity.

Validation ledger:

- `focused red`: attached native automatic exposure selected synchronous
  full-frame readback and repeated the scene render. Classified as a product
  performance defect; deterministic headless capture semantics were retained.
- `implementation`: attached native surfaces submit one fixed 16x16 asynchronous
  meter copy (256 samples) through two reusable aligned buffers, apply a prior
  completed sample, and expose `Disabled`, `Pending`, `Converged`, or
  `Unavailable`. Work metrics count scene passes, submissions, samples,
  readbacks, and blocking waits.
- `focused green`: remote unit/integration proof passed for
  `attached_gpu_auto_exposure_uses_prior_async_meter_sample`, the existing
  deterministic renderer-managed exposure test, and compilation of
  `native_surface_hardware_proof`.
- `physical green`: exact Windows candidate
  `4e84ccf05ad7d525c39e5d13091019e31a793984` records first-frame and
  convergence durations and allocations, a pending-to-converged exposure
  sequence, exactly one scene pass, one bounded 256-sample meter submission,
  zero synchronous full-frame readbacks, and zero blocking waits.
- `full`: recorded in section 10. Zero RC pushes, one full matrix, and one user
  hardware action.

### P03 — Replace full luminance sort with bounded metering

- [x] Add synthetic luminance distributions with exact percentile references.
- [x] Implement histogram or selection over the bounded/downsampled input.
- [x] Define binning/error tolerance and outlier/highlight behavior.
- [x] Prove memory is bounded independently of output resolution.
- [x] Acceptance: exposure result stays within the specified EV tolerance while
  eliminating the O(N log N) full-frame sort.

Validation ledger:

- `focused red`: exact flat distributions accumulated enough `f32` log-sum
  error to drift, and NaN RGB was sanitized before the finite-input check.
- `implementation`: fixed-size logarithmic bins replace the full-frame vector
  and sort; the aggregate log sum uses `f64`; non-finite RGB is rejected before
  clamping. Storage is independent of output resolution.
- `focused green`: `bounded_histogram_matches_sorted_highlight_reference_within_one_bin`,
  `bounded_meter_covers_exact_flat_outlier_and_invalid_distributions`, and
  `luminance_meter_storage_is_resolution_independent` pass remotely. The tests
  pin exact flat/outlier/invalid behavior and a one-bin percentile EV bound.
- `full`: deferred to section 10; no threshold was widened.

### P04 — Hoist CPU raster reciprocal work

- [x] Add instruction/work-count or microbenchmark coverage for opaque,
  transparent, overdraw, and clipped triangles.
- [x] Reuse one inverse area per triangle in the primary path as the OIT path
  already does.
- [x] Preserve edge rules, winding, depth, perspective interpolation, and exact
  or established-tolerance output.
- [x] Acceptance: identical reference frames and a measured reduction in hot
  loop divisions without new allocations.

Validation ledger:

- `focused`: opaque and OIT interpolation share one inverse-area multiply path.
  `opaque_and_oit_hot_loops_pin_one_reciprocal_and_zero_per_pixel_divisions`
  semantically counts one reciprocal per triangle and zero `/ area` operations
  in either covered-pixel loop. Existing CPU reference/oracle tests pin edge,
  winding, depth, clipping, and interpolation output.
- `scoped`: the focused source-semantic proof passed remotely; full reference
  frames remain part of the single final chain. No allocations were introduced.

### P05 — Replace non-separable CPU bloom

- [x] Add impulse, edge, HDR-bright-region, radius, threshold, and repeated-run
  references plus controlled benchmarks.
- [x] Specify whether the existing box kernel is an API-visible look or only an
  implementation detail.
- [x] Implement separable blur or a multiscale chain with bounded scratch reuse.
- [x] Compare full-frame diff/SSIM and worst-region artifacts at every public
  setting.
- [x] Acceptance: visual contract meets approved thresholds and maximum-radius
  work is reduced from quadratic kernel cost.

Validation ledger:

- `implementation`: the legacy box appearance is treated as the compatibility
  contract and evaluated by two separable passes using bounded reusable scratch.
- `focused green`: `separable_bloom_is_repeatable_at_edges_and_has_linear_radius_work`
  proves edge/repeat behavior and linear radius work;
  `separable_bloom_matches_legacy_box_contract_across_public_controls` compares
  impulse, edge, bright region, threshold, intensity, and radii through 12 with
  max channel delta 1, RMSE 0.35, SSIM 0.99999, and worst-region bbox artifacts.
  Both passed remotely and the report is written under
  `target/gate-artifacts/p05-cpu-bloom/`.
- `full`: deferred to section 10.

### P06 — Cache GPU format/sample-count capabilities

- [x] Instrument adapter format-feature queries per prepare/frame for sample
  count 1 and MSAA modes.
- [x] Cache maximum supported sample count per device plus target-format set at
  prepare/device initialization.
- [x] Invalidate on device rebuild and surface-format change, not every frame.
- [x] Preserve structured unsupported-sample errors and WebGPU/WebGL2 policy.
- [x] Acceptance: steady render performs zero adapter format-feature probes and
  produces identical capability/output results.

Validation ledger:

- `implementation`: sample-count capability results are cached per live device
  and target-format set, populated during prepare, and dropped with device
  state. Surface-format refresh requests the new set; unsupported-sample errors
  and browser backend policy are unchanged.
- `focused green`: `format_capability_cache_probes_each_format_only_once` pins
  one probe per new format; the retained re-entry rendered proof asserts
  `gpu_format_feature_probes == 0` during steady render and exact full-prepare
  output parity.
- `full`: final hardware lanes verify the cache across actual surface formats.

### P07 — Remove clipping clones and formatted environment cache keys

- [x] Add allocation counters for steady GPU render, auto-exposure rerender, and
  unchanged-environment prepare.
- [x] Borrow immutable prepared clipping planes through encoding or use stable
  retained storage without violating mutable renderer ownership.
- [x] Replace `format!` environment identity with a typed/hashable key derived
  from immutable identity fields/revision.
- [x] Prove cache invalidates on every environment field that changes lighting.
- [x] Acceptance: named allocations disappear and output/cache correctness is
  unchanged.

Validation ledger:

- `implementation`: clipping planes are retained as `Arc<[ClippingPlane]>` and
  encoding clones only the allocation-free Arc handle; environment identity is
  a typed hash key covering source digest/dimensions, resolutions, delivery, and
  sidecar identity, with an allocation-free active-key match.
- `focused green`: retained re-entry proves clipping storage pointer identity;
  environment cache tests pin field-complete identity, unchanged revision reuse,
  handle replacement reuse, and invalidation. Native proof allocation counters
  cover steady present and asynchronous exposure convergence.
- `full`: exact Windows hardware execution supplies the native allocation and
  output evidence; no push is allowed before it is zero-error green.

### P08 — Make guide performance representative and report timing honestly

- [x] Update local-checkout render guidance to build/run a release binary for
  representative rendering; keep debug commands only for development checks.
- [x] Add separately measured prepare, render, readback/capture, and total
  duration fields to `scena.render_introspection.v1`, with clock/source and
  unavailable semantics.
- [x] Version/update schema fixtures and ensure timings are observational, not
  deterministic output fields or hosted blocking thresholds.
- [x] Add a clean guide smoke measuring that the intended release binary is
  used; do not hardcode the reported 14.7s as a gate.
- [x] Acceptance: agents can distinguish compilation, prepare, render, and
  capture cost and do not judge renderer performance from debug profile.

Validation ledger:

- `implementation`: the guide builds `scena` with `cargo build --release --bin
  scena --features agent` and labels debug `cargo run` as CLI-development only.
  `scena.render_introspection.v1` carries explicit prepare/render/capture/total
  observations plus clock/source and unavailable semantics; timing is excluded
  from deterministic frame identity and blocking hosted thresholds.
- `focused green`: `render_introspection_timings_are_explicit_observations`, the
  stable schema fixture, and `a03_llm_guide_smoke` pin the stage model and release
  binary command without hardcoding a wall-clock result.
- `full`: clean packaged guide execution waits for section 10.

## 6. Documentation, RFC, and public-contract alignment

### D01 — Make the RFC unambiguously canonical and current

- [x] Remove the stale “proposed until owner ratification” sentence.
- [x] Record shipped SSR, DoF, LTC area lights, tiled light assignment, LOD,
  occlusion culling, and semantic AOV capture in scope without implying every
  backend has identical capabilities.
- [x] Keep SSAO, which the RFC already mentions, accurately described.
- [x] Point the one active implementation backlog to this checklist.
- [x] Reconcile roadmap/checklist status tags with current capability reports.
- [x] Acceptance: charter status, shipped scope, non-goals, and active backlog
  do not contradict source or each other.

Validation ledger:

- `implementation`: the RFC is the ratified canonical charter, lists the shipped
  renderer systems with backend-capability qualification, preserves scena's
  non-simulation boundary, and points to this v1.9 remediation file as the only
  active implementation backlog. Archived v1.8 roadmaps now point here instead
  of declaring themselves active.
- `focused green`: remote xtask mutation
  `d05_d06_doctor_rejects_multiple_active_backlogs_and_persistence_overclaim`
  passed and rejects both a second active backlog and stale historical v1.8
  pointers. `state_of_art` now pins the actual “implementation in progress”
  status.
- `full`: the grouped `doctor --full` gate follows after documentation freezes.

### D02 — Correct README/example validation claims

- [x] Replace the incomplete “compile all public examples” command with the Q02
  feature-matrix command/script.
- [x] Verify every README command from a clean packaged checkout.
- [x] Keep installation feature guidance aligned with A04.
- [x] Acceptance: every command labeled as proof performs exactly the claimed
  coverage.

Validation ledger:

- `focused`: README uses the maintained public-example manifest/feature matrix
  and the Q02 mutation test rejects an uncompiled required-feature example.
  Installation guidance consistently distinguishes the default CLI from the
  `agent` feature contract.
- `full green`: the maintained `scripts/release_publish_dry_run.sh` extracted
  the packaged crate into a clean temporary checkout, ran the default tests,
  examples, rustdoc, demo/proof WASM builds, and completed `cargo publish
  --dry-run`. The separate A03/A04 packaged-guide tests also passed with the
  agent feature enabled. The script's aggregate status remained failed only
  for the subsequently fixed 502-line doctor finding and intentionally absent
  release/Windows provenance; neither failure contradicted a README command.

### D03 — State the measurement authority boundary publicly

- [x] Add a concise README/API/guide statement that scena measurements are
  scene-space visualization/inspection aids, not calibrated or authoritative
  metrology.
- [x] Document units, source-unit conversion, transform assumptions,
  precision, snapping, occlusion, and unsupported tolerance claims.
- [x] Add structured metadata needed for hosts to disclose these limitations.
- [x] Acceptance: no public example or schema implies certified dimensional
  authority.

Validation ledger:

- `implementation`: README/API/guide text defines measurements as scene-space
  visualization aids, not calibrated metrology. `SceneHostMeasurementAuthorityV1`
  records unit conversion, transforms, f32 precision, snapping, occlusion, and
  unsupported tolerance/calibration claims in the stable contract.
- `focused green`: scene-host measurement and stable-contract fixtures passed
  remotely; public examples use the same disclosure language.

### D04 — Document actual proof levels

- [x] Define smoke, conformance, deterministic reference, cross-backend parity,
  hardware evidence, and release evidence.
- [x] Link each headline claim to its enforcing workflow/schema/artifact.
- [x] Remove stale language describing Q01 as sample-only or, conversely,
  calling nonblack smoke parity.
- [x] Acceptance: a reader can tell which claims are local, CI, and physical
  GPU evidence without inspecting source.

Validation ledger:

- `implementation`: `docs/specs/release-gates.md` owns the proof-level taxonomy
  and maps each claim to workflow, schema, artifact, hardware requirement, and
  release-evidence eligibility. README and review documents link to it and use
  smoke/parity terminology consistently.

### D05 — Align changelog/release notes and public errors

- [x] Add user-visible entries for each corrected behavior and any migration.
- [x] Call out stricter failures for formerly silent invalid input.
- [x] Document CLI code/exit changes, capture provenance schema changes, and
  installed feature behavior.
- [x] Run link/code-block/version-pin doctor checks.
- [x] Acceptance: release notes describe observable changes without promising
  optional F-items.

Validation ledger:

- `implementation`: v1.9 changelog entries cover corrected import, capture,
  viewport, animation, renderer, CLI/error, schema, performance, proof, and
  installed-feature behavior, including newly strict invalid-input failures.
  P03 additionally records stable `f64` aggregation and non-finite RGB
  rejection; optional F01-F09 are explicitly not advertised as shipped.
- `scoped green`: remote `cargo run -p xtask -- doctor --full` completed with
  `mode=Full status=pass` after the documentation and source tree froze for
  this scoped pass. It exercised link, required-document, code-block,
  version-pin, schema-fixture, source-ownership, and release-proof rules.

### D06 — Reconcile the formerly collapsed external findings before closure

- [x] Owner supplied B14-B25, R1-R8, P6-P13, and A6-A14 in full after the
  authenticated artifact could not be opened by this session.
- [x] Audit records every supplied claim as confirmed, partial, or requiring a
  focused rendered reproducer before changing production behavior.
- [x] Checklist includes a remediation/acceptance owner for every confirmed or
  partial actionable finding and corrects the resize and native-m8 disputes.
- [x] At implementation closure, reconcile any newer review findings added
  after source baseline `a28f2149c39290aac7a059232b4e21de266ea88c`.
- [x] Acceptance: closure names the exact frozen baseline and does not imply
  later code or undisclosed findings were audited automatically.

Validation ledger:

- `closure baseline`: all supplied findings and implementation-discovered
  defects were reconciled against
  `a28f2149c39290aac7a059232b4e21de266ea88c`; no claim is made about future
  commits or findings not disclosed in this review.
- `late findings`: final gates exposed and closed three additional concrete
  issues: the split WASM scene-loading module used the wrong/private texture
  helper path, detached publish verification did not explicitly bootstrap the
  ignored canonical agent files, and `src/render/frame.rs` exceeded the
  enforced 500-significant-line KISS limit. Focused WASM compilation, publish
  bootstrap mutation coverage, and `doctor --full` respectively failed before
  the fixes and passed afterward.
- `remaining boundary`: optional F01-F09 are explicitly separate renderer
  projects, not undisclosed correctness defects. Windows hardware and public
  release rows remain execution gates in section 10, not review omissions.

## 7. Repository hygiene and dependency work

### H01 — Make cache/disk cleanup scoped and observable

- [x] Add/document a status command listing task-scoped target/cache/temp paths,
  sizes, ages, and whether they are reproducible.
- [x] Define retention guidance for local and remote validation artifacts.
- [x] Require explicit exact targets; never recursively clean home/workspace or
  unrelated caches.
- [x] If cleanup is requested, record what was removed and recoverability.
- [x] Acceptance: a 100+ GB target tree is discoverable without authorizing an
  unsafe broad deletion.

Validation ledger:

- `implementation`: executable `scripts/scena_task_cache_status.sh` validates a
  strict task slug and emits versioned JSON containing exact checkout/target/
  temp paths, sizes, ages, reproducibility, retention, and cleanup authority.
  Troubleshooting docs forbid broad home/workspace cleanup.
- `focused green`: `tests/h01_cache_status.rs` passes for valid status and
  malicious/ambiguous target rejection on POSIX hosts, which is the documented
  Bash/SSH remote-builder contract. The integration target is explicitly
  excluded on Windows instead of pretending that Git Bash path semantics are a
  supported cache-root API. No cleanup was requested or performed; therefore
  there is no deletion/recoverability event to record.

### H02 — Handle Dependabot PRs independently

- [x] Re-check the open PR set at execution time.
- [x] Batch compatible GitHub Action pin updates only after reading changelogs
  and verifying permissions/output behavior.
- [x] Keep dependency-only evidence separate from the correctness RC unless the
  owner explicitly combines them.
- [x] Acceptance: merged updates have green workflow evidence and no release
  permission/provenance regression.

Validation ledger:

- `read-only GitHub audit`: open Dependabot PRs were #7 setup-node 7, #8
  upload-artifact 7, #9 download-artifact 8, and #10 checkout 7. Only #8 had an
  all-green observed check set; the others had Linux/macOS/Windows or release-
  evidence failures.
- `disposition`: no dependency PR was merged, rebased, or folded into this
  correctness candidate. Future merges remain conditional on upstream change
  review plus fully green workflow/provenance evidence, satisfying this section
  without expanding the authorized product-change scope.

## 8. Doctor/checklist enforcement map

Every repeated/static failure family must become mechanically harder to
reintroduce:

- [x] Texture collections used by raw glTF index cannot be compacted (C01).
- [x] Every advertised quantized semantic/component pair has a fixture (C02).
- [x] Capture schemas bind pixels to readback provenance (C03).
- [x] Camera mutation cannot bypass validation/revision (C08).
- [x] All public animation constructors invoke the shared validator (C09).
- [x] Every generated WGSL variant appears in offline validation (Q01).
- [x] Every public required-feature example appears in CI (Q02).
- [x] Release evidence cannot be minted by local/self-reported provenance (Q03).
- [x] Smoke-only evidence cannot claim parity/release status (Q04).
- [x] Native m8 cannot claim release evidence without its full-frame/asymmetry
  oracle (Q06).
- [x] Required parity lanes cannot return before assertions (Q08).
- [x] Reference regeneration/provenance cannot self-certify (Q11).
- [x] Required-document, release-artifact, version, and WGSL coverage is
  semantic and fail-closed in the mode making the claim (Q12).
- [x] Field-model metadata and accepted recipe paths remain exhaustive (A02).
- [x] Canonical guide blocks execute from a clean temp CWD (A03).
- [x] Preset registries and vocabulary remain exhaustive (A07).
- [x] CLI contract/error/exit documentation is generated or parity-tested
  against source metadata (A01/A10).
- [x] SceneHost Resize/ScaleFactorChanged cannot leave viewport/picking stale
  (C11).
- [x] Retained templates cannot store view-dependent culling results (C12).
- [x] Optional-extension mappings and cached warnings cannot disappear silently
  (C20/C21).
- [x] RFC's active backlog exists and status text is coherent (D01).
- [x] README “all” commands match the maintained manifests (D02).

Doctor rules must have executed known-bad mutations. A source substring pin is
acceptable only when it proves the intended invariant and cannot be replaced by
a semantic compile/fixture test.

Validation ledger:

- `scoped green`: remote `cargo run -p xtask -- doctor --full` passed after all
  listed semantic/fixture/mutation rules were wired. The final release
  checkpoint will rerun this once on the frozen candidate; this scoped pass
  does not substitute for the final full chain.

## 9. Optional renderer projects — not correctness blockers

Each item requires a separate RFC/API/proof plan and its own final checkpoint.

- [ ] **F01 point and spot shadows:** cube/projected shadow maps, filtering,
  bias, atlas/lifetime, capability truth, native/WebGPU/WebGL2 references.
- [ ] **F02 HDR post chain:** linear `Rgba16Float` scene/post path where
  supported, tone mapping at output, WebGL2 degradation, bloom headroom proofs.
- [ ] **F03 glTF/GLB export:** explicit supported/lossy table, extension policy,
  round-trip semantic and external-validator evidence.
- [ ] **F04 camera paths:** versioned path/interpolation contract, deterministic
  capture, no hidden clock, CLI sequence output.
- [ ] **F05 animation blend/crossfade:** weighted transform/morph blending,
  additive policy, interruption/root-motion contract, deterministic tests.
- [ ] **F06 GPU OIT:** overlap order invariance, memory/capability policy,
  reference images and WebGL2 fallback.
- [ ] **F07 AgX tonemapper:** published transform/reference vectors, Blender
  comparison, cross-backend color evidence, explicit output encoding.
- [ ] **F08 image diff:** extend `scena diff` with versioned PNG diff metrics,
  heatmap, worst-region bbox, thresholds, inequality exit code, optional AOV
  attribution.
- [ ] **F09 screenshot annotations:** renderer-owned presentation overlays with
  deterministic layout, accessibility/export policy, and semantic exclusions.

Before accepting a feature, update the RFC and capability report; do not use
feature work to postpone C/Q/A/D closure.

## 10. Single final integration and release checkpoint

Start only when C01-C23, Q01-Q12, A01-A15 as selected for the release, P01-P08
as selected for the release, D01-D06, H01-H02 as applicable, and section 8 are
green under focused/scoped evidence. Freeze one exact commit and do not mix new
features or dependency updates after the checkpoint begins.

### 10.1 Preflight and source identity

- [x] Clean intended worktree; record branch, exact 40-character commit, version,
  diff, tags, remote, toolchains, lockfile hash, and submodule state.
- [x] Run remote preflight, sync the exact tree, manually bootstrap agent files,
  and record source/destination hashes.
- [x] Record validation path, target dir, shared-checkout status, free space,
  and host/tool versions.

The committed renderer/browser candidate was
`4e84ccf05ad7d525c39e5d13091019e31a793984` on
`codex/full-review-1.9-audit`, package `1.9.0`. The only later source changes
repair xtask/test fixtures and this closure ledger; the final amended
40-character commit is recorded by the push and CI evidence. Validation used
`/home/johannes/.cache/codex-worktrees/scena-full-review-1-9-publish` with
`CARGO_TARGET_DIR=/home/johannes/.cache/codex-targets/scena-full-review-1-9-publish`;
the obsolete shared checkout was absent. Canonical/destination hashes were
`d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
for `AGENTS.md` and
`a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
for `.codex/skills/**`.

### 10.2 CPU builder full chain — once

Use the task-scoped `validation_path` and `CARGO_TARGET_DIR` printed by
preflight. Confirm exact commands against the release workflow at execution
time; the minimum intended chain is:

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --all-targets --all-features -- -D warnings`
- [x] `cargo test`
- [x] required feature-matrix tests, including `--all-features` where the
  release workflow supports it and every Q02 example combination
- [x] `cargo run -p xtask -- doctor --full`
- [x] `cargo doc --no-deps --all-features`
- [x] WASM/WebGL2 compile and static/evaluator tests for public browser features
- [x] public API/semver/schema golden checks
- [x] packaged-crate/canonical-guide smoke from a clean temporary directory
- [x] cargo package/publish dry-run using the maintained release script

The single full chain ran on committed candidate
`4e84ccf05ad7d525c39e5d13091019e31a793984`. It passed the examples, benches,
rustdoc, WASM checks, shader manifest, doctor, claim audit, package list,
publish dry-run, npm/browser probes, demo/proof builds, and WASM release/size
gates. It exposed only clippy/test-fixture defects and four legacy wasm-bindgen
launcher failures. Those were reduced to focused causes and repaired without
changing renderer/browser production code: clippy now passes for the full
workspace/all targets/all features; `cargo test -p xtask` passes 392 tests; the
packaged all-feature CLI contract passes; and all four legacy browser targets
pass under the maintained Playwright Chromium plus matching driver. Per the
execution contract, only these affected gates were rerun; the full chain was
not repeated.

Do not invent a replacement command when the repository provides a release
script. Record the script, exact commit, and artifact hashes.

### 10.3 Rendered and hardware proof

- [x] Run current Q01 full-frame known-bad-mutation proof on the required real
  GPU backend set.
- [x] Run native m8 WaterBottle full-frame/asymmetry proof and verify
  `release_evidence` is false if that proof did not execute.
- [x] Run affected native-surface color/capture proof for C03/C04.
- [x] Run required browser WebGPU/WebGL2 lanes for changed browser-visible
  surfaces.
- [ ] Run macOS/Windows/native lanes required by the release policy through the
  maintained workflow/runner, not ad-hoc overlays.
- [x] Ask for user-operated hardware only if no maintained runner can satisfy a
  required claim, and only after a written root-cause checkpoint. One approved
  run uses one checksum-verified bundle and automatic archive upload.
- [ ] Verify CI provenance/attestation fields and artifact digests from Q03.

The maintained Windows runner passed all 18 stages for exact renderer/browser
candidate `4e84ccf05ad7d525c39e5d13091019e31a793984` on native DX12, browser
WebGPU, and browser WebGL2. The uploaded archive SHA-256 is
`f10851b0452e2b954d704d8fc98ce821ce4986c1b3de09db74a375cd207f0b7a`.
Its independent validator passed and its execution metadata binds the source
commit and executable/WASM hashes. Its summary correctly leaves
`release_evidence:false` because CI-issued provenance is unavailable in a
user-operated run. The maintained macOS/CI provenance rows therefore remain
open for section 10.4.

### 10.4 GitHub and public release proof

Only after the user explicitly authorizes commit/push/release actions:

- [ ] Commit with a standalone product-change subject; no internal round labels.
- [ ] Push one frozen release candidate.
- [ ] Collect all failures from the deciding workflow before any fix/push.
- [ ] Require every configured required check and release artifact to be green
  for the exact commit.
- [ ] Verify branch, tag, release object, Latest marker, crates.io/public
  version, and downloadable artifact provenance separately.
- [ ] Keep monitoring until the requested public version bump is confirmed; do
  not stop at a local version edit or successful push.

### 10.5 Final closure ledger

- [x] `focused`: every mandatory row names its red/green proof.
- [x] `scoped`: every touched surface names the narrow additional gates.
- [x] `full`: one frozen-commit chain, with reused unchanged evidence called out.
- [x] `skipped`: optional features and unavailable/non-required hardware are
  explicit, never silently passed.
- [x] Counts: investigation time, remediation attempts, RC pushes, full-matrix
  runs, user-required actions.
- [ ] No unresolved critical/high defect, no unknown release-evidence hole, no
  broken canonical command, and no contradictory public capability claim.

Checkpoint counts before the second corrective GitHub push: one local
full-chain checkpoint, two completed GitHub matrices, two RC pushes, and one
successful user-operated hardware run for the exact production candidate. The
first GitHub matrix passed Linux native, Linux WebGL2, Linux WebGPU, macOS
Metal, wasm32/package, and 4K performance; Windows DX12 alone failed because
the H01 shell helper hardcoded `python3` on a runner that exposes Python as
`python`. A fail-closed runnable-interpreter probe fixed that real portability
gap, but the second matrix proved the remaining H01 failure was the test
harness applying a documented POSIX Bash/SSH cache-path contract to native
Windows paths. The same second matrix also exhausted the Linux hosted runner
to 91 MB free and three unrelated `rust-lld` processes terminated with
`SIGBUS`; all preceding xtask tests passed 392/392. The batched harness fix
therefore scopes H01 to Unix, disables unused Cargo dev/test debug symbols on
both hosted CI and release workflows, and captures the xtask test list before
grepping so a successful exact-name check cannot emit a broken-pipe error.
Doctor pins both workflow copies so the release-only duplicate cannot drift
back after a green PR matrix. No renderer source changed, so the successful
exact-candidate Windows hardware proof remains applicable.

Earlier remediation comprised one batched clippy/all-feature test-fixture
repair, one batched doctor-fixture repair, and one browser-launcher
investigation. The browser investigation preserved the common 404 signature,
falsified driver mismatch, then identified both the long Unix-socket path and
`/tmp` quota constraints; all four affected browser targets passed together on
the corrected maintained runtime.

Only then change this checklist status to complete. Closure must name the exact
frozen source baseline and may say “all independently known defects at the frozen baseline are reconciled,” not imply that future code was audited
automatically.
