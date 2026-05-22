use crate::app::prelude::*;
use crate::app::tests_12::{VALID_GUIDE, write_easy_scene_fixture};

pub(crate) fn expanded_material_preset_guide() -> String {
    format!(
        "{VALID_GUIDE} viewer.play_clip(\n```rust\nlet matte = assets.create_material(MaterialDesc::matte(Color::GRAY));\nlet body = assets.create_material(MaterialDesc::plastic(Color::BLUE));\nlet shaft = assets.create_material(MaterialDesc::metal(Color::LIGHT_GRAY));\nlet rough = assets.create_material(MaterialDesc::rough_metal(Color::GRAY));\nlet chrome = assets.create_material(MaterialDesc::chrome());\nlet steel = assets.create_material(MaterialDesc::brushed_steel());\nlet cover = assets.create_material(MaterialDesc::clearcoat_plastic(Color::BLUE));\nlet satin = assets.create_material(MaterialDesc::satin(Color::GRAY));\nlet leather = assets.create_material(MaterialDesc::leather(Color::GRAY));\nlet glass = assets.create_material(MaterialDesc::clear_glass(Color::CYAN));\nlet frosted = assets.create_material(MaterialDesc::frosted_glass(Color::CYAN));\nlet foot = assets.create_material(MaterialDesc::rubber());\n```",
    )
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
        "material_presets_scene browser-pbr-material-preset-expanded-set webgl2_smooth_metal_sample_floor blend-plus-transmission-preview-no-refraction-claim",
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
        "assertMaterialPresetProof pbr-material-presets webgl2_smooth_metal_sample_floor < 96",
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
