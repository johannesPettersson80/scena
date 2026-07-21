use crate::app::prelude::*;

pub(crate) fn directional_shadow_shader_has_pcf3x3(shader: &str) -> bool {
    shader
        .matches("textureSampleCompareLevel(shadow_map, shadow_sampler")
        .count()
        == 9
        && shader.contains("textureDimensions(shadow_map)")
        && shader.contains("shadow_texel_size")
        && shader.contains("shadow_visibility / 9.0")
}

pub(super) fn check_q05_directional_shadow_pcf3x3(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q05-SHADOW-PCF3X3";
    let required: &[(&str, &[&str])] = &[
        (
            "src/render/gpu/shadow.rs",
            &[
                "averages an explicit 3×3 texel grid",
                "mag_filter: wgpu::FilterMode::Nearest",
                "min_filter: wgpu::FilterMode::Nearest",
            ],
        ),
        (
            "src/render/gpu/output.rs",
            &[
                "triangle_shaders_use_nine_comparison_taps_for_reported_pcf3x3",
                "has_directional_shadow_pcf3x3",
                "single_tap_mutation",
            ],
        ),
        (
            "src/render/prepare/stats.rs",
            &["DIRECTIONAL_SHADOW_PCF_KERNEL: u8 = 3"],
        ),
        (
            "src/diagnostics/capabilities.rs",
            &["directional_shadow_pcf_kernel: 3"],
        ),
        (
            "tests/assets/stable-contracts/capability_report.v1.json",
            &["\"directional_shadow_pcf_kernel\": 3"],
        ),
        (
            "tests/assets/stable-contracts/capture.v1.json",
            &["\"directional_shadow_pcf_kernel\": 3"],
        ),
        (
            "tests/assets/stable-contracts/capture_baseline.v1.json",
            &["\"directional_shadow_pcf_kernel\": 3"],
        ),
        (
            "docs/rendering.md",
            &[
                "explicit 3×3 texel grid",
                "nearest-filtered depth-comparison samples averaged once",
                "Point/spot shadow maps and cascaded",
                "directional maps are not currently shipped",
            ],
        ),
        (
            "docs/capabilities.md",
            &[
                "directional_shadow_pcf_kernel: 3",
                "point/spot and cascaded shadows",
            ],
        ),
        ("README.md", &["nine-comparison-tap 3×3 PCF"]),
        (
            "CHANGELOG.md",
            &["reported directional-shadow PCF 3×3 kernel real"],
        ),
        (
            "docs/release-notes/v1.8.0.md",
            &[
                "fragment shader issued one linearly filtered",
                "depth comparison, an implicit 2×2 footprint",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    for relative in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/gpu/output_shader_texture_2d.wgsl",
    ] {
        let valid = match fs::read_to_string(root.join(relative)) {
            Ok(source) => directional_shadow_shader_has_pcf3x3(&source),
            Err(_) => false,
        };
        if !valid {
            findings.push(Finding::new(
                RULE,
                format!(
                    "{relative} must implement the reported PCF 3x3 kernel as exactly nine comparison taps with texel-sized offsets and one 1/9 average"
                ),
            ));
        }
    }
}
