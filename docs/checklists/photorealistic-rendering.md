## 1. Automatic lighting solver, not a preset

- [x] Analyze the subject's world bounds, dominant dimensions, surface
      orientations, material lobes, reflectivity, transparency, and authored
      environment before selecting any lighting.
- [x] Preserve complete, deliberate authored lighting; supplement or replace
      only lighting that cannot produce a readable photograph.
- [x] When useful lighting is absent, synthesize neutral photographic lighting
      algorithmically instead of selecting a named studio or hero preset.
- [x] Represent key, fill, rim, overhead, and environment illumination as
      continuously adjustable variables.
- [x] Derive initial emitter position, direction, distance, size, intensity,
      spread, and color temperature from the subject bounds and camera view.
- [x] Scale emitter size and distance with the subject so shadow softness and
      highlight size remain physically coherent across model scales.
- [x] Choose lighting from actual material response: dark matte surfaces need
      retained shadow detail, metals need shaped grazing highlights, glass
      needs readable transmission and edge reflections, and mixed materials
      need balanced treatment.
- [x] Rotate or synthesize environment illumination so reflective surfaces
      receive highlights that reveal curvature without obscuring the product.
- [x] Keep fill illumination strong enough to preserve detail without
      eliminating form, depth, or material contrast.
- [x] Add rim illumination only when the subject would otherwise merge with
      the background.
- [x] Render inexpensive previews while adjusting lighting variables.
- [x] Adjust lighting until the subject has readable surfaces, controlled
      highlights, useful shadows, and clear background separation.
- [x] Apply the same solver to imported models, authored scenes, assemblies,
      and multi-material subjects.
- [x] Do not make photographic success depend on a named intent, fixed light
      rig, asset-specific angle, or hand-tuned hero constants.

## 2. Scene-linear HDR rendering

- [x] Render the main scene into a floating-point HDR target such as
      `Rgba16Float`.
- [x] Keep lighting, reflections, transparency, bloom, depth of field,
      exposure, and compositing in scene-linear HDR space.
- [x] Decode base-color and emissive textures from sRGB before shading.
- [x] Treat normal, metallic, roughness, occlusion, depth, and other data
      textures as linear data.
- [x] Generate sRGB texture mipmaps by decoding to linear, filtering, and
      re-encoding instead of averaging encoded values.
- [x] Preserve radiance above `1.0` through every intermediate render and
      post-processing pass.
- [x] Use physically coherent light intensity, falloff, surface energy
      conservation, and environment contribution.
- [x] Meter exposure from scene-linear pixels before exposure, tonemapping,
      bloom, or display encoding.
- [x] Ensure metering never measures a frame that already contains the
      exposure value it is trying to calculate.
- [x] Weight metering toward geometry-derived subject pixels while retaining
      limited surround influence.
- [x] Apply exposure compensation on top of automatic exposure rather than
      replacing automatic metering.
- [x] Estimate and apply white balance before the display transform while
      preserving deliberate material colors.
- [x] Use AgX or a carefully validated ACES transform for highlight roll-off,
      saturation retention, and display conversion.
- [x] Drive bloom and glare from genuine HDR energy instead of already
      compressed 8-bit brightness.
- [x] Convert into the selected display color space only at final output.
- [x] Apply dithering and integer quantization only after the photographic
      pipeline is complete.

## 3. Automatic photographic surroundings

- [x] Determine whether the authored scene already contains an intentional
      floor, backdrop, room, or environment.
- [x] Preserve authored surroundings when they contribute to the requested
      photograph.
- [x] When surroundings are absent, infer a support height from the subject's
      lowest stable contact region.
- [x] Generate a neutral shadow catcher at the inferred support height.
- [x] Generate a seamless cyclorama-style background that extends beyond the
      camera frustum and cannot expose a floor edge or horizon seam.
- [x] Size all generated surroundings from the subject bounds and camera
      composition rather than fixed world dimensions.
- [x] Derive background luminance, hue, and warmth from the subject materials
      so the silhouette remains separated without distorting material colors.
- [x] Avoid pure black and pure white backgrounds when they would hide shadow
      detail or clip highlights.
- [x] Never add an automatic CAD grid, axis, construction line, technical
      floor marking, or visible staging slab to photographic output.
- [x] Generate soft contact shadows directly beneath supporting surfaces.
- [x] Keep contact shadows visible enough to ground the subject without
      appearing painted, crushed, or detached.
- [x] Add restrained floor reflections only when they improve grounding and
      material readability.
- [x] Derive reflection strength and roughness from the scene rather than
      using a mirror-like default.
- [x] Prevent floor reflections from duplicating or overpowering the subject.
- [x] Handle floating, suspended, wall-mounted, and environment-scale subjects
      without forcing an inappropriate floor underneath them.
- [x] Keep automatically generated surroundings separate from the authored
      semantic scene unless the caller explicitly requests persistence.

## 4. Automatic surface-realism improvement

- [x] Preserve every valid glTF PBR property, texture assignment, UV set,
      sampler, and color-space declaration.
- [x] Support the relevant glTF material extensions for clearcoat,
      transmission, volume, sheen, anisotropy, iridescence, specular response,
      emissive strength, and index of refraction.
- [x] Validate metallic, roughness, normal, occlusion, and emissive channel
      selection before shading.
- [x] Generate missing vertex normals using geometry and appropriate smoothing
      boundaries.
- [x] Generate a valid tangent frame whenever normal mapping requires one.
- [x] Detect inverted normals, invalid winding, degenerate triangles,
      disconnected faces, and malformed transforms before rendering.
- [x] Apply weighted-normal reconstruction when imported normals make
      manufactured surfaces look faceted or uneven.
- [x] Add scale-aware micro-bevel geometry or bevel-normal shading to
      unnaturally sharp manufactured edges.
- [x] Preserve deliberately sharp blades, sheet edges, panel gaps, and
      authored hard boundaries.
- [x] Detect physically valid but visually uninformative materials such as
      perfectly uniform gray metal or perfectly smooth plastic.
- [x] Add restrained, scale-aware micro-roughness to otherwise uniform
      manufactured surfaces.
- [x] Add subtle micro-normal variation only where it plausibly represents a
      real surface finish.
- [x] Prevent procedural surface detail from changing the object's identity,
      dimensions, base color, markings, or design.
- [x] Use a physically plausible neutral material when material data is
      entirely absent.
- [x] Never silently claim a specific substance such as steel, aluminum,
      rubber, or painted plastic without supporting asset information.
- [x] Use correct anisotropic filtering, mip selection, texture resolution,
      and normal-map orientation at every viewing distance.
- [x] Keep material response consistent between preview, native GPU, WebGPU,
      WebGL2, and final-image rendering paths.

## 5. Realistic indirect lighting, shadows, reflections, and transmission

- [x] Keep the raster renderer as the responsive preview path.
- [x] Add a high-quality final-image path using path tracing or hybrid ray
      tracing.
- [x] Build acceleration structures for meshes and instances, with updates for
      changed transforms and geometry.
- [x] Trace physically meaningful intersections for opaque, masked,
      transparent, transmissive, and instanced surfaces.
- [x] Trace soft shadows from the actual area, position, orientation, and
      shape of each emitter.
- [x] Support directional, point, spot, rectangular-area, emissive-geometry,
      and environment-light shadowing.
- [x] Add multi-bounce diffuse global illumination so shadowed surfaces receive
      realistic reflected light.
- [x] Add complete glossy and mirror reflections that are not limited to
      information already visible on screen.
- [x] Integrate rough reflections according to the material BRDF instead of
      applying a generic blurred reflection.
- [x] Handle Fresnel response, refraction, transmission, volume absorption,
      and total internal reflection for transparent materials.
- [x] Importance-sample lights, emissive geometry, the environment, and
      material lobes.
- [x] Use multiple-importance sampling to control noise in mixed direct,
      indirect, and environment lighting.
- [x] Accumulate samples progressively until the final image reaches the
      selected quality level.
- [x] Denoise without erasing small geometry, edges, labels, texture detail, or
      sharp reflections.
- [x] Suppress unstable fireflies without broadly clamping legitimate HDR
      highlights.
- [x] Preserve stable close-range contact shadows and small material details.
- [x] Use identical material and light definitions in preview and final paths
      so the final image improves fidelity without changing artistic intent.
- [x] Make the high-quality light-transport path the normal final output for
      photographic rendering when supported by the active backend.

## 6. Physically based automatic camera

- [x] Model sensor dimensions, focal length, aperture, focus distance, shutter
      time, sensitivity, and exposure compensation explicitly.
- [x] Derive an initial focal length from subject proportions and the required
      perspective instead of choosing a fixed field of view.
- [x] Solve focal length and camera distance together.
- [x] Prefer natural product-photography perspective and avoid wide-angle
      distortion unless spatial constraints require it.
- [x] Keep the subject large in the frame without cropping important geometry.
- [x] Center the subject's visual mass rather than only its node origin or AABB
      midpoint.
- [x] Account for asymmetric parts, attachments, open regions, and negative
      space during composition.
- [x] Select a view direction that reveals principal dimensions, depth, and
      important surfaces.
- [x] Avoid dead-front or perfectly axial views when they flatten
      three-dimensional form.
- [x] Preserve a deliberately authored camera when it is photographically
      coherent.
- [x] Resolve autofocus from visible subject-depth samples rather than a
      guessed constant or bounds center.
- [x] Bias focus toward important visible subject surfaces instead of
      occluders, reflections, or background pixels.
- [x] Derive aperture from focal length, subject depth range, desired
      sharpness, and requested background separation.
- [x] Compute depth of field from a physical circle of confusion instead of an
      arbitrary blur radius in pixels.
- [x] Keep the complete subject acceptably sharp for documentation and product
      views.
- [x] Use shallow focus only when it does not hide important structure.
- [x] Model aperture shape and bokeh consistently in the final renderer.
- [x] Apply lens distortion, vignette, glare, or chromatic aberration only when
      physically justified, never as shortcuts for realism.

## 7. Appearance-driven render, measure, and adjustment loop

- [x] Generate the initial candidate from asset analysis, lighting,
      surroundings, materials, and the physical camera.
- [x] Render low-resolution candidates through the same lighting, material,
      HDR, and color pipeline as the final image.
- [x] Identify the subject through geometry-derived masks or semantic buffers
      rather than color-difference heuristics.
- [x] Measure subject framing, visual center, crop, perspective distortion,
      and empty space.
- [x] Measure subject luminance separately from background luminance.
- [x] Measure low-end clipping, highlight clipping, retained dynamic range,
      local contrast, and color preservation.
- [x] Measure highlight size, shape, continuity, and distribution on metallic
      and glossy surfaces.
- [x] Determine whether specular structure reveals the subject's curvature and
      material instead of merely making it brighter.
- [x] Measure shadow softness, contact-shadow presence, grounding, and
      separation along the silhouette.
- [x] Measure white balance, color casts, saturation loss, distracting
      reflections, and washed-out surfaces.
- [x] Detect silhouettes, flat lighting, empty framing, weak camera angles,
      hidden material detail, and implausible surroundings.
- [x] Adjust camera pose and focal length when composition or perspective is
      wrong.
- [x] Adjust emitter position, size, spread, and intensity when surface form or
      shadows are wrong.
- [x] Adjust environment orientation when reflective materials lack readable
      highlight structure.
- [x] Adjust fill and rim illumination when dark surfaces merge into the
      background.
- [x] Adjust exposure only for global photographic brightness; never use it to
      compensate for defective lighting.
- [x] Adjust white balance and output transform only after lighting and
      exposure are physically coherent.
- [x] Evaluate multiple candidates when one configuration cannot satisfy the
      competing photographic requirements.
- [x] Select the candidate with the strongest combined photographic result,
      not merely the closest mean luminance or frame-fill value.
- [x] Stop when the candidate meets the appearance objective or further
      iterations no longer improve it.
- [x] Render the selected solution at final resolution using the high-quality
      light-transport path.
- [x] Keep the complete decision process deterministic for the same asset,
      viewport, renderer version, and authored scene state.

## 8. Repair or reject unusable assets

- [x] Inspect geometry, transforms, units, materials, textures, animations,
      cameras, and scene hierarchy before photographic rendering begins.
- [x] Classify each problem as safely repairable, repairable only with explicit
      appearance changes, or unrecoverable.
- [x] Repair missing normals and tangents when they can be reconstructed
      without changing the intended shape.
- [x] Repair safe winding, indexing, finite-value, and duplicate-vertex
      defects.
- [x] Remove or isolate degenerate triangles that cannot contribute valid
      shading.
- [x] Correct invalid scale and coordinate conventions only when metadata or
      strong geometric evidence makes the intended conversion unambiguous.
- [x] Resolve missing or incompatible texture data when an unambiguous
      authorized source is available.
- [x] Substitute a physically neutral material only when the asset has no
      usable material definition.
- [x] Detect open, folded, self-intersecting, inverted, incomplete, or severely
      malformed geometry that cannot produce a credible photograph.
- [x] Detect important components that are hidden, detached, microscopic,
      duplicated, or positioned far outside the primary scene.
- [x] Detect non-finite or physically impossible material values and texture
      combinations.
- [x] Detect texture resolution that is insufficient for the requested final
      image.
- [x] Never invent missing product components, markings, logos, manufacturing
      detail, semantic materials, or texture content.
- [x] Reject the photographic render when safe repair cannot recover a
      coherent visible subject.
- [x] Explain which asset information is missing or malformed and what the
      caller must supply.
- [x] Never report photographic success for a blank frame, silhouette,
      malformed model, materially unknowable object, or visibly broken image.
- [x] Define the supported promise as automatic photorealistic rendering for
      coherent geometry with sufficient physical material information.
- [x] Apply safe repairs automatically while making every appearance-changing
      substitution explicit.
