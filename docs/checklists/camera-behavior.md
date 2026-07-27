# Camera behavior checklist

Goal:

```text
Scene/model in -> subject -> compose -> meter -> focus -> render/probe -> measure -> adjust -> final pass/fail
```

## Implementation ledger — 2026-07-27

Implemented and proved in this slice:

- [x] `scena photo render model.glb --out photo.png --report photo.json` works without mandatory `--intent`.
- [x] Asset-input emitted recipes no longer include hand-authored cameras, lights, render exposure, focus, scene background, floor, grid, or staging constants.
- [x] Recipe `photo.intent` and CLI photo render use the same bounded internal camera/exposure loop.
- [x] The loop composes from the resolved subject, renders/probes, measures final pixels, adjusts camera fill when feasible, adjusts exposure by measured subject luminance, and stops at a bounded attempt limit.
- [x] Subject exposure/readability is measured from foreground pixels, not whole frame or background-heavy projected rectangles.
- [x] Composition/fill is measured from projected subject geometry, while exposure/readability is measured from foreground pixels.
- [x] Subject focus resolves from subject bounds instead of a guessed recipe `focus_distance`.
- [x] Photo reports expose one subject-region bridge object built from `SubjectObservationV1`, world bounds, frame key, projected/visible bounds, and resolved focus distance.
- [x] Every measured camera-loop candidate now records the actual rendered camera world transform, projection, vertical FOV, focus distance, exposure, measured subject metrics, adjustment, and failure codes.
- [x] Failed photo loops now preserve a structured report and candidate history; the focused clipped-subject fixture proves a nonzero photo result still reports measured attempts.
- [x] Retry reports expose `budget_exhausted`; a deterministic unit fixture spends all six attempts and proves the full attempt history is preserved.
- [x] New implementation call sites use neutral `camera_behavior_*` planner/scorer wrappers; `product_hero` remains only as a legacy compatibility spelling/API alias.
- [x] The public demo hero proof is generated from `evidence/demo-hero/hero.recipe.json` with `scena recipe render --gpu --verify`; checked proof image hash: `915e9e36c31b7d9a1c46d8cc68c380e6fa0aeb09e97dd8d46fe9a41bb0dba10b`.
- [x] The demo page uses the checked proof image and the checked recipe, not a stale still.
- [x] Focused/scoped tests cover the demo proof, no-intent CLI render, recipe photo-intent render, dark-subject recovery, failed-loop candidate history, guide smoke, and CLI contract table.

Still open / not claimed complete:

- [x] Replace the camera-loop call sites with neutral camera-behavior names where that does not break the public recipe compatibility alias.
- [x] Quarantine legacy `product_hero` names behind compatibility aliases; canonical fixtures, reports, docs, CLI help, demo recipe, and candidate ids use `camera_behavior`.
- [x] Promote the implicit subject data into one typed subject-region bridge object shared by composition, metering, focus, and quality reports.
- [x] Record camera pose/FOV and focus distance per candidate attempt, not only the selected composition id/fill/exposure/metrics.
- [x] Add a hard failed-loop fixture that preserves failure diagnostics, now that dark subjects correctly recover.
- [x] Add a budget-exhaustion fixture where all retry attempts are spent, not just a fail-fast clipped-subject fixture.
- [ ] Run the full test/release gate chain only at the final checkpoint before push/release.

## 0. Hard rules

- [ ] Do not implement another preset.
- [ ] Do not use hardcoded hero angles as the core behavior.
- [ ] Do not solve the demo with hand-tuned camera, exposure, focus, background, floor, or staging constants.
- [ ] Do not mark success from hash existence, PNG existence, draw count, or `ok:true` alone.
- [ ] Do not close any row unless the final rendered pixels are measured.
- [ ] If the visual output is bad while tests are green, the test is wrong.
- [ ] Full tests run only after the final camera loop passes focused proof.

## 1. Subject

- [x] Accept a scene/model with no manual camera requirement.
- [x] Resolve the main subject from declared target if present.
- [x] If no target is declared, select the primary visible/imported object deterministically.
- [x] Produce one authoritative subject-region bridge.
- [x] `SubjectRegion` must include:
  - [x] target identity
  - [x] world bounds
  - [x] projected bounds
  - [x] visible foreground mask or fallback foreground pixels
  - [x] depth/focus distance
  - [x] camera/scene/viewport revision
- [x] Reject stale subject data.
- [x] Report fallback subject detection explicitly.
- [x] Test: wrong/missing subject fails with actionable diagnostics.

## 2. Compose

- [x] Implement a camera composition solver, not a preset selector.
- [x] Solver input:
  - [x] subject bounds
  - [x] viewport aspect
  - [x] visible subject pixels
  - [x] desired fill range
  - [x] center tolerance
  - [x] clipping/crop tolerance
- [x] Solver output:
  - [x] camera pose
  - [x] focal length/FOV
  - [x] projected subject fill
  - [x] center offset
  - [x] crop/clipping status
- [x] Fill must be based on final image composition, not generic max-axis AABB fill.
- [x] If subject width/area is too small, move camera/adjust FOV and retry.
- [x] If subject is cropped, pull back/adjust FOV and retry.
- [x] If subject is off-center, recompose and retry.
- [x] Test: current bad hero case fails before solver, then passes after solver.

## 3. Meter

- [x] Implement camera-like metering from actual subject/foreground pixels.
- [x] Do not meter the whole frame when a subject exists.
- [x] Do not meter a projected rectangle including large background unless explicitly fallback-reported.
- [x] Metering modes:
  - [x] subject-weighted
  - [x] matrix fallback
  - [x] highlight protection
- [x] Exposure output:
  - [x] measured subject mean luminance
  - [x] low clip fraction
  - [x] high clip fraction
  - [x] suggested exposure delta
  - [x] applied exposure
- [x] Exposure retry must apply delta and re-measure final pixels.
- [x] Test: dark subject on bright/dark background meters subject correctly.

## 4. Focus

- [x] Compute focus distance from subject/camera geometry.
- [x] Do not require guessed `focus_distance`.
- [x] If depth is unavailable, report fallback.
- [x] Focus output:
  - [x] target subject
  - [x] resolved distance
  - [x] confidence/source
- [x] Test: focus resolves from subject, not from a hand-authored number.

## 5. Render/probe loop

- [x] Implement an internal camera loop:

```text
compose
render/probe
measure
adjust
repeat until pass or budget exhausted
```

- [x] Each iteration records:
  - [x] camera pose/FOV
  - [x] subject fill
  - [x] center offset
  - [x] exposure
  - [x] focus distance
  - [x] luminance/clip metrics
  - [x] failure reasons
  - [x] adjustment made
- [x] Adjustment rules:
  - [x] too small -> closer/narrower FOV
  - [x] too large/cropped -> farther/wider FOV
  - [x] off-center -> re-center
  - [x] too dark -> raise exposure
  - [x] too bright -> lower exposure
  - [x] low contrast/readability -> adjust view/light only if allowed; otherwise fail
- [x] Loop must have a bounded attempt limit.
- [x] Failed loop returns failure with all candidate diagnostics.
- [x] Budget-exhausted loop returns failure with all candidate diagnostics.
- [x] Test: final selected image must be produced by a measured pass, not the first render.

## 6. Measure

- [x] Define one authoritative final image measurement.
- [x] Measurement must use final PNG/render bytes.
- [x] Measurement must use subject foreground pixels/mask.
- [x] Required final metrics:
  - [x] subject fill width/area
  - [x] subject center offset
  - [x] crop/clipping fraction
  - [x] mean luminance
  - [x] low clip fraction
  - [x] high clip fraction
  - [x] luminance range/stddev
  - [x] background separation
- [ ] Remove or demote contradictory whole-rect/whole-frame metrics.
- [ ] Test: projected-rect background cannot create false pass/fail against subject foreground truth.

## 7. Final acceptance

- [x] Final success requires:
  - [x] `ok:true`
  - [x] no error reasons
  - [x] subject resolved without stale data
  - [x] composition in band
  - [x] exposure in band
  - [x] focus resolved or intentionally disabled
  - [x] final pixels measured from final output
- [x] Final failure requires:
  - [x] nonzero exit
  - [x] structured diagnostics
  - [x] candidate history
  - [x] suggested next action
- [x] Test: no success if nested quality/composition/photo checks fail.

## 8. CLI/API surface

- [x] Primary command:

```bash
scena photo render model.glb --out photo.png --report photo.json
```

- [x] Recipe equivalent:

```bash
scena recipe render scene.recipe.json --verify --out photo.png
```

- [ ] Intent may set target bands only.
- [ ] Intent must not hardcode camera angle/staging/preset.
- [x] Report includes:
  - [x] subject report
  - [x] composition report
  - [x] metering report
  - [x] focus report
  - [x] iteration/candidate history
  - [x] final acceptance metrics
- [x] Test: recipe and CLI paths use the same camera loop and acceptance gate.

## 9. Demo proof

- [x] Use the real demo/model path.
- [x] No manual camera.
- [x] No manual exposure.
- [x] No manual focus.
- [x] No manual floor/background/staging constants.
- [x] Run:

```bash
scena recipe render evidence/demo-hero/hero.recipe.json --gpu --verify --out target/demo-hero/hero.png
```

- [x] Assert from final PNG/report:
  - [x] subject fill in band
  - [x] subject luminance in band
  - [x] low/high clip in band
  - [x] center offset in band
  - [ ] no stale/fallback subject unless explicitly accepted
  - [x] final `ok:true`
- [x] Only update checked-in demo image after this passes.

## 10. Validation order

- [x] First: focused failing proof for current bad hero.
- [x] Second: implement subject/composition/meter/focus loop.
- [x] Third: rerun same focused proof until green.
- [x] Fourth: run scoped tests for touched modules.
- [x] Fifth: render final GPU demo proof.
- [ ] Last only: full test suite / release gates.

## 11. Deletion/cleanup

- [x] Remove or quarantine legacy preset-like `product_hero` implementation as core behavior.
- [x] Keep only reusable generic pieces:
  - [x] subject measurement
  - [x] exposure report
  - [x] focus report
  - [x] final pixel quality gate
  - [x] fail-closed aggregation
- [x] Remove docs claiming preset-based photo mode solves camera behavior.
- [x] Replace checklist rows that were closed by weak proof.
