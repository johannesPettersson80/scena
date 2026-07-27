# Troubleshooting

## Task-scoped build/cache disk usage

Inspect an exact task cache without deleting anything:

```bash
scripts/scena_task_cache_status.sh <task-slug>
```

The versioned JSON lists the isolated validation checkout, Cargo target, and
task-local temp directory with exact paths, byte sizes, modification ages,
reproducibility, retention guidance, and cleanup authority. Run the same script
over SSH to inspect the maintained builder. Keep caches while their task or
release evidence is active. A reproducible task cache may be removed only after
an explicit operator request naming that exact path; never infer deletion from
this read-only report and never recursively clean a home directory, workspace,
shared cache root, or unrelated task.

This page lists common problems and the first places to look.

## Release readiness reports a missing or incomplete artifact root

Run readiness against the canonical staged bundle explicitly:

```bash
cargo run -p xtask -- release-readiness --artifact-root target/gate-artifacts
```

`RELEASE-READY-ARTIFACT-ROOT` means neither the flag nor a non-empty
`SCENA_RELEASE_ARTIFACT_ROOT` selected a bundle. A missing/unreadable directory
or missing required files is reported as `RELEASE-READY-ARTIFACTS`. Inspect the
`scena.release_readiness.v1` fields `artifact_root`,
`discovered_artifact_count`, `required_artifact_count`, and
`validated_artifact_count`; zero validated artifacts is always failure. First
run `stage-release-artifacts <downloaded-root> target/gate-artifacts` if the
lane archives have not yet been assembled.

## The output is blank

The documented high-level glTF viewer and raw-asset CLI render paths install a
neutral presentation when an asset has no authored lighting/environment. Check
`FirstRender::diagnostics()` for `MissingLightingOrEnvironment` with
`fallback_applied = true`; that warning means the frame was recovered but the
presentation is still not authored. Low-level `Renderer` construction remains
black and unlit by default, so a successful byte capture alone is not proof of
a visible scene.

Check:

- The scene has an active camera.
- The camera is looking at the model.
- The model was instantiated into the scene.
- The model bounds are inside the camera frustum.
- `prepare()` was called after the latest scene or asset change.
- The renderer target size is non-zero.

If the renderer reports `NoActiveCamera`, call `Scene::add_default_camera` for
the standard framed camera or `Scene::set_active_camera` for an authored one.
Those exact remedies remain present after SceneHost and JSON conversion.

Useful examples:

- `examples/first_visible_render.rs`
- `examples/camera_framing.rs`
- `examples/headless_ci.rs`

## Camera behavior or subject-aware render failed

For product/model hero stills, start from the intent path:

```bash
scena photo render model.glb --out hero.png --report hero.report.json
```

or from a recipe with `photo.intent:"camera_behavior"` and an explicit
`photo.subject`. Treat `ok:false` from `scena.photo_render_result.v1` or
`scena.recipe_render_result.v1` as a domain failure on stdout. Do not reclassify
it by parsing the message; CLI dispatch errors still use `scena.cli_error.v1`
with `exit_class` and `code`.

Use the reason codes in the report:

| Symptom | Codes to inspect | First remedy |
|---|---|---|
| underexposed subject | `subject_luminance_below_min`, `subject_low_clip_above_max` | Keep `auto_exposure` enabled. Apply the report's `exposure_report.suggested_compensation_ev` as `render.exposure_compensation_ev`, or let `scena photo render` use its bounded retry. Do not replace subject metering with fixed `exposure_ev` unless the shot is deliberately manual. |
| subject too small | `subject_fill_below_min`, `subject_too_small_in_frame`, `subject_tiny_in_frame` | Use `photo.intent` or raise the composition fill constraint; do not pull the camera distance by hand for the public hero path. |
| subject too large or clipped | `subject_fill_above_max`, `subject_clipped_by_section_box`, `subject_clipped_by_clipping_plane`, `subject_outside_viewport` | Loosen the fill constraint, widen or clear clipping/section boxes, then re-render so the subject observation frame key is fresh. |
| stale_subject_observation | `stale_subject_observation` | Re-render after the camera, viewport, transform, visibility, material, or render-generation change. A stale subject observation is invalid evidence. |
| unresolved subject | `unresolved_target`, `invalid_photo_subject`, `subject_visible_pixels_missing` with a zero handle count | Run `scena validate-recipe --full`, check target candidates, and fix `photo.subject`, `render.metering.target`, or `render.depth_of_field.focus.target`. |
| unsupported subject mask | `subject_visible_mask_backend_unsupported`, `subject_transparent_unsupported`, `subject_visible_mask_missing` | Use a backend whose capability report supports exact subject masks, switch to opaque subject geometry for strict evidence, or accept a reported degraded path only when the operator policy allows it. |
| focus fallback | `focus_report.status:"unresolved"` or `focus_report.reason` | Check that the focus target is visible and has finite depth. Subject focus uses visible depth; it should not be replaced by guessed `focus_distance` in the easy path. |
| failed camera-behavior acceptance | `failure_codes[]` in `photo_report` or render verification | Fix the subject, declared target, staging constraints, or material readability until the camera-behavior report `status:"passed"`; a written PNG alone is not acceptance. |

For subject-aware lower-level recipes, inspect
`subject_observations[]`, `exposure_report`, and `focus_report` in the
introspection/verification result. Those reports are bound to the completed
readback frame; stale frame keys mean the diagnostics no longer certify the
pixels.

## A name was not found

Read the structured `candidates` array on the lookup error, recipe diagnostic,
SceneHost error, or `scena.cli_error.v1` response. Scena ranks at most three
names with one case- and separator-normalized algorithm, so a typo such as a
schema, template, node, geometry/mesh-resource, material, animation, variant,
anchor, connector, or environment preset can be repaired from the first
response. Do not scrape the human message, and do not auto-apply a candidate
when the surrounding task makes the choice ambiguous.

For a recipe, run the structured path before changing camera or material data:

```bash
scena validate-recipe recipe.json --full
scena recipe build recipe.json
scena diagnose recipe.json --visibility
scena repair recipe.json
```

Full validation reports `policy`, `resources`, and resource-attached
diagnostics. Check the normalized URI, `required` flag, and `allowed_roots`
before changing recipe content. `--syntax-only` deliberately does not establish
that render-time resources are available.

If the normalized URI is intentionally in an external model library, do not
move the file or disable the sandbox. Inspect and then reuse a narrow operator
root:

```bash
scena policy recipe --allow-root /srv/model-library
scena validate-recipe recipe.json --full --allow-root /srv/model-library
scena diagnose recipe.json --visibility --allow-root /srv/model-library
```

The directory must exist. The reported path is canonical, and canonical asset
paths must remain below it; a symlink or `..` traversal that lands elsewhere is
still a `policy_violation`. Repeat `--allow-root` for multiple libraries.

For a raw glTF/GLB, use the same asset path with `scena render`,
`scena diagnose --visibility`, and `scena repair`. Introspection/verification
is allowed to fail a provably invisible result even though a low-level capture
could return bytes.

These asset-or-recipe verbs do not use a compatibility shortcut for recipes.
They build every import through the same sandbox and resolver as
`scena recipe build`. If import 2 or later is missing or outside the allowed
roots, the command exits nonzero with its exact recipe-build diagnostic instead
of rendering a partial scene. `scena doctor recipe.json` therefore emits
`scena.recipe_build_result.v1`; `scena doctor model.glb` continues to emit
`scena.asset_doctor.v1`.

`scena repair target --from report.json` applies the same distinction before
planning. A raw target must load cleanly through asset doctor; a recipe must
complete policy-aware validation/build. If the target is missing, malformed,
or outside the allowed roots, fix or authorize the target first—the report is
not processed against an unchecked path. Do not pass a second positional
target; it is rejected rather than silently ignored.

If an older generated template reports a `policy_violation` for
`tests/assets/environment/polyhaven/...` or `tests/assets/gltf/...`, regenerate
it with the current installed CLI. Current templates use packaged builtin
assets and work outside a checkout. Explicit URI environments remain required
assets and fail closed; named `studio`/`neutral_studio` presets are portable.

## The model is too large or too small

Check unit metadata and import options.

See [Units, axes, and handedness](guides/units-axes-handedness.md).

## The model is rotated sideways

Check the authored up-axis and coordinate-system metadata.

See:

- [Units, axes, and handedness](guides/units-axes-handedness.md)
- [Troubleshooting misplaced assets](guides/troubleshooting-misplaced-assets.md)

## Textures are missing

Check:

- external image paths are correct,
- browser URLs are fetchable,
- image files are deployed next to the glTF file,
- optional texture features are enabled when required,
- unsupported required extensions are reported in the asset error.

`missing_texture` / `AssetError::MissingTexture` reports the material, slot,
raw glTF texture index, image source, and resolution reason. Repair that exact
entry; do not renumber later textures to compensate. Unreferenced invalid
texture entries are tolerated without changing any later material binding.

For application-generated pixels, use `TextureMemoryDesc` rather than a fake
path. A reused `TextureMemoryId` must keep identical pixels/options. Use
`rgba8_for_slot` or `load_texture_for_slot` so base-color/emissive/sheen-color
data is sRGB and normal/metallic/roughness/occlusion-style data is linear.
Native size/allocation failures are `AssetError::TextureSizeLimit`. Browser
resizes are reported as `AssetLoadWarning::TextureDownscaled` in structured
load telemetry and `Assets::texture_warnings()` as well as the console.

See [Assets](assets.md).

On native hosts, an absent file is `AssetError::NotFound` with the requested
path and curated help. Other filesystem failures remain `AssetError::Io`, so
callers can distinguish a typo/missing deployment from permissions or device
errors without parsing prose.

## Rendering fails after a resize

Resize and surface events invalidate prepared renderer state. Forward the event
to the renderer and call `prepare()` again before rendering.

See [Lifecycle](lifecycle.md).

## The CLI used an unexpected backend

Inspect the top-level `backend_selection` object. CPU is the default;
`source:"cli_flag"` appears only when `--gpu` was passed. The CLI ignores
`SCENA_USE_GPU`, which remains test/proof metadata, so shell state cannot
silently switch production rendering. If `fallback_used:true`, read the
machine-readable `reason` and `remedy`; the selected
backend is `headless` because the explicitly requested preferred-GPU path could
not build. Use strict GPU construction for hardware proof.

For a pre-render check, run `scena capabilities --live --json`. A successful
report must say `probe.status:"measured"` and identify the selected adapter and
device. Exit 1 with `probe.status:"unavailable"` is an actionable hardware or
driver result, not a test skip. `scena capabilities --json` is intentionally
`static_no_device` and cannot validate the current machine. The headless probe
does not validate window/browser presentation; use the matching rendered
surface lane for that claim.

## Help or template discovery was treated as an error

Current builds return `scena.cli_help.v1` on stdout with exit 0 for both global
and per-command help, including `scena diff --help --json`. Discover templates
with `scena examples agent list`; do not provoke an unknown-template error.
Canonical template names use kebab-case. Older underscore aliases still run and
record their replacement in the generated manifest `notes`.

`scena diff` exits 0 for an unequal but valid comparison. Use `--exit-code`
only when CI should treat inequality as exit 1; the JSON report remains on
stdout in either mode.

## Converter progress corrupted JSON output

Use `scena-convert --json ...`. Current builds capture FBX2glTF stdout/stderr
inside the `scena.asset_conversion.v1` `diagnostics` array and emit exactly one
JSON document. Exit 2 means the conversion request was invalid; exit 1 means
the tool was unavailable or failed, and the same stdout report contains the
remedy and captured diagnostic lines. Use `--human` only when live plain-text
tool output is intentional.

## Browser rendering is unavailable

Check:

- browser support for WebGPU or WebGL2,
- secure context requirements for WebGPU,
- canvas creation,
- requested backend,
- capability report,
- console errors from asset fetching.

See [Browser and WASM](browser.md).

## Picking misses objects

Check:

- camera and viewport dimensions,
- cursor coordinate conversion,
- object visibility,
- layer masks,
- the current morph weights and skin bindings,
- singular transforms that collapse the target triangle,
- scene preparation after moving objects.

`pick_with_assets` evaluates morph targets before skinning, matching render
preparation. Missing or invalid deformation inputs return
`LookupError::InvalidSkinBinding` instead of testing the undeformed mesh.

Start with `examples/picking_selection_hover.rs`.

## Anchors or connectors do not align

Check:

- connector forward/up vectors,
- source units,
- coordinate-system conversion,
- left-handed versus right-handed data,
- whether the anchor belongs to the expected imported node.

Malformed marker transforms now fail during asset loading instead of becoming
identity. Read the exact extras path in `AssetError::Parse`. `forward` and `up`
must be present together, finite, nonzero, and nonparallel; quaternion rotations
must be normalized; scale must be finite and nonzero; and matrices must be
finite affine 4x4 transforms that decompose without shear.

See:

- [Place and connect objects](guides/place-and-connect-objects.md)
- [Authoring glTF anchors and connectors](guides/authoring-gltf-anchors-connectors.md)
- [Troubleshooting misplaced assets](guides/troubleshooting-misplaced-assets.md)
