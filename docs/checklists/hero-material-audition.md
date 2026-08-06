# Hero Material Audition

This checklist isolates material response before any further `demo_hero` asset
changes. The source GLB and hero recipe remain frozen until the controlled
material board has been inspected at native resolution.

## Constraints

- Use only the existing public scene-recipe, material-pack, and imperfection
  interfaces.
- Keep Studio Small 08, PBR Neutral, camera, lights, and exposure fixed across
  every sample.
- Treat clean samples as diagnostic controls only. Every final candidate has
  deterministic, material-appropriate surface history.
- Do not change source geometry, the renderer, lighting correction, or the hero
  recipe until the board identifies a convincing candidate.
- Stop after two remedies with the same visual signature and add one smaller
  discriminating probe.
- Do not commit or push.

## Checklist

- [x] **1. Build and validate the controlled board**
  - Render current, manifest-clean, and manifest-lived columns on both flat and
    curved geometry.
  - Use `1920x1260`, SSAA1, fixed `-0.21909584 EV`, and PBR Neutral.
  - focused: the recipe was absent before the slice, then
    `validate-recipe --full` passed with every pack and Studio Small 08 loaded.
    Twenty-one isolated cell renders report `ok:true`, `headless_gpu`, and
    `gpu_device:true`; the independently rendered flywheel current/lived
    controls are byte-identical (`69f4b41e25c5d8e...`).
  - visual: `target/hero-material-audition/hero-material-audition-board.png` is
    `1920x1260`; each cell was rendered at the same transform and camera before
    assembly, avoiding position-dependent reflection bias.
  - scoped: SHA-256 verification passed for the board and every native cell.
  - skipped: the first all-material scene exceeded the default decoded-texture
    budget, so it was replaced by isolated same-pose cells rather than raising
    policy or changing renderer behavior.

- [x] **2. Inspect material response at 1:1**
  - Produce named native-resolution crops for flywheel, gearbox, baseplate,
    powder coat, bores, rubber, and small steel.
  - Record separately whether the base material or imperfection creates the
    improvement.
  - Reject perfect-looking surfaces, generic noise, visible tiling, and damage.
  - focused: clean-versus-lived normalized RMSE is flywheel `0.747%`, gearbox
    `0.107%`, baseplate `0.289%`, powder coat `0.114%`, bores `0.042%`, rubber
    `0.092%`, and small steel `0.033%`.
  - visual: catalog substitutions are visible, but every non-flywheel
    imperfection is effectively invisible at 1:1. The `0.10` strength control
    in `imperfection-strength-010-calibration.png` remains visually unchanged.
  - scoped: one discriminating calibration set every non-flywheel profile to
    the flywheel's proven `0.10` strength while holding material, camera,
    environment, exposure, geometry, seed, and physical scale fixed. The result
    rules out insufficient strength as the primary cause.
  - skipped: no further strength tuning. The current profiles affect mostly
    roughness/normal response and do not create readable surface history on
    these materials under the controlled studio.

- [ ] **3. Apply only proven bindings**
  - Preserve the flywheel's accepted `Metal052B` oil-film treatment.
  - Change only hero material bindings whose lived audition sample is visibly
    better than the current sample.
  - focused: blocked by row 2; the lived candidates do not meet the stated
    native-resolution visibility criterion.
  - visual: the hero recipe and source GLB remain unchanged by this audition.
  - scoped: none; no binding was promoted from a failed audition.
  - skipped: applying technically present but invisible imperfections would
    reproduce the prior regression cycle.

- [ ] **4. Prove the assembled hero once**
  - Render one `3840x2520` SSAA1 diagnostic with the deterministic product-photo
    setup and inspect named 1:1 crops.
  - Render SSAA2 only after the diagnostic is accepted.
  - focused: not entered because row 3 made no hero change.
  - visual: not rendered; there is no proven assembled-material delta to test.
  - scoped: none.
  - skipped: both SSAA1 diagnostic and SSAA2 final avoid wasting a long render
    on candidates already rejected by the material-only proof.

## Validation ledger counters

- elapsed investigation time: approximately 35 minutes for this audition
- remediation-attempt count: 2 (texture-budget split and same-pose harness fix)
- release-candidate push count: 0
- full-matrix run count: 0
- user-required action count: 0
- scoped gate: `cargo run -p xtask -- doctor --full` completed in the isolated
  builder checkout and reported the same 31 existing repository findings; none
  names the new audition recipe or checklist.
- full gates: skipped because this proof changed no Rust, public API, schema, or
  accepted hero behavior and is explicitly not release-ready.
