use super::instancing::INSTANCE_BYTE_LEN;
use super::output;
use super::resource_encoding::{EncodedDrawResources, encode_draw_resources};
use super::vertices::{VERTEX_BYTE_LEN, encode_vertices_iter};
use super::{GpuDeviceState, GpuPreparedResources};
use crate::render::prepare::{
    PreparedDepthStats, PreparedGpuLightUniform, PreparedInstanceSet, PreparedPrimitive,
    PreparedStrokeSegment,
};
use crate::render::{PrepareWorkCounter, RasterTarget};

pub(in crate::render) struct DynamicDrawStateUpdate<'a> {
    pub(in crate::render) target: RasterTarget,
    pub(in crate::render) light_uniform: PreparedGpuLightUniform,
    pub(in crate::render) light_from_world: [f32; 16],
    pub(in crate::render) primitives: &'a [PreparedPrimitive],
    /// Blended triangles whose vertex-buffer slot holds another triangle
    /// after a re-sort. They occupy consecutive slots, in this order.
    pub(in crate::render) blend_vertex_rewrite: &'a [PreparedPrimitive],
    pub(in crate::render) instances: &'a [PreparedInstanceSet],
    pub(in crate::render) strokes: &'a [PreparedStrokeSegment],
    /// Depth-prepass requirements of `primitives` and `instances`.
    pub(in crate::render) depth_stats: PreparedDepthStats,
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
            blend_vertex_rewrite,
            instances: draw_instances,
            strokes: draw_strokes,
            depth_stats,
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
        if depth_prepass_needed(&encoded, depth_stats, label_quad_count)
            && resources.depth_prepass.is_none()
        {
            // The colour pass only depth-tests against a prepass. Drawing a
            // list that needs one without it would paint in submission order.
            return Err("draw list needs a depth prepass the last full prepare did not build");
        }
        let blend_vertices = blend_vertex_write(resources, blend_vertex_rewrite)?;
        if let Some((offset, bytes)) = &blend_vertices {
            self.queue
                .write_buffer(&resources.vertex_buffer, *offset, bytes);
        }
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

/// Whether a full prepare of this draw list would build a depth prepass.
#[cfg(not(target_arch = "wasm32"))]
fn depth_prepass_needed(
    _encoded: &EncodedDrawResources,
    depth_stats: PreparedDepthStats,
    _label_quad_count: usize,
) -> bool {
    depth_stats.passes > 0
}

/// Whether a full prepare of this draw list would build a depth prepass. The
/// browser path skips it for a single triangle without depth-tested overlays.
#[cfg(target_arch = "wasm32")]
fn depth_prepass_needed(
    encoded: &EncodedDrawResources,
    depth_stats: PreparedDepthStats,
    label_quad_count: usize,
) -> bool {
    super::prepare_resources_support::browser_depth_prepass_required(
        depth_stats.passes > 0,
        false,
        super::prepare_resources_wasm_support::browser_rasterized_triangle_instances(encoded),
        !encoded.stroke_batches.is_empty() || label_quad_count > 0,
    )
}

/// Encodes the re-sorted blended triangles for the slots they now occupy.
///
/// Returns the byte offset into the vertex buffer and the bytes to write, or
/// `None` when every triangle already sits in its slot.
fn blend_vertex_write(
    resources: &GpuPreparedResources,
    rewrite: &[PreparedPrimitive],
) -> Result<Option<(u64, Vec<u8>)>, &'static str> {
    let Some(first) = rewrite.first() else {
        return Ok(None);
    };
    let offset = u64::from(first.original_vertex_offset()) * VERTEX_BYTE_LEN as u64;
    let (bytes, _metrics) = encode_vertices_iter(rewrite.iter(), rewrite.len());
    if offset.saturating_add(bytes.len() as u64) > resources.vertex_buffer.size() {
        return Err("re-sorted blended triangles do not fit the prepared vertex buffer");
    }
    Ok(Some((offset, bytes)))
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
