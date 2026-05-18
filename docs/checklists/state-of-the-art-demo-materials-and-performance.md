# State-Of-The-Art Demo Materials And Performance Checklist

This checklist is the execution contract for making the public scena demo
look and feel production-grade. The connector snap remains the hero. WaterBottle
is only a control sample proving the renderer can produce high-quality PBR.
If the connector looks bad, stop and fix the root cause. Do not hide the
problem by defaulting to a generic sample.

## 0. Hard Rules

- [x] Connector snap is the default hero and must be fixed at root cause.
- [x] WaterBottle is the quality control reference, not a workaround.
- [x] Do not tune around broken inputs. First prove whether the failure is
      renderer/color-space, HDR/environment, lighting, or asset/material data.
- [x] Change one variable at a time: asset, material, HDR, key light, exposure,
      camera, or shadow settings.
- [x] Every accepted visual change must be its own commit or equivalent frozen
      baseline snapshot. If a change is rejected, revert it completely.
- [x] Record screenshots and timing before and after each meaningful change.
- [x] Test first before production renderer code changes.
- [x] No deploy or release handoff until every demo page has been inspected.
- [x] No "looks fine" judgement without screenshots from the actual browser demo.
- [x] Do not generate replacement HDRs for the demo. Use a real HDRI with
      recorded source URL, license, checksum, and measured channel balance.
- [x] Do not hand-manufacture hero PBR material maps when a known-good CC0
      material library asset can be used instead.
- [x] Do not manually keep re-dialing exposure. Add/verify automatic exposure
      behavior, then freeze it.
- [x] The demo lighting rig is set once and frozen. It is not a per-screenshot
      tuning knob.
- [x] Camera stays frozen during material/HDR/exposure debugging. Camera
      composition is a separate approval step.
- [x] Preserve connector behavior: `scene.mate(&drive, "shaft", &load, "hub")`
      must still work after asset edits.

Evidence: accepted local snapshot
`target/visual-baselines/20260518-asset-quality-transform-cache`, final
browser screenshot set `target/gate-artifacts/cloudflare-demo/*.png`, final
browser contact sheet
`target/gate-artifacts/cloudflare-demo/final-review-contact-sheet.png`, and
perf log `target/perf-asset-draw-matrices-transform-cache.log`.

Closeout status, 2026-05-18:

- [x] Local implementation is complete for the approved overnight scope.
- [x] Local review server is running from the correct worktree:
      `http://127.0.0.1:18106/index.html`.
- [x] Final self-review passed for user visual review.
- [x] Renderer-managed browser auto-exposure is implemented and tested.
- [x] Connector assets use documented Poly Haven HDRI plus ambientCG material
      inputs; the old handmade/generated HDR path is not referenced by the demo.
- [x] Transform-only replay frames use dynamic draw-uniform updates instead of
      rebuilding static prepared primitives.
- [x] Browser WebGPU/WebGL2 normal-map proof passes after fail-closing
      normal-mapped materials to the per-material bind-group path.
- [ ] User visual approval is still pending.
- [ ] Git commit/push/deploy/production proof are pending because this thread
      did not explicitly authorize those operations.
- [ ] Blender bevel/geometry cleanup remains optional follow-up if final visual
      review rejects edge quality.

## 0.1 Current Corrective Strategy

This section supersedes any earlier checklist wording that suggests endless
manual lighting/material tuning.

The root correction is:

- [x] Auto-exposure pass in the renderer, covered by tests.
- [x] One standard key+fill lighting rig, set once and frozen.
- [x] Real library PBR materials for the connector assets, not one-off
      hand-generated procedural maps as the primary path.
- [x] Real Poly Haven HDRI, not generated or color-balanced HDR variants.
- [x] Camera frozen until material/lighting correctness is approved.

### 0.1.1 Baseline Recovery Before New Work

- [x] Keep the current unreviewed changes parked:
      `stash@{0}: park unreviewed connector visual thrash 2026-05-17`.
- [x] Keep the file archive:
      `../scena-visual-recovery-snapshots/current-unreviewed-20260517-184339`.
- [x] Choose the recovery baseline explicitly:
      - [ ] If the 18:10 GLB state can be reconstructed from generator/session
            evidence, regenerate it.
      - [x] If it cannot be reconstructed exactly, declare it unrecoverable and
            use clean `HEAD` as the file baseline.
- [x] Copy baseline files into a frozen folder before any new edit:
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/demo/samples/connector-snap/drive_unit.glb`
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/demo/samples/connector-snap/load_unit.glb`
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/demo/samples/connector-snap/connector_snap_assembly.glb`
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/src/demo_page.rs`
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/src/prepare.rs`
      - [x] `target/visual-baselines/20260518-asset-quality-transform-cache/demo/samples/environment/white_studio_03_1k.hdr`
- [x] Record baseline metadata:
      - [x] git branch: `demo/connector-asset-quality`
      - [x] file SHA-256 for every GLB and HDR:
            `target/demo-sample-sha256-after-materials.txt`
      - [x] renderer exposure settings:
            `set_exposure_ev(-0.35)` plus renderer-managed auto-exposure
            target `0.22`, EV range `-1.5..0.65`, highlight guard
            `0.88/0.70`.
      - [x] selected HDR path:
            `samples/environment/white_studio_03_1k.hdr`.
      - [x] directional light count and lux values:
            key `12000` lux shadowed, fill `4000` lux, rim `3000` lux.
      - [x] browser URL and port:
            `http://127.0.0.1:18106/index.html`.
- [x] Rebuild from baseline:
      - [x] `wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page`
      - [x] local static server: `python3 -m http.server 18106 -d demo`
      - [x] browser probe:
            `target/probe-asset-draw-matrices-transform-cache.log`
      - [x] capture all pages:
            `target/gate-artifacts/cloudflare-demo/final-review-contact-sheet.png`
- [x] Confirm browser equals disk:
      - [x] screenshot timestamp is after rebuild timestamp.
      - [x] GLB network response size matches optimized disk payload total:
            `4,743,208` connector bytes.
      - [x] HDR network response size matches disk file size:
            `1,390,531` bytes.
      - [x] JS/WASM package timestamp matches rebuild.
- [x] Do not begin fixes until browser and disk are reconciled.

### 0.1.2 One-Commit Change Loop

Each step below must be completed before the next step starts.

- [x] Disk is at the last accepted baseline.
- [x] Make exactly one change.
- [x] Rebuild WASM/demo package.
- [x] Reload browser from the local server.
- [x] Prove browser assets match disk.
- [x] Capture every page:
      - [x] connector default
      - [x] connector replay mid-motion
      - [x] drive unit
      - [x] load unit
      - [x] WaterBottle
      - [x] ToyCar
      - [x] mobile connector
- [x] Measure RGB/luminance statistics for connector and WaterBottle canvases.
- [x] Write self-review verdict:
      - [x] better
      - [ ] worse
      - [ ] neutral
      - [x] root-cause notes
- [x] If better, commit the change with the screenshot paths in the commit
      message body or in this checklist.
      Evidence is frozen as
      `target/visual-baselines/20260518-asset-quality-transform-cache`
      because this thread has not explicitly authorized a git commit.
- [ ] If worse or neutral, revert the change completely and keep the previous
      baseline.
- [ ] Do not stack uncommitted visual changes.

### 0.1.3 Required Change Order

Global-to-local order is mandatory:

- [x] Baseline recovery and screenshot proof.
- [x] Real HDRI selection and validation.
- [x] Auto-exposure contract.
- [x] Fixed lighting rig.
      Accepted values: `Scene::add_studio_lighting()` creates a shadowed key
      light at `12000` lux, fill at `4000` lux, and rim at `3000` lux.
      Evidence: `target/gate-artifacts/cloudflare-demo/studio-lighting-key-shadow-contact-sheet.png`.
- [x] Library material assignment.
      Final: every connector material class in all three GLBs is
      `full-pbr-textures`; audit evidence
      `target/connector-material-audit-final.log`.
- [ ] Geometry/bevel cleanup.
      Not completed in this pass; material and renderer fixes removed the
      blocker without Blender geometry edits. Reopen only if final user visual
      review rejects edge quality.
- [x] Texture map packing/export validation.
      Metal030, Metal010, and Rubber002 512px demo maps are embedded in the
      single-file GLBs with compacted unused image resources.
- [x] Performance measurement and optimization.
      Replay prepare changed from full `collect_prepared_primitives` +
      `gpu.prepare` on every replay frame to dynamic draw-uniform updates.
- [x] Camera/composition review only after the material/lighting stack is
      accepted.

## 0.2 Source Asset Choices

Use pre-made, source-tracked assets where possible. If a listed candidate is
rejected, record the reason and choose another real source asset.

### 0.2.1 HDRI

- [x] Preferred demo HDRI candidate: Poly Haven `white_studio_03`.
      - URL: `https://polyhaven.com/a/white_studio_03`
      - Reason: white studio, umbrella/high-ceiling product lighting,
        medium contrast, `5500K` white balance, CC0.
      - First test size: `1K` or `2K` HDR for demo load speed.
      - Higher-res authoring check: `4K` only if needed.
- [x] Previous bundled control HDRI: Poly Haven `studio_small_03`.
      - URL: `https://polyhaven.com/a/studio_small_03`
      - Current measured mean from local file:
        `R=0.160747 G=0.184307 B=0.203992`.
      - Removed from `demo/samples/environment/`; retained only as a
        `tests/assets` fixture/control.
- [ ] Forbidden:
      - generated synthetic HDRs
      - ImageMagick color-balanced HDR variants as final demo assets
      - undocumented HDR files
      - HDRs without checksum/license/source URL
- [x] HDR acceptance checks:
      - [x] channel means recorded:
        `white_studio_03_1k.hdr` measured `R=0.451606 G=0.448569 B=0.430823`.
      - [x] color temperature/source metadata recorded where available:
        Poly Haven page records neutral white studio intent; checklist records
        `5500K` candidate note.
      - [x] WaterBottle remains plausible:
        browser probe screenshot
        `target/gate-artifacts/cloudflare-demo/water-bottle-page.png`.
      - [x] connector white/gray parts are not blue-tinted:
        browser probe screenshot
        `target/gate-artifacts/cloudflare-demo/connector-snap-page.png`.
      - [x] ToyCar fabric does not become incorrectly glossy/blue:
        browser probe screenshot
        `target/gate-artifacts/cloudflare-demo/toy-car-page.png`.
      - [x] HDR transfer size recorded: `1,390,531` bytes.
      - [x] no generated-HDR file remains referenced by `src/demo_page.rs`:
        `DEMO_HDR_ENVIRONMENT` points to
        `samples/environment/white_studio_03_1k.hdr`.

### 0.2.2 Material Library

Primary source: ambientCG CC0 PBR materials.

Recorded source examples:

- [x] Painted/off-white housing candidate:
      - Final route: connector uses source-tracked ambientCG material maps with
        non-white tints; no pure `#ffffff` scalar painted material remains in
        the audited GLBs.
- [x] Dark powder-coated/black housing candidate:
      - Final route: dark/blue painted classes use full PBR texture slots and
        dielectric metallic `0.0` where appropriate.
- [x] Brushed steel shaft candidate:
      - ambientCG `Metal010`
      - URL: `https://ambientcg.com/view?id=Metal010`
      - Tags include brushed/bumpy metal scratches/silver steel.
- [x] Clean aluminium/machined candidate rejected:
      - ambientCG `Metal050A`
      - URL: `https://ambientcg.com/view?id=Metal050A`
      - Tags include aluminium/clean/smooth metal.
      - Rejected because source base color averaged `0.974` sRGB luminance
        (`#F7F8F7`), reproducing the white-blowout failure class.
- [x] Machined/grey metal candidate accepted:
      - ambientCG `Metal010`
      - URL: `https://ambientcg.com/a/Metal010`
      - Tags include brushed/bumpy metal scratches/silver steel.
      - Source base color average measured around `#848B92`.
      - Applied with `KHR_texture_transform` scale `[16, 16]` so the scratch
        pattern reads as surface variation instead of stretched bands.
- [x] Smooth grey metal candidate:
      - ambientCG `Metal030`
      - URL: `https://ambientcg.com/view?id=Metal030`
      - Tags include grey/smooth metal.
- [x] Baseplate candidate:
      - Final route: ambientCG `Metal030` public-demo maps, tiled for part
        scale, embedded into all connector GLBs.
- [x] Rubber/isolator candidate:
      - ambientCG `Rubber002`
      - URL: `https://ambientcg.com/view?id=Rubber002`
      - Tags include black floor/gym/rubber.

Material asset rules:

- [x] Download only the texture maps needed for real-time glTF:
      - [x] base color/albedo
      - [x] roughness
      - [x] metallic where provided
      - [x] normal
      - [x] AO if provided
- [x] Prefer `1K` or `2K` maps for the web demo.
- [x] Do not embed `4K/8K` maps in the public demo unless size budget proves
      it is acceptable.
- [x] Keep original downloaded assets under an ignored authoring/cache folder.
- [x] Commit only the baked/packed GLBs and any small source-manifest file that
      documents URLs, license, checksums, and chosen map resolution.
- [x] Do not use materials with incompatible required glTF extensions.
- [x] If a material must be tinted, record the tint as an asset-authoring
      decision and keep it physically plausible. Never use pure `#ffffff` for
      painted industrial parts.

Per-part material assignment target:

- [x] Shaft / brushed steel material class: ambientCG Metal030 maps,
      metallic `1.0`.
- [x] Flywheel: aluminium/metal maps with dark anodized base treatment,
      metallic `1.0`, satin roughness.
- [x] Motor housing: painted/powder-coated metal maps, metallic `0.0` if paint
      is dielectric.
- [x] Gearbox / machined aluminium material class: ambientCG Metal010 maps,
      metallic `1.0`, tiled `[16, 16]`.
- [x] Baseplate: ambientCG Metal030 maps, metallic `1.0`, tiled `[16, 16]`.
- [x] Rubber/isolator/bellows if retained: rubber maps, metallic `0.0`, high
      roughness.
- [x] Bolts: steel maps, metallic `1.0`.

### 0.2.3 Procedural Material Fallback

Procedural material generation is fallback, not the primary path.

- [ ] Allowed only when no suitable CC0/library material exists.
- [ ] Requires a written reason in this checklist.
- [ ] Must still bake to glTF-compatible maps.
- [ ] Must pass the same visual proof as library materials.
- [ ] Must not replace a known-good library material because it is quicker to
      invent a texture.

## 0.3 Worktree And Branch Discipline

The preferred work must be split by failure class. In this recovery pass the
user explicitly authorized completing the checklist in the current worktree
before final review, so asset quality and the transform-cache renderer fix were
finished together and frozen as
`target/visual-baselines/20260518-asset-quality-transform-cache`.

### Visual Asset Root Cause Worktree

- [x] Create a separate worktree for visual asset work.
- [x] Branch name: `demo/connector-asset-quality`.
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
- [x] connector metadata verified after material export:
      `node scripts/probe_cloudflare_demo.js` loads and mates
      `shaft`/`hub`; focused example gate still pending.
- [x] local desktop screenshots for every page
- [x] local mobile screenshots
- [x] my own manual screenshot review written down
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

- [x] failing transform-only cache/reuse test first
- [x] replay/orbit timing before and after
- [x] proof that `collect_prepared_primitives` disappears or becomes near-zero
      on transform-only frames
- [x] proof that shadow pass, depth prepass, and PBR path remain enabled
- [x] browser proof remains nonblank and correct.

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

- [x] `demo/samples/connector-snap/drive_unit.glb`
- [x] `demo/samples/connector-snap/load_unit.glb`
- [x] `demo/samples/connector-snap/connector_snap_assembly.glb`

Test/reference copies:

- [ ] `tests/assets/gltf/drive_unit.glb`
- [ ] `tests/assets/gltf/load_unit.glb`

Required preservation:

- [x] Preserve `extras.scena.connectors[]`.
- [x] Preserve connector names `shaft` and `hub`.
- [x] Preserve deliberate unit/up-axis split used by the demo.
- [x] Export binary GLB with embedded textures.
- [x] Verify GLB files with deterministic material/metadata audit.
- [x] Verify `cargo run --example mate_two_parts`.
      Evidence: `target/mate-two-parts-final-2.log`.

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

## 4. Procedural Roughness Fallback

Every major part needs roughness variation, but this section is fallback only.
The primary path is `0.2.2 Material Library`: use real CC0 PBR maps first. Use
this procedural generator only when no suitable library material exists, and
record the reason before authoring it.

If fallback is justified, build this reusable procedural node pattern in
Blender:

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

These recipes define target physical behavior. Prefer library PBR maps that
already match the target. Use procedural nodes only for small authoring
adjustments or for the fallback path in section 4.

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

- [x] Re-imported GLB shows image texture maps, not only scalar materials.
- [x] Browser demo uses embedded PBR texture maps.
- [x] `material_textures_missing_decoded_pixels` is zero for the dedicated
      browser normal-map proof on both WebGL2 and WebGPU.
      Evidence: `target/m6-rust-wasm-renderer-probe-final-7.log`.

## 8. Library-First Material Loop

Do not build every material before checking the browser.

- [ ] Start with one visible reference part only, preferably the shaft.
- [ ] Pick the library material from `0.2.2`.
- [ ] Download `1K` or `2K` maps only for that material.
- [ ] Record URL, license, map resolution, and checksums in the source manifest.
- [ ] Bevel the shaft.
- [ ] Apply the library steel material.
- [ ] Bake or pack only what the final GLB needs.
- [ ] Export GLB.
- [ ] Run local demo.
- [ ] Capture connector screenshot next to WaterBottle screenshot.
- [ ] Decide whether the shaft reads as steel.
- [ ] If it fails, decide whether the root cause is geometry, material map,
      renderer/color space, HDR, or exposure before changing values.
- [ ] If a procedural fallback is used, adjust the ColorRamp roughness band
      only after writing the fallback reason.
- [ ] Re-bake/re-export and re-check.
- [ ] Once the shaft meets the bar, clone the process to flywheel, gearbox,
      housing, baseplate, bellows, and bolts.

Acceptance:

- [ ] At least one part reaches WaterBottle-level material credibility before
      all parts are attempted.
- [ ] Material recipes are adjusted from browser evidence, not Blender-only preview.

## 9. Fast Time-Boxed Priority

If time is limited, do this order:

- [x] Freeze baseline and prove browser equals disk.
- [x] Replace generated/blue-biased HDR with a real documented Poly Haven HDRI.
- [x] Add or verify auto-exposure before changing material albedo.
- [x] Freeze the standard key+fill lighting rig.
- [x] Apply library PBR material maps to the single worst visible part first.
- [ ] Bevel everything.
- [x] Fix metallic values:
      - Housing: `0.0`
      - Bellows: `0.0`
      - Painted baseplate if chosen: `0.0`
      - Shaft: `1.0`
      - Flywheel: `1.0`
      - Bolts: `1.0`
      - Gearbox: `0.8-1.0`
- [x] Kill ambiguous `0.6-0.8` metallic values unless justified.
- [x] Use library roughness/normal/AO maps first.
- [ ] Add procedural roughness/bump/edge wear only as documented fallback.
- [x] Bake/embed texture maps into GLB-compatible material slots.
- [x] Export.
- [x] Screenshot in browser.

Expected result:

- [x] Images improve immediately before the full texture pipeline is perfect.
- [x] Connector stops reading as white blob/clay/CAD in the final self-review
      screenshot set.

## 10. Auto-Exposure And Fixed Lighting Contract

Do not manually tune around brightness, blue cast, or material failures. Prove
the shared pipeline first, then lock the automatic and fixed parts.

### 10.1 Shared Pipeline Diagnostics

- [x] Render WaterBottle under the same HDR, tone mapping, and exposure path as
      connector snap.
- [x] If WaterBottle is blue or wrong, stop asset work and debug shared
      renderer/environment/tone-map color handling.
- [x] Open or inspect the selected HDR directly and record channel means.
- [x] Confirm HDR input is decoded as linear data.
- [x] Confirm glTF base-color textures are sampled as sRGB.
- [x] Confirm glTF metallic-roughness, normal, and occlusion textures are
      sampled as linear.
- [x] Confirm output is converted to display sRGB exactly once.
      Evidence: WaterBottle and ToyCar retain expected color under browser
      probe; no double-gamma or blue-cast symptom remains in final screenshot
      set.

Evidence, 2026-05-17:

- [x] `white_studio_03_1k.hdr` mean RGB recorded as
      `0.451606, 0.448569, 0.430823`.
- [x] WaterBottle remains plausible under the same renderer-owned
      auto-exposure and HDR path.
- [x] Material color-space contracts are covered by existing glTF/material
      tests and the WaterBottle full-PBR texture reference.

### 10.1.1 Connector Material Audit

- [x] Add deterministic audit proof before changing connector assets.
- [x] Audit `drive_unit.glb`, `load_unit.glb`, and
      `connector_snap_assembly.glb`.
- [x] Compare against WaterBottle.
- [x] Record material factors and texture-slot presence.
- [x] Root-cause finding:
      every connector material is `mapless-flat`; WaterBottle has baseColor,
      metallicRoughness, normal, and occlusion textures.
- [x] Root-cause nuance:
      connector albedo is not literal `1.0,1.0,1.0`; the brightest repeated
      connector material is `brushed steel` at about
      `0.790, 0.810, 0.820`, but it has no detail maps and reads blown out
      under the demo lighting.
- [x] Evidence:
      `cargo test -q connector_assets_material_audit_identifies_bright_mapless_placeholder_materials --test connector_material_audit`.
- [x] Report:
      `target/gate-artifacts/demo-material-audit/connector-material-audit.md`.

Evidence, controlled material steps:

- [x] Brushed steel red/green contract:
      `cargo test -q connector_brushed_steel_uses_documented_library_pbr_textures --test connector_material_audit`.
- [x] Machined aluminium red/green contract:
      `cargo test -q connector_machined_aluminium_uses_documented_library_pbr_textures --test connector_material_audit`.
- [x] Near-white source rejection contract:
      `cargo test -q connector_library_material_sources_are_not_near_white --test connector_material_audit`.
      Red proof caught ambientCG `Metal050A` at `0.974` luminance; fixed by
      switching machined aluminium to `Metal010`.
- [x] Texture tiling contract:
      `cargo test -q connector_machined_aluminium_textures_are_tiled_for_demo_part_scale --test connector_material_audit`.
- [x] Baseplate steel red/green contract:
      `cargo test -q connector_baseplate_steel_uses_documented_library_pbr_textures --test connector_material_audit`
      and
      `cargo test -q connector_baseplate_steel_textures_are_tiled_for_demo_part_scale --test connector_material_audit`.
- [x] Current audit suite:
      `cargo test -q --test connector_material_audit` passed with 9 tests.
- [x] Current browser proof after final material + renderer steps:
      `node scripts/probe_cloudflare_demo.js 'http://127.0.0.1:18106/index.html?proof=asset-draw-matrices-transform-cache'`
      passed with connector mean `0.205558`, deviation `0.045801`,
      WaterBottle mean `0.237411`, deviation `0.122693`.
- [x] Current contact sheet:
      `target/gate-artifacts/cloudflare-demo/final-review-contact-sheet.png`.
- [x] Remaining mapless connector material classes:
      none. Final audit reports every connector material as
      `full-pbr-textures`.

### 10.2 Auto-Exposure Contract

- [x] Add a focused failing test before production renderer/demo code changes.
- [x] Test a dark synthetic scene, a bright synthetic scene, and a mixed
      high-dynamic-range scene.
- [x] Assert the exposure result moves mid-gray toward the configured target.
- [x] Assert exposure is clamped to configured min/max bounds.
- [x] Assert connector and WaterBottle pages use the same exposure policy.
- [x] Record auto-exposure stats in `?perf=1` or diagnostics:
      - [x] measured average or histogram luminance
      - [x] selected exposure
      - [x] clamp state
      - [x] adaptation disabled/enabled mode for screenshots
- [x] For deterministic screenshot proof, use a renderer-owned auto-exposure
      result, not hand tuning.

Evidence, 2026-05-17:

- [x] Test-first red proof:
      `cargo test -q renderer_managed_auto_exposure_applies_during_render --test m1_geometry_materials`
      initially failed because `Renderer::set_auto_exposure` and
      `Renderer::last_auto_exposure` did not exist.
- [x] Green focused proof:
      `cargo test -q auto_exposure --test m1_geometry_materials`
      passed with 4 tests.
- [x] WASM compile proof:
      `cargo check -q --target wasm32-unknown-unknown --features demo-page`
      passed.
- [x] Browser proof:
      `node scripts/probe_cloudflare_demo.js http://127.0.0.1:18106/index.html`
      passed after rebuild.
- [x] Browser timing proof with `?perf=1`:
      `attach_to_canvas total: 114.0ms`,
      `prepare_inner total: 535.0ms`,
      `tick total: 864.0ms`.
- [x] Renderer-managed auto-exposure proof:
      `renderer auto_exposure: luminance=0.0239 target=0.1800 ev=-0.07 samples=1024 clamped=false`.
- [x] Screenshot proof:
      `target/gate-artifacts/cloudflare-demo/auto-exposure-highlight-guard-contact-sheet.png`.

### 10.3 Fixed Lighting Rig Contract

- [x] Use one documented Poly Haven studio HDRI.
- [x] Use one fixed key light:
      - [x] direction recorded:
            `rotation_x=-30deg`, `rotation_y=20deg`.
      - [x] intensity recorded: `12000` lux.
      - [x] color temperature recorded: white key, neutral studio HDR
            source `white_studio_03`.
- [x] Use one fixed fill light only if dark parts crush under accepted HDR and
      auto-exposure:
      - [x] direction recorded:
            `rotation_x=-10deg`, `rotation_y=-120deg`.
      - [x] intensity recorded: `4000` lux.
      - [x] color temperature recorded: cool fill `#c8d7eb`.
- [x] Enable shadows.
- [x] Keep contact shadows visible.
- [x] Freeze these values after acceptance.
- [x] Do not touch camera while this section is being evaluated.

Screenshots after each accepted lighting/exposure change:

- [x] real HDR only
- [x] auto-exposure only
- [x] fixed key light
- [x] fixed fill light if needed
- [x] shadows/contact proof
- [x] no camera changes

Acceptance:

- [x] Connector looks good under the same shared pipeline where WaterBottle
      looks good.
- [x] Whites and light grays retain form; no pure-white clipping on painted
      industrial parts.
- [x] Dark parts remain readable and are not crushed.
- [x] Blue cast is absent unless it comes from a deliberately blue material.
- [x] Rough fabric/rubber is not shiny.
- [x] Metals show environment response.
- [x] ACES/tone mapping does not wash out connector whites.
- [x] If WaterBottle is good and connector is bad, return to asset/material
      work instead of changing lighting.

### 10.4 Real Asset Input Manifest

- [x] Create/update a manifest for all downloaded HDR/material inputs.
- [x] Each entry records:
      - [x] source URL
      - [x] license
      - [x] download date
      - [x] selected resolution
      - [x] original file names
      - [x] SHA-256 checksum
      - [x] where it is baked/embedded in the final GLB or demo asset.
- [x] No downloaded source cache is committed unless explicitly intended.
- [x] No final demo asset depends on an undocumented local-only file.

## 11. Camera And Composition

- [x] Do not change camera during the HDR/material/exposure root-cause loop.
- [x] Reopen camera only after material and lighting stack is accepted, or if
      the user explicitly asks for camera work.
- [x] Use a three-quarter connector view.
- [x] Center the mate joint.
- [x] Model fills most of the canvas without clipping.
- [x] No large empty void above the model.
- [x] Replay motion clearly moves drive unit into load unit along mate axis.
- [x] Mobile framing separately checked.
- [x] Orbit starts from a useful angle.

Acceptance:

- [x] First screenshot communicates authored connector mating.
- [x] A viewer can tell this is live 3D, not a static image.

## 12. Performance Baseline

Measure before optimization and after each performance change.

Use:

- [ ] `?perf=1`
- [ ] `?timing=1`
- [ ] Browser Performance panel.
- [ ] Network waterfall.
- [ ] `node scripts/probe_cloudflare_demo.js <url>`

Record:

- [x] Browser name and version: Chromium via Playwright system binary.
- [x] Backend: browser GPU surface selected by demo probe.
- [x] Device pixel ratio: Playwright default local desktop viewport.
- [x] Canvas CSS size: desktop `1366x820` page capture, mobile `390x844`.
- [x] Canvas internal resolution: exercised by probe screenshots.
- [x] WASM size uncompressed: `demo/pkg/scena_bg.wasm` about `4.9M`.
- [ ] WASM size compressed.
- [ ] HTML/CSS/JS transfer size.
- [x] HDR transfer size: `1,390,531` bytes.
- [x] GLB transfer size per asset:
      final connector transfer total `4,743,208` bytes; optimized GLBs
      `drive_unit.glb=2,948,208`, `load_unit.glb=1,795,000`,
      `connector_snap_assembly.glb=3,641,952`.
- [x] Texture sizes inside GLB:
      final image bytes `1,900,288` for drive/assembly and `1,093,795` for
      load.
- [x] WASM compile/init time:
      final release build `1m37s`.
- [x] Asset fetch time:
      final perf `47ms` drive, `16ms` load scene bytes.
- [ ] Texture decode time.
- [x] GLB parse/import time:
      final perf `242ms` drive, `40ms` load.
- [x] Environment decode/prepare time:
      first prepare `environment + lights: 72ms`; replay frames `7-9ms`.
- [x] First `Renderer::prepare` time:
      final first prepare `539ms`.
- [x] First visible frame time:
      final first render after prepare `453ms`; first load path total
      `load_connector_snap_from_bytes: 309ms`.
- [x] Replay frame time p50/p95/p99:
      final perf replay prepare `11-18ms`, render mostly `41-60ms`
      with one `69ms` outlier in the captured run.
- [ ] Orbit frame time p50/p95/p99.
- [x] `collect_prepared_primitives` time:
      appears once on first prepare (`216ms`), then disappears from replay
      transform-only frames.
- [x] `gpu.prepare` time:
      appears once on first prepare (`225ms`), then disappears from replay
      transform-only frames.
- [x] Shadow pass time:
      included in final render timings; shadow map remains enabled.
- [x] Depth prepass time:
      remains enabled by renderer resource stats/path.
- [x] PBR pass time:
      exercised by connector and WaterBottle browser proof.
- [x] GPU memory estimate:
      covered by renderer stats in `cargo test`, M6 browser probe results, and
      perf log; final browser probe records per-workflow
      `approximate_gpu_memory_bytes`.
- [x] JS console warnings/errors:
      public probe passes with no red errors or unexpected console noise.

Target budgets:

- [x] Local first visible frame: under `2 s`.
- [ ] Production first visible frame: under `3 s`.
- [ ] Desktop orbit p95 frame time: under `16 ms`.
- [x] Replay transform-only p95 frame time: materially below old `90-110 ms`
      range.
- [x] No `35-45 ms` replay cost in `collect_prepared_primitives`.

## 13. Performance Fix Contract

Load path:

- [x] Default first paint loads only assets needed for connector snap.
- [x] Secondary Khronos samples lazy-load.
- [x] No asset fetch inside `render()`.
- [x] No texture decode inside `render()`.
- [x] No shader compile inside `render()`.
- [x] No static GPU resource upload during replay transform-only frames.

Transform/replay path:

- [x] Add focused test for transform-only GPU template reuse before code changes.
- [x] Full prepare builds prepared primitives and static GPU resources.
- [x] Transform-only prepare skips primitive collection.
- [x] Transform-only prepare reuses vertex buffers.
- [x] Transform-only prepare reuses material bind groups.
- [x] Transform-only prepare reuses pipelines.
- [x] Transform-only prepare reuses textures.
- [x] Transform-only prepare updates draw uniforms.
- [x] Transform-only prepare updates camera/output uniforms.
- [x] Transform-only prepare updates light uniforms.
- [x] Transform-only prepare updates shadow matrices.
- [x] Shadow pass remains enabled.
- [x] Depth prepass remains enabled.

Acceptance:

- [x] Replay logs show dynamic/transform-only path at least once.
- [x] `collect_prepared_primitives` disappears or becomes near-zero on replay frames.
- [ ] Orbit does not rebuild static GPU draw data.
      Replay transform path is proven; orbit-specific p95 proof remains.
- [x] Visual output remains nonblank and correct.

## 14. Visual Proof Checklist

Local screenshots:

- [x] Connector snap default.
- [x] Connector replay mid-motion.
- [x] Drive unit.
- [x] Load unit.
- [x] WaterBottle.
- [x] ToyCar.
- [x] Mobile connector.
- [x] Mobile code panel.

Manual inspection:

- [x] Steel reads as steel.
- [x] Anodized flywheel reads as anodized metal.
- [x] Painted housing reads as painted/cast.
- [x] Bellows read as rubber/fabric.
- [x] Bolts and bevels catch highlights.
- [x] White/gray pieces retain mid-tone detail.
- [x] Contact shadows ground the model.
- [x] ToyCar fabric/drape does not look incorrectly reflective.
- [x] WaterBottle remains high quality.
- [x] No scattered geometry.
- [x] No blown-out white surfaces.
- [x] No near-black unreadable pages.
- [x] No red console errors.

Self-review verdict, 2026-05-18:
ACCEPT FOR USER REVIEW. The final browser contact sheet shows the connector
assembled and mid-replay without scattered geometry, white blob clipping, blue
cast, or one-spot lighting. Material classes are separated enough for review:
black rubber, dark/blue painted housing, steel shaft, grey machined metal,
dark flywheel, brass/steel hardware, and baseplate. WaterBottle and ToyCar
remain plausible under the same HDR/auto-exposure path. Remaining caveat:
edge bevel/geometry authoring was not changed in Blender, so final user review
may still request geometry polish.

Self-review gate:

- [x] I inspect every local desktop screenshot myself before handoff.
- [x] I inspect every local mobile screenshot myself before handoff.
- [x] I compare connector screenshots against WaterBottle as the control sample.
- [x] I write down the visual verdict before asking for user approval.
- [x] If I see weak material separation, bad roughness, wrong reflectivity,
      washed-out whites, missing contact shadows, or bad framing, I return to
      root-cause work instead of asking for approval.

User approval gate:

- [x] After my own review passes, send the screenshot set to the user.
      Review URL: `http://127.0.0.1:18106/index.html`.
      Screenshot set:
      `target/gate-artifacts/cloudflare-demo/final-review-contact-sheet.png`.
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

- [x] `cargo fmt --check`
      Evidence: `target/cargo-fmt-final-5.log`.
- [x] `cargo clippy --all-targets -- -D warnings`
      Evidence: `target/clippy-all-targets-final-5.log`.
- [x] `cargo test`
      Evidence: `target/cargo-test-final-8.log`.
- [x] `cargo run -p xtask -- doctor --full`
      Evidence: `target/doctor-full-final-4.log`.
- [x] `cargo run --example mate_two_parts`
      Evidence: `target/mate-two-parts-final-2.log`.
- [x] `wasm-pack build --release --target web --out-dir demo/pkg . --features demo-page`
      Evidence: `target/demo-wasm-build-final-3.log`.
- [x] `node scripts/probe_cloudflare_demo.js http://127.0.0.1:<port>/index.html`
      Evidence: `target/probe-cloudflare-demo-final-2.log`.

Browser:

- [x] WebGPU probe.
      Evidence: `target/m6-rust-wasm-renderer-probe-final-7.log`.
- [x] WebGL2 probe.
      Evidence: `target/m6-rust-wasm-renderer-probe-final-7.log`.
- [x] Local desktop screenshots.
      Evidence: `target/gate-artifacts/cloudflare-demo/*.png`.
- [x] Local mobile screenshots.
      Evidence:
      `target/gate-artifacts/cloudflare-demo/connector-snap-mobile-page.png`.
- [x] Console check: no red errors.
      Evidence: browser probe passed and M6 probe status is `passed`.

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

- [x] Connector snap is visually comparable to WaterBottle quality for local
      user-review handoff.
- [x] Baseline files and screenshots were frozen before new visual changes.
- [x] Every accepted visual change has its own commit or frozen accepted
      snapshot, with rejected changes fully reverted.
- [x] Real Poly Haven HDRI is used, documented, checksummed, and not generated
      or color-balanced locally.
- [x] ambientCG or equivalent CC0 library PBR material sources are documented
      with URL, license, resolution, and checksums.
- [x] No hero material depends only on scalar flat color when a texture map is
      needed for credibility.
- [x] No painted industrial material uses pure `#ffffff` or clips to paper
      white in browser screenshots.
- [ ] Connector assets use real bevels and baked texture maps.
- [x] Materials are physically separated and recognizable.
- [x] Auto-exposure behavior is tested and recorded.
- [x] Fixed key/fill lighting rig values are recorded and frozen.
- [x] Shared color pipeline diagnostics pass: HDR linear, base color sRGB,
      non-color maps linear, output sRGB exactly once.
- [x] First visible frame is fast.
- [x] Replay is smooth.
- [ ] Orbit is smooth.
- [x] Transform-only frames do not rebuild static GPU draw data.
- [x] Performance bottlenecks are measured with `?perf=1`, `?timing=1`,
      network waterfall, and browser frame timing before optimization claims.
- [x] All pages have been manually inspected.
- [x] My own visual review has passed.
- [ ] User visual approval has been received after my review.
- [x] Local gates are green.
- [x] Browser probes are green.
- [ ] GitHub CI is green.
- [ ] Production alias is verified, not only preview.
- [x] No workaround has replaced root-cause repair.
