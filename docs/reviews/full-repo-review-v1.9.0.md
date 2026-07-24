# Full-repository review: `main@a28f214` / scena 1.9.0

Reviewed: 2026-07-22

Source baseline: `a28f2149c39290aac7a059232b4e21de266ea88c`

External input: the owner supplied the summary of a separate eight-pass review
and its Claude artifact URL. The artifact body was not available without the
originating authenticated Claude session during the first audit. The owner then
supplied the collapsed medium/low, performance, proof, and agent findings in
full. This revision reconciles that complete supplied list against the current
tree.

## Verdict

The review found real release-blocking defects. In particular, a malformed or
unsupported texture entry can shift every following glTF texture binding,
quantized tangent and morph accessors are decoded through f32-only readers,
capture freshness is checked against a different frame from the returned
pixels, and native non-sRGB surfaces reuse color-management state built for an
sRGB target. These are correctness defects, not polish.

The browser Q01 triangle oracle is a full-frame CPU/WebGPU comparison with
strict thresholds, diff artifacts, and known-bad mutations including vertical
flip. It does not close the separate native Metal WaterBottle lane: that m8
headline still writes `release_evidence:true` after seven region samples and
four color histograms when its full-frame reference diff did not run. The first
revision of this report conflated those independent oracles. Other proof gaps
include incomplete WGSL variant validation, scene-host examples omitted by the
ordinary CI example command, and insufficient CI-run provenance in
release-evidence schemas.

The codebase is strong but not "perfect." A useful closure condition is: no
known critical/high defect, every accepted input has deterministic semantics or
a structured rejection, no release claim can pass without the evidence it
names, and the documented first-run commands are continuously executed. The
companion remediation checklist defines that condition without making optional
multi-release features block the correctness release.

## Audit method and limits

- Inspected the current Rust, JavaScript, workflow, docs, schema, and release
  gate implementations at the exact source baseline above.
- Followed the relevant data paths instead of accepting line-local claims; for
  example, texture parsing was checked through material slot resolution, and
  capture validation through rendered/readback frame provenance.
- Distinguished product defects, proof defects, documentation defects,
  performance hypotheses, and feature proposals.
- Did not treat a code-reading performance estimate as a measured speedup.
- Did not rerun the full Rust/browser/GPU release matrix merely to write a
  review. The checklist requires focused red/green tests per fix and one final
  full checkpoint after the implementation batch freezes.
- The repository contained about 306,000 physical Rust lines and 1,607 tracked
  files at review time. These are inventory facts, not evidence of review
  quality.

## Claim-by-claim result

Status meanings:

- **Confirmed**: the current implementation supports the claim and a focused
  regression can be specified.
- **Partial**: a real issue exists, but the supplied wording overstates or
  conflates it.
- **Stale/incorrect**: the current source has already replaced the described
  behavior or the cited proof does more than claimed.
- **Not independently resolvable**: the supplied summary omits the exact claim
  or reproducer needed to identify it safely.

### Correctness bugs

| Claim | Result | Current-tree evidence and required correction |
|---|---|---|
| Dropped glTF texture entries shift later material bindings | **Confirmed, release blocker** | `src/assets/gltf/textures.rs` collects document textures with `filter_map`, while material slots later call `textures.get(raw_gltf_index)`. Preserve index identity with an index-aligned `Vec<Option<_>>` or explicit index map and reject a referenced invalid entry with path/index context. |
| Quantized tangent and morph accessors use f32-only readers | **Confirmed, release blocker** | `src/assets/gltf/meshes.rs` uses `read_tangents()` and `read_morph_targets()`, whose typed iterators assume f32 accessors. Implement component-aware BYTE/UBYTE/SHORT/USHORT/F32 decoding under `KHR_mesh_quantization`, including normalized rules and morph POSITION/NORMAL/TANGENT deltas. |
| Capture can certify stale pixels | **Confirmed, release blocker** | `src/capture.rs` obtains readback provenance but validates returned bytes against `rendered_frame_state()`. A newer present-only frame can therefore certify pixels from an older readback. Bind the descriptor and staleness check to the exact readback frame state. |
| Browser wheel treats raw `deltaY` as notches | **Confirmed, high** | `src/viewer_element/element.js` forwards browser wheel values directly and `src/controls.rs` applies a linear `1 + delta * 0.1` factor. Normalize `deltaMode` and use bounded exponential zoom so direction and magnitude are device-independent. |
| Native non-sRGB surface is too dark | **Confirmed, release blocker** | Native `src/render/gpu/draw.rs` reuses color-management state derived from the offscreen sRGB format for the actual surface; the browser surface path derives it from the target format. Build target-specific output state or use an explicit linear-intermediate conversion. |
| Importer ignores glTF `scene`/`scenes` | **Confirmed, high** | `src/assets/gltf.rs` imports all document nodes and `SceneAsset::root_indices()` derives graph roots instead of honoring `document.default_scene()` and selected scene roots. Define default/explicit scene-selection semantics and reject ambiguous invalid selections. |
| Hot reload reparents imports to the scene root | **Confirmed, high** | `Scene::replace_import` instantiates a replacement with default root placement and does not preserve the old import-root parents. Preserve host parent relationships and root-local transforms atomically across replacement. |
| Resize desynchronizes SceneHost picking | **Confirmed, high** | `SceneHostCore::handle_surface_event` updates `self.viewport` only for `ViewportChanged`; `Resize` resizes the renderer but reports CSS dimensions through the stale DPR, and `ScaleFactorChanged` is discarded. `pick_hit` then builds NDC from stale `self.viewport`. A 640x480 host followed by `SurfaceEvent::Resize { width:1280, height:960 }` therefore renders at the new size and picks against 640x480. Rebuild the viewport in `Resize`, update DPR on scale changes, and pin direct platform-event mapping. |
| Camera intrinsic changes can skip rerender | **Confirmed, high** | framing mutates camera aspect/near/far directly while dirty/revision tracking is primarily transform/node based. If the transform remains unchanged, render-on-change can reuse stale output. Camera descriptors need validated mutation APIs and an intrinsic revision included in frame identity. |
| NaN keyframes poison transforms | **Partial, high** | Imported glTF clips receive finite-value validation, but public `AnimationSourceClip::new` is unchecked and `rebind` produces an unchecked clip. Make source construction/rebinding fallible or apply the same validation contract before mixer use. |
| Exploded-view math is wrong | **Confirmed for hierarchical mode, high** | descendant target locals are derived against original parents and then applied together, so a moved parent and child can double-count ancestor displacement. Compute final local transforms against the planned final parent world transform and prove exact three-level hierarchy output. |

### Performance claims

| Claim | Result | Current-tree evidence and correction |
|---|---|---|
| Retained dynamic fast path is defeated by any culled source object | **Confirmed design defect** | the retained template represents the culled prepared set, while `src/render/prepare_retained.rs` requires coverage of all source nodes before dynamic reuse. Retain a source-complete template and re-evaluate culling during dynamic preparation. |
| Auto exposure synchronously reads back and renders twice | **Confirmed** | automatic exposure selects synchronous GPU readback, applies the measured exposure, and renders again when it changes. Replace with asynchronous/downsampled metering and explicit convergence behavior. |
| Exposure metering sorts millions of floats | **Confirmed** | full-frame luminance values are allocated and sorted for highlight guarding. Use a bounded histogram/selection over a downsampled metering buffer. |
| CPU rasterizer performs three divisions per covered pixel | **Confirmed micro-optimization opportunity** | the primary raster path divides each barycentric component by area; the OIT path already hoists the reciprocal. Share the reciprocal form and prove bit/tolerance parity. |
| CPU bloom is a 625-tap non-separable blur | **Confirmed** | the maximum radius produces a 25-by-25 box for each output pixel. Replace with a separable or multiscale implementation under a visual oracle. |

These are not ranked by intuition alone. Each optimization first needs a stable
benchmark distribution, identical rendered-output contract, and allocation or
work-count measurement. Shared GitHub runner wall-clock thresholds remain
report-only.

### Proof and release-gate claims

| Claim | Result | Current-tree evidence and correction |
|---|---|---|
| Browser Q01 is only a sample/nonblack proof | **Stale/incorrect for browser Q01** | `scena.q01.required_webgpu_pixel_parity.v1` compares complete CPU/GPU frames for the single `m6-identical-unlit-triangle-v1` fixture and rejects mutations including vertical flip. Keep it, while stating its deliberately narrow unlit-triangle scope. |
| Native macOS m8 WaterBottle headline can claim release evidence from seven samples/four histograms | **Confirmed proof blocker** | `.github/workflows/ci.yml` runs `m8_real_asset_waterbottle_gpu_headline` without `SCENA_REFERENCE_DIFF`. `tests/m8_real_asset_proof.rs` gates the full-frame comparison on that variable but unconditionally writes `release_evidence:true`; a mirrored or structurally wrong lit/textured frame can satisfy the sparse checks. Require the full-frame/asymmetry oracle or write `release_evidence:false`. |
| Browser headline asserts only `nonblack > 0` | **Partial** | a browser-consumer smoke layer still checks nonblack/draw/submission health, while browser Q01 supplies full-frame triangle parity. Rename smoke/conformance outputs clearly so they cannot be mistaken for parity or for coverage of lit/textured native rendering. |
| Doctor never compiles WGSL | **Partial** | a Naga parse/validate test exists for one assembled material-shader path, but there is no exhaustive manifest of render/post/shadow/stroke/label/AOV variants. Add offline validation of every generated variant and retain semantic negative tests that parsing alone cannot replace. |
| Five scene-host examples are compiled by no CI step | **Confirmed** | examples gated by `scene-host` are skipped by the ordinary `cargo check --examples` lane; CI explicitly covers only a subset. Compile every public example in its required feature matrix and correct the README's "all examples" command. |
| `release_evidence=true` has no CI provenance stamp | **Partial, hardening required** | artifacts bind an exact source commit and hashes, but the schema does not require repository, workflow/ref, workflow SHA, run id/attempt, job, artifact digest, or attestation. Add CI-issued provenance for release-grade evidence; keep local diagnostic artifacts explicitly non-release. |

### Agent, CLI, and documentation claims

| Claim | Result | Current-tree evidence and correction |
|---|---|---|
| `AGENTS.md` misroutes application builders to contributor-only infrastructure | **Partial** | root `AGENTS.md` is intentionally the contributor contract and correctly requires the private builder for repo work. The public application-builder skill exists in the repository but is not packaged/discoverable for `cargo install` users. Add a public `scena guide agent`/export surface; do not weaken contributor controls. |
| Default `cargo install scena` cannot render | **Confirmed friction, documented today** | default crate features are empty; docs now recommend `--features agent`, but the executable name still suggests a usable CLI. Decide explicitly whether the CLI binary always includes the agent surface, a default feature enables it, or a separate CLI package owns it. Make every recovery hint consistent with that decision. |
| Schema field model omits promoted guide fields | **Confirmed** | the advertised field model covers only a subset of recipe fields while guides promote camera, framing, environment, post, and material controls. Generate it from authoritative metadata or add exhaustive parity tests against accepted recipe paths. |
| Runtime failures all become `invalid_arguments` | **Confirmed** | top-level `src/bin/scena.rs` collapses string errors into one code/exit status except unknown commands. Introduce a typed CLI error taxonomy for input, policy, I/O, asset parse, unsupported capability, backend/runtime, inequality, and internal failures. |
| Two canonical guide commands fail as written | **Confirmed** | the LLM guide redirects into a directory before creating it and later switches from `primitive-scene` to `primitive_scene`. Fix the paths and execute canonical shell blocks in CI from a clean temporary directory. |
| Discovery is weaker than verification | **Confirmed product theme** | public schema/versioned JSON and verify/repair are strong, but capabilities, feature requirements, vocabulary, example names, sandbox policy, and command output schemas require too many failed attempts to discover. Address this through generated, queryable CLI metadata rather than duplicated prose. |

## Supplemental finding adjudication

This section records the findings that were collapsed in the inaccessible
artifact and later supplied directly by the owner.

### Remaining correctness findings B14-B25

| ID | Result | Evidence and correction |
|---|---|---|
| B14 retained template bakes view-dependent culling | **Confirmed, medium** | Full prepare stores only `culled_primitives.visible`. The coverage guard usually forces a full prepare for an absent source node, but a node represented by another retained list (for example a stroke while its mesh was culled) can satisfy coverage while the missing primitive cannot re-enter. Retain the source template before view-dependent culling and cull per dynamic encode. This is the correctness half of performance R1. |
| B15 256px occlusion buffer can false-cull thin visible geometry | **Confirmed risk; rendered reproducer required before patch** | Occlusion is decided on a buffer capped at 256 pixels with center samples and a fixed depth epsilon. Downsampling can erase high-resolution gaps and make an occluder appear solid. Add a high-resolution sliver/gap fixture that the baseline falsely rejects, then choose a conservative dilation/bias/resolution policy; do not tune constants without the red image. |
| B16 suboptimal surface reconfigures the same cached configuration | **Confirmed, low** | `Suboptimal` returns `reconfigure_after_present`, whose consumers call `reconfigure_existing_surface`; that submits the unchanged cached config. Call `refresh_surface_configuration` and force prepare when format/present mode changes. |
| B17 visibility restoration aborts and leaves isolate bookkeeping | **Confirmed, low** | `restore_visibility` uses `?` for every snapshot entry and clears isolate state only after the loop. A removed node stops restoration mid-way. Skip missing snapshot nodes, restore all live entries, always clear bookkeeping, and report skipped stale entries structurally. |
| B18 hover transition omits `Left` and emits meaningless empty `Moved` | **Confirmed, low** | direct A-to-B maps to one `Entered` event for B; `(None,None)` maps to `Moved`. Specify transition events, emit `Left(A)` then `Entered(B)`, and emit nothing for unchanged empty hover unless the API explicitly defines an empty movement event. |
| B19 interactive viewer picks with builder camera, renders active camera | **Confirmed, low** | `InteractiveGltfViewer::pick_at` uses `self.camera`; `render_next_frame` uses `scene.active_camera()`. After the active camera changes, visual and pick rays diverge. Resolve the active camera for every pick or keep both states impossible to diverge. |
| B20 retired anchor/connector records discard generation | **Confirmed, low** | retired maps are keyed by `slot_index`, which truncates `KeyData::as_ffi()` to `u32`. Slot reuse can overwrite or misassociate diagnostics. Key by full typed key/`KeyData` or a generation-preserving value. |
| B21 any pointer release clears orbit and pan | **Confirmed, low** | `Released | Cancelled` clears both booleans without consulting the initiating button/pointer. Track gesture ownership per button/pointer and release only the matching gesture; cancellation may clear all for that pointer. |
| B22 invalid material-variant mappings disappear silently | **Confirmed, low** | `parse_primitive_material_variant_bindings` uses `filter_map` and drops out-of-range materials without adding an `AssetLoadWarning`, despite its comment promising future diagnostics. Preserve mapping context and warn or fail according to optional-extension policy. |
| B23 byte-loaded scene cache loses telemetry warnings | **Confirmed, low** | `load_scene_from_bytes` caches telemetry containing only fetched byte count; the disk/external-resource path extends telemetry with `scene.load_warnings()`. Populate the same warnings before caching so cache hits preserve the first load report. |
| B24 animated morph width silently truncates | **Confirmed, low** | sampled weight vectors are stored unchecked, and geometry application zips targets with weights. No mixer-time check binds channel width to the target mesh width. Validate during rebind/instantiation and reject mismatches before playback. |
| B25 GPU fallback prints unversioned prose | **Confirmed, low** | `warn_gpu_fallback` calls `eprintln!` outside the structured JSON outcome. Put fallback status/warning in the emitted envelope and keep stderr machine-valid under the CLI output contract. |

### Remaining performance findings R1-R8

| ID | Result | Evidence and correction |
|---|---|---|
| R1 retained fast path under culling | **Confirmed; same root as B14** | The coverage guard forces full prepare whenever a culled node has no surviving representation. Fix B14 once and benchmark camera pans/object entry-exit. |
| R2 synchronous auto-exposure/readback and second render | **Confirmed** | already recorded in the main performance table. Specify one-frame-late/asynchronous metering semantics before optimizing. |
| R3 full-frame allocation and sort | **Confirmed** | sRGB bytes become a `Vec<Color>`, luminances become another vector, and `highlight_guard_ev` sorts it. Scratch reuse plus `select_nth_unstable` is a smaller first step; a bounded histogram/downsample is the architectural target. |
| R4 format capabilities probed every frame | **Confirmed** | native draw calls `max_supported_sample_count`, which queries adapter format features on each frame even for sample count one. Cache supported counts per prepared target-format set and invalidate on device/surface-format change. |
| R5 three opaque barycentric divisions | **Confirmed** | already recorded; hoist `area.recip()` as the OIT path does and retain edge-rule parity. |
| R6 non-separable CPU bloom | **Confirmed** | already recorded; use two passes or a multiscale chain under a full-frame oracle. |
| R7 avoidable per-render/per-prepare allocations | **Confirmed, smaller** | clipping planes are cloned before each GPU draw and again when auto exposure rerenders; environment cache identity allocates a `format!` string. Borrow immutable prepared clipping data through encoding and store/hash typed environment identity components. |
| R8 guide uses debug-profile rendering and introspection omits render duration | **Confirmed usability/performance issue** | the local-checkout guide uses plain `cargo run`, so users measure debug rendering. Recommend a built `--release` binary for actual renders and add separately defined prepare/render/capture timing fields to introspection without making shared-runner timing a correctness gate. |

### Remaining proof/gate findings P6-P13

| ID | Result | Evidence and correction |
|---|---|---|
| P6 MSAA/FXAA lack pixel proof | **Partial** | FXAA already has CPU pixel-level hard-edge/intermediate-value assertions and an m2 on/off visual proof. The hardware PF01 lane mostly proves output difference, not lower edge energy, and MSAA lacks an equivalent effect-specific oracle. Add a diagonal-edge fixture for MSAA and strengthen required hardware AA evidence; retain existing FXAA tests rather than describing them as absent. |
| P7 parity tests silently skip on macOS/Windows | **Confirmed** | `tests/support/parity.rs` returns early unless `SCENA_USE_GPU`, `VK_ICD_FILENAMES`, or lavapipe exists. Normal macOS `cargo test --all-targets` and Windows lanes do not force these transmission/clipping/dynamic-transform proofs. Add explicit required hardware invocations or structured non-release skip artifacts consumed fail-closed. |
| P8 adapter display string widens WaterBottle tolerance | **Confirmed** | the m8 test detects `"Apple Paravirtual device"` plus `"Metal"` and widens olive tolerances from 25 to 35. Use a structured adapter/backend key with recorded expected values; do not branch quality policy on free-form display text. |
| P9 known-bad WaterBottle mutations are pixel edits | **Confirmed** | `q01_waterbottle_cpu_reference` mutates the passing RGBA frame; it does not rerender wrong camera/material state. Add at least one real scene/material/camera mutation that passes through prepare/render and must fail the same oracle. |
| P10 reference regeneration lacks independent ratchet | **Confirmed governance gap** | the 256px CPU reference is protected by a checked hash but has no external anchor; the Blender comparison targets the separate 512px reference and only color families. Require reviewed regeneration provenance plus an independent invariant/reference or dual approval outside the generator path. |
| P11 cross-architecture stability needs explicit evidence | **Confirmed hardening gap** | the tolerance is tight but not bit-identical, and CI implicitly relies on deterministic behavior across architectures. Add repeated in-process determinism tests and record per-architecture metrics before considering any tolerance change; introduce per-arch references only if measured deterministic differences require them. |
| P12 reference metadata overclaims the disabled diff | **Confirmed docs defect** | `reference_metadata.toml` says any internal drift trips the m8 test, while the full-frame diff is opt-in and absent from workflows. Wire it into the required lane or soften the metadata and `release_evidence` claim. |
| P13 doctor brittleness/fail-open cluster | **Confirmed with qualifications** | material uniform byte length is redundantly pinned as literal text despite source-level size assertions; `REQUIRED_DOCS` omits v1.9.0; `finding_reference` hardcodes the current release note; showcase WASM size returns when both artifacts are absent. Replace redundant text pins with semantic tests, derive current release/doc references, and make artifact absence fail only in the release mode that claims those artifacts. Complete WGSL variant validation remains Q01 in the checklist. |

### Remaining agent/Rust API findings A6-A14

| ID | Result | Evidence and correction |
|---|---|---|
| A6 preset vocabularies omitted | **Confirmed** | `vocabulary_report_v1` exposes seven basic sets but omits material, lens, framing, named color, light, scene, environment, and quality presets promoted by the guide. Generate vocabulary from authoritative preset registries. |
| A7 mandatory `--introspect` ceremony | **Confirmed** | render parsing rejects the command when the flag is absent even though introspection is the intended safe default. Emit introspection by default and accept the old flag as a compatibility no-op. |
| A8 no generic validate dispatch/JSON Schema export | **Confirmed** | verify subcommands parse their expectation type directly and schema discovery lacks exported JSON Schema. Add `scena validate <file>` dispatching on embedded schema plus versioned JSON Schema export; keep domain validators authoritative. |
| A9 no single CLI contract/exit 74 docs | **Confirmed** | `IO_ERROR_EXIT_CODE = 74` lives in source and `docs/errors.md` does not define a complete command/error/exit/output table. Generate or test one public CLI contract page. |
| A10 placement works only on imports | **Confirmed limitation** | place resolves `args.import_id` through `runtime.import_index`; authored starter nodes cannot use the same bounds verbs. Accept a typed authored-node/import target while keeping anchor/connector-only verbs constrained appropriately. |
| A11 compact/pretty JSON inconsistency | **Confirmed** | ordinary outcomes use `to_string_pretty`, global help builds compact JSON, and there is no uniform `--compact` policy. Define one default and a global formatting switch without changing envelope semantics. |
| A12 incomplete Rust error help | **Confirmed** | curated `help()` exists for Asset/Prepare/Render/Lookup only; top-level `Error`, Build, Import, Instantiate, and Animation lack a uniform route, and `Display` omits remedies. Add `Error::help()` delegation and complete subtype coverage; keep `Display` concise if a structured diagnostic adapter carries help. |
| A13 texture APIs/diagnostics | **Partial** | there is no public in-memory RGBA texture constructor or slot-typed loading helper, and native decode-limit errors are classified as generic parse. Browser >2048px resizing is not silent—it writes a console warning—but that warning is absent from structured asset telemetry. Add typed constructors/slots and structured resize/limit diagnostics. |
| A14 prelude/framing/features ergonomics | **Partial** | the crate root exports hundreds of v1 types and has no curated prelude; native missing files are raw `Io`; framing has many near-duplicate entry points. Those are real ergonomics debt. `controls`, `controls-winit`, and `controls-web` intentionally gate no implementation and are documented compatibility aliases, so they are not a silent bug; retain with an explicit compatibility test or deprecate deliberately. |

### Feature and scope claims

- Point/spot shadows, HDR post, glTF export, camera paths, animation crossfade,
  and GPU OIT fit the renderer charter. They are optional RFC projects, not
  release blockers for the defects above.
- `scena diff` already exists. A PNG visual comparison should extend it with a
  versioned image-diff result rather than introduce a parallel command family.
- `scena lint` should compose existing validate/diagnose/doctor contracts into
  a documented fail-closed profile, not duplicate validation logic.
- AgX is appropriate only with color-reference images, transfer-function
  tests, and cross-backend evidence.
- `product_options` and `visual_states` both apply visual patches. Consolidate
  their state primitive or document product options strictly as presentation
  grouping over visual states.
- Measurement APIs and docs must say that outputs are scene-space presentation
  measurements, not calibrated or authoritative metrology.

## Independent findings missed or understated by the supplied summary

### I01 — Invalid active-camera projection can fail open to a blank successful frame

Camera descriptor fields are publicly mutable. `PerspectiveCamera` accepts
finite positive fields without consistently bounding field of view below 180
degrees, and prepare-time projection diagnostics are collected without making
an invalid active camera a structured failure. Projection can then return no
screen coordinates for every vertex while render reports success.

Fix the family, not one value: define central validation for finite FOV,
aspect, near/far, orthographic extents, and projection matrices; route every
constructor and mutation through it; include camera descriptor revision in
frame state; and return a structured prepare/render error for an invalid active
camera. Add boundary and mutation tests for NaN, infinity, zero/negative,
FOV-at-or-above-180, and invalid near/far.

### I02 — Public animation source construction bypasses the importer’s safety contract

The external NaN claim is narrower than the real API issue. Public authored
source clips can bypass finite/time/channel validation and later become an
unchecked runtime clip during rebinding. One validator must own imported and
authored clip invariants, with structured path/channel/keyframe diagnostics.

### I03 — The canonical RFC contradicts itself and understates shipped scope

The RFC header says it is canonical and owner-ratified, while its introduction
still calls it proposed until ratification. SSAO is already named, so the
external claim that the RFC mentions none of the listed features is false;
however SSR, depth of field, LTC area lights, tiled lighting, LOD, occlusion
culling, and semantic AOV capture are shipped but absent or unclear. The RFC's
single active backlog also still points to the completed v1.8 remediation
checklist.

### I04 — The README's “compile all public examples” proof is not all-public

`cargo check --examples` skips required-feature examples. This is both a docs
bug and a gate-integrity bug: users are promised a proof that CI does not
actually execute. The example manifest and CI feature matrix must be generated
or checked together so a new gated example cannot be omitted silently.

## Repository hygiene observations

- Local `target/` occupied roughly 126 GB and `/tmp` roughly 7.4 GB at audit
  time. This does not justify deleting user data automatically. Add a documented
  task-scoped cleanup/status command and retention guidance; only delete the
  exact caches the user authorizes.
- Four Dependabot action-version pull requests were open. Treat each as normal
  dependency work with workflow evidence; they are not part of this review's
  correctness batch unless deliberately included.

## Final priority

1. Texture index identity, quantized attribute decoding, capture provenance,
   and native non-sRGB output.
2. glTF scene selection, hot-reload parent preservation, camera validation and
   intrinsic revisions, authored animation validation, exploded hierarchy, and
   normalized browser zoom.
3. Exhaustive WGSL validation, all-example feature compilation, typed CLI
   failures, field-model completeness, executable onboarding docs, and release
   evidence provenance.
4. Measure and fix the retained-culling, exposure, CPU raster, and bloom hot
   paths.
5. Update the RFC/README/public measurement boundary, then run the single final
   release checkpoint on a frozen commit.
6. Schedule optional renderer features independently after correctness and
   proof debt close.

The executable closure plan is
`docs/checklists/full-repo-review-v1.9.0-remediation.md`.
