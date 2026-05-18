# DamagedHelmet Moving-Hole Root Cause

Date: 2026-05-16

## Finding

The apparent "moving hole" in the local DamagedHelmet demo is not an alpha cutout
or Scena-created geometry hole. It is the combination of authored DamagedHelmet
damage/cavity content plus view-dependent PBR terms:

- `demo/samples/DamagedHelmet.glb` has one opaque material, `Material_MR`.
- The material has base-color, metallic-roughness, normal, occlusion, and
  emissive textures.
- The material has no `alphaMode`, no alpha cutoff, and no blend path.
- The primitive contains `POSITION`, `NORMAL`, and `TEXCOORD_0`, but no authored
  `TANGENT`.
- Scena is not using a fixed fallback tangent for that case. The prepare path
  calls `accumulate_vertex_tangents()` and generates MikkTSpace tangent frames
  when `GeometryDesc::tangents()` is absent.

That means the black/open regions seen while orbiting are model/material content,
not renderer-generated transparency. The part that can feel like it moves is the
normal-map/specular/environment response, which is view dependent by definition.

## Evidence

Browser WebGL2 screenshots were captured from the local demo at
`http://127.0.0.1:18104/index.html` after rebuilding the WASM package.

- Full PBR:
  `target/gate-artifacts/helmet-root-cause-current/full-front.png`
  `target/gate-artifacts/helmet-root-cause-current/full-orbit.png`
- Environment disabled:
  `target/gate-artifacts/helmet-root-cause-current/full-no-env-front.png`
  `target/gate-artifacts/helmet-root-cause-current/full-no-env-orbit.png`
- Normal-map green channel flipped:
  `target/gate-artifacts/helmet-root-cause-current/normal-green-flip-front.png`
  `target/gate-artifacts/helmet-root-cause-current/normal-green-flip-orbit.png`
- Normal texture removed:
  `target/gate-artifacts/helmet-root-cause-current/no-normal-front.png`
  `target/gate-artifacts/helmet-root-cause-current/no-normal-orbit.png`
- Occlusion texture removed:
  `target/gate-artifacts/helmet-root-cause-current/no-occlusion-front.png`
  `target/gate-artifacts/helmet-root-cause-current/no-occlusion-orbit.png`
- Base-color-only unlit material:
  `target/gate-artifacts/helmet-root-cause-current/basecolor-unlit-front.png`
  `target/gate-artifacts/helmet-root-cause-current/basecolor-unlit-orbit.png`

The base-color-only unlit variant still contains the large black face cavity and
rear/side dark openings. Removing occlusion reduces near-black pixels, especially
in the front view, but does not remove the authored openings. Disabling the
environment makes the front cavity more starkly black, which confirms that the
environment is adding view-dependent reflected light rather than creating the
hole.

Pixel counts from the current Chromium WebGL2 capture:

```text
basecolor-unlit-front.png        dark=56541 nearblack=52262
full-front.png                   dark=14020 nearblack=1459
full-no-env-front.png            dark=61032 nearblack=54574
no-occlusion-front.png           dark=6321  nearblack=99
full-orbit.png                   dark=24466 nearblack=3957
basecolor-unlit-orbit.png        dark=29158 nearblack=18604
```

## Renderer Bug Found While Investigating

The lit PBR shader computed:

```wgsl
let occlusion_applied = mix(1.0, occlusion_sample, occlusion_strength);
```

but multiplied the lit branch by raw `occlusion_sample`. This ignored
`occlusionTexture.strength` for lit PBR materials. Both shader variants now use
`occlusion_applied`, and the contract is pinned by
`render::gpu::output::tests::triangle_shader_applies_occlusion_strength_to_lit_pbr_output`.

This is a real PBR correctness fix, but it is not the main DamagedHelmet visual
cause because this asset uses the default occlusion strength of `1.0`.

## Conclusion

Do not remove normal mapping or occlusion globally to hide this on DamagedHelmet.
That would make Scena less faithful to glTF source materials. If the demo needs
to make the helmet read less like a black hole for non-renderer users, use demo
presentation choices: brighter background, a better environment map, a fixed
initial view, or a separate "material debug" toggle. The renderer should preserve
the source material.
