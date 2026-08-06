# Photo-Like Output: Six Fixes

Scope: Fix only these six items, in order, for the four existing handmade
assets. No Blender, replacement assets, V3D work, new quality metrics,
infrastructure, or unrelated refactors.

- [x] **Curved geometry.** Add bounded Scena render-time refinement for coarse
      smooth curves and bevels; preserve the original asset designs.
- [x] **Rubber.** Remove the speckled asphalt look from the hero bellows and
      speaker foot by correcting the existing rubber material.
- [x] **Studio.** Remove the backdrop edge, wall-floor seams, speaker-foot
      smudge, and confirmed output-gradient banding.
- [x] **Grounding.** Tune the existing physical shadows for attached, suitably
      sharp contact; do not add a duplicate fake shadow pass.
- [x] **Reflections.** Isolate and fix the dotted light-panel and cubemap-seam
      artifacts; change only the proven cause.
- [x] **Quality policy.** After the first five fixes pass native-resolution
      visual review, calibrate the existing metrics and make only reliable ones
      blocking.

Work top to bottom. Mark a row complete only after its affected 1:1 output crop
is visually clean.
