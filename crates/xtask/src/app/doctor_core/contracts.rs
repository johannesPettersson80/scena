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
