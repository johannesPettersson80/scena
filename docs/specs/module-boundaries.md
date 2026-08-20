# Module boundaries

Status: active architecture contract

The canonical ownership map is:

- `scene`: graph state, transforms, cameras, lights, imports, and revisions.
- `scene_host`: host-facing orchestration, composition checks, semantic AOV
  observation, recipe execution, and camera-behavior photo planning that composes
  `Scene`, `Assets`, and `Renderer` through explicit prepare/render/capture
  probes.
- `assets`: fetching, parsing, decode, cache, reload, and asset provenance.
- `geometry`: geometry descriptors and primitive construction.
- `material`: material descriptors and authored color/material state.
- `render`: preparation, backend resources, drawing, output, and readback.
- `animation`: clip sampling and animation state contracts.
- `controls`: reusable renderer-facing camera controls.
- `picking`: renderer-scene intersection and pick result contracts.
- `diagnostics`: typed errors, capability reports, and statistics.
- `platform`: native/browser host adapters only.
- `vocabulary`: stable public enumerations shared by schemas, the CLI, and API consumers.

No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`
is permitted. Resource work is explicit in prepare or a separately named
output-preparation operation.

Photo intent planning and candidate selection stay outside `render`. Host/CLI
setup may perform explicit bounded `prepare -> render -> capture` probes before
the final render, but `src/render/**` must not own photo-candidate planning,
photo-report schemas, exposure-retry loops, or intent selection.

Shared recipe target resolution stays centralized in `src/scene/recipe/target_resolution.rs`.
Subject metering, subject focus, recipe expectations, composition checks,
subject observations, and photo plan/render subject selection may wrap the
canonical resolver to add caller-specific error wording, but must not
reimplement import/node handle matching in separate modules.

## Host-owned convenience facade exceptions

`HeadlessGltfViewer` and `InteractiveGltfViewer` are the v1.0 host-owned convenience
facade exceptions. They compose `Scene`, `Assets`, and `Renderer`; they do not
move ownership out of those modules. Mutable accessors remain explicit escape hatches
and must preserve lifecycle invalidation.

These are the only current host-owned convenience facade exceptions.

## Large module allowlist

The architecture scanner recognizes the historically large facade owners
`src/assets.rs` and `src/viewer.rs` only through an explicit reviewed policy.
The subject-driven photo RFC temporarily allows the following oversized modules
to keep the release branch reviewable while the final feature proof is
integrated:

- `src/bin/scena/photo.rs`
- `src/bin/scena/recipe.rs`
- `src/bin/scena/recipe/quality/verification.rs`
- `src/bin/scena/recipe/verification.rs`
- `src/diagnostics/capabilities.rs`
- `src/render/exposure.rs`
- `src/render/prepare.rs`
- `src/render/prepare/primitives.rs`
- `src/scene/recipe/field_model.rs`
- `src/scene/recipe/validation/expectations/quality.rs`
- `src/scene/recipe/validation/photo.rs`
- `src/scene/recipe/validation/setup/render.rs`
- `src/scene_host/composition/subject.rs`
- `src/scene_host/photo.rs`

This is a split-debt allowlist, not a precedent for new catch-all modules. Each
entry remains bound to its current owner, and follow-up work should split these
by contract boundary before adding more behavior. New catch-all owners or size
exemptions require a documented architecture decision; file movement alone does
not establish ownership.

`render::prepare` and `render::prepare::primitives` are explicit exceptions for
the v1.10 release branch. Each owns one synchronous preparation boundary:
scene-to-prepared-scene orchestration and triangle primitive baking respectively.
Their focused preparation suite is the regression boundary while their internal
subsystems continue to be split by material, lighting, environment, stroke, and
geometry contracts. No new public API or unrelated renderer ownership may be
added to either exception.
