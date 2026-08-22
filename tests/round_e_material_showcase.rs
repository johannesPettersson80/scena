use std::{collections::BTreeSet, fs};

use scena::material_showcase::{
    MaterialShowcaseGeometry, glass_background_target_bars, material_preset_showcase,
};

const REQUIRED_IDS: &[&str] = &[
    "matte",
    "plastic",
    "metal",
    "rough_metal",
    "chrome",
    "brushed_steel",
    "clearcoat_plastic",
    "satin",
    "leather",
    "clear_glass",
    "frosted_glass",
    "rubber",
];

#[test]
fn round_e_material_showcase_is_shared_and_not_a_single_shape_grid() {
    let presets = material_preset_showcase();
    let ids = presets.iter().map(|preset| preset.id).collect::<Vec<_>>();
    assert_eq!(ids, REQUIRED_IDS);

    let geometries = presets
        .iter()
        .map(|preset| preset.geometry)
        .collect::<BTreeSet<_>>();
    assert!(
        geometries.len() >= 8,
        "Round E material proof needs material-specific geometry, got {geometries:?}"
    );
    assert!(geometries.contains(&MaterialShowcaseGeometry::CurvedPart));
    assert!(geometries.contains(&MaterialShowcaseGeometry::BrushedPlate));
    assert!(geometries.contains(&MaterialShowcaseGeometry::GlassBlockGrid));
    assert!(geometries.contains(&MaterialShowcaseGeometry::GlassScreenGrid));
    assert!(geometries.contains(&MaterialShowcaseGeometry::StrapPanel));
    assert!(geometries.contains(&MaterialShowcaseGeometry::GasketFoot));
    assert!(geometries.contains(&MaterialShowcaseGeometry::FoldedSheet));
}

#[test]
fn round_e_material_showcase_names_source_backed_surface_separately() {
    let presets = material_preset_showcase();
    for id in ["satin", "leather", "rubber"] {
        let preset = presets
            .iter()
            .find(|preset| preset.id == id)
            .unwrap_or_else(|| panic!("missing material showcase preset {id}"));
        assert_eq!(
            preset.source_surface, "Assets::material_presets()",
            "{id} must stay on the source-backed material API surface"
        );
    }
}

#[test]
fn round_e_glass_background_target_is_opaque_scene_color_geometry() {
    let bars = glass_background_target_bars();
    assert!(
        bars.len() >= 6,
        "glass proof target needs enough opaque high-contrast bars for refraction/blur metrics"
    );
    assert!(
        bars.iter()
            .any(|bar| bar.scale.x > 0.30 && bar.scale.x > bar.scale.y * 8.0),
        "glass proof target needs horizontal opaque bars"
    );
    assert!(
        bars.iter()
            .any(|bar| bar.scale.y > 0.20 && bar.scale.y > bar.scale.x * 5.0),
        "glass proof target needs vertical opaque bars"
    );
    for path in [
        "src/demo_page/material_presets.rs",
        "src/browser_probe/workflows/pbr/material_presets.rs",
        "tests/examples_visual_proof.rs",
    ] {
        let source = std::fs::read_to_string(path).expect("material proof source is readable");
        assert!(
            source.contains("glass_background_target_bars()")
                && source.contains("MaterialDesc::matte(bar.color)")
                && source.contains("GeometryDesc::box_xyz(1.0, 1.0, 1.0)")
                && !source.contains("GeometryDesc::grid(0.44, 6)"),
            "{path} must use opaque bars for glass background proof; line-grid targets are not \
             included in the opaque scene-color transmission pass"
        );
    }
}

#[test]
fn round_e_material_demo_does_not_apply_one_global_key_light_to_ibl_sensitive_presets() {
    let source = include_str!("../src/demo_page/material_presets.rs");
    assert!(
        !source.contains("DirectionalLight::key_light()"),
        "Round E material proof must not use one global direct key light: chrome, brushed steel, \
         and glass are IBL-sensitive and must be judged under the pinned HDR rather than the same \
         white highlight used for studio-fill presets"
    );
}

#[test]
fn round_e_material_proof_camera_matches_external_reference_fixture() {
    let source = include_str!("../src/demo_page/material_presets.rs");
    let proof_js = include_str!("../demo/proof.js");
    assert!(
        source.contains("pub async fn load_material_proof_scene")
            && source.contains(".azimuth_elevation(-18.0, 18.0)")
            && proof_js.contains("load_material_proof_scene")
            && proof_js.contains("load_material_proof_scene(canvas.width, canvas.height)")
            && proof_js.contains("set_background_scheme(app, \"neutral_gray\")")
            && proof_js.contains("applyCanvasBackground(\"neutral_gray\")")
            && !proof_js.contains("load_material_presets_scene"),
        "Round E hard-reference proof must use the material-specific proof export and camera \
         pinned in round_e_material_fixture.toml; comparing the public 12-sphere route against \
         material-specific references is not meaningful"
    );
}

#[test]
fn browser_demo_timing_trace_reports_render_submission_and_scene_work() {
    let source = include_str!("../src/demo_page.rs");
    assert!(
        source.contains("[scena-demo] renderer frame:")
            && source.contains("gpu_draw_submissions")
            && source.contains("gpu_queue_submissions")
            && source.contains("readback_copies"),
        "browser timing diagnostics must distinguish an empty draw plan, an unsubmitted frame, \
         and a readback-only failure"
    );
}

#[test]
fn attached_browser_render_only_uses_probe_path_for_explicit_readback() {
    let frame = include_str!("../src/render/frame.rs");
    let surface = include_str!("../src/render/gpu/draw_surface.rs");
    let probe = include_str!("../src/browser_probe.rs");
    assert!(
        !frame.contains("let _ = readback_mode;")
            && frame.contains("readback_mode,")
            && surface.contains("readback_mode: RenderReadbackMode")
            && surface.contains("readback_mode == RenderReadbackMode::Synchronous")
            && surface.contains(
                "let surface_readback = (readback_mode == RenderReadbackMode::Synchronous)",
            )
            && surface.contains(
                "if readback_mode == RenderReadbackMode::Synchronous\n            && !post_enabled",
            )
            && surface.contains(
                "let renderer_readback = (readback_mode == RenderReadbackMode::Synchronous\n                && surface_readback.is_none())",
            )
            && surface.contains("render_browser_probe(")
            && probe.contains("render_with_readback_mode(")
            && probe.contains("RenderReadbackMode::Synchronous"),
        "an attached demo frame must present to its canvas; only an explicit synchronous \
         renderer-owned capture may divert into the browser probe path"
    );
}

#[test]
fn browser_device_rebuild_releases_the_lost_renderer_before_requesting_an_adapter() {
    let lifecycle = include_str!("../src/browser_probe/probes.rs");
    let gpu = include_str!("../src/render/gpu.rs");
    let surface = include_str!("../src/render/surface.rs");
    let release = lifecycle
        .find("drop(renderer);")
        .expect("the lost browser renderer must be released explicitly");
    let rebuild = lifecycle
        .find("rebuild_after_surface_loss(")
        .expect("the browser lifecycle probe must rebuild its renderer");

    assert!(
        release < rebuild,
        "WebGPU replacement must drop the lost renderer and its Device/Queue before requesting a fresh adapter"
    );

    let drain = lifecycle
        .find("wait_for_submitted_browser_work().await")
        .expect("simulated device loss must wait for real submitted browser work");
    assert!(
        drain < rebuild,
        "the browser probe must drain submitted GPU work before dropping and rebuilding its renderer"
    );
    assert!(
        gpu.contains("on_submitted_work_done")
            && gpu.contains("Promise::race")
            && gpu.contains("10_000")
            && !gpu.contains("while !complete.load")
            && surface.contains("wait_for_submitted_browser_work"),
        "the lifecycle drain must use WebGPU queue completion with a bounded browser timeout"
    );

    let replacement = lifecycle
        .split("async fn rebuild_after_surface_loss")
        .nth(1)
        .expect("browser lifecycle replacement helper is present");
    let capture = replacement
        .find("render_with_readback_mode")
        .expect("replacement lifecycle proof must request an explicit capture");
    let readback = replacement
        .find("browser_readback_rgba8")
        .expect("replacement lifecycle proof must consume its explicit capture");
    let visible = replacement
        .find(".render(scene, camera)")
        .expect("replacement lifecycle proof must still present a visible surface frame");
    assert!(
        capture < readback
            && readback < visible
            && replacement.contains("RenderReadbackMode::Synchronous"),
        "capture and await the renderer-owned buffer before submitting the replacement's visible surface frame"
    );
}

#[test]
fn browser_state_lifecycle_uses_explicit_capture_through_context_recovery() {
    let lifecycle = include_str!("../src/browser_probe/probes/state_lifecycle.rs");
    let probes = include_str!("../src/browser_probe/probes.rs");
    let recovery = lifecycle
        .find("verify_context_recovery(")
        .expect("state lifecycle must exercise context recovery");
    let capture_mode = lifecycle
        .get(recovery..)
        .and_then(|suffix| {
            suffix
                .find("RenderReadbackMode::Synchronous")
                .map(|offset| recovery + offset)
        })
        .expect("state lifecycle context recovery must request explicit capture");
    let drain = lifecycle
        .find("wait_for_submitted_browser_work")
        .expect("state lifecycle must drain its recovery capture submission");
    let readback = lifecycle
        .find("browser_readback_rgba8")
        .expect("state lifecycle must consume its explicit capture");
    assert!(
        recovery < capture_mode
            && capture_mode < drain
            && drain < readback
            && probes.contains("render_mode: RenderReadbackMode")
            && probes.contains("render_with_readback_mode(scene, camera, render_mode)"),
        "state recovery must capture explicitly before draining and reading the renderer-owned buffer"
    );
}

#[test]
fn round_e_material_demo_uses_fixed_fixture_exposure() {
    let wasm_exports = include_str!("../src/demo_page/controls.rs");
    let demo_page = include_str!("../src/demo_page.rs");
    let demo_js = include_str!("../demo/main.js");

    assert!(
        wasm_exports.contains("pub fn set_fixed_exposure_ev")
            && wasm_exports.contains("renderer.clear_auto_exposure()"),
        "Round E material proof needs a browser-exported fixed exposure path; managed \
         auto exposure lifts the dark studio background and invalidates external-reference comparisons"
    );
    assert!(
        demo_page.contains("renderer.clear_auto_exposure()")
            && !demo_page
                .contains("renderer.set_auto_exposure(AutoExposureConfig::product_studio())"),
        "the public live showcase must use curated fixed exposure on the hot path; managed browser \
         auto exposure samples the canvas and adds a second first-frame render"
    );
    assert!(
        demo_js.contains("set_fixed_exposure_ev")
            && demo_js.contains("set_fixed_exposure_ev(app, 0.0)"),
        "demo/?sample=material-presets must render with the fixture-pinned exposure_ev=0.0"
    );
}

#[test]
fn round_e_material_demo_synchronizes_canvas_css_background() {
    let wasm_exports = include_str!("../src/demo_page/controls.rs");
    let demo_js = include_str!("../demo/main.js");

    assert!(
        wasm_exports.contains("pub fn background_scheme_css_color"),
        "Round E browser proof needs a named-background CSS color export; WebGL2 canvas alpha \
         can remain transparent even when the wgpu surface prefers opaque alpha"
    );
    assert!(
        demo_js.contains("applyCanvasBackground")
            && demo_js.contains("background_scheme_css_color")
            && demo_js.contains("applyCanvasBackground(\"dark_studio\")"),
        "demo/?sample=material-presets must set the canvas CSS background to the same \
         fixture-pinned dark_studio color used by Renderer::set_background"
    );
}

#[test]
fn round_e_ibl_extension_gates_are_value_bounded_browser_metrics() {
    let shader_tests =
        fs::read_to_string("src/render/gpu/output.rs").expect("GPU output tests are readable");
    for required in [
        "triangle_shader_applies_anisotropy_lobe_to_environment_ibl",
        "triangle_shader_applies_clearcoat_lobe_to_environment_ibl",
        "triangle_shader_applies_anisotropy_lobe_in_native_and_webgl2_variants",
        "triangle_shader_applies_clearcoat_lobe_in_native_and_webgl2_variants",
    ] {
        assert!(
            shader_tests.contains(required),
            "Round E IBL extension proof must keep shader contract test {required}"
        );
    }

    let proof_script = fs::read_to_string("scripts/probe_cloudflare_material_presets.mjs")
        .expect("Round E material proof script is readable");
    let evaluator = fs::read_to_string("scripts/round_e_material_evaluator.cjs")
        .expect("shared Round E material evaluator is readable");
    let proof_sources = format!("{proof_script}\n{evaluator}");
    for required in [
        "anisotropy_aspect_ratio_ibl",
        "ANISOTROPY_EPSILON",
        "aspect + ANISOTROPY_EPSILON >= thresholds.brushed_steel.anisotropy_aspect_ratio_ibl",
        "brushed_steel_anisotropy",
        "clearcoat_lobe_delta",
        "delta >= thresholds.clearcoat_plastic.clearcoat_lobe_delta",
        "clearcoat_lobe",
    ] {
        assert!(
            proof_sources.contains(required),
            "Round E browser proof must enforce value-bounded IBL metric {required}"
        );
    }

    let thresholds = fs::read_to_string("tests/visual/references/round_e_material_thresholds.toml")
        .expect("Round E material thresholds are readable");
    for required in [
        "anisotropy_aspect_ratio_ibl = 2.00",
        "clearcoat_lobe_delta = 0.05",
    ] {
        assert!(
            thresholds.contains(required),
            "Round E thresholds must pin {required}"
        );
    }
}

#[test]
fn round_e_glass_ordering_uses_non_overlapping_opaque_target_alternative() {
    let presets = material_preset_showcase();
    let clear = presets
        .iter()
        .find(|preset| preset.id == "clear_glass")
        .expect("clear glass showcase preset exists");
    let frosted = presets
        .iter()
        .find(|preset| preset.id == "frosted_glass")
        .expect("frosted glass showcase preset exists");

    assert!(
        clear.background_target_position().is_some()
            && frosted.background_target_position().is_some(),
        "Round E glass proof needs opaque background targets behind every transparent glass preset"
    );
    let clear_half_width = clear.geometry.scale().x;
    let frosted_half_width = frosted.geometry.scale().x;
    let center_distance = (clear.position().x - frosted.position().x).abs();
    assert!(
        center_distance > clear_half_width + frosted_half_width,
        "Round E accepted showcase alternative requires glass presets to be non-overlapping; \
         otherwise browser/native transparency ordering or OIT proof is required"
    );

    for path in [
        "src/demo_page/material_presets.rs",
        "src/browser_probe/workflows/pbr/material_presets.rs",
        "tests/examples_visual_proof.rs",
    ] {
        let source = fs::read_to_string(path).expect("material proof source is readable");
        assert!(
            (source.contains("background_target_position()")
                || source.contains("geometry.uses_background_target()"))
                && source.contains("glass_background_target_bars()"),
            "{path} must insert opaque background targets through the shared showcase metadata"
        );
    }
}

#[test]
fn round_e_fixture_does_not_depend_on_ssao_ssr_or_msaa_taa() {
    let checklist =
        fs::read_to_string("docs/checklists/next-release-easy-use-and-state-of-the-art.md")
            .expect("Round E checklist is readable")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
    assert!(
        checklist.contains("SSAO/contact grounding is not used by the current Round E fixture")
            && checklist.contains("SSR is not used by the current Round E fixture")
            && checklist.contains(
                "MSAA/TAA beyond current FXAA is not used by the current Round E fixture"
            ),
        "Round E conditional renderer-quality lanes must be explicitly closed as not used by the \
         current fixture, rather than left as hidden public-demo blockers"
    );
}

#[test]
fn internal_material_sphere_review_uses_no_local_glass_cards_or_panels() {
    let source = include_str!("../src/demo_page/material_presets.rs");
    assert!(
        !source.contains("add_material_sphere_glass_reveal_panel")
            && !source.contains("glass reveal panel")
            && !source.contains("preset.id == \"clear_glass\" || preset.id == \"frosted_glass\""),
        "the internal 12-sphere review page must not hide glass shortcomings behind a local \
         square/card/panel; glass has to improve in the material or renderer path"
    );
    assert!(
        source.contains("material_studio_backdrop_geometry()")
            && source.contains(".add_studio_lighting()"),
        "the approved 12-sphere review scene keeps the same full-scene backdrop and global \
         lighting while using a single gradient backdrop mesh instead of dozens of per-band \
         primitives"
    );
}

#[test]
fn public_material_presets_demo_uses_approved_sphere_showcase() {
    let demo_js = include_str!("../demo/main.js");
    let scene_source = include_str!("../src/demo_page/material_presets.rs");
    assert!(
        demo_js.contains("load_material_presets_scene")
            && demo_js.contains("load_material_presets_scene(canvas.width, canvas.height)")
            && demo_js.contains("browser-rendered WebGL2 material showcase")
            && demo_js.contains("rendering browser material showcase"),
        "the public Cloudflare material-presets route must load the approved 12-sphere showcase"
    );
    assert!(
        scene_source.contains("pub async fn load_material_presets_scene")
            && scene_source
                .contains("load_material_spheres_scene(viewport_width, viewport_height).await"),
        "the stable public WASM export must delegate to the approved 12-sphere showcase"
    );
}

#[test]
fn public_showcase_material_section_uses_live_spheres_not_png_proof_thumbnails() {
    let demo_js = include_str!("../demo/main.js");
    let demo_html = include_str!("../demo/index.html");

    assert!(
        demo_js.contains("load_material_presets_scene")
            && !demo_js.contains("load_single_material_sphere_scene")
            && !demo_js.contains("assets/showcase/materials/${id}.png"),
        "the public materials section must render the approved browser 12-sphere scene once, \
         not swap PNG proof thumbnails or rebuild a fresh single-material renderer per click"
    );
    assert!(
        demo_html.contains("data-scene=\"material\"")
            && demo_html.contains("material-choices")
            && demo_html.contains("material-selected")
            && !demo_html.contains("thumb-grid"),
        "the public materials section should expose browser-rendered material choices, not the \
         old PNG thumbnail grid"
    );
}

#[test]
fn public_showcase_detaches_inactive_webgl2_surfaces_before_loading_next_section() {
    let demo_js = include_str!("../demo/main.js");
    let wasm_exports = include_str!("../src/demo_page.rs");

    assert!(
        wasm_exports.contains("pub fn detach_from_canvas")
            && wasm_exports.contains("renderer.release_surface()")
            && wasm_exports.contains("pub fn transfer_renderer_to")
            && wasm_exports.contains("target.renderer = Some(renderer)")
            && wasm_exports.contains("renderer surface reuse failed; rebuilding renderer")
            && !wasm_exports.contains("app.renderer = None"),
        "the browser showcase must detach only the WebGL2 surface and keep the renderer alive; \
         dropping app.renderer discards pipelines, prefiltered IBL, and uploaded GPU resources"
    );
    assert!(
        demo_js.contains("detach_from_canvas")
            && demo_js.contains("detachOtherStages")
            && demo_js.contains("transferWarmRendererTo(this)")
            && demo_js.contains("transfer_renderer_to(controller.app, target.app)")
            && demo_js.contains("this.attached = false"),
        "the public showcase must detach inactive WebGL2 surfaces before activating the next \
         live section; otherwise later canvases fail with CreateSurface"
    );
}

#[test]
fn public_showcase_renders_canvases_at_device_resolution() {
    let demo_js = include_str!("../demo/main.js");

    assert!(
        demo_js.contains("MAX_DEVICE_PIXEL_RATIO")
            && demo_js.contains("MIN_CANVAS_RENDER_SCALE")
            && demo_js.contains("window.devicePixelRatio")
            && demo_js.contains("cssWidth * pixelRatio")
            && demo_js.contains("MAX_CANVAS_DIMENSION = 2048"),
        "the public showcase must size the WebGL canvas backing store above CSS pixels, capped \
         at WebGL2's safe 2048 dimension, so material sphere edges do not look pixelated"
    );
}

#[test]
fn public_showcase_waits_for_warm_frames_before_rendered_status() {
    let demo_js = include_str!("../demo/main.js");

    assert!(
        demo_js.contains("MIN_RENDERED_FRAME_COUNT")
            && demo_js.contains("MIN_RENDERED_FRAME_COUNT = 1")
            && demo_js.contains("framesSinceAttach")
            && demo_js.contains("await nextAnimationFrame()")
            && demo_js.contains("resize(this.app, width, height)")
            && demo_js.contains("renderedForActivation"),
        "the public showcase must resize after attach and render warm-up frames before flipping \
         a live section to rendered; otherwise a blank first frame can be reported as rendered"
    );
    assert!(
        !demo_js
            .contains("resize(this.app, width, height);\n          this.framesSinceAttach = 0;"),
        "resize observers must not reset the warm-frame counter after attach; otherwise a noisy \
         resize loop can leave hero/model stuck at rendering forever"
    );
}

#[test]
fn public_showcase_connector_replay_keeps_grid_lines_out_of_prepare_hot_path() {
    let wasm_exports = include_str!("../src/demo_page.rs");

    assert!(
        wasm_exports.contains("let floor = scene\n        .add_grid_floor(")
            && wasm_exports.contains("scene\n        .set_visible(floor.grid, false)")
            && wasm_exports.contains(
                "connector replay keeps the animated scene on the dynamic GPU prepare path"
            ),
        "the public connector replay must not keep decorative grid-line primitives visible during \
         animation; line primitives are intentionally not depth-prepass eligible, and their \
         presence forces a full WebGL2 prepare on every connector tick"
    );
}

#[test]
fn demo_tick_does_not_dirty_camera_controls_when_nothing_changed() {
    let wasm_exports = include_str!("../src/demo_page.rs");

    assert!(
        wasm_exports.contains("controls_dirty: bool")
            && wasm_exports.contains("needs_prepare: bool")
            && wasm_exports.contains("if app.controls_dirty")
            && wasm_exports.contains("app.controls_dirty = false")
            && wasm_exports.contains("if app.needs_prepare")
            && wasm_exports.contains("app.needs_prepare = false")
            && wasm_exports
                .contains("app.controls_dirty = !matches!(action, OrbitControlAction::None)"),
        "demo tick must not call OrbitControls::apply_to_scene every frame when the user has not \
         moved the camera; that bumps the scene transform revision and forces full WebGL2 GPU \
         prepare work on every warm frame"
    );
}

#[test]
fn same_size_resize_does_not_invalidate_prepared_gpu_state() {
    let surface_rs = include_str!("../src/render/surface.rs");

    assert!(
        surface_rs.contains("if self.target.width == width && self.target.height == height")
            && surface_rs.contains("return Ok(());"),
        "same-size browser resize events must not bump target_revision; otherwise every \
         attach/ResizeObserver cycle invalidates prepared WebGL2 resources and re-runs full GPU \
         prepare"
    );
}

#[test]
fn public_showcase_keeps_environment_cache_renderer_owned() {
    let environment_cache_rs = include_str!("../src/render/environment_cache.rs");

    assert!(
        environment_cache_rs.contains("EnvironmentLightingCacheKey")
            && environment_cache_rs.contains("entries: HashMap")
            && environment_cache_rs.contains("source_sha256")
            && !environment_cache_rs.contains("static GLOBAL_ENVIRONMENT_LIGHTING_CACHE")
            && !environment_cache_rs.contains("OnceLock")
            && !environment_cache_rs.contains("Mutex::new"),
        "prepared HDR IBL reuse must stay renderer-owned and keyed by environment identity/profile; \
         a process-wide render singleton violates the renderer architecture contract"
    );
    assert!(
        environment_cache_rs.contains("active: Option<ActiveEnvironmentLightingCache>")
            && environment_cache_rs.contains("pub(super) fn clear_active")
            && environment_cache_rs.contains("self.environment_lighting_cache.entries.get(&key)"),
        "each public showcase section keeps its own renderer after surface detach; its \
         renderer-owned environment cache must survive detach/re-attach without relying on a \
         global singleton"
    );
}

#[test]
fn public_showcase_reuses_browser_asset_bytes() {
    let demo_js = include_str!("../demo/main.js");

    assert!(
        demo_js.contains("const byteCache = new Map()")
            && demo_js.contains("byteCache.get(url)")
            && demo_js.contains("return new Uint8Array(buffer)")
            && !demo_js.contains("buffer.slice(0)"),
        "the public showcase should reuse fetched GLB bytes across sections instead of issuing \
         duplicate downloads and multi-megabyte buffer copies for the same asset"
    );
}

#[test]
fn public_showcase_probe_checks_visible_canvas_pixels() {
    let probe = include_str!("../scripts/probe_showcase_demo.mjs");

    assert!(
        probe.contains("assertCanvasVisible")
            && probe.contains("foregroundRatio")
            && probe.contains("renderScale")
            && probe.contains("renderedForActivation")
            && probe.contains("model showcase"),
        "the showcase browser probe must reject black/blank live canvases and low-resolution \
         material canvases instead of trusting stale status text alone"
    );
}

#[test]
fn browser_gpu_live_render_routes_postprocess_to_gpu_settings() {
    let frame_rs = include_str!("../src/render/frame.rs");
    let draw_surface_rs = include_str!("../src/render/gpu/draw_surface.rs");

    assert!(
        frame_rs.contains("GpuPostSettings::new")
            && frame_rs.contains("self.stats.fxaa_passes = gpu_result.post_counts.fxaa")
            && draw_surface_rs.contains("post::encode_chain(")
            && draw_surface_rs.contains("post::encode_blit_to_view(")
            && draw_surface_rs.contains("scena.browser.overlay_final_surface_pass")
            && !frame_rs.contains("fn cpu_frame_postprocess_applies"),
        "browser live WebGL2/WebGPU rendering must route post-processing through the GPU post \
         settings instead of the removed CPU frame postprocess gate"
    );
}

#[test]
fn public_showcase_prefetches_and_idle_prepares_below_the_fold_scenes() {
    let demo_html = include_str!("../demo/index.html");
    let demo_js = include_str!("../demo/main.js");
    let probe = include_str!("../scripts/probe_showcase_demo.mjs");

    for required in [
        r#"<link rel="modulepreload" href="/main.js"#,
        r#"<link rel="modulepreload" href="/pkg/scena.js"#,
        r#"<link rel="preload" href="/pkg/scena_bg.wasm"#,
        r#"<link rel="preload" href="/samples/connector-snap/connector_snap_assembly.glb""#,
        r#"<link rel="preload" href="/samples/environment/white_studio_03_1k.hdr.prefilter.bin""#,
        r#"<link rel="prefetch" href="/samples/connector-snap/drive_unit.glb""#,
        r#"<link rel="prefetch" href="/samples/connector-snap/load_unit.glb""#,
    ] {
        assert!(
            demo_html.contains(required),
            "public showcase must provide resource hint {required}"
        );
    }

    for required in [
        "async prepareScene()",
        "async prepareSceneNow()",
        "function schedulePrefetch()",
        "requestIdleCallback",
        "setTimeout(callback, 200)",
        "controllers.get(scene)?.prepareScene()",
        "schedulePrefetch().catch",
        "prepared: controller.loaded && !controller.attached",
    ] {
        assert!(
            demo_js.contains(required),
            "public showcase must keep idle scene preparation contract {required}"
        );
    }

    for required in [
        "waitForPreparedControllers",
        "HARDWARE_SECTION_ACTIVATION_BUDGET_MS",
        "SOFTWARE_SECTION_ACTIVATION_BUDGET_MS",
        "browserWebglRendererInfo",
        "isSoftwareWebglRenderer",
        "activation_ms",
    ] {
        assert!(
            probe.contains(required),
            "showcase probe must enforce preloaded activation contract {required}"
        );
    }
}

#[test]
fn public_showcase_uses_hdr_sidecar_without_parallel_render_cache() {
    let environment_loading_rs = include_str!("../src/assets/environment_loading.rs");
    let environment_rs = include_str!("../src/assets/environment.rs");
    let sidecar_rs = include_str!("../src/assets/environment_sidecar.rs");
    let prepare_environment_rs = include_str!("../src/render/prepare/environment.rs");
    let environment_cache_rs = include_str!("../src/render/environment_cache.rs");
    let xtask_rs = include_str!("../crates/xtask/src/app/prerender_environment.rs");
    let doctor_rs =
        include_str!("../crates/xtask/src/app/doctor_easy_scene/showcase_performance.rs");

    assert!(
        environment_loading_rs.contains("try_load_environment_sidecar")
            && environment_loading_rs.contains("sidecar_path_for_environment")
            && environment_loading_rs.contains("from_equirectangular_hdr_sidecar_bytes")
            && environment_loading_rs.contains("EnvironmentPrefilterSidecar::parse"),
        "Assets::load_environment must prefer a matching .prefilter.bin sidecar before the \
         renderer reaches the expensive runtime prefilter path"
    );
    assert!(
        environment_rs.contains("AssetProvenance::from_source_bytes(path, source_bytes)")
            && environment_rs
                .contains("AssetProvenance::new(path).with_source_sha256(source_sha256)")
            && environment_rs.contains("from_equirectangular_hdr_sidecar_bytes")
            && environment_rs.contains("prefilter_sidecar(")
            && environment_rs.contains("prefilter_sidecar: Some(std::sync::Arc::new(sidecar))"),
        "EnvironmentDesc must carry source SHA and sidecar metadata as asset data, not as a \
         global renderer singleton"
    );
    assert!(
        environment_loading_rs.contains("environment_from_hdr_bytes")
            && environment_loading_rs.contains("from_equirectangular_hdr_sidecar_bytes")
            && environment_loading_rs
                .contains("from_equirectangular_hdr_bytes(path, source_bytes)"),
        "Assets::load_environment must avoid full HDR decode when a matching sidecar is present \
         and fall back to runtime HDR decode only when the sidecar is absent or stale"
    );
    assert!(
        sidecar_rs.contains("SIDECAR_FILE_SUFFIX")
            && sidecar_rs.contains("SCENA_ENV_PF_V2")
            && sidecar_rs.contains("const SIDECAR_VERSION: u32 = 2")
            && sidecar_rs.contains("EnvironmentSidecarHeader")
            && sidecar_rs.contains("bytemuck")
            && sidecar_rs.contains("source_sha256"),
        "environment sidecar format must stay binary and SHA-pinned"
    );
    assert!(
        prepare_environment_rs.contains("environment.prefilter_sidecar(sidecar_profile)")
            && prepare_environment_rs.contains("load_prefilter_sidecar")
            && prepare_environment_rs.contains("bake_environment_ibl")
            && prepare_environment_rs.contains("EnvironmentIblBakeRequest"),
        "render prepare must consume sidecars through the existing environment_lighting_for_prepare \
         path and only fall back to the current Rust IBL baker when the sidecar is absent"
    );
    assert!(
        environment_cache_rs.contains("EnvironmentLightingCache")
            && environment_cache_rs.contains("EnvironmentSidecarIdentity")
            && environment_cache_rs
                .contains("source_sha256: sidecar.header().source_sha256_bytes()")
            && !environment_cache_rs.contains("OnceLock")
            && !environment_cache_rs.contains("Mutex::new"),
        "sidecar-backed prepared lighting must populate the existing renderer-owned environment \
         cache; no parallel global cache is allowed"
    );
    assert!(
        xtask_rs.contains("run_prerender_environment")
            && xtask_rs.contains("EnvironmentSidecarProfile::InteractiveWebGl2")
            && xtask_rs.contains("precompute_environment_sidecar"),
        "xtask prerender-environment must be the source of truth for regenerating the sidecar"
    );
    assert!(
        doctor_rs.contains("DEMO-HDR-SIDECAR-CURRENT")
            && doctor_rs.contains("white_studio_03_1k.hdr.prefilter.bin")
            && doctor_rs.contains("header.source_sha256_hex()")
            && doctor_rs.contains("InteractiveWebGl2"),
        "doctor must validate the committed demo HDR sidecar header without regenerating it"
    );
}

#[test]
fn public_and_proof_wasm_bundles_are_split() {
    let cargo_toml = include_str!("../Cargo.toml");
    let build_script = include_str!("../scripts/build_demo_wasm.js");
    let package_json = include_str!("../package.json");
    let proof_js = include_str!("../demo/proof.js");
    let doctor_rs =
        include_str!("../crates/xtask/src/app/doctor_easy_scene/showcase_performance.rs");

    assert!(
        cargo_toml.contains("proof-harness = [\"demo-page\"]"),
        "proof-only wasm exports must be behind a proof-harness feature"
    );
    assert!(
        cargo_toml.contains("[profile.release]")
            && cargo_toml.contains("lto = true")
            && cargo_toml.contains("codegen-units = 1")
            && cargo_toml.contains("panic = \"abort\""),
        "public showcase release WASM must keep size-oriented release profile settings so cold \
         browser instantiation does not regress"
    );
    assert!(
        build_script.contains("outDir: \"demo/pkg\"")
            && build_script.contains("features: \"demo-page\"")
            && build_script.contains("outDir: \"demo/proof/pkg\"")
            && build_script.contains("features: \"demo-page,proof-harness,browser-probe\""),
        "demo:build must build the lean public bundle and proof:build must build the proof bundle"
    );
    assert!(
        package_json.contains("\"proof:build\"")
            && proof_js.contains("from \"./proof/pkg/scena.js")
            && proof_js.contains("./proof/pkg/scena_bg.wasm"),
        "the proof harness must import its own proof bundle instead of the public showcase bundle"
    );
    assert!(
        doctor_rs.contains("PUBLIC-SHOWCASE-WASM-SIZE")
            && doctor_rs.contains("PUBLIC_SHOWCASE_WASM_BASELINE_RAW_BYTES")
            && doctor_rs.contains("PUBLIC_SHOWCASE_WASM_BASELINE_BROTLI_BYTES")
            && doctor_rs.contains("PROOF_HARNESS_WASM_BASELINE_RAW_BYTES")
            && doctor_rs.contains("PROOF_HARNESS_WASM_BASELINE_BROTLI_BYTES")
            && doctor_rs.contains(".size.json")
            && doctor_rs.contains("if !public_exists && !proof_exists")
            && doctor_rs.contains("return;")
            && doctor_rs.contains("brotli_bytes"),
        "doctor must enforce separate raw and brotli WASM size budgets for generated public \
         and proof bundles while still allowing doctor --full to run before npm builds in a \
         clean checkout"
    );
}
