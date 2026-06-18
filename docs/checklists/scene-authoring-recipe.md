# Scene Authoring Recipe — Agent-Facing "Build From Scratch"

The agent surface (CLI + stable JSON) can today **import** glTF, place/transform
imports, add overlays (section box, measurements, callouts, exploded view),
render, introspect, diagnose, verify, and repair. It cannot **author a new scene
from primitives** through JSON. This checklist closes that one missing layer so
an LLM can build a useful CAD / twin / dashboard / configurator scene from
nothing and continue the verify loop on the objects it created.

Builds on the findings in
[`application-builder-example-findings.md`](application-builder-example-findings.md).

## Foundation already in place (verified)

These were implemented and independently verified (source + artifacts) and are
the base this layer builds on:

- `scene_recipe.v1` import + overlay directives (section box, measurement,
  callout, exploded view) with fail-closed validation, routed through real
  SceneHost operations, plus real (no longer deferred) `scena examples agent`
  CAD/documentation templates.
- `render_introspection.v1` fail-closed frame proof; `visibility_diagnosis.v1`;
  `verify appearance|animation|interaction`; `repair --from`; `place` verbs;
  schema discovery (`schema list`/`get`).
- `frame_all_with_overlays` (over-zoom margin regression fixed, lab fill-guard);
  `HeadlessGltfViewer::render_introspection`.
- `browser-proof`: `--dry-run` emits a contract for inspection only; the real
  `scene-host` / `m6` paths spawn the wasm-pack + Playwright lane and report
  `passed`/`failed` from the actual exit code. **Browser-visible claims require
  the real path, not the dry-run contract.**
- Stable u64 handles, fail-closed exit codes (0/1/2), byte-stable rounded JSON,
  `BTreeMap` ordering, fixture + `schema_catalog` + doctor `FIXTURES` lockstep.

## Load-bearing principle: build a scene, return a typed manifest

The recipe must **build a scene and return an id→handle manifest**. Without it
the LLM authors objects it cannot then reference for verify/diagnose/patch.

- `build_recipe(recipe, policy) -> { scene, assets, manifest, diagnostics }`.
- Emits a `scene_recipe_build` manifest (a new v1 stable contract). It is
  **typed, not a flat id→handle map** — because geometry and material ids are
  asset *resources*, not targetable SceneHost nodes:
  - `nodes`, `cameras`, `lights` — **targetable**; each exposes a stable public
    `handle` usable by verify / diagnose / repair / overlays.
  - `imports` — each entry is
    `{ id, import_handle, root_handles: [...], primary_root, nodes_by_path }`: an
    import is an import handle plus one or more root node handles (per
    `SceneHostCore::import_roots`); `primary_root` is `root_handles[0]`. Imported
    **child** nodes are addressable by the deterministic id `"<import_id>:/<path>"`
    (resolved via `SceneHostCore::node_handle`), so
    `target:{kind:"node", id:"machine:/Assembly/Bolt3"}` works — not just roots.
  - `geometries`, `materials` — **resources**; carry their recipe `id` and a
    summary (kind, vertex/index counts), **no node handle**.
- Authored node ids and imported node ids share the *same handle space and the
  same manifest*, so verification, overlays, cameras, and placement targets use
  the same target-id vocabulary for authored and imported nodes. Do not keep a
  separate import-roots path.

### Fail-closed semantics (LLM-trust contract)

- `ok: true` means **every requested renderable directive was created.**
- A skipped or failed *required* object — unknown geometry/material ref, an
  `unsupported_feature` on a requested node, or a policy rejection of a
  renderable — sets `ok: false` and fails the build/render.
- `warnings` are reserved for **unused resources** (a declared geometry/material
  no node references) and **optional fallbacks** — never for a dropped
  renderable. `skipped` lists only such non-required items.
- **Requested render content that fails is an error, not a silent fallback.** A
  material texture URI that fails to load fails the build (`ok: false`) unless
  the recipe explicitly marks that input optional (e.g. `"optional": true` on the
  texture slot), in which case the fallback is reported as a `warning`. An LLM
  must never receive a "successful" flat-color render in place of a texture it
  requested.

## Design decisions (locked)

1. **Typed manifest, one handle space** (above): targetable objects expose
   handles; resources do not; authored and imported nodes are unified.
2. **One transform/placement vocabulary.** A single `TransformSpecV1` —
   `raw` (escape hatch) ∪ `translation/scale/rotation_degrees` ∪ placement verbs
   (`center/ground/fit_to_size/look_at/place_on/align_to_anchor`) — used both as
   a node transform and as `scena place` input, backed by the existing placement
   implementation. `rotation_degrees` uses a pinned, documented Euler order.
3. **Deterministic resolution order.** Build geometries → materials → nodes, then
   resolve placement against already-built targets; detect forward refs / cycles
   and fail closed.
4. **Overlay targets must include authored nodes.** Section box, callout,
   measurement, and exploded-view targets take a union
   `{ kind: "node" | "import" | "world", id | position }`, where `"node"`
   accepts any authored or imported node id from the manifest, `"import"` targets
   the whole import subtree (all `root_handles`), and `"world"` a position. To
   target a single node inside an import, use `"node"` with the deterministic id
   `"<import_id>:/<path>"` (resolved via `node_handle`).
5. **Expectations are sugar over existing verify contracts.** `expect_*` compiles
   to appearance / interaction / render-introspection — no second engine.
6. **Resource + path policy is the untrusted-input boundary** (LLM JSON is
   untrusted; scena is a load-bearing dependency downstream), enforced inside the
   executor from the first executable slice.
7. **Thin vertical slice first**, then widen vocabulary.
8. **Starter snippets are real recipes** that round-trip through the executor and
   pass gates.
9. **Renderer-only boundary holds.** Primitives, overlays, dimensions, imported
   assets, and simple visual approximations are in scope. DXF/DWG parsing,
   constraints, exact B-rep, and feature recognition are explicit non-goals for a
   host/CAD layer.
10. **Globally unique ids.** Every recipe `id` — across **every id-bearing
    directive**: colors, geometries, materials, nodes, lights, cameras, imports,
    animations, fonts, skins, morphs, particle sets — must be unique across the
    whole document; duplicates fail closed. Simpler and safer for an LLM than
    per-type namespaces.

### Module ownership (SOLID/KISS, AGENTS.md)

- `scene::recipe::types` — recipe + manifest + policy structs.
- `scene::recipe::validation` (+ `validation/overlays.rs`,
  `validation/suggestions.rs`) — per-directive fail-closed validation and
  did-you-mean.
- `scene::recipe::build` (new) — the executor (`build_recipe`); **move the
  existing `apply_recipe_*` logic out of `src/bin/scena/input.rs`** into it so
  the CLI stays a thin adapter (input.rs is already trending toward a catch-all).
- `src/bin/scena/recipe.rs` (new) — CLI adapter for `scena recipe render
  --introspect --verify`. No catch-all manager/engine type.

## Proposed contract sketch (Slice 1 target)

A minimal valid authored recipe (additive fields on `scene_recipe.v1`):

```json
{
  "schema": "scena.scene_recipe.v1",
  "colors": { "plate_blue": "#3A7BD5" },
  "geometries": [
    { "id": "plate_geo", "primitive": { "kind": "box", "size": [0.12, 0.06, 0.004] } }
  ],
  "materials": [
    { "id": "plate_mat", "kind": "pbr_metallic_roughness",
      "base_color": "plate_blue", "metallic": 0.1, "roughness": 0.6 }
  ],
  "nodes": [
    { "id": "plate", "geometry": "plate_geo", "material": "plate_mat",
      "name": "CAD plate", "transform": { "kind": "center" } }
  ],
  "cameras": [
    { "id": "main", "kind": "perspective", "fov_degrees": 40, "active": true,
      "transform": { "kind": "look_at", "eye": [0.2, 0.15, 0.2], "target": "plate" } }
  ],
  "capture": { "width": 320, "height": 220 }
}
```

Returned `scene_recipe_build` manifest (typed; resources vs targetable handles):

```json
{
  "schema": "scena.scene_recipe_build.v1",
  "ok": true,
  "geometries": [ { "id": "plate_geo", "kind": "box", "vertex_count": 24, "index_count": 36 } ],
  "materials":  [ { "id": "plate_mat", "kind": "pbr_metallic_roughness" } ],
  "nodes":      [ { "id": "plate", "handle": 4294967297, "parent": 4294967296, "name": "CAD plate" } ],
  "cameras":    [ { "id": "main", "handle": 4294967298, "active": true } ],
  "lights":     [],
  "imports":    [],
  "warnings":   [],
  "skipped":    [],
  "diagnostics": []
}
```

**Schema naming.** `scene_recipe_build` and `recipe_render_result` (Slice 5) are
written here without the `scena.` prefix only so this checklist passes doctor's
markdown reference check before the contracts exist. When each lands, give it the
canonical `scena.`-namespaced `vN` schema id and add it to `schema_catalog` +
doctor `FIXTURES`. Do not freeze the placeholder form.

### Per-directive field sketches

- **colors**: map `id -> "#RRGGBB" | "srgb8(r,g,b)" | "linear(r,g,b)" | "kelvin(k)" | <named>`.
- **geometries[]**: `id`; one of `primitive { kind: box|sphere|cylinder|plane|line|polyline|arrow|grid|axes|cone|torus|disc|wedge, <params> }` or `mesh { topology: triangles|lines, positions[], normals[], indices[], colors?[], uvs?[] }`.
- **materials[]**: `id`; `kind: unlit|pbr_metallic_roughness|line|wireframe|edge`; `base_color` (color ref); `metallic`, `roughness`, `emissive`, `emissive_strength`, `alpha_mode: opaque|mask{cutoff}|blend`, `double_sided`, `stroke_width_px`, advanced-PBR factor/texture fields (Slice 9), texture slots `{ uri, transform?, optional? }` (uri under path policy).
- **fonts[]** (Slice 11): `id`; `uri` (TrueType/asset font, under path policy); `optional?`. Referenced by a label's `font` id.
- **nodes[]**: `id`; `geometry?`, `material?` (resource refs) or `empty: true`; `parent?` (node id, default root); `transform: TransformSpec`; `name?`, `tags?[]`, `visible?`, `layer_mask?`, `render_group?`, `tint?`.
- **instance_sets[]** (Slice 6): `id`; `geometry`; `material`; `parent?`; root
  `transform?`; `instances[]` with stable `id`, `transform?`, opaque `tint?`,
  and `visible?`.
- **labels[]** (Slice 6): `id`; `text`; `parent?`; `transform?`; opaque
  `color?`; opaque `background?`; opaque `halo?`; `size_px?`.
- **clipping_planes[]** (Slice 6): `id`; finite non-zero `normal`; finite
  `distance`; `active?` (default true), bounded by renderer
  `max_clipping_planes`.
- **lights[]**: `id`; `kind: directional|point|spot` or `preset: sun|key|fill|rim|softbox|...`; `color?`, intensity (`illuminance_lux`|`intensity_candela`), `range?`, cone angles (spot); `transform: TransformSpec`.
- **cameras[]**: `id`; `kind: perspective|orthographic`; `fov_degrees?`/ortho extents; `aspect?`, `depth_range?`; `active?`; `transform: TransformSpec`.
- **TransformSpec** (exact wire variants):
  - `{ "kind": "trs", "translation": [x,y,z], "rotation_degrees": [rx,ry,rz], "scale": [sx,sy,sz] }`
  - `{ "kind": "raw", "translation": [x,y,z], "rotation": [x,y,z,w], "scale": [sx,sy,sz] }`
  - `{ "kind": "look_at", "eye": [x,y,z], "target": "<node id>" | [x,y,z], "up": [0,1,0] }`
  - `{ "kind": "center" }` | `{ "kind": "ground", "plane_y": 0.0 }` | `{ "kind": "fit_to_size", "size": [w,h,d] }`
  - `{ "kind": "place_on", "target": "<node id>", "offset": [x,y,z] }`
  - `{ "kind": "align_to_anchor", "anchor": "<import id>.<anchor name>" }`
- **scene** (Slice 4): `background`, `environment`, `grid`.
- **render** (Slice 4): `profile`, `quality`, `anti_aliasing`, `bloom`, `ssao`, `exposure_ev`, `tonemapper`.
- **overlays** (section_box/measurements/callouts/exploded_view): existing fields + `target: { kind: node|import|world, id|position }`.
- **expect** (Slice 5): `expect_visible`, `expect_color { target, swatch, tolerance? }`, `expect_bbox_fit { min?, max? }`, `expect_pick { x, y, target }`, `expect_no_warnings`.

### Coordinate & rotation convention (pin exactly)

- Right-handed, **Y-up** (`GltfYUpRightHanded`); linear units are **meters**.
  Cameras/lights face local **−Z**, up **+Y** (matches `Transform::looking_at`).
- `rotation_degrees: [rx, ry, rz]` (degrees) is computed by **exactly calling**
  `Transform::default().rotate_x_deg(rx).rotate_y_deg(ry).rotate_z_deg(rz)`. The
  contract is that literal call, not a re-described Euler convention. Pin it with
  a non-commuting regression test (e.g. `[90, 90, 0]` ≠ `[0, 90, 90]`) asserting
  the exact resulting quaternion or a rotated basis vector.
- TRS composition (local→world and parent→child) is **scale → rotate →
  translate**, matching `Transform::compose` (parent ∘ child).
- `raw.rotation` is `[x, y, z, w]` (glam order), normalized on ingest.
- Self-placement verbs operate on the node's **world-space bounds** (local
  geometry bounds ∘ resolved world transform), resolved after parents/targets, so
  parent composition is already applied. Pivots, matching `src/scene/placement.rs`:
  `center` moves the bounds **center** to the target point; `ground` moves the
  bounds **min Y** (bottom) to `plane_y`; `fit_to_size` scales the **extent** into
  `[min, max]` **about the node's own origin (model origin), not the bbox
  center** (matches `src/scene/placement.rs:178`) — compose with `center` when
  bbox-centered scaling is wanted. They adjust only the node's own local transform
  (translation for center/ground, uniform scale for fit_to_size).

## Tier A — sequenced implementation slices

### Slice 0 — Executor, manifest, and policy (foundation)
- [x] `build_recipe(recipe, policy) -> { scene, assets, manifest, diagnostics }`.
- [x] **Pre-register every planned root key as `unsupported_feature` first.**
      Before any slice lands, add `colors`, `geometries`, `materials`, `nodes`,
      `cameras`, `lights`, `scene`, `render`, `expect`, `animations`, `fonts`,
      `skins`, `morphs`, `particles` to `UNSUPPORTED_SECTION_FIELDS`
      (`src/scene/recipe/validation/suggestions.rs:67`) and the future-section
      test (`tests/scene_recipe_contracts.rs:53`), so a pre-slice recipe gets
      `unsupported_feature` (planned), not a generic `unknown_field` (typo) guess.
      Each slice then MOVES its key from `UNSUPPORTED_SECTION_FIELDS` to
      `ROOT_FIELDS` (reconcile the existing `primitives` name vs the recipe's
      `geometries`).
- [x] `scene_recipe_build` typed manifest (new v1 contract): `geometries`,
      `materials` (resources, id + summary), `nodes`, `imports`, `cameras`,
      `lights` (targetable, id + `handle`), `skipped`, `diagnostics`,
      `ok`.
- [x] `RecipeBuildPolicy` — **operator/CLI config + named profiles, NOT an
      author-facing wire contract** (the untrusted recipe must not set its own
      limits; so it gets no schema / fixture / doctor pin). Enforced in the
      executor, fail-closed, structured
      diagnostics: `max_vertices`, `max_indices`, `max_nodes`, `max_instances`,
      `max_particles`, `max_animations`, `max_animation_channels`,
      `max_animation_keyframes`, `max_materials`, `max_textures`,
      `max_texture_bytes`, `max_image_dimension`, `max_output_pixels`,
      `fetch_byte_limit`, `max_recipe_bytes`, `allowed_uri_roots`,
      `allowed_uri_schemes`, `allow_network` (default false). Canonicalize
      paths; reject `..`/symlink traversal outside roots. No timeout knob is
      exposed until the asset-loading seam can enforce it.
- [x] Default limits (implementation may tune, but must ship + document defaults;
      do not leave unbounded):

      | field | default | field | default |
      |---|---|---|---|
      | `max_vertices` | 2_000_000 | `max_image_dimension` | 8192 |
      | `max_indices` | 6_000_000 | `max_output_pixels` | 16_777_216 (4096²) |
      | `max_nodes` | 10_000 | `fetch_byte_limit` | 64 MiB |
      | `max_instances` | 100_000 | `allow_network` | `false` |
      | `max_particles` | 100_000 | `max_recipe_bytes` | 8 MiB |
      | `max_animations` | 4_000 | `allowed_uri_schemes` | `["file"]` |
      | `max_animation_channels` | 100_000 | `allowed_uri_roots` | current working dir by default; caller-configurable |
      | `max_animation_keyframes` | 2_000_000 | `max_materials` | 2_000 |
      | `max_textures` | 256 | - | - |
      | `max_texture_bytes` | 64 MiB | - | - |

- [x] Defaults are **documented defaults with named override profiles** (the
      `testing()` profile uses the safe defaults; callers can construct higher
      memory or `http(s)` fetch policies explicitly);
      `allow_network=false` + `["file"]` stay the safe CLI defaults.
- [x] **Per-resource `optional` flag.** Imports default to required
      (load failure ⇒ `ok:false`); only explicit `"optional": true` downgrades to
      a warning + skipped import. Texture/font optional flags land with their
      recipe sections. Do not rely on current lenient asset behavior.
- [x] Proof: policy rejects oversize / out-of-root / wrong-scheme / network
      inputs; manifest round-trips byte-stable.

### Slice 1 — Thin authored slice (proves the loop)
- [x] **Allow authored-only recipes.** Relax validation so `imports` may be
      empty when `nodes` contains authored renderables. Today
      `src/scene/recipe/validation.rs:187` errors on empty `imports`, so an
      authored-only Slice 1 recipe would fail by design.
- [x] `colors` + one geometry primitive + one material + one node + a camera
      (per the sketch above).
- [x] `scena render --introspect` over a built-from-scratch recipe.
- [x] Proof: end-to-end test — author → build → manifest `handle`s → render
      introspection `ok`; golden build-manifest fixture.

### Slice 2 — Widen the authoring vocabulary
- [x] Geometry: all primitives + custom mesh (positions/normals/indices/topology,
      optional colors/UVs).
- [x] Materials: full params + texture slots (loaded under path policy).
- [x] Lights: directional/point/spot + presets.
- [x] Node attributes (`name/tags/visible/layer_mask/render_group/tint`) +
      hierarchy.
- [x] Overlay `target` union extended to authored node ids.
- [x] Proof: per-directive validation tests + a multi-node authored scene that
      renders and inspects; a callout/section-box targeting an authored node.

### Slice 3 — Transform / placement
- [x] `TransformSpecV1` (raw + TRS + placement verbs) reused for node transforms
      and `scena place`, backed by the existing placement impl.
- [x] Executor resolution order + forward-ref/cycle detection; pinned Euler order.
- [x] Proof: `place_on` an authored id and `align_to_anchor` an import anchor
      resolve; a cycle fails closed.

### Slice 4 — Scene / render setup
- [x] `scene`: background, environment/IBL, grid.
- [x] `render`: profile, quality, anti-aliasing, bloom, ssao, exposure,
      tonemapper.
- [x] Proof: two settings produce observably different introspection / pixels
      (no inert knobs).

### Slice 5 — Verification expectations
- [x] `expect_*` compiling to the appearance / interaction / render-introspection
      contracts.
- [x] `scena recipe render --introspect --verify` → build + render + verify in one
      fail-closed report: a new cataloged `recipe_render_result` contract nesting
      `{ build, capture, introspection, verification }`, with top-level `ok` true
      only when build/introspection/verification are ok and a capture exists.
      (New contract → its own fixture + catalog entry + doctor pin; see
      schema-naming note.)
- [x] Proof: a recipe whose color/pick/fit expectations pass, and a negative
      recipe failing each with a structured reason.

### Slice 6 — Instancing, labels, clipping planes
- [x] Instance sets (per-instance transform/tint/visibility); free-standing
      labels (`LabelDesc`); arbitrary clipping planes.

### Slice 7 — Starter snippets
- [x] `scena examples agent get <name>`: `primitive_scene`, `cad_plate`,
      `dashboard_bars`, `machine_state_viewer`, `product_configurator` — real
      recipes that round-trip through `build_recipe` and pass `validate-recipe` +
      render introspection in a test.

## Slices 8–13 — renderer capabilities (build the Rust API, then expose)

These build the missing Rust capability **and** expose it through the recipe in
the same slice. Sequenced after the core authoring loop, but all required — none
deferred. (Former Tier B findings #14→Slice 8, #15→9, #16→10, #17→11; #18 split
into Slice 12 skin/morph authoring + Slice 13 particle rendering.)

### Slice 8 — Keyframe animation authoring (#14)
- [x] **Add the missing public authoring seam.** `AnimationClip`/`AnimationChannel`
      are public (`src/animation.rs`) but `AnimationClipKey::fresh()` is
      `pub(crate)` (`src/animation.rs:101`) and `Scene::create_animation_mixer`
      only consumes imported clips (`src/scene/mixers.rs:26`). Add a public
      authored-clip API (e.g. `Scene::add_animation_clip` /
      `create_authored_animation_mixer`) so a caller can mint a clip and play it.
- [x] Recipe `animations`: `[{ id, duration, channels: [{ target, path:
      translation|rotation|scale|weights, interpolation, times[], values[] }] }]`,
      bound to authored/imported node ids. `duration` must cover the largest
      keyframe time. `weights` channels require a morph-capable target with
      exactly one value component per morph target; non-morph imported targets
      fail closed instead of accepting an inert channel.
- [x] Recipe animation payloads are under `RecipeBuildPolicy`: `max_animations`,
      `max_animation_channels`, and aggregate `max_animation_keyframes` are
      enforced during validation and again before mixer creation.
- [x] Proof: author a clip, seek, and confirm the node moved via
      `verify animation --expect-translations`; fail-closed on non-finite times,
      mismatched times/values lengths, keyframe times beyond duration, over-cap
      keyframe payloads, imported non-morph weight targets, or unknown target.

### Slice 9 — Advanced PBR in the recipe (#15)
- [x] The public setters **already exist** (`with_clearcoat_factor`,
      `with_sheen_color_factor`, `with_anisotropy_strength_factor`,
      `with_iridescence_factor`, `with_transmission_factor`, + texture variants in
      `src/material/extensions.rs`). The task is **recipe mapping**, not new
      builders: expose these fields in the recipe `materials` directive.
- [x] **Validate fail-closed BEFORE the setters sanitize.** Those setters clamp
      (`clamp_unit_or`, `finite_or`, …), so recipe validation must reject
      out-of-range / non-finite values up front — otherwise the LLM gets a
      "success" that silently differs from what it requested.
- [x] Proof: each exposed field **changes rendered pixels on the headless-GPU
      path** vs a baseline (a real render diff, not a stored-but-inert field), and
      a negative recipe is rejected at validation.

### Slice 10 — Primitive coverage (#16)
- [x] Add the committed primitive set to `GeometryDesc` (e.g. cone, torus, disc,
      wedge) with deterministic tessellation; recipe `primitive` kinds.
- [x] Proof: each primitive asserts **deterministic vertex/index counts, finite
      bounds, and projected silhouette / bbox / normal-facing evidence** — not a
      bare "renders non-empty".

### Slice 11 — Real fonts (#17)
**Major slice, not a small extension** — labels now support both the embedded
5×7 bitmap path and loaded TrueType/OpenType faces.
- [x] TrueType/asset font loading: parsed font metrics + glyph raster caching + font asset
      ownership; `LabelDesc` font selection; recipe label `font` ref under path
      policy.
- [x] **Pin font scope explicitly:** basic Latin glyph metrics + kerning pairs +
      hard-edged glyph-cell shapes are **in**; antialiasing is **out** until the
      transparent billboard path exists; complex-script shaping (Arabic/Indic,
      bidi, ligature substitution) is **out** for this slice and must fail closed
      with a clear `unsupported_feature`, not render garbage.
- [x] Proof: text rendered with a loaded font produces distinct hard-edged glyph
      cells vs the bitmap path (pixel proof); fail-closed on missing/oversize and
      present-but-corrupt fonts under policy.
      Evidence: `label_desc_truetype_font_changes_metrics_and_rendered_coverage`,
      `label_desc_truetype_rejects_complex_script_text`, and
      `scene_recipe_slice11_fonts_validate_build_render_and_fail_closed`.

### Slice 12 — Skeletal / morph authoring (#18a)
Larger than "expose existing types": today skin binding is read-only / import-
oriented (`src/scene/skinning.rs` exposes `skin_binding`/`skin_matrices`
accessors only).
- [x] **Add the missing public skin-binding authoring seam** so recipe ids can
      bind joints, inverse-bind matrices, geometry vertex weights, and morph
      targets/weights deterministically (the same render data scena already plays
      from glTF).
- [x] Recipe directives for authored skin + morph; this is where authored
      morph-weight animation (deferred from Slice 8) becomes valid.
- [x] Document the current lighting-normal scope: authored/imported skin and
      morph deformation is position-correct for rendered silhouettes, while
      morph normals are not authored/deformed and skinned normals use the joint
      direction transform rather than an inverse-transpose normal matrix for
      non-uniform joint scale.
- [x] Proof: compare the **undeformed vs deformed pose** (pixel/bounds change),
      plus a **negative test that fails if joints/weights/morph deltas are
      ignored** — introspection alone can pass on an ignored deformation;
      fail-closed on malformed joints/weights/targets.
      Evidence:
      `scene_recipe_slice12_skin_morph_authoring_deforms_rendered_output_and_fails_closed`
      + `scene_recipe_slice12_skin_morph_authoring_changes_headless_gpu_silhouette`
      + `scene_recipe_slice12_skin_only_changes_headless_gpu_silhouette`
      + `scene_recipe_slice12_joint_animation_rebakes_headless_gpu_vertices`
      + `scene_recipe_slice12_authored_morph_weight_animation_changes_rendered_output`
      and `scene_recipe_build_manifest_golden_matches_executor_for_stable_recipe`.

### Slice 13 — Particle / point-sprite rendering (#18b)
A **new render path** — grep finds no particle/point-sprite rendering today.
- [x] New GPU/CPU-visible primitive class: host-supplied particle buffer
      (position / opaque color / size / rotation) rendered as camera-facing
      screen-sized sprites, with bounds, capture proof, and an explicit
      picking/visibility policy decision. The current path bakes sprite quads
      during prepare; a shader-instanced particle path is a future performance
      improvement, not a claimed Slice 13 capability.
- [x] Recipe directive for a static or host-driven particle set.
- [x] Proof: assert expected **per-particle color / size / rotation /
      screen-position / depth** behavior, not just non-empty pixels; fail-closed
      on malformed buffers and translucent Rust particle colors until a real
      transparent particle path exists.
- [x] **Renderer boundary (the one carve-out):** time-stepped particle
      *simulation* (emitter physics / lifetime / velocity integration over time)
      stays host-side per AGENTS.md — scena **renders host-supplied particle
      state, it does not run the sim loop.** Flipping this is an RFC/charter
      change, not part of this checklist.
      Evidence:
      `scene_recipe_slice13_particles_render_per_particle_output_and_fail_closed`
      + `scene_recipe_slice13_particles_change_headless_gpu_pixels_by_color_size_position_and_depth`
      and `scene_recipe_build_manifest_golden_matches_executor_for_stable_recipe`.

## Tier C — plumbing every new directive needs

- [x] **Fail-closed validation per directive** (reject bad refs, non-finite
      values, out-of-range params), per the semantics above — and reject
      out-of-range values **before** any public setter silently clamps them
      (advanced PBR, Slice 9). Recipe-authored `transmission_texture` and
      `thickness_texture` are rejected until the GPU material path can sample
      them without exceeding WebGL2's fragment texture-unit floor. Scalar KHR
      volume fields `thickness_factor`, `attenuation_distance`, and
      `attenuation_color` stay in scope and require a coupled GPU volume-scene
      proof because they only affect pixels when transmission, thickness, and a
      finite attenuation distance are active together.
- [x] **GPU/browser proofs must FAIL when the backend is unavailable, not
      silently skip** — a skipped proof is not acceptance evidence (esp. the
      headless-GPU pixel diffs in Slices 9/13 and the browser proof).
- [x] **Catalog / doctor pins — correctly scoped.** Additive fields on the
      existing `scene_recipe.v1` update its single golden fixture and catalog
      example (they do **not** create one catalog entry per section). Only a
      genuinely new contract — `scene_recipe_build` and `recipe_render_result` —
      each gets its own golden fixture, `schema_catalog` entry, and doctor
      `FIXTURES` pin; the bidirectional catalog↔FIXTURES check must stay green.
- [x] **Root-field migration per slice.** For each new section, move its key from
      `UNSUPPORTED_SECTION_FIELDS` to `ROOT_FIELDS` in
      `scene::recipe::validation::suggestions`, refresh the `nearest_root_field`
      did-you-mean candidates, and update the schema examples, CLI goldens
      (`tests/assets/cli-golden/*`), and the `scene_recipe.v1` stable fixture in
      lockstep.
- [x] **Round-trip determinism** (rounded floats, `BTreeMap` ordering) so
      authored recipes and the build manifest are byte-stable.
- [x] The verify/introspect/diagnose/repair loop already works on authored nodes
      once they have stable handles — **no new verification surface needed.**

## Earlier review findings — resolved on this branch (verified)

Recorded for traceability; confirmed closed against source:

- `double_sided` GPU parity — GPU pipeline honors it (`gpu/depth.rs`,
  `gpu/instancing.rs`) and `placeholder_regression.rs` now asserts the
  HeadlessGpu path, not only CPU.
- `DebugOverlay` inert knob — removed from `src/`.
- appearance unmatched target — now emits `kind:"empty"` (zero pixels), not a
  whole-frame `frame_content` color.

## Definition of done

- **All slices 0–13 are implemented** — no deferred-by-design items. Every
  checklist box ends either implemented with a fail-closed proof, or removed from
  the surface.
- An LLM, given only the `scena` CLI + stable JSON + the starter snippets, can
  author a multi-object scene from primitives — geometry, materials (incl.
  clearcoat/sheen/anisotropy/iridescence/transmission), lights, cameras, keyframe
  animation, skin/morph, fonts, and host-supplied particles — build it, receive a
  typed id→handle manifest, render it, and verify color/visibility/pick in one
  `scena recipe render --introspect --verify` run, fail-closed.
- `ok: true` is true only when no requested renderable directive was skipped.
- Additive recipe fields update the existing `scene_recipe.v1` fixture/catalog;
  the new `scene_recipe_build` and `recipe_render_result` contracts each have
  their own fixture + catalog entry + doctor pin. The full gate chain and
  `doctor --full` pass.
- Resource/path policy rejects oversize/unsafe inputs by default.
- Renderer-only boundary intact: particle / skin / morph **rendering** is in
  scope; CAD-kernel, DXF/DWG/B-rep parsing, and time-stepped particle
  **simulation** stay host-side.
