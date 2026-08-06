# Renderer fidelity forward backlog

Status: active implementation backlog

This is the forward renderer-quality backlog required by section 8 of
`full-repo-review-v1.9.1-remediation.md`. It is subordinate to the renderer
charter in `docs/RFC-rust-3d-renderer.md`; it does not add CAD, simulation, or
asset-replacement responsibilities to `scena`.

## Product contract

`scena` owns the complete rendering path needed to turn the user's original
handmade glTF/GLB assets into convincing product stills. The renderer may derive
temporary render geometry, lighting, staging, material sampling, and reflection
data. It must not mutate the source GLB, require Blender, download replacement
models, or silently substitute a lower-quality path.

The four acceptance subjects are frozen in
`tests/assets/photo/final/fixture_manifest.json`. Their source geometry is the
product input, not a placeholder to be replaced when a test is difficult.

## Evidence classes

- `software_conformance`: lavapipe proves deterministic CPU/software Vulkan
  behavior, contracts, and resource lifecycle only.
- `supported_hardware`: a non-V3D physical GPU proves native final-photo output.
- `v3d_diagnostic`: Raspberry Pi V3D is an explicitly unstable diagnostic lane
  until its beauty-draw loss is fixed.
- `browser_conformance`: WebGPU/WebGL2 runtime proof is separate. A final-photo
  request must fail with `final_photo_unsupported` until that lane is
  implemented and proven.

No evidence class may be reported as another.

## 0. Frozen acceptance set

- [x] Track the three generated acceptance GLBs and pin all four original asset
      hashes in `tests/assets/photo/final/fixture_manifest.json`.
      Test-first exception: this is immutable binary test data, so SHA-256
      identity is the deterministic proof.
- [ ] Move the four reproducible final recipes beside the fixture manifest once
      `photo.quality = "final"` is accepted by the schema.
- [ ] Correct the speaker body binding from mottled `Metal046A` to clean smooth
      aluminium `Metal050A`, preserving its dark anodized color and a plausible
      satin roughness.
- [ ] Record a native-resolution baseline and a deliberately degraded negative
      control for every admitted quality metric.

## 1. Explicit final-photo contract

- [x] Add `photo.quality` with `preview` and `final`; omission remains `preview`
      for v1 compatibility.
- [x] Make `final` fail closed unless the backend can provide its complete
      contract. Native/headless GPU is the first supported backend.
- [x] Require at least 8 megapixels and supersample factor 2 for explicit final
      captures. The default final capture is 3840x2520 with SSAA2 and tent
      reconstruction.
- [ ] Keep the linear scene and post path in `Rgba16Float`, apply the existing
      photographic tonemapper once, and encode the final PNG as sRGB.
- [ ] Report requested/effective quality, capture dimensions, supersampling,
      reconstruction filter, environment resolution, probe count, shadow mode,
      edge-rounding coverage, and evidence class.
- [x] Return structured `final_photo_unsupported` on WebGPU/WebGL2 until their
      runtime path has equivalent proof. Never degrade to preview silently.

## 2. Material-aware quality oracle

The oracle starts report-only. A proxy is admitted only when the positive and
negative controls differ by at least 20% in the expected direction across the
frozen set. Admitted thresholds live in a tracked
`photo_final_policy_v1.json`; no threshold may be widened to make a failing
frame pass.

- [x] Extend the same-pass beauty semantic attachment with material identity so
      analysis uses pixels actually written by the beauty pass.
- [x] Measure reflection structure on smooth metallic subject pixels, excluding
      silhouettes and direct-light saturation.
- [x] Measure grounding along actual subject/floor semantic boundaries instead
      of a rectangular band under projected bounds.
- [x] Measure curved-contour polygonality on semantic silhouettes at native
      output resolution.
- [x] Measure highlight distribution per material class, including p99,
      near-white coverage, and clipped coverage.
- [ ] Measure projected texture density and flag undersampled material maps.
- [x] Keep each new metric report-only until its controls, calibration set,
      threshold version, and false-positive review are recorded.

## 3. Reflection stack

- [x] Use the bundled 1024x512 HDR source for final photography and require an
      HDR source of at least that size for custom final environments.
- [x] Bake a 512-pixel environment cube for final mode without routing through
      the 128x64 preview thumbnail.
- [ ] Add typed `ReflectionProbe` scene state and recipe authoring with
      box-projected bounds and explicit material/component assignment.
- [ ] Prepare and prefilter at most four 256-pixel local probes before render;
      probe capture includes other opaque subject parts and staging, excludes
      its assigned component, and disables probes recursively.
- [ ] Cache probe results by scene, transform, material, lighting, environment,
      and staging revisions.
- [ ] Select the smallest containing probe deterministically and fall back to
      the global environment outside all probe volumes.

## 4. Grounding and shadows

- [ ] Add final-mode PCSS area-light shadows with a 4096 key map and 2048 fill
      map; rim and overhead lights remain unshadowed.
- [ ] Derive blocker search and penumbra from light radius and receiver depth,
      with deterministic sampling and explicit resource validation.
- [ ] Disable the fake photographic contact-shadow proxy in final mode.
- [ ] Prove attached contact at the actual beauty-semantic subject/floor
      boundary for all four subjects.

## 5. Final sampling

- [ ] Make 3840x2520 SSAA2 plus tent reconstruction the final default, without
      redundant MSAA.
- [ ] Validate exact transient texture dimensions and byte budgets before
      prepare. Reject unsupported requests rather than downscaling.
- [ ] Preserve the existing post chain at supersampled resolution and resolve
      once before PNG encoding.
- [ ] Prove nonblank, correctly framed, nonclipped output at native resolution;
      inspect 1:1 crops rather than enlarged preview screenshots.

## 6. Imported-mesh edge rounding

- [ ] Add opt-in `edge_rounding` to imports and derive render-only geometry;
      never mutate or export the source GLB.
- [ ] Support static closed two-manifold triangle meshes first. Reject skinned,
      morphed, open, or nonmanifold inputs with structured diagnostics.
- [ ] Round hard edges above 30 degrees using three segments and a default
      radius of 0.25% of subject dominant extent, capped at 20% of adjacent edge
      length.
- [ ] Preserve/interpolate UVs and vertex colors, then recompute normals and
      tangents.
- [ ] Enforce an explicit derived-vertex/triangle budget and fail rather than
      emitting corrupt geometry.
- [ ] Report eligible, rounded, skipped, and rejected edge counts.

## 7. Material catalog resolution variants

- [ ] Version the material catalog/manifest so one material can expose verified
      1K, 2K, and 4K source variants while v1 packs remain loadable.
- [ ] Add `scena materials fetch --resolution {1k,2k,4k}` with archive hashes
      and decoded-map dimensions in the lock/pack evidence.
- [ ] Key material caches by material identity and resolution.
- [ ] Select 2K only when projected UV density would otherwise fall below one
      source texel per output pixel. Require an explicit budget increase for 4K.
- [ ] Preserve the 64 MiB default decoded-texture budget and fail with a
      resource plan instead of silently lowering resolution.

## 8. Blocking gate and integration

- [ ] Calibrate `photo_final_policy_v1.json` from the four positives and their
      controlled degradations, review false positives, then make admitted
      final-photo metrics blocking only for `photo.quality = "final"`.
- [ ] Add a doctor rule that prevents final mode from losing its fail-closed
      backend, evidence-class, HDR-resolution, probe, shadow, edge-rounding, or
      quality-policy contracts.
- [ ] Run focused red/green proof per logical slice, scoped remote-builder gates
      after each slice, and the full release chain once on the frozen integrated
      diff.
- [ ] Produce the four-subject final contact sheet and 1:1 crops on supported
      physical GPU hardware.
- [ ] Run a separate V3D diagnostic bundle; do not block supported-hardware
      acceptance on the known unstable adapter.
- [ ] Run WebGPU and WebGL2 runtime proof before enabling final mode in browsers.

## Validation ledger

- `focused`: fixture SHA-256 values match
  `tests/assets/photo/final/fixture_manifest.json`; final recipe/backend
  contract, same-pass material attribution, synthetic quality controls,
  photo-report schema, and final 1024x512/512 environment selection each passed
  their expected-red/green proof on `scena-builder`.
- `scoped`: the final CLI contract and final SceneHost lighting/environment
  tests pass in the isolated builder checkout. Broader gates remain deferred
  until the cross-renderer reflection/shadow integration checkpoint.
- `full`: deferred to the integrated cross-backend checkpoint.
- `skipped`: visual acceptance is not claimed from the frozen inputs alone.
- `investigation`: 0 remediation attempts, 0 release-candidate pushes,
  0 full-matrix runs, 0 user-required hardware actions in this backlog run.
