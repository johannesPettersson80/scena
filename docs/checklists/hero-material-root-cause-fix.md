# Hero Material Root-Cause Fix

## Scope

- Fix only the imperfection-strength dead zone.
- Keep the existing compositor, profiles, masks, shaders, lighting, exposure,
  staging, geometry, and source GLB unchanged.
- Render the hero only after all six non-flywheel material treatments are
  clearly visible and restrained on the controlled material board.
- Do not commit or push.

## Checklist

- [x] **1. Fix imperfection amplitude**
  - Add a focused test proving every profile's default changes roughness by a
    useful encoded amount instead of only 1-3 byte values.
  - Confirm the test fails before changing the defaults.
  - Recalibrate only the profile defaults and authoring guidance; preserve
    explicitly authored strengths, including the flywheel's accepted `0.10`.
  - focused: the new default-amplitude regression failed on the old code with
    `Dust ... got 5`, then passed after the profile defaults changed to dust
    `0.30`, smudge `0.40`, fine scratches `0.30`, and oil film `0.65`.
    The recipe guidance contract also passed and no longer recommends the
    `0.08 through 0.20` dead zone. Explicit strengths remain unchanged.
  - scoped: remote `cargo fmt --check` passed.

- [x] **2. Create and inspect the six material treatments**
  - Render gearbox, baseplate, powder coat, bores, rubber, and small steel at
    the same camera, environment, exposure, geometry, scale, and seed as the
    existing audition.
  - Pass only treatments that are visible at 1:1 without reading as noise or
    damage.
  - focused: six isolated recipes validated and rendered with `ok:true`,
    `headless_gpu`, `gpu_device:true`, and no fallback.
  - visual: failed the promotion gate. The stronger baseplate and gearbox
    response is detectable, but the bore, rubber, and small-steel treatments
    remain effectively unchanged at 1:1; the set does not reproduce the
    flywheel's readable surface history. See
    `target/hero-material-root-cause/materials-clean-vs-lived.png`.
  - scoped: every candidate PNG is checksum-pinned in
    `target/hero-material-root-cause/SHA256SUMS`.

- [x] **3. Apply passed materials and render the hero**
  - Change only the hero material bindings proven by row 2.
  - Preserve the source GLB and the accepted flywheel treatment.
  - Render once with the contract-valid native-resolution final settings and
    inspect named 1:1 crops.
  - focused: replacement material cells passed. The assembled all-treatment
    recipe exceeded the fixed 64 MiB texture policy, while the visible-priority
    subset passed `recipe build` under the same policy.
  - provenance correction: the earlier `3840x2520` failure used an authored
    diagnostic recipe with `photo.quality: preview`. That master-tier change
    disabled the final studio environment and all local reflection probes, so
    its `52.75` subject mean and the material-only A/B were invalid promotion
    evidence. No individual light parameter had changed, but the execution tier
    had; the earlier conclusion is withdrawn.
  - visual: the corrected `final` recipe passed at native `3840x2520`, SSAA2,
    with the 2048x1024 studio environment, 512-pixel cubemap, four local probes,
    subject mean `98.6`, highlight fraction `0.2323`, and no failure codes.
    Named 1:1 crops retain the accepted flywheel while showing the promoted
    gearbox, baseplate, and powder surfaces under the correct studio response.
  - scoped: source GLB remains
    `409fe353579af47b67d3a1f22b87a99bb38e1c24b5fd897909866b0eea4956d8`.
  - reopened: the final studio lacks deliberate bright and dark reflection
    structure. Keep the technically valid final rig and fix only this material-
    appearance gap with two deterministic, capture-only reflection cards.
  - fixed scope: one white strip and one black flag at `2.0 *` subject radius,
    hero baseplate/gearbox texture-scale corrections, zero-config WaterBottle
    proof, then one native hero final if the focused visual gates pass.
  - focused: test-first proof failed on missing reflection-card functions, then
    passed with exactly two cards at `+/-40` degrees, `2.0 *` subject radius,
    fixed `2.0 * height` by `0.35 * height` dimensions, and complete removal
    before the beauty scene. `cargo fmt --check` passed on the isolated builder.
  - generic visual: the untouched WaterBottle photo rig gained useful structured
    reflections with highlight coverage increasing from `0.0507` to `0.1035`;
    no card or new shadow appeared. Its separate pre-existing luminance failure
    remained (`79.95` before, `79.43` after).
  - hero visual: failed. The fixed cards added a dark rim band, but the flywheel
    stayed gray and did not recover a broad white reflection band. Its semantic
    material `p99` was only `157.95`, which makes the required rim `p95 >= 180`
    impossible. The row remains open; no second placement or extra card was
    attempted.
  - root cause: the bright card was diffuse white at linear radiance `1.0`,
    about `4.5` stops below the prior reflection source. Keep its proven
    placement and make it capture-only emissive at linear radiance `4.0`; one
    diagnostic render decides the row, with no material or lighting retuning.
  - bounded retry: radiance `4.0` raised the flywheel material `p99` from
    `157.95` to `215.17`, but the white band remained too narrow at 1:1. Use the
    one permitted retry at radiance `6.0`; stop if the same broad-band gate
    still fails.
  - final retry: radiance `6.0` produced a clean `3840x2520` final SSAA2 render
    with four probes, no fallback, no clipping, and flywheel material `p99`
    `232.17`. The fixed rim region retained `p05 < 45`, but only about `4%` of
    its pixels reached `180`, leaving `p95` around `162-166`. The strict
    `p95 >= 180` broad-band gate therefore remains open. Stop: no higher
    radiance, card resizing, repositioning, or material/lighting retuning.
  - artifacts: final PNG SHA-256
    `95be62e4288717d5563e0da182847825954a3aeb00b13cf8143ab6e5c0cbd64c`;
    report SHA-256
    `76fb367c57ed32957925008e46a30b0e2223d3b55d4fe303ebe295c7f11a96f7`;
    named 1:1 flywheel crop SHA-256
    `e51f7fcc05a912884ec58b058106adb05c33a23ca61ce921cace1b7b0d9d2582`.
  - provenance: the flywheel binding remained byte-identical at SHA-256
    `ce3806d0c1e67a544f958d901bb97354fcf88d01e61d18b48ae814df980396bc`.
    The emitted final recipe records `3840x2520`, SSAA2, tent reconstruction,
    PBR Neutral, final quality, and four local probes.

## Minimal continuation

- [x] **A. Test two replacement materials**
  - Bore: replace sub-pixel `fine_scratches` with `oil_film` on `Metal052B`.
  - Small steel: replace sub-pixel `fine_scratches` with `oil_film` on
    `Metal009`.
  - Keep camera, lighting, exposure, geometry, physical scale, and seed fixed.
  - Pass only if both treatments are readable and restrained at 1:1.
  - focused: bore oil at `0.30` reached `0.942%` RMSE with a 31-LSB peak;
    small-steel oil at `0.65` reached `0.827%` with a 21-LSB peak. Both
    `headless_gpu` cells passed without fallback.

- [x] **B. Promote the passed material set**
  - Promote the already-passed gearbox oil, baseplate smudge, and powder dust.
  - Promote the two replacement treatments only if row A passes.
  - Use the clean rubber preset with no imperfection or extra texture budget.
  - Keep small steel clean because its surface history is not resolved on the
    tiny bolts/data plate and the visible shaft treatment does not fit the
    fixed texture budget beside the three dominant treatments.
  - resource proof: the all-treatment recipe exceeded the fixed 64 MiB decoded
    texture policy. After two identical failures, a build-only diagnostic kept
    history on the three dominant surfaces and passed. The promoted hero
    therefore uses baseplate smudge, flywheel oil, and gearbox oil; bore,
    powder, shaft, and rubber retain their improved base materials clean.
  - Preserve the accepted flywheel treatment exactly.
  - result: the subset passed the build policy and is retained after the
    corrected final-tier render passed.

- [x] **C. Render the hero once**
  - The renderer contract does not allow SSAA1 with `photo.quality: final`;
    final photography requires SSAA2 or greater, no redundant MSAA, and tent
    reconstruction. Run one contract-valid `3840x2520` SSAA2 render and inspect
    the full frame plus named native-resolution material crops.
  - focused: the authoritative current binary was built from the isolated
    synced checkout and SHA-256 pinned before rendering.
  - visual: the corrected final-tier SSAA2 render passed the technical photo
    gate; the later reopened material-appearance gate failed as recorded above.

## Final bounded card and shaft closure

- [x] **D. Audit every hero material binding**
  - Check all eleven bindings against the named part, effective roughness, and
    latest native-resolution response.
  - Result: ten are the correct class or an acceptable photographic
    approximation. The only proven mismatch is `turned steel shaft` on
    `metal009` (effective roughness about `0.47`); the existing `metal052b`
    path is about `0.17` and matches ground/turned steel. No other binding is
    authorized to change.
  - grouped verdict: `metal009` remains appropriate for satin gearbox,
    baseplate, machined flange/bracket, bolts, and data plate; `metal052b`
    remains appropriate for the flywheel and recessed bore; `plastic013a`
    remains appropriate for powder coat; the rubber preset remains appropriate
    for the isolator and bellows. The baseplate brushing is visually strong but
    is not a wrong material class.

- [x] **E. Widen the generic cards and correct the shaft binding**
  - Set card width to `2.0 * max(subject_extent.x, subject_extent.z)` while
    preserving radiance, angle, distance, height, capture-only lifecycle, and
    the dark flag.
  - Rebind only `turned steel shaft` to `metal052b`, tile `0.22`; keep the
    flywheel byte-identical and select the existing `reflective` ground intent.
  - focused: the width test failed against `height * 0.35`, then passed after
    the implementation used `2.0 * max(extent.x, extent.z)`. The test continues
    to pin angle, distance, height, radiance `6.0`, dark-flag value, and removal
    before beauty rendering. `cargo fmt --check` passed remotely.
  - recipe: the shaft alone now uses `metal052b` at tile `0.22`; the recipe
    build passed the existing 64 MiB policy with no diagnostics or skipped
    assets. Source GLB and flywheel hashes remain unchanged.

- [ ] **F. Render and decide once**
  - Render one native `3840x2520` final SSAA2 hero and inspect all material
    regions at 1:1.
  - Require a broad flywheel white/dark reflection band and a continuous shaft
    highlight. If the hero passes, render the untouched WaterBottle once to
    confirm the wider generic cards are unchanged-or-better.
  - Stop on a failed focused visual gate; do not change exposure, lights,
    backdrop, additional materials, or card radiance.
  - hero: final `3840x2520` SSAA2 software-conformance render passed with no
    fallback or failure codes. Fixed flywheel-rim pixels measured `p05 44.94`,
    `p95 236.17`, with `14.66% >= 180`. Across the unobstructed shaft span,
    every 32-pixel longitudinal segment carried a continuous bright stripe
    (`p95` minimum `183.2`, median about `195`); collar-obscured end segments
    were excluded.
  - material review: flywheel, bore, gearbox, machined flange/bracket, bolts,
    baseplate, data plate, powder coat, isolator, and bellows retained the
    audited bindings and remained coherent at 1:1. No additional binding was
    changed.
  - generic proof: untouched WaterBottle passed `ok:true` with no failure codes;
    mean luminance improved from `79.43` to `86.35`, highlight fraction from
    `0.1035` to `0.1696`, and highlight continuity to `0.9895`.
  - reflective-ground addendum: the requested final SSAA2 render completed and
    visibly produced the intended soft floor reflection, but the final gate
    rejected it with `subject_luminance_above_max`: measured `100.32` sRGB
    against the fixed maximum `100.00`. This is classified as a product-gate
    failure. Per the one-render stop rule, no exposure, lighting, material,
    ground-strength, or card-radiance tuning was attempted; this row remains
    open.
  - artifacts: hero PNG SHA-256
    `a3e174e98c9469a7eb16f0e9caa834ae3a7f6bc9f9cf58083d7a236667939517`;
    hero report
    `eeaca4f4cace8200e94726106a353ed437fd9ebbc175b61b6ca20fe86adad971`;
    WaterBottle PNG
    `65fb512907b97e83126c0f2a65709f49187046d225b0cb3c81a0437f4af29308`;
    WaterBottle report
    `b3aa9e1fbf14e3445d545fbf20f99eec932d2f1c4b5a8bc5e3ecfe78e0a25f3d`;
    reflective-ground PNG
    `d7675c327e4d81660698d2dae978cd175268a67cfb879eee1b7e0ffed4260c0e`;
    reflective-ground report
    `d80450ec14f94d1a5a1d46f1267a0799d2f949c3f0982bffaf5130bd2b603c17`.

## Validation ledger

- elapsed investigation time: approximately 190 minutes
- remediation-attempt count: 2 texture-budget remedies followed by one
  discriminating build probe; one invalid preview-tier luminance A/B; one
  corrected final-tier render; one fixed reflection-card placement with zero
  placement remedies; two bounded radiance remedies (`4.0`, then `6.0`) and no
  further radiance tuning; one width correction and one shaft-binding
  correction closed the focused visual gates
- release-candidate push count: 0
- full-matrix run count: 0
- user-required action count: 0
- full: skipped because the focused contract test, scoped formatting/build
  proof, and direct final-quality GPU renders cover this bounded change.
- skipped: the proposed preview-vs-final hardening rules remain outside this
  bounded reflection-card continuation.
