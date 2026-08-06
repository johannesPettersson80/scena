# Deterministic Product Photo Default

This checklist replaces closed-loop correction in the default product-photo
path with the smallest predictable feature that produces a useful studio image.
The existing optimizer may remain only as explicit advanced behavior.

## Scope

- Preserve the frozen source GLBs, the hero prepared-geometry repairs, PBR
  Neutral, Studio Small 08, and one-shot camera candidate selection.
- Do not retune the four recipes, add another lighting model, add generic
  material machinery, or change source assets.
- Keep the default deterministic: the same scene and options produce the same
  camera, rig, exposure decision, and pixels on a given backend.
- After two remedies with the same visual signature, stop and add one smaller
  discriminator.
- Do not commit or push.

## Bootstrap

- canonical source and destination: `/home/johannes/projects/scena`
- branch: `demo/hero-scene`
- HEAD: `a1ef67f0a6e1602fb7ebcb0affe09de93fb2a30f`
- `AGENTS.md` SHA-256:
  `d0ed47595f6b6a6ec733651dfde650b7893aa2939d6137dc2f7eeade7528ac7a`
- `.codex/skills/**` aggregate SHA-256:
  `a333a1ac0f97feaa5abf4512d2eac8b2ec77b0f4b3b59f24a608331c48216fa3`
- bootstrap status: canonical files present and matched before edits

## Checklist

- [x] **1. Pin the known-good visual anchor**
  - Preserve `target/photo-realism-row8/demo-hero.png` and its report as the
    target look: exposure `-0.21909584`, subject mean `95.89`, dark-material
    mean `26.75`, PBR Neutral, SSAA2.
  - Preserve the corrected straight shaft and clean plate edges from
    `row8-hero-corrected-v2.png`.
  - focused: SHA-256 rechecked: hero PNG `d0aaff9f74a59f03...`,
    report `c4d9c39d6e13572e...`, corrected-geometry frame
    `5647b5a95faf6344...`.
  - visual: the anchor and corrected shaft/plate frame remain unchanged under
    `target/`; no source GLB was edited.
  - scoped: source-vs-derived geometry protection remains outside this patch.
  - skipped: no new native-resolution render was needed to pin existing bytes.

- [x] **2. Make correction one-shot and bounded**
  - Keep bounded one-shot camera candidate selection.
  - The default path renders the selected camera with the deterministic studio
    rig, computes at most one exposure correction, clamps it to `+/-0.75 EV`
    from that rig's base, renders once more only when the correction is
    non-zero, and never changes lighting from image metrics.
  - The iterative optimizer is not entered by the default path.
  - focused: test-first
    `default_photo_correction_is_one_shot_bounded_and_fail_closed` passed on
    `scena-builder`; it covers default/optimizer routing, the fixed studio
    base, the `+/-0.75 EV` bound, and the two-render budget.
  - visual: `demo-hero-simple-default-proof.png` selected candidate 2 at
    `0.7719931 EV`, subject mean `80.15`, and high-clip fraction `0.0`.
  - scoped: default un-authored photos use `Scene::add_studio_lighting`; the
    old metric-driven lighting solver remains available only with
    `--optimize`.
  - skipped: no optimizer visual retuning and no new light model.

- [x] **3. Make acceptance fail closed without controlling the image**
  - Quality metrics may accept or reject the delivered frame with named
    reasons, but cannot trigger another lighting/exposure loop.
  - A subject mean below the existing minimum remains a failure even when one
    dark-material scalar falls inside `20-45` sRGB.
  - focused: the same focused test first failed on the Goodhart frame, then
    passed after whole-subject one-shot exposure and fail-closed acceptance
    were separated.
  - visual: WaterBottle reports `subject_luminance_below_min`; terminal block
    reports `subject_luminance_structure_below_min`; RiggedSimple and hero
    report `contact_shadow_missing`. None triggered another lighting pass.
  - scoped: reports identify `deterministic_one_shot` and
    `built_in_studio_directional`; generated local-light count is zero.
  - skipped: report failures were not converted into automatic scene edits.

- [x] **4. Keep the sweep smooth and texture only the floor**
  - Use separate generated floor and cyclorama receiver geometry/materials.
  - Keep the subtle deterministic matte normal on the near floor only.
  - The curved sweep and rear wall remain optically smooth; do not use extreme
    `400x` tiling as a substitute for separating the surfaces.
  - focused: test-first
    `generated_floor_keeps_microstructure_off_the_smooth_pbr_backdrop`
    failed for both the unlit backdrop and narrow floor, then passed with a
    smooth PBR/no-normal backdrop and floor width matched to the `1.8x`
    backdrop framing envelope.
  - visual: the controlled reflective-ground A/B retained the four spots,
    proving the floor normal was not their source; the built-in-light hero
    proof removes the spots, and `control-terminal-block-wide-floor.png`
    removes the white corner wedges.
  - scoped: only the matte floor retains the existing subtle normal; sweep and
    rear wall have no normal texture.
  - skipped: no extreme retiling, SSR, or new backdrop shader.

- [ ] **5. Prove the default without fixture-specific tuning**
  - Render one corrected 3840x2520 SSAA2 hero and native crops for exposure,
    flywheel, shaft/plate, and backdrop.
  - Run zero-config deterministic controls on WaterBottle and two existing
    untouched assets without adding recipe overrides.
  - Record software conformance separately from any physical-GPU evidence.
  - focused: 640x420 hero and 320x210 zero-config WaterBottle, terminal-block,
    and RiggedSimple software-conformance renders completed. A ToyCar control
    was stopped after 7m14s at 100% CPU because it was not a bounded small
    control on lavapipe.
  - visual: `demo-hero-simple-default-proof.png` is bright, spot-free, and has
    no floor wedge; the three untouched controls produce deterministic images
    and honest named rejection reasons.
  - scoped: all proof above is remote lavapipe `software_conformance` only.
  - skipped: the post-fix 3840x2520 SSAA2 hero and native crops remain open;
    the earlier full render took about 76 minutes and predates the final
    built-in-light/floor-width patch. No physical-GPU claim was made.

- [ ] **6. Integrate once**
  - Run formatting, focused tests, the scoped CLI/renderer tests, and the
    applicable doctor gate in the isolated builder checkout.
  - Run the full release chain only if the focused visual anchor is green and
    the change is being handed off as release-ready.
  - Update `CHANGELOG.md` under Unreleased only after the behavior is proven.
  - focused: both named focused tests passed in the isolated checkout.
  - visual: small hero preview proof and three zero-config controls inspected
    at native output size; the final PBR Neutral/SSAA2 lane remains row 5.
  - scoped: `cargo fmt --all -- --check` passed. `doctor --full` ran once and
    reported the known 31 repository findings, including unrelated
    architecture-size and stale contract pins.
  - full: not run; this handoff is not release-ready while row 5's native final
    and the repository-wide doctor findings remain open.
  - skipped: clippy/test/doc/browser/publish chain and changelog update; no
    release-readiness claim, commit, or push.

## Validation ledger counters

- elapsed investigation time: approximately 210 minutes for this correction
- remediation-attempt count: 5 (two same-signature staging remedies, then a
  discriminating ground A/B before the light-source fix; one exposure
  calibration; one floor-coverage fix)
- release-candidate push count: 0
- full-matrix run count: 0
- user-required action count: 0
