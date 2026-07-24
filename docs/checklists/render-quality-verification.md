# Render-quality verification (`scena.render_quality.v1`) — build checklist

## Purpose

Add a render-quality dimension to recipe verification so a caller can prove a
render is not broken or low-quality across the whole frame — not just that
content is present and correctly coloured. Correctness checks (`expect_color`,
introspection, `ok:true`) do not measure exposure, contrast, edge/text
integrity, noise, or reference fidelity. This closes that gap so an automated
caller can self-verify visual quality, and it permanently guards the label
anti-aliasing bug from returning.

Status legend: `[ ]` todo · `[x]` done.

## Tiers

- T1 — render integrity (objective). Implement.
- T2 — reference fidelity (objective, where a golden exists). Implement.
- T3 — aesthetic (VLM / human). Advisory artifacts/warnings only; never a gate.

## Profiles (versioned structs in code; schema exposes stable IDs)

- `product` — exposure, contrast, edge.
- `documentation` — text + exposure strict; no material-contrast requirement.
- `cad` — framing/text/line-contrast strict; material flatness allowed.
- `dashboard` — text/line/bar contrast strict; material quality irrelevant.
- `twin` — exposure/framing/labels/state-colour.

## Metrics (region-based, on the native-resolution capture)

The default region comes from the render-introspection content bbox, so exposure
and contrast are evaluated on the subject crop rather than the whole background.
Label-quality fixtures use explicit native-resolution label crops. Recipe-level
per-label projected bboxes are not exposed until label projection data carries
metrics rectangles; do not claim per-label recipe masking before that lands.

- Exposure — luminance percentiles (p01/p05/p50/p95/p99) + low/high clip
  fractions on the subject mask.
- Contrast/flatness — p95−p05, stddev, Sobel edge energy; profile-specific.
- Noise/fireflies — local median / Laplacian outlier fraction (isolated
  outliers, not legitimate texture).
- Text/labels (label-aware) — expected bbox from `LabelDesc::metrics()`;
  fragmentation / ink-isolation; ink coverage in expected range; stroke
  continuity; legibility = diff vs a re-rasterised glyph mask (coverage ratio,
  connected components, IoU with ±1px shift tolerance, TrueType edge-AA ratio on
  **composited luminance**, not alpha). Generic edge energy is NOT a label
  discriminator.
- Reference — exact RGBA mean-difference, ΔE2000 (colour), and grayscale SSIM
  (structure) over exact-dimension PNG references. No perceptual-hash hard gate;
  never "nonblack>0" or "before≠after".
- Grounding is intentionally outside `scena.render_quality.v1` until recipe
  verification can consume a floor/receiver report and compute a contact-darkening
  band. Do not add an `expect_quality.grounding` knob without that computation.

## Always-on vs opt-in

- Always-on baseline (any `recipe render`): blank/empty, gross
  over/under-exposure, **severe** black-crush / blown-out subject, cropped/tiny.
  Egregious-only (high thresholds, profile-aware) so intentionally dark/moody
  renders do not false-fail.
- `expect_quality { profile, exposure, contrast, noise, text }` —
  opt-in, profile-scoped gates.
- `expect_reference [{ id, image, metric, mean_max }]` — opt-in, curated recipes
  with goldens only. A novel authored scene gets T1 objective checks only.

## Report

Add `quality` as a separate report under `SceneRecipeVerificationReportV1`
(alongside appearance/interaction), schema `scena.render_quality.v1`. Per-check:
`{ id, code, severity, region{kind,handle}, observed, threshold, fix_hint }`.
Every failed check carries a concrete, action-oriented `fix_hint`
(e.g. "increase capture resolution", "use TrueType label coverage path",
"add lights/environment", "change background"). Keep
the compact `ok` + reasons path as well.

## Determinism

Coarse checks (exposure/clipping/blank/huge-noise) may share thresholds.
Text/glyph/edge metrics need per-backend tolerances. The report carries
backend/capability metadata. CI hard-gates CPU/lavapipe; real GPU/WebGL2 use
backend-specific baselines or advisory thresholds until proven.

## Label AA renderer fix (correctness fix — ships independent of the metric suite)

Root cause: the old `LabelGlyphCell` path represented each glyph as a grid of
per-pixel rectangles. That representation caused blocky glyphs, discarded
coverage, and CPU/GPU divergence. The replacement is the glyph-atlas path from
`docs/checklists/label-glyph-rendering.md`: one sampled atlas quad per glyph,
manual 4-tap bilinear coverage sampling in CPU and GPU paths, and no per-cell
label primitive path.

- [x] Retire `LabelGlyphCell` / `glyph_cells()` and the per-cell label primitive
      path.
- [x] TrueType atlas entries preserve original per-pixel alpha; the old 5×7
      bitmap font path is removed by `overlay-rendering-quality.md`.
- [x] CPU: sample the atlas with the dedicated manual bilinear label sampler.
- [x] GPU: sample the same atlas with matching manual bilinear WGSL.
- [x] Keep renderer-owned AA coverage distinct from user colour alpha; the user
      label colour-alpha ≠ 1.0 fail-closed validation must be unaffected.

## Build order

- [x] 1. `render::quality` pixel-metrics module (luminance percentiles,
      subject/foreground mask, Sobel energy, local outlier/noise, connected
      components) + `RenderQualityReportV1` schema.
- [x] 2. Label quality fixtures: known-bad (saved native-res eroded-label
      capture + recipe + crop) fails with an exact reason code; known-good label passes.
- [x] 3. Label coverage AA renderer fix (CPU + GPU); both label tests pass.
- [x] 4. Wire `expect_quality` (labels + exposure first) into recipe
      verification and the separate report block.
- [x] 5. `expect_reference` PNG loading + exact-dimension reference metrics;
      promote colour conversion + ΔE2000 into reusable render-quality code; add
      validated grayscale SSIM.
- [x] 6. Deterministic good/bad calibration fixtures + doctor guards.

## Acceptance / discipline (non-negotiable)

- [x] All metrics computed on the native-resolution capture, never downscaled.
- [x] Every check has a known-bad AND known-good fixture and must FAIL on the
      bad with the exact reason code. (A prior label guard passed on broken
      output because it only checked for non-background pixels — do not repeat.)
- [x] Label fixture fails before the renderer fix and passes after; a live
      render of the same recipe passes; CPU and GPU both pass.
- [x] Profile defaults use egregious always-on thresholds plus deterministic
      good/bad fixtures so legitimate CAD/dashboard/product variety does not
      false-fail by default.
- [x] Validate the hand-rolled grayscale SSIM against a known reference pair
      before it gates anything.
- [x] Failed checks carry actionable fix hints.

## Calibration anchors (measured)

- Eroded vs clean label: ink-isolation 0.0355 vs 0.000; ink-coverage 0.072 vs
  0.189; generic edge energy 0.049 vs 0.039 (non-discriminating). Label
  fragmentation threshold ≈ 0.01.
- Pitch-black vs good render: frame/subject black-crush (low-clip) 0.82 vs
  ≤0.012. Always-on product/twin severe-black-crush gate is 0.80
  (egregious only).
- Exposure/contrast does NOT catch pixelation/aliasing — that relies on the
  edge-AA + resolution check.

## Gates

- [x] `cargo fmt --check` · `clippy --all-targets -D warnings` (default and
      `--features scene-host,inspection`) · `cargo test` (default and features) ·
      `cargo run -p xtask -- doctor --full` · `RUSTDOCFLAGS="-D warnings" cargo doc`.
- [x] No `cargo publish --dry-run` compressed-size re-check required: this slice
      adds no dependency, font, or asset.
