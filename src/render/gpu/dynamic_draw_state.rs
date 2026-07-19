use super::instancing::INSTANCE_BYTE_LEN;
use super::output;
use super::resource_encoding::encode_draw_resources;
use super::{GpuDeviceState, GpuPreparedResources};
use crate::render::prepare::{
    PreparedGpuLightUniform, PreparedInstanceSet, PreparedPrimitive, PreparedStrokeSegment,
};
use crate::render::{PrepareWorkCounter, RasterTarget};

pub(in crate::render) struct DynamicDrawStateUpdate<'a> {
    pub(in crate::render) target: RasterTarget,
    pub(in crate::render) light_uniform: PreparedGpuLightUniform,
    pub(in crate::render) light_from_world: [f32; 16],
    pub(in crate::render) primitives: &'a [PreparedPrimitive],
    pub(in crate::render) instances: &'a [PreparedInstanceSet],
    pub(in crate::render) strokes: &'a [PreparedStrokeSegment],
    pub(in crate::render) semantic_aov_capture_enabled: bool,
    pub(in crate::render) label_quad_count: usize,
    pub(in crate::render) work: Option<&'a PrepareWorkCounter>,
}

impl GpuDeviceState {
    pub(in crate::render) fn update_dynamic_draw_state(
        &mut self,
        update: DynamicDrawStateUpdate<'_>,
    ) -> Result<(), &'static str> {
        let DynamicDrawStateUpdate {
            target,
            light_uniform,
            light_from_world,
            primitives: draw_primitives,
            instances: draw_instances,
            strokes: draw_strokes,
            semantic_aov_capture_enabled,
            label_quad_count,
            work,
        } = update;
        let semantic_attribution = semantic_aov_capture_enabled
            .then(|| {
                crate::render::semantic_aov::build_gpu_semantic_attribution(
                    draw_primitives,
                    draw_instances,
                    draw_strokes.len(),
                    label_quad_count,
                )
            })
            .transpose()
            .map_err(|_| "semantic AOV palette exhausted")?;
        let encoded = encode_draw_resources(
            draw_primitives,
            draw_instances,
            draw_strokes,
            semantic_attribution.as_ref(),
        );
        if let Some(work) = work {
            work.record_draw_uniform_indexing(
                encoded.draw_uniforms.len(),
                encoded.draw_uniform_index_metrics.lookup_probes,
                (encoded.draw_uniforms.len() as u64)
                    .saturating_mul(output::DRAW_UNIFORM_ENTRY_STRIDE),
            );
        }
        let Some(resources) = self.resources.as_mut() else {
            return Err("no GPU resources");
        };
        validate_dynamic_capacity(
            resources,
            target,
            &encoded.draw_uniforms,
            &encoded.instance_bytes,
        )?;
        self.queue.write_buffer(
            &resources.draw_uniform_buffer,
            0,
            &output::encode_draw_uniform_bytes(&encoded.draw_uniforms),
        );
        self.queue
            .write_buffer(&resources.instance_buffer, 0, &encoded.instance_bytes);
        resources.draw_uniforms = encoded.draw_uniforms;
        resources.draw_batches = encoded.draw_batches;
        resources.instance_batches = encoded.instance_batches;
        resources.instance_count = encoded.instance_count;
        resources.identity_instance = encoded.identity_instance;
        if let Some(strokes) = resources.strokes.as_mut() {
            strokes.batches = encoded.stroke_batches;
        }
        if let (Some(resources), Some(attribution)) =
            (resources.semantic_aov.as_mut(), semantic_attribution)
        {
            super::semantic_aov::update_attribution(resources, attribution);
        }
        resources.light_uniform = light_uniform;
        resources.light_from_world = light_from_world;
        Ok(())
    }
}

fn validate_dynamic_capacity(
    resources: &GpuPreparedResources,
    target: RasterTarget,
    draw_uniforms: &[super::vertices::DrawUniformValue],
    instance_bytes: &[u8],
) -> Result<(), &'static str> {
    if resources.target != target {
        return Err("target changed");
    }
    if draw_uniforms.len() > resources.draw_uniform_capacity {
        return Err("draw uniform capacity exceeded");
    }
    if instance_bytes.len() > resources.instance_buffer_capacity * INSTANCE_BYTE_LEN {
        return Err("instance buffer capacity exceeded");
    }
    Ok(())
}
