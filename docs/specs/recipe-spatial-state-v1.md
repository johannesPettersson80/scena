# Recipe spatial features and named states v1

Status: accepted implementation contract under the canonical renderer RFC

This contract un-defers the renderer-owned `anchors`, `connectors`, `bounds`,
and `named_states` sections of `scena.scene_recipe.v1`. It does not add workflow
sequencing, simulation, robotics, or domain behavior. A recipe remains one
declarative scene snapshot.

## Persistent identity and targets

Every row has a caller-owned `id` unique across the complete recipe ID
namespace. Runtime slot-map keys and SceneHost handles remain build-scoped.
`scena.scene_recipe_build.v1` maps each persistent feature ID to its resolved
target and outcome without presenting a runtime handle as persistence.

Spatial targets are closed tagged objects:

- `{ "kind":"node", "id":"machine" }` addresses an authored recipe node,
  instance-set node, particle-set node, or label node.
- `{ "kind":"import_root", "id":"part" }` addresses the import's unique
  primary root and fails when the import has no root.
- `{ "kind":"import_node", "import":"part", "path":"Root/Flange" }`
  addresses an import-local exact path.

Missing, stale, skipped-optional, or ambiguous targets are build errors. There
is no first-match fallback.

## Units and coordinate spaces

All authored recipe positions, bounds, offsets, snap tolerances, and clearances
are local or world scene meters after the C05 import conversion boundary.
Authored frames use glTF Y-up right-handed axes. An `import` anchor or connector
alias preserves the imported feature's source-unit and source-coordinate
metadata, while its connection transform is already converted to scene space.
Recipe fields never apply a second per-value unit conversion.

## Anchors

An anchor has `id` and a tagged `source`:

- `authored`: `target` plus an optional local `transform`, `tags`, and `label`;
- `import`: exact `import` ID plus imported anchor `name`.

Authored anchors become `scene::AnchorFrame`s. Imported anchors are aliased
through `AnchorFrame::from_import_anchor`; absent or ambiguous names fail.

## Connectors and mating

A connector has `id` and a tagged `source`:

- `authored`: `target`, optional local `transform`, `connector_kind`,
  `allowed_mates`, `tags`, non-negative scene-meter `snap_tolerance` and
  `clearance_hint`, `roll_policy`, and `polarity`;
- `import`: exact `import` ID plus imported connector `name`.

An optional `mate` names another persistent connector ID. All connector aliases
are resolved before mating, so declaration order does not matter. V1 uses the
existing `Scene::connect_by_key` compatibility, handedness, source metadata,
snap-tolerance, roll, alignment, parenting, and finite-transform validation.
The source connector's node moves; the target stays fixed. Failure produces a
structured recipe diagnostic and no success row.

## Bounds

A bounds row has `id`, `target`, and `source`:

- `computed` resolves the target's current renderer/asset-backed local or
  combined import bounds;
- `imported` requires an import target and records the converted imported
  bounds provenance;
- `authored` requires finite `min` and `max` scene-meter vectors with
  `min <= max` and assigns that local AABB to a non-renderable target node.

Authored bounds cannot replace geometry- or asset-owned bounds. Build-manifest
rows report source, persistent target, finite min/max, and scene-meter units.

## Named states

A named state contains persistent-target `transforms`, `tints`, and
`visibility` rows. V1 intentionally excludes camera state, animation time,
selection/hover, labels, material variants, and section boxes; unknown fields
fail rather than being dropped.

One optional `inherits` name forms single inheritance. Parents may be declared
later. Cycles and missing parents are errors. Child entries override parent
entries for the same channel and persistent target; other entries are inherited
in deterministic target order. At most one state may set `active:true`.

Build resolves persistent targets to a `VisualPatchV1`, stores every resolved
state in SceneHost, and applies the active state once after recipe authoring and
connector mating. Missing targets fail the build. Named states do not tick,
seek, start, stop, or inherit animation time. V1 rejects transform entries that
target nodes driven by a recipe animation, avoiding an implicit precedence rule.
The host may explicitly apply a stored state later; subsequent host animation
ticks remain host-owned.

## Proof requirements

- Validation and round-trip tests for every tagged source/target and malformed
  or cyclic form.
- Manifest mapping for every anchor, connector/mate, bounds row, and state.
- Known-bad compatibility, snap, bounds, inheritance, missing-target, and
  animated-target fixtures.
- Deterministic rendered proof that connector mating and an active named state
  change only the declared placement/appearance.
- Doctor coverage pinning owner modules, fail-closed diagnostics, stable schema
  fields, and the focused proof.
