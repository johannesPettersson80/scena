---
name: scena-gltf-assets
description: Use when implementing or reviewing glTF/GLB loading, KHR extensions, asset cache/dedup/hot reload, anchors/extras, units, coordinate conversion, animation clips, skinning, morph targets, or import rebinding.
---

# Scena glTF And Assets

## Required RFC Contracts

- glTF/GLB is the primary asset format.
- `Assets` owns fetching, parsing, cache identity, retain policy, and hot reload.
- `Scene::instantiate()` creates import-local `NodeKey`s and name/path indexes.
- Animation channels are rebound from source node indices to import-local `NodeKey`s.
- Anchors load from glTF `extras.scena.anchors` and can also be created in code.
- Unsupported required extensions must produce explicit errors.

## v1.0 glTF Gates

Support:

- node hierarchy and transforms
- meshes, materials, textures, vertex colors
- cameras and `KHR_lights_punctual`
- `KHR_materials_unlit`
- `KHR_materials_emissive_strength`
- `KHR_texture_transform`
- `KHR_mesh_quantization`
- animation clips, skinning, morph targets

Correctness gate:

- `RiggedSimple`
- `SimpleSkin`
- `SimpleMorph`
- `MorphCube`
- `RiggedFigure`
- `BrainStem`
