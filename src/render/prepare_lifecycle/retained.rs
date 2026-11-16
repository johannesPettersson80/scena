//! The retained prepare path: bring prepared state up to date for a changed
//! scene without baking the scene again.
//!
//! A full prepare bakes every visible source into a template (`retained_*` in
//! [`PreparedSceneState`](crate::render::state::PreparedSceneState)) and uploads
//! it. As long as the template still describes the scene, a later prepare only
//! has to re-derive what the change touched: per-draw matrices and tints, which
//! sources are hidden, and which triangles the camera culls.

use std::sync::Arc;

use crate::assets::{Assets, MaterialHandle};
use crate::scene::Scene;

use super::super::prepare_retained::prepared_instance_count;
use super::super::{PrepareWorkCounter, RasterTarget, Renderer, camera, culling, gpu, prepare};

/// What the retained path needs from the preamble every prepare runs.
pub(super) struct RetainedPrepareInputs<'a> {
    pub(super) target: RasterTarget,
    pub(super) camera: Option<&'a camera::CameraProjection>,
    pub(super) light_uniform: prepare::PreparedGpuLightUniform,
    pub(super) material_handles: &'a [MaterialHandle],
    pub(super) work: Option<&'a PrepareWorkCounter>,
}

impl Renderer {
    /// Updates the prepared state from the retained template.
    ///
    /// On `Err` nothing was changed and the caller has to run a full prepare;
    /// the message says why the template could not be reused.
    pub(super) fn prepare_retained<F>(
        &mut self,
        scene: &Scene,
        assets: Option<&Assets<F>>,
        inputs: RetainedPrepareInputs<'_>,
    ) -> Result<(), &'static str> {
        if self.gpu.is_none() {
            return Err("the CPU renderer bakes every prepare");
        }
        if let Some(reason) =
            self.dynamic_gpu_prepare_rejection_reason(scene, inputs.material_handles)
        {
            return Err(reason);
        }
        let (primitives, strokes, instances) = self
            .reencode_retained_draws(scene)
            .ok_or("visible source missing from retained template")?;
        let culled = culling::cull_prepared_primitives(
            primitives,
            inputs.camera,
            inputs.target,
            self.cpu_occlusion_culling
                && scene.clipping_planes().planes().is_empty()
                && scene.section_box().is_none(),
            true,
        );
        let primitives = culled.visible;
        let light_from_world = prepare::collect_dynamic_light_from_world(scene, assets)
            .map_err(|_error| "dynamic shadow projection failed")?;
        let depth_stats = self.depth_prepass_stats(
            primitives
                .iter()
                .chain(instances.iter().flat_map(|set| set.primitives().iter())),
            inputs.target.backend,
        );
        let label_quad_count = self
            .prepared
            .as_ref()
            .map_or(0, |prepared| prepared.labels.quads().len());
        let semantic_aov_capture_enabled = self.semantic_aov_capture_enabled;
        let gpu = self.gpu.as_mut().ok_or("no GPU device")?;
        gpu.update_dynamic_draw_state(gpu::DynamicDrawStateUpdate {
            target: inputs.target,
            light_uniform: inputs.light_uniform,
            light_from_world,
            primitives: &primitives,
            instances: &instances,
            strokes: &strokes,
            depth_stats,
            semantic_aov_capture_enabled,
            label_quad_count,
            work: inputs.work,
        })?;

        let prepared = self.prepared.as_mut().ok_or("no prepared template")?;
        prepared.transform_revision = scene.transform_revision();
        prepared.camera_revision = scene.camera_revision();
        prepared.appearance_revision = scene.appearance_revision();
        prepared.visibility_revision = scene.visibility_revision();
        prepared.primitives = Arc::from(primitives);
        prepared.strokes = Arc::from(strokes);
        prepared.instances = Arc::from(instances);
        self.stats.instances = prepared_instance_count(prepared);
        self.stats.culled_objects = culled.culled;
        self.prepare_telemetry.dynamic_template_prepares = self
            .prepare_telemetry
            .dynamic_template_prepares
            .saturating_add(1);
        self.prepare_telemetry.draw_uniform_only_updates = self
            .prepare_telemetry
            .draw_uniform_only_updates
            .saturating_add(1);
        Ok(())
    }
}
