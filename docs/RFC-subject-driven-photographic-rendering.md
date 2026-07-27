# RFC: Subject-driven photographic rendering

Status: Draft
Created: 2026-07-26
Scope: renderer, scene host, recipe schema, CLI, verification, documentation
Canonical charter: `docs/RFC-rust-3d-renderer.md`
Execution checklist: `docs/checklists/subject-driven-photo-rendering.md`

## Summary

`scena` should make the common camera-behavior/model-viewer path behave like a
camera, not like a pile of unrelated numeric knobs. The user should be able to
declare the subject and intent, then receive a well-exposed, well-framed,
focused, machine-verifiable image without hand-tuning camera distance, exposure
EV, focus distance, floor size, or background.

This RFC introduces a deterministic "virtual photographer" pipeline:

- a typed subject target declared in the recipe or CLI;
- a frame-bound subject observation produced from geometry and rendered
  attribution, not from color guessing;
- subject-aware exposure metering with exposure compensation rather than manual
  absolute EV;
- subject autofocus based on visible subject depth;
- deterministic composition/staging candidate selection for camera-behavior
  renders;
- a strict acceptance gate that rejects the failure mode that produced the demo
  hero silhouette.

The design is renderer-owned presentation logic. It does not add simulation,
robotics, physics, domain model semantics, or a hidden application loop.

## Motivation

The documented recipe path can build the demo hero, but the first attempts
showed the wrong product behavior:

- the subject could render as a dark silhouette while the render reported
  success;
- fixed `exposure_ev` values had to be guessed by humans;
- a later attempt improved the background but underexposed and shrank the
  subject;
- depth of field required a guessed `focus_distance`, even though framing had
  already solved the camera-to-subject relationship.

The renderer already owns most of the information needed to do better:

- `FramingOutcome` carries projected bounds and framing distance;
- semantic AOV capture can attribute visible pixels to scene entities;
- render quality checks already measure subject-level exposure and salience;
- auto-exposure already has presets and deterministic reports.

The missing product primitive is the bridge between those systems. Exposure,
focus, and verification should consume one canonical subject observation instead
of each subsystem re-deriving "the subject" with its own weaker heuristic.

## Goals

- One easy path for a product/model hero image:

  ```bash
  scena photo render machine.glb --out hero.png --report hero.json
  ```

- Equivalent recipe control for agents and applications:

  ```json
  {
    "photo": {
      "intent": "camera_behavior",
      "subject": { "kind": "import", "id": "machine" }
    }
  }
  ```

- No manual camera, fixed exposure, guessed focus distance, floor geometry, or
  background color required on the easy path.
- Explicit advanced controls remain available and deterministic.
- All automatic choices are reported in versioned JSON and can be reproduced by
  emitted recipe output.
- The first implementation is accepted only when a connector/camera-behavior
  fixture passes a strict visual-quality gate without hand-tuned overrides.

## Non-goals

- No domain-specific product recognition, CAD feature inference, robotics,
  simulation, physics, process logic, or gameplay behavior.
- No hidden render loop. The CLI may run bounded internal candidate renders for
  a single command, and a host may call the same planning API, but `render()`
  remains a draw of already-prepared state.
- No black-box generative image enhancement. The output remains a render of the
  authored scene.
- No promise that one global mean-luminance range fits every art direction.
  Product-hero fixtures may use specific luminance bands; the generic API
  reports measured quality and intent-specific acceptance.

## Architecture

The system has two layers.

Layer 1 is camera intelligence:

- resolve the subject;
- observe where it actually lands in the frame;
- meter exposure from subject-weighted scene luminance;
- solve focus from visible subject depth;
- report quality and fallbacks.

Layer 2 is photographer intelligence:

- choose camera/lens/view candidates;
- choose staging defaults such as environment, background, ground, and grid
  policy;
- score low-cost candidate views;
- select a final composition;
- run the same camera intelligence and acceptance checks.

Subject metering alone is not enough to make `scena` easy. A black, centered,
well-exposed object that fills only a quarter of the frame is still a failed
hero image. The product path needs both layers.

## Public Model

### Subject declaration

The authored contract is a target, not a computed rectangle:

```rust
pub struct SubjectSpec {
    pub target: SceneTargetSpec,
    pub fallback: SubjectFallbackPolicy,
}

pub enum SubjectFallbackPolicy {
    Error,
    AverageMeteringWithWarning,
}
```

`SceneTargetSpec` uses the same target grammar across metering, focus,
expectations, diagnostics, and photo planning. Whole-import targets must be
supported; rejecting whole imports in one verifier while accepting them in a
planner is not allowed.

Example recipe:

```json
{
  "render": {
    "auto_exposure": "product_studio",
    "metering": {
      "mode": "subject",
      "target": { "kind": "import", "id": "machine" },
      "fallback": "error"
    },
    "exposure_compensation_ev": 0.3
  }
}
```

### Subject observation

The computed bridge is frame-bound and explicitly staleable:

```rust
pub struct SubjectObservation {
    pub frame: CompositionFrameKey,
    pub subject: ResolvedSubject,
    pub world_bounds: Aabb,
    pub projected_bounds: ScreenRect,
    pub visible_bounds: ScreenRect,
    pub visible_pixel_count: u32,
    pub depth_percentiles: SubjectDepthPercentiles,
    pub coverage: SubjectCoverageMetrics,
}
```

The important distinction: `SubjectObservation` is not a loose rect stored on a
camera. It is tied to the scene, camera, renderer target, viewport, transform
revision, visibility revision, material revision, and render generation needed
to prove it describes the pixels being judged. Any mismatch becomes a
structured stale-observation result.

`projected_bounds` is useful for candidate planning and fallback. Exposure and
quality decisions should prefer visible attribution from rendered ID/depth data
when available, because an AABB can include empty background, hidden parts, or
occluded geometry.

### Metering modes

```rust
pub enum MeteringMode {
    Average,
    CenterWeighted,
    HighlightWeighted,
    Subject { target: SceneTargetSpec },
    Spot { rect: NormalizedRect },
}
```

Subject metering uses a weighted luminance distribution:

- subject-visible pixels receive full weight;
- surrounding pixels receive a smaller guard weight, initially `0.1`;
- highlight protection remains global so a bright background can limit the EV
  without dominating the subject midtone;
- overlays, labels, grids, and post-processing are excluded unless explicitly
  requested by a future intent.

The fallback color-difference heuristic is allowed only when no subject
observation exists and the caller explicitly permits fallback. Explicit
`mode:"subject"` with `fallback:"error"` must fail closed.

### Exposure compensation

`exposure_ev` remains a full manual override. Automatic exposure gains a
composable compensation term:

```json
{
  "render": {
    "auto_exposure": "product_studio",
    "exposure_compensation_ev": 0.3
  }
}
```

`exposure_compensation_ev` is valid only when auto exposure is active. The
reported final exposure is:

```text
final_ev = clamp(metered_ev + compensation_ev, min_ev, max_ev)
```

The exposure report must include:

- metering mode and target;
- metering domain;
- subject mean luminance;
- subject low/high clip fractions;
- global highlight limiter contribution;
- base metered EV;
- compensation EV;
- final EV;
- whether a clamp or fallback was applied;
- a suggested compensation when quality targets are missed.

Preset EV limits are safety rails. They must not be the load-bearing mechanism
that decides whether a dark subject is visible.

### Metering domain

The target state is scene-linear metering before tonemapping, display encoding,
bloom, FXAA, overlays, and final surface presentation. If a backend cannot
provide pre-tonemap scene-linear samples, it must report a degraded
`metering_domain` and the acceptance gate must decide whether that backend is
allowed for the claimed evidence.

The design does not require a full user-visible HDR post chain in the first
slice, but the metering path must not be silently coupled to already-exposed
LDR surface bytes.

### Subject focus

Manual `focus_distance` stays valid. The easy path uses subject focus:

```json
{
  "render": {
    "depth_of_field": {
      "focus": { "mode": "subject", "target": { "kind": "import", "id": "machine" } },
      "coverage": "all",
      "strength": "subtle"
    }
  }
}
```

Subject focus resolves from visible depth percentiles, not merely the center of
the world-space bounds. The default focal plane is the weighted median visible
depth. `coverage:"all"` chooses a depth-of-field radius/aperture policy that
keeps the subject depth span acceptably sharp; `coverage:"feature"` may target a
named anchor or feature point in a later RFC.

### Photo intent

`photo.intent` is a constraint policy, not a static preset that can contain
conflicting choices:

```json
{
  "photo": {
    "intent": "camera_behavior",
    "subject": {
      "target": { "kind": "import", "id": "machine" },
      "fallback": "error"
    },
    "composition": {
      "fill": { "min": 0.65, "preferred": 0.76, "max": 0.85 },
      "view": "auto"
    },
    "exposure": {
      "metering": "subject",
      "compensation_ev": 0.0,
      "fallback": "error"
    },
    "focus": { "mode": "subject", "coverage": "all" },
    "staging": {
      "style": "product_studio",
      "ground": "matte",
      "grid": false
    }
  }
}
```

The planner may choose existing renderer-owned helpers: camera framing,
three-quarter view presets, lens presets, studio lighting, bundled
environments, dark or light studio backgrounds, floor/grid policy, and
post-processing. It must not infer domain state or hide scene mutations from
the report.

### Candidate planning

The camera-behavior planner is deterministic:

1. Resolve the subject and world bounds.
2. Generate a bounded set of camera/lens/staging candidates from the intent.
3. Score cheap geometry/AOV observations for fill, centering, clipping,
   occlusion, floor proportion, and view informativeness.
4. Render low-resolution shaded candidates for material readability,
   background separation, reflection structure, and exposure feasibility.
5. Select the highest-scoring candidate with stable tie-breaking.
6. Render the final frame.
7. Verify the final frame against the intent gate.
8. Perform at most one bounded retry when the verifier suggests a deterministic
   compensation or fill adjustment.
9. Emit the chosen resolved recipe and full report.

Candidate count, deterministic seed, rejected candidates, scores, and selected
candidate ID are part of the `PhotoReport`, not hidden implementation details.

### CLI surface

The intended CLI shape is:

```bash
scena photo plan model.glb --out plan.json
scena photo render model.glb \
  --out hero.png \
  --report hero.json \
  --emit-recipe resolved.recipe.json
```

Recipe rendering also supports the same fields:

```bash
scena recipe render hero.recipe.json --gpu --verify --out hero.png
```

`photo plan` is for auditability and agents. `photo render` is the easy path.
Both surfaces use the same planner and reports.

### Reports

Add versioned JSON contracts:

- future `subject_observation.v1`;
- future `exposure_report.v1`;
- future `focus_report.v1`;
- future `photo_plan.v1`;
- `scena.photo_report.v1`.

The final render/introspection output should link to these reports or embed
them when requested. If an authored subject produces zero visible pixels, the
response must name the reason: unresolved target, hidden, outside viewport,
behind camera, clipped, occluded, too small, transparent/degraded, or stale
observation.

## Module Ownership

- `scene`: target specs, target resolution, subject definitions, bounds,
  projections, and frame-state keys.
- `scene_host::photo`: recipe/CLI orchestration, candidate generation, staging
  policy, candidate scoring, bounded retry, report assembly.
- `render`: exposure metering, focus application, luminance histograms, work
  metrics, backend-specific sample capture.
- `render::quality` and `scene_host::composition`: final acceptance metrics and
  subject/object-level visibility diagnostics.
- `diagnostics`: structured errors, help, capabilities, and report schemas.
- `bin/scena`: `photo plan`, `photo render`, CLI flags, command help, and
  stable output envelopes.
- `xtask doctor`: drift checks for schema/docs/guide/fixture/CI coverage and
  known silent-failure families.

`Renderer::render()` remains a rendering operation. It does not resolve recipe
strings, fetch assets, compile shaders, select camera candidates, or run an
application loop.

## Failure Policy

The easy path should be strict enough to earn trust:

- explicit subject metering with an unresolved subject is an error;
- stale subject observation is an error unless the caller requested fallback;
- fallback metering is reported with a warning and cannot claim strict
  camera-behavior release evidence;
- manual fixed exposure disables automatic exposure reports;
- a photo intent that cannot satisfy its acceptance gate exits nonzero and
  returns candidate diagnostics rather than writing a misleading success.

## Acceptance Gate

The first product gate is a connector/camera-behavior fixture rendered only through
the recipe or `scena photo render` surface.

Required input constraints:

- no manual camera pose;
- no manual `exposure_ev`;
- no manual floor geometry;
- no manual background color;
- no grid for the camera-behavior intent;
- subject declared as a whole import.

Required output constraints:

- `ok:true`;
- no warnings or fallbacks for subject resolution, metering, focus, or
  composition;
- visible subject fills most of the frame, initially 65-85 percent of width for
  this fixture;
- no subject clipping;
- subject luminance in the fixture-specific accepted band;
- strict low-clip fraction;
- steel/metal readability passes;
- floor/background do not dominate the frame;
- final `PhotoReport` names the selected candidate and why rejected candidates
  lost.

The same gate must reject known-bad mutations:

- average metering instead of subject metering;
- stale or shifted subject mask;
- wrong target;
- old product-studio EV cap behavior;
- metering from post-tonemap LDR output where a stricter backend is required;
- pulled-back camera with excess empty slab;
- wrong focus distance;
- missing reflection structure for steel.

## Rollout

1. Land the red acceptance gate and fixture matrix first.
2. Unify target resolution and add `SubjectSpec`.
3. Add frame-bound `SubjectObservation`.
4. Add subject-weighted metering and exposure compensation as opt-in.
5. Add subject focus.
6. Add `photo.intent`, `photo plan`, and `photo render` with the gate in the
   same slice.
7. Flip the easy `photo.intent` path to subject metering by default. Existing
   raw renderer and recipe behavior stays average/manual unless explicitly
   opted in or routed through `photo.intent`.
8. Update docs, schemas, examples, capabilities, troubleshooting, and doctor
   rules.
9. Run full release gates once at the final frozen checkpoint.

## Open Questions

- Which exact fixture becomes the first public camera-behavior gate:
  `connector_snap_assembly.glb`, the demo hero machine, or both?
- Should strict subject metering require semantic AOV support on every claimed
  backend, or allow projected-rect fallback with degraded evidence?
- What is the initial low-resolution candidate budget for the CLI default?
- Which quality bands are fixture-specific and which become generic
  `camera_behavior` intent defaults?
- How much of the future HDR post chain is required before subject metering can
  claim full backend parity?
