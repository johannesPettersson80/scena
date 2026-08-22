#![cfg(target_arch = "wasm32")]

use crate::diagnostics::Backend;

use super::instancing::InstanceDrawBatch;
use super::stats::GpuResourceStats;
use super::vertices::PrimitiveDrawBatch;

pub(super) fn browser_trace(
    started_ms: f64,
    backend: Backend,
    stage: &'static str,
    detail: serde_json::Value,
) {
    #[cfg(feature = "browser-probe")]
    {
        let message = serde_json::json!({
            "schema": "scena.webgl2_prepare_trace.v1",
            "backend": format!("{backend:?}"),
            "stage": stage,
            "elapsed_ms": (js_sys::Date::now() - started_ms).round(),
            "detail": detail,
        })
        .to_string();
        web_sys::console::info_1(&wasm_bindgen::JsValue::from_str(&message));
    }
    #[cfg(not(feature = "browser-probe"))]
    let _ = (started_ms, backend, stage, detail);
}

pub(super) fn browser_rasterized_triangle_instances(
    draw_batches: &[PrimitiveDrawBatch],
    instance_batches: &[InstanceDrawBatch],
) -> u64 {
    draw_batches
        .iter()
        .map(|batch| u64::from(batch.vertex_count / 3))
        .chain(instance_batches.iter().map(|batch| {
            u64::from(batch.vertex_count / 3).saturating_mul(u64::from(batch.instance_count))
        }))
        .fold(0_u64, u64::saturating_add)
}

pub(super) fn browser_base_resource_stats(
    surface_pipeline_count: u64,
    triangle_shader_cache_hit: bool,
    vertex_buffer_size: u64,
    instance_buffer_size: u64,
    draw_uniform_capacity: usize,
) -> GpuResourceStats {
    GpuResourceStats {
        buffers: 4,
        pipelines: surface_pipeline_count,
        bind_groups: 1,
        shader_modules: 1,
        shader_module_creations: u64::from(!triangle_shader_cache_hit),
        approximate_gpu_memory_bytes: vertex_buffer_size
            .saturating_add(instance_buffer_size)
            .saturating_add(super::output::OUTPUT_UNIFORM_BYTE_LEN)
            .saturating_add(
                super::output::DRAW_UNIFORM_ENTRY_STRIDE
                    .saturating_mul((draw_uniform_capacity as u64).max(1)),
            ),
        ..GpuResourceStats::default()
    }
}
