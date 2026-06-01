use serde_json::json;

use crate::{Diagnostic, RendererStats};

pub(super) fn diagnostics_json(diagnostics: &[Diagnostic]) -> serde_json::Value {
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

pub(super) fn stats_json(stats: RendererStats) -> serde_json::Value {
    let mut object = serde_json::Map::new();
    object.insert("buffers".to_string(), json!(stats.buffers));
    object.insert("textures".to_string(), json!(stats.textures));
    object.insert("materials".to_string(), json!(stats.materials));
    object.insert(
        "material_bindings".to_string(),
        json!(stats.material_bindings),
    );
    object.insert(
        "material_texture_bindings".to_string(),
        json!(stats.material_texture_bindings),
    );
    object.insert(
        "material_sampler_bindings".to_string(),
        json!(stats.material_sampler_bindings),
    );
    object.insert(
        "material_textures_missing_decoded_pixels".to_string(),
        json!(stats.material_textures_missing_decoded_pixels),
    );
    object.insert(
        "material_batch_layers".to_string(),
        json!(stats.material_batch_layers),
    );
    object.insert(
        "material_bind_groups".to_string(),
        json!(stats.material_bind_groups),
    );
    object.insert("render_targets".to_string(), json!(stats.render_targets));
    object.insert("pipelines".to_string(), json!(stats.pipelines));
    object.insert("bind_groups".to_string(), json!(stats.bind_groups));
    object.insert("shader_modules".to_string(), json!(stats.shader_modules));
    object.insert("environments".to_string(), json!(stats.environments));
    object.insert(
        "environment_cubemaps".to_string(),
        json!(stats.environment_cubemaps),
    );
    object.insert(
        "environment_prefilter_passes".to_string(),
        json!(stats.environment_prefilter_passes),
    );
    object.insert(
        "environment_brdf_luts".to_string(),
        json!(stats.environment_brdf_luts),
    );
    object.insert("scene_imports".to_string(), json!(stats.scene_imports));
    object.insert("shadow_maps".to_string(), json!(stats.shadow_maps));
    object.insert(
        "depth_prepass_passes".to_string(),
        json!(stats.depth_prepass_passes),
    );
    object.insert(
        "depth_prepass_draws".to_string(),
        json!(stats.depth_prepass_draws),
    );
    object.insert(
        "ambient_occlusion_passes".to_string(),
        json!(stats.ambient_occlusion_passes),
    );
    object.insert(
        "order_independent_transparency_passes".to_string(),
        json!(stats.order_independent_transparency_passes),
    );
    object.insert("bloom_passes".to_string(), json!(stats.bloom_passes));
    object.insert("fxaa_passes".to_string(), json!(stats.fxaa_passes));
    object.insert(
        "live_logical_handles".to_string(),
        json!(stats.live_logical_handles),
    );
    object.insert(
        "pending_destructions".to_string(),
        json!(stats.pending_destructions),
    );
    object.insert("frames_rendered".to_string(), json!(stats.frames_rendered));
    object.insert("draw_calls".to_string(), json!(stats.draw_calls));
    object.insert("triangles".to_string(), json!(stats.triangles));
    object.insert("culled_objects".to_string(), json!(stats.culled_objects));
    object.insert(
        "gpu_culling_dispatches".to_string(),
        json!(stats.gpu_culling_dispatches),
    );
    object.insert("skipped_frames".to_string(), json!(stats.skipped_frames));
    object.insert("gpu_submissions".to_string(), json!(stats.gpu_submissions));
    object.insert(
        "approximate_gpu_memory_bytes".to_string(),
        json!(stats.approximate_gpu_memory_bytes),
    );
    object.insert("cpu_frame_ms".to_string(), json!(stats.cpu_frame_ms));
    object.insert("gpu_frame_ms".to_string(), json!(stats.gpu_frame_ms));
    object.insert("primitives".to_string(), json!(stats.primitives));
    object.insert("target_width".to_string(), json!(stats.target_width));
    object.insert("target_height".to_string(), json!(stats.target_height));
    object.insert(
        "directional_shadow_map_resolution".to_string(),
        json!(stats.directional_shadow_map_resolution),
    );
    object.insert(
        "directional_shadow_pcf_kernel".to_string(),
        json!(stats.directional_shadow_pcf_kernel),
    );
    serde_json::Value::Object(object)
}
