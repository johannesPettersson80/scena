use serde_json::json;

pub(super) fn capabilities_json(capabilities: crate::Capabilities) -> serde_json::Value {
    json!({
        "backend": format!("{:?}", capabilities.backend),
        "color_target_format": capabilities.color_target_format,
        "gpu_device": capabilities.gpu_device,
        "surface_attached": capabilities.surface_attached,
        "hardware_tier": format!("{:?}", capabilities.hardware_tier),
        "output_stage": format!("{:?}", capabilities.output_stage),
        "alpha_pipeline": format!("{:?}", capabilities.alpha_pipeline),
        "forward_pbr": format!("{:?}", capabilities.forward_pbr),
        "directional_shadow_map_default_size": capabilities.directional_shadow_map_default_size,
        "directional_shadow_map_max_size": capabilities.directional_shadow_map_max_size,
        "directional_shadow_pcf_kernel": capabilities.directional_shadow_pcf_kernel,
        "ibl_cubemap_default_size": capabilities.ibl_cubemap_default_size,
        "ibl_brdf_lut_default_size": capabilities.ibl_brdf_lut_default_size,
        "default_clipping_planes": capabilities.default_clipping_planes,
        "max_clipping_planes": capabilities.max_clipping_planes,
        "gpu_frustum_culling": format!("{:?}", capabilities.gpu_frustum_culling),
        "per_instance_culling": format!("{:?}", capabilities.per_instance_culling),
        "compute_shaders": format!("{:?}", capabilities.compute_shaders),
        "storage_buffers": format!("{:?}", capabilities.storage_buffers),
        "readback_headless_screenshots": format!("{:?}", capabilities.readback_headless_screenshots),
        "reversed_z_depth": format!("{:?}", capabilities.reversed_z_depth),
    })
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

pub(super) fn stats_json(stats: crate::RendererStats) -> serde_json::Value {
    json!({
        "buffers": stats.buffers,
        "textures": stats.textures,
        "materials": stats.materials,
        "render_targets": stats.render_targets,
        "pipelines": stats.pipelines,
        "bind_groups": stats.bind_groups,
        "shader_modules": stats.shader_modules,
        "environments": stats.environments,
        "scene_imports": stats.scene_imports,
        "shadow_maps": stats.shadow_maps,
        "depth_prepass_passes": stats.depth_prepass_passes,
        "depth_prepass_draws": stats.depth_prepass_draws,
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
