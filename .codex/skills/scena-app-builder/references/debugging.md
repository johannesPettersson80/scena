# Debugging And Recovery

Use structured reports. Do not guess from image presence.

Use `RECIPE` for the recipe path resolved from the template manifest `files[]`
or from the file you authored directly.

## Validation Fails

Run:

```bash
scena validate-recipe "$RECIPE"
```

Fix the field named by each diagnostic `path`. For unknown fields or enum
values, use `scena schema get scena.scene_recipe.v1` and apply the closest
schema-backed name only when the suggestion is unambiguous.

## Blank Or Empty Frame

Run render with introspection and inspect reasons:

```bash
scena recipe render "$RECIPE" --introspect --verify --out frame.png
```

Common causes:

- no active camera;
- object outside frustum;
- object behind camera;
- hidden parent or layer mask;
- alpha/material makes target invisible;
- clipping plane removed the target;
- missing geometry/material/texture evidence.

Use:

```bash
scena inspect "$RECIPE"
scena diagnose "$RECIPE" --visibility --handle <handle>
scena repair "$RECIPE" --from diagnosis.json
```

## Tiny Or Cropped Subject

Use overlay-aware framing when labels, measurements, section boxes, or callouts
are present. Treat `tiny_in_frame` or `cropped` as a failed app proof when the
task is CAD/documentation, even if global introspection leaves warnings as
`ok:true`.

## Wrong Color Or Material

Use appearance expectations. Whole-frame average color or pixel-change tests are
not enough for multi-part scenes.

Check:

- target id resolves to the intended node;
- material variant is active;
- source material/texture is present when required;
- swatch tolerance is explicit;
- no generated fallback is being accepted as source material.

## Animation Or Twin State Does Not Change

Use animation/state verification instead of comparing one still image.

Check:

- the target node/id is named explicitly;
- sampled times include before and after states;
- the expected channel/path matches the authored clip;
- host-ticked time is advanced.

## Pick Or Hover Fails

Use synthetic interaction verification.

Check:

- viewport and device-pixel-ratio match the expectation;
- expected handle/id is resolved from the build manifest;
- strokes or overlays are not expected to be picked unless the current contract
  says they are pickable.

## Repair Loop

Apply only explicit, reversible repairs:

- camera/framing repairs are presentation repairs;
- visibility/scale/alpha repairs are content repairs and must be reported;
- if a report has no patch or says `auto_fixable:false`, stop and ask for
  host/user input.
