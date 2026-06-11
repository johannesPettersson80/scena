use super::instancing::INSTANCE_BYTE_LEN;
use super::output;
use super::resource_encoding::encode_draw_resources;
use super::{GpuDeviceState, GpuPreparedResources};
use crate::render::RasterTarget;
use crate::render::prepare::{
    PreparedGpuLightUniform, PreparedInstanceSet, PreparedPrimitive, PreparedStrokeSegment,
};

impl GpuDeviceState {
    pub(in crate::render) fn update_dynamic_draw_state(
        &mut self,
        target: RasterTarget,
        light_uniform: PreparedGpuLightUniform,
        light_from_world: [f32; 16],
        draw_primitives: &[PreparedPrimitive],
        draw_instances: &[PreparedInstanceSet],
        draw_strokes: &[PreparedStrokeSegment],
    ) -> Result<(), &'static str> {
        let encoded = encode_draw_resources(draw_primitives, draw_instances, draw_strokes);
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
