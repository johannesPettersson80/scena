# M3A application features acceptance

Status: active evidence index

- [x] `assets_load_scene_caches_gltf_asset_and_rejects_required_extensions`
  and `scene_instantiate_creates_import_hierarchy_and_name_lookups` anchor
  `ARCH-M3A-SCENE-IMPORT`.
- [x] One opt-in shadowed directional light is enabled with
  `with_shadows(true)` and uses a Single shadow map with PCF 3x3 under
  `ARCH-SHADOW-MAP`.
- [x] The renderer has a Depth pre-pass (`ARCH-DEPTH-PREPASS`) and reports
  `pub depth_prepass_passes: u64` plus `pub depth_prepass_draws: u64`.
- [x] M2 also prepares a depth pre-pass so depth behavior has one renderer
  owner rather than a milestone-specific duplicate.
