# Placeholder remediation checklist

Every public contract field must carry **computed** data, every public API knob must
**change behavior**, every proof must assert **correctness** (not mere presence), and no
reported metadata may be a **hardcoded literal presented as observed**.

**Completion rule for every item below:** it ends in exactly one of two states —

1. the **real implementation** (the field/knob/proof does what its name claims), or
2. the placeholder **removed** (field, enum variant, knob, or claim deleted from the
   public surface and contract), with the fixture/schema updated.

"Leave as-is / documented MVP" is not an acceptable end state.

Locations are file:line as of the audit; confirm before editing. Effort: **S** = compute
from data already in scope · **M** = moderate · **L** = real renderer feature.

---

## A. Contract fields emitting placeholder data (highest priority)

- [x] **A1** `render_introspection.nodes_detail[].coverage` — `src/render/introspection.rs:147`.
      Always `"unknown"` for visible nodes. Implement real per-node pixel coverage (object-id
      buffer keyed by node handle, or project node world-AABB to screen and sample non-background
      pixels in that bbox — `appearance/regions.rs` already projects bbox). Acceptance: two nodes
      with different on-screen sizes report different coverage; a fully-occluded node reports zero. **L**
- [x] **A2** `render_introspection.nodes_summary.transparent` — `:355`. Hardcoded `0`. Count
      `inspection.draw_list` entries with `alpha_mode != "opaque"` (or base_color.a < 1). Acceptance:
      a scene with one transparent material reports `1`. **S**
- [x] **A3** `render_introspection.nodes_summary.clipped` — `:354`. Hardcoded `0`. Count draws fully
      outside active clipping planes / section box. Acceptance: a section box that excludes a draw
      increments this. **M**
- [x] **A4** `render_introspection.nodes_summary.unknown_coverage` — `:358`. Mirrors `visible`. After
      A1 lands, this is the count of nodes whose coverage is genuinely indeterminate, not an echo. **S**
- [x] **A5** `render_introspection.reasons[].affected_handles` — `:428`. Always empty. Populate from the
      handles already in scope at each reason site (alpha_zero → nodes with a≈0; nan_transform → the
      offending node). Acceptance: an alpha-zero reason lists the node handle. **S**
- [x] **A6** `render_introspection.fixes[].target_handle` + `fixes[].patch` — `:439,440`. Always `None`.
      Emit the node handle and an apply-ready `scena.visual_patch.v1` body, exactly as
      `visibility_diagnosis` already does for the same actions. **S**
- [x] **A7** `interaction_verification.observed.hover_handle` / `selection_handle` —
      `src/bin/scena/verify_interaction.rs:81,87,93`. These mirror the `pick()` return instead of
      reading persisted state, which also produces **wrong verdicts** for `expect_hover`/`expect_selection`.
      Read `host.interaction().hover()` / `.primary_selection()` and resolve via `handle_for_hit_target`.
      Acceptance: a selection that persists across a later hover step is reported on that step. **S**
- [x] **A8** `interaction_verification.summary.rendered_feedback_checked` —
      `src/scene_host/interaction_verification.rs:220`. Always `false`. Either re-render after the
      interaction and diff the highlighted region to set it truthfully, or remove the field. **M**
- [x] **A9** `appearance.targets[]` no-draw fallback — `src/render/appearance.rs:108`. Targets matched via
      node/tag with no draw row sample the **whole-frame** average and feed it into color_family/swatch.
      Always resolve the target's draw/node-AABB and sample within its own screen bbox (the draw-backed
      path already does). Also `appearance.fixes[].patch` (`evaluation.rs:289`) is always `None` — emit a
      patch (e.g. set_alpha → material patch). **M**
- [x] **A10** `scene_host_grounding` does not ground — `src/scene_host/product.rs:62-81`. `target` echoes
      the input handle; no drop offset is applied. `physical_shadow_claimed` hardcoded `false`,
      `floor_receiver` hardcoded `true`, `DirectionalShadowReceiver` never constructed. Compute and apply
      a real bounds-based drop to the target transform; derive the flags from actual receiver state. **M**
- [x] **A11** `asset_readiness` — `src/assets/catalog/`. `preview.status` always `"declared"` (`checks.rs:151`)
      and the preview path is never fetched (`checks.rs:111-125`); `summary.warning_count` always `0` and
      `AssetReadinessSeverityV1::{Warning,Info}` are defined but never constructed (`types.rs:155-156`).
      Fetch the preview, set status accordingly, and classify soft findings (missing optional
      license/provenance, skipped-unsupported texture) as Warning/Info. **M**
- [x] **A12** `asset_load_report` glTF provenance/geometry — `src/assets/gltf.rs:109`,
      `src/assets/gltf/scene_asset.rs:146`. `provenance.generator` never read from `asset.generator`;
      `license`/`derivatives` always null/empty; `geometry.source_coordinate_systems` hardcoded
      `Vec::new()`. Populate `generator` from the glTF JSON, collect real coordinate systems (mirror
      `source_units_summary()`), and either record real derivatives or drop the field. **S**
- [x] **A13** `asset_load_report.material_fallbacks` — `src/assets/load.rs:206`,
      `src/assets/gltf/material_fallbacks.rs:56-64`, `textures.rs:56-75`. Single variant
      (`TextureBasisuFallback`), and it is inert when the `ktx2` feature is on. Add and emit variants at the
      real degrade sites (unsupported material extension, missing-texture defaulted), or scope the field to
      basisu-only and document it cannot fire under `ktx2`. **M**
- [x] **A14** `asset_load_report.external_resources` `SkippedUnsupportedFormat` not surfaced in readiness —
      `src/assets/catalog/checks.rs:162` only inspects `Missing`. Emit a finding for skipped-unsupported
      entries so an undecodable texture does not pass readiness silently. **S**
- [x] **A15** `host_event.Pick.button` / `Pick.modifiers` — `src/scene_host/events.rs:240`. Always
      `None`/default; `pick()` accepts no button/modifier args, and `HostEventButtonV1::*` is never
      constructed. Thread button + modifiers through `pick()` and populate, or remove the fields + enum. **M**
- [x] **A16** `host_event.HostEventV1::DeviceRecovered` — `src/scene_host/events.rs:88`. Defined, never
      emitted (no upstream `SurfaceEvent::DeviceRestored`). Add the surface signal and emit it on real
      device recovery, mirroring `ContextRestored`, or remove the variant. **M**
- [x] **A17** `scene_inspection.draw_list[].visible` — `src/scene/inspection/builders.rs:177`. Tautologically
      `true` (list is pre-filtered to visible). Drop the field, or stop pre-filtering and compute real
      per-draw visibility. **S**
- [x] **A18** `SceneHostErrorCode::NoActiveCamera` — `src/scene_host/error.rs:23`. Never constructed. Emit it
      where a SceneHost op needs an active camera and none is set, or remove the dead variant. **S**

## B. Validation declared but unenforced / unreachable

- [x] **B1** Recipe library validator skips real checks — `src/scene/recipe/validation.rs:285-372`.
      `expected_extent` is only shape-checked (never compared to actual asset bounds) and `uri` existence is
      never checked; the real checks live only in the `validate-recipe` CLI. Fold extent-vs-bounds and
      asset-load checks into the library contract, or rename/scope `scena.scene_recipe_validation.v1` as
      shape-only and state the real checks require the CLI. Also confirm extent-mismatch severity
      (`validate_recipe.rs:51-67` emits `warning`, so `ok` stays true). **M**
- [x] **B2** Connector `unit_mismatch` / `coordinate_system_mismatch` unreachable —
      `src/scene/connectors/validation.rs:41,50`. Both checks are gated behind `import_live.is_none()`, but
      every browser candidate sets `import_live = Some(..)`, so mismatched units/coordinate systems between
      two imports pass as `compatible:true`. Run the checks for import-sourced frames, or remove the
      unreachable `invalid_reasons` mappings. **M**
- [x] **B3** Connector `roll_policy` never drives geometry — `src/scene/connectors.rs:127,235`. Stored and
      serialized but never read by `roll_transform`; roll comes only from `ConnectOptions`. Map the connector
      policy into the applied roll, or mark the field report-only and remove it from the apply contract. **M**
- [x] **B4** Connector `clearance_hint` never consumed — `src/scene_host/connectors/types.rs:48`. Parsed,
      stored, serialized; drives no clearance/gap logic. Enforce a real min-gap (surface as candidate
      distance/invalid_reason), or document advisory-only and stop implying a guarantee. **M**
- [x] **B5** Connector `magnet.tolerance` does not gate the mate — `src/scene/connectors/magnet.rs:73-88`;
      `Scene::connect` ignores `snap_tolerance` entirely. Tolerance only flips a visual `SnapReady`/`OutOfRange`
      cue; the full mate transform applies regardless of distance. Either withhold the mate when
      `distance > tolerance`, or remove the implication that magnet snapping is tolerance-gated. **M**
- [x] **B6** Connector alignment axis semantics collapsed — `src/scene/connectors/options.rs:136-145`.
      `ForwardToForward`/`NormalToNormal` both map to identity and `ForwardToBack`/`NormalToOpposite` both to a
      fixed 180° Y flip; forward vs normal axis is never distinguished. Compute the real axis-mapping rotation,
      or remove the modes that are not implemented. **L**

## C. Public API knobs that do nothing (verified dead — implement or remove)

- [x] **C1** `RenderSettings` `Quality` (Low/Med/High) — `src/render/settings.rs:53`. Stored, never branches
      rendering. Drive MSAA/SSAA/shadow-res/IBL budget on it, or remove the knob. **L / S-to-remove**
- [x] **C2** `Material::double_sided` — `src/material.rs:64`. Never drives `cull_mode` (zero hits in
      `src/render/`). Add back-face culling for single-sided + a no-cull variant, or remove. **M**
- [x] **C3** Orbit `damping_factor` — `src/controls.rs:93`. Never integrated in `advance()`; the
      cinematic/snappy/presentation presets differ only in this dead field. Implement velocity + exponential
      decay, or remove the field and collapse the presets. **M**
- [x] **C4** `DebugOverlay` renderer API surface — removed instead of retaining an inert
      overlay knob.
      No render branch; every variant renders identical to `None`. Implement a pipeline/shader branch per
      variant, or remove the enum + setter. **L**
- [x] **C5** `hover_style` / `selection_style` (`InteractionStyle`) — `src/render.rs:97`. No outline/highlight
      pass; picked objects render identically. (Confirm: `InteractionStyle::color()`/`outline_width_px()`
      appear uncalled.) Add a screen-space outline pass, or remove the styles. **L**
- [x] **C6** `LabelRasterization::Sdf`/`Msdf` — `src/scene/labels.rs:25`. Always the hardcoded 5×7 bitmap;
      both variants produce byte-identical output. Implement real SDF/MSDF atlas generation + sampling shader
      (branch single- vs multi-channel), or remove the variants and keep one honest `Bitmap` path. **L**
- [x] **C7** Minor dead surface — implement-or-remove each: `InstanceCullingPolicy` single variant with no
      branch (`src/scene/instances.rs:11`); `AnchorFrame::bounds_hint` never read (`src/scene/anchors.rs:19`);
      `TextureTransform.tex_coord` with no 2nd UV channel (`src/material/types.rs:12`); `CalloutReport.anchor_kind`
      never serialized/read (`src/scene/callouts.rs:42`); connector `lock` per-axis DOF claimed but whole-node
      only (`src/scene/connectors/locks.rs`). **S–M each**

## D. Hardcoded metadata reported as observed

- [x] **D1** `src/browser_probe/workflows/ergonomics/source_materials.rs:105-106` — `"camera_framing":"Scene::frame"`,
      `"lighting":"DirectionalLight"` literals. Report the actual camera/framing bounds and the actual lights
      enumerated from the scene. **S**
- [x] **D2** `src/browser_probe/probes.rs:352-353` — `context_recovered` and `device_recovered` set to the same
      single render's `draw_calls`. Capture each recovery render's result separately. **S**
- [x] **D3** `src/browser_probe/probes/state_lifecycle.rs:161-170` — eight `dirty_state` keys all hardcoded
      `"requires explicit prepare"`. Emit the actual observed reason per edit type. **S**
- [x] **D4** `src/browser_probe/workflows.rs:337` — `"rasterization":["sdf","msdf"]` (both are the same bitmap
      path). Report the truth (`"bitmap-5x7"`) until C6 lands. **S**
- [x] **D5** Remaining hardcoded label/status strings — `probes.rs:118` `"final_prepare":"ok"`;
      `scene_host_browser_proof.js:3022,3080` hardcoded `status:"passed"` + editorial/hardware prose; static
      `scene_api`/`prepare_api`/`render_api` labels. Derive from results; derive `status` from a real pass/fail
      tally; populate hardware from `pageProof.webgl.renderer`. **S–M**

## E. Weak or broken proofs

- [x] **E1** `tests/m6_browser_renderer_parity.rs:43,47` — the only CI-headless browser test silently `return`s
      on build/render failure and checks only `draw_calls`/`gpu_submissions` (zero pixel inspection). Remove the
      silent returns (fail instead) and add a pixel assertion. **S**
- [x] **E2** `tests/m1_visual_proof.rs:361` — `validate_nonblack` is an empty no-op body wired as the validator
      for 6 fixtures. Make it assert, or delete the shim and its call sites. **S**
- [x] **E3** Missing fixture `tests/assets/environment/studio_1024x512.hdr` referenced by
      `tests/m2_visual_proof.rs:239` (`.expect`, not in git). Commit the asset or fix the test; confirm the IBL
      proof actually runs. **S**
- [x] **E4** No reference-image / ΔE comparison anywhere in the browser path — ~50 assertions are `nonblack>0`
      or `before≠after`. Wire committed golden PNGs + a per-pixel/ΔE diff (the `reference_image` regression API
      exists) into the CI-run `browser:m6` path; upgrade the depth/light/shadow/normal checks from
      channel-dominance to golden ΔE. **L**
- [x] **E5** WaterBottle golden diff is off by default — `tests/m8_real_asset_proof.rs`. Double env-gated
      (`SCENA_REFERENCE_DIFF` + `SCENA_RUN_UNSTABLE_…`), so default `cargo test` only SHA-pins bytes; and the
      metric is RGB-Chebyshev mislabeled "ΔE". Promote the live-vs-golden diff into a real CI lane and fix the
      naming (or implement ΔE2000). **M**
- [x] **E6** `tests/browser/scene_host_browser_proof.js` runs in no CI/release lane and hardcodes
      `status:"passed"` (cannot report failed). Add it to a CI lane (or stop labeling it a proof), and derive
      `status` from a real tally. **M**
- [x] **E7** `tests/reference_image_regression_api.rs` exercises the diff API only on 2×1 inline byte arrays —
      never on renderer output. Point it at a real capture + committed golden. **S**

## F. GPU/CPU fidelity divergence (headline GPU path drops what CPU computes)

- [x] **F1** GPU encodes only the first light per type — `src/render/prepare/lighting.rs:155,171,185`. 2nd+
      directional/point/spot lights are silently dropped on GPU while the CPU path loops all. Loop all lights
      per type in WGSL using the already-uploaded counts. **L**
- [x] **F2** KHR_materials_volume attenuation ignored on GPU — `src/render/gpu/material_uniform.rs:139`.
      `attenuation_distance` packed but never read in WGSL; `attenuation_color` never packed. Read `.w` and pack
      the color; implement Beer-Lambert volume attenuation on the GPU path (CPU already does). **L**

---

## Guardrails to prevent recurrence

- [x] **G1** Per agent-facing contract, add a test that runs the **real builder** on two distinct scenes and
      asserts the would-be-placeholder fields actually differ (e.g. `transparent`, `clipped`, per-node
      `coverage`, `affected_handles`) — a constant value across inputs fails the test.
- [x] **G2** Add a "no inert public knob" test: for each public render/material/controls setting, set two
      distinct values and assert an observable difference (rendered stat or pixels). This class (C1–C5) would
      have been caught by it.
- [x] **G3** Keep the bidirectional doctor catalog↔FIXTURES check (already added) and extend the spirit: no
      contract field may be a hardcoded literal in its builder without an inline justification that it is
      genuinely constant.
- [x] **G4** Every "proof" test must assert on rendered output (pixels/golden), not only counters; and must not
      `return` early on failure. Audit `tests/*visual_proof*` and the browser harnesses against this rule.

## Definition of done for the sweep

- No item above remains in placeholder state (each implemented or removed, with fixtures/schemas updated).
- `cargo run -p xtask -- doctor --full` passes; `cargo fmt --check`, `clippy --all-targets -D warnings`
  (default and `--features scene-host,inspection`), `cargo test` (default + features), and
  `RUSTDOCFLAGS="-D warnings" cargo doc` pass.
- G1–G2 guard tests exist and pass.
