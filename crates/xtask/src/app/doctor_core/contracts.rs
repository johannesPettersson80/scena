use crate::app::prelude::*;

pub(crate) const REQUIRED_SOURCE_MODULES: &[&str] = &[
    "src/lib.rs",
    "src/scene.rs",
    "src/scene/camera.rs",
    "src/scene/connectors.rs",
    "src/scene/dirty.rs",
    "src/scene/inspection.rs",
    "src/scene/lights.rs",
    "src/scene/origin.rs",
    "src/scene/skinning.rs",
    "src/diagnostics/capabilities.rs",
    "src/assets.rs",
    "src/assets/environment.rs",
    "src/assets/load.rs",
    "src/assets/gltf/skins.rs",
    "src/assets/gltf/transform.rs",
    "src/assets/gltf/meshes.rs",
    "src/assets/gltf/materials.rs",
    "src/assets/gltf/textures.rs",
    "src/geometry.rs",
    "src/geometry/bounds.rs",
    "src/geometry/primitive.rs",
    "src/geometry/skinning.rs",
    "src/geometry/static_batch.rs",
    "src/material.rs",
    "src/render.rs",
    "src/viewer.rs",
    "src/render/build.rs",
    "src/render/camera.rs",
    "src/render/culling.rs",
    "src/render/surface.rs",
    "src/render/gpu/build.rs",
    "src/render/gpu/depth.rs",
    "src/render/gpu/draw.rs",
    "src/render/gpu/shadow.rs",
    "src/render/gpu/vertices.rs",
    "src/render/prepare/strokes.rs",
    "src/animation.rs",
    "src/animation/sampling.rs",
    "src/controls.rs",
    "src/picking.rs",
    "src/diagnostics.rs",
    "src/platform.rs",
    "src/bin/scena-convert.rs",
];

pub(crate) const STALE_DOC_TERMS: &[&str] = &[
    "TBD",
    "TODO",
    "FIXME",
    "not final API",
    "complete working example",
    "Renderer::prepare(&mut self, &mut scene)",
    "Renderer::render(&mut self, &scene",
    "RenderError::BackendCapabilityMismatch",
    "MutationQueueFull",
    "HardwareTier::Low / Mid",
    "rotation_quat",
    "gpu_memory_mb",
    "frame_time_ms",
    "render_on_change_skips",
    "texture_count",
    "Assets owns all GPU",
    "Load error unless feature enabled",
    "load error unless feature enabled",
    "Scene::replace_import(import, new_scene_asset)",
    "instantiate(scene_asset)",
    "instantiate_with(scene_asset",
    "Color::from_rgb(",
];

pub(crate) const SOURCE_SCOPE_TERMS: &[&str] = &[
    "plc",
    "robotics",
    "robot",
    "physics",
    "simulation",
    "process semantics",
    "game engine",
    "game loop",
];

pub(crate) const MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE: usize = 500;
pub(crate) const MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE: usize = 600;

pub(crate) const CATCH_ALL_TYPE_NAMES: &[&str] = &[
    "World",
    "Engine",
    "Manager",
    "Registry",
    "ServiceLocator",
    "Service",
    "Handler",
    "Provider",
    "Factory",
    "Helper",
    "Util",
    "Coordinator",
    "Orchestrator",
    "Bag",
];

pub(crate) const CATCH_ALL_TYPE_SUFFIXES: &[&str] = &[
    "Manager",
    "Engine",
    "Service",
    "Handler",
    "Provider",
    "Factory",
    "Helper",
    "Util",
    "Coordinator",
    "Orchestrator",
    "Bag",
];

pub(crate) const ALLOWED_CONTEXT_TYPES: &[&str] = &[
    "InteractionContext",
    "RenderContext",
    "PrepareContext",
    "DiagnosticContext",
];

pub(crate) fn check_cli_output_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "CLI-MACHINE-OUTPUT",
        "src/bin/scena/process_output_shared.rs",
        &[
            "write_stdout_line",
            "io::BufWriter",
            "scena.cli_io_error.v1",
            "serde_json::json!",
        ],
    );
    require_contains(
        root,
        findings,
        "CLI-MACHINE-OUTPUT",
        "src/bin/scena.rs",
        &["write_stdout_line", "io::ErrorKind::BrokenPipe"],
    );
    require_contains(
        root,
        findings,
        "CLI-MACHINE-OUTPUT",
        "src/bin/scena-convert.rs",
        &[
            "write_stdout_line",
            "io::ErrorKind::BrokenPipe",
            "serde_json::to_string",
        ],
    );
    for path in [
        "src/bin/scena.rs",
        "src/bin/scena-convert.rs",
        "src/bin/scena/process_output_shared.rs",
    ] {
        forbid_contains(
            root,
            findings,
            "CLI-MACHINE-OUTPUT",
            path,
            &["println!(", "print!("],
        );
    }
    forbid_contains(
        root,
        findings,
        "CLI-MACHINE-OUTPUT",
        "src/bin/scena-convert.rs",
        &["json_escape", ".replace('\\\\', \"\\\\\\\\\")"],
    );
    forbid_contains(
        root,
        findings,
        "CLI-MACHINE-OUTPUT",
        "src/bin/scena/process_output_shared.rs",
        &["\\\"schema\\\""],
    );

    check_cli_render_success_golden(root, findings);
    check_cli_inspect_success_golden(root, findings);
}

fn check_cli_render_success_golden(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "CLI-GOLDEN-RENDER-SUCCESS";
    const REL: &str = "tests/assets/cli-golden/render_introspection_stdout.json";
    let path = root.join(REL);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            RULE,
            format!("could not read {REL}; successful render output must be pinned"),
        ));
        return;
    };
    let Ok(report) = serde_json::from_str::<Value>(&text) else {
        findings.push(Finding::new(RULE, format!("{REL} must contain valid JSON")));
        return;
    };

    let ok = report.get("ok").and_then(Value::as_bool);
    let failed_material = report
        .pointer("/nodes_summary/failed_material")
        .and_then(Value::as_u64);
    if ok != Some(true) || failed_material != Some(0) {
        findings.push(Finding::new(
            RULE,
            format!(
                "{REL} is the successful render golden: ok must be true and \
                 nodes_summary.failed_material must be 0 (found ok={ok:?}, \
                 failed_material={failed_material:?})"
            ),
        ));
    }
}

fn check_cli_inspect_success_golden(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "CLI-GOLDEN-INSPECT-SUCCESS";
    const REL: &str = "tests/assets/cli-golden/inspect_asset_stdout.json";
    let path = root.join(REL);
    let Ok(text) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            RULE,
            format!("could not read {REL}; successful inspect output must be pinned"),
        ));
        return;
    };
    let Ok(report) = serde_json::from_str::<Value>(&text) else {
        findings.push(Finding::new(RULE, format!("{REL} must contain valid JSON")));
        return;
    };

    if inspect_report_contains_invalid_material(&report) {
        findings.push(Finding::new(
            RULE,
            format!(
                "{REL} must describe decoded textures without fallbacks; a successful \
                 inspect golden cannot normalize a missing material resource"
            ),
        ));
    }
}

fn inspect_report_contains_invalid_material(value: &Value) -> bool {
    match value {
        Value::Array(values) => values.iter().any(inspect_report_contains_invalid_material),
        Value::Object(object) => {
            let invalid_material = object.get("material").is_some_and(|material| {
                let fallbacks = material
                    .get("fallbacks")
                    .and_then(Value::as_array)
                    .is_none_or(|fallbacks| !fallbacks.is_empty());
                let invalid_texture = material
                    .get("textures")
                    .and_then(Value::as_array)
                    .is_none_or(|textures| {
                        textures.iter().any(|texture| {
                            texture.get("has_decoded_pixels").and_then(Value::as_bool) != Some(true)
                        })
                    });
                fallbacks || invalid_texture
            });
            invalid_material
                || object
                    .values()
                    .any(inspect_report_contains_invalid_material)
        }
        _ => false,
    }
}

pub(crate) fn require_files(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    paths: &[&str],
) {
    for rel in paths {
        if !root.join(rel).is_file() {
            findings.push(Finding::new(rule, format!("missing required file {rel}")));
        }
    }
}
