# Label glyph rendering — representation change (glyph atlas + sampled quad)

## Root cause (why patching the current path cannot win)

scena draws each glyph as a grid of per-pixel rectangles — `LabelGlyphCell`,
one quad per "on" glyph pixel (`src/render/prepare/labels.rs`), with coverage
folded into vertex alpha — instead of rasterising the glyph to sampled texture
data and drawing one quad per glyph. A glyph is a mosaic of tiny squares, not a
sampled shape. Every symptom is downstream of that one choice: blocky/low
ceiling, erosion + the threshold bug, CPU/GPU divergence, fragile AA. Adding
`coverage` to the cell refines the wrong representation.

Status legend: `[ ]` todo · `[x]` done.

## Goal

Represent each glyph as sampled texture data drawn as **one quad per glyph**,
sampled through a **dedicated label sampler with CPU/GPU parity**. Outcome: AA is
automatic, CPU and GPU match within tolerance, and the per-cell geometry cost is
gone.

## Decision: Option A first, SDF/MSDF later (two steps)

- **A — coverage-texture atlas (now).** Rasterise each glyph to grayscale
  coverage (reuse the fontdue coverage already produced in
  `src/scene/labels/font.rs`), pack into a per-(font,size) atlas, draw one quad
  per glyph. Fixes blockiness, erosion, CPU/GPU divergence, fragile AA, per-cell
  cost. Sufficient for single-frame / recipe / headless renders, where the
  label's pixel size is known at prepare time and the atlas is rasterised at that
  size.
- **B — SDF/MSDF (later).** Adds distance generation, in-shader thresholding, a
  matching CPU threshold, tuning, new fixtures, and likely a dependency/size
  review. Defer until the concrete trigger lands: a **live zoomable viewer**
  (where a coverage atlas re-rasters and goes briefly soft) or world-scaled text.
- Not wasted work: the atlas, dedicated label pass, instanced one-quad-per-glyph,
  CPU/GPU parity machinery, cache, and migration are shared with B; A→B later
  only swaps atlas content (coverage→distance) and the fragment sample.
- Note (measured): labels scale with camera distance (77px → 102px at 2× closer),
  so they are not fixed-pixel-size — which is precisely why B's payoff is
  interactive zoom, and why A keys the atlas on the actual rendered size.

## Design

- One glyph atlas (texture); a label is instanced quads (one per glyph) sampling
  the atlas; flat/unlit; user colour stays opaque; renderer-owned AA from the
  sampled coverage stays distinct from the hardened user colour-alpha ≠ 1.0
  fail-closed invariant.
- **CPU/GPU parity is engineered, NOT "by construction."** CPU
  (`TextureDesc::sample_bilinear`, `src/assets/texture.rs`) and GPU (WGSL
  `textureSample`, `src/render/gpu/output_shader.wgsl`) are not a shared sampler.
  To guarantee parity: a dedicated label-atlas sampler; **linear** coverage
  (not sRGB); clamp-to-edge + padded gutters; **manual 4-tap bilinear in both CPU
  and WGSL using the same texel-space formula**; compare with **tolerance**, never
  exact bytes.
- **Dedicated label pass, not the material pipeline** (the material path is tied
  to PBR/unlit bind groups and the WebGL2 texture-unit ceiling,
  `src/render/gpu/material_bindings.rs`). Model the GPU label pass on the stroke
  instanced-quad pass (`src/render/gpu/strokes.rs`, `strokes.wgsl`): static unit
  quad, one instance per glyph carrying corner basis + atlas UV rect + colour +
  depth; fragment samples coverage and applies label colour; depth-tested overlay,
  no lighting, no material bind group.
- Module split: `scene::labels` (descriptors, layout, font fingerprint, raster
  source) · `render::prepare::labels` (deterministic atlas packing and prepared
  glyph quads) · `render::cpu_labels` (CPU atlas sampling) ·
  `render::gpu::labels` (atlas texture, bind group, shader, pass).
- Option A atlas scope: per prepared scene; deterministic BTreeMap packing;
  rebuild on structural text/font/size change; no global LRU until needed.
- The old 5×7 bitmap font path is removed by
  `docs/checklists/overlay-rendering-quality.md`; `LabelDesc::new` now uses the
  embedded TrueType atlas source.

## Acceptance (non-negotiable)

- [x] CPU and GPU label renders are crisp at **native resolution** and **match
      each other within a defined tolerance over the ENTIRE label region
      (background pill + glyphs)** (max/mean channel delta or SSIM/ΔE) — verified
      at native res on lavapipe; not exact-byte equality; **not a glyph-only crop.**
      The full-region proof now uses the label metrics bbox and includes the
      background pill; red proof: mean channel delta 11.749, green proof: 5.127.
- [x] The render-quality label check runs on **recipe renders per projected label
      region** — each authored label/callout bbox projected from
      `LabelDesc::metrics()`, `evaluate_label_region_quality` run per region (today
      it evaluates the introspection content/subject bbox, `src/render/quality.rs`,
      so a broken label is diluted by the whole subject). Must fail on the old
      per-cell output, pass on the atlas output.
- [x] The known-bad fixture set **MUST include a GPU (lavapipe) native-resolution
      capture of the current per-cell eroded label**, and the per-label quality
      check must FAIL on it. CPU-only known-bad fixtures are insufficient — that is
      exactly why the eroded GPU recipe render previously passed `ok:true`. Since
      the atlas fix makes the live GPU render crisp, this saved pre-fix GPU capture
      is the only way to keep proving the verifier catches GPU label erosion.
- [x] `LabelGlyphCell` / `LabelDesc::glyph_cells()` / the per-cell draw path is
      **retired**, not left beside the atlas.

## Build order

- [x] 1. `render::prepare::labels` — glyph rasterise + deterministic atlas pack.
- [x] 2. CPU rasteriser samples the atlas (manual bilinear); replace per-cell draw.
- [x] 3. `render::gpu::labels` pass (modelled on strokes) samples the atlas (manual
      bilinear matching CPU).
- [x] 4. CPU/GPU parity proof at native res on lavapipe (tolerance diff) + project
      per-label regions and wire `evaluate_label_region_quality` into recipe
      verification as the gate.
- [x] 5. Retire `LabelGlyphCell` / `glyph_cells()`; convert labels to atlas
      quads; rewrite per-cell/pixel-geometry tests (keep metrics tests).
- [x] 6. Doctor guard: assert no `LabelGlyphCell` / no per-cell label primitive path.
- [ ] 7. (Later) SDF/MSDF phase for live zoom / world-scaled text.

## Gates

- [ ] `cargo fmt --check` · `clippy -D warnings` (default + `scene-host,inspection`)
      · `cargo test` (×2) · `doctor --full` · `doc -D warnings`.
- [ ] `cargo publish --dry-run` compressed size (<10 MiB) re-check if a dependency
      is added (none expected for Option A).
