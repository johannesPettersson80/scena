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

/// Native targets filter cascaded shadows with percentage-closer soft
/// shadows. The kernel must never be smaller than the reported 3x3 grid, and
/// the blocker search that sizes it must read real occluder depths.
pub(crate) fn directional_shadow_shader_has_pcss(shader: &str) -> bool {
    let filter_taps = shader
        .split_once("const SHADOW_FILTER_TAPS: u32 = ")
        .and_then(|(_, tail)| tail.split_once("u;"))
        .and_then(|(count, _)| count.parse::<u32>().ok())
        .unwrap_or(0);
    filter_taps >= 9
        && shader.contains("for (var i = 0u; i < SHADOW_FILTER_TAPS; i = i + 1u) {")
        && shader.contains(
            "visibility += textureSampleCompareLevel(shadow_map, shadow_sampler, tap / atlas, compare_depth);",
        )
        && shader.contains("return visibility / f32(SHADOW_FILTER_TAPS);")
        && shader.contains("let occluder = textureLoad(shadow_map, vec2<i32>(tap), 0);")
        && shader.contains("camera.shadow_cascades[cascade]")
}

pub(super) fn check_q05_directional_shadow_pcf3x3(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q05-SHADOW-PCF3X3";
    let required: &[(&str, &[&str])] = &[
        (
            "src/render/gpu/shadow.rs",
            &[
                "hardware 2x2 percentage-closer lookup",
                "mag_filter: wgpu::FilterMode::Linear",
                "min_filter: wgpu::FilterMode::Linear",
                "compare: Some(wgpu::CompareFunction::LessEqual)",
                "NATIVE_SHADOW_CASCADE_COUNT: usize = 4",
            ],
        ),
        (
            "src/render/gpu/output.rs",
            &[
                "triangle_shaders_use_nine_comparison_taps_for_reported_pcf3x3",
                "has_directional_shadow_pcf3x3",
                "single_tap_mutation",
                "has_directional_shadow_pcss",
                "narrow_kernel_mutation",
                "no_blocker_search_mutation",
                "include_str!(\"shadow_filter_cascaded.wgsl\")",
                "include_str!(\"shadow_filter_single.wgsl\")",
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
                "fit four cascades to the camera frustum",
                "percentage-closer soft shadows",
                "explicit 3×3 grid of nine",
                "depth-comparison samples averaged once per shadowed fragment",
                "smallest comparison kernel any GPU backend runs",
                "Point/spot shadow maps",
                "are not currently shipped",
            ],
        ),
        (
            "docs/capabilities.md",
            &[
                "`directional_shadow_pcf_kernel: 3` is the smallest comparison kernel",
                "wider soft-shadow kernel over four",
                "point/spot shadows are not claimed",
            ],
        ),
        (
            "README.md",
            &[
                "four-cascade soft shadows on native targets",
                "nine-comparison-tap 3×3 PCF on WebGL2",
            ],
        ),
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

    type ShaderOracle = fn(&str) -> bool;
    let filters: [(&str, ShaderOracle, &str); 2] = [
        (
            "src/render/gpu/shadow_filter_single.wgsl",
            directional_shadow_shader_has_pcf3x3,
            "must implement the reported PCF 3x3 kernel as exactly nine comparison taps with texel-sized offsets and one 1/9 average",
        ),
        (
            "src/render/gpu/shadow_filter_cascaded.wgsl",
            directional_shadow_shader_has_pcss,
            "must filter cascaded shadows with a blocker search and at least nine comparison taps",
        ),
    ];
    for (relative, oracle, requirement) in filters {
        let valid = fs::read_to_string(root.join(relative)).is_ok_and(|source| oracle(&source));
        if !valid {
            findings.push(Finding::new(RULE, format!("{relative} {requirement}")));
        }
    }
}
