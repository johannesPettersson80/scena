# State-Of-The-Art Demo Materials And Performance Checklist

This checklist is the execution contract for making the public scena demo
look and feel production-grade. The connector snap remains the hero. WaterBottle
is only a control sample proving the renderer can produce high-quality PBR.
If the connector looks bad, stop and fix the root cause. Do not hide the
problem by defaulting to a generic sample.

## 0. Hard Rules

- [ ] Connector snap is the default hero and must be fixed at root cause.
- [ ] WaterBottle is the quality control reference, not a workaround.
- [ ] Change one variable at a time: asset, material, HDR, key light, exposure,
      camera, or shadow settings.
- [ ] Record screenshots and timing before and after each meaningful change.
- [ ] Test first before production renderer code changes.
- [ ] No deploy or release handoff until every demo page has been inspected.
- [ ] No "looks fine" judgement without screenshots from the actual browser demo.
- [ ] Preserve connector behavior: `scene.mate(&drive, "shaft", &load, "hub")`
      must still work after asset edits.

## 0.1 Worktree And Branch Discipline

The work must be split by failure class. Do not mix asset quality, renderer
optimization, demo shell changes, and deployment in one dirty checkout.

### Visual Asset Root Cause Worktree

- [ ] Create a separate worktree for visual asset work.
- [ ] Branch name: `demo/connector-asset-quality`.
- [ ] Scope: Blender, GLB, material, texture, bevel, bake, and connector
      metadata verification only.
- [ ] Allowed primary edits:
      - `demo/samples/connector-snap/drive_unit.glb`
      - `demo/samples/connector-snap/load_unit.glb`
      - `demo/samples/connector-snap/connector_snap_assembly.glb`
      - `tests/assets/gltf/drive_unit.glb`
      - `tests/assets/gltf/load_unit.glb`
      - screenshot/timing artifacts or checklist evidence files.
- [ ] Forbidden in this worktree:
      - renderer performance code
      - transform cache code
      - UI redesign
      - deployment-only changes
      - unrelated cleanup.
- [ ] This worktree answers exactly one question:
      `Do the connector assets look production-grade?`

Required gate before this worktree is considered complete:

- [ ] `cargo run --example mate_two_parts`
- [ ] connector metadata verified after Blender export
- [ ] local desktop screenshots for every page
- [ ] local mobile screenshots
- [ ] my own manual screenshot review written down
- [ ] explicit user visual approval after my review
- [ ] external review if requested

### Renderer Performance Worktree

- [ ] Create a separate worktree for transform/replay/orbit performance.
- [ ] Branch name: `render/transform-template-cache`.
- [ ] Scope: renderer prepare/render performance only.
- [ ] Allowed primary edits:
      - renderer internals
      - focused renderer tests
      - performance probe scripts/artifacts
      - doctor rules only if a silent-failure family is exposed.
- [ ] Forbidden in this worktree:
      - Blender/GLB asset edits
      - material authoring
      - visual lighting/camera tuning
      - page layout changes.
- [ ] This worktree answers exactly one question:
      `Are replay and orbit fast without static rebuilds?`

Required gate before this worktree is considered complete:

- [ ] failing transform-only cache/reuse test first
- [ ] replay/orbit timing before and after
- [ ] proof that `collect_prepared_primitives` disappears or becomes near-zero
      on transform-only frames
- [ ] proof that shadow pass, depth prepass, and PBR path remain enabled
- [ ] browser proof remains nonblank and correct.

### Demo Shell And Deployment Worktree

- [ ] Create a separate worktree for demo shell integration.
- [ ] Branch name: `demo/showcase-shell`.
- [ ] Scope: page layout, code panel, diagnostics, replay button, README GIF,
      local/Cloudflare/production proof.
- [ ] This worktree consumes approved GLBs from `demo/connector-asset-quality`.
- [ ] Forbidden in this worktree:
      - Blender source material work
      - renderer performance architecture
      - unapproved fallback to WaterBottle as hero.
- [ ] This worktree answers exactly one question:
      `Is the public page assembled and deployed correctly?`

Required gate before this worktree is considered complete:

- [ ] local desktop and mobile screenshots inspected
- [ ] local browser probe passes
- [ ] Cloudflare preview verified
- [ ] production alias verified
- [ ] production desktop and mobile screenshots inspected
- [ ] production console has no red errors.

### Integration Order

- [ ] Finish `demo/connector-asset-quality` first.
- [ ] Do not begin public shell/deployment completion until connector visuals
      have passed my own review and user visual approval.
- [ ] Merge or rebase approved connector assets into `demo/showcase-shell`.
- [ ] Run visual proof again after integration.
- [ ] Keep `render/transform-template-cache` separate until visual quality is
      stable.
- [ ] Final integration may combine branches only after each branch has passed
      its own evidence gate.
- [ ] Only one branch/worktree may change the hero screenshot at a time.
- [ ] Every checklist item must end with evidence: screenshot path, command,
      timing number, validator output, or explicit `not done`.

## 1. Files And Assets

Primary connector assets:

- [ ] `demo/samples/connector-snap/drive_unit.glb`
- [ ] `demo/samples/connector-snap/load_unit.glb`
- [ ] `demo/samples/connector-snap/connector_snap_assembly.glb`

Test/reference copies:

- [ ] `tests/assets/gltf/drive_unit.glb`
- [ ] `tests/assets/gltf/load_unit.glb`

Required preservation:

- [ ] Preserve `extras.scena.connectors[]`.
- [ ] Preserve connector names `shaft` and `hub`.
- [ ] Preserve deliberate unit/up-axis split used by the demo.
- [ ] Export binary GLB with embedded textures.
- [ ] Verify GLB files with glTF validation.
- [ ] Verify `cargo run --example mate_two_parts`.

## 2. Blender Setup

- [ ] Work in Blender on copied source files first; do not overwrite the only
      known-working GLBs until export has been validated.
- [ ] Enable export of custom properties so connector metadata survives.
- [ ] Use Cycles/Eevee only as authoring previews; the deliverable is glTF
      metallic-roughness textures that scena can load.
- [ ] Use WaterBottle and ToyCar as material quality references.
- [ ] Use the browser demo as the final judge, not Blender viewport lighting.

## 3. Bevels And Geometry

Do this before material work. Razor-sharp CAD edges do not catch highlights.

- [ ] Add a Bevel modifier to every manufactured hard edge.
- [ ] Use small real-world bevel widths, starting at roughly `0.5-1.0 mm`
      at asset scale.
- [ ] Use `2` bevel segments as the first pass.
- [ ] Add a Weighted Normal modifier after beveling.
- [ ] Apply or export final bevel geometry into the GLB.
- [ ] Bevel shaft ends, baseplate edges, flywheel rim, gearbox edges, bolt
      heads, pedestal edges, and connector-facing faces.
- [ ] Check cylinders for faceting and incorrect smoothing.
- [ ] Confirm bellows folds have real depth and self-shadowing.
- [ ] Re-export and screenshot before doing the full material bake.

Acceptance:

- [ ] Bright edge highlights appear on metal and machined parts.
- [ ] Edges no longer read as perfect untextured CAD.
- [ ] No broken normals or faceted shading artifacts appear in the browser.

## 4. Core Procedural Roughness Generator

Every major part needs roughness variation. Flat roughness is the main reason
the connector assets look like clay.

Build this reusable procedural node pattern in Blender:

- [ ] Fine Noise Texture:
      - Scale: `15-40`
      - Detail: high
      - Purpose: small surface variation.
- [ ] Broad Noise Texture:
      - Scale: about `3`
      - Strength: low mix contribution
      - Purpose: broad worn/uneven patches.
- [ ] Mix the fine and broad noise.
- [ ] Feed the mixed noise into a ColorRamp.
- [ ] Compress the ColorRamp into the target roughness band for the material.
- [ ] Plug the result into the Principled BSDF Roughness input while authoring.
- [ ] Bake this procedural roughness to a texture before GLB export.

Roughness bands:

- [ ] Polished steel: `0.12-0.28`
- [ ] Machined gearbox: `0.35-0.50`
- [ ] Anodized aluminium: `0.30-0.42`
- [ ] Cast/painted housing: `0.50-0.65`
- [ ] Baseplate painted steel: about `0.55`
- [ ] Baseplate bare steel: about `0.50`
- [ ] Rubber/fabric bellows: `0.80-0.92`
- [ ] Bolts/screws: `0.30-0.45`

Acceptance:

- [ ] No major connector part uses one flat roughness value across the mesh.
- [ ] Roughness variation is visible in browser highlights.
- [ ] Dark materials remain dark and do not become milky.
- [ ] Metals commit to real metal behavior instead of the `0.4-0.6` plastic
      middle zone unless that middle zone is deliberately rough metal.

## 5. Material Recipes

### 5.1 Polished Steel Shaft

Shader authoring:

- [ ] Metallic: `1.0`
- [ ] Base color: neutral steel gray.
- [ ] Roughness: procedural band `0.12-0.28`.
- [ ] Add directional scratches:
      - Wave Texture
      - Bands type
      - Thin spacing
      - High distortion only if it still reads longitudinal
      - Aligned with shaft axis
      - Mixed subtly into roughness.
- [ ] Add very subtle Wave Texture or Noise Texture into Bump.
- [ ] Bump strength target: `0.03-0.08` for fine steel scratches.

Bake/export:

- [ ] Bake base color.
- [ ] Bake roughness.
- [ ] Bake tangent-space normal.
- [ ] Bake AO/contact where shaft meets other parts if applicable.

Acceptance:

- [ ] Shaft catches crisp environment reflections.
- [ ] Shaft reads as steel, not white plastic or gray clay.
- [ ] Highlight direction supports the cylindrical form.

### 5.2 Dark Anodized Flywheel

Shader authoring:

- [ ] Metallic: `1.0`
- [ ] Base color: dark graphite with slight blue tint.
- [ ] Roughness: procedural band `0.30-0.42`.
- [ ] Avoid roughness stuck around `0.50` with no variation.
- [ ] Add concentric brushed detail:
      - Wave Texture
      - Rings mode
      - Centered on flywheel hub
      - Fine spacing
      - Mixed into roughness.
- [ ] Use subtle ring bump if it does not shimmer.

Bake/export:

- [ ] Bake base color.
- [ ] Bake roughness.
- [ ] Bake tangent-space normal.
- [ ] Bake AO around hub and rim.

Acceptance:

- [ ] Flywheel reads as dark anodized metal.
- [ ] It has a satin sheen, not a blurry gray smear.
- [ ] Circular/ring detail is visible but not noisy.

### 5.3 Cast Motor Housing

Shader authoring:

- [ ] Metallic: `0.0`.
- [ ] Reason: painted housing is dielectric paint, not bare metal.
- [ ] Base color: dark navy/industrial blue.
- [ ] Roughness: procedural band `0.50-0.65`.
- [ ] Add cast texture:
      - Voronoi or Noise Texture into Bump
      - Low strength, `0.05-0.15`
      - Scale tuned until it reads as pebbled cast surface, not dirt.
- [ ] Fake soft paint sheen by keeping roughness below `0.65`.
- [ ] Do not use clearcoat/transmission extensions.

Bake/export:

- [ ] Bake base color with subtle color variation.
- [ ] Bake roughness.
- [ ] Bake tangent-space normal.
- [ ] Bake AO/contact in recesses and mounting points.

Acceptance:

- [ ] Blue housing reads as painted/cast material.
- [ ] It is not metallic blue clay.
- [ ] Soft environment highlight is visible without looking like plastic wrap.

### 5.4 Machined Gearbox Gray

Shader authoring:

- [ ] Metallic: `0.8-1.0`.
- [ ] Base color: machined mid gray, not pure white.
- [ ] Roughness: procedural band `0.35-0.50`.
- [ ] Add fine machining marks:
      - Wave Texture
      - Fine parallel lines
      - Mixed into roughness and optionally bump.
- [ ] Use subtle low-frequency noise for worn patches.

Bake/export:

- [ ] Bake base color.
- [ ] Bake roughness.
- [ ] Bake tangent-space normal.
- [ ] Bake AO around seams, bolt recesses, and connector areas.

Acceptance:

- [ ] Gearbox separates clearly from white housing and steel parts.
- [ ] Fine machined detail catches light.
- [ ] It no longer reads as uniform matte plastic.

### 5.5 Baseplate

Choose one material direction and commit.

Option A, painted steel:

- [ ] Metallic: `0.0`
- [ ] Base color: darker industrial gray.
- [ ] Roughness: about `0.55`, with procedural variation.

Option B, bare steel:

- [ ] Metallic: `1.0`
- [ ] Base color: neutral steel gray.
- [ ] Roughness: about `0.50`, with procedural variation.

Do not use ambiguous half-metal values without a reason.

Edge wear:

- [ ] Use Geometry Pointiness if available in the Blender version.
- [ ] Pointiness -> ColorRamp -> Mix:
      - Lighten base color on exposed edges.
      - Lower roughness slightly on exposed edges.
- [ ] If Pointiness is unavailable, use bevel-driven/curvature mask workflow
      or manually generated wear masks.

Bake/export:

- [ ] Bake base color including edge wear.
- [ ] Bake roughness including edge wear.
- [ ] Bake tangent-space normal.
- [ ] Bake AO under mounted parts and around bolts.

Acceptance:

- [ ] Baseplate grounds the model.
- [ ] Edges catch highlights.
- [ ] Contact regions do not float visually.

### 5.6 Bellows

The bellows must stop reading as white plastic washers.

Shader authoring:

- [ ] Metallic: `0.0`
- [ ] Base color: charcoal rubber or dark fabric.
- [ ] Roughness: `0.80-0.92`
- [ ] Add fine rubber/fabric normal noise:
      - Noise Texture -> Bump
      - Strength `0.05-0.12`
- [ ] Add strong AO in fold valleys.
- [ ] Keep specular response soft and minimal.

Geometry/material decision:

- [ ] If bellows still dominate the hero and do not support the shaft-hub story,
      simplify or remove them only after a root-cause note explains why.

Bake/export:

- [ ] Bake base color.
- [ ] Bake roughness.
- [ ] Bake tangent-space normal.
- [ ] Bake AO with fold valleys darkened.

Acceptance:

- [ ] Bellows read as rubber/fabric.
- [ ] Fold valleys are visibly darker.
- [ ] Bellows no longer dominate the model as bright white rings.

### 5.7 Bolts And Small Hardware

Shader authoring:

- [ ] Metallic: `1.0`
- [ ] Base color: steel gray.
- [ ] Roughness: `0.30-0.45`.
- [ ] Bevel bolt heads.
- [ ] Add pointiness/curvature edge wear.
- [ ] Add AO under bolt heads.

Bake/export:

- [ ] Bake base color or use shared atlas.
- [ ] Bake roughness.
- [ ] Bake normal.
- [ ] Bake AO.

Acceptance:

- [ ] Bolts catch small highlights.
- [ ] Bolt recesses are readable.
- [ ] Hardware helps scale the model.

### 5.8 White And Off-White Parts

Pure white destroys mid-tone detail.

- [ ] Use off-white or light industrial gray, not `#ffffff`.
- [ ] Keep roughness varied, not flat.
- [ ] Use AO and normals so ribs and edges remain readable.
- [ ] Avoid high albedo plus high IBL diffuse that washes out shape.

Acceptance:

- [ ] White/gray pieces retain detail under HDR.
- [ ] Ribs, seams, and bevels remain visible.
- [ ] They do not collapse into one bright mass.

## 6. Procedural Normal/Bump Recipes

Generic micro-surface:

- [ ] Noise Texture -> Bump Node -> Normal.
- [ ] Bump strength: `0.05-0.15`.
- [ ] Tune scale per part.
- [ ] Keep it subtle enough to avoid noisy shimmer.

Machined surfaces:

- [ ] Wave Texture -> Bump Node -> Normal.
- [ ] Align lines with machining direction.
- [ ] Use fine scale and low strength.

Cast surfaces:

- [ ] Voronoi or Noise Texture -> Bump Node -> Normal.
- [ ] Low strength.
- [ ] Avoid chunky stone-like pattern.

Bake:

- [ ] Bake all procedural bump effects into tangent-space normal maps.
- [ ] Verify normal map orientation in the browser.

Acceptance:

- [ ] Flat CAD smoothness is gone.
- [ ] Light catches surface detail.
- [ ] No inverted normal artifacts.

## 7. Baking Pipeline

Procedural Blender nodes do not export as GLB material logic. They are generators.
The deliverable is baked texture maps wired into glTF-compatible Principled BSDF.

Per major part:

- [ ] UV unwrap.
- [ ] Create image texture targets.
- [ ] Use `1024x1024` for hero/large material islands.
- [ ] Use `512x512` for small hardware/bolts.
- [ ] Bake Base Color.
- [ ] Bake Roughness as grayscale.
- [ ] Bake Normal in tangent space.
- [ ] Bake AO.
- [ ] Build final Principled BSDF using baked image textures.
- [ ] Plug baked base color into Base Color.
- [ ] Plug baked roughness into Roughness.
- [ ] Set metallic scalar or packed metallic map.
- [ ] Plug baked normal through Normal Map node.
- [ ] Include AO according to glTF-compatible workflow.
- [ ] Export GLB.

glTF metallic-roughness notes:

- [ ] glTF packs roughness in the green channel.
- [ ] glTF packs metallic in the blue channel.
- [ ] Let Blender's glTF exporter pack channels where possible by wiring the
      Principled BSDF correctly.
- [ ] Do not require unsupported material extensions.
- [ ] Do not use clearcoat, transmission, or anisotropy as required extensions.

AO bake:

- [ ] Bake AO with the part isolated for self-occlusion.
- [ ] Confirm fold valleys, bolt bases, recesses, and contact areas darken.
- [ ] If baking combined AO into base color, keep it subtle and physically plausible.

Acceptance:

- [ ] Re-imported GLB shows image texture maps, not only scalar materials.
- [ ] Browser demo uses the baked maps.
- [ ] `material_textures_missing_decoded_pixels` is zero in browser proof.

## 8. Empirical Material Loop

Do not build every material before checking the browser.

- [ ] Start with the shaft only.
- [ ] Bevel the shaft.
- [ ] Apply steel material recipe.
- [ ] Bake shaft maps.
- [ ] Export GLB.
- [ ] Run local demo.
- [ ] Capture connector screenshot next to WaterBottle screenshot.
- [ ] Decide whether the shaft reads as steel.
- [ ] Adjust ColorRamp roughness band if needed.
- [ ] Re-bake and re-check.
- [ ] Once the shaft meets the bar, clone the process to flywheel, gearbox,
      housing, baseplate, bellows, and bolts.

Acceptance:

- [ ] At least one part reaches WaterBottle-level material credibility before
      all parts are attempted.
- [ ] Material recipes are adjusted from browser evidence, not Blender-only preview.

## 9. Fast Time-Boxed Priority

If time is limited, do this order:

- [ ] Bevel everything.
- [ ] Fix metallic values:
      - Housing: `0.0`
      - Bellows: `0.0`
      - Painted baseplate if chosen: `0.0`
      - Shaft: `1.0`
      - Flywheel: `1.0`
      - Bolts: `1.0`
      - Gearbox: `0.8-1.0`
- [ ] Kill ambiguous `0.6-0.8` metallic values unless justified.
- [ ] Add procedural roughness maps using two-noise-plus-tight-ramp recipe.
- [ ] Add procedural bump normals.
- [ ] Add edge wear with Pointiness or curvature mask.
- [ ] Bake.
- [ ] Export.
- [ ] Screenshot in browser.

Expected result:

- [ ] Images improve immediately before the full texture pipeline is perfect.
- [ ] Connector stops reading as clay/CAD.

## 10. Runtime Lighting Contract

Only tune lighting after asset baseline work is measurable.

- [ ] Use neutral studio HDR environment.
- [ ] Add directional key light from three-quarter/front-above.
- [ ] Enable shadows.
- [ ] Keep contact shadows visible.
- [ ] Confirm dark materials are not lifted too much by diffuse IBL.
- [ ] Confirm rough fabric/rubber is not shiny.
- [ ] Confirm metals show environment response.
- [ ] Confirm ACES/exposure does not wash out connector whites.
- [ ] Change only one lighting parameter at a time.

Screenshots after each change:

- [ ] HDR only.
- [ ] Key light only.
- [ ] Shadows only.
- [ ] Exposure only.
- [ ] Camera only.

Acceptance:

- [ ] Connector looks good under the same lighting where WaterBottle looks good.
- [ ] If WaterBottle is good and connector is bad, return to asset work.

## 11. Camera And Composition

- [ ] Use a three-quarter connector view.
- [ ] Center the mate joint.
- [ ] Model fills most of the canvas without clipping.
- [ ] No large empty void above the model.
- [ ] Replay motion clearly moves drive unit into load unit along mate axis.
- [ ] Mobile framing separately checked.
- [ ] Orbit starts from a useful angle.

Acceptance:

- [ ] First screenshot communicates authored connector mating.
- [ ] A viewer can tell this is live 3D, not a static image.

## 12. Performance Baseline

Measure before optimization and after each performance change.

Use:

- [ ] `?perf=1`
- [ ] `?timing=1`
- [ ] Browser Performance panel.
- [ ] Network waterfall.
- [ ] `node scripts/probe_cloudflare_demo.js <url>`

Record:

- [ ] Browser name and version.
- [ ] Backend: WebGPU or WebGL2.
- [ ] Device pixel ratio.
- [ ] Canvas CSS size.
- [ ] Canvas internal resolution.
- [ ] WASM size compressed.
- [ ] WASM size uncompressed.
- [ ] HTML/CSS/JS transfer size.
- [ ] HDR transfer size.
- [ ] GLB transfer size per asset.
- [ ] Texture sizes inside GLB.
- [ ] WASM compile/init time.
- [ ] Asset fetch time.
- [ ] Texture decode time.
- [ ] GLB parse/import time.
- [ ] Environment decode/prepare time.
- [ ] First `Renderer::prepare` time.
- [ ] First visible frame time.
- [ ] Replay frame time p50/p95/p99.
- [ ] Orbit frame time p50/p95/p99.
- [ ] `collect_prepared_primitives` time.
- [ ] `gpu.prepare` time.
- [ ] Shadow pass time.
- [ ] Depth prepass time.
- [ ] PBR pass time.
- [ ] GPU memory estimate.
- [ ] JS console warnings/errors.

Target budgets:

- [ ] Local first visible frame: under `2 s`.
- [ ] Production first visible frame: under `3 s`.
- [ ] Desktop orbit p95 frame time: under `16 ms`.
- [ ] Replay transform-only p95 frame time: materially below old `90-110 ms`
      range.
- [ ] No `35-45 ms` replay cost in `collect_prepared_primitives`.

## 13. Performance Fix Contract

Load path:

- [ ] Default first paint loads only assets needed for connector snap.
- [ ] Secondary Khronos samples lazy-load.
- [ ] No asset fetch inside `render()`.
- [ ] No texture decode inside `render()`.
- [ ] No shader compile inside `render()`.
- [ ] No static GPU resource upload during orbit.

Transform/replay path:

- [ ] Add focused test for transform-only GPU template reuse before code changes.
- [ ] Full prepare builds prepared primitives and static GPU resources.
- [ ] Transform-only prepare skips primitive collection.
- [ ] Transform-only prepare reuses vertex buffers.
- [ ] Transform-only prepare reuses material bind groups.
- [ ] Transform-only prepare reuses pipelines.
- [ ] Transform-only prepare reuses textures.
- [ ] Transform-only prepare updates draw uniforms.
- [ ] Transform-only prepare updates camera/output uniforms.
- [ ] Transform-only prepare updates light uniforms.
- [ ] Transform-only prepare updates shadow matrices.
- [ ] Shadow pass remains enabled.
- [ ] Depth prepass remains enabled.

Acceptance:

- [ ] Replay logs show dynamic/transform-only path at least once.
- [ ] `collect_prepared_primitives` disappears or becomes near-zero on replay frames.
- [ ] Orbit does not rebuild static GPU draw data.
- [ ] Visual output remains nonblank and correct.

## 14. Visual Proof Checklist

Local screenshots:

- [ ] Connector snap default.
- [ ] Connector replay mid-motion.
- [ ] Drive unit.
- [ ] Load unit.
- [ ] WaterBottle.
- [ ] ToyCar.
- [ ] Mobile connector.
- [ ] Mobile code panel.

Manual inspection:

- [ ] Steel reads as steel.
- [ ] Anodized flywheel reads as anodized metal.
- [ ] Painted housing reads as painted/cast.
- [ ] Bellows read as rubber/fabric.
- [ ] Bolts and bevels catch highlights.
- [ ] White/gray pieces retain mid-tone detail.
- [ ] Contact shadows ground the model.
- [ ] ToyCar fabric/drape does not look incorrectly reflective.
- [ ] WaterBottle remains high quality.
- [ ] No scattered geometry.
- [ ] No blown-out white surfaces.
- [ ] No near-black unreadable pages.
- [ ] No red console errors.

Self-review gate:

- [ ] I inspect every local desktop screenshot myself before handoff.
- [ ] I inspect every local mobile screenshot myself before handoff.
- [ ] I compare connector screenshots against WaterBottle as the control sample.
- [ ] I write down the visual verdict before asking for user approval.
- [ ] If I see weak material separation, bad roughness, wrong reflectivity,
      washed-out whites, missing contact shadows, or bad framing, I return to
      root-cause work instead of asking for approval.

User approval gate:

- [ ] After my own review passes, send the screenshot set to the user.
- [ ] The user must visually approve the connector improvement before deploy,
      merge, release, or production proof is considered complete.
- [ ] If the user rejects the visual result, record the rejection reason and
      return to root-cause analysis. Do not route around it.

External review:

- [ ] Send before/after screenshot set for review.
- [ ] Require explicit verdict.
- [ ] If verdict is reject, root-cause and fix. Do not route around.

## 15. Required Gates

Local:

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test`
- [ ] `cargo run -p xtask -- doctor --full`
- [ ] `cargo run --example mate_two_parts`
- [ ] `wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page`
- [ ] `node scripts/probe_cloudflare_demo.js http://127.0.0.1:<port>/index.html`

Browser:

- [ ] WebGPU probe.
- [ ] WebGL2 probe.
- [ ] Local desktop screenshots.
- [ ] Local mobile screenshots.
- [ ] Console check: no red errors.

Deployment:

- [ ] Push branch.
- [ ] Confirm Cloudflare preview.
- [ ] Probe Cloudflare preview.
- [ ] Confirm production alias `https://scena-demo.pages.dev/index.html`.
- [ ] Probe production alias.
- [ ] Capture production desktop screenshot.
- [ ] Capture production mobile screenshot.
- [ ] Confirm production console has no red errors.
- [ ] Keep repo clean after commit/push/deploy.

## 16. Definition Of Done

- [ ] Connector snap is visually comparable to WaterBottle quality.
- [ ] Connector assets use real bevels and baked texture maps.
- [ ] Materials are physically separated and recognizable.
- [ ] First visible frame is fast.
- [ ] Replay is smooth.
- [ ] Orbit is smooth.
- [ ] Transform-only frames do not rebuild static GPU draw data.
- [ ] All pages have been manually inspected.
- [ ] My own visual review has passed.
- [ ] User visual approval has been received after my review.
- [ ] Local gates are green.
- [ ] Browser probes are green.
- [ ] GitHub CI is green.
- [ ] Production alias is verified, not only preview.
- [ ] No workaround has replaced root-cause repair.
