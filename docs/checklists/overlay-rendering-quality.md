# Overlay rendering quality — smooth text + smooth lines

## Purpose

Two visible quality defects remain in overlay primitives, both confirmed at
native resolution on the GPU showcase renders:

1. **Pixelated text.** Labels default to the embedded **5×7 bitmap font**, so any
   label without an explicit TTF renders blocky (e.g. `BODY`, `PUMP 2 FAULT`).
   The glyph-atlas work fixed *erosion*, not the bitmap's inherent low resolution.
2. **Jagged / non-straight lines.** Dimension lines and callout leaders render as
   hard 1px aliased strokes — stair-stepped and segmented (e.g. the flashlight
   `LENGTH: 0.595 M` line), not smooth straight lines.

And a verifier gap: both passed `quality.ok` — the verifier catches *eroded*
text but not *pixelated* text or *jagged* lines. The fixes must also close that,
or we can't gate them.

Goal: overlay text and lines are smooth and high-quality at native resolution on
**both** CPU and GPU, and the quality verifier fails the current pixelated/jagged
output and passes the fixed output.

Status legend: `[ ]` todo · `[x]` done.

## Issue 1 — de-pixelate text

- **A — embed a TrueType font as the default AND ONLY built-in font (immediate).**
  The atlas TTF path is already proven smooth (DejaVuSans via the `fonts` section
  rendered crisp AA). **Embed** one small, license-clear font (OFL/Apache; subset
	  to basic Latin; `include_bytes!`, so it's always available with no runtime asset
	  or path-policy dependency; mind the <10 MiB publish gate) and make it the
	  default. **REMOVE the 5×7 bitmap font entirely** — no fallback, no opt-in "pixel
	  style." Keep `LabelDesc::new` as the default embedded-TrueType constructor;
	  delete `LabelDesc::bitmap`, the 5×7 glyph table, and the bitmap atlas
	  source; migrate any internal users (debug overlays, etc.) to the embedded TTF;
  remove/convert bitmap tests/fixtures and add a doctor guard that the bitmap path
  is gone.
- **B — SDF/MSDF glyph atlas (the deferred Option B, now triggered).** A
  coverage atlas rasterised at small `size_px` (15–18) still softens/limits
  detail; SDF stays razor-sharp at any size. The trigger we named for B was
  exactly this — text quality at small label sizes. Sample + threshold in-shader
  with a matching CPU threshold; the atlas/instanced-quad/parity foundation from
  the glyph-atlas work is reused (B is a content + fragment-sample swap, not a
  rewrite).
- Complementary: encourage adequate label `size_px` / capture resolution in the
  skill; tiny text has limited detail regardless of font tech.
- Decision (see open questions): A immediately, then B — or B now, given repeated
  text-quality dissatisfaction and that the foundation is already built.

## Issue 2 — smooth, straight lines

- Anti-alias dimension lines and callout leaders: render each as an AA'd quad /
  capsule segment with coverage falloff across the stroke width (the same
  coverage/AA discipline used for glyphs), instead of a hard 1px line.
- Ensure each dimension line is a **single continuous stroke**, not disjoint
  stepped segments (the native crop shows step offsets between segments).
- CPU and GPU must match within tolerance (engineered parity, like the labels);
  likely the existing stroke pass (`src/render/gpu/strokes.rs`) is the right home.

## Verifier extension (so these can be gated)

- **Text smoothness / AA-presence:** enforce `min_intermediate_edge_fraction`
  (AA edge ratio) in the default profiles so **bitmap/pixelated text fails** —
  not just eroded text. Today the showcase passed with blocky bitmap text.
- **Line quality:** add a line-straightness / edge-AA check over dimension and
  leader regions so a stair-stepped/aliased line **fails**.
- Both checks must FAIL on the current pixelated/jagged output and PASS after the
  fix (known-bad + known-good fixtures, exact reason codes, native resolution).

## Acceptance (non-negotiable)

- [x] At native resolution, on **both CPU and GPU**: label text is smooth (not
      blocky/pixel-stepped) at the label sizes used in the showcase scenes.
- [x] At native resolution, on both backends: dimension lines and callout leaders
      are smooth and straight — no stair-stepping, no segment offsets.
- [x] The verifier **fails** the current bitmap-text + jagged-line output and
      **passes** the fixed output (text AA-presence check + line-quality check),
      with exact reason codes; CPU/GPU match within tolerance over the full region.

## Build order

- [x] 1. Embed a small clean TTF as the default+only built-in font; **delete the
      5×7 bitmap font** (`LabelDesc::bitmap`, glyph table, bitmap atlas source);
      migrate internal users; remove bitmap tests/fixtures; doctor guard "no bitmap path."
- [x] 2. Verifier: enforce text AA-presence so bitmap/pixelated text fails;
      known-bad (bitmap) + known-good (TTF) fixtures.
- [x] 3. Anti-alias dimension/leader line rendering (CPU + GPU, parity, single
      continuous stroke).
- [x] 4. Verifier: add line straightness/AA check; known-bad (aliased) + known-good.
- [ ] 5. SDF/MSDF glyph atlas for small-size crispness (Option B), reusing the
      existing atlas/parity foundation.
- [ ] 6. Re-render the showcase set; confirm smooth text + smooth lines at native
      res on both backends; verifier green.

## Open questions / decision

1. TTF-default first then SDF, or SDF now? (Foundation is built; B is a
   content/shader swap. Repeated text-quality dissatisfaction argues for doing
   both, TTF first for the immediate win.)
2. Line AA: reuse the stroke instanced-quad pass, or a dedicated overlay-line
   pass? Capsule-segment coverage vs distance-to-segment in-shader?
3. Which bundled font (DejaVu Sans / Liberation / Inter / Noto), and subset size
   vs the publish gate?

## Gates

- [ ] `cargo fmt --check` · `clippy -D warnings` (default + `scene-host,inspection`)
      · `cargo test` (×2) · `doctor --full` · `doc -D warnings`.
- [ ] `cargo publish --dry-run` compressed size (<10 MiB) re-check — a bundled
      font adds weight; subset to basic Latin if needed.
