use serde_json::json;

use super::checks::{checked_check, error_check, observed_pairs};
use crate::{
    AntiAliasing, Backend, ReconstructionFilter, RenderIntrospectionReportV1, Renderer,
    SceneCompositionCheckV1, SceneRecipeExpectV1, SceneRecipeV1,
};

pub(super) fn composition_backend_conformance_checks(
    recipe: &SceneRecipeV1,
    renderer: &Renderer,
    introspection: &RenderIntrospectionReportV1,
    expect: Option<&SceneRecipeExpectV1>,
) -> Vec<SceneCompositionCheckV1> {
    let mut checks = Vec::new();
    if let Some(expect_backend) = expect.and_then(|expect| expect.expect_backend.as_ref()) {
        let actual_backend = backend_name(introspection.capabilities.backend);
        let actual_gpu_device = introspection.capabilities.gpu_device;
        let backend_matches = expect_backend
            .backend
            .as_deref()
            .is_none_or(|expected| expected == actual_backend);
        let gpu_matches = expect_backend
            .gpu_device
            .is_none_or(|expected| expected == actual_gpu_device);
        let observed = observed_pairs([
            ("expected_backend", json!(expect_backend.backend)),
            ("actual_backend", json!(actual_backend)),
            ("expected_gpu_device", json!(expect_backend.gpu_device)),
            ("actual_gpu_device", json!(actual_gpu_device)),
        ]);
        if backend_matches && gpu_matches {
            checks.push(checked_check(
                "expect_backend".to_owned(),
                "backend_conformance",
                "backend_expectation_satisfied",
                Some("expect_backend".to_owned()),
                Vec::new(),
                observed,
                (
                    "actual render backend matches the declared backend expectation",
                    "no action needed",
                ),
            ));
        } else {
            checks.push(error_check(
                "expect_backend".to_owned(),
                "backend_conformance",
                "backend_expectation_mismatch",
                Some("expect_backend".to_owned()),
                Vec::new(),
                observed,
                (
                    "actual render backend does not match the declared backend expectation",
                    "rerun with the required backend flag/capability or relax expect_backend only if fallback is acceptable",
                ),
            ));
        }
    }

    if let Some(render) = recipe.render.as_ref() {
        if let Some(expected) = render.anti_aliasing.as_deref() {
            let actual = anti_aliasing_name(renderer.anti_aliasing());
            push_string_conformance(
                &mut checks,
                StringConformance {
                    id: "render.anti_aliasing",
                    pass_code: "render_antialiasing_active",
                    fail_code: "render_antialiasing_mismatch",
                    expected,
                    actual,
                    pass_message: "anti-aliasing mode requested by the recipe is active",
                    fail_message: "anti-aliasing mode requested by the recipe is not active",
                    fix_hint: "set render.anti_aliasing to the intended mode or fix renderer setup",
                },
            );
        }
        if let Some(expected) = render.supersample {
            let actual = renderer.supersample_factor();
            let observed =
                observed_pairs([("expected", json!(expected)), ("actual", json!(actual))]);
            if actual == u32::from(expected) {
                checks.push(checked_check(
                    "render.supersample".to_owned(),
                    "backend_conformance",
                    "render_supersample_active",
                    Some("render.supersample".to_owned()),
                    Vec::new(),
                    observed,
                    (
                        "supersample factor requested by the recipe is active",
                        "no action needed",
                    ),
                ));
            } else {
                checks.push(error_check(
                    "render.supersample".to_owned(),
                    "backend_conformance",
                    "render_supersample_mismatch",
                    Some("render.supersample".to_owned()),
                    Vec::new(),
                    observed,
                    (
                        "supersample factor requested by the recipe is not active",
                        "lower the requested factor to a supported value or fix renderer setup",
                    ),
                ));
            }
        }
        if let Some(expected) = render.reconstruction.as_deref() {
            let actual = reconstruction_name(renderer.reconstruction_filter());
            push_string_conformance(
                &mut checks,
                StringConformance {
                    id: "render.reconstruction",
                    pass_code: "render_reconstruction_active",
                    fail_code: "render_reconstruction_mismatch",
                    expected,
                    actual,
                    pass_message: "reconstruction filter requested by the recipe is active",
                    fail_message: "reconstruction filter requested by the recipe is not active",
                    fix_hint: "set render.reconstruction to the intended filter or fix renderer setup",
                },
            );
        }
        if render.bloom.is_some() {
            push_bool_conformance(
                &mut checks,
                "render.bloom",
                "render_bloom_active",
                "render_bloom_inactive",
                renderer.bloom().is_some(),
                "bloom post-process requested by the recipe is active",
                "restore bloom setup or remove render.bloom when the effect is not required",
            );
        }
        if render.ssao.is_some() {
            push_bool_conformance(
                &mut checks,
                "render.ssao",
                "render_ssao_active",
                "render_ssao_inactive",
                renderer.screen_space_ambient_occlusion().is_some(),
                "SSAO requested by the recipe is active",
                "restore SSAO setup or remove render.ssao when the effect is not required",
            );
        }
    }

    if let Some(environment) = recipe
        .scene
        .as_ref()
        .and_then(|scene| scene.environment.as_ref())
    {
        match environment
            .kind
            .as_deref()
            .or(environment.preset.as_deref().map(|_| "preset"))
        {
            Some("preset") => {
                let active = renderer.environment().is_some();
                if active || environment.optional {
                    checks.push(checked_check(
                        "scene.environment".to_owned(),
                        "backend_conformance",
                        if active {
                            "environment_active"
                        } else {
                            "environment_optional_skipped"
                        },
                        Some("scene.environment".to_owned()),
                        Vec::new(),
                        observed_pairs([
                            ("kind", json!("preset")),
                            ("preset", json!(environment.preset)),
                            ("optional", json!(environment.optional)),
                            ("active", json!(active)),
                        ]),
                        (
                            "recipe environment preset request was handled according to its optionality",
                            "restore environment preset loading or mark it optional only when IBL is not required",
                        ),
                    ));
                } else {
                    checks.push(error_check(
                        "scene.environment".to_owned(),
                        "backend_conformance",
                        "environment_missing",
                        Some("scene.environment".to_owned()),
                        Vec::new(),
                        observed_pairs([
                            ("kind", json!("preset")),
                            ("preset", json!(environment.preset)),
                            ("optional", json!(environment.optional)),
                            ("active", json!(active)),
                        ]),
                        (
                            "required recipe environment preset was not active",
                            "fix environment preset loading or mark it optional only when no IBL is acceptable",
                        ),
                    ));
                }
            }
            Some("default" | "uri") => {
                let active = renderer.environment().is_some();
                if active || environment.optional {
                    checks.push(checked_check(
                        "scene.environment".to_owned(),
                        "backend_conformance",
                        if active {
                            "environment_active"
                        } else {
                            "environment_optional_skipped"
                        },
                        Some("scene.environment".to_owned()),
                        Vec::new(),
                        observed_pairs([
                            ("kind", json!(environment.kind)),
                            ("optional", json!(environment.optional)),
                            ("active", json!(active)),
                        ]),
                        (
                            "recipe environment request was handled according to its optionality",
                            "no action needed",
                        ),
                    ));
                } else {
                    checks.push(error_check(
                        "scene.environment".to_owned(),
                        "backend_conformance",
                        "environment_missing",
                        Some("scene.environment".to_owned()),
                        Vec::new(),
                        observed_pairs([
                            ("kind", json!(environment.kind)),
                            ("optional", json!(environment.optional)),
                            ("active", json!(active)),
                        ]),
                        (
                            "required recipe environment is not active in the renderer",
                            "fix the environment URI/load path or mark it optional only when no IBL is acceptable",
                        ),
                    ));
                }
            }
            Some("none") => {
                checks.push(checked_check(
                    "scene.environment".to_owned(),
                    "backend_conformance",
                    "environment_none_active",
                    Some("scene.environment".to_owned()),
                    Vec::new(),
                    observed_pairs([("active", json!(renderer.environment().is_some()))]),
                    (
                        "recipe explicitly requested no environment lighting",
                        "no action needed",
                    ),
                ));
            }
            Some(_) | None => {}
        }
    }

    checks
}

struct StringConformance<'a> {
    id: &'a str,
    pass_code: &'a str,
    fail_code: &'a str,
    expected: &'a str,
    actual: &'a str,
    pass_message: &'a str,
    fail_message: &'a str,
    fix_hint: &'a str,
}

fn push_string_conformance(checks: &mut Vec<SceneCompositionCheckV1>, spec: StringConformance<'_>) {
    let observed = observed_pairs([
        ("expected", json!(spec.expected)),
        ("actual", json!(spec.actual)),
    ]);
    if spec.expected == spec.actual {
        checks.push(checked_check(
            spec.id.to_owned(),
            "backend_conformance",
            spec.pass_code,
            Some(spec.id.to_owned()),
            Vec::new(),
            observed,
            (spec.pass_message, "no action needed"),
        ));
    } else {
        checks.push(error_check(
            spec.id.to_owned(),
            "backend_conformance",
            spec.fail_code,
            Some(spec.id.to_owned()),
            Vec::new(),
            observed,
            (spec.fail_message, spec.fix_hint),
        ));
    }
}

fn push_bool_conformance(
    checks: &mut Vec<SceneCompositionCheckV1>,
    id: &str,
    pass_code: &str,
    fail_code: &str,
    active: bool,
    message: &str,
    fix_hint: &str,
) {
    let observed = observed_pairs([("active", json!(active))]);
    if active {
        checks.push(checked_check(
            id.to_owned(),
            "backend_conformance",
            pass_code,
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (message, "no action needed"),
        ));
    } else {
        checks.push(error_check(
            id.to_owned(),
            "backend_conformance",
            fail_code,
            Some(id.to_owned()),
            Vec::new(),
            observed,
            (message, fix_hint),
        ));
    }
}

fn backend_name(backend: Backend) -> &'static str {
    match backend {
        Backend::Headless => "headless",
        Backend::HeadlessGpu => "headless_gpu",
        Backend::SurfaceDescriptor => "surface_descriptor",
        Backend::NativeSurface => "native_surface",
        Backend::WebGpu => "web_gpu",
        Backend::WebGl2 => "web_gl2",
    }
}

fn anti_aliasing_name(anti_aliasing: AntiAliasing) -> &'static str {
    match anti_aliasing {
        AntiAliasing::None => "none",
        AntiAliasing::Fxaa => "fxaa",
        AntiAliasing::Msaa4 => "msaa4",
        AntiAliasing::Msaa8 => "msaa8",
    }
}

fn reconstruction_name(reconstruction: ReconstructionFilter) -> &'static str {
    match reconstruction {
        ReconstructionFilter::Box => "box",
        ReconstructionFilter::Tent => "tent",
        ReconstructionFilter::Gaussian => "gaussian",
    }
}
