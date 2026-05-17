use crate::assets::Assets;
use crate::diagnostics::{Diagnostic, DiagnosticCode, RendererStats};
use crate::scene::Scene;

use super::{Renderer, prepare};

impl Renderer {
    pub fn diagnose_scene(&self, scene: &Scene) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        if scene.active_camera().is_none() {
            diagnostics.push(Diagnostic::error(
                DiagnosticCode::MissingActiveCamera,
                "scene has no active camera",
                "call Scene::add_default_camera or Scene::set_active_camera before rendering",
            ));
        }
        diagnostics.extend(prepare::collect_camera_projection_diagnostics(scene));
        diagnostics.extend(prepare::collect_camera_visibility_diagnostics(
            scene,
            self.target,
        ));

        if scene.visible_drawable_count() == 0 {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::InvisibleScene,
                "scene has no visible drawables for the active camera",
                "check node visibility, parent visibility, camera layer masks, or add a mesh/renderable node",
            ));
        }

        if scene.light_nodes().count() == 0 && self.environment.is_none() {
            diagnostics.push(Diagnostic::warning(
                DiagnosticCode::MissingLightingOrEnvironment,
                "scene has no active light nodes and no renderer environment",
                "call renderer.set_environment for image-based lighting or add a scene light for lit materials",
            ));
        }

        diagnostics
    }

    pub fn diagnose_scene_with_assets<F>(
        &self,
        scene: &Scene,
        assets: &Assets<F>,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = self.diagnose_scene(scene);
        diagnostics.extend(prepare::collect_asset_camera_visibility_diagnostics(
            scene,
            self.target,
            assets,
        ));
        diagnostics.extend(prepare::collect_material_texture_diagnostics(scene, assets));
        diagnostics
    }

    pub fn stats(&self) -> RendererStats {
        self.stats
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }
}
