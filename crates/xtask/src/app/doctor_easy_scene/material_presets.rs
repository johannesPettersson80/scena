use crate::app::prelude::*;

pub(super) fn check_honest_material_presets(root: &Path, findings: &mut Vec<Finding>) {
    check_round_e_material_fixture_contract(root, findings);
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/material/presets.rs",
        &[
            "pub const fn matte(",
            "pub const fn plastic(",
            "pub const fn metal(",
            "pub const fn rough_metal(",
            "pub const fn chrome()",
            "pub const fn brushed_steel()",
            "pub const fn clearcoat_plastic(",
            "pub const fn satin(",
            "pub const fn leather(",
            "pub const fn clear_glass(",
            "pub const fn frosted_glass(",
            "pub const fn rubber()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/material_showcase.rs",
        &[
            "pub fn material_preset_showcase()",
            "pub const fn glass_background_target_bars()",
            "MaterialShowcaseBackgroundBar",
            "MaterialShowcaseGeometry::CurvedPart",
            "MaterialShowcaseGeometry::BrushedPlate",
            "MaterialShowcaseGeometry::GlassBlockGrid",
            "MaterialShowcaseGeometry::GlassScreenGrid",
            "MaterialShowcaseGeometry::StrapPanel",
            "MaterialShowcaseGeometry::GasketFoot",
            "SOURCE_BACKED_SURFACE",
            "Assets::material_presets()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_e_material_showcase.rs",
        &[
            "round_e_material_showcase_is_shared_and_not_a_single_shape_grid",
            "round_e_material_showcase_names_source_backed_surface_separately",
            "round_e_glass_background_target_is_opaque_scene_color_geometry",
            "MaterialShowcaseGeometry::GlassBlockGrid",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/geometry_generated_uvs.rs",
        &[
            "generated_showcase_triangle_meshes_carry_non_degenerate_uvs",
            "must not collapse material textures to a single texel",
            "must span the full material texture domain",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/assets/material_presets.rs",
        &[
            "pub struct MaterialPresetAssets",
            "pub struct MaterialPresetProvenance",
            "pub fn material_presets(&self)",
            "pub async fn satin(&self)",
            "pub async fn leather(&self)",
            "pub async fn rubber(&self)",
            "ambientCG Fabric001",
            "ambientCG Leather001",
            "ambientCG Rubber002",
            "texture_bytes_budget",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_e_source_backed_material_presets.rs",
        &[
            "source_backed_material_presets_load_texture_backed_surfaces",
            "base_color_texture",
            "normal_texture",
            "metallic_roughness_texture",
            "has_decoded_pixels",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "demo/samples/SOURCES.md",
        &[
            "ambientCG `Fabric001`",
            "ambientCG `Leather001`",
            "ambientCG `Rubber002`",
            "cea13f8f2b44ba8a9d4bced83d26ab344e39502dba24b0b4025b7bd3a180a4c2",
            "8d3ac9280bec6a1e1e5384b93e5130414a085033290dde305bacaddd0aa6b96a",
            "CC0",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/round_b_material_presets.rs",
        &[
            "honest_material_presets_are_public_pbr_shortcuts",
            "expanded_material_presets_use_only_backed_material_lanes",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/examples_visual_proof.rs",
        &[
            "round_b_material_preset_reference_docs_image",
            "assert_round_e_reference_docs_image_metrics",
            "round_e_material_reference_docs_image_metrics",
            "material_identity_thresholds",
            "round-b-material-preset-reference-docs-image",
            "reference-image+docs-image",
            "material_preset_showcase",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/diagnostics/diagnostic.rs",
        &["MaterialPresetFallback"],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/diagnostics/capabilities.rs",
        &[
            "DiagnosticCode::MaterialPresetFallback",
            "Complete real-world material presets are in fallback",
            "Assets::material_presets() only for lanes with Round E",
            "capability row proof artifacts",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "docs/guides/easy-scene-setup.md",
        &[
            "MaterialDesc::matte(",
            "MaterialDesc::plastic(",
            "MaterialDesc::metal(",
            "MaterialDesc::rough_metal(",
            "MaterialDesc::chrome()",
            "MaterialDesc::brushed_steel()",
            "MaterialDesc::clearcoat_plastic(",
            "MaterialDesc::satin(",
            "MaterialDesc::leather(",
            "MaterialDesc::clear_glass(",
            "MaterialDesc::frosted_glass(",
            "MaterialDesc::rubber()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "src/browser_probe/workflows/pbr/material_presets.rs",
        &[
            "material_presets_scene",
            "material_preset_showcase",
            "browser-pbr-material-preset-expanded-set",
            "webgl2_smooth_metal_sample_floor",
            "scene-color-ior-thickness-rough-blur-sorted-transparency",
            "/demo/samples/environment/white_studio_03_1k.hdr",
            "showcase_geometry",
            "source_surfaces",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "tests/browser/m6_rust_wasm_renderer_probe.js",
        &[
            "assertMaterialPresetProof",
            "pbr-material-presets",
            "webgl2_smooth_metal_sample_floor < 96",
            "/demo/samples/environment/white_studio_03_1k.hdr",
            "single-shape grid",
            "Assets::material_presets()",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "scripts/probe_cloudflare_material_presets.mjs",
        &[
            "round-e-cloudflare-material-proof",
            "https://scena-demo.pages.dev/?sample=material-presets",
            "delta_e2000_vs_reference",
            "reference_delta_gate",
            "Public material approval must fail closed",
            "return true;",
            "neighbor_delta_e2000",
            "specular_dynamic_range",
            "anisotropy_aspect_ratio_ibl",
            "clearcoat_lobe_delta",
            "texture_variance",
            "roughness_variance",
            "sheen_width",
            "passed_physical_refraction",
            "darkTargetOffset",
            "dark_target_pixel_count",
            "physical_refraction_status",
            "passed_high_frequency_contrast_reduction",
            "sobelEdgeEnergy",
            "rough_transmission_status",
            "checksum_matches_build",
            "environment_hdr_sha256",
            "cache_buster",
            "CIEDE2000",
        ],
    );
    require_contains(
        root,
        findings,
        "HONEST-MATERIAL-PRESETS",
        "package.json",
        &[
            "cloudflare:materials",
            "scripts/probe_cloudflare_material_presets.mjs",
            "showcase:probe",
            "scripts/probe_showcase_demo.mjs",
        ],
    );
    require_contains(
        root,
        findings,
        "WEBGL2-IBL-SMOOTH-METAL",
        "src/render/prepare/environment_prefilter.rs",
        &[
            "sample_count_for_roughness(0.28, EnvironmentPrefilterQuality::InteractiveWebGl2)",
            "2 => 96",
            "_ => 192",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "src/demo_page.rs",
        &["samples/environment/white_studio_03_1k.hdr"],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "demo/samples/SOURCES.md",
        &[
            "white_studio_03",
            "neutral studio HDRI",
            "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "tests/m8_real_asset_proof.rs",
        &[
            "polyhaven_white_studio_demo_hdr_is_real_documented_neutral_radiance_file",
            "WHITE_STUDIO_DEMO_HDR_PATH",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "demo/index.html",
        &[
            "scena 1.5 live showcase",
            "Beautiful 3D in Rust",
            "Twelve materials. Twelve names.",
            "technical proof",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "demo/proof/index.html",
        &[
            "scena proof harness",
            "Render controls",
            "Diagnostics",
            "proof.js",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "demo/main.js",
        &[
            "load_material_presets_scene",
            "load_material_presets_scene(canvas.width, canvas.height)",
            "set_fixed_exposure_ev(app, 0.0)",
            "applyCanvasBackground(\"dark_studio\")",
            "background_scheme_css_color",
            "load_single_material_sphere_scene",
            "ResizeObserver",
            "browser-rendered WebGL2 material showcase",
        ],
    );
    require_contains(
        root,
        findings,
        "INTERNAL-MATERIAL-SPHERE-REVIEW",
        "demo/internal-material-spheres.html",
        &[
            "scena internal material sphere review",
            "12 material preset spheres",
            "sphere-labels",
            "12 spheres rendered live by scena WebGL2",
            "internal-material-spheres.js",
        ],
    );
    require_contains(
        root,
        findings,
        "INTERNAL-MATERIAL-SPHERE-REVIEW",
        "demo/internal-material-spheres.js",
        &[
            "MAX_RENDER_DIMENSION = 1920",
            "load_material_spheres_scene",
            "set_background_scheme(app, \"dark_studio\")",
            "set_fixed_exposure_ev(app, 0.0)",
            "requestRender()",
        ],
    );
    require_contains(
        root,
        findings,
        "DEMO-MATERIAL-PRESETS-PROOF",
        "src/demo_page/material_presets.rs",
        &[
            "material_preset_showcase",
            "background_target_position",
            "assets.material_presets().satin().await",
            "assets.material_presets().leather().await",
            "assets.material_presets().rubber().await",
        ],
    );
}

const ROUND_E_FIXTURE: &str = "tests/visual/references/round_e_material_fixture.toml";
const ROUND_E_THRESHOLDS: &str = "tests/visual/references/round_e_material_thresholds.toml";
const ROUND_E_PRESETS: &[&str] = &[
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

fn check_round_e_material_fixture_contract(root: &Path, findings: &mut Vec<Finding>) {
    let fixture_path = root.join(ROUND_E_FIXTURE);
    let threshold_path = root.join(ROUND_E_THRESHOLDS);
    let Ok(fixture) = fs::read_to_string(&fixture_path) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_FIXTURE}"),
        ));
        return;
    };
    let Ok(thresholds) = fs::read_to_string(&threshold_path) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("could not read {ROUND_E_THRESHOLDS}"),
        ));
        return;
    };

    for (key, minimum) in [
        ("chrome.specular_dynamic_range", 2.0),
        ("chrome.bright_reflection_luminance_p99_min", 230.0),
        ("chrome.reflection_edge_contrast", 0.30),
        ("brushed_steel.anisotropy_aspect_ratio_direct", 3.0),
        ("brushed_steel.anisotropy_aspect_ratio_ibl", 2.0),
        ("clearcoat_plastic.clearcoat_lobe_delta", 0.05),
        ("clear_glass.refraction_offset_min", 4.0),
        ("frosted_glass.high_frequency_contrast_reduction_min", 0.50),
        ("leather.texture_variance_min", 0.02),
        ("rubber.roughness_variance_min", 0.02),
        ("satin.sheen_width_min", 0.20),
        ("global.neighbor_delta_e2000_min", 6.0),
    ] {
        require_threshold_bound(&thresholds, key, ThresholdCheck::Minimum(minimum), findings);
    }
    for (key, maximum) in [
        ("chrome.dark_reflection_luminance_p05_max", 85.0),
        ("clear_glass.background_delta_e2000_max", 8.0),
        ("global.reference_delta_e2000_max", 4.0),
    ] {
        require_threshold_bound(&thresholds, key, ThresholdCheck::Maximum(maximum), findings);
    }

    let entries = parse_round_e_fixture(&fixture);
    for preset in ROUND_E_PRESETS {
        let Some(entry) = entries.get(*preset) else {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{ROUND_E_FIXTURE} is missing preset {preset}"),
            ));
            continue;
        };
        require_fixture_value(
            entry,
            "environment_hdr_path",
            "demo/samples/environment/white_studio_03_1k.hdr",
            *preset,
            findings,
        );
        require_fixture_value(
            entry,
            "environment_hdr_sha256",
            "ae94a965734e6306216feb48d6dd7154b1dbc484a605200bf13cb9ae23799b7b",
            *preset,
            findings,
        );
        require_fixture_value(
            entry,
            "reference_renderer",
            "@google/model-viewer",
            *preset,
            findings,
        );
        let reference_path = entry
            .get("reference_path")
            .map(|value| unquote(value))
            .unwrap_or_default();
        let reference_sha = entry
            .get("reference_sha256")
            .map(|value| unquote(value))
            .unwrap_or_default();
        if !reference_path.starts_with("tests/visual/references/round_e/") {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{preset} reference_path must stay under tests/visual/references/round_e"),
            ));
            continue;
        }
        let full_reference_path = root.join(&reference_path);
        if !full_reference_path.is_file() {
            findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("{preset} reference PNG is missing at {reference_path}"),
            ));
            continue;
        }
        match sha256_hex(&full_reference_path) {
            Ok(actual) if actual == reference_sha => {}
            Ok(actual) => findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!(
                    "{preset} reference SHA mismatch: fixture {reference_sha}, actual {actual}"
                ),
            )),
            Err(error) => findings.push(Finding::new(
                "HONEST-MATERIAL-PRESETS",
                format!("could not hash {reference_path}: {error}"),
            )),
        }
    }
}

enum ThresholdCheck {
    Minimum(f32),
    Maximum(f32),
}

fn require_threshold_bound(
    thresholds: &str,
    key: &str,
    check: ThresholdCheck,
    findings: &mut Vec<Finding>,
) {
    let Some(value) = parse_threshold_value(thresholds, key) else {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{ROUND_E_THRESHOLDS} is missing numeric threshold {key}"),
        ));
        return;
    };
    match check {
        ThresholdCheck::Minimum(minimum) if value < minimum => findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{key}={value} is below the committed floor {minimum}"),
        )),
        ThresholdCheck::Maximum(maximum) if value > maximum => findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{key}={value} is above the committed ceiling {maximum}"),
        )),
        _ => {}
    }
}

fn parse_threshold_value(thresholds: &str, key: &str) -> Option<f32> {
    let (wanted_section, wanted_key) = key.split_once('.')?;
    let mut section = "";
    for raw_line in thresholds.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            section = name.trim();
            continue;
        }
        if section == wanted_section {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if name.trim() == wanted_key {
                return value.trim().parse::<f32>().ok();
            }
        }
    }
    None
}

fn parse_round_e_fixture(fixture: &str) -> BTreeMap<String, BTreeMap<String, String>> {
    let mut current: Option<String> = None;
    let mut entries = BTreeMap::<String, BTreeMap<String, String>>::new();
    for raw_line in fixture.lines() {
        let line = raw_line
            .split_once('#')
            .map_or(raw_line, |(prefix, _)| prefix)
            .trim();
        if line.is_empty() {
            continue;
        }
        if let Some(section) = line
            .strip_prefix("[[presets.")
            .and_then(|value| value.strip_suffix("]]"))
        {
            current = Some(section.to_string());
            entries.entry(section.to_string()).or_default();
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(current) = current.as_ref() else {
            continue;
        };
        entries
            .entry(current.clone())
            .or_default()
            .insert(key.trim().to_string(), value.trim().to_string());
    }
    entries
}

fn require_fixture_value(
    entry: &BTreeMap<String, String>,
    key: &str,
    expected: &str,
    preset: &str,
    findings: &mut Vec<Finding>,
) {
    if entry.get(key).map(|value| unquote(value)) != Some(expected.to_string()) {
        findings.push(Finding::new(
            "HONEST-MATERIAL-PRESETS",
            format!("{preset} fixture must pin {key} = {expected}"),
        ));
    }
}

fn unquote(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}
