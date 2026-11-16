//! The retained prepare path: bring prepared state up to date for a changed
//! scene without baking the scene again.
//!
//! A full prepare bakes every visible source into a template (`retained_*` in
//! [`PreparedSceneState`](crate::render::state::PreparedSceneState)) and, on a
//! GPU backend, uploads it. While the template still describes the scene
//! ([`Renderer::full_prepare_reason`] finds no objection), a later prepare only
//! re-derives what a change can touch:
//!
//! - per-draw matrices and tints, from the nodes as they are now;
//! - which sources are hidden;
//! - which triangles the active camera culls;
//! - the back-to-front order of alpha-blended triangles for that camera.
//!
//! The result is what a full prepare of the same scene and camera produces.
//! It is exactly that when no draw moved. A draw that moved keeps the
//! model-space vertices recovered from its first bake, which a full prepare
//! would recover again through the new matrix, so its pixels can differ in the
//! last bit.

use std::sync::Arc;

use crate::assets::Assets;
use crate::scene::Scene;

use super::super::prepare_retained::{BlendSlotAssignment, PreparedView, prepared_instance_count};
use super::super::{PrepareWorkCounter, RasterTarget, Renderer, camera, culling, gpu, prepare};
use super::reasons::FullPrepareReason;

/// What the retained path needs from the preamble every prepare runs.
pub(super) struct RetainedPrepareInputs<'a> {
    pub(super) target: RasterTarget,
    pub(super) camera: Option<&'a camera::CameraProjection>,
    pub(super) light_uniform: prepare::PreparedGpuLightUniform,
    pub(super) work: Option<&'a PrepareWorkCounter>,
}

/// The culled, ordered draw lists a retained prepare hands to the GPU.
struct GpuDraws<'a> {
    primitives: &'a mut [prepare::PreparedPrimitive],
    instances: &'a [prepare::PreparedInstanceSet],
    strokes: &'a [prepare::PreparedStrokeSegment],
    /// Depth-prepass requirements of `primitives` and `instances`.
    depth_stats: prepare::PreparedDepthStats,
}

/// The retained template as the scene wants it drawn now, before culling.
pub(super) struct RetainedDraws {
    pub(super) primitives: Vec<prepare::PreparedPrimitive>,
    pub(super) strokes: Vec<prepare::PreparedStrokeSegment>,
    pub(super) instances: Vec<prepare::PreparedInstanceSet>,
    pub(super) labels: prepare::PreparedLabelAtlas,
}

impl Renderer {
    /// Updates the prepared state from the retained template.
    ///
    /// On `Err` nothing was changed and the caller has to run a full prepare.
    pub(super) fn prepare_retained<F>(
        &mut self,
        scene: &Scene,
        assets: Option<&Assets<F>>,
        inputs: RetainedPrepareInputs<'_>,
    ) -> Result<(), FullPrepareReason> {
        use FullPrepareReason as Reason;
        if let Some(reason) = self.full_prepare_reason(scene, assets, inputs.camera) {
            return Err(reason);
        }
        let RetainedDraws {
            primitives,
            strokes,
            instances,
            labels,
        } = self
            .reencode_retained_draws(scene)
            .ok_or(Reason::TemplateCoverage)?;
        let on_gpu = self.gpu.is_some();
        let baked_labels = &self.prepared.as_ref().ok_or(Reason::NoTemplate)?.labels;
        if labels.quads() != baked_labels.quads() {
            // The label atlas is packed, and on a GPU uploaded, for the
            // labels that were visible at the last full prepare.
            return Err(Reason::Labels);
        }
        let culled = culling::cull_prepared_primitives(
            primitives,
            inputs.camera,
            inputs.target,
            self.cpu_occlusion_culling
                && scene.clipping_planes().planes().is_empty()
                && scene.section_box().is_none(),
            on_gpu,
        );
        let mut primitives = culled.visible;
        // Blended triangles are drawn farthest first. Re-ordering them for the
        // camera of this frame is what lets a camera move keep the template.
        prepare::resort_blended_primitives(&mut primitives, inputs.camera);
        let depth_stats = self.depth_prepass_stats(
            primitives
                .iter()
                .chain(instances.iter().flat_map(|set| set.primitives().iter())),
            inputs.target.backend,
        );
        let blend_slots = if on_gpu {
            let draws = GpuDraws {
                primitives: &mut primitives,
                instances: &instances,
                strokes: &strokes,
                depth_stats,
            };
            Some(self.update_gpu_draws(scene, assets, &inputs, draws)?)
        } else {
            None
        };

        let prepared = self.prepared.as_mut().ok_or(Reason::NoTemplate)?;
        if let Some(blend_slots) = &blend_slots {
            prepared.blend_vertex_slots.record_upload(blend_slots);
        }
        prepared.transform_revision = scene.transform_revision();
        prepared.camera_revision = scene.camera_revision();
        prepared.appearance_revision = scene.appearance_revision();
        prepared.visibility_revision = scene.visibility_revision();
        prepared.view = PreparedView::of(scene);
        prepared.primitives = Arc::from(primitives);
        prepared.strokes = Arc::from(strokes);
        prepared.instances = Arc::from(instances);
        prepared.labels = Arc::new(labels);
        self.stats.instances = prepared_instance_count(prepared);
        self.stats.culled_objects = culled.culled;
        self.stats.depth_prepass_passes = depth_stats.passes;
        self.stats.depth_prepass_draws = depth_stats.draws;
        self.prepare_telemetry.dynamic_template_prepares = self
            .prepare_telemetry
            .dynamic_template_prepares
            .saturating_add(1);
        if on_gpu {
            self.prepare_telemetry.draw_uniform_only_updates = self
                .prepare_telemetry
                .draw_uniform_only_updates
                .saturating_add(1);
        }
        Ok(())
    }

    /// Hands the re-derived draw lists to the GPU resources of the last full
    /// prepare. Blended triangles get the vertex-buffer slots of their new
    /// order; the returned assignment is recorded once the upload is known to
    /// have been accepted.
    fn update_gpu_draws<F>(
        &mut self,
        scene: &Scene,
        assets: Option<&Assets<F>>,
        inputs: &RetainedPrepareInputs<'_>,
        draws: GpuDraws<'_>,
    ) -> Result<BlendSlotAssignment, FullPrepareReason> {
        use FullPrepareReason as Reason;
        let GpuDraws {
            primitives,
            instances,
            strokes,
            depth_stats,
        } = draws;
        let prepared = self.prepared.as_ref().ok_or(Reason::NoTemplate)?;
        let blend_slots = prepared.blend_vertex_slots.assign(primitives);
        // The prepared list keeps the CPU-only stroke quads, as a full
        // prepare's does; the GPU is handed the triangles it draws.
        let gpu_primitives = super::gpu_triangle_primitives(primitives, inputs.work);
        let label_quad_count = prepared.labels.quads().len();
        let light_from_world = prepare::collect_dynamic_light_from_world(scene, assets)
            .map_err(|_error| Reason::ShadowProjection)?;
        let semantic_aov_capture_enabled = self.semantic_aov_capture_enabled;
        let gpu = self.gpu.as_mut().ok_or(Reason::Gpu("no GPU device"))?;
        gpu.update_dynamic_draw_state(gpu::DynamicDrawStateUpdate {
            target: inputs.target,
            light_uniform: inputs.light_uniform,
            light_from_world,
            primitives: &gpu_primitives,
            blend_vertex_rewrite: blend_slots.stale(&gpu_primitives),
            instances,
            strokes,
            depth_stats,
            semantic_aov_capture_enabled,
            label_quad_count,
            work: inputs.work,
        })
        .map_err(Reason::Gpu)?;
        Ok(blend_slots)
    }
}
