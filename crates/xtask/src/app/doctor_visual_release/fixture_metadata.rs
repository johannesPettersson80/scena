use crate::app::prelude::*;

pub(crate) fn check_default_environment_manifest(root: &Path, findings: &mut Vec<Finding>) {
    let manifest_rel = "tests/assets/environment/default-environment.toml";
    let manifest_path = root.join(manifest_rel);
    let Ok(text) = fs::read_to_string(&manifest_path) else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("missing required default environment manifest {manifest_rel}"),
        ));
        return;
    };

    require_manifest_value(findings, manifest_rel, &text, "name", "neutral-studio");
    require_manifest_value(findings, manifest_rel, &text, "license", "CC0-1.0");
    require_manifest_value(findings, manifest_rel, &text, "wasm_delivery", "bundled");
    require_manifest_value(
        findings,
        manifest_rel,
        &text,
        "status",
        "text-fixture-not-ibl-proof",
    );
    require_manifest_u32(findings, manifest_rel, &text, "cubemap_resolution", 256);
    require_manifest_u32(findings, manifest_rel, &text, "brdf_lut_size", 256);

    let Some(source_path) = quoted_manifest_assignment(&text, "source_path") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_path"),
        ));
        return;
    };
    let Some(source_sha256) = quoted_manifest_assignment(&text, "source_sha256") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing source_sha256"),
        ));
        return;
    };
    let Some(generator) = quoted_manifest_assignment(&text, "generator") else {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} is missing generator"),
        ));
        return;
    };
    if !generator.contains(&source_path) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} generator does not reference source_path"),
        ));
    }
    if binary_render_asset_extension(Path::new(&source_path)) {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} source_path {source_path} must not use a binary render asset extension unless real binary bytes are committed"),
        ));
    }
    check_manifest_file_hash(root, findings, manifest_rel, &source_path, &source_sha256);

    let derivatives = derivative_manifest_entries(&text);
    if derivatives.len() < 2 {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} must declare at least cubemap and BRDF LUT derivatives"),
        ));
    }
    for (path, sha256) in derivatives {
        if path.contains("placeholder") {
            findings.push(Finding::new(
                "VISUAL-DEFAULT-ENV",
                format!("{manifest_rel} derivative {path} still points at a placeholder file"),
            ));
        }
        if binary_render_asset_extension(Path::new(&path)) {
            findings.push(Finding::new(
                "VISUAL-DEFAULT-ENV",
                format!("{manifest_rel} derivative {path} must not use a binary render asset extension unless real binary bytes are committed"),
            ));
        }
        check_default_environment_derivative_payload(root, findings, manifest_rel, &path);
        check_manifest_file_hash(root, findings, manifest_rel, &path, &sha256);
    }
}

pub(crate) fn check_default_environment_derivative_payload(
    root: &Path,
    findings: &mut Vec<Finding>,
    manifest_rel: &str,
    path: &str,
) {
    let Ok(text) = fs::read_to_string(root.join(path)) else {
        return;
    };
    if text.contains("not a renderer-consumable") {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} declares itself non-consumable"),
        ));
    }
    let valid_magic =
        text.starts_with("SCENA_CUBEMAP_V1\n") || text.starts_with("SCENA_BRDF_LUT_V1\n");
    if !valid_magic {
        findings.push(Finding::new(
            "VISUAL-DEFAULT-ENV",
            format!("{manifest_rel} derivative {path} is missing a scena environment magic header"),
        ));
    }
}

pub(crate) fn check_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    check_ndc_smoke_fixture_classification(
        root,
        findings,
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "primitive-fullscreen",
            "unlit-asset-mesh",
            "pbr-asset-mesh",
            "transparent-blend",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/fixtures/m1-headless-core.toml",
        &[
            "[suite]",
            "name = \"m1-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m1-visual\"",
            "reference = \"tests/visual/references/m1-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"primitive-fullscreen\"",
            "name = \"unlit-asset-mesh\"",
            "name = \"pbr-asset-mesh\"",
            "name = \"transparent-blend\"",
            "name = \"line-material\"",
            "name = \"wire-edge-materials\"",
            "name = \"default-cube\"",
            "luminance_gate = \"center-nonblack\"",
            "silhouette_gate = \"corner-black\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/visual/references/m1-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [110, 189, 240, 255]",
            "nonblack_pixels = 117",
            "rgba_hash = \"fnv1a64:cfc5e9027c8e3ed0\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-FIXTURE-METADATA",
        "tests/m1_visual_proof.rs",
        &[
            "m1_headless_visual_artifacts_cover_core_material_paths",
            "m1_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m1-visual",
            "include_str!(\"visual/fixtures/m1-headless-core.toml\")",
            "include_str!(\"visual/references/m1-headless-core.toml\")",
            "rgba_within_tolerance",
            "rgba_fnv1a64",
        ],
    );
}

pub(crate) fn check_m2_visual_fixture_metadata(root: &Path, findings: &mut Vec<Finding>) {
    check_ndc_smoke_fixture_classification(
        root,
        findings,
        "tests/visual/fixtures/m2-headless-core.toml",
        &[
            "direct-lights-pbr",
            "shadowed-directional-light",
            "ibl-environment",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/fixtures/m2-headless-core.toml",
        &[
            "[suite]",
            "name = \"m2-headless-core\"",
            "format = \"ppm\"",
            "encoding = \"srgb8\"",
            "artifact_dir = \"target/gate-artifacts/m2-visual\"",
            "reference = \"tests/visual/references/m2-headless-core.toml\"",
            "reference_mode = \"sampled-rgba\"",
            "max_abs_diff = 0",
            "name = \"direct-lights-pbr\"",
            "name = \"shadowed-directional-light\"",
            "name = \"ibl-environment\"",
            "name = \"fxaa-edge\"",
            "name = \"clipping-half-space\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/visual/references/m2-headless-core.toml",
        &[
            "[suite]",
            "status = \"reference\"",
            "max_abs_diff = 0",
            "center_rgba = [151, 0, 0, 255]",
            "center_rgba = [80, 80, 80, 255]",
            "nonblack_pixels = 149",
            "rgba_hash = \"fnv1a64:4d3a874730a5e5bc\"",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "tests/m2_visual_proof.rs",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2_headless_reference_tolerances_match_current_fixtures",
            "write_ppm_artifact",
            "target/gate-artifacts/m2-visual",
            "include_str!(\"visual/fixtures/m2-headless-core.toml\")",
            "include_str!(\"visual/references/m2-headless-core.toml\")",
            "validate_shadowed_directional_light",
            "validate_ibl_environment",
            "validate_clipping_half_space",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-M2-FIXTURE-METADATA",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "m2_headless_visual_artifacts_cover_lighting_depth_and_clipping",
            "m2-headless-core.toml",
            "VISUAL-M2-FIXTURE-METADATA",
        ],
    );
}

pub(crate) fn check_ndc_smoke_fixture_classification(
    root: &Path,
    findings: &mut Vec<Finding>,
    fixture_rel: &str,
    fixture_names: &[&str],
) {
    let Ok(text) = fs::read_to_string(root.join(fixture_rel)) else {
        findings.push(Finding::new(
            "VISUAL-HARNESS-SMOKE-P0",
            format!("could not read {fixture_rel}"),
        ));
        return;
    };

    for name in fixture_names {
        let Some(block) = fixture_block(&text, name) else {
            findings.push(Finding::new(
                "VISUAL-HARNESS-SMOKE-P0",
                format!("{fixture_rel} is missing fixture '{name}'"),
            ));
            continue;
        };
        for required in [
            "proof_class = \"harness-smoke\"",
            "production_claim = false",
        ] {
            if !block.contains(required) {
                findings.push(Finding::new(
                    "VISUAL-HARNESS-SMOKE-P0",
                    format!(
                        "{fixture_rel} fixture '{name}' must contain {required} so NDC/fullscreen smoke cannot satisfy production proof"
                    ),
                ));
            }
        }
    }
}

pub(crate) fn fixture_block<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!("name = \"{name}\"");
    let start = text.find(&needle)?;
    let rest = &text[start..];
    let next_fixture = rest.find("\n[[fixture]]").unwrap_or(rest.len());
    Some(&rest[..next_fixture])
}

pub(crate) fn check_m1_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "Cargo.toml",
        &[
            "wasm-bindgen",
            "wasm-bindgen-test",
            "CanvasRenderingContext2d",
            "ImageData",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "tests/m1_browser_rendered_output.rs",
        &[
            "wasm_bindgen_test_configure!(run_in_browser)",
            "fn m1_browser_wasm_renders_color_and_alpha_to_canvas",
            "fn m1_browser_wasm_renders_technical_materials_to_canvas",
            "Renderer::headless(4, 4)",
            "MaterialDesc::line",
            "MaterialDesc::wireframe",
            "MaterialDesc::edge",
            "put_image_data",
            "get_image_data",
            "[158, 0, 159, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M1",
        "docs/checklists/m1-geometry-materials.md",
        &[
            "m1_browser_rendered_output",
            "Rust/WASM browser rendered-output proof",
        ],
    );
}

pub(crate) fn check_m2_browser_rendered_output(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.js",
        &[
            "m2_browser_lighting_clipping_smoke.html",
            "scenaM2BrowserLightingClippingSmoke",
            "webgl2",
            "webgpu",
            "m2-browser-lighting-clipping-smoke.json",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "tests/browser/m2_browser_lighting_clipping_smoke.html",
        &[
            "runWebGpuScene",
            "runWebGl2Scene",
            "directLightPassed",
            "clippingPassed",
            "vec4<f32>(1.0, 0.0, 0.0, 1.0)",
            "vec4(1.0, 0.0, 0.0, 1.0)",
            "directCenter",
            "clippingLeft",
            "clippingRight",
            "clippingNonBlackPixels",
        ],
    );
    require_contains(
        root,
        findings,
        "VISUAL-BROWSER-M2",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "node tests/browser/m2_browser_lighting_clipping_smoke.js",
            "m2-browser-lighting-clipping-smoke.json",
            "VISUAL-BROWSER-M2",
        ],
    );
}
