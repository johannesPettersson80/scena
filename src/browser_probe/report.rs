use serde_json::json;

pub(super) fn capabilities_json(capabilities: crate::Capabilities) -> serde_json::Value {
    crate::CapabilityReport::new(capabilities, None).to_schema_json()["capabilities"].clone()
}

pub(super) fn diagnostics_json(diagnostics: &[crate::Diagnostic]) -> serde_json::Value {
    serde_json::Value::Array(
        diagnostics
            .iter()
            .map(|diagnostic| {
                json!({
                    "code": format!("{:?}", diagnostic.code),
                    "severity": format!("{:?}", diagnostic.severity),
                    "message": diagnostic.message,
                    "help": diagnostic.help,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Backend, Capabilities};

    #[test]
    fn capabilities_json_reports_supported_gpu_material_statuses() {
        let value = capabilities_json(Capabilities::for_attached_gpu_backend(Backend::WebGl2));

        assert_eq!(value["forward_pbr"], "supported");
        assert_eq!(value["physical_glass_transmission"], "supported");
    }
}

pub(super) fn stats_json(stats: crate::RendererStats) -> serde_json::Value {
    json!({
        "buffers": stats.buffers,
        "textures": stats.textures,
        "materials": stats.materials,
        "material_bindings": stats.material_bindings,
        "material_texture_bindings": stats.material_texture_bindings,
        "material_sampler_bindings": stats.material_sampler_bindings,
        "material_textures_missing_decoded_pixels": stats.material_textures_missing_decoded_pixels,
        "render_targets": stats.render_targets,
        "pipelines": stats.pipelines,
        "bind_groups": stats.bind_groups,
        "shader_modules": stats.shader_modules,
        "environments": stats.environments,
        "scene_imports": stats.scene_imports,
        "shadow_maps": stats.shadow_maps,
        "depth_prepass_passes": stats.depth_prepass_passes,
        "depth_prepass_draws": stats.depth_prepass_draws,
        "ambient_occlusion_passes": stats.ambient_occlusion_passes,
        "order_independent_transparency_passes": stats.order_independent_transparency_passes,
        "bloom_passes": stats.bloom_passes,
        "fxaa_passes": stats.fxaa_passes,
        "live_logical_handles": stats.live_logical_handles,
        "pending_destructions": stats.pending_destructions,
        "frames_rendered": stats.frames_rendered,
        "draw_calls": stats.draw_calls,
        "triangles": stats.triangles,
        "culled_objects": stats.culled_objects,
        "gpu_culling_dispatches": stats.gpu_culling_dispatches,
        "skipped_frames": stats.skipped_frames,
        "gpu_submissions": stats.gpu_submissions,
        "approximate_gpu_memory_bytes": stats.approximate_gpu_memory_bytes,
        "cpu_frame_ms": stats.cpu_frame_ms,
        "gpu_frame_ms": stats.gpu_frame_ms,
        "primitives": stats.primitives,
        "target_width": stats.target_width,
        "target_height": stats.target_height,
        "directional_shadow_map_resolution": stats.directional_shadow_map_resolution,
        "directional_shadow_pcf_kernel": stats.directional_shadow_pcf_kernel,
    })
}
