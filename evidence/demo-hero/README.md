# Demo hero — camera-behavior recipe path

`hero.recipe.json` is the public demo hero authored through the documented
photo-intent recipe surface. It declares the connector-snap assembly as the
subject and lets `photo.intent:"camera_behavior"` choose composition, staging,
subject metering, focus policy, and acceptance.

The recipe intentionally contains no hand-tuned camera, fixed `exposure_ev`,
manual `focus_distance`, floor geometry, grid, or background override. Older
PNG files in this directory are historical repro artifacts from the manual
recipe path; the current proof artifacts are generated from `hero.recipe.json`
and must pass the camera-behavior report gate before being copied into the demo.

Reproduce (remote builder per `AGENTS.md`):

```bash
scena validate-recipe evidence/demo-hero/hero.recipe.json --full
scena recipe render evidence/demo-hero/hero.recipe.json --gpu --verify \
  --out target/demo-hero/hero.png > target/demo-hero/hero.render.json
```

The direct CLI equivalent is:

```bash
scena photo render demo/samples/connector-snap/connector_snap_assembly.glb \
  --intent camera-behavior --out target/demo-hero/hero.png \
  --report target/demo-hero/hero.report.json \
  --emit-recipe target/demo-hero/hero.resolved.recipe.json
```

Current checked proof:

- `hero-camera-behavior.png` — `1800x1150`, subject mean luminance `96.1`,
  SHA-256 `915e9e36c31b7d9a1c46d8cc68c380e6fa0aeb09e97dd8d46fe9a41bb0dba10b`.
- `hero-camera-behavior.render.json` — `ok:true`, no render reasons, and
  camera-behavior composition checks for `subject_fit_sane`,
  `subject_exposure_sane`, visible coverage, and texture variation.
