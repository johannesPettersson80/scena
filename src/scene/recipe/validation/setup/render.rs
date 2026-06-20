use serde_json::Value;

use super::{
    SceneRecipeDiagnosticV1, diagnostic, validate_finite_number_optional, validate_known_fields,
    validate_non_negative_number_required, validate_u8, validate_u8_max,
    validate_unit_number_required,
};

const RENDER_FIELDS: &[&str] = &[
    "profile",
    "quality",
    "anti_aliasing",
    "supersample",
    "bloom",
    "ssao",
    "exposure_ev",
    "tonemapper",
];
const BLOOM_FIELDS: &[&str] = &["threshold_srgb", "intensity", "radius_px"];
const SSAO_FIELDS: &[&str] = &["radius_px", "intensity", "depth_threshold"];

pub(in crate::scene::recipe::validation) fn validate_render_setup(
    render: Option<&Value>,
    diagnostics: &mut Vec<SceneRecipeDiagnosticV1>,
) {
    let Some(render) = render else {
        return;
    };
    let Some(object) = render.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render",
            "render must be an object",
            "emit render:{profile?,quality?,anti_aliasing?,supersample?,bloom?,ssao?,exposure_ev?,tonemapper?}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render", object, RENDER_FIELDS, diagnostics);
    super::validate_enum(
        "$.render.profile",
        object.get("profile"),
        &["auto", "quality", "balanced", "compatibility", "industrial"],
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.quality",
        object.get("quality"),
        &["low", "medium", "high"],
        "invalid_render_setting",
        diagnostics,
    );
    super::validate_enum(
        "$.render.anti_aliasing",
        object.get("anti_aliasing"),
        &["none", "fxaa", "msaa4", "msaa8"],
        "invalid_render_setting",
        diagnostics,
    );
    validate_supersample(object.get("supersample"), diagnostics);
    super::validate_enum(
        "$.render.tonemapper",
        object.get("tonemapper"),
        &["standard", "aces", "pbr_neutral"],
        "invalid_render_setting",
        diagnostics,
    );
    validate_finite_number_optional(
        "$.render.exposure_ev",
        object.get("exposure_ev"),
        diagnostics,
    );
    validate_bloom(object.get("bloom"), diagnostics);
    validate_ssao(object.get("ssao"), diagnostics);
}

fn validate_supersample(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(factor) = value.as_u64() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.supersample",
            "supersample must be an integer factor 1, 2, 3, or 4",
            "emit supersample:2, supersample:3, or supersample:4 for hero-shot quality; cost grows with N^2",
            None,
            false,
        ));
        return;
    };
    if !(1..=4).contains(&factor) {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.supersample",
            "supersample must be 1, 2, 3, or 4",
            "use 1 to disable full-frame supersampling; use 2-4 only for hero-shot quality because cost grows with N^2",
            None,
            false,
        ));
    }
}

fn validate_bloom(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.bloom",
            "bloom must be an object",
            "emit bloom:{threshold_srgb,intensity,radius_px}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render.bloom", object, BLOOM_FIELDS, diagnostics);
    validate_u8(
        "$.render.bloom.threshold_srgb",
        object.get("threshold_srgb"),
        diagnostics,
    );
    validate_unit_number_required(
        "$.render.bloom.intensity",
        object.get("intensity"),
        diagnostics,
    );
    validate_u8_max(
        "$.render.bloom.radius_px",
        object.get("radius_px"),
        12,
        diagnostics,
    );
}

fn validate_ssao(value: Option<&Value>, diagnostics: &mut Vec<SceneRecipeDiagnosticV1>) {
    let Some(value) = value else {
        return;
    };
    let Some(object) = value.as_object() else {
        diagnostics.push(diagnostic(
            "invalid_render_setting",
            "error",
            "$.render.ssao",
            "ssao must be an object",
            "emit ssao:{radius_px,intensity,depth_threshold}",
            None,
            false,
        ));
        return;
    };
    validate_known_fields("$.render.ssao", object, SSAO_FIELDS, diagnostics);
    validate_u8_max(
        "$.render.ssao.radius_px",
        object.get("radius_px"),
        12,
        diagnostics,
    );
    validate_unit_number_required(
        "$.render.ssao.intensity",
        object.get("intensity"),
        diagnostics,
    );
    validate_non_negative_number_required(
        "$.render.ssao.depth_threshold",
        object.get("depth_threshold"),
        diagnostics,
    );
}
