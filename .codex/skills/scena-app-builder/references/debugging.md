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
scena recipe render "$RECIPE" --introspect --out frame.png
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

## Line Through Label

Run `scena recipe render "$RECIPE" --introspect --verify --out frame.png` and
inspect `verification.composition`. A failed `overlay_label_intersects_line`
means a projected leader, dimension, or helper line crosses the interior of a
label region. Move the label, offset the line endpoint, or shorten the leader
before the label; do not accept the render from image presence alone.

## Overlapping Labels

Run `scena recipe render "$RECIPE" --introspect --verify --out frame.png` and
inspect `verification.composition`. A failed `overlay_label_intersects_label`
means two projected label regions overlap at native resolution. Move one label,
reduce label size, or add an explicit layout offset; do not treat the render as
acceptable because both labels are individually visible.

## Clipped Label

Run `scena recipe render "$RECIPE" --introspect --verify --out frame.png` and
inspect `verification.composition`. A failed `overlay_label_clipped_by_viewport`
means the unclipped projected label rectangle extends outside the capture
viewport. Move the label, reduce the size, or use overlay-aware framing; do not
accept a partially visible label as proof that the overlay is readable.

## Floating Or Sinking Subject

When the subject must touch a floor, grid, fixture, or base plane, add
`expect_grounded` to the recipe expectation block and re-run
`scena recipe render "$RECIPE" --introspect --verify --out frame.png`. A failed
`ground_contact_missing` means the inspected world-space bounds do not touch
the declared `plane_y` within tolerance. Use a `ground` transform, adjust the
translation, or change `plane_y` only if the floor really moved.

## Helper Drawn Over Subject

When a grid, helper line, wireframe, or section helper should be hidden behind
solid geometry, add `expect_helper_occluded` and run
`scena recipe render "$RECIPE" --introspect --verify --out frame.png`. A failed
`helper_layer_overdraws_subject` means helper-coloured pixels are visible
inside the occluder region. Keep depth-tested helpers in the 3D layer, move the
helper behind the subject, or remove the expectation only when the overdraw is
intentional.

## Wrong Object In Front

When two solid objects overlap and a specific one must be in front, add
`expect_occlusion` with `front`, `back`, and optional `tolerance_pixels`, then
run `scena recipe render "$RECIPE" --introspect --verify --out frame.png`. A
failed `object_depth_order_mismatch` means pixels from the expected back object
are visible inside the expected front object's projected interior. Fix object
transforms, opacity/material choice, or the expectation if the overlap is
intentional. A failed `object_depth_order_color_ambiguous` means the current
native-resolution color-probe cannot distinguish the expected front/back draw
colours; use high-contrast opaque materials before relying on the depth-order
expectation.

## Wrong Backend Or Missing Quality Knob

For GPU/hero renders that must not fall back to CPU, add `expect_backend` with
`{"backend":"headless_gpu","gpu_device":true}` and run
`scena recipe render "$RECIPE" --gpu --introspect --verify --out frame.png`.
`backend_expectation_mismatch` means the actual backend did not match the proof
requirement. Checked `render_antialiasing_active`,
`render_supersample_active`, and `render_reconstruction_active` entries confirm
that the requested render-quality knobs were active.

## Section Box Or Clipping Missing

For cutaways and sectioned views, add `expect_clipping` with the exact active
user clipping-plane count and section-box state. `clipping_plane_count_mismatch`
means the renderer did not activate the expected user clipping planes.
`section_box_missing` means no section box is active even though the recipe or
expectation requires one. `section_box_inversion_mismatch` means the cutaway is
inverted differently than requested.

## Material Variant State Wrong

For product/configurator scenes, add `expect_state` for each import whose
material variant matters. `material_variant_state_mismatch` means the rendered
import has a different active variant than the recipe expected; apply the
variant before rendering or update the expectation if the default variant is
intentional.

## Object Too Dark, Blown Out, Or Low Contrast

When a render passes visibility checks but looks visually dead, inspect
`verification.composition`; product renders run severe subject exposure checks
for authored and imported objects even without an explicit `expect_quality`
profile. A failed `subject_black_crushed` means a declared object region is dominated by
near-black pixels; add lighting/environment, raise exposure, or avoid dead
metallic-black materials unless that is the intended proof. A failed
`subject_blown_out` means highlights are clipped. A failed
`subject_salience_too_low` means the object is too close to the background to
read clearly; change material, background, or rim lighting.

## Subject Too Small Or Over-Cropped

When a user-facing render looks weakly framed, include an `expect_quality`
profile and inspect `verification.composition`. A failed
`subject_too_small_in_frame` means the declared object is technically visible
but too small for the selected profile; move the camera closer, reduce FOV, or
frame the subject explicitly. A failed `subject_too_large_in_frame` means the
object fills too much of the frame; pull back, widen FOV, or use an explicit
`expect_bbox_fit` range if the crop is intentional.

## Textured Material Looks Flat

When a textured material passes build/visibility but looks like a flat fill,
include an `expect_quality` profile and inspect `verification.composition`.
A failed `texture_result_flat` means a decoded base-color texture exists, but
the rendered target region does not show enough native-resolution texture
variation. Check UVs, texture transforms, sampler/wrap mode, and whether the
intended texture is mapped onto the target.

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
