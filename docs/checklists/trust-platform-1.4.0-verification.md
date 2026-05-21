# Trust-platform 1.4.0 verification — WebGL2 source-material blank canvas

Status: Investigation only (no scena code or tests changed)
Date: 2026-05-21
Applies to: scena 1.4.0 (latest crates.io / GitHub release), trust-platform
trust-twin VS Code webview consumer, Playwright Firefox proof.
Evidence baseline: clean `main` at `15b623b` (the v1.4.0 ship state),
trust-platform downstream report dated 2026-05-21.

## Reporter summary (trust-platform)

After updating from scena 1.1.x to **scena 1.4.0** the trust-twin
`robot-cell` Playwright proof produces visibly empty PNGs for every
frame that uses the source glTF/PBR material path:

| Artifact | non-background ratio | max deviation | Verdict |
| --- | --- | --- | --- |
| `target/gate-artifacts/trust-twin-robot-cell-before.png` | 0.000000 | 3 | blank |
| `target/gate-artifacts/trust-twin-robot-cell-closed-grip.png` | 0.000000 | 3 | blank |
| `target/gate-artifacts/trust-twin-robot-cell-after.png` | 0.000000 | 3 | blank |
| `target/gate-artifacts/trust-twin-robot-cell-stale.png` | 0.284564 | — | visible |

Only the `stale` frame — which applies offline override materials —
renders visible pixels. The "before / closed-grip / after" frames feed
the renderer the verbatim source glTF/PBR materials; those render
nothing. The downstream renderer is exercising `Backend::WebGl2` in
Playwright Firefox.

This is consistent with the failure mode described in the existing
2026-05-16 investigations
(`docs/checklists/trust-platform-digital-twin-webgl-investigation.md`,
`docs/checklists/trust-platform-finding-2-webgl-materials.md`) but
verifies that the same failure persists after the 1.4.0 update.

## Trust-platform's APIs used (all 1.4 surfaces)

- `Assets::load_scene_with_report_options(..., AssetLoadOptions::default().with_strict_textures(true))`
  — strict-texture loading, recommendation #3 of the 2026-05-16 finding.
- `Background::Studio`
- `AutoExposureConfig::product_studio()`
- `PerspectiveCamera::standard()`
- Named light presets (`DirectionalLight::key_light`/`fill_light`/`rim_light`)
- Named material presets (`MaterialDesc::matte`/`plastic`/`metal`/`rubber`)
- Preserved source PBR material handles for packaged assets (no
  `asset_unlit_material()` override).

Their validation chain (`cargo check / cargo test / npm run
build:trust-twin / package-assets check`) passes; only the live
Playwright source-material render is blank.

## Cross-check against scena 1.4.0 source

### Capability surface still flags WebGL2 PBR as degraded

`src/diagnostics/capability_status.rs:3-5`:

```rust
pub(in crate::diagnostics) const fn forward_pbr_status(_backend: Backend) -> CapabilityStatus {
    CapabilityStatus::Degraded
}
```

`forward_pbr` returns `Degraded` for **every** backend at 1.4.0,
including `Backend::WebGl2`. `src/diagnostics/capabilities.rs:273-275`
still emits `DiagnosticCode::ForwardPbrDegraded` when that status is
set. The 2026-05-16 investigation's Finding #1 ("WebGL PBR path is not
usable for this proof yet") therefore still holds in 1.4.0 by scena's
own capability contract. The release notes for 1.4.0
(`docs/release-notes/v1.4.0.md`) do not claim
`forward_pbr` flipped to `Supported`, and the v1.4 checklist (sections
3.1 / 3.2) marks dense WebGL2 PBR as an open proof gap.

### `with_strict_textures(true)` does exist and is enforced

`src/assets/load.rs:24-143`, `src/assets/scene_loading.rs:274-291`:
strict mode promotes `ExternalImageMissing` to an error and the scene
load aborts. So if trust-platform's strict-texture call had picked up
missing PNGs, the loader would have failed — not silently rendered
nothing. **Strict load did not error**, so the 1.4.0 build is finding
the texture bytes; this rules out the "silent missing-image fallback"
chain (Finding #2 of 2026-05-16) as the *primary* cause for this
report. The 1.4.0 strict-texture surface effectively closes the
diagnosis gap recommendation #3 from May 16.

### WebGL2 path is still per-material bind groups with `Texture2d`

`src/render/gpu.rs:76-85` continues to force
`MaterialTextureBindingMode::Texture2d` for `Backend::WebGl2`. The
`output_shader_texture_2d.wgsl` shader workaround is still load-bearing
("wgpu 29's GL backend sampled material texture arrays as black in
Chromium WebGL2"). Trust-platform's Firefox run hits the same
codepath.

### IBL prefilter is heavily downsampled for `Backend::WebGl2`

`src/render/prepare/environment_prefilter.rs:315-330`:

```rust
EnvironmentPrefilterQuality::Reference => match stepped {
    0 => 32, 1|2 => 96, 3|4 => 192, 5|6 => 384, _ => 768,
},
EnvironmentPrefilterQuality::InteractiveWebGl2 => match stepped {
    0 => 4, 1|2 => 8, _ => 16,
},
```

`src/render/prepare/environment.rs:49-57` routes `Backend::WebGl2` to
the `InteractiveWebGl2` profile. With ~8 GGX importance samples at
metal-roughness mips, polished/metallic glTF material slots that
depend on env reflection produce flat or near-uniform-dark results.
This is a *visual quality* issue (not a "blank canvas" cause by
itself) but it compounds with the dense-scene rendering gaps and was
verified locally in a separate session (notes appended to
`docs/checklists/next-release-easy-use-and-state-of-the-art.md`,
"Live-demo smooth-metal investigation handoff (2026-05-21)").

### No scena CI proof that dense glTF source materials render in WebGL2

`tests/browser/m6_rust_wasm_renderer_probe.js` and the
`src/browser_probe/workflows/pbr/*` suite remain the only browser PBR
proofs. They cover small synthetic boxes, material presets, and
point/spot/normal-map probes — the same scope flagged in the May 16
investigation as not proving dense industrial assets. **No regression
test was added between 1.1.0 and 1.4.0 that exercises a dense
UR10/Schunk/YCB/table-class scene through `Backend::WebGl2`.** That is
exactly the gate the May 16 doc asked for as recommendation #1 ("Add
the dense WebGL2 repro first"), and it is still open.

### Stale docs

`docs/getting-started.md:17` still shows `scena = "1.3"`.
`docs/feature-flags.md` shows `scena = { version = "1.3", ... }` six
times (lines 15, 42, 48, 54, 63, 69). The 1.4.0 release notes
(`docs/release-notes/v1.4.0.md:9`) use the correct
`scena = "1.4"` snippet. Trust-platform's report listing
`docs/getting-started.md` as stale is correct.

### wasm-bindgen `snippets/` import is real and load-bearing

`demo/pkg/scena.js` line 2:

```js
import { scenaPrepareBrowserCanvasOutputColorSpace,
         scenaRefreshBrowserCanvasOutputColorSpace }
  from './snippets/scena-98d3370b3c3f0797/inline0.js';
```

`demo/pkg/snippets/scena-98d3370b3c3f0797/` is generated by
wasm-bindgen for the inline JS shim that scena 1.4 ships with for
`OutputColorSpace` / Display P3 canvas support (one of the new
`v1.4.0` features per the release notes). Downstream packagers must
copy `pkg/snippets/**` alongside `scena_bg.wasm` and `scena.js`; if
they don't, the browser fails before scena ever gets a chance to
render and the failure is silent (no scena diagnostic — the JS import
fails at module-load time and the page just stays blank).

Trust-platform's report says they fixed this in their packager
(`editors/vscode/scripts/build-trust-twin-webview.js` now copies
`snippets/`). That's a downstream fix, but the 1.4.0 install docs
do not mention this packaging requirement. Recommend adding a note
to `docs/getting-started.md` and/or `docs/release-notes/v1.4.0.md`.

## What this report does and does not prove

### Likely root cause (high confidence)

The trust-platform symptom — **WebGL2 + dense source-material glTF
scene → blank** — matches **the same `forward_pbr = Degraded` gap
scena has tracked since 1.1.0 and that 1.4.0 did not close.** The
2026-05-16 investigation explicitly listed "dense WebGL2 PBR repro"
as recommendation #1, and that test does not exist in 1.4.0.

The trust-platform consumer code is doing things correctly per the
1.4.0 surface (strict textures, named presets, preserved source PBR
handles). It is hitting a known unproven path.

### Contributing factors (medium confidence)

- The IBL prefilter undersample on `Backend::WebGl2` (8 samples at
  metal-roughness) further degrades any visible output that does get
  through.
- The depth-prepass gate referenced in the May 16 Finding #4 may have
  changed; the parallel dirty patch mentioned there (changing
  `DEPTH_PREPASS_MIN_PRIMITIVES` from 2 → 1) needs a 1.4.0 verification
  to see whether the trust-platform `RENDER-DEPTH-SENTINEL` is still a
  prepass disabler. Did not verify in this pass.

### Not the cause this time (low confidence)

- "Missing external PNG bytes" — trust-platform uses
  `with_strict_textures(true)` and the load did not error, so the
  texture bytes are reaching scena. The May 16 Finding #2 silent-
  fallback chain is plausible historically but not the current report.

## Outstanding scena work, ordered by what unblocks trust-platform

1. **Add the dense WebGL2 source-material regression** (recommendation
   #1 from 2026-05-16, still open). Required: load a UR10/Schunk/YCB-
   class glTF via `Assets::load_scene`, render through
   `Backend::WebGl2` in a real browser harness, compare to the headless
   `Reference`-quality render. **This is the deciding gate** —
   without it, every `forward_pbr` change is unverifiable for dense
   industrial scenes.

2. **Verify whether the depth-prepass gate** (`src/render/prepare/
   stats.rs::DEPTH_PREPASS_MIN_PRIMITIVES`, all-or-nothing eligibility)
   changed between 1.1.0 and 1.4.0. If still all-or-nothing, the
   trust-platform `RENDER-DEPTH-SENTINEL` and edge-overlay paths can
   still suppress the prepass for the whole scene. Confirmed pending.

3. **Flip `forward_pbr_status` once the dense WebGL2 proof passes** —
   only after #1 actually renders. Until then, every downstream
   consumer is using a self-declared Degraded path.

4. **Ship a WebGL2 IBL prefilter sample-count bump** so smooth
   metallic materials in a dense source-material scene resolve
   distinct env reflections instead of averaging to flat color. See
   the 2026-05-21 smooth-metal handoff appended to
   `docs/checklists/next-release-easy-use-and-state-of-the-art.md`.

5. **Fix stale docs**:
   - `docs/getting-started.md:17` → `scena = "1.4"`.
   - `docs/feature-flags.md` lines 15, 42, 48, 54, 63, 69 →
     `scena = { version = "1.4", ... }`.
   - Add a packaging note to `docs/getting-started.md` and/or
     `docs/release-notes/v1.4.0.md` calling out that wasm-bindgen
     `pkg/snippets/**` must be copied alongside `scena_bg.wasm`/
     `scena.js` (it is required by `OutputColorSpace` / Display P3
     canvas support). Trust-platform hit this; future consumers will too.

## Outstanding trust-platform work (not scena's responsibility)

These are downstream gaps the May 16 investigation already raised and
that the current report does not contradict:

- The Playwright analyzer keys on the deliberate cyan/orange override
  colors (`scripts/trust_twin_robot_cell_playwright.mjs:798-804`).
  Source PBR materials will fail those predicates even after scena's
  dense WebGL2 path renders correctly, because the source assets are
  not those colors. Split the analyzer into geometry-presence checks
  and override-color checks before judging post-fix output.
- Remove `RENDER-DEPTH-SENTINEL` from the proof once scena's depth
  prepass gate is verified — the sentinel itself is a line/wire
  primitive and can still suppress the prepass for unrelated opaque
  triangles in scenarios where the 1.4.0 gate is unchanged from
  1.1.0.
- Extend the trust-twin package-asset check to verify external
  `.png`/`.ktx2` URIs referenced by packaged glTF files are reachable
  through the VS Code webview origin, not just that the `.gltf` exists
  in the package.

## Reproducer scena needs (none currently exists)

A minimal scena-side reproducer would look like:

1. Bundle a representative dense industrial glTF (UR10 arm + Schunk
   gripper + YCB cracker box + table) under
   `tests/assets/gltf/industrial/`.
2. Add `tests/browser/m7_dense_webgl2_source_material_smoke.{html,js}`
   that loads it via `Assets::load_scene_with_report_options(...
   with_strict_textures(true))`, attaches a
   `PlatformSurface::browser_webgl2_canvas_element`, and captures the
   canvas as PNG.
3. Assert non-background ratio ≥ 0.10 (or a tighter scene-tuned bound).
4. Run as part of the M6 browser-proof CI lane.

Without this, "WebGL2 dense source-material PBR works" is unprovable
by gate, and downstream consumers like trust-platform have to discover
the gap by hitting it in production.

## Open questions

- Has the depth-prepass behavior changed between 1.1.0 and 1.4.0?
  (`DEPTH_PREPASS_MIN_PRIMITIVES`, all-or-nothing eligibility.)
- Does the trust-platform Playwright Firefox run capture a console /
  WGL error when the source-material frames render blank? Capturing
  `console.error` and any `RendererStats::diagnostics` payload from
  the live wasm during those frames would distinguish "renderer
  reported degraded" from "renderer succeeded but pixels were
  clipped/culled".
- Are the source glTF materials in the failing scene textured-PBR
  (base-color texture + normal map + metallic-roughness texture) or
  factor-only PBR? The investigation needs both cases tested.
