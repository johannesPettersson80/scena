# Full-repository review v1.9.1 remediation checklist

Created: 2026-07-25

Status: **all 43 mandatory rows implemented; 38 fully closed, 5 carrying a
single blocked sub-item each (see section 9). Sections 7 and 8 are out of
scope for this batch.**

Source baseline: `codex/release-v1.9.0-finalizer-fix@931ac41cfc3f197379e968b075fcc411146bda87`
(tree `88d38e97a11461ff4f7b020e27dd30ba81c19442`, identical to tag `v1.9.0` at
`5a290318a3ea22533576b9c9857d352e449857f8`), package version `1.9.0`.

Audits reconciled by this checklist:

- Round 2 review, findings `N1`-`N26`.
- Independent second review: bugs `B1`-`B4`, performance `P1`-`P5`, agent
  economics `A1`-`A4`.
- Round 3 adjudication, which confirmed both above with corrections and added
  five independent findings.

Canonical charter: `docs/RFC-rust-3d-renderer.md`

Predecessor backlog: `docs/checklists/full-repo-review-v1.9.0-remediation.md`
(its C/Q/A/P/D/H rows are closed; its `F01`-`F09` optional features carry
forward into section 8 here).

## 0. Execution contract

### 0.1 Release target

Runtime defects require a code release. This batch ships as **v1.9.1**.

- Do **not** recut, move, or mutate the existing `v1.9.0` tag.
- The `v1.9.0` GitHub release *body* may be corrected in place (it still reads
  "Release candidate prepared").
- Code, changelog, and metadata corrections land in `1.9.1`.

### 0.2 Test cadence — focused per fix, full chain once at the end

Unchanged from the v1.9.0 contract. For every implementation row:

1. Add the narrowest deterministic regression that fails on the baseline for
   the claimed reason.
2. Run that focused proof and preserve the command, output, and artifact.
3. Classify the failure as product, harness, environment, policy, or
   provenance.
4. Make the smallest production change that closes the tested contract.
5. Rerun the identical focused proof until green.
6. Run the scoped gate implied by touched files — **and any gate that digests,
   pins, or hashes the contents of a file you touched, even in another crate.**
   This step as originally written was too narrow and let five defects through
   to the end of the batch (see below).
7. Update the row's validation ledger before starting the next logical item.

**Correction, recorded after the fact.** Step 6 caught none of the following;
all five surfaced only when a broad suite was run:

| Defect | Caused by | Found by |
|---|---|---|
| `guide agent --markdown` exiting 70 | `G08` | `T05` running A05 with `--features agent` |
| CLI process-table digest pin stale | `G06` | the post-`X01` CLI-suite sweep |
| `CliError::classify` pin stale | `X01` | `cargo test -p xtask` |
| `place.rs` over the size cap | `X01` | `cargo test -p xtask` |
| `feature_unavailable` code changed to `unsupported` | `X01` | the full `cargo test --tests` run |

The common shape: a file's *contents* were consumed by a gate in a different
crate — a SHA-256 digest, a pinned truth substring, a line-count rule, or an
end-to-end CLI contract. Editing the file passed every gate its own directory
implies. The cadence is otherwise unchanged and worked as intended.

Run the **full chain exactly once** in section 9 after all mandatory rows are
green and the release-candidate diff is frozen.

### 0.3 Standing rules

- [x] Never weaken an oracle or widen a threshold merely to obtain green.
      **Held.** Two thresholds moved and both were tightened, not loosened:
      `R03`'s data-texture assertion went from a `120..=135` range to exact
      `[128, 128, 128, 255]`, and `T04`'s new cross-renderer bound is half the
      hue separation the anchor itself exhibits rather than the measured value
      rounded up. Two goldens were re-baselined (`T07`), each with the causing
      row named and the reason the new bytes are correct; the assertions
      themselves are unchanged and still fail on any unreviewed change.
- [x] Never convert unavailable hardware, missing evidence, or unknown
      provenance into a pass. **Held.** The 7 GPU-adapter failures and the 6
      `HONEST-MATERIAL-PRESETS` findings are reported as failures in section 9,
      not skipped or annotated away. `E03` now writes a **failed** artifact on a
      strict lane without hardware instead of a passing one.
- [x] A test that asserts only `ok == true` or `status.success()` on rendered
      output is not an oracle. `G04` exists because of exactly this.
      **Held, and applied again in `T04`:** the new anchor test asserts that a
      channel-rotated anchor and a live wrong-material render each fail every
      material band, so it cannot degrade into an always-pass.
- [x] After two failed remediation attempts with the same signature, freeze
      product and harness edits until a smaller probe falsifies a competing
      cause. **Invoked once:** the `ARCH-KISS-SIZE` violation moved from
      `cpu_render.rs` to `cpu.rs` on the first attempt. Rather than a third
      edit, the file was split (`src/render/cpu_clip.rs`). The same discipline
      applied to `place.rs` → `src/bin/scena/resource_uris.rs`.
- [x] Before any `git push` touching Rust, run the full
      `.codex/skills/scena-release-hygiene/SKILL.md` chain. "Doctor passes"
      does not imply `fmt` or `clippy` passes. **Nothing has been pushed.** The
      three runnable chain commands were run after the final edit; the two that
      need a GPU or network are recorded in section 9.2 as blocked.

### 0.4 Checkpoints

- [x] **A — runtime blockers:** `R01`-`R05` focused red/green and scoped gates.
- [x] **B — agent surface:** `G01`-`G08`.
- [x] **C — coverage bijection:** `T01`-`T08`.
- [x] **D — contracts, evidence, docs:** `X01`-`X03`, `E01`-`E05`,
      `D01`-`D14`.
- [ ] **E — performance:** `P01`-`P05`. **Out of scope for this batch** by the
      execution contract; carried forward in section 7.
- [ ] **F — final:** section 9 once on one frozen source commit. **Requires a
      GPU + network host**; see section 9.

Section 8 forward-backlog items are **not** prerequisites for any checkpoint.

### 0.5 Corrections carried into this document

Recorded so they are not re-litigated:

- The claim "CI runs no inspection tests" is **false**. CI does run several
  targeted inspection and scene-host commands at `.github/workflows/ci.yml:53`.
  The real defect is that specific gated binaries are omitted (`T01`).
- `B1`'s "all three annotations invisible" is slightly overstated: a clipped
  callout-leader stub survives. All meaningful annotation *text* is absent and
  the dimension line is lost. The stub conveys nothing.
- `P2` was mis-cited in the second review. The resolved-cache hot path is
  `compose_transform` → `Transform::compose` (`src/scene/math.rs:102`), which
  normalizes **twice** per node — `rotate_vec3_by_quat` (`math.rs:197`) and
  `compose_rotations` (`math.rs:188`). The private `multiply_quat` in
  `src/scene/transforms.rs:120` is used by `local_transform_from_world`, not by
  the cache. `compose_rotations` carries a documented drift-bounding rationale.
- `N22` exits **70 / `internal_error`**, not `runtime_error`.
- The live demo serves **1.7.1** cache markers and bytes — worse than the
  reported "pre-remediation bytes under a 1.9.0 label". Re-confirmed by live
  fetch while closing `D05`: `1.7.1-public-a76468dfd9ba`.
- `N12` is contradictory evidence, not a demonstrated publish bypass: the job
  still fails.

Corrections found **while implementing**, each recorded in full in its own row:

- The Metal lane runs **12** `release_lane_command.sh` invocations. Neither
  "seven" nor the adjudicated "six" was right, and the lane **does** run the
  full-frame reference comparison the old `CLAUDE.md` said it skipped. Region
  tolerance is **25** everywhere; the documented "up to 35" did not exist in
  the code. (`D03`)
- `D11` named five reason codes; **four of them do not exist**
  (`clipped_by_section_box`, `clipped_by_plane`, `outside_viewport`,
  `occluded`). The real vocabulary is 20 codes, read from
  `src/render/visibility_diagnosis/`. (`D11`)
- `E05`/`D14` said three unregistered environment flags; a full sweep found
  **seven**, four of which gate release evidence. The registry could not have
  caught them: its scan covered only `tests/` and `scripts/`. (`E05`)
- `E04`'s "generic Q08 suffixes" understates it — Q07 and C09 shared the same
  path-length fallback. (`E04`)
- The orphaned-binary count is **29**, matching the review's 28 within one. An
  earlier count of 68 was wrong because it included
  `not(target_arch = "wasm32")` gates, which are always true on a Linux
  runner. (`T02`)
- `target/` measured **171 GB**, not the reported 136 GB. (`D07`)

---

## 1. Release-blocking runtime correctness

### R01 — Auto-exposure convergence and meter ordering

Owner: `src/render/exposure/meter.rs`, `src/render/exposure.rs`,
`src/render/gpu/draw/native.rs`, `src/render/gpu/readback.rs`.

Closes `N1` and `N10` together. They are one defect: an absolute correction
applied to an already-exposed sample, fed by an unordered sample queue.

Verified mechanism:

- `meter.rs:85` computes `raw_ev = (target_luminance / measured_luminance).log2()`
  — an *absolute* correction.
- `draw/native.rs:448` samples the already-exposed surface.
- `exposure.rs:355` replaces current exposure with that correction.
- Net: `EV_next = desired_EV - EV_current`. Derivative `-1` ⇒ two-cycle
  oscillation. Amplitude is scene-dependent; the instability is not.
- `readback.rs:124` scans `for slot in 0..self.pending.len()`, and
  `PendingAutoExposureMeter` carries only `receiver` and `format` — no
  submission sequence. A newer slot-0 sample can be consumed before an older
  slot-1 sample, letting exposure step backward.

- [x] Add a focused multi-frame attached-surface convergence proof over at
      least 6-10 frames that fails on the baseline by detecting oscillation
      (sign-alternating EV deltas that do not decay).
- [x] Add a focused red test that submits meter samples completing
      out-of-order and proves exposure steps backward.
- [x] Record `submission_sequence` and `sample_ev` in each meter submission.
- [x] Reject samples older than the last applied sequence.
- [x] Apply a damped delta: `new_ev = sample_ev + correction * smoothing`.
- [x] Prove the damped path converges monotonically and rejects oscillation.
- [x] Record the existing-test limitation in the ledger: current tests cover
      the CPU/pre-exposure path and only confirm that attached GPU metering is
      asynchronous. They do not test multi-frame convergence.
- [x] **Correction found while closing `T08`:** the first `R01` fix damped
      *every* feedback sample, including the one-shot headless path, which
      ships a knowingly half-corrected image. Split and fixed; see below.
- [ ] **Longer-term (deferred to section 8):** meter pre-tonemap HDR-linear
      scene color instead of the exposed surface. Deferred here because it
      depends on `F02`'s linear `Rgba16Float` scene/post path; the damped
      delta makes the loop a contraction under *any* monotone tonemapper, so
      this is an accuracy improvement rather than a stability fix.

Validation ledger:

- `focused red`: `cargo test --lib -- render::exposure` →
  `surface_auto_exposure_converges_instead_of_oscillating` and
  `feedback_exposure_never_reflects_around_the_current_value` FAILED with the
  exact two-cycle `[3.1699252, -8.6e-8, 3.1699252, -8.6e-8, ...]`, confirming
  the predicted `-1` derivative.
  `cargo test --lib -- render::gpu::readback` → both ordering tests FAILED
  (`Some(0)` for a newer slot 1; stale sample accepted against
  `last_applied = 6`).
- `classification`: product.
- `implementation`: `src/render/exposure.rs` (`next_feedback_exposure_ev`,
  `SURFACE_AUTO_EXPOSURE_SMOOTHING`, feedback-aware
  `estimate_auto_exposure_from_current_frame`),
  `src/render/gpu/readback.rs` (`select_completed_meter_slot`, submission
  sequence plumbing, superseded-slot retirement).
- `focused green`: identical commands → 10 passed, 0 failed.
- `scoped`: `cargo fmt --all --check` clean;
  `cargo clippy --lib --all-features -- -D warnings` exit 0.
- `blocked`: the physical attached-surface proof. This host has no working GPU
  adapter — `render::phase5_tests::skinned_joint_transform_rejects_dynamic_gpu_prepare_fast_path`
  and `render::post_tests::gpu_post_toggle_preserves_srgb8_transfer` fail with
  `RequestDevice { backend: HeadlessGpu }` on the **unmodified baseline** as
  well, so they are environment failures, not regressions. Lavapipe
  (`VK_ICD_FILENAMES=.../lvp_icd.json`) exceeded a 10-minute budget for the
  lib suite. The deterministic 10-frame proof above closes the math; the
  hardware lane must still run it once on a real adapter.
- `counts`: 0 threshold changes, 0 oracles weakened, 0 tests skipped.

**Note on method:** both fixes landed via a behavior-preserving seam first
(`next_feedback_exposure_ev` and `select_completed_meter_slot` initially
returned the pre-fix result verbatim), because both defective paths sample
post-exposure data that is unreachable from the public API on a CPU-only host
— `linear_frame` is `Some` only when there is no GPU (`build.rs:259`). The
seam makes the real production decision testable without a GPU; the red runs
above are against that seam with pre-fix behavior.

Scoped gate: focused exposure tests + one rendered comparison on each affected
backend. Unit tests alone cannot close a shader/output change.

Validation ledger (correction, found while enumerating `T08`):

- `how it surfaced`: `T08` asks which rendered artifacts this batch invalidates.
  Tracing `docs/assets/easy-scene-showcase/auto-exposure-presets.jpg` showed it
  is produced by `Renderer::headless_gpu` with `set_auto_exposure`, so it runs
  through `apply_managed_auto_exposure_after_render` — the path the first `R01`
  fix had started damping.
- `the defect I introduced`: `src/render/frame.rs:135-142` renders, meters,
  re-renders, then breaks on `auto_exposure_attempted`. It is a **one-shot**
  correction with no later frame. Damping it by 0.5 meant every headless
  auto-exposure render shipped at half the metered correction. The original
  1.9.0 bug was a different one — assigning the relative correction as an
  absolute EV, discarding `current_ev`, which is what made the loop reflect.
- `fix`: `next_feedback_exposure_ev` now takes the step size.
  `ONE_SHOT_AUTO_EXPOSURE_SMOOTHING = 1.0` for the headless path (correct: the
  step never repeats, so stability is not at stake and accuracy is);
  `SURFACE_AUTO_EXPOSURE_SMOOTHING = 0.5` for the continuous attached-surface
  loop (correct: it re-meters every frame, and damping absorbs tonemapper
  nonlinearity and sample noise).
- `red proof`: setting the one-shot constant back to `0.5` produced
  `the one-shot path must apply the whole correction: expected 1, got 0.5`.
  Reverted.
- `test`: `one_shot_applies_the_whole_correction_and_the_surface_loop_damps_it`.
  It also asserts the damped step moves *less far* but in the *same direction*,
  and pins the fixture inside the configured EV range so it measures the step
  size rather than the clamp.
- `both original R01 proofs still pass`:
  `surface_auto_exposure_converges_instead_of_oscillating` and
  `feedback_exposure_never_reflects_around_the_current_value` — the continuous
  loop is unchanged. 9 exposure tests pass.

---

### R02 — Explicit scene selection must not be silently ignored

Owner: `src/assets/gltf/scene_selection.rs`.

Closes `N11`. Verified: the error is guarded by
`if !scenes.is_empty() && !matches!(selection, GltfSceneSelection::Default)` at
`scene_selection.rs:31`. When the document has **no** scenes *and* an explicit
index/name was requested, the request is dropped without diagnostic.

- [x] Add a fixture with zero scenes and an explicit index request; prove the
      baseline succeeds silently.
- [x] Raise a structured error naming the requested selection and the empty
      scene table, independent of `scenes.is_empty()`.
- [x] Keep `Default` selection on an empty table behaving as today.

Validation ledger:

- `focused red`: `cargo test --test gltf_validation_contracts -- explicit_gltf_scene_request_fails_closed_when_the_document_has_no_scenes`
  FAILED — the explicit index-0 request returned a `SceneAsset` with
  `roots: [0]` and `selected_gltf_scene: None`, silently substituting the
  root-node fallback for the scene the caller asked for.
- `classification`: product.
- `implementation`: `src/assets/gltf/scene_selection.rs` — dropped the
  `!scenes.is_empty()` guard so an explicit request fails closed regardless of
  table size, with an empty-table-specific candidate message.
- `focused green`: `cargo test --test gltf_validation_contracts` → 5 passed,
  0 failed (including both pre-existing selection contracts).

Scoped gate: focused fixture test + glTF conformance/import target.

### R03 — sRGB mip filtering happens in encoded space

Owner: `src/render/gpu/material_mips.rs`.

Closes `N15`. Verified: `downsample_rgba8_mip` (`material_mips.rs:22`) feeds
raw RGBA8 bytes to `image::imageops::resize` with `FilterType::Triangle`. For
sRGB-encoded textures this averages non-linear values, darkening mips and
shifting color.

- [x] Add a focused red test with a high-contrast sRGB texture proving the
      generated mip differs from a linearized-filter reference beyond
      tolerance.
- [x] Linearize → filter → re-encode for sRGB-format textures; leave linear and
      data textures (normal, ORM, etc.) filtering in their native space.
- [x] Prove data-texture (non-sRGB) mips are byte-unchanged by this fix.
- [x] Confirm which existing byte-pinning tests must be re-baselined and
      record why the new bytes are correct.

Validation ledger:

- `focused red`: `cargo test --lib -- render::gpu::material_mips` →
  `srgb_mip_downsample_filters_in_linear_light` FAILED with
  `expected ~188, got 128` for a black/white sRGB pair — a 60-level error, and
  exactly the distance-darkening the audit described.
- `classification`: product.
- `implementation`: `src/render/gpu/material_mips.rs` decodes sRGB sources to
  linear, filters with the *identical* Triangle kernel, and re-encodes; alpha
  is linear in both encodings and is never transformed. `srgb` is threaded
  from `upload.format`/`template.format` at both call sites
  (`materials.rs`, `material_batched.rs`).
- `focused green`: identical command → 5 passed, 0 failed.
- **No pinned test needed re-baselining.** Both existing pins
  (`material_texture_mip_downsample_averages_rgba8_pixels`, 128, and
  `material_texture_mip_downsample_4x4_checker_pins_midgrey`, ~130) operate on
  raw RGBA buffers with no declared colour space. They now pass `srgb: false`
  and keep their exact byte assertions, which is correct: an undeclared buffer
  is data, not sRGB. Only textures the uploader declares `Rgba8UnormSrgb`
  change.

Scoped gate: focused mip tests + the smallest rendered comparison on each
affected backend.

### R04 — Authored morph weight width is unvalidated

Owner: `src/scene/morphs.rs`, geometry morph consumer.

Closes `N18`. Verified: `set_morph_weights` (`morphs.rs:15-18`) validates only
`self.nodes.contains_key(node)`. Geometry then consumes zipped weights, so a
too-short or too-long vector truncates silently.

- [x] Add a focused red test setting a weight vector narrower and wider than
      the node's morph-target count; prove silent truncation today.
- [x] Validate the width against the target's morph-target count and return a
      structured `LookupError` naming expected vs supplied width.
- [x] Confirm the import path (`set_initial_morph_weights`) stays consistent
      with the authored path or document the deliberate difference.

Validation ledger:

- `focused red`: `cargo test --lib -- geometry::morph::r04_tests` FAILED —
  `morphed_vertices(&[1.0])` against a two-target geometry returned `Some`,
  having zipped the single weight against the first target and reported
  success.
- `classification`: product.
- `implementation`: `src/geometry/morph.rs` adds
  `morph_weight_width_matches`; `morphed_vertices` and `morphed_tangents` now
  fail closed on a mismatch instead of zipping.
  `src/scene/morphs.rs` rejects non-finite weights and any width that differs
  from the width already established for the node, via two new
  `LookupError` variants (`InvalidMorphWeights`,
  `MorphWeightWidthMismatch`) with `Display` and `help` arms.
- **Architectural note:** geometry lives in `Assets`, not `Scene`, so the
  authored entry point cannot see the true morph-target count. Validation is
  therefore split — `Scene` checks finiteness and the established width,
  `GeometryDesc` checks the true count at consumption. This is recorded rather
  than pretended away.
- `focused green`: identical command → passed;
  `cargo test --test c04_gltf_deformation_contracts` → 26 passed, 0 failed,
  confirming glTF import still supplies exact-width weights.

Scoped gate: focused morph tests + `c04_gltf_deformation_contracts`.

### R05 — Deprecated `rebind` panic surface

Owner: `src/animation.rs`.

Closes `N17`. Low severity and intentional: a deprecated compatibility wrapper
around `try_rebind`. It remains a panic surface on a public API.

- [x] Decide and record one of: keep with an explicit documented panic
      contract, or remove in a future major.
- [x] If kept, document the panic condition in rustdoc and reference
      `try_rebind` as the non-panicking replacement.

**Decision: keep, with a documented panic contract.** Removing it now would be
a breaking change on a 1.x line, and it cannot be made non-panicking without
changing its return type. It carries a `# Panics` section naming the exact
conditions, points at `try_rebind`, and is marked for removal in the next
major.

Validation ledger:

- `focused`: `cargo test --lib -- animation::validation_tests` → 4 passed,
  including the new
  `deprecated_rebind_panics_where_try_rebind_returns_an_error`, which pins the
  contract behaviorally: `try_rebind` returns `Err` on a non-finite rebound
  value while `rebind` panics on the same input. A future refactor cannot
  quietly change it.
- `classification`: policy/documentation; no runtime behavior changed.

---

## 2. Agent-surface correctness and diagnostics

### G01 — Define overlay clipping semantics

Owner: `src/scene/overlay_ownership.rs`, `src/render/cpu_geometry.rs`,
`src/render/cpu_labels.rs`, recipe schema.

Closes the architectural half of `B1`. Verified: `point_is_clipped`
(`cpu_geometry.rs:291`) applies the section box uniformly;
`draw_label_atlas_cpu` (`cpu_labels.rs:8-24`) threads it into every label quad.
`overlay_ownership.rs` knows which nodes are overlays but is `pub(super)` to
`scene` and is never consulted by `src/render/`.

No document in `docs/` specifies whether a section box *should* clip
annotations. The semantic is unspecified, not intended.

- [x] Add a focused red test proving an annotation label outside an active
      section box is currently absent from the frame.
- [x] Introduce an explicit per-overlay `clip_with_scene` semantic.
- [x] Default annotation labels and leader/dimension overlays to
      `clip_with_scene: false`.
- [x] Keep explicit opt-in so a section view can intentionally clip
      annotations.
- [x] Expose overlay ownership to `src/render/` (narrow accessor, not a
      visibility widening of the whole module).
- [x] Document the semantic in `docs/schema-contracts.md`.
- [x] Prove the opt-in path still clips.
- [x] **Doc naming corrected during the final review.** The changelog and agent
      guide first named the opt-in `clip_with_scene`, which is the *field* and
      getter. The call a caller writes is
      `LabelDesc::with_scene_clipping(true)`, as `docs/schema-contracts.md`
      already stated. User-facing prose now names the method; a reader copying
      `clip_with_scene` would not have compiled.

Validation ledger:

- `focused red (labels)`:
  `cargo test --test m2_lighting_depth_clipping -- section_box_does_not_clip`
  → `left: 8  right: 1568`. The label rendered 1568 pixels unsectioned and 8
  sectioned — 99.5% destroyed.
- `focused red (leader geometry)`: same target, with the consumer branches
  temporarily forced to the pre-fix `if true` → `left: 0  right: 236`. The
  dimension line was erased completely.
- `classification`: product.
- `implementation`: `clip_with_scene` threaded as an explicit semantic through
  `LabelDesc` → `PreparedLabelQuad` → `draw_label_atlas_cpu`, and
  `GeometryPrimitiveSource` → `StrokeBakeInputs`/`StrokeSegmentStyle` →
  `PreparedStrokeSegment`/stroke-quad `PreparedPrimitive` → `draw_strokes_cpu`
  and the per-primitive raster context in `cpu_render.rs`.
  `Scene::is_overlay_owned_node` is a narrow `pub(crate)` accessor over the
  existing registry — the module's visibility was not widened.
- `focused green`: 2 passed, 0 failed. `label_text` 10/10,
  `measurement_overlays` 2/2, `c10_overlay_ownership` 2/2.
- `scoped`: `cargo fmt --all --check` clean;
  `cargo clippy --all-targets --all-features -- -D warnings` exit 0.

**Architecture correction.** This row assumed leader/dimension lines were
primitives. They are not: `auto_instance.rs:103` and
`primitives.rs:54` both divert `Line | Wireframe | Edge` materials, so
annotation lines are prepared as *strokes*, which emit both a
`PreparedStrokeSegment` and screen-space stroke-quad `PreparedPrimitive`s.
Both are clipped in `cpu_strokes.rs`/`cpu_render.rs`, so both needed the
exemption. A label-only fix would have left a section view with legible
dimension text attached to an erased line — precisely the "clipped
callout-leader stub" the round-3 audit observed.

**Oracle note.** The first version of the label test counted dark pixels and
**passed on unfixed code**, because `Renderer::headless` clears to a dark
background that matched the filter. Both tests are now differential: they
render the same scene with and without the section box and require the
annotation's footprint to be unchanged. This is the `G04` failure mode
occurring inside `G01`'s own proof.

Scoped gate: focused render tests + rendered-output proof. Unit tests alone
cannot close a browser-visible rendering change.

### G02 — Fix the shipped `cad-plate` template content

Owner: `src/bin/scena/examples_agent/starter.rs`, `src/scene/measurements.rs`.

Closes the content half of `B1` and all of `B4`. Verified: the recipe places a
tight section box alongside the measurement, callout, and label at
`starter.rs:83`. `measurements.rs:210` always formats `"{label}: {value}"`
while the template supplies the value *as* its label, rendering
`120.0 mm: 120.0 mm`.

- [x] Change the template measurement label to a semantic one
      (`plate width` → `plate width: 120.0 mm`).
- [x] Make the template's annotations visible in the rendered frame — via
      `G01`'s default, and by re-authoring the box if still needed.
- [x] Decide whether `label_text` should suppress a duplicated suffix when the
      supplied label already ends with the formatted value; record the
      decision either way.
- [x] Re-render and confirm `CAD plate`, `datum A`, and the dimension text all
      appear.

**Decision on `label_text`:** left as `"{label}: {formatted_value}"`. The
renderer owns the magnitude and unit; the label names *what* is measured. A
suffix-suppression heuristic would silently change output for any caller whose
label legitimately ends in a unit word, and would hide authoring mistakes
rather than surface them. The template is corrected instead, and
`shipped_template_measurement_labels_are_semantic_not_duplicated_values`
prevents the pattern returning to any shipped template.

Validation ledger:

- `focused red`:
  `cargo test --features agent --test scena_cli_agent_templates -- shipped_template_measurement_labels`
  FAILED — `template cad-plate measurement String("plate-width") uses the
  formatted value "120.0 mm" as its label, which renders as
  "120.0 mm: 120.0 mm"`.
- `classification`: product (shipped template content).
- `implementation`: `src/bin/scena/examples_agent/starter.rs:89` label
  `"120.0 mm"` → `"plate width"`.
- `focused green`: identical command → passed. The re-rendered template shows
  `CAD plate`, `datum A`, and `plate width: 120.0 mm`; annotation visibility
  came from `G01` with no further template change needed.

Depends on `G01`. Scoped gate: `G04`'s new oracle must pass.

### G03 — Introspection must report non-contributing entities

Owner: `src/render/introspection.rs`, `src/render/introspection/types.rs`.

Closes `B2` and `B3`. Verified: `introspection.rs:149` gives every visible node
an empty reason list; `drawn` is only `inspection.draw_list.len()`
(`introspection.rs:355/362`). `--detail` returns 14 visible nodes with
`reason_codes: []` while three `Label` nodes contribute zero pixels.

- [x] Add a focused red test proving a visible node that cannot reach the
      frame is reported with `reason_codes: []` today.
- [x] Track prepared / contributing / rejected state for authored labels,
      callouts, measurements, and nodes.
- [x] Emit reasons: `clipped_by_active_clipping_plane`, `behind_camera`,
      `outside_frustum`, `all_culled`, plus the node-level vocabulary
      (`node_hidden`, `parent_hidden`, `zero_scale`, `layer_masked`,
      `missing_geometry`, `missing_material_upload`, `alpha_zero`,
      `transparent_material`).
- [x] Do **not** fail every intentionally occluded node — that would create
      noise. Start with annotations and explicitly required entities.
- [x] Make declared template expectations load-bearing: fail when a *required*
      annotation disappears.
- [x] Surface the render → `diagnose` path in the render result so an agent
      does not need the handle in advance.

Validation ledger:

- `focused red`:
  `cargo test --features inspection --test render_introspection_contracts -- detail_reports_why_a_visible_node`
  FAILED — `a visible node that cannot reach the frame must carry a reason
  code, got []`. Re-confirmed with the **final** test form against the
  pre-fix `reason_codes` expression, so the red is not an artifact of editing
  the test between runs.
- `classification`: product (diagnosability).
- `implementation`: `src/render/introspection.rs` replaces the
  `node.visible`-only derivation with `node_reason_codes`, which reuses the
  existing per-node visibility diagnosis rather than duplicating its
  vocabulary.
- `focused green`: identical command → 7 passed, 0 failed.

**Noise control — a real defect found while implementing.** The first working
version attributed `clipped_by_active_clipping_plane` to *every* node,
including cameras and lights. A camera cannot be clipped, so that is noise
rather than diagnosis. Reason codes are now emitted only for node kinds that
can rasterize, and a scene-level cause is reported against a drawable only
when its `affected_handles` are empty or name that node. The test pins this:
`Camera`, `Light`, and `Empty` entries must carry no render-visibility
reasons.

**Honest limit, documented in the code.** These are *candidate causes*, not
confirmed per-node pixel attribution — the renderer does not track which node
produced which pixel outside the semantic-AOV path. That limitation is stated
in the rustdoc so no reader mistakes a candidate for a confirmation.

**Doctor regression caught and fixed in the same change.** The `G01` closure
pushed `src/render/cpu_render.rs` to 501 significant lines, tripping
`ARCH-KISS-SIZE` (limit 500). Relocating the helper into `cpu.rs` merely moved
the violation (505), so the clip context and its per-primitive narrowing were
extracted into a new `src/render/cpu_clip.rs`. Doctor is back to its baseline
finding set.

### G04 — Template acceptance needs a real oracle

Owner: `tests/scena_cli_agent_templates.rs`.

Closes independent finding 2. Verified: the CAD template test *does* render,
but asserts only `command_output.status.success()` (~`:364`) and schema/`ok`.
This is a textbook green test preserving broken output — it is the reason
`B1` shipped.

- [x] Add pixel/content assertions, not only `ok == true`.
- [x] Assert each declared annotation's text is actually present (glyph
      coverage or a per-entity contributing-pixel count from `G03`).
- [x] Prove the new oracle fails on the pre-`G02` template.
- [x] Apply the same oracle shape to every shipped template that declares
      annotations.

Validation ledger:

- `focused red`: with the `G01` consumer branches temporarily forced back to
  `if true` (reproducing the shipped v1.9.0 product),
  `shipped_template_annotations_contribute_pixels_to_the_rendered_frame`
  FAILED — `declared label/callout badges must reach the frame; found 0 pixels
  of [29, 39, 51] in 640x480`.
- **The oracle gap, demonstrated side by side.** On that *same* broken
  product, the pre-existing
  `scena_examples_agent_templates_generate_and_run_cli_smoke_commands`
  **passed** (171.50s, 1 passed). Old oracle green, new oracle red, identical
  product state — this is exactly how `B1` shipped through a green pipeline.
- `classification`: harness (test oracle), not product.
- `implementation`: `tests/scena_cli_agent_templates.rs` decodes the PNG the
  CLI actually wrote and requires the declared label/callout badge colour and
  the declared dimension colour to reach the frame. `png` added to
  `[dev-dependencies]` (it was a normal dependency, unreachable from an
  integration test); the public dependency surface is unchanged.
- `focused green`: restored product → 2 passed, 0 failed.
- `note`: the oracle asserts on declared-annotation colours rather than glyph
  shapes, so it is text-content-agnostic and applies to any template declaring
  annotations. Per-entity contributing-pixel attribution arrives with `G03`
  and can replace the colour heuristic then.

### G05 — `nodes_summary` counts different populations

Owner: `src/render/introspection.rs`.

- [x] `visible` and `drawn` measure different populations (`drawn` excludes
      overlay/label work). Either rename to make the populations explicit, or
      document the difference inline in the schema, or report both populations
      consistently. Record the choice.

**Choice: report both populations.** Renaming `visible` would break the stable
contract; documentation alone still leaves the misleading comparison one
subtraction away. `nodes_summary` now carries `visible_drawable` — the
population `drawn` is drawn from — so an agent can compare like with like.

Validation ledger:

- `focused red`:
  `cargo test --features inspection --test render_introspection_contracts -- nodes_summary_exposes_a_population`
  failed to compile — `no field visible_drawable on type
  RenderIntrospectionNodesSummaryV1 ... available fields are: visible, hidden,
  drawn, culled, transparent, failed_material`.
- `implementation`: `visible_drawable`, populated from
  `inspection.counts.visible_drawable`, `#[serde(default)]` so older fixtures
  deserialize. Additive only.
- `focused green`: passed. Triggered `T07`; both golden fixtures updated in the
  same change.

### G06 — `--detail` is undocumented

Owner: `src/bin/scena/help.rs`, `src/bin/scena/recipe.rs`.

- [x] `--detail` appears 0 times in `scena --help`. Add it to the declared
      command surface an agent is told to read.
- [x] Audit for other flags accepted by a command but absent from `--help`.

Validation ledger:

- `focused red`:
  `cargo test --features inspection --test scena_cli_agent -- help_declares_every_accepted`
  FAILED — ``scena --help` must declare the accepted `recipe render` flag
  --detail`.
- `implementation`: `--detail` added to both `RECIPE_RENDER_COMMAND` and the
  usage list in `src/bin/scena/help.rs`.
- `focused green`: passed. The test asserts the whole accepted flag set, so a
  future flag added without a help entry fails here.

### G07 — Envelope-only validation returns `ok: true`

Owner: `src/contract_validation.rs`.

Closes `N23`. Verified design sharp edge: `contract_validation.rs:137`
discloses `validation_level: envelope`, but `ok: true` is unsafe for an agent
that does not inspect the level.

- [x] Make the result unambiguous: either gate `ok` on full validation, or
      rename the field so a partially validated result cannot read as fully
      valid.
- [x] Prove an agent reading only `ok` cannot conclude "fully validated".

**Both options offered by this row were rejected, with reasons.** Gating `ok`
on full validation would make `scena validate` exit **65** for every
envelope-only contract (`src/bin/scena/validate.rs:14` maps `!ok` → 65),
turning a working validation into a failure for every agent using it today —
a much larger break than the ambiguity it fixes. Renaming `ok` breaks the
published contract outright.

Instead the report gained `fully_validated`, and the partial-validation limit
was promoted from a prose `limitations` string to a structured **warning
diagnostic** (`envelope_validation_only`). An agent now has two machine
signals rather than one prose note, `ok` keeps its documented meaning ("the
checks that ran passed"), and no exit code changes.

**Residual, stated plainly:** `ok: true` alone is still insufficient to
conclude full validation — that is inherent to keeping `ok` compatible. It is
now documented directly on the field, and `fully_validated` is the field to
key on. If the owner prefers the harder fail-closed reading, flipping `ok` is
a one-line change plus an exit-code migration note.

Validation ledger:

- `focused red`:
  `cargo test --features agent --test a09_generic_validation -- envelope_only_validation`
  failed to compile — `no field fully_validated on type
  ContractValidationReportV1`.
- `classification`: product (contract clarity).
- `implementation`: `src/contract_validation.rs` adds `fully_validated`
  (`#[serde(default)]`, so an older fixture deserializes to the fail-closed
  `false`) and the `envelope_validation_only` warning diagnostic.
- `focused green`: `cargo test --features agent --test a09_generic_validation`
  → 5 passed, 0 failed. `stable_contracts` 63/63 — this change did not
  re-trigger `T07`.
- `note`: the first draft of the test picked
  `scena.capability_report.v1`, which *is* typed-validated, so it asserted
  nothing. `scena.render_introspection.v1` is genuinely envelope-only and is
  what the test now uses.

### G08 — Agent response economics

Owner: `src/bin/scena/output.rs`, `src/schema_catalog/agent_guide.rs`.

Closes `A1`, `A2`, `A3`. Measured on this tree: render response 3,272 compact
bytes of which `policy` is 1,325 (40%); `guide agent --json` 29,891 bytes of
which `markdown` is 27,864 (93.6%); `--compact` is whitespace-only (16 keys
either way).

- [x] Emit a `policy_digest` by default; move the full block behind
      `--include policy`.
- [x] Add `guide agent --contract` that omits `markdown`.
- [x] Add `--fields ok,reasons,fixes` projection.
- [x] Keep every existing schema field reachable; this is response *shaping*,
      not removal. Old fixtures must still deserialize.

Measured result on the shipped `cad-plate` template (compact bytes):

| response | before | after |
|---|---:|---:|
| `recipe render` (default) | 3,272 | **2,044** |
| `recipe render --include policy` | 3,272 | 3,379 |
| `recipe render --fields ok,reasons,fixes` | 3,272 | **434** |
| `guide agent --json` | 29,891 | 29,891 |
| `guide agent --contract` | — | **1,219** |

A default render turn drops ~38%; a projected verification turn drops ~87%;
the machine-readable guide drops ~96%. Nothing was removed — `--include
policy` restores the block and `--json` still carries the prose guide.

Validation ledger:

- `focused red`:
  `cargo test --features agent --test scena_cli_agent -- agent_responses_support_projection`
  FAILED — `the constant policy block must not be repeated on every response`.
- `classification`: product (response contract).
- `implementation`: `src/bin/scena/output.rs` adds `recipe_policy_digest`,
  `--fields` projection (always retaining `schema` and `ok` so a projected
  response stays self-describing), and `--include policy`. Stripping happens
  in `apply_output_format` rather than at the 14 `add_recipe_policy_to_outcome`
  call sites, so no command needed threading. `src/bin/scena/guide.rs` adds
  the `--contract` form.
- `focused green`: identical command → passed; whole suite 21 passed, 0 failed.

**An oracle moved rather than weakened.** `scena_agent_cli_stdout_matches_golden_fixtures`
asserted that `repair` output carries the full policy block with its schema,
network flag, and sandbox roots. That is no longer the default contract. Those
*content* assertions were moved onto the `--include policy` path in the G08
test, and the golden helper now requires a well-formed `sha256:` digest **and**
asserts the full block is absent by default. Coverage is strictly greater than
before: the same content is still checked, plus digest well-formedness and
default-omission, which were previously unchecked.

---

## 3. Test and CI coverage bijection

### T01 — Feature-gated integration binaries are orphaned

Owner: `.github/workflows/`, `Cargo.toml`.

Closes `N2`, `N6`, part of `N5`, and independent finding 1. Because
`default = []` (`Cargo.toml:108`), a default `cargo test` executes **zero**
tests from a feature-gated binary.

Independently enumerated on this tree: **34** crate-level feature-gated
integration binaries, **5** named by a workflow, **29 orphaned**:

```
a02_recipe_policy_cli            a08_transform_grammar    fr07_recipe_diff
a03_llm_guide_smoke              a09_generic_validation   fr08_recipe_spatial_state
a06_repair_doctor_inputs         a11_authored_node_placement
a07_name_candidates              appearance_introspection_contracts
a08_default_introspection        c07_handle_namespaces    capture_contracts
connector_browser_contracts      contact_grounding        fr02_recipe_build_cli
fr04_cli_schema_matrix           fr05_capture_sequence    material_variant_helpers
m8_compressed_asset_release_proof                         presentation_timeline
product_configurator_helpers     render_introspection_contracts
round_c_khronos_samples          round_d_asset_hot_reload scena_cli_interaction
scene_host                       visibility_diagnosis_contracts
visual_repair_contracts
```

(The round-3 audit counted 28; the one-item delta is classification, not
substance.)

- [x] Add a Linux feature-contract lane covering all feature-gated integration
      binaries — preferably `cargo test --workspace --all-features --tests` if
      runtime is acceptable.
- [x] Otherwise maintain an explicit workflow test manifest.
- [x] `capture_contracts`, `gltf_validation_contracts`, and A03-A05 in agent
      mode are mandatory entries.
- [ ] Record the runtime cost of the new lane. **Blocked on this host** — see
      below.

Validation ledger:

- `focused red`: doctor reported 29 `TESTS-FEATURE-GATED-WORKFLOW-BIJECTION`
  findings (`T02`).
- `implementation`: `.github/workflows/ci.yml` gains a
  `linux-feature-contract` job running
  `cargo test --workspace --all-features --tests`. The blanket form was chosen
  over an explicit manifest so a newly added gated binary is covered
  automatically; `T02`'s rule still fails if someone replaces the lane with a
  narrower command that misses a binary.
- `focused green`: doctor reports 0 such findings; all 29 named binaries are
  covered, including the three the audit called mandatory.
- `blocked`: the lane's **runtime cost is unmeasured here**.
  `--all-features` enables `ktx2`, which builds `basisu_c_sys` — a native C
  encoder — and the compile exceeded a 10-minute budget twice on this Pi 5
  host without finishing. The command's *correctness* as a bijection-closing
  lane is proven by doctor; its wall-clock must be recorded from the first CI
  run. If it proves too slow, `T02`'s rule accepts an explicit
  `--test <stem>` manifest instead, and that fallback is already implemented.

### T02 — Doctor must enforce the bijection

Owner: `crates/xtask/src/app/doctor_core/`.

- [x] Extend doctor so every crate-level feature-gated integration binary must
      map to a workflow command.
- [x] Checking that the function text exists is insufficient — the rule must
      bind binary → executed workflow command.
- [x] Prove the rule fails when a new gated binary is added without a workflow
      entry.

**The pre-existing rule was the weak form this row describes.**
`TESTS-FEATURE-GATED-CONTRACT-SUITES` checks only that a command *string*
appears in `docs/checklists/application-builder-roadmap.md`, and only for
files whose path contains `contract` plus two named exceptions. Prose in a
checklist is not execution, and the name filter excluded most of the orphans.

The new `TESTS-FEATURE-GATED-WORKFLOW-BIJECTION` binds binary → workflow: it
enumerates every `tests/*.rs` with a crate-level `#![cfg(...)]` requiring a
cargo feature and requires `.github/workflows/` to either name it with
`--test <stem>` or carry a blanket `--all-features --tests` lane.

Crucially it ignores gates like `not(target_arch = "wasm32")`, which are
always true on a Linux runner — those binaries *do* run by default and are not
orphans. Conflating them is what produced a 68-file false count during
planning before the filter was corrected.

Validation ledger:

- `focused red`: `cargo run -p xtask -- doctor --full` reported **29**
  `TESTS-FEATURE-GATED-WORKFLOW-BIJECTION` findings — exactly the independent
  count established during planning, naming `a02_recipe_policy_cli`,
  `a03_llm_guide_smoke`, `a06_repair_doctor_inputs`, and 26 more.
- `classification`: harness (coverage enforcement).
- `implementation`:
  `crates/xtask/src/app/doctor_core/feature_gated_tests.rs`.
- `focused green`: after `T01`'s lane landed, the same command reports **0**
  such findings and doctor returns to its baseline 6.

### T03 — Missing mesh+stroke retained re-entry test

Owner: retained-culling tests.

Closes `N16`. Mesh and stroke tests exist separately; the original multi-list
re-entry case is absent.

- [x] Restore a focused test covering re-entry across both draw lists.

Validation ledger:

- `test`: `render::prepare_lifecycle::support::tests::retained_re_entry_restores_mesh_and_stroke_draw_lists_together`.
  It drives the real dynamic-path entry point `Renderer::reencode_retained_draws`,
  which is pure with respect to `self.prepared` and needs no GPU adapter, so the
  proof is deterministic on any host.
- `scene`: one triangle mesh (`GeometryDesc::box_xyz` + unlit material →
  primitive draw list) and one line mesh (`GeometryDesc::line` +
  `MaterialDesc::line` → stroke draw list), both visible, then both hidden, then
  both restored.
- `red proof`: mutating `reencode_retained_draws` to pass strokes through
  unfiltered (`prepared.retained_strokes.to_vec()`) produced
  `assertion left == right failed: leaving the active camera must drop the
  primitive list and the stroke list together / left: (0, 1) / right: (0, 0)` —
  the mesh list dropped, the stroke list did not.
- `blindness confirmed`: under that mutation `cargo test --lib render::` reported
  `262 passed; 3 failed`, and the only new failure was this test. The other two
  are the pre-existing `RequestDevice { backend: HeadlessGpu }` adapter failures.
  The whole existing render suite cannot see an asymmetric multi-list
  regression, which is exactly `N16`.
- `green`: mutation reverted, test passes.

### T04 — 256px reference lacks an automated external anchor

Owner: Q11 reference promotion.

Closes `N7`. Promotion binds human review to the Blender anchor, but no
automated 256px-vs-independent-render pixel assertion exists.

- [x] Add an automated assertion, or record an explicit, justified deferral
      naming what human step substitutes and why that is acceptable.

Resolution: **automated assertion added**, not deferred.

Validation ledger:

- `test`: `q11_cpu_reference_agrees_with_independent_blender_anchor_on_material_families`
  in `tests/q01_waterbottle_cpu_reference.rs` — the default lane, no GPU needed.
- `why an anchor was possible`: `reference_blender_cycles_512.png` (Blender 4.3.2
  Cycles, 128 spp) is bundled and is produced by no scena code path. The 512px
  scena-gold reference was already compared against it; the 256px CPU reference
  was not.
- `oracle`: mean RGB over a patch at matched framing coordinates (the 256px
  reference is sampled at half the anchor's coordinates), converted to hue.
  Each material must land in its declared hue band in **both** images, and the
  two renderers must agree within 19 degrees. Measured: body 48.2 deg (Blender)
  vs 58.3 deg (scena), cap 10.0 vs 0.0 — both about 10 deg apart.
- `why hue and not pixels`: the two renderers differ by roughly 2.5x in
  luminance (Blender body mean `[156, 139, 70]`, scena `[58, 56, 6]`) because
  of different tonemapping and lighting. Any pixel- or luminance-based
  comparison would have to be loosened until it proved nothing. Hue is the
  property `reference_metadata.toml` already claims agreement on.
- `why 19 degrees is not fitted to the data`: it is half the 38-degree body/cap
  hue separation the anchor itself exhibits, so a pass can never be produced by
  confusing the two materials. It is not the measured 10 rounded up.
- `oracle is discriminating (permanent, in-test)`: a channel-rotated anchor —
  which preserves luminance and contrast exactly — must fail every material
  band, and a live wrong-material render must fail every band. Both are
  asserted, so the test cannot silently degrade into an always-pass.
- `red proof`: rotating the decoded 256px reference's channels produced
  `bottle body 256px reference sample [6.1, 57.7, 56.2] has hue 178.3 deg,
  outside the declared olive/yellow band (40.0, 70.0) — the committed reference
  renders the wrong material`.
- `blindness confirmed`: under that same mutation
  `q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders`
  and `q11_waterbottle_cpu_is_byte_deterministic_before_reference_comparison`
  both **passed**. That is `N7` exactly: every pre-existing assertion compares
  scena against scena, so a reference promoted from a wrong-material build
  satisfies all of them.
- `metadata updated in the same change`: `reference_metadata.toml` now records
  `external_anchor_test` and `external_anchor_policy`, so the human
  `external_anchor_reviewed` attestation is documented as additional review
  rather than the only anchor.
- `green`: mutation reverted; all 3 tests in the file pass.

### T05 — A03/A04/A05 agent-mode coverage

Closes the accurate part of `N5`. A03's agent-enabled smoke is absent from CI;
A04/A05 run under default `cargo test` but not the complete installed
agent-feature path. All three pass with `--features agent`.

- [x] Cover the installed agent-feature path in CI (folds into `T01`).

Validation ledger:

- `lane`: the `linux-feature-contract` job added by `T01` runs
  `cargo test --workspace --all-features --tests`. A03 is
  `#![cfg(all(not(target_arch = "wasm32"), feature = "agent"))]`, so
  `--all-features` is what makes it run at all.
- `A04/A05 agent surface`: neither is feature-gated, so both already ran under
  default `cargo test` — but against a *default-feature* binary. Under
  `--all-features`, `CARGO_BIN_EXE_scena` is the agent-featured binary, so the
  lane exercises the installed agent command surface. No new environment
  variable is needed: `a04_packaged_cli_contract.rs:16` falls back to
  `cfg!(feature = "agent")` when `SCENA_A04_EXPECT_AGENT` is unset, so the smoke
  asserts the matching contract automatically.
- `verified locally`:
  `cargo test --features agent --test a03_llm_guide_smoke --test a04_packaged_cli_contract --test a05_public_agent_guide`
  → 3 passed, 0 failed.
- `defect this row surfaced`: the first run of that command failed A05 at
  `assertion failed: markdown.status.success()`. `guide agent --markdown` was
  exiting 70. That is `X02`, and `G08` had widened it from
  `--markdown --compact` to plain `--markdown`; see the `X02` ledger. This is
  the concrete evidence that the orphaned-binary gap was hiding real breakage —
  A05 runs in the default lane but its Markdown assertion had never been run
  against the post-`G08` binary in any workflow.

### T06 — Doctor pinned substrings must move in lockstep

Owner: `crates/xtask/src/app/doctor_core/recipe_policy.rs`,
`crates/xtask/src/app/doctor_easy_scene/next_release.rs`.

`CLAUDE.md` warns that doctor's truth substrings pin contract text in specific
files, and that rewriting the underlying file requires updating the pins in
lockstep. Verified pins that this batch will disturb:

- `recipe_policy.rs:88` pins `src/render/exposure.rs`.
- `recipe_policy.rs:159` pins `"pub const fn auto_exposure"`.
- `next_release.rs:146` pins `src/render/exposure.rs`.
- `next_release.rs:158-171` pin `tests/round_c_auto_exposure_presets.rs`, its
  test names, and the auto-exposure docs image slug.
- `doctor_docs/contract_pins.rs:155` pins `camera_position_exposure: vec4<f32>`.

`R01` rewrites `exposure.rs`. `G03`/`G08` rewrite introspection output.

- [x] Re-run doctor and update every pin the change invalidates **in the same
      commit as the code change**.
- [x] Do not silence a pin by deleting it. If a pinned contract genuinely no
      longer applies, record why in the row.

Validation ledger — the pins this batch actually disturbed were **not** the ones
this row predicted:

- `predicted and not disturbed`: `recipe_policy.rs:88`/`:159`,
  `next_release.rs:146`/`:158-171`, and
  `doctor_docs/contract_pins.rs:155`. `R01` changed the *body* of
  `src/render/exposure.rs` but not the pinned strings (`pub const fn
  auto_exposure`, the test names in `tests/round_c_auto_exposure_presets.rs`,
  the docs image slug, or the WGSL `camera_position_exposure: vec4<f32>`
  declaration). All still pass.
- `disturbed, and each fixed in the same change as its cause`:
  1. `A09-FEATURE-DISCOVERABILITY` pinned the *old* SHA-256 of the CLI process
     table (`feature_discoverability.rs:231`). `G06` added `[--detail]` to
     `RECIPE_RENDER_COMMAND`, which changed the digest. Pin updated
     `5b04b0e3…` → `af63ca1f…`, together with
     `tests/assets/cli-golden/process_contract_table.sha256` and the literal
     command string in `tests/a10_cli_contract_table.rs`.
  2. `A07-NAME-CANDIDATES-REMEDIES` pinned `"CliError::classify("` in
     `src/bin/scena.rs` (`name_candidates.rs:87`). `X01` moved the dispatch site
     to `CliError::from_failure`. The pin **followed the call** rather than
     being deleted: it is now `"CliError::from_failure("`, with a comment
     explaining that `from_failure` classifies on the typed kind and still
     passes structured candidates through. The contract the pin protects —
     candidates reaching the emitted error — is unchanged and still enforced.
  3. `ARCH-KISS-SIZE` fired on `src/bin/scena/place.rs` at 503 significant lines
     (cap 500) after `X01`'s typed-error expansion. Closed by extracting
     `src/bin/scena/resource_uris.rs` (49 lines of URI rebasing, a cohesive
     unit) rather than by golfing the code or raising the cap; `place.rs` is now
     457.
- `no pin was deleted`.
- `process lesson`: all three were found **late**, by the post-`X01` sweep, not
  by the focused proof of the row that caused them. §0.2 step 6 ("run only the
  scoped gate implied by touched files") is too narrow when a file's *contents*
  are digested or pinned elsewhere. Recorded in `T07` as well.

### T07 — Golden and stable-contract fixtures must be re-baselined

Owner: `tests/assets/cli-golden/`, `tests/assets/stable-contracts/`.

Verified: both `tests/assets/cli-golden/render_introspection_stdout.json` and
`tests/assets/stable-contracts/render_introspection.v1.json` contain
`nodes_detail`, `reasons`, and `fixes` — precisely the fields `G03` rewrites.
`doctor_docs/stable_fixtures.rs:79-80` and `doctor_core/contracts.rs:191` pin
both paths.

- [x] Update the golden CLI fixture when `G03`/`G08` change the emitted shape.
- [x] Keep `stable-contracts` **backward compatible**: new reason codes and
      response-shaping fields must be additive, and the pre-change fixture must
      still deserialize. Prove it with the existing stable-contract test.
- [x] Record, per changed fixture, why the new bytes are correct.

Validation ledger (first trigger — `G05`):

- `trigger`: `G05` added `nodes_summary.visible_drawable`, and
  `cargo test --features inspection,scene-host --test stable_contracts`
  FAILED on `render_introspection_golden_matches_live_schema_serialization`.
  The diff was exactly one key: `"visible_drawable": Number(0)`. Nothing else
  in the report changed.
- `fixtures updated`: `tests/assets/stable-contracts/render_introspection.v1.json`
  and `tests/assets/cli-golden/render_introspection_stdout.json`, in the same
  change as the code that invalidated them.
- `why the new bytes are correct`: the field is purely additive and reports a
  count the renderer already computed (`inspection.counts.visible_drawable`).
  No existing field changed value. Backward compatibility is proven by the
  other 62 stable-contract assertions passing unchanged, including the
  older-fixture deserialization path, which works because the field carries
  `#[serde(default)]`.
- `note`: `G03`'s reason-code change did **not** trigger this row — the golden
  report has an empty `nodes_detail`, so the shape was unchanged there.

Validation ledger (second trigger — `G06`):

- `trigger`: `G06` added `[--detail]` to `RECIPE_RENDER_COMMAND` in
  `src/bin/scena/help.rs`. That string is pinned twice by
  `tests/a10_cli_contract_table.rs`: once as a literal in the agent-gate list,
  and once inside the SHA-256 digest of the whole `command_contracts` table.
- `how it was found`: **late, and not by the change that caused it.** The
  focused `G06` proof asserted `--detail` appears in `scena --help`; it did not
  run `a10`. The failure surfaced only when the post-`X01` sweep ran the CLI
  contract suites: `missing agent gate: recipe render ...` and
  `CLI process table changed ... left: af63ca1f… right: 5b04b0e3…`.
- `why the new bytes are correct`: `--detail` is genuinely accepted by
  `recipe render` (`src/bin/scena/recipe.rs:290`) and was undocumented — that is
  the entire defect `G06` closes. The declared surface now matches the parsed
  surface. Nothing else in the table moved: the only diff is the one flag.
- `fixtures updated`: the literal in `tests/a10_cli_contract_table.rs` and
  `tests/assets/cli-golden/process_contract_table.sha256`.
- `oracle not weakened`: the digest assertion is unchanged and still fails on
  any future unreviewed table change; it was re-baselined, not relaxed.
- `process correction`: "run only the scoped gate implied by touched files"
  (§0.2 step 6) is too narrow for `help.rs`, whose contents are digested by a
  test in a different file. Touching the declared command surface requires
  running `a10_cli_contract_table` as well.

### T08 — Visual references invalidated by rendering changes

Owner: `tests/visual/references/`, `docs/assets/`.

Three rows in this batch change rendered pixels: `R01` (exposure convergence),
`R03` (linear-space mip filtering), and `G01` (annotation clipping default).
Affected artifacts include the 13 `tests/visual/references/round_e/*.png`
material references and `docs/assets/easy-scene-showcase/auto-exposure-presets.jpg`,
which is itself doctor-pinned at `doctor_easy_scene/showcase_performance.rs:260`.

This row exists so the standing "never weaken an oracle" rule does not stall
the batch: a reference that changes because the renderer got **more correct**
must be re-baselined deliberately, with justification — not by widening a
threshold.

- [x] Enumerate every reference image and pinned docs image the batch changes.
- [x] For each, record which row changed it and why the new pixels are correct.
- [x] Re-baseline in the same commit as the causing row.
- [x] Confirm the Q11 256px reference and its approval metadata are either
      unaffected or re-promoted through the documented approval path — never
      silently overwritten.

Validation ledger — enumeration, then a per-artifact verdict:

**No committed image changed.** `git status --porcelain tests/visual/ docs/assets/
tests/assets/` lists four modified files, all text: the two CLI golden fixtures,
the stable-contract fixture, and `reference_metadata.toml`. Zero `.png`/`.jpg`.
That is a starting point, not the answer — a reference can be stale without the
test noticing.

| Artifact | Changed by | Verdict |
|---|---|---|
| `tests/visual/references/round_e/*.png` (13) | — | **Unaffected.** Pinned by SHA-256 in `round_e_material_fixture.toml` and generated by `scripts/generate_round_e_model_viewer_references.mjs` against a deployed browser page. They are external anchors, not local renderer output; `R01`/`R03`/`G01` cannot move them. |
| `tests/assets/gltf/khronos/WaterBottle/reference_cpu_256.png` | — | **Unaffected, and now better anchored.** Rendered by the CPU renderer, which populates `linear_frame` (`surface.rs:107-111`), so auto exposure takes the absolute path `R01` did not change. `R03` is scoped to GPU material mips. No section box, so `G01` does not apply. Byte-identical: its pinned SHA-256 still matches and `q01_default_cpu_waterbottle_matches_reference_and_rejects_known_bad_renders` passes. Approval metadata was **not** overwritten; `T04` added an automated Blender anchor alongside it without touching the image. |
| `tests/assets/gltf/khronos/WaterBottle/reference_512.png`, `reference_blender_cycles_512.png` | — | **Unaffected.** Byte-identical; consumed only by GPU lanes that do not run here. |
| `docs/assets/easy-scene-showcase/auto-exposure-presets.jpg` | `R01` | **Invalidated — regeneration blocked on this host.** See below. |
| Remaining 16 `docs/assets/easy-scene-showcase/*.jpg` | — | **Unaffected.** They render through `Renderer::headless` (CPU) or fixed exposure; none uses a section box with annotations. |

The one genuinely invalidated artifact:

- `docs/assets/easy-scene-showcase/auto-exposure-presets.jpg` is produced by
  `render_subject_with_exposure` (`examples/easy_scene_showcase.rs:612`), which
  uses `Renderer::headless_gpu` with `set_auto_exposure`. With a GPU there is no
  `linear_frame`, so it runs the feedback branch `R01` changed.
- **Tracing this is what found the `R01` one-shot defect** (recorded in `R01`).
  Under the first `R01` fix the regenerated image would have been *worse* —
  half-corrected. After the split it applies the full correction, so the
  regenerated image should be very close to the committed one; the residual
  difference is `R01`'s actual fix, replacing an absolute assignment with a
  correction relative to the current EV.
- **Blocked here:** `Renderer::headless_gpu` fails on this host with
  `RequestDevice { backend: HeadlessGpu }`, so the asset cannot be regenerated
  or compared. It must be regenerated on the GPU lane, in the same commit as the
  `R01` change, before release.
- **Not silently stale in the meantime:** the doctor rule that owns this file
  (`showcase_performance.rs:260`) checks measured statistics
  (`min_luma_stddev: 0.08`, `min_edge_mean: 0.0025`), not a pixel hash, so it
  validates the committed file rather than asserting a byte identity that is no
  longer true. The staleness is recorded here rather than hidden behind a
  passing gate.
- **No threshold was widened and no reference was re-baselined to obtain
  green.**

---

## 4. CLI contract typing

### X01 — Replace prose-based error classification

Owner: `src/bin/scena/cli_error.rs`.

Closes `N4`. Verified: `cli_error.rs:86` classifies by matching string
fragments. A malformed appearance expectation reproduced `invalid_arguments`,
exit 2.

- [x] Introduce typed error kinds; classify on the type, not the message text.
- [x] Prove a message-wording change cannot alter the exit class.
- [x] Keep the emitted `scena.cli_error.v1` shape stable.

Validation ledger:

- `types added` (`src/bin/scena/cli_error.rs`):
  - `CliErrorKind` — `InvalidArguments`, `InputNotFound`, `InvalidInput`,
    `Unsupported`, `Io`, `Internal`, `Runtime`, plus `Unclassified` for
    not-yet-migrated producers. `code_and_class()` is the single source of truth
    for `(code, exit_class)`.
  - `CliFailure { kind, message }` — the command error channel. Every command
    signature moved from `Result<CliOutcome, String>` to
    `Result<CliOutcome, CliFailure>` (79 signatures).
  - `CliUsageError` — the argument layer's error type. It is *only* convertible
    into `CliErrorKind::InvalidArguments`, so a usage error cannot be
    reclassified by rewording. 26 parser signatures moved onto it.
  - `CliError::from_failure` replaces `CliError::classify` at the dispatch site;
    `classify` is now reachable only for `Unclassified`.
- `real defect this surfaced`: `verify interaction` rejected an unknown action
  with `"unsupported interaction action '{other}'"`. The prose heuristic matches
  `contains("unsupported")` → `unsupported` / exit **69**, telling an agent the
  build lacks a feature. It is a caller typo. It is now
  `CliFailure::invalid_arguments` → `invalid_arguments` / exit **2**. So `N4`
  was not only fragile in principle; it was mis-classifying a live command.
- `proof` (`x01_typed_classification_tests` in `cli_error.rs`):
  - `typed_failures_ignore_message_wording` pairs six kinds with messages whose
    prose the legacy heuristic classifies *differently*, and asserts the
    heuristic really would disagree **before** asserting the type wins — so the
    test cannot pass vacuously. Example: `CliErrorKind::Io` carrying the text
    `"unknown schema"` must still be `io_error` / exit 74.
  - `argument_parsing_failures_are_always_usage_errors` runs five messages
    through `CliUsageError` (`"no such file"`, `"unsupported action"`,
    `"gpu adapter unavailable"`, `"failed to write output"`, `"interrupted"`) —
    each of which the heuristic would send to a different exit code — and
    asserts all stay exit 2.
- `red proof`: forcing `from_failure` to always fall through to `classify`
  produced `assertion left == right failed: typed InvalidArguments must set the
  code / left: "input_not_found" / right: "invalid_arguments"` and
  `argument-layer message "no such file" must stay exit 2 / left: 65 /
  right: 2`. Reverted; both pass.
- `schema stable`: `scena.cli_error.v1` gains no field and loses none.
  `a01_cli_error_taxonomy` (3 tests, including
  `cli_errors_expose_stable_typed_exit_taxonomy` and
  `every_declared_command_has_error_schema_and_exit_class_inventory`) passes
  unchanged, as do `a13_error_remedies` and `cli_output_contracts`.
- `scope recorded honestly`: producers that still return a bare `String` land in
  `Unclassified` and keep the prose fallback. `CliErrorKind`'s own rustdoc says
  so. This row types the two families the audit named — the argument layer and
  the classification site — and the IO/decode/internal producers reached along
  the way; it does not claim every string in the binary is typed.
- `gates`: `cargo clippy --workspace --all-targets -- -D warnings` clean;
  `cargo clippy --bin scena --all-features -- -D warnings` clean;
  `cargo build --bin scena` emits **0** warnings.

### X02 — Markdown output under `--compact` fails

Owner: `src/bin/scena/output.rs`.

Closes `N22`. Reproduced on this tree: `guide agent --markdown --compact`
exits **70 / `internal_error`** with *"JSON output formatting requires a JSON
command result: expected value at line 1 column 1"*. Cause: generic JSON
formatting applied to Markdown at `output.rs:208`.

- [x] Add a focused red test for the exact invocation.
- [x] Either honor `--compact` as a no-op for Markdown, or reject the
      combination as a `usage` error at exit 2 — never `internal_error`.

Resolution: **reject as a usage error at exit 2.** A silent no-op would let an
agent believe it received a shaped response; a usage error names the fix.

Validation ledger:

- `test`: `markdown_output_is_not_json_shaped_and_rejects_json_only_flags_as_usage`
  in `tests/a12_json_formatting.rs`. It covers plain `--markdown` plus five
  shaping combinations, including the global-position form
  `--compact guide agent --markdown`.
- `red proof`: before the fix the test failed with the exact `N22` envelope —
  `"exit_class": "internal"`, `"exit_code": 70`, `"code": "internal_error"`,
  `"message": "JSON output formatting requires a JSON command result: expected
  value at line 1 column 1"`, and `"help": "preserve this JSON report and file a
  scena issue with the CLI version"`.
- `scope correction — this batch had made it worse`: `G08`'s unconditional
  policy strip removed the early return, so *every* invocation was JSON-parsed.
  Plain `guide agent --markdown` — which worked in 1.9.0 — started exiting 70.
  `T05` caught it; see that ledger.
- `fix`: `CliOutcome` now carries a `CliPayload` (`Json` | `Markdown`) set at
  construction, and `apply_output_format` returns a typed
  `CliOutputFormatError`. A Markdown payload with no shaping flags returns
  early untouched (the implicit policy strip no longer reaches it); a Markdown
  payload with any shaping flag returns
  `JsonShapingOnNonJsonPayload`, which `src/bin/scena.rs` maps to
  `CliError::invalid_arguments` — exit 2, `invalid_arguments`, `usage`.
- `classification is typed, not prose`: `output_format_error` matches on the
  error enum. Rewording either message cannot move an invocation between exit
  classes. This is the `X01` discipline applied to the formatting path; the
  general `cli_error.rs` classifier is still `X01`'s own row.
- `schema stability`: `scena.cli_error.v1` is unchanged — same fields, only the
  `code`/`exit_class`/`exit_code` values differ for this invocation, which is
  the point.
- `green`: `cargo test --test a12_json_formatting` → 4 passed;
  `--test a05_public_agent_guide` → 1 passed;
  `cargo clippy --workspace --all-targets -- -D warnings` → clean.

### X03 — `internal` and `runtime` share exit 70

Closes `N21`. Confirmed but documented policy, and JSON consumers can
distinguish them; `$?`-only consumers cannot.

- [x] Decide: keep (and document the limitation prominently) or split. Record
      the decision. Not necessarily a defect.

Decision: **keep the shared exit 70, document the limitation prominently.**

Rationale:

- `internal` and `runtime` differ in *who can act*, not in what the shell can
  do: neither is retryable by changing arguments, and both mean "the command
  did not produce its result". A caller branching on `$?` alone would take the
  same action for both.
- Splitting would change the exit code of one of the two classes. Exit codes
  are part of `scena.cli_error.v1`'s observable contract and are pinned by
  `a01_cli_error_taxonomy::cli_errors_expose_stable_typed_exit_taxonomy`.
  Breaking them is a 2.0 change, not a 1.9.1 patch.
- The distinction *is* machine-readable today: `code`
  (`internal_error` vs `runtime_error`) and `exit_class`
  (`"internal"` vs `"runtime"`) are separate fields in every emitted error.
  `X01` makes those two fields authoritative rather than prose-derived, which
  strengthens exactly the channel a caller should be reading.
- Guidance stands: an agent should read the JSON error envelope, not `$?`.
  `internal` means file a bug; `runtime` means the operation itself failed.

- [x] Documented in `docs/errors.md` under the exit-class table.

---

## 5. Evidence integrity

### E01 — Q07 mutation results are hardcoded

Owner: `tests/q07_antialiasing_effect.rs`.

Closes `N13`. `rejected: true` is a literal at `q07_antialiasing_effect.rs:185`.
The in-test oracle mitigates it, but the artifact is not bound to computed
mutation results.

- [x] Bind recorded mutation results to actual computed outcomes.
- [x] Prove a mutation that should pass flips the artifact value.

Validation ledger:

- `defect`: `q07_antialiasing_effect.rs:185` wrote
  `{"name":"no_op","rejected":true}` and
  `{"name":"blur_everything","rejected":true}` as JSON literals. Those values
  stayed `true` however the oracle behaved.
- `fix`: a new `evaluate_known_bad_mutations` runs each mutation against **this
  run's live baseline frame** and returns the computed verdict. The artifact
  serializes that verdict, and the producer asserts before writing — a mutation
  the live oracle accepts aborts the run rather than publishing
  `rejected: false`.
- `second bullet, answered honestly`: "prove a mutation that should pass flips
  the value" cannot be shown with the two known-bad mutations, because both
  *must* be rejected — a passing one would be a product defect. What the row is
  really asking for is that the recorded values are not constants. That is
  established structurally (they are computed) and observably by a new
  `positive_control` field: the supersampled diagonal, evaluated through the
  same code path, is recorded as `accepted: true`. An oracle degraded to
  "reject everything" therefore shows up in the artifact as
  `positive_control.accepted: false` instead of hiding behind two `true`s.
- `green`: `cargo test --test q07_antialiasing_effect` → 4 passed, including
  `q07_effect_oracle_rejects_noop_and_blur_everything_mutations`.

### E02 — Windows validator trusts JSON metrics

Owner: `tests/release/windows_complete_hardware_proof_validation.js`.

Closes `N14`. It recomputes predicates from recorded metrics while only
checking PPM existence and magic bytes (`:555`).

- [x] Bind Windows Q07 metrics to actual image bytes, not to recorded metrics.

Validation ledger:

- `defect`: `validateQ07Antialiasing` recomputed every predicate from the
  `metrics` object the producer wrote into JSON. The PPM frames were opened only
  to check they began with `P6` (`:735`). A producer emitting plausible metrics
  next to wrong pixels passed the independent validator.
- `fix`: added `readPpmRgb` (binary P6 parser, comment- and whitespace-tolerant)
  and `measureEdgesFromPpm`, a direct port of `measure_edges` from
  `tests/q07_antialiasing_effect.rs`. `assertMetricsMatchFrame` recomputes all
  four metrics from the frame the report itself names and requires exact
  equality, for the baseline, `fxaa`, `msaa4`, and `msaa8` when it passed.
- `red proof`: a synthetic proof root with a hard-diagonal baseline and an
  antialiased candidate. Recorded metrics forged by **+1** on one field — still
  satisfying the AA effect predicate — were rejected:
  `Q07 fxaa recorded intermediate_luma_pixels=95 but
  q07-antialiasing-effect/fxaa.ppm measures 94; Q07 metrics must be derived from
  the image bytes, not asserted alongside them`.
- `green proof`: the same report with metrics measured from those exact bytes
  was accepted. Both directions demonstrated, so the check is discriminating
  rather than merely strict.
- `why a port and not a shared implementation`: the validator is deliberately
  independent of the Rust producer — a shared implementation would let one bug
  satisfy both sides. Divergence between the two makes the recomputed metrics
  disagree, which fails; that is the intended coupling.

### E03 — Q08 writes a pass artifact before the hardware assertion

Owner: `tests/support/parity.rs`.

Closes `N12`. The artifact is written before the strict assertion (`:165`). The
job still fails, so this is contradictory evidence rather than a demonstrated
publish bypass.

- [x] Write failed-lane Q08 artifacts as **failed**.
- [x] Prove a failing lane cannot leave a passing artifact on disk.

Validation ledger:

- `defect`: `record_cpu_gpu_parity_pass` called `write_parity_gate_result(...,
  "passed", ...)` and only *then* asserted `!strict || hardware`. Under
  `SCENA_REQUIRE_GPU_PARITY=1` on a software adapter, a `"status": "passed"`
  artifact reached disk before the panic. As `N12` correctly notes, the job
  still failed — so this was contradictory evidence rather than a demonstrated
  publish bypass. It is fixed because an artifact that disagrees with its own
  job is a trap for anything reading the directory.
- `fix`: introduced `ParityOutcome` (`ReleaseEvidence` /
  `DiagnosticConformance` / `StrictHardwareMissing`) and the pure
  `parity_outcome(strict, hardware)`. The decision now happens **before** any
  write. `StrictHardwareMissing` writes `"status": "failed"`,
  `release_evidence: false`, `proof_class: "strict-hardware-missing"`, and then
  panics — so the artifact names why it failed instead of reusing a diagnostic
  class.
- `structural guarantee`: no caller can pass a status that disagrees with the
  outcome, because the status and proof class both come from the enum.
- `proof without a GPU`: `parity_outcome_tests` in `tests/support/parity.rs` —
  `strict_mode_without_hardware_never_yields_a_passing_outcome` and
  `only_strict_mode_on_hardware_is_release_evidence`. They run in every parity
  test binary that includes the module, which is the right blast radius: those
  are the binaries that write these artifacts.
- `green`: `cargo test --test c13_depth_clipping_parity` → 3 passed.

### E04 — Q08 source selection depends on path length

Owner: `crates/xtask/src/app/release/stage_artifacts.rs`.

Closes `N19`. Generic Q08 suffixes receive no explicit lane rank (`:255`).

- [x] Give each lane an explicit rank; remove path-length dependence.

Validation ledger:

- `defect`: `stage_source_rank` returned `(usize::MAX - preferred, text.len(),
  text)`. A suffix matching none of the explicit lane branches got
  `preferred = 0`, so selection fell through to **shortest path wins**. Q07,
  Q08, and C09 artifacts carry no lane marker in their path yet are produced
  only by the macOS Metal lane (`lane_artifacts.rs:310-325`) — exactly the
  generic case.
- `fix`: extracted `stage_source_owner_lane(suffix)`, which names the owning
  lane for every staged suffix including the three generic families, and
  removed `text.len()` from the sort key. The remaining path comparison is a
  deterministic lexicographic tiebreak between candidates that already rank
  equally on lane ownership.
- `red proof`:
  `stage_source_selection_is_independent_of_competing_path_length` failed with
  `a competitor path of length 2 must not change the selection / left:
  Some("a/q08-required-parity/core-pbr-brdf-...json") / right:
  Some("macos-metal-gate-artifacts/q08-required-parity/core-pbr-brdf-...json")`
  — a two-character directory beat the owning lane.
- `green`: both new tests pass —
  `generic_suffixes_select_their_owning_lane_not_the_shortest_path` (covering
  two Q08 suffixes, Q07, and C09) and the length-independence test. The
  pre-existing `q01_stage_source_prefers_the_finalized_headless_cpu_producer`
  still passes, so the Linux-owned selection is unchanged.

### E05 — Three environment flags escape the doctor registry

Owner: `crates/xtask/src/app/doctor_core/runner/env_contract.rs`.

Closes `N20`. They are absent from the registry (`:20`). Some appear in prose
elsewhere, but not in the enforced table.

- [x] Add the newly discovered flags to the enforced registry.
- [x] Prove doctor fails when a flag is read but unregistered.

- `red proof`: removing `SCENA_REQUIRE_CI_PROVENANCE` from the registry produced
  `TESTS-ENV-FLAGS-DOCUMENTED: crates/xtask/src/app/release/readiness.rs reads
  env var 'SCENA_REQUIRE_CI_PROVENANCE' that is absent from the shared
  test/script env registry` **and** the same finding for
  `release/stage_artifacts.rs`. Both are release-tooling files that the
  pre-`E05` scan could not see at all. Reverted; the rule reports zero findings.
- `no false positives from the widening`: `cargo test -p xtask` reports 394
  passed with the only failures being the 6 pre-existing
  `HONEST-MATERIAL-PRESETS` findings. The two documented exclusions
  (`tests_NN.rs` fixture files and whole-line `//` comments) hold.

Validation ledger:

- `root cause was broader than the finding`: the rule only scanned `tests/` and
  `scripts/`
  (`env_contract.rs`: `collect_test_contract_sources(&root.join("tests"))` and
  `.join("scripts")`). Every flag read from product code, examples, or the
  release tooling was structurally invisible to it — so the registry could not
  have caught them however many were added.
- `seven flags found, not three`:
  - `SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU` — **product code**
    (`src/render/gpu/build.rs:21`); overrides the refusal of a V3D adapter known
    to hang.
  - `SCENA_GPU_EVIDENCE_CLASS` — declares whether a GPU parity artifact may
    claim `hardware-release` or only `software-conformance`.
  - `SCENA_REQUIRE_CI_PROVENANCE` — requires CI-issued provenance instead of
    self-reported commit metadata.
  - `SCENA_RELEASE_ARTIFACT_ROOT` — where `release-readiness` reads staged
    artifacts from.
  - `SCENA_DOCTOR_REQUIRE_GENERATED_ARTIFACTS` — makes missing generated WASM
    blocking.
  - `SCENA_GLTF_VALIDATOR` — path to the official Khronos validator.
  - `SCENA_EASY_SCENE_SHOWCASE_ONLY` — showcase example subset selector.
  The first four all gate release evidence. That is why the narrow scan
  mattered: an undocumented flag that decides whether an artifact counts as
  hardware evidence is exactly the kind `N20` exists to surface.
- `scan widened` to `src/`, `examples/`, and `crates/xtask/src/`, with two
  documented exclusions so the widening does not produce false findings:
  xtask's own `tests_NN.rs` files, which embed synthetic sources such as
  `env::var("MY_OTHER_FLAG")` as fixtures for `find_env_var_names`; and
  whole-line `//` comments, because `runner.rs`'s doc comment describes the
  scanner's own call shape as `env::var("FOO")`.
- `GITHUB_ACTIONS` added to `STANDARD_EXEMPTIONS`, alongside the `GITHUB_SHA`,
  `GITHUB_RUN_ID`, and `GITHUB_REPOSITORY` entries already there.
- `documented in the same change`: all seven have `CLAUDE.md` rows stating what
  they control and the unset default, which is what `D14` requires.

---

## 6. Documentation, release metadata, deployment

### D01 — v1.9.0 changelog and release body

Closes `N3`. Verified: shipped v1.9.0 work sits under `## [Unreleased]`
(`CHANGELOG.md:5`); the `1.9.0` section is still dated July 21; the release
body still reads "Release candidate prepared."

- [x] Move the shipped entries under a dated `1.9.0` heading.
- [ ] Correct the `v1.9.0` GitHub release body **in place**. **Blocked** —
      needs authenticated `gh` access to the GitHub release object; it is an
      outward-facing edit and is left for the release step.
- [x] Open a fresh `[Unreleased]` for 1.9.1.

Validation ledger:

- `moved`: the 19 entry blocks that sat under `## [Unreleased]` are all shipped
  v1.9.0 work — the review baseline `931ac41` has a tree identical to tag
  `v1.9.0`, so every one of them is inside the tag.
- `date corrected`: the section read `## [1.9.0] - 2026-07-21`, but
  `git log -1 --format=%ci v1.9.0` gives `2026-07-24 16:06:49 +0200`. The
  heading is now `## [1.9.0] - 2026-07-24`.
- `fresh section`: `## [Unreleased]` now holds `### 1.9.1 (in progress)` and
  points at this checklist.
- `doctor safe`: `CHANGELOG.md` is listed in
  `HISTORICAL_VERSION_PATH_PREFIXES` (`doctor_docs/version_alignment.rs:6`), so
  `D01-PUBLIC-VERSION-ALIGNMENT` does not treat historical version strings in
  it as drift.

### D02 — Deprecations claim a future version

Closes independent finding 3. Verified: `src/animation.rs:188` and `:242` both
declare `since = "1.9.1"` in released 1.9.0 source.

- [x] Correct the metadata as part of the 1.9.1 release, when the claim
      becomes true.

Resolution: **no source edit is warranted; the claim is made true by the
version bump.**

- `since = "1.9.1"` was false in the 1.9.0 tag, and that cannot be retracted —
  the tag is immutable and must not be recut (§0.1).
- The correct value for these attributes *is* `1.9.1`: that is the release in
  which the deprecations take effect. Changing them to anything else would make
  them wrong again.
- The claim becomes true the moment `Cargo.toml` reads `version = "1.9.1"`.
  That bump is a section 9 step, not a mid-batch one, because it invalidates
  roughly seven pinned version sites and every fixture that embeds the version;
  doing it now would force a re-run of the whole batch.
- `carried into section 9`: the version bump plus its pin sweep is listed there
  as a blocking release step. If 1.9.1 is not cut, these attributes stay
  wrong — that dependency is recorded, not hidden.

### D03 — Stale `CLAUDE.md`

Closes `N8`. `CLAUDE.md:79` still describes the old Metal oracle and tolerance
35, and the Metal lane runs six commands, not seven.

- [x] Update to the current oracle, tolerances, and command count.

Validation ledger — three claims were stale, and one of them was inverted:

- `tolerance`: the text said region checks use Chebyshev 25 "but the measured
  Apple Paravirtual Metal body sample permits up to 35". Every
  `WaterBottleRegionExpectation` in `tests/m8_real_asset_proof.rs` has
  `tolerance: 25` — `grep 'tolerance:' | sort -u` returns exactly one value.
  What varies per profile is the *expected* sample, not the tolerance
  (`cap_dome` is `[76, 28, 12]` on Apple Paravirtual versus `[76, 27, 12]`
  portable). Documenting a looser tolerance than the code enforces invites
  someone to "restore" it.
- `reference comparison`: the text said the lane does **not** run the opt-in GPU
  reference comparison. `.github/workflows/ci.yml:255` sets both
  `SCENA_REFERENCE_DIFF=1` and
  `SCENA_RUN_UNSTABLE_HEADLESS_GPU_RELEASE_TESTS=1`. The claim was backwards.
- `command count`: neither my "seven" nor the adjudication's "six" was right.
  The lane runs **12** `release_lane_command.sh` invocations
  (`ci.yml:252-263`). The corrected text enumerates what they cover instead of
  restating a number that drifts.
- `also added`: profiles are selected by a structured adapter key, never a
  free-form adapter name, and each carries owner, review date, expiry, and
  evidence hash.

### D04 — Checklist and RFC pointer are stale

Closes `N24`. 10.4/public-release boxes remain open in the v1.9.0 checklist,
and the RFC still points to that nearly-complete checklist as its one active
backlog.

- [x] Close or explicitly defer the remaining v1.9.0 boxes.
- [x] Repoint the RFC at this document, and then at the section 8 forward
      backlog.

Validation ledger:

- `v1.9.0 checklist`: a dated disposition block now precedes section 10.4 and
  gives each remaining group an explicit status — the maintained-runner and Q03
  provenance rows are `deferred, superseded` and carry forward to this
  document's section 9; 10.4 is `done for v1.9.0` except the release body text
  (`D01`); 10.5's final clause is recorded as **not satisfied**, which is
  precisely why this batch exists.
- `boxes deliberately left unticked`: the disposition says what happened
  instead. Ticking them would have claimed work that was not done — the
  opposite of what `N24` asks for.
- `RFC`: `docs/RFC-rust-3d-renderer.md` now names
  `full-repo-review-v1.9.1-remediation.md` as the one active implementation
  backlog and points at its section 8 (forward features) and section 7
  (performance). The v1.9.0 checklist moved into "Historical evidence
  tracks".

### D05 — Public demo is two minor versions behind

Closes `N9` and independent finding 4. A live fetch from
`scena-demo.pages.dev` returned `1.7.1-public-a76468dfd9ba`; repository demo
files say 1.9.0. Deployment **and** repository are both out of alignment.

- [ ] Redeploy the demo from the released renderer. **Blocked, and sequenced
      after the release** — see below.
- [x] Verify the served cache marker against the repository marker.
- [x] Load the page in a real browser, check `pageerror` and console errors, and
      capture a screenshot before claiming it shipped.

Validation ledger:

- `deployed state confirmed by live fetch`: `https://scena-demo.pages.dev/`
  serves cache marker `public-a76468dfd9ba`, version string
  `1.7.1-public-a76468dfd9ba`. The repository's `demo/` carries
  `1.9.0-public-0215db283505`. The two disagree, exactly as the finding says.
- `the deployment is not produced by any workflow in this repository`. The
  release workflow (`release.yml:109-113`) starts
  `python3 -m http.server 18104 --directory demo` and probes **that local
  server** with `npm run cloudflare:demo` and `npm run cloudflare:materials`.
  Nothing in `.github/workflows/` or `scripts/` pushes to Cloudflare. So the
  drift is not a broken automated step — there is no automated step. Redeploy
  requires Cloudflare credentials, is outward-facing, and is not authorized by
  this batch.
- `sequencing`: "redeploy from the released renderer" cannot be satisfied before
  1.9.1 is cut, since the released renderer does not yet exist. It belongs to
  the release chain, not to this row's remediation.
- `browser proof of the artifact that would be deployed`: served the repository
  `demo/` on `127.0.0.1:18777` and loaded it in real Chromium via Playwright:
  - HTTP 200, `pageerror` **0**, console errors **0**, failed requests **0**.
  - Canvas 992x744, with 736,609 of 738,048 sampled pixels non-background — the
    scene actually renders rather than leaving a blank canvas.
  - Screenshot captured and inspected at native resolution: the glTF machine
    model renders with PBR materials, studio lighting, ground grid, and the
    "rendered" badge. Not a placeholder and not a broken canvas.
- `what this does and does not prove`: the 1.9.0 demo bundle in this repository
  is functional and deployable. It does **not** prove anything about the live
  1.7.1 deployment, which remains two minor versions behind until someone with
  Cloudflare access redeploys.
- `knock-on effect recorded elsewhere`: this stale deployment is why
  `doctor --full` cannot pass on any host without a live probe against it.
  `target/gate-artifacts/round-e-cloudflare-material-proof.json` is produced by
  `scripts/probe_cloudflare_material_presets.mjs` driving Chromium against
  `https://scena-demo.pages.dev/proof/?sample=material-presets`. The 6
  `HONEST-MATERIAL-PRESETS` findings that block three gates on this host all
  trace to it.

### D06 — Animation migration note

Closes `N26`. Current docs describe strict timestamps, but the v1.9.0 release
notes do not call out the new load-time rejection behavior.

- [x] Add an explicit migration note for the load-time rejection change.

Validation ledger:

- `where`: `docs/release-notes/v1.9.0.md`, as a named
  "Migration: imported animation is now rejected at load time" subsection
  under "Evidence and migration". The v1.9.0 notes are where a user upgrading
  from 1.8.0 looks; burying it in 1.9.1 would miss them.
- `what it states`: the behavior change can turn a previously-loading asset into
  a load failure; the exact failure conditions (empty, non-finite, duplicate or
  non-monotonic timestamps, keyframe count not matching the interpolation
  mode); the preserved glTF static-case exception (one key at time zero); a
  three-step recovery path including `scena doctor <asset>`; and that there is
  deliberately no opt-out, because the previous behavior had no defined result.
- `linked`: also states the authored-side rule —
  `AnimationSourceClip::try_rebind` returns the error, the deprecated `rebind`
  panics on the same input (`R05`) and is removed in the next major.

### D07 — Maintenance

Closes `N25`. Local `target/` is 136 GB. Four Dependabot PRs remain open, two
green and two failing.

- [x] Reclaim local build cache.
- [x] Triage all four Dependabot PRs.
- [ ] Land the two green Dependabot PRs. **Needs authorization** — merging into
      `main` is an outward-facing action outside this branch's scope, and both
      touch `.github/workflows/*.yml`, which this branch also modifies (`T01`).
      Merge order matters; see below.

Validation ledger:

- `cache`: `target/` measured **171 GB**, not the 136 GB the finding reported.
  Removing `target/debug/incremental` reclaimed **50 GB**. It is now 152 GB;
  the remainder is live build output for the default, `--all-features`, and
  `wasm32` profiles and is regenerated on demand.
- `what filled it`: the `--all-features` build required by `T01`'s new lane
  compiles `basisu_c_sys`, a native C encoder. That is a real cost of the lane
  and is recorded in `T01`.
- `PR #10 actions/checkout 4.3.1 → 7.0.1` — **green**, all 8 checks pass.
- `PR #8 actions/upload-artifact 4.6.2 → 7.0.1` — **green**, all 8 checks pass.
- `PR #9 actions/download-artifact 4.3.0 → 8.0.1` — **red, and it is a pin
  problem, not a breakage.** `xtask` fails with
  `RELEASE-CI-M9: .github/workflows/ci.yml is missing required contract text
  'actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4.3.0'`
  in both `tests_08::m9_release_metadata_contracts_are_source_enforced` and
  `tests_08::release_readiness_has_no_open_release_deferrals`. The rule pins the
  exact SHA **and** the `# v4.3.0` comment at
  `doctor_visual_release/ci_release_lanes.rs:77` and `:134`. Dependabot moves
  the workflow; the pin does not follow. To land it, update both pinned strings
  to the new SHA and version comment **in the same commit** — this is `T06`'s
  lockstep rule applied to CI action pins. #10 and #8 are green precisely
  because no doctor rule pins those two actions.
- `PR #7 actions/setup-node 4.4.0 → 7.0.0` — **red for an unrelated reason:
  a stale base.** It fails
  `tests_11::stage_release_artifacts_generates_canonical_release_evidence` with
  `local release provenance must fail before artifact validation:
  RELEASE-SOURCE-EVIDENCE: release artifact release-lanes/linux-native-vulkan.json
  is missing required non-blank schema provenance`. Its run is from 2026-07-19,
  before the v1.9.0 provenance work landed; the same test passes on the current
  base in #10 and #8. It needs a rebase, not a fix.
- `recommended order`: land #10 and #8 first (no pin coupling); then #7 after a
  rebase; then #9 together with the two `ci_release_lanes.rs` pin updates. All
  of it should happen after this branch's `ci.yml` change merges, to avoid a
  three-way conflict in the workflow file.

### D08 — Changelog entries for every 1.9.1 fix

- [x] Every closed row in sections 1-5 that changes user-visible behavior gets
      a `[1.9.1]` changelog entry written from the user's point of view, not
      the row ID.
- [x] Rows that are purely internal (test coverage, evidence binding) are
      listed once as a grouped entry rather than omitted silently.

Validation ledger:

- `user-visible entries`: 13 entries under `### 1.9.1 (in progress)`, each
  written from the caller's point of view. No row ID appears in the entry text.
- `internal entries`: one `#### Internal — no user-visible behavior change`
  group covering `T01`/`T02` (feature-gated lane and bijection rule),
  `T04` (Blender anchor), `T03` (multi-list re-entry), `E05` (env registry), and
  `D13` (narrowed claim). Listed rather than dropped, so a reader can see the
  coverage work happened.
- `still open`: `E01`-`E04` and `T06`/`T08` have no entry yet because they are
  not closed. Entries are written when the row closes, not in advance.

### D09 — Migration notes for user-visible rendering changes

Three rows change what existing users' renders look like. None of them is a
bug-for-bug-compatible fix, so each needs an explicit note:

- `R01` — auto-exposure now converges instead of oscillating; a scene that
  happened to be captured on a favourable frame will change.
- `R03` — mips filter in linear space; textured surfaces change appearance,
  generally darker-correct at distance.
- `G01` — annotations no longer clip against a section box by default; renders
  that relied on the old behavior must set the new opt-in.

- [x] Write a migration note for each, naming the opt-out or opt-in where one
      exists.
- [x] Fold these into the 1.9.1 release notes alongside `D06`.

Validation ledger:

- `where`: `CHANGELOG.md`, `### 1.9.1 (in progress)` →
  `#### Migration — three fixes change what existing renders look like`. Each
  note states what moves, what to do, and whether an opt-out exists.
- `R01`: opt-out **none** — the previous loop had no stable fixed point, so
  there is nothing coherent to opt back into. Headless captures with a fixed
  `set_exposure_ev` are called out as unaffected.
- `R03`: opt-out **none**, but the blast radius is narrowed explicitly: data
  textures (normal, roughness, metallic, occlusion, every non-sRGB format) are
  **byte-identical**, and alpha is never transformed. This is asserted by the
  `R03` test, not merely claimed here.
- `G01`: opt-in **`clip_with_scene`, per-overlay**. The note states there is
  deliberately no global switch, so one dimension line can clip while its
  neighbours do not.
- `also updated`: `docs/guides/llm-app-builder.md` (`D12`) carries the same
  section-box semantics where an agent actually reads them.

### D10 — Document the new CLI surface

- [x] Document `G08`'s `--fields`, `--include policy`, and
      `guide agent --contract`, and `G06`'s `--detail`, in `scena --help` and
      `README.md`.
- [x] Verify the documented invocations actually run, rather than only
      appearing in prose.

Validation ledger:

- `scena --help`: all four already appear — `--fields`, `--include`, and
  `--detail` were added to the declared surface by `G06`/`G08`, and
  `guide agent --contract` by `G08`. `a10_cli_contract_table` digests that
  table, so it cannot drift silently.
- `README.md`: `guide agent --contract` documented next to `--json`/`--markdown`
  with the reason to prefer it (the `--json` form is over 90% Markdown by
  bytes); `--fields`, `--include policy`, and `--detail` documented in the
  global JSON-formatting section with runnable examples.
- `docs/api.md` was **not** changed: it documents the Rust API surface, and none
  of these are Rust API. Putting CLI flags there would be filing them where a
  reader will not look. The row's requirement is met by `--help` plus
  `README.md`.
- `invocations executed, not just written`:
  - `scena guide agent --contract` → exit 0, keys
    `commands, name, policies, schema, schemas, templates, version`, and
    `markdown` confirmed **absent**.
  - `scena guide agent --markdown` → exit 0, first line
    `# LLM App Builder Guide`.
  - `scena --help --fields schema,commands` → exit 0, keys reduced to
    `commands, schema`.
  - `scena --help --include policy` → exit 0.
  - `scena --help --include bogus` → `invalid_arguments` / `usage`, proving the
    flag validates its value rather than ignoring it.
  - `scena --help` contains
    `recipe render <recipe.json> [--verify] --out <png> [--introspect] [--detail] ...`.

### D11 — Document the new reason-code vocabulary

- [x] Publish the reason-code vocabulary in `docs/schema-contracts.md`.
- [x] State which codes are advisory and which are load-bearing for a
      declared template expectation.

**Correction — this row named codes that do not exist.** `clipped_by_plane`,
`outside_viewport`, `occluded`, and `clipped_by_section_box` are not emitted by
`src/render/visibility_diagnosis/`. Documenting them would have published a
vocabulary an agent could never match. The real vocabulary is 20 codes:

- `error` (load-bearing — sets `ok: false` and fails a declared expectation):
  `node_hidden`, `parent_hidden`, `layer_masked`, `stale_handle`,
  `nan_transform`, `zero_scale`, `missing_geometry`,
  `missing_material_upload`, `alpha_zero`, `behind_camera`, `outside_frustum`,
  `not_prepared`, `missing_camera`, `no_visible_drawables`, `all_culled`,
  `import_has_no_roots`, `import_roots_stale`.
- `warning` (advisory — reported, never sets `ok: false` alone):
  `clipped_by_active_clipping_plane`, `transparent_material`,
  `backend_capability_degraded`.

Validation ledger:

- `source of truth`: every code and severity above was read from
  `src/render/visibility_diagnosis/analysis.rs` and `analysis/node.rs`, not
  from the finding text.
- `published`: `docs/schema-contracts.md`, under
  `#### nodes_detail[].reason_codes vocabulary`, as a table with severity and
  meaning, plus the explicit load-bearing/advisory rule.
- `also documented`: codes attach only to node kinds that can rasterize
  (`Mesh`, `Label`, `Renderable`, `Model`, `InstanceSet`, `ParticleSet`) —
  a reason on a camera or light would be noise; and there is deliberately **no**
  `clipped_by_section_box`, with the reason.
- `visible vs drawn`: the same section now states that `visible` is never
  comparable to `drawn` and that `visible_drawable` is the right comparand
  (`G05`).

### D12 — Update the section-box guide

`docs/guides/llm-app-builder.md:484` documents `section_box_active` and the
clipping-mismatch diagnostics that `G01` changes.

- [x] Update it to the new clipping semantics, including the annotation
      default and the explicit opt-in.

Validation ledger:

- `where`: `docs/guides/llm-app-builder.md`, immediately after the
  `expect_clipping` / `section_box_active` passage the finding named, so an
  agent reading about section-box expectations sees the semantics in the same
  place.
- `states`: a section box and clipping plane section model geometry only;
  annotations stay visible since 1.9.1; `clip_with_scene` is the per-overlay
  opt-in; there is no global switch back; and the pre-1.9.1 behavior is named
  so someone with a stored render knows what changed.
- `cross-referenced`: points at the `nodes_detail[].reason_codes` vocabulary and
  states that plane-clipped nodes report `clipped_by_active_clipping_plane`
  (advisory), and that no `clipped_by_section_box` code exists.

### D13 — Narrow the "fail-closed end-to-end" claim

Closes independent finding 5. The configured publish workflow did fail closed
and passed at the exact tag commit — but orphaned test binaries, contradictory
failed-lane artifacts, hardcoded mutation booleans, and metrics unbound to PPM
bytes mean the evidence ecosystem as a whole is not yet universally
fail-closed.

- [x] Narrow the wording wherever the broader claim is made, so it describes
      the publish workflow rather than the whole evidence ecosystem.
- [x] Re-widen it only once `T01`, `T02`, and `E01`-`E04` are all closed.
      Recorded as a gated condition with a live status table, not a promise.

Validation ledger:

- `where`: `docs/specs/release-gates.md`, a new
  `### Scope of the fail-closed claim` subsection directly under the required
  release commands — the place a reader forms the impression the finding
  objects to.
- `narrow claim stated`: the publish workflow refuses to publish without
  CI-issued, attestation-verified provenance for the exact source commit, and
  was verified passing at the v1.9.0 tag commit. That is what the evidence
  supports and all that is claimed.
- `gap table`: `T01`/`T02` marked closed (with the 29 orphaned binaries the
  bijection rule found); `E01`-`E04` marked open. Re-widening is gated on that
  table reading closed throughout, so the condition is checkable rather than
  aspirational.
- `explicit instruction`: a document, release note, or PR description saying the
  evidence is "fail-closed end to end" is overstating what is enforced until
  those rows close.

### D14 — Environment-flag table parity

- [x] Add `E05`'s newly registered flags to the `CLAUDE.md` environment
      table so the prose table and the enforced doctor registry agree.
- [x] Confirm no other flag is in one and not the other.

Validation ledger:

- `count corrected`: the row said three flags. A sweep of every
  `env::var`/`env::var_os`/`process.env` read across `src/`, `examples/`,
  `tests/`, `scripts/`, and `crates/` found **seven** unregistered product and
  tooling flags. See the `E05` ledger for the list and why the original count
  was low.
- `both directions checked`: no registered flag lacks a `CLAUDE.md` row, and no
  read flag is missing from the registry. The one registry entry with no
  current reader is `SCENA_WEBGL2_BROWSER`, which is a declared test-only
  selector documented in the table and kept deliberately.
- `enforced, not just written`: the `TESTS-ENV-FLAGS-DOCUMENTED` doctor rule
  checks both directions, and `E05` widened its scan so a flag added to product
  code can no longer escape it.

---

## 7. Performance

Ordered after correctness. Every row needs a benchmark distribution and
rendered-output parity on a controlled host. GitHub-hosted timing stays
report-only.

### P01 — Dirty-subtree transform invalidation

Owner: `src/scene/resolved_cache.rs`.

Confirmed algorithmically: `matches()` compares whole-scene revisions, so any
`set_transform` triggers `cache.nodes.clear()` and a full root-down traversal
(`resolved_cache.rs:50`). Cost of moving **one** node scales with **total**
node count. The cache-hit path is already constant-time and allocation-free.

The second review's debug timings were not independently reproduced; the
`O(total nodes)` scaling from one-node movement is certain regardless.

- [ ] Add a benchmark measuring one-node movement across scene sizes.
- [ ] Track dirty subtree roots; re-traverse only those subtrees.
- [ ] Prove world transforms are identical to the full-rebuild result.
- [ ] Prove the cache-hit path stays constant-time.

This is the best internal performance project after correctness.

### P02 — Quaternion normalization in the compose hot path

Owner: `src/scene/math.rs`.

Corrected citation (see 0.5). `Transform::compose` (`math.rs:102`) normalizes
**twice** per node: `rotate_vec3_by_quat` (`math.rs:197`) and
`compose_rotations` (`math.rs:188`). `compose_rotations` carries a documented
rationale — bounding drift across many composed rotations.

"Normalize only once at the end of the chain" is **unsafe** here: intermediate
world transforms are observable through the resolved cache.

- [ ] Benchmark a fast path that trusts validated unit inputs and renormalizes
      only past an error threshold.
- [ ] Prove drift stays bounded across a deep chain versus today's behavior.
- [ ] Do not land without the drift proof.

### P03 — Sliding-window bloom

Owner: `src/render/output.rs`.

Confirmed: separable but re-sums every window (`output.rs:413`). Sliding sums
make both passes `O(pixels)`.

- [ ] Convert both passes to running sums.
- [ ] Prove output is byte-identical (or justify any difference).
- [ ] Confirm the existing gate
      `blur_sample_reads <= pixels * kernel_width * 2` (`output.rs:588-591`)
      still passes — it does under a sliding window.

### P04 — Offscreen occluder clipping

Owner: `src/render/culling.rs`.

Confirmed: any vertex outside NDC rejects the entire projected primitive
(`culling.rs:300`). Large near-field occluders are exactly the geometry that
extends past the frame edge.

- [ ] Clip such primitives to the viewport instead of rejecting them.
- [ ] Prove the prepass stays conservative (never culls a visible primitive).
- [ ] Benchmark the recovered culling on an overlap-heavy scene.

### P05 — SSR / physical-transmission parallelism

Owner: `src/render/cpu_render/parallel_policy.rs`.

Source fact confirmed (`parallel_policy.rs:3`); the optimization is
**speculative**. Whether these are the two slowest paths, and whether they can
safely share a snapshot, needs a benchmark and a race-free design proof.

- [ ] Benchmark first. Do not implement before the measurement justifies it.
- [ ] If pursued, require a race-free design proof alongside the benchmark.

---

## 8. Forward backlog — not blockers

The v1.9.0 `F01`-`F09` list carries forward. Recommended order:

1. **HDR-linear post chain** (`F02`) — first; several items depend on it.
2. **Point and spot shadows** (`F01`).
3. **Versioned image diff** (`F08`).
4. **Deterministic camera paths** (`F04`).
5. **AgX tonemapper** (`F07`) — only after HDR-linear rendering.

Additional legitimate capability gaps:

- [ ] **Draco** mesh compression.
- [ ] **Linux physical headless-GPU lane.**
- [ ] **glTF/GLB export** (`F03`): the RFC describes an import-oriented
      surface but does **not** declare export forbidden. Export widens the
      charter and requires an RFC decision, but does not inherently conflict
      with renderer scope if kept debug/interchange-grade.

Agent-surface forward work (addressing `A4` — the current backlog is
renderer-only):

- [ ] `scena author --watch` delta loop. Useful, but lower priority than
      trustworthy diagnostics (`G03`) and response shaping (`G08`).
- [ ] Create a real forward backlog document **before** starting HDR/shadows,
      so the RFC stops pointing at a closing remediation file (`D04`).

---

## 9. Final integration and release checkpoint

Start only when checkpoints A-E are green and the release-candidate diff is
frozen.

- [x] Run the full `.codex/skills/scena-release-hygiene/SKILL.md` chain:
      `cargo fmt --all --check`, `cargo clippy --all-targets -D warnings`,
      `cargo test`, `cargo run -p xtask -- doctor --full`,
      `cargo doc -D warnings`. **All pass.**
- [x] Run the **whole CI lane**, not just that chain — the chain is a
      five-command subset. 28 of 29 commands pass; see section 9.4.
- [ ] Run the new feature-contract lane (`T01`). Needs a host that can finish
      the `--all-features` build; see section 9.5.
- [ ] Run `cargo build --target wasm32-unknown-unknown --tests`.
- [ ] Check the compressed-MiB number from `cargo publish --dry-run` (< 10 MiB).
- [ ] Sweep every version pin site for the 1.9.1 bump before pushing the tag.
      This is what makes `D02`'s `since = "1.9.1"` deprecation metadata true.
- [ ] Run the platform lanes required by release policy.
- [ ] Verify CI provenance/attestation fields and artifact digests.
- [x] Commit with a standalone product-change subject; no internal round
      labels. Three commits on `codex/v1.9.1-remediation`: the remediation batch,
      the inspection-only build fix, and this document's corrections.
- [ ] Push one frozen release candidate.
- [ ] Collect all failures from the deciding workflow before any fix/push.
- [ ] Require every configured required check and release artifact to be green.
- [ ] Verify branch, tag, release object, Latest marker, and crates.io
      publication.
- [ ] Redeploy and verify the public demo (`D05`).
- [ ] Confirm the `v1.9.0` release body correction landed (`D01`).

Closure condition: no known critical/high defect, every accepted input has
deterministic semantics or a structured rejection, no release claim can pass
without the evidence it names, every feature-gated test binary is executed by a
named workflow command, and the documented first-run commands are continuously
executed.

### 9.1 What is blocked on this host, and why

All 43 mandatory rows are implemented. Five carry one blocked sub-item each.
None of the five is blocked on a decision or on unfinished work; each needs a
capability this machine does not have. They are listed so nothing is silently
carried as done.

| Row | Blocked sub-item | Needs |
|---|---|---|
| `R01` | Meter pre-tonemap HDR-linear on the GPU surface path | Deferred to section 8 by the row itself; not a 1.9.1 blocker. |
| `T01` | Record the runtime cost of the feature-contract lane | A host that can finish `cargo test --workspace --all-features --tests`. It builds `basisu_c_sys` (native C) and exceeded a 10-minute budget twice here; a later run also hit a linker OOM at default parallelism. Retry at `-j 2`. Correctness of the lane is already proven by doctor reporting 0 bijection findings. |
| `D01` | Correct the `v1.9.0` GitHub release body in place | Authenticated write access to the release object. Outward-facing; left for the release step. |
| `D05` | Redeploy the public demo | Cloudflare credentials, **and** a released 1.9.1 renderer to deploy — it is sequenced after the release, not before. |
| `D07` | Land the two green Dependabot PRs | Authorization to merge into `main`. Triage of all four is complete, including the exact doctor-pin update `#9` needs. |

### 9.2 Environment misconfiguration, and what it hid

**An earlier revision of this section claimed `cargo test` could not pass here
because the host has no GPU adapter. That was wrong.** It is recorded rather
than deleted, because the standing rule "never convert unavailable hardware into
a pass" has a mirror image that was violated here: *never convert a
misconfiguration into an excuse.*

- 7 tests failed with `RequestDevice { backend: HeadlessGpu }`. I verified 2 of
  them against stashed 1.9.0 source and asserted, without checking, that the
  other 5 were the same.
- The fix was one environment variable:
  `VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json`, which forces Mesa
  lavapipe. `/usr/share/vulkan/icd.d/lvp_icd.json` was present the whole time.
- **`CLAUDE.md`'s own environment-flag table documents this exact host**: "On the
  Pi 5 / V3DV-broken hosts, point this at `/usr/share/vulkan/icd.d/lvp_icd.json`
  to force Mesa lavapipe (software Vulkan)." That table was read and edited
  during this batch (`D03`, `D14`) and never applied to the test runs.
- The related claim that "the full chain must run on CI" was also wrong. CI does
  not use different hardware: `ci.yml:38-41` installs `mesa-vulkan-drivers` and
  exports the same lavapipe ICD. The local run is a faithful reproduction of the
  lane, not a degraded substitute.

Two secondary host limits, both real and neither a code problem:

- A linker failure (`linking with cc failed`) on `m8_assets_materials_ecosystem`
  during the first lavapipe run was memory pressure — 15 GiB RAM linking several
  large test binaries at once. It links cleanly at `-j 2`.
- CI's default lane also exports `LIBGL_ALWAYS_SOFTWARE=1` and
  `SCENA_REQUIRE_PARITY=1`, neither of which had been set locally.

What was reported as blocked, and was not:

**An earlier revision of this section claimed `doctor --full` was blocked on the
1.7.1 demo deployment. That was also wrong.** The 6 `HONEST-MATERIAL-PRESETS`
findings came from a single **stale build artifact**:
`target/gate-artifacts/round-e-cloudflare-material-proof.json`, dated
**2026-06-22** — five weeks older than this batch — written by a previous
version of the probe. It has no `schema` field at all and records
`status: "pass"` where the contract requires `"passed"`.

`target/` is gitignored, so the file is not in the repository and not in any
commit. The rule
(`doctor_easy_scene/material_presets_cloudflare.rs:40-60`) returns silently when
the artifact is **absent** — guarded by
`round_e_material_parity_claimed_shipped()`, which is `false` here — and
validates strictly when it is **present**. A fresh CI checkout has no `target/`,
so this never fires there. It fired only on this machine, because of a leftover
file.

Removing it took `doctor --full` to **0 findings**, and
`tests_08::release_readiness_has_no_open_release_deferrals` and
`tests_10::easy_scene_setup_contracts_are_source_enforced` both pass.

The misdiagnosis persisted because the filename contains "cloudflare", which was
taken to mean the probe requires the live deployment. It does not:
`probe_cloudflare_material_presets.mjs:18` reads `process.argv[2]` first, and
`release.yml:109-113` serves the repository's `demo/` on `127.0.0.1:18104` and
probes **that**. CI has never probed the public site.

A real defect was found while investigating: the probe called
`page.waitForFunction(fn, { timeout: 120000 })`, but Playwright's signature is
`(pageFunction, arg, options)`. The options object was being passed as `arg`, so
the intended 120s budget silently fell back to Playwright's 30s default on every
run, CI included. Fixed by passing `undefined` as `arg`.

### 9.3 Measured integration-suite result on this host

`cargo test --tests --no-fail-fast` over **162 test binaries**, with
`VK_ICD_FILENAMES` pointed at lavapipe as `CLAUDE.md` and `ci.yml` both require:

**1283 passed, 0 failed.**

The progression, kept so the two real regressions are not lost:

| Run | Configuration | Result |
|---|---|---|
| 1 | no ICD set | 1274 passed, 9 failed |
| 2 | no ICD set, after the `feature_unavailable` fix | 1276 passed, 7 failed |
| 3 | `VK_ICD_FILENAMES` = lavapipe, `-j 2` | **1283 passed, 0 failed** |

- Runs 1 to 2 closed the only **real** failures: two `X01` regressions.
- Runs 2 to 3 closed 7 failures that were never failures — see section 9.2.
  Those tests now *execute* under the software rasteriser instead of dying at
  renderer construction, which is why the passing count rises by more than 7.

**Two real regressions, both introduced by `X01`, both fixed:**

- `a04_packaged_cli_contract::packaged_cli_matches_the_declared_install_feature_contract`
  and
  `a09_feature_discoverability::unavailable_agent_commands_name_one_installable_feature_remedy`
  failed with `left: "unsupported", right: "feature_unavailable"`.
- Cause: `X01` routed `feature_required` through `CliErrorKind::Unsupported`,
  which emits code `unsupported`. The prose classifier being replaced had a
  *separate* branch (`contains("unavailable in this build")` to
  `feature_unavailable`), and that code is published contract: `a04` asserts
  exit 69 with it.
- Fix: a distinct `CliErrorKind::FeatureUnavailable` mapping to
  `("feature_unavailable", CliExitClass::Unsupported)`. `Unsupported` now means
  only "the host lacks a capability".
- This is `X01`'s own hazard recurring inside `X01`: a `code` string the prose
  branch produced was absent from the typed enum, so typing the site silently
  changed a published code. Enumerate every `code` the old path could emit
  before replacing it.
- Caught only by the **whole** integration suite. Neither `X01`'s scoped gate nor
  `cargo test -p xtask` touches those binaries. The four affected suites now pass
  (`a01_cli_error_taxonomy`, `a04_packaged_cli_contract`,
  `a09_feature_discoverability`, `a13_error_remedies`).

### 9.4 Local chain result

The default CI lane (`ci.yml`, job `linux-native-vulkan`) was reproduced
locally, command for command — 28 commands from "Cargo gates" plus the Docs
step — with the environment CI sets (`VK_ICD_FILENAMES` at lavapipe,
`LIBGL_ALWAYS_SOFTWARE=1`, `SCENA_REQUIRE_PARITY=1`) and `-j 2` to avoid a
linker OOM on a 15 GiB host.

**This matters because the `.codex/skills/scena-release-hygiene` chain cited
throughout this batch is a five-command *subset* of that lane.** Running the
real lane found three defects the subset could not:

| Defect | Found by |
|---|---|
| `feature_unavailable` code silently changed to `unsupported` by `X01` | `cargo test` (full integration suite) |
| Five `X01` stubs left untyped, breaking `--features inspection` without `scene-host` | `cargo test --features inspection --test m7_threejs_ergonomics` |
| Stale `round-e-cloudflare-material-proof.json` failing 6 doctor rules | `cargo test -p xtask` |

First lane run: **26 passed, 3 failed**. All three were fixed and each fix
verified in isolation (`tests_08`/`tests_10` pass; `m7_threejs_ergonomics` 94
passed; `measurement_visual_proof` 1 passed), then committed and the lane re-run.

Notable commands passing here that had never been run before this session:
`cargo test --doc`, `cargo run -p xtask -- claim-audit`,
`RUSTDOCFLAGS="-D warnings" cargo doc --no-deps --all-features`,
`cargo test --lib --features scene-host,inspection`, and the feature-gated CLI
suites `scena_cli_agent`, `scena_cli_recipe`, `scena_cli_agent_templates`,
`scene_recipe_contracts`, and `label_text`.

Feature-combination build sweep: default, `inspection`, `scene-host`,
`scene-host,inspection`, `agent`, and `ktx2` all build. `--all-features` was
still linking when a 10-minute budget expired; it is the `basisu_c_sys` native C
build and is a host-speed limit, not a code result.

### 9.5 What still requires access this host does not have

- **`D05` public redeploy.** `https://scena-demo.pages.dev/` serves
  `1.7.1-public-a76468dfd9ba`; the repository's `demo/` is
  `1.9.0-public-0215db283505`. No workflow in this repository deploys it, so it
  needs Cloudflare credentials. This is **separate from any gate**: doctor,
  `cargo test`, and the whole lane pass without it.
- **`D01` release-body correction** and **`D07` Dependabot merges** — both
  outward-facing, both needing authorization rather than capability.
- **`T01` runtime cost** — needs a host that can finish
  `cargo test --workspace --all-features --tests`.
- **Platform lanes** — macOS Metal, Windows DX12, and the browser lanes cannot
  run here at all and must come from CI.

---

## 10. Defects found after sign-off — demo hero work (2026-07-26)

Found while rebuilding the public demo hero, **after** the 43 remediation rows
were signed off. None of these are regressions from `R01`-`D14`; all predate
this checklist. They are recorded here because the hero work is the first thing
in the repository that exercised these paths end to end from a caller's
position rather than from a test's.

Prefix `H` (hero). Not yet scheduled — see the plan discussion before
committing to an order.

### H01 — `ScreenSpaceReflectionConfig` is a frame mirror, not a reflection

- [ ] **Defect.** `screen_space_reflections::apply_rgba8`
      (`src/render/screen_space_reflections.rs:96`) computes
      `horizon_y = target.height * config.horizon_fraction()` and, for every row
      below it, blends a vertically mirrored sample across the **full frame
      width**. It never consults the floor plane, the floor's extent, scene
      depth, or surface normals. The public name, the `studio_floor()` preset,
      and the docs all present it as a floor reflection.
- [ ] **Consequences.** Reflections appear beyond the floor's edges and over
      background; the mirror line sits at an image-height fraction rather than
      the geometric horizon, so it does not move with the camera. Combined with
      `set_supersample_factor(2)` **and** the rest of the post chain it resolves
      into a hard, obviously displaced second copy of the subject — reported
      from outside as "two images of the same hero, a little shifted".
- [ ] **Reproduction.** Held at
      `scratchpad/repro/diag_hero_toggles.rs`: six variants over
      {supersample, SSR, AO+bloom+DoF}. Only the all-three variant produces the
      duplicate; SSR alone and supersample alone are clean.
- [ ] **Decision needed.** Either implement a real reflection (mirrored
      geometry under the floor plane, or depth/normal-buffer SSR), or rename
      the type and preset to describe what it does and document the limitation.
      Shipping it under the current name is the actual defect.
- [ ] **Current mitigation.** `examples/hero_machine.rs` does not enable SSR and
      records why inline.

### H02 — `Scene::mate` rejects the repository's own canonical connector assets

- [ ] **Defect.** `cargo run --release --example mate_two_parts` fails at
      runtime with `SnapToleranceExceeded { distance: 0.615, tolerance: 0.01 }`.
      `validate_snap_tolerance_for_apply`
      (`src/scene/connectors/solving.rs:32`) compares `preview.snap_distance` —
      the distance the source part must **travel** to reach the mated pose —
      against `snapTolerance`, which
      `scripts/generate_connector_demo_assets.js:192` authors as `0.01`, a
      **seating/fit** tolerance. Those are different quantities.
- [ ] **Consequences.** Default `mate()` can only succeed on parts that are
      already assembled, which is the opposite of its doc comment ("Mate two
      imported parts by named connector"). Every shipped example that mates —
      `mate_two_parts`, `easy_scene_showcase`, `connector_auto_framing`,
      `connector_snap_hero` — fails at runtime.
- [ ] **Decision needed.** Either compare the **residual misalignment after**
      the mate (~0) rather than travel distance, or scope the tolerance check to
      interactive drag-snapping and exempt an explicit `mate()` call.

### H03 — CI compiles examples but never runs them

- [ ] **Defect.** Every lane uses `cargo check --examples --all-features`
      (`ci.yml:72`, `ci.yml:263`, `release.yml:66`, `release.yml:210`,
      `release.yml:243`). The single exception is
      `native_surface_hardware_proof` on the manual hardware workflow.
- [ ] **Consequences.** This is what let `H02` ship: four examples that fail on
      their first line of real work are green in CI indefinitely. Examples are
      the documented entry point for new users, so a runtime break there is
      more damaging than a comparable break in library internals.
- [ ] **Fix.** Run the examples that need no GPU/network in a lane, or add a
      smoke test that executes each and asserts exit status.

### H04 — `AutoExposureConfig::product_studio()` underexposes a dark-studio product shot

- [ ] **Observation, not yet isolated.** The hero frame — a small bright
      subject against `Background::DarkStudio` — meters roughly **4 EV** under.
      `examples/hero_machine.rs` compensates with a fixed
      `set_exposure_ev(4.0)` and says why inline.
- [ ] **Confirmed not caused by `R01`.** On first application `current_ev` is
      `0`, so the old and new arithmetic agree, and `auto_exposure_attempted`
      guards a single application.
- [ ] **Not the cap.** `AutoExposureConfig::product_studio()` is capped at
      `max_ev: 0.65` (`src/render/exposure.rs:133`), and raising it was tried
      and reverted. Against the preset's own `Background::Studio`, the frame
      metered at `mean_luminance 0.098` while the meter was *not* asking for
      lift: the whole-frame average already sat near target because the field
      is bright and the subject is small. The cap only binds once the
      background is overridden to dark. Raising it also inverts a deliberate
      ordering pinned by
      `tests/round_c_auto_exposure_presets.rs` (`indoor.max_ev() >
      product.max_ev()`), which encodes that a product studio is controlled
      lighting needing less lift.
- [ ] **Actual defect.** Metering is whole-frame average, so a small subject in
      a large field is exposed for the field rather than for the subject. This
      is the one row here that needs design work, not a constant change:
      subject-weighted or region-weighted metering. Until then the correct lever
      is an explicit `render.exposure_ev`, which `H10` makes usable.
- [ ] **Fix.** Region-weight `product_studio` metering toward the framed
      subject. Do not raise the cap on its own.

### H05 — A glossy `add_grid_floor` blows out at grazing camera angles

- [ ] **Observation.** `GridFloorOptions::roughness(0.28)` under a 12-degree
      camera elevation produces a large white specular sheet across the floor
      whose position is unrelated to the subject. Evidence:
      `d-glossyfloor` variant of the `H01` repro.
- [ ] **Likely not a code defect** — a smooth dielectric at grazing incidence
      is expected to behave this way — but the helper is presented as the easy
      path and its default is matte (`0.96`), so lowering roughness is an
      inviting mistake with an ugly result. Warrants a documented caution in
      `docs/guides/easy-scene-setup.md`.

### H06 — Grounding expectations cannot address imported nodes

- [ ] **Defect.** `expect_grounded[].target.kind` and
      `expect_quality.grounding.target.kind` accept only `import`, `node`, and
      `world`. `kind:"import"` is rejected at build with
      `unsupported_feature` ("this expectation target does not support
      whole-import matching"), and per
      `docs/specs/recipe-spatial-state-v1.md:19` `kind:"node"` addresses **an
      authored recipe node** only. Imported nodes are addressed by
      `kind:"import_node"`, which the expectation target enum does not accept.
- [ ] **Consequence.** A recipe whose subject is an imported glTF/GLB — the
      normal case for a product still — cannot assert that its subject rests on
      the floor. Passing `imports[].nodes_by_path` keys such as
      `machine:/drive_unit/drive baseplate` as a `node` id fails verification
      with `ground_target_unresolved`.
- [ ] **Why it matters.** "Is the subject floating?" is the defect the demo
      hero actually shipped with, and a human caught it by eye after several
      rounds of machine-green renders. `expect_grounded` is the check built for
      exactly that failure and it is unreachable for the case that needs it.
- [ ] **Fix.** Accept `kind:"import_node"` (with `import` + `path`) in
      expectation targets, or resolve `imports[].nodes_by_path` keys as valid
      `node` ids. Either closes the gap without new vocabulary.
- [ ] **Evidence.** `demo-next/hero.recipe.json` at
      `scena validate-recipe --full` and `recipe render --gpu --verify`;
      the grounding expectations had to be removed to obtain a passing render.

### H07 — The `product` quality profile cannot fail an unusable exposure

- [ ] **Defect.** `expect_quality.profile:"product"` gates subject exposure on
      `max_low_clip_fraction: 0.8`. A frame whose subject region is 57% clipped
      to black at `mean_luminance = 0.098` reports `subject_exposure_sane` and
      `severe_black_crush` as `status:"checked", severity:"info"`.
- [ ] **Measured evidence.** `demo-next/hero.recipe.json` rendered through
      `recipe render --gpu --verify` with `scene.preset:"product_studio"`:
      `subject_exposure_sane` observed
      `{low_clip_fraction: 0.371, mean_luminance: 0.098}` against threshold
      `{max_low_clip_fraction: 0.8}`; `severe_black_crush` observed `0.569`
      against `0.8`. The rendered image is visibly unusable — the subject reads
      as a black silhouette.
- [ ] **Why it matters.** The guide sells this profile as the check that
      catches `subject_black_crushed`. At a 0.8 ceiling it only fires on a
      near-total blackout, so it certifies images no one would ship. This is
      the same failure mode the SSR work hit from the other direction: a green
      machine result on a bad frame.
- [ ] **Interaction with `H04`.** The underlying cause of the dark frame is
      that `scene.preset:"product_studio"` pairs `Background::Studio` (bright)
      with `AutoExposureConfig::product_studio()`. A small bright subject in a
      large bright field meters low. The preset's own two halves fight each
      other, and its own quality gate does not notice.
- [ ] **Thresholds (located).** `src/render/quality/types.rs:198` for the
      product profile, and the same loose default at
      `src/scene_host/composition/object_pixels.rs:199` for object-level
      composition.
- [ ] **Fix.** Tighten `max_low_clip_fraction` to a value that fails a
      silhouette (a subject at `mean_luminance < 0.15` should not pass), and
      add a mean-luminance floor to the product profile.

### H08 — Recipe framing cannot express a custom camera angle

- [ ] **Gap.** `framing_presets` is a closed list of eleven fixed views
      (`front`, `isometric`, `three_quarter_front_right`, ...). The Rust API
      offers `FramingOptions::azimuth_elevation(-34.0, 12.0)` and the guide
      documents it, but the recipe surface exposes no equivalent. The only
      recipe escape is `cameras[].transform` with `kind:"look_at"` and literal
      eye coordinates — exactly the hand-typed camera distance the framing
      helpers exist to avoid.
- [ ] **Consequence.** An agent authoring a hero still cannot pick the low
      three-quarter angle product photography actually uses; it must accept a
      fixed preset elevation or drop to raw coordinates.
- [ ] **Fix.** Accept `framing: {azimuth_degrees, elevation_degrees}` in the
      recipe camera block, routing to the existing `FramingOptions` method.

### H09 — `scene.grid.reflection` renders as flat grey quads, not a reflection

- [ ] **Defect.** With `scene.grid.reflection:{enabled:true, strength:0.55}`
      the floor shows hard-edged, flat light-grey rectangles offset from the
      subject rather than a reflected image. The rectangles have straight
      axis-aligned borders unrelated to the subject silhouette.
- [ ] **Interaction with the gate.** At `strength:0.32` the same floor fails
      `reflection_structure_missing` with `sobel_energy 0.019` against a `0.020`
      threshold; at `0.55` it passes the gate while looking visibly wrong. So
      the reflection check can be satisfied by an artifact — passing it is not
      evidence the floor reads as reflective.
- [ ] **Why it matters.** `docs/guides/llm-app-builder.md` names this the
      product-floor reflection path and contrasts it with material SSR
      ("without requiring material SSR"). It is the documented answer for the
      most common hero requirement, and it is the third distinct reflection
      mechanism in the codebase to produce a wrong image — after
      `ScreenSpaceReflectionConfig` (`H01`) and the glossy grid floor (`H05`).
- [ ] **Confirmed by isolation.** With exposure fixed (see `H10`) the
      artifacts are unmistakable white rectangles:
      `evidence/demo-hero/fixed-exposure-with-reflection-artifacts.png`.
      Removing `scene.grid.reflection` and changing nothing else eliminates
      them: `evidence/demo-hero/fixed-exposure-no-reflection.png`. The two
      recipes differ only in that key.

### H10 — `scene.preset` silently overrides fixed `render.exposure_ev`

- [ ] **Mechanism (corrected after review).** `exposure_ev` is *not* dropped;
      it is applied and then overwritten. `src/scene_host/recipe/setup.rs:86`
      calls `set_exposure_ev`. Afterwards `scene.preset` runs
      `apply_scene_setup_preset_renderer`, which installs the preset's auto
      exposure whenever `renderer.auto_exposure().is_none()`
      (`src/scene_host/product.rs:137`). `Renderer::set_exposure_ev` sets only
      the fixed EV and never clears auto exposure
      (`src/render/settings.rs:133`), so that guard is still `none` and the
      preset re-enables metering on top of the fixed value.
- [ ] **Compounding cap.** `AutoExposureConfig::product_studio()` is capped at
      `max_ev: 0.65` (`src/render/exposure.rs:133`). The shot needs roughly
      +4 EV, so even uncapped metering could not rescue it — the preset cannot
      expose this class of frame at all.
- [ ] **Observed effect.** Rendering `evidence/demo-hero/hero.recipe.json` with
      `render.exposure_ev: 12.0` produces a frame indistinguishable from the
      same recipe without it. Measured full-frame mean luminance:
      **26.9 (baseline) vs 27.1 (EV+12)** on 0-255. Twelve stops is a 4096x
      linear increase; the frame should be fully blown out. The render report
      emits no warning that the field was dropped.
- [ ] **Contrast with the Rust API.** `Renderer::set_exposure_ev(4.0)` in
      `examples/hero_machine.rs` visibly and correctly brightens the identical
      model, lighting rig, and environment. The recipe field does not route to
      the same behavior.
- [ ] **Blast radius.** Exposure is the only lever an agent has once a scene
      preset has been chosen, because `auto_exposure` and `exposure_ev` are
      mutually exclusive in v1. With `exposure_ev` inert and
      `AutoExposureConfig::product_studio()` metering a bright field low
      (`H04`), a recipe-authored product still has **no working exposure
      control at all**. Every attempt in this session — `auto_exposure`
      variants, fixed `exposure_ev`, a `dark_studio` background override, an
      explicit `scene.environment:{preset:"studio"}` — left the subject pinned
      between `mean_luminance` 0.083 and 0.096, rendering the steel assembly
      as a black silhouette.
- [ ] **Consequence for the agent surface.** This is the headline finding of
      the demo-hero work. `docs/guides/llm-app-builder.md` presents the recipe
      surface as the way an LLM produces a good-looking scene, and following it
      exactly yields an unusable frame that the accompanying quality gate
      (`H07`) certifies as sane. The equivalent Rust composition renders
      correctly, so the gap is in the recipe layer, not the renderer.
- [ ] **Fix.** Route `render.exposure_ev` to `Renderer::set_exposure_ev`; if a
      scene preset's auto exposure takes precedence, reject the combination at
      validation instead of dropping the field silently.
