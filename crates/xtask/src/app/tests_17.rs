use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

const ROUND_E_CLOUDFLARE_TEST_PRESETS: &[&str] = &[
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

pub(crate) fn expanded_material_preset_guide() -> String {
    format!(
        "{VALID_GUIDE} viewer.play_clip(\n```rust\nlet matte = assets.create_material(MaterialDesc::matte(Color::GRAY));\nlet body = assets.create_material(MaterialDesc::plastic(Color::BLUE));\nlet shaft = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));\nlet rough = assets.create_material(MaterialDesc::rough_metal(Color::GRAY));\nlet chrome = assets.create_material(MaterialDesc::chrome());\nlet steel = assets.create_material(MaterialDesc::brushed_steel());\nlet cover = assets.create_material(MaterialDesc::clearcoat_plastic(Color::BLUE));\nlet satin = assets.create_material(MaterialDesc::satin(Color::GRAY));\nlet leather = assets.create_material(MaterialDesc::leather(Color::GRAY));\nlet glass = assets.create_material(MaterialDesc::clear_glass(Color::CYAN));\nlet frosted = assets.create_material(MaterialDesc::frosted_glass(Color::CYAN));\nlet foot = assets.create_material(MaterialDesc::rubber());\n```",
    )
}

#[test]
pub(crate) fn cloudflare_material_proof_rejects_values_outside_committed_thresholds() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/round-e-cloudflare-proof");
    write_round_e_threshold_fixture(&fixture_root);
    fs::create_dir_all(fixture_root.join("target/gate-artifacts"))
        .expect("cloudflare proof artifact dir");
    fs::write(
        fixture_root.join("target/gate-artifacts/round-e-cloudflare-material-proof.json"),
        r#"{
          "proof_class": "round-e-cloudflare-material-proof",
          "status": "pass",
          "fixture": {
            "environment_hdr_path": "demo/samples/environment/white_studio_03_1k.hdr",
            "environment_hdr_sha256": "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b",
            "tonemapper": "model-viewer-neutral-reference",
            "output_color_space": "srgb",
            "exposure_ev": 0.0,
            "webgl2_smooth_metal_sample_floor": 96
          },
          "cache_buster": { "bumped": true },
          "wasm": { "checksum_matches_build": true },
          "per_material": {
            "chrome": {
              "delta_e2000_vs_reference": 99.0,
              "delta_e2000_max": 999.0,
              "reference_delta_gate": "hard",
              "passed_reference_delta": true,
              "specular_dynamic_range": 2.0,
              "luminance_p05": 40.0,
              "luminance_p99": 240.0,
              "neighbor_delta_e2000": 6.0
            }
          },
          "neighbor_pairs": [
            { "pair": ["metal", "rough_metal"], "delta_e2000": 6.0, "passed": true }
          ]
        }"#,
    )
    .expect("cloudflare proof artifact");
    let mut findings = Vec::new();

    crate::app::doctor_easy_scene::material_presets_cloudflare::check_round_e_cloudflare_material_proof(
        &fixture_root,
        &mut findings,
    );

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "HONEST-MATERIAL-PRESETS"
                && finding.message.contains("chrome delta_e2000_vs_reference")
                && finding.message.contains("committed threshold")
        }),
        "doctor must compare artifact values against committed thresholds, not artifact-local thresholds: {findings:?}",
    );
}

#[test]
pub(crate) fn cloudflare_material_proof_fails_closed_on_reference_delta() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let script = fs::read_to_string(root.join("scripts/probe_cloudflare_material_presets.mjs"))
        .expect("cloudflare material proof script");

    assert!(
        script.contains("Public material approval must fail closed"),
        "public material proof must document why reference DeltaE is a hard gate",
    );
    assert!(
        script.contains("return true;"),
        "public material proof must not leave reference DeltaE in diagnostic-only mode",
    );
    assert!(
        script.contains("metrics.reference_delta_gate === \"hard\"")
            && script.contains("!metrics.passed_reference_delta"),
        "public material proof must turn failed reference DeltaE into a script failure",
    );
}

#[test]
pub(crate) fn internal_material_sphere_page_is_review_only_and_bounded() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let html = fs::read_to_string(root.join("demo/internal-material-spheres.html"))
        .expect("internal material sphere page");
    let js = fs::read_to_string(root.join("demo/internal-material-spheres.js"))
        .expect("internal material sphere script");

    assert!(
        html.contains("12 material preset spheres") && html.contains("sphere-labels"),
        "internal review page must keep the requested twelve labeled sphere surface",
    );
    assert!(
        js.contains("MAX_RENDER_DIMENSION = 1920"),
        "internal review page must clamp browser render dimensions",
    );
    assert!(
        js.contains("requestRender()") && !js.contains("function animate("),
        "internal review page must use one-shot rendering, not a continuous animation loop",
    );
}

#[test]
pub(crate) fn easy_scene_setup_contracts_reject_material_preset_guide_subset() {
    let root = repo_root().expect("test runs inside the scena workspace");
    let fixture_root = root.join("target/xtask-doctor-regressions/material-preset-guide-subset");
    let guide = format!(
        "{VALID_GUIDE}\n```rust\nlet body = assets.create_material(MaterialDesc::plastic(Color::BLUE));\nlet shaft = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));\nlet cover = assets.create_material(MaterialDesc::clearcoat_plastic(Color::BLUE));\nlet glass = assets.create_material(MaterialDesc::clear_glass(Color::CYAN));\nlet foot = assets.create_material(MaterialDesc::rubber());\n```",
    );
    write_easy_scene_fixture(
        &fixture_root,
        &guide,
        "frame_bounds(()) bounds_for_transforms add_grid_floor",
        r#"<details id="diagnostics" class="diagnostics"><strong id="metric-frame">0</strong></details>"#,
    );
    write_expanded_material_preset_doctor_fixture(&fixture_root);
    let mut findings = Vec::new();

    check_easy_scene_setup_contracts(&fixture_root, &mut findings);

    assert!(
        findings.iter().any(|finding| {
            finding.rule == "HONEST-MATERIAL-PRESETS"
                && finding.message.contains("docs/guides/easy-scene-setup.md")
        }),
        "doctor must reject easy-scene guide material snippets that omit shipped presets: {findings:?}",
    );
}

pub(crate) fn write_expanded_material_preset_doctor_fixture(fixture_root: &Path) {
    for rel in [
        "src/material_showcase.rs",
        "src/assets/material_presets.rs",
        "src/diagnostics/diagnostic.rs",
        "src/diagnostics/capabilities.rs",
        "src/demo_page/material_presets.rs",
        "tests/round_e_material_showcase.rs",
        "tests/geometry_generated_uvs.rs",
        "tests/round_e_source_backed_material_presets.rs",
        "scripts/probe_cloudflare_material_presets.mjs",
        "demo/proof/index.html",
        "demo/internal-material-spheres.html",
        "demo/internal-material-spheres.js",
        "tests/visual/references/round_e_material_fixture.toml",
        "tests/visual/references/round_e_material_thresholds.toml",
        "tests/visual/references/round_e_failing_baseline.json",
        "tests/visual/references/round_e_failing_baseline_glossy_grid.png",
    ] {
        copy_repo_fixture_file(fixture_root, rel);
    }
    for preset in ROUND_E_CLOUDFLARE_TEST_PRESETS {
        copy_repo_fixture_file(
            fixture_root,
            &format!("tests/visual/references/round_e/{preset}.png"),
        );
    }
    append_fixture_text(
        fixture_root,
        "src/demo_page.rs",
        " samples/environment/white_studio_03_1k.hdr ",
    );
    append_fixture_text(
        fixture_root,
        "demo/index.html",
        " scena 1.5 live showcase Beautiful 3D in Rust Twelve materials. Twelve names. technical proof ",
    );
    append_fixture_text(
        fixture_root,
        "demo/main.js",
        " load_material_presets_scene(canvas.width, canvas.height) detach_from_canvas detachOtherStages set_fixed_exposure_ev(app, 0.0) applyCanvasBackground(\"dark_studio\") assets.material_presets().leather().await? ResizeObserver browser-rendered WebGL2 material showcase ",
    );
    append_fixture_text(
        fixture_root,
        "tests/examples_visual_proof.rs",
        " assert_round_e_reference_docs_image_metrics round_e_material_reference_docs_image_metrics material_identity_thresholds material_preset_showcase ",
    );
    append_fixture_text(
        fixture_root,
        "package.json",
        " cloudflare:materials scripts/probe_cloudflare_material_presets.mjs showcase:probe scripts/probe_showcase_demo.mjs ",
    );
    fs::write(
        fixture_root.join("tests/m8_real_asset_proof.rs"),
        "polyhaven_white_studio_demo_hdr_is_real_documented_neutral_radiance_file WHITE_STUDIO_DEMO_HDR_PATH",
    )
    .expect("white studio proof fixture");
    fs::create_dir_all(fixture_root.join("demo/samples")).expect("demo samples fixture dir");
    fs::write(
        fixture_root.join("demo/samples/SOURCES.md"),
        "ambientCG `Fabric001` ambientCG `Leather001` ambientCG `Rubber002` cea13f8f2b44ba8a9d4bced83d26ab344e39502dba24b0b4025b7bd3a180a4c2 8d3ac9280bec6a1e1e5384b93e5130414a085033290dde305bacaddd0aa6b96a CC0 white_studio_03 neutral studio HDRI ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b",
    )
    .expect("material source fixture");
    fs::write(
        fixture_root.join("src/material/presets.rs"),
        "pub const fn matte(color: Color) {} pub const fn plastic(color: Color) {} pub const fn metal(color: Color) {} pub const fn rough_metal(color: Color) {} pub const fn chrome() {} pub const fn brushed_steel() {} pub const fn clearcoat_plastic(color: Color) {} pub const fn satin(color: Color) {} pub const fn leather(color: Color) {} pub const fn clear_glass(color: Color) {} pub const fn frosted_glass(color: Color) {} pub const fn rubber() {}",
    )
    .expect("expanded material preset fixture");
    fs::write(
        fixture_root.join("tests/round_b_material_presets.rs"),
        "honest_material_presets_are_public_pbr_shortcuts expanded_material_presets_use_only_backed_material_lanes",
    )
    .expect("expanded material test fixture");
    fs::create_dir_all(fixture_root.join("src/browser_probe/workflows/pbr"))
        .expect("browser pbr fixture dir");
    fs::write(
        fixture_root.join("src/browser_probe/workflows/pbr/material_presets.rs"),
        "material_presets_scene material_preset_showcase browser-pbr-material-preset-expanded-set webgl2_smooth_metal_sample_floor scene-color-ior-thickness-rough-blur-sorted-transparency /demo/samples/environment/white_studio_03_1k.hdr showcase_geometry source_surfaces",
    )
    .expect("browser material preset fixture");
    let browser_probe = fixture_root.join("tests/browser/m6_rust_wasm_renderer_probe.js");
    fs::create_dir_all(browser_probe.parent().expect("browser fixture parent"))
        .expect("browser test fixture dir");
    let mut browser_probe_fixture = fs::read_to_string(&browser_probe).unwrap_or_default();
    if !browser_probe_fixture.is_empty() {
        browser_probe_fixture.push(' ');
    }
    browser_probe_fixture.push_str(
        "assertMaterialPresetProof pbr-material-presets webgl2_smooth_metal_sample_floor < 96 /demo/samples/environment/white_studio_03_1k.hdr single-shape grid Assets::material_presets()",
    );
    fs::write(browser_probe, browser_probe_fixture).expect("browser probe fixture");
    fs::create_dir_all(fixture_root.join("src/render/prepare"))
        .expect("render prepare fixture dir");
    fs::write(
        fixture_root.join("src/render/prepare/environment_prefilter.rs"),
        "sample_count_for_roughness(0.28, EnvironmentPrefilterQuality::InteractiveWebGl2) 2 => 96 _ => 192",
    )
    .expect("environment prefilter fixture");
}

fn copy_repo_fixture_file(fixture_root: &Path, rel: &str) {
    let root = repo_root().expect("test runs inside the scena workspace");
    let src = root.join(rel);
    let dst = fixture_root.join(rel);
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).expect("fixture parent dir");
    }
    fs::copy(&src, &dst).unwrap_or_else(|err| panic!("copy fixture {rel} from {src:?}: {err}"));
}

fn append_fixture_text(fixture_root: &Path, rel: &str, text: &str) {
    let path = fixture_root.join(rel);
    let mut current = fs::read_to_string(&path).unwrap_or_default();
    current.push_str(text);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("fixture parent dir");
    }
    fs::write(&path, current).expect("append fixture text");
}

fn write_round_e_threshold_fixture(root: &Path) {
    fs::create_dir_all(root.join("tests/visual/references"))
        .expect("round e reference fixture dir");
    fs::write(
        root.join("tests/visual/references/round_e_material_thresholds.toml"),
        r#"
[chrome]
specular_dynamic_range = 2.00
dark_reflection_luminance_p05_max = 85.00
bright_reflection_luminance_p99_min = 230.00
reflection_edge_contrast = 0.30
delta_e2000_max = 4.00

[brushed_steel]
anisotropy_aspect_ratio_direct = 3.00
anisotropy_aspect_ratio_ibl = 2.00
delta_e2000_max = 4.00

[clearcoat_plastic]
clearcoat_lobe_delta = 0.05
delta_e2000_max = 4.00

[clear_glass]
background_delta_e2000_max = 8.00
refraction_offset_min = 4.00
delta_e2000_max = 4.00

[frosted_glass]
high_frequency_contrast_reduction_min = 0.50
delta_e2000_max = 4.00

[leather]
texture_variance_min = 0.02
delta_e2000_max = 4.00

[rubber]
roughness_variance_min = 0.02
delta_e2000_max = 4.00

[satin]
sheen_width_min = 0.20
delta_e2000_max = 4.00

[global]
neighbor_delta_e2000_min = 6.00
reference_delta_e2000_max = 4.00
"#,
    )
    .expect("round e thresholds");
    let mut fixture = String::new();
    for preset in ROUND_E_CLOUDFLARE_TEST_PRESETS {
        fixture.push_str(&format!(
            r#"
[[presets.{preset}]]
environment_hdr_path = "demo/samples/environment/white_studio_03_1k.hdr"
environment_hdr_sha256 = "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b"
tonemapper = "model-viewer-neutral-reference"
output_color_space = "srgb"
exposure_ev = 0.0
"#,
        ));
    }
    fs::write(
        root.join("tests/visual/references/round_e_material_fixture.toml"),
        fixture,
    )
    .expect("round e fixture");
}
