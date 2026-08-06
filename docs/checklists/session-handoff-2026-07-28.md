# scena — photographic render session handoff, 2026-07-28

Context for an external reviewer. Everything below is measured unless marked as
a hypothesis. Nothing is committed; all changes are in the working tree on
branch `demo/hero-scene` (HEAD `a1ef67f`).

## Hardware and lanes

- Local machine **is** the Raspberry Pi 5 (`raspberrypi`, aarch64, kernel
  6.18.33-v8-16k+). GPU: **V3D 7.1.10.2 / V3DV Mesa**, `/dev/dri/renderD128`.
  lavapipe also installed (`lvp_icd.json`).
- `scena-builder` is a Hetzner **CPU-only** box. Its `--gpu` silently uses
  lavapipe, so renders from it are software rasterization and are not GPU
  evidence.
- V3D is refused by default by `Renderer::headless_gpu`; the escape hatch is
  `SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU=1` (diagnostic only).
- Release build on the Pi takes 7–14 min and saturates the machine. Compiling
  belongs on the builder; only the render should run locally.

## Starting state

A large uncommitted remediation batch against
`docs/checklists/photorealistic-rendering-findings.md` was already in the tree
(W3 bind split-sum BRDF table, W4 SSR CPU parallelism, W7/W8 semantic-mask
coverage, W11 behavioural doctor rule, W14 cyclorama normals + backdrop sizing,
dominant-axis fit, capture-size precedence). A session crash had interrupted a
demo-hero render mid-flight.

The agreed render set is the four subjects scoped by the findings doc:
`dark_metal_speaker`, `colored_travel_mug`, `valve_manifold`, and the demo hero.
Assets live at `target/photo-test/assets/*.glb`; recipes at
`target/photo-test/g-*.recipe.json` and `evidence/demo-hero/hero.recipe.json`.

---

## Part 1 — Fixes made this session (in the working tree, uncommitted)

### 1.1 Exposure correction was unreachable — FIXED

`render_camera_behavior_candidates` (`src/bin/scena/photo.rs`) applied **at most
one** correction per attempt and always preferred composition:

```rust
if let Some(next) = corrected_composition_candidate(...) { composition = next; continue; }
let Some(next_ev) = corrected_exposure_ev(...) else { break };
```

`corrected_composition_candidate` returns `Some` whenever fit is out of band. For
a subject whose fill target is unreachable (aspect not the frame's, so more zoom
clips), that is every attempt, so `corrected_exposure_ev` was **never called** in
any of the 6 attempts, and the run then reported an underexposure it had never
tried to correct.

Fix: compute both corrections, apply both, `break` only when neither yields.
Measured on the demo hero: failure codes went from
`[subject_fill_below_min, subject_luminance_below_min]` to
`[subject_fill_below_min]`; subject mean luminance **66.22 → 88.25** (band
80–100).

### 1.2 Backdrop sized for a camera that no longer existed — FIXED

`apply_photographic_surroundings` runs at setup, *before* the candidate loop
picks the camera; the loop then re-frames substantially and the backdrop kept its
original size, so its edge entered the frame (a 191×28 px wedge of void in the
demo hero's top-left corner).

Fix: `resized_photographic_surroundings` re-runs remove+apply on each re-frame in
the acceptance loop; `SelectedCapture` now carries the staging report that
produced the returned frame, so the disclosed staging matches the delivered
image. Measured: largest vertical luma jump in the top 120 rows **102.7 → 1.7**.

### 1.3 `surroundings_extent` modelled only horizontal coverage — FIXED

The solve used `max(tan(half_v), tan(half_h))` as a single half-width. That
describes an unpitched camera aimed along the wall normal at the subject centre,
against a wall of unbounded height — none of which hold. The camera-behavior loop
pitches down and aims off-centre, and `cyclorama_geometry` builds a wall only
`1.8 * extent` tall.

Fix: project the frustum's four corner rays onto the wall plane and require the
wall to contain every hit, across and up, solved by fixed-point iteration
(`extent` appears on both sides via the wall's depth). New `BackdropCamera` /
`BackdropPlane` types. Tests: a sweep over focal length × aspect × radius ×
distance × pitch × aim-offset, plus a negative control proving the retired
width-only solve fails **260 of 1296** swept framings (so the coverage test is
not a tautology).

### 1.4 Photographic path never requested antialiasing — FIXED (partly)

`configure_camera_behavior_renderer` set tonemapper, auto-exposure, bloom and
SSAO but never touched AA or supersampling, so the flagship path inherited
`AntiAliasing::Fxaa` at supersample factor 1.

Fix: `renderer.set_anti_aliasing(AntiAliasing::Msaa4)`. Measured silhouette
transition width: Jul-27 baseline FXAA @1280 = 2.00 px; today MSAA4 @2560 = 3.00
px = **1.50 px at 1280-equivalent**, with a continuous coverage distribution (not
clustered at 0.25/0.50/0.75, so not quantisation-limited).

Supersampling is **not** enabled — see bug 2.2 and the resolution in Part 5.

### 1.5 The screen-space "reflection" pass — DISABLED in the photographic path

`src/render/gpu/post/ssr.wgsl` does not compute reflections. It takes a
horizontal scanline at `config.z * height` (hardcoded to **0.48** by
`apply_photographic_surroundings`), and for every pixel below it blends in the
pixel **mirrored vertically about that line**:

```wgsl
let horizon_y  = clamp(post.config.z * height, 1.0, height - 2.0);
if f32(coord.y) < horizon_y { return source; }
let floor_mask = clamp(1.0 - luma(source.rgb), 0.0, 1.0);   // "is this pixel dark?"
let mirrored_y = i32(clamp(2.0 * horizon_y - f32(coord.y), 0.0, height - 1.0));
return vec4<f32>(mix(source.rgb, reflected_sample(...), alpha), source.a);
```

No depth buffer, no normals, no ray march. Everything below the line — including
the subject's own lower body — receives an upside-down translucent copy of what
is above it. This is the "doubling" visible in every render, and it is
**pre-existing** (present in the Jul-27 baseline).

Confirmed by A/B: rendering with it disabled removes the ghost entirely, same
camera, same recipe.

Fix: the photographic path no longer enables it. The renderer setting remains,
now behind `SCENA_DEBUG_ENABLE_MIRROR_SSR=1`. **A real SSR needs a depth-buffer
ray march and has not been written.**

### 1.6 Debug instrumentation added

- `SCENA_DEBUG_DISABLE_SSAO=1`
- `SCENA_DEBUG_ENABLE_MIRROR_SSR=1`
- `SCENA_DEBUG_LOG_STAGING=1` — viewport, extent, support class, SSR config, and
  the computed mirror row in pixels
- `SCENA_DEBUG_LOG_ENVIRONMENT=1` — per-mip cubemap statistics: size, mean/peak
  radiance, and mean absolute neighbour delta (the quantity that becomes
  high-frequency detail on smooth metal)

### 1.7 Files changed by this session

```
src/assets.rs                                |  test    (temporary 64 -> 256 experiment removed)
src/bin/scena/photo.rs                       | 243 +++-  (1.1, 1.2, 1.4)
src/bin/scena/recipe.rs                      |   1 +
src/render/gpu/output_shader.wgsl            | W3, 5.1 (temporary specular-AA experiment removed)
src/render/gpu/output_shader_texture_2d.wgsl | W3, 5.1 (temporary specular-AA experiment removed)
src/render/pbr_brdf.wgsl                     | W3       (temporary specular-AA experiment removed)
src/render/prepare/environment.rs            |  67 +-    (1.6)
src/scene_host/photographic_surroundings.rs  | 690 +++-  (1.2, 1.3, 1.5, 1.6)
crates/xtask/.../photographic_output.rs      (untracked, doc-comment fix)
```

---

## Part 2 — Original unresolved report

This section preserves the observations and hypotheses handed to the expert.
Their resolved status is recorded in Part 5; do not treat the hypotheses below
as the current diagnosis.

### 2.1 THE MAIN ONE: fine lines on all smooth metal

Visible as fine contour/stripe lines across every smooth metal surface,
strongest on the flat baseplate and the chrome pipe. The user describes the
output as "not professional" and "not high resolution" primarily because of this.

**Measured facts:**

High-frequency energy (RMS of the residual after a 9 px horizontal box smooth)
scales inversely with material roughness:

| surface            | roughness | HF RMS |
|--------------------|-----------|--------|
| chrome pipe        | 0.055     | 16.25  |
| subject baseplate  | 0.33      | 6.09   |
| generated floor    | ~0.8      | 0.34   |
| cyclorama backdrop | diffuse   | 0.35   |

At **identical resolution and identical FXAA**, only the environment differing:

| render                                | res      | AA   | HF RMS |
|---------------------------------------|----------|------|--------|
| Jul-27 19:36 baseline, old environment | 1280×840 | FXAA | 2.47   |
| today 16:24, new environment           | 1280×840 | FXAA | 12.31  |

Note the timing: commit `8f01319 "Derive a real captured environment for
camera-behavior renders"` landed 2026-07-28 02:08, i.e. **after** the clean
19:36 baseline. But see below — the correlation did not survive testing.

**FFT of the plate region:** the dominant components are low-frequency horizontal
(periods 61–183 px, the broad arcs). The visually offensive signal is **vertical
and chirped** — the stripe period shrinks with depth into the frame. A vertical
luma profile down a column oscillates between ~94 and ~137 (amplitude ~40 levels)
with a period falling from ~5 rows to ~3 rows.

**Three hypotheses tested, all falsified:**

| hypothesis | change | plate | pipe |
|---|---|---|---|
| normal jitter → geometric specular AA (`dpdx`/`dpdy` normal variance folded into roughness) | `pbr_brdf.wgsl` + both output shaders | −10% | −2% |
| cubemap texel banding → resolution 64 → 256 | `src/assets.rs` | −12% | −2% |
| W3's 32×32 BRDF table → revert to `split_sum_brdf_approx` | shader A/B | +4% | +2% |

The cubemap change did exactly what it promised **at the data level** — per-mip
neighbour delta 0.0735 → 0.0190, a 3.9× reduction — and the rendered result
barely moved. So the lines are not the environment's texel structure.

Both temporary implementations were removed before commit. The bundled 128x64
HDR again uses its explicit 64-face bake, and the environment IBL again uses the
authored material roughness without derivative-based widening.

Also useful: the prefilter chain's relative neighbour delta **rises** as mips get
coarser (0.074 at mip 0, 0.130 at mip 1, 0.163 at mip 2). Pushing a mirror to a
blurrier mip therefore makes the step between texels *larger*. This is why the
specular-AA approach cannot work here and is worth knowing before anyone tries it
again.

**Ruled out:** textures (the asset has **zero** images/textures — five plain
metallic-roughness materials), SSR, SSAO (identical with both disabled), MSAA
(the 1280/FXAA comparison above has no MSAA in either render), edge
antialiasing (silhouettes measure clean, see 1.4).

**Current leading hypothesis, untested:** geometric aliasing of the mesh itself.
The asset is 33 lathe-style parts (`Cylinder`/`Torus`/`Sphere`, 1,600–1,800
vertices each, 32,256 triangles total) — heavily tessellated curved surfaces
whose facet edges alias under perspective. The chirped vertical frequency fits
undersampling of a periodic 3D structure. Vertex normals carry a quantised
±0.002 wobble.

If that is right, the fix is **supersampling**, which is broken — see 2.2.

### 2.2 Supersampling breaks the render on HeadlessGpu

`renderer.set_supersample_factor(2)` passes `validate_supersample_target`
(2560×1680 → 5120×3360 = 17.2 MP, well inside the 128 MP / 16384 px caps) and
then the render fails with:

```
Render: GPU resources for HeadlessGpu were not prepared
```

Reproduces on **lavapipe as well as V3D**, so it is not an adapter limitation.
Reproduces at 1280×840 as well as 2560×1680, so it is not a size limit. This
blocks the most likely fix for 2.1.

### 2.3 Explicit MSAA above the device maximum is a hard failure

`set_anti_aliasing(AntiAliasing::Msaa8)` fails the render outright:

```
Prepare: backend HeadlessGpu supports at most 4 samples, but explicit prepare requested 8
```

It does not degrade. Arguably it should, or the caller needs a way to ask.

### 2.4 The capability report overstates the hardware

`renderer_sample_counts` (`src/diagnostics/capabilities/sample_counts.rs`) is a
`const fn` of the **backend enum**:

```rust
Backend::HeadlessGpu | Backend::NativeSurface => [1, 4, 8],
```

It never queries the device. V3D reports `render_sample_counts: [1, 4, 8]` and
`explicit_msaa: supported` while actually capping at 4. The real query,
`max_supported_sample_count` (`src/render/gpu/msaa.rs`), exists but is
`pub(super)`, so neither the CLI nor a host can reach it. This is what caused
2.3.

### 2.5 CRITICAL: the acceptance gate passes frames with no subject in them

A `dark_metal_speaker` render at 2560×1680 came back containing only floor,
backdrop and a contact shadow — **no subject** — and the gate reported
`ok=true` with **zero failure reasons**.

The semantic AOV pass reported `899,856` subject pixels at
`confidence: exact_opaque_semantic_aov`, while the colour pass drew nothing.
Every composition check reads the AOV; nothing cross-checks it against the colour
frame. Subject mean luminance measured 96.07 — inside the 80–100 band — because
the empty backdrop in the mask region happens to be mid-grey.

This undercuts the W7/W8 semantic-mask work: a mask is only trustworthy if it is
verified to agree with the pixels actually shipped. **Suggested fix: a check that
the AOV-reported subject region and the colour frame agree, e.g. that the masked
region's variance/edge content is non-degenerate.**

### 2.6 V3D drops draws far more often than documented

`CLAUDE.md` documents "roughly 7% of headless renders return a frame containing
only the clear colour". Observed today:

- **demo hero: 11 of 11 attempts blank** (3 distinct colours, luma 122–123),
  fully deterministic, at three different capture sizes (1280×840, 1600×1024,
  1800×1150) — so not resolution-dependent.
- `dark_metal_speaker`: 4 of 5 attempts blank or subject-missing.
- `colored_travel_mug`, `valve_manifold`: reliable.

The hero case being deterministic makes it much better debugging material than
the intermittent flake currently documented. The same hero recipe renders
correctly on lavapipe.

### 2.7 The demo hero cannot satisfy its own fill gate

`fit_fraction` 0.644 against a 0.65 minimum, with
`center_offset_fraction [0.158, 0.014]` against a 0.16 maximum. Pre-existing and
recorded in the findings doc. The subject's aspect is not the frame's, so any
further zoom clips. The dominant-axis change (already in the batch) did not
resolve it. Needs a design decision: re-aim the hero recipe, widen the band with
a stated rationale, or have the corrector trade fill against clipping explicitly.

### 2.8 Framing centres a different point than the gate measures

`frame_node_with_photo_candidate` centres `photographic_visual_center` — a
surface-area × √primitive-count weighted centroid of per-draw bounding-box
centres. The composition gate measures the **projected bounds** centre. In the
demo hero these differ by 0.158 of frame width: `introspection.framing` reports
`center_offset_fraction [0.0, 0.0]` while the composition check simultaneously
reports `[0.158, 0.014]`. Two "centres" in one report, 0.002 from failing.

---

## Part 3 — Questions for the expert

1. **2.1 is the priority.** Given the falsified hypotheses and the chirped
   vertical frequency, is geometric/specular aliasing of the tessellated meshes
   the right read? If so, what is the correct fix in a forward renderer that
   currently has neither working supersampling nor temporal accumulation?
2. **2.2** — why would `set_supersample_factor(2)` pass target validation and
   then leave `HeadlessGpu` without prepared resources, on both lavapipe and
   V3D?
3. **2.5** — what is the right shape for an AOV-vs-colour-frame agreement check
   that cannot itself be fooled?
4. Is the auto-staging architecture (generated cyclorama + derived environment +
   auto-exposure + screen-space effects) the right basis for a believable product
   still? The hand-tuned legacy hero still beats the automatic path on specular
   headroom: p99 205.6 vs 162.6, near-white 0.012% vs 0.000%.

## Part 4 — Reproduction

```bash
# render (local, V3D)
SCENA_ALLOW_UNSTABLE_V3D_HEADLESS_GPU=1 \
VK_ICD_FILENAMES=/usr/share/vulkan/icd.d/broadcom_icd.json \
SCENA_DEBUG_LOG_STAGING=1 SCENA_DEBUG_LOG_ENVIRONMENT=1 \
  ./target/release/scena recipe render \
  target/photo-test/g-valve_manifold.recipe.json --gpu --verify \
  --out out.png > out.render.json

# build (MUST be on the builder, not the Pi)
ssh scena-builder 'cd "$HOME/.cache/codex-worktrees/scena-photorealistic-rendering" \
  && env CARGO_TARGET_DIR="$HOME/.cache/codex-targets/scena-photorealistic-rendering" \
  cargo build --release --features agent --bin scena'
```

Artifacts from this session are under `target/photo-fixed/`:
`final-valve.png` (both fixes + 256 environment), `dbg-noboth.png` (SSR and SSAO
off), `ab-approx.png` (analytic BRDF fit), `dbg-nossr.png`, and the matching
`*.render.json` / `*.log` files.

---

## Part 5 — Expert resolution

### 5.1 The metal contours were generated material detail, not mesh aliasing

The geometric-aliasing hypothesis is false. The same coherent contours occur on
the valve's planar baseplate, where tessellated curvature cannot be the source.
History then pins the regression to commit `19e52f0`, which introduced
`PhotographicMicroSurface` after the clean Jul-27 baseline.

For every untextured PBR material, the photographic surface path generated two
deterministic world-space sine waves and used them to perturb both normal and
roughness. Perspective projects those coherent waves into the observed chirped
bands; low-roughness metal makes the result dominant.

Fix: unresolved generated micro-detail is now represented as its mean roughness
contribution (`strength * 0.175`) while the authored normal is preserved. It is
not sampled as resolvable world-space waves. On the same V3D valve render and
corresponding 60x40 planar plate interior, 9 px box high-pass RMS fell from
**10.020 to 0.356**, a **96.4% reduction**. The post-framing-change crop moves
from `x=560..619, y=620..659` to `x=551..610, y=552..591`; using the old
absolute coordinates would intersect the moved plate edge and is not a valid
comparison. Supersampling is therefore not required to fix this defect.

### 5.2 Supersampling prepared the wrong-size semantic target

`prepare_target()` correctly expands the beauty target for supersampling, but
the semantic AOV resources were also allocated at that expanded target.
`capture_semantic_aovs_gpu()` later requests the public capture dimensions, so
its strict prepared-resource check returned the generic
`GpuResourcesNotPrepared` error.

Fix: GPU preparation now receives two explicit targets: the supersampled beauty
target and the public-resolution semantic AOV target. The FR06 GPU parity test
now renders beauty at factor 2, captures AOVs at 96x72, and compares identity,
depth, and normals with CPU truth.

The final V3D `render.supersample:2` probe reaches verification, reports
`render_supersample_active` with `actual:2`, and captures its semantic AOV at
the public 1280x840 dimensions. It no longer returns
`GpuResourcesNotPrepared`. V3D dropped the beauty draws on that attempt, and
the separate agreement gate correctly rejected the frame; that is issue 2.6,
not a recurrence of the supersample resource bug.

### 5.3 Sample-count reports now distinguish unprobed from measured support

Static GPU capability construction no longer claims `[1, 4, 8]`. Until a device
exists it reports `[1, 0, 0]` and `explicit_msaa: error_if_required`. Native
renderer construction and surface recovery then intersect the actual adapter
format features for every required color format and `Depth32Float`, and publish
the measured counts.

An explicitly requested unsupported count remains a structured hard failure.
That is intentional: silently degrading a caller's explicit quality request
would make the result unverifiable. Callers can now inspect truthful live
capabilities before choosing 4x or 8x.

### 5.4 The color/AOV gate is a conservative stopgap, not an attestation

The immediate gate compares color discontinuity across the semantic silhouette.
It rejects the known flat-beauty/stale-AOV failure and reports confidence
`heuristic_local_semantic_boundary`; it does **not** claim exact agreement.
A coincident background edge or surviving contact shadow can still fool any
post-hoc image heuristic.

The non-foolable design is a same-pass witness: write semantic draw identity or
at least subject coverage into a second attachment in the beauty render pass,
then validate that attachment before accepting the frame. A separate AOV pass
cannot prove that the beauty pass completed, regardless of how accurately that
separate pass is measured. Until that MRT witness exists, V3D output must remain
diagnostic-only and the heuristic failure must remain blocking.

On final hardware, this stopgap rejected both measured V3D failure shapes:

- the blank demo hero: boundary delta **0.53** across 2,976 samples while the
  separate AOV claimed 227,897 subject pixels;
- a supersampled blank valve that retained a contact shadow: boundary delta
  **0.47** across 4,108 samples while the AOV claimed 169,031 subject pixels.

Both exit 1 with `subject_color_frame_agreement_below_min`. This is strong
evidence for the current failures, but it does not upgrade the heuristic into
the same-pass attestation described above.

### 5.5 Framing and the demo-hero fill conflict are resolved at the contract

The camera no longer recentres on the primitive-weighted
`photographic_visual_center`; it frames the same world bounds whose projected
centre the gate measures. An asymmetric seven-part regression fixture failed
with a horizontal offset of **0.233** before the change and now remains within
**0.01**.

The fill corrector and gate now use the subject's limiting projected axis rather
than requiring both width and height to independently match a frame-shaped
target. Clipping remains a separate upper guard. This removes the unsatisfiable
aspect-ratio constraint instead of widening the acceptance band.

### 5.6 Auto-staging is a baseline, not the final art direction

Generated cyclorama, physically based environment lighting, exposure, and
composition correction are a sound fallback for an unattended product still.
Authored staging must remain authoritative, and screen-space effects must only
be enabled when they implement their named physical signal. The current mirror
pass is not SSR and stays disabled.

The automatic path still needs a highlight-shape/headroom objective. Mean
exposure alone can place the subject in band while flattening metal; the legacy
hero's p99 and near-white measurements prove that gap. The next quality metric
should judge controlled specular highlight area and headroom, not simply raise
exposure or add post effects.

### 5.7 Raspberry Pi V3D proof

The initial four-subject proof binary was cross-built on `scena-builder`
against the Pi glibc 2.41 sysroot before the two failed experiments were
removed. SHA-256:
`a2160c75551cbfc3067a0cfeca1f604187045f4fb0bd8e34d650f7f16c2c22fa`;
its highest required symbol is `GLIBC_2.39`. The local adapter was
`V3D 7.1.10.2`, driver `V3DV Mesa`, backend `Vulkan`.

- Live measured capability report: color and `Depth32Float` each support
  `[1,4]`; the public matrices are `[1,4,0]`, not the old `[1,4,8]`.
- `target/photo-fixed/final-v3d-colored-travel-mug-1280.png`: complete subject,
  `ok:true`, boundary delta 32.62 over 1,481 samples.
- `target/photo-fixed/final-v3d-dark-metal-speaker-1280.png`: complete subject,
  `ok:true`, boundary delta 70.17 over 1,651 samples.
- `target/photo-fixed/final-v3d-valve-1280.png`: complete subject, no contour
  bands, no mirror ghost, `ok:true`, boundary delta 28.41 over 4,108 samples.
- `target/photo-fixed/final-v3d-demo-hero-1280.png`: blank V3D beauty frame,
  correctly rejected with `subject_color_frame_agreement_below_min`.
- `target/photo-fixed/final-v3d-valve-ss2.png`: supersample lifecycle completes
  and public-size AOV capture runs; V3D beauty draw drop is correctly rejected.

The V3D refusal therefore remains correct. The renderer and its verification
now report the failure honestly; the underlying driver draw-drop defect is not
fixed in scena.

### 5.8 Failed line-removal experiments were not retained

The derivative-based specular-AA path and the bundled-environment 64-to-256
cubemap increase were controls for diagnoses that the measurements falsified.
They did not independently justify their rendering cost or behavior:

- derivative widening changed only the primary environment lobe, not direct
  lights, clearcoat, or anisotropy, and measured only 2% on the chrome pipe;
- baking a 256-face cubemap from the bundled 128x64 HDR adds no source detail
  and processes 16 times as many face texels as its explicit 64-face bake.

Both were removed. The split-sum BRDF table and the matched CPU/GPU
`strength * 0.175` unresolved-micro-roughness fix remain.

The exact cleaned tree was then cross-built against the same sysroot. Binary
SHA-256:
`55c7b2edc4d5263793ad49e3e1819c6d6f7a3b70656fcf5f1598873318d6bdb3`;
its highest required symbol remains `GLIBC_2.39`. The cleanup-specific hardware
proof is
`target/photo-fixed/final-v3d-valve-cleaned-1280.png`: the V3D log confirms a
64-face environment bake, verification reports `ok:true`, boundary delta 33.32
over 4,108 samples, and the same 60x40 plate crop measures high-pass RMS 0.370.
That is effectively unchanged from 0.356 with the experiments and remains a
96.3% reduction from 10.020 with the generated waves.
