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
fn round_e_material_demo_camera_matches_external_reference_fixture() {
    let source = include_str!("../src/demo_page/material_presets.rs");
    assert!(
        source.contains(".azimuth_elevation(-18.0, 18.0)"),
        "Round E browser/demo proof must use the same camera azimuth/elevation pinned in \
         round_e_material_fixture.toml so external-reference comparisons are meaningful"
    );
}

#[test]
fn round_e_material_demo_uses_fixed_fixture_exposure() {
    let wasm_exports = include_str!("../src/demo_page/controls.rs");
    let demo_js = include_str!("../demo/main.js");

    assert!(
        wasm_exports.contains("pub fn set_fixed_exposure_ev")
            && wasm_exports.contains("renderer.clear_auto_exposure()"),
        "Round E material proof needs a browser-exported fixed exposure path; managed \
         auto exposure lifts the dark studio background and invalidates external-reference comparisons"
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
    for required in [
        "anisotropy_aspect_ratio_ibl",
        "passed_anisotropy_aspect_ratio_ibl",
        "brushed_steel anisotropy aspect ratio",
        "clearcoat_lobe_delta",
        "passed_clearcoat_lobe_delta",
        "clearcoat_plastic lobe delta",
    ] {
        assert!(
            proof_script.contains(required),
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
            && wasm_exports.contains("app.renderer = None"),
        "the browser showcase needs an explicit detach export so a WebGL2 page with several \
         sections does not keep stale surfaces alive"
    );
    assert!(
        demo_js.contains("detach_from_canvas")
            && demo_js.contains("detachOtherStages")
            && demo_js.contains("this.attached = false"),
        "the public showcase must detach inactive WebGL2 surfaces before activating the next \
         live section; otherwise later canvases fail with CreateSurface"
    );
}
