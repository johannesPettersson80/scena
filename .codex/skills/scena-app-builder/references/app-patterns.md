# App Patterns

Use these patterns to select the right recipe sections and verification checks.

## Model Viewer

Use imports or authored primitives, one camera, lights/environment, and optional
labels. Verify render introspection and framing. For browser delivery, run the
browser proof path.

## CAD Inspection

Use imported CAD/glTF or authored visual primitives, section boxes,
measurements, callouts, labels, and overlay-aware framing.

Verify:

- target part visible and not tiny;
- grounded parts satisfy `expect_grounded` when they must sit on a base plane;
- helper/wireframe/grid elements that must stay behind the inspected part
  satisfy `expect_helper_occluded`;
- section/cutaway visible when requested;
- measurement/callout labels visible, uncropped, and not crossed by leader or
  dimension lines;
- no CAD-kernel claims. DXF/DWG/B-rep parsing and constraints belong to the
  host or a geometry kernel.

## Digital Twin

Represent host state as visual patches, named states, animations, tints,
visibility, labels, and timeline samples. The host owns telemetry and time.

Verify:

- each named state renders;
- sampled times change the intended target;
- equipment expected to sit on the floor satisfies `expect_grounded`;
- floor grids and helper wires expected behind equipment satisfy
  `expect_helper_occluded`;
- warning/fault colors match expectations;
- no hidden autonomous loop inside scena.

## Product Configurator

Use material variants, product options, visibility toggles, tints, and camera
bookmarks. The host owns pricing, SKU, inventory, and compatibility rules.

Verify:

- chosen option is active;
- intended node color/material matches a swatch or material expectation;
- hidden accessories are not visible;
- missing material/texture fallbacks fail closed.

## Industrial Dashboard

Use authored primitives, labels, callouts, state colors, and simple timelines.
The host owns telemetry, alarms, units, and process semantics.

Verify:

- active alarm/status component visible;
- expected status color matches;
- machines or bars intended to be grounded satisfy `expect_grounded`;
- labels are visible, not cropped, and not crossed by helper lines;
- helper grid or status-line overlays expected behind the machines satisfy
  `expect_helper_occluded`;
- render stays within the requested viewport.

## Documentation Renderer

Use deterministic camera/framing, labels, measurements, callouts, and capture
artifacts. Prefer static recipes that render in CI.

Verify:

- output PNG and descriptor exist;
- overlays are not cropped and labels are not crossed by leader/dimension lines;
- subject fill is above the documented floor;
- no warning is present when `expect_no_warnings` is requested.

## Interaction Viewer

Use synthetic interaction verification for pick, hover, and select. Do not
claim interaction works from a static render.

Verify:

- pick at coordinates returns the expected handle;
- hover/select changes expected state;
- CSS and physical pixel coordinates are not confused.

## Guided Tour

Use camera bookmarks, named visual states, callouts, exploded view helpers, and
timeline samples. The host advances time.

Verify:

- every sampled step renders;
- camera target remains framed;
- labels/callouts remain visible and clear of crossing line overlays.
