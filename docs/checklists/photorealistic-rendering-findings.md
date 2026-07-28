# Photorealistic Rendering — Post-Implementation Findings

Date: 2026-07-27

Findings from an end-to-end test of the camera-behavior workflow after
`docs/checklists/photorealistic-rendering.md` was completed (139/139). Three
self-authored glTF subjects plus the public demo hero were rendered through
`scena photo render --intent camera-behavior` and `scena recipe render --gpu
--verify`, and every claim below is backed by a measurement recorded here.

Test subjects (single-root GLB, five PBR materials each, no authored camera,
lights, floor, or staging):

- `dark_metal_speaker` — anodized-black aluminium, 30,192 tris
- `colored_travel_mug` — saturated teal/orange plastic, 9,380 tris
- `valve_manifold` — chrome/brass/steel assembly, 33 parts, 32,256 tris

Host: Raspberry Pi 5, Mesa 25.0.7-2+rpt4. GPU results in sections 1-5 are
**lavapipe software rasterization**; the V3D adapter is refused by default (see
section 6, which also records V3D hardware results obtained with the documented
escape hatch after the LTC fix). No result here is hardware release evidence.

## 1. Outcome

| Asset | `photo render --gpu` | `recipe render --gpu --verify` | Gate |
|---|---|---|---|
| dark_metal_speaker | timed out at 2400 s | exit 0, 121 s | passed |
| colored_travel_mug | timed out (skipped) | exit 0, 165 s | passed |
| valve_manifold | (skipped) | exit 1, 201 s | failed: `subject_fill_below_min`, `subject_clipped_by_frame` |
| demo hero | — | exit 1, 344 s | failed: `subject_fill_below_min` (`fit 0.644` vs min 0.65) |

Measured image statistics (subject + surroundings, full frame):

| Image | mean L | p99 L | saturation | % > 250 |
|---|---|---|---|---|
| speaker before → after | 41.5 → 140.9 | 165 → 204 | 0.301 → 0.134 | 0.000 → 0.040 |
| valve before → after | 43.4 → 51.8 | 139 → 157 | 0.293 → 0.230 | 0.001 → 0.003 |
| hero camera-behavior old → new | 52.1 → 126.0 | 162.6 → 162.6 | 0.296 → 0.154 | 0.002 → 0.000 |
| hero legacy hand-tuned | 90.3 | 205.6 | 0.161 | 0.012 |

The staging work landed: subjects are grounded on a floor with contact shadows
against a cyclorama instead of floating in a dark void, and materials read as
their authored substance. The legacy hand-tuned hero still wins on specular
headroom (p99 205.6 vs 162.6; 0.012 % vs 0.000 % near-white), so the automatic
path has closed the gap on staging and grounding but not on highlight range.

## 2. P0 — blocking

- [x] **Move the LTC lookup tables from shader constants into a uniform block.**
      *(Done. The original prescription below said "textures"; that was wrong —
      `downlevel_defaults()` caps sampled textures AND samplers at 16 per
      fragment stage on every backend, not only WebGL2, and the shaders already
      use exactly 16 of each. A uniform block costs neither. See the V3D
      measurements in section 6.)*
      `src/render/area_ltc_tables.wgsl` declares two `const array<vec4<f32>, 256>`
      tables, and `src/render/area_ltc.wgsl:53-64` indexes them eight times with
      runtime indices. Hardware without indexed constant-register access must
      expand each read into a select chain over all 256 entries. Measured with
      `V3D_DEBUG=shaderdb`, one fragment shader out of 39 compiles to
      `22,518 inst, 2 threads, 7 loops, 15,088 uniforms, 64 max-temps,
      200:599 spills:fills, 639 nops`; the median of the other 38 is 74
      instructions. The fix is two 16x16 RGBA float textures with a linear
      sampler, replacing the software `ltc_bilinear_sample` with
      `textureSampleLevel`. This is how the upstream reference
      (`selfshadow/ltc_code`) ships the tables. The shader runs on every
      backend, so the 2-thread occupancy drop and heavy spilling are not
      V3D-specific.
- [ ] **Explain and fix the `photo render --gpu` vs `recipe render --gpu` gap.**
      Identical intent, asset, and backend: 2400 s timeout versus 121 s. The
      photo path additionally runs `render_photographic_final` at
      `PhotographicTransportQuality::Final` plus an extra AOV capture and focus
      pass (`src/bin/scena/photo.rs:407-423`). Measure before designing a fix;
      this may share a root cause with the SSR item below.
- [ ] **Stop SSR from disabling CPU parallel rasterization.**
      `src/scene_host/photographic_surroundings.rs:195` enables SSR when
      `reflection_strength > 0.035`, i.e. `reflective_fraction > 0.194` —
      roughly one reflective material in five.
      `src/render/cpu_render/parallel_policy.rs:24` returns `false` whenever
      `screen_space_reflections.is_some()`, so the rasterizer drops to one
      core. Observed: 92 % of a single core for >25 min without completing on a
      4-core host. The subjects that most need reflections get the slowest path.

## 3. P1 — verification integrity

- [ ] **Subject measurement appears to count the generated surroundings as
      subject.** `visible_pixel_coverage_available` reports
      `foreground_pixels == region_pixels` (`foreground_fraction: 1.0`) on the
      valve manifold, which then fails `subject_clipped_by_frame` even though
      the rendered subject sits fully inside the frame with margins on all
      sides. Before this work the background was empty, so foreground detection
      was trivial; the new floor and cyclorama fill the frame with lit geometry.
      At least one of the two gate failures in section 1 is therefore likely
      spurious.
- [ ] **Relax the GPU gate on the exact subject-mask composition check.**
      `src/scene_host/composition/subject.rs:216` skips with
      `SkippedNoBackendSupport` whenever `backend != Backend::Headless`, so
      every GPU verification runs with its most precise check disabled. GPU
      semantic AOV capture now works (section 7), so this gate can be revisited.
- [x] **Stop reporting synthetic focus values as `resolved`.** *(Done: the
      fallback now reports `unresolved` with the reason. A test that asserted a
      focus distance always exists was asserting the fabricated value; it now
      requires a distance only when the report says `resolved`.)*
      `src/bin/scena/photo.rs:1161-1162` hardcodes `visible_pixel_count: 1` and
      `confidence: 0.65` on the fallback path and still calls
      `FocusReportV1::resolved`. When the AOV-measured path is unavailable the
      report should be `unresolved`, not a resolution that was never measured.
- [ ] **Add a test that renders a metal subject and asserts it is not flat.**
      `camera_behavior_shaded_candidate_scoring_rejects_black_silhouette_and_flat_gray_metal`
      feeds hand-written synthetic observations into the scorer and renders
      nothing. It proves the ranking function discriminates, not that the
      renderer can produce non-flat metal.
- [ ] **Make the photorealism doctor rules behavioural.**
      `crates/xtask/src/app/doctor_render/material_reflection.rs` is
      `require_contains` source-substring pinning. A green doctor run attests
      that contract text is present, not that output is photographic.
- [ ] **Commit `src/bin/scena/photo.rs`.** It is untracked (`??`), so the whole
      camera-behavior feature is invisible to `git diff` — which is why the
      GPU regression in section 7 could not be seen in review.

## 4. P2 — staging and quality

- [ ] **Raise the generated cyclorama, do not widen it.** Its edge shows as a
      hard curved seam in the top-left of both `colored_travel_mug` and the demo
      hero.
      *Diagnosed, and the obvious fix does not work.* Deriving `extent_m` from
      the camera frustum instead of `max(radius * 5, camera_distance * 1.35, ...)`
      was tried and measured: the seam remained. `cyclorama_geometry` sweeps a
      90-degree curve whose top edge sits `curve_radius = extent * 0.18` above the
      support, so widening the backdrop raises its top edge at only a fifth of
      the rate and it stays in frame at any width. The sweep must instead rise
      far enough to clear the frustum's vertical extent at the backdrop's
      distance, which means changing the curve height rather than `extent_m`.
      Verify by rendering a tall narrow subject and asserting the top-left
      corner holds no hard gradient, not by eye: the environment capture adds
      real gradient to the backdrop, so a naive corner-gradient metric compares
      environments rather than seams.
- [ ] **Stop crushing the backdrop to near-black.** Subject metering reaches its
      target while the surroundings stay underexposed, so composites read dim
      (`mean_luminance 0.263` on the valve, `0.411` on the hero). Consider
      metering or grading the surround relative to the subject.
- [x] **Use a real environment for image-based lighting.** *(Done: derives the
      bundled 27 KiB studio capture. Valve manifold p99 luminance 162.6 -> 203.6,
      near-white 0.005% -> 0.063%. Gated by `subject_specular_headroom_srgb8`,
      verified to reject the flat fixture at 16.86 against a 24 minimum.)*
      `apply_photographic_lighting` installs `assets.default_environment()`,
      which resolves to `EnvironmentDesc::neutral_studio()` with
      `source_kind: BundledPreviewFixture`. Its generated cubemap is 25 lines
      with one constant radiance per cube face, and the fixture source states
      `kind = cpu-preview-environment` / `not HDR input and not IBL proof`. A
      real 1K studio HDRI is already bundled at
      `tests/assets/environment/polyhaven/studio_small_03_1k.hdr` and referenced
      by `src/assets/environment_preset.rs:35`, but the camera-behavior path
      never selects it. Rotating a six-flat-colour cubemap changes which flat
      colour a highlight sees, not whether there is structure to reflect.
- [x] **Fix the environment ordering hazard.** *(Done: the host records
      `generated_environment` and answers through `has_authored_environment()`,
      so a derived environment no longer reads as an authored one. Confirmed the
      ordering test fails against the old predicate.)*
      `photographic_surroundings.rs:80` computes
      `preserved_authored_environment = self.renderer.environment().is_some()`,
      conflating "the user authored an environment" with "one exists". Because
      `apply_photographic_lighting` installs the default environment, any
      ordering where surroundings runs after lighting silently suppresses both
      the cyclorama and the derived background. Not triggered by recipes that
      declare no `scene.environment`, but fragile.
- [ ] **Fix the demo hero.** It fails its own gate at `fit_fraction 0.644`
      against a `0.65` minimum, with `center_offset_fraction [0.158, 0.014]`
      against a `0.16` maximum. Separately, `evidence/demo-hero/README.md` pins
      SHA-256 `915e9e36…` but a re-render produces `7352e341…` (14.03 % of
      pixels differ, max Chebyshev 111), so the committed proof does not
      reproduce.
- [ ] **Document or tighten the `fill_width_fraction` waiver.** Measured values
      of 0.278–0.528 pass against a published minimum of 0.65 because
      `width_fill_target_is_actionable` (`photo.rs:643`) waives the width check
      for subjects too tall to fill 65 % of width without exceeding max fit.
      Principled, but currently undocumented.

### Subject clipping and gate convergence (partial)

`clipped_fraction` came from the union of per-draw projected AABBs clamped to
the frame. That union is conservative: for a 33-part assembly of rotated
cylinders it bounds a volume much larger than the silhouette, so it reported
clipping for subjects visibly inside the frame. Verified on the valve manifold,
whose bbox overflowed the bottom edge by 35.6 px while no rendered pixel did.

It is now gated on the semantic mask reaching a frame edge. The false positive
is gone: the valve no longer reports `subject_clipped_by_frame` on its first
candidate. The retry loop also returns the least-bad attempt with its own frame
instead of whichever attempt ran last.

**The valve manifold still does not pass**, and the remaining reason is real
rather than a measurement artifact. Its retry trail at 640x420:

| candidate | adjustment | fill_width | luminance | codes |
|---|---|---|---|---|
| 1 | initial composition | 0.477 | 174.8 | fill, luminance |
| 2 | camera composition | 0.612 | 155.5 | fill, luminance, clipped |
| 3-5 | exposure delta | 0.612 | 122.3 -> 95.5 | fill, clipped |

Exposure converges correctly. Composition does not: the corrector zooms once to
0.612, still under the 0.65 minimum, and any further zoom makes the subject
genuinely touch a frame edge, so clipping then fires legitimately.

`width_fill_target_is_actionable` (`src/bin/scena/photo.rs`) judges the width
target reachable from `fill_width / fill_fraction * max_fit`, which for this
subject predicts 0.675 and so demands a width the frame cannot hold. For a wide
subject in a landscape frame the two constraints are simply incompatible, and
the heuristic does not know that. Deciding whether the gate should widen its
fill band by aspect, or the corrector should trade fill against clipping
explicitly, needs a design call rather than another threshold nudge.

### Split-sum BRDF: measured, and the table is worth binding

`PreparedEnvironmentCubemap::brdf_lut` is baked on every prepare, uploaded to
the GPU, and never bound. The shader calls the analytic `split_sum_brdf_approx`
instead, because binding the table needs a texture unit and the fragment stage
already uses all 16 that `downlevel_defaults()` allows on every backend. scena
pays to compute a table it discards.

Whether to bind it or delete it was an open question, so it was measured against
the production reference integrator (`integrate_brdf_lut_cell`, 4096 samples,
33x33 grid over `n_dot_v` 0.05..1.0, excluding the grazing singularity where the
integral itself is degenerate):

| term | max error | at |
|---|---|---|
| scale | **0.361** | `n_dot_v` 0.05, roughness 1.0 |
| bias | 0.084 | same |

That is the rough-metal grazing corner, which is where a brushed or blasted
metal product spends its silhouette, so the divergence is not confined to a
corner nobody renders. **The table is worth binding, not deleting.**

It should go through a uniform block rather than a texture. The texture route is
what ran out of units and left the table computed-and-discarded; a uniform block
costs no texture unit and no sampler, exactly as the LTC tables now do.

`analytic_split_sum_fit_diverges_where_the_baked_table_is_needed` pins the
measurement, so improving the analytic fit prompts revisiting the decision
rather than silently invalidating it. Binding the table is not yet implemented.

### SSR and CPU parallelism: the shape of the fix (not implemented)

`should_parallelize_cpu_geometry_pass` (`src/render/cpu_render/parallel_policy.rs`)
returns false whenever screen-space reflections are set, so the CPU rasterizer
drops to one core exactly for the reflective subjects that most need it.
`photographic_surroundings.rs` enables SSR at `reflective_fraction > 0.194`,
roughly one reflective material in five, so most product subjects hit it.

The reason it is disabled is real but narrower than the policy assumes. The
serial pass does two separable things when SSR is on:

1. fills a per-pixel `MaterialReflectionPixel` scratch while rasterizing, which
   is row-scoped and parallelises exactly like the colour and depth buffers, and
2. calls `screen_space_reflections::apply_material_linear` over the whole frame
   at the end, which genuinely needs every row.

Both are keyed off the same `screen_space_reflections.is_some()`, and
`draw_cpu_geometry_pass_serial` carries a `debug_assert!` stating that
row-scoped passes do not own the full scratch. Splitting them requires:

- a row-slice of the reflection scratch zipped into the existing
  `par_chunks_mut` chain in `parallel_pass.rs` alongside `linear_frame`,
  `depth_frame`, `frame` and `oit_scratch`;
- distinguishing "fill reflections" from "apply reflections" in the serial pass,
  since a row-scoped worker must do the first and not the second;
- moving `apply_material_linear` into the dispatcher, after the reduce;
- dropping only the SSR term from the policy. The
  `!has_physical_transmission` term must stay: transmission needs a whole-frame
  scene-colour snapshot mid-pass, which is a separate and larger change.

It must ship with a pixel-identity oracle - serial and parallel output byte-equal
with SSR enabled, above `CPU_PARALLEL_MIN_PIXELS` and `CPU_PARALLEL_MIN_PRIMITIVES` -
because a subtly wrong parallel path is a silent quality regression rather than a
visible failure. That, plus asserting the reported `CpuRowBandMetrics.workers`
exceeds one, is what makes the change safe to land.

## 5. Fixed during this investigation (uncommitted)

- [x] **Wire GPU semantic AOV capture through the camera-behavior path.**
      All five capture sites in `src/bin/scena/photo.rs` called the CPU-only
      `capture_semantic_aovs()`, which hard-rejects any backend other than
      `Headless`, with no `gpu` branch and no
      `set_semantic_aov_capture_enabled(true)` before prepare. Because
      `src/bin/scena/recipe.rs` shares those functions, this broke
      `recipe render --gpu` on any `photo.intent` recipe — a path that
      previously returned `ok:true`. Added
      `capture_camera_behavior_semantic_aovs(host, gpu)`, threaded `gpu`
      through `apply_camera_behavior_setup_with_plan`,
      `select_camera_behavior_shaded_candidate`,
      `render_camera_behavior_shaded_candidates`,
      `render_camera_behavior_candidates`, and
      `apply_visible_subject_physical_focus`, and enabled AOV resources before
      prepare in both callers. Verified: `recipe render --gpu --verify` went
      from exit 70 to exit 0 in 121 s.

## 6. V3D adapter — investigated, closed

`Renderer::headless_gpu` refuses the Pi's V3D adapter unless
`SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU` is set (`src/render/gpu/build.rs:20-23`).
The refusal is correct but its framing is wrong.

Measured with the escape hatch enabled: adapter enumeration, device creation,
and readback all succeed (`gpu_device: true`, `readback.status: supported`,
1 s). A 320x240 render then spends 1159 s in `vkCreateGraphicsPipelines` and
**does complete**, writing a valid PNG. It is pathological, not a hang, and no
kernel GPU reset occurs because no command buffer is ever submitted.

Symbolized stack (via `mesa-vulkan-drivers-dbgsym`, build-id `cd58235c…`):

```
v3dv_CreateGraphicsPipelines   v3dv_pipeline.c:3009
  pipeline_init                v3dv_pipeline.c:1939
    pipeline_compile_shader_variant v3dv_pipeline.c:1653
      v3d_compile              vir.c:2070
        v3d_attempt_compile    vir.c:1879
          v3d_nir_to_vir       nir_to_vir.c:5019
            v3d_register_allocate vir_register_allocate.c:1554
              ra_allocate / add_node_to_stack  register_allocate.c
```

`V3D_DEBUG=ra` shows 25 register-allocation failures across 9 programs, walking
Mesa's documented 13-entry fallback ladder (`vir.c:1923`).

**This is not a Mesa bug and no upstream report is warranted.** The ladder is
deliberate, documented, and bounded; V3DV's own internal shaders and scena's
vertex shader compile instantly with zero spills. The cost is entirely driven by
the LTC shader in section 2 — fix that and V3D becomes viable. Upgrading Mesa
does not help: no newer package exists for this host
(`Candidate: 25.0.7-2+rpt4`) and Mesa 26.1.0's v3dv entries are all features
(`robustness2`, `present_id`, `hdr_metadata`) with nothing on compile time.

### V3D after the LTC uniform-block fix (2026-07-27)

The LTC change (section 2) removed the register-allocation failures outright.
Measured on this Pi 5 with `V3D_DEBUG=shaderdb,ra`, same asset and resolution:

| | before | after |
|---|---|---|
| fragment instructions | 22,518 | 8,894 |
| uniforms | 15,088 | 2,707 |
| spills:fills | 200:599 | 199:481 |
| RA failures | 25, across all 13 strategies | **0** |
| 320x240 render | 1,159 s, no output | **10 s, `ok:true`** |

**Correctness is confirmed on hardware.** A 1280x840 valve-manifold render on V3D
matches the lavapipe render to within a mean Chebyshev distance of 0.911, with
only 574 pixels (0.05 %) differing by more than 16. The adapter renders correctly.

**It is roughly six times faster than software at rasterization.** A plain
1280x840 render of the valve manifold (no `photo.intent`, so no camera-behavior
loop) takes **1 s on V3D against 6 s on lavapipe**, producing the same image:
both 20.2 % non-black, luminance means 36.19 vs 35.95, 0.7 % of pixels differing
by more than 16. The GPU is doing real work and is clearly worth using.

An earlier revision of this document claimed V3D was "not faster than software",
from 205 s on V3D against 201 s on lavapipe. That comparison was invalid: both
were *camera-behavior* renders, where rasterization is not the bottleneck.

**The camera-behavior loop, not rasterization, dominates wall clock.** The same
asset and resolution costs 1 s as a plain render and 205 s through
`photo.intent: camera_behavior` on the same adapter, so about 99.5 % of that time
is spent outside rasterization. It scales with pixel count rather than draw
count: 320x240 takes 10 s and 1280x840 takes 205 s, a 20x rise for 14x the
pixels. The candidate loop only accounts for a handful of full-resolution
renders (`CAMERA_BEHAVIOR_MAX_ATTEMPTS = 6`, plus three 160x105 previews); the
remainder is per-attempt semantic AOV capture and readback plus
`measure_subject`'s per-pixel scans in Rust, both running over the whole frame.
This is the largest remaining performance defect in the feature, it is
backend-independent, and it supersedes the occupancy question below in priority.

**Occupancy is inherent, not a defect to chase.** V3D compiles the fragment
shader at 2-thread occupancy with 199:481 spills. Bisecting by deleting the five
optional material contributions, then also the LTC evaluation, cut it from 8,894
to 3,198 instructions and from 199:481 to 17:25 spills, yet occupancy stayed at
2 threads and max-temps stayed pinned at 64. Every shader in the same build shows
V3D granting 4 threads at 27 and 32 temps but only 2 at 35 and 39, so 4-thread
occupancy needs roughly 32 temps or fewer, unreachable for a full PBR, IBL,
shadow and tiled-light fragment shader. Recovering it would need per-feature
shader permutations, which a 6x speedup over software does not justify.

The five optional contributions (clearcoat, sheen, anisotropy, iridescence,
dispersion) account for 3,181 instructions and 56:285 of the spilling, but each
already early-returns on a zero factor, so that is static footprint rather than
executed cost for materials that do not use them.

**The refusal must stay, for a different reason than the one recorded.**
Removing it and measuring 40 identical headless renders found roughly 7% return
a frame containing only the clear colour. lavapipe and the CPU rasterizer were
clean over the same runs, so it is V3D-specific.

On a failing run scena's own state is indistinguishable from a success:

| probe | success | failure |
|---|---|---|
| draw batches | 33 | 33 |
| draws submitted | 33 | 33 |
| camera transform | identical | identical |
| `on_uncaptured_error` | silent | silent |
| `runtime_fault` | clean | clean |
| output | correct image | clear colour only |

The failing frames share one payload hash, so the render pass runs, clears, and
discards the geometry rather than corrupting it. The fault channel is genuinely
consulted on this path (`src/render/gpu/draw/native.rs:43`, alongside the two
surface paths), so nothing is being swallowed - the driver reports success and
returns an empty frame.

Ruled out by measurement: adapter selection instability (five runs, all
`V3D 7.1.10.2`), nondeterministic framing (camera byte-identical), missing
readback synchronisation (`map_async` then `poll(wait_indefinitely)` then
`recv`), scena skipping draws, and an unconsulted fault channel. Isolating the
remainder needs Mesa debug builds and GPU job dumps.

Silently returning an empty frame is worse than refusing the adapter, so the
guard stays until that is understood. Note that a distinct `AdapterRefused
{ reason }` variant means adding to the public `BuildError` enum
(`src/diagnostics.rs:61`), which is not `#[non_exhaustive]` and therefore a
breaking change.

### V3D intermittent empty frame — minimal reproduction

Reproduces with a single authored cube: no glTF import, no textures, no area
lights, one draw call.

```json
{
  "schema": "scena.scene_recipe.v1",
  "geometries": [{ "id": "box", "primitive": { "kind": "box", "size": [1.0, 1.0, 1.0] } }],
  "materials": [{ "id": "grey", "kind": "pbr_metallic_roughness", "base_color": "#c8c8c8" }],
  "nodes": [{ "id": "cube", "geometry": "box", "material": "grey" }],
  "capture": { "width": 320, "height": 240 }
}
```

```bash
SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU=1 \
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/broadcom_icd.json \
scena recipe render min.recipe.json --gpu --out out.png
```

Roughly one run in forty returns `ok:false` with `empty_frame`, and the frame
contains only the clear colour.

Measured behaviour:

| condition | failures |
|---|---|
| idle | 1/40 |
| under build/test load | 3/40 |
| `V3D_DEBUG=sync` | 3/20 |
| fixed `render.exposure_ev` (auto-exposure off) | 3/40 |
| 320x240 / 640x420 / 1280x840 | 1/30 / 0/30 / 1/30 |
| minimal authored cube | 1/40 |
| lavapipe, CPU rasterizer | 0/16 |

So it is V3D-specific, size-independent, load-sensitive, and unrelated to asset
import, material complexity, area lights, or auto-exposure. `V3D_DEBUG=sync`
making it *more* frequent points at a per-submission race rather than a
workload threshold.

Eliminated by measurement, each with the probe used:

- scena skipping draws — instrumented `encode_unlit_pass`: a failing run
  submitted the same 33 batches and 33 draws as a successful one.
- nondeterministic framing — capture descriptors byte-identical.
- nondeterministic adapter selection — five runs, all `V3D 7.1.10.2`.
- missing readback synchronisation — `map_async`, `poll(wait_indefinitely)`,
  `recv`, which is the correct wgpu pattern.
- a swallowed GPU fault — `on_uncaptured_error` never fires, and the headless
  path does consult `runtime_fault` (`src/render/gpu/draw/native.rs:43`).
- depth never cleared — the prepass clears unconditionally
  (`src/render/gpu/depth.rs:407`).
- auto-exposure applying a stale meter sample — failures persist with a fixed
  exposure.
- tile/binning memory exhaustion — no size dependence.

Nothing on scena's side differs between a success and a failure, so this is not
fixable in scena. Isolating it further needs a Mesa debug build and GPU job
dumps (`V3D_DEBUG=cl,clif`), and the reproduction above is small enough to file
upstream. scena already fails closed here: the `empty_frame` check rejects the
render rather than shipping a black image, which is why the refusal stays.

Guard improvements worth making anyway:

- [ ] Return a distinct `AdapterRefused { reason }` instead of
      `BuildError::RequestDevice`. No device is ever requested, and the current
      error reads as a driver failure — it cost two rounds of misdiagnosis
      during this investigation.
- [ ] Gate on driver version rather than the permanent name substring test
      `info.name.to_ascii_lowercase().contains("v3d")`
      (`src/render/gpu/build.rs:50-52`), so a fixed future V3DV is not banned
      forever.
- [ ] Narrow the scope: V3D handles device creation, limits negotiation, and
      readback correctly. Only pipeline compilation is affected.

## 7. Reproduction

```bash
cargo build --release --features agent --bin scena

# assets are generated from a Blender script; any single-root GLB works
scena photo render <asset.glb> --intent camera-behavior \
  --out target/photo-test/<name>.png \
  --report target/photo-test/<name>.report.json \
  --emit-recipe target/photo-test/<name>.recipe.json

VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/lvp_icd.json \
scena recipe render target/photo-test/<name>.recipe.json --gpu --verify \
  --out target/photo-test/<name>.recipe.png \
  > target/photo-test/<name>.recipe.render.json
```

V3D shader-compile evidence:

```bash
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/broadcom_icd.json \
SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU=1 \
V3D_DEBUG=shaderdb,ra \
scena recipe render <recipe.json> --gpu --out /tmp/v3d.png
```
