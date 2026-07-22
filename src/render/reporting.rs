use crate::assets::Assets;
use crate::diagnostics::{
    CapabilityProbeV1, CapabilityReport, Diagnostic, DiagnosticCode, PostProcessingDepthSourceV1,
    PostProcessingPassV1, PostProcessingReportV1, RendererStats,
};
use crate::scene::Scene;

use super::{Renderer, prepare};

impl Renderer {
    pub fn capability_report(&self) -> CapabilityReport {
        CapabilityReport::new_with_post_processing(
            self.capabilities,
            self.gpu_adapter_report(),
            self.post_processing_report(),
        )
    }

    /// Returns live adapter/device/format provenance for GPU-backed renderers.
    /// CPU and descriptor-only renderers return `None` because no adapter was
    /// requested and static capability tables must not be presented as probes.
    pub fn live_capability_probe(&self, probed_at_unix_ms: u64) -> Option<CapabilityProbeV1> {
        self.gpu
            .as_ref()
            .map(|gpu| gpu.live_capability_probe(self.target.backend, probed_at_unix_ms))
    }

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

    fn post_processing_report(&self) -> PostProcessingReportV1 {
        let anti_aliasing = self.anti_aliasing.uses_post_fxaa();
        let bloom = self.bloom.is_some();
        let screen_space_reflections = self.screen_space_reflections.is_some();
        let screen_space_ambient_occlusion = self.screen_space_ambient_occlusion.is_some();
        let depth_of_field = self.depth_of_field.is_some();
        let mut active_passes = Vec::new();
        if screen_space_reflections {
            active_passes.push(PostProcessingPassV1::ScreenSpaceReflections);
        }
        if screen_space_ambient_occlusion {
            active_passes.push(PostProcessingPassV1::ScreenSpaceAmbientOcclusion);
        }
        if depth_of_field {
            active_passes.push(PostProcessingPassV1::DepthOfField);
        }
        if bloom {
            active_passes.push(PostProcessingPassV1::Bloom);
        }
        if anti_aliasing {
            active_passes.push(PostProcessingPassV1::Fxaa);
        }
        PostProcessingReportV1 {
            active_passes,
            anti_aliasing,
            bloom,
            screen_space_reflections,
            screen_space_ambient_occlusion,
            depth_of_field,
            ssao_depth_source: screen_space_ambient_occlusion.then(|| {
                if self.gpu.is_some() {
                    PostProcessingDepthSourceV1::DepthColorTarget
                } else {
                    PostProcessingDepthSourceV1::CpuDepthFrame
                }
            }),
            dof_depth_source: depth_of_field.then(|| {
                if self.gpu.is_some() {
                    PostProcessingDepthSourceV1::DepthColorTarget
                } else {
                    PostProcessingDepthSourceV1::CpuDepthFrame
                }
            }),
        }
    }
}
