use crate::app::prelude::*;

use super::{PF06_RULE, PF09_RULE, check_pf10_source_contract, check_rule_source_contract};

pub(crate) fn check_pf06_spatial_acceleration_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required, forbidden) in [
        (
            "src/geometry.rs",
            &["struct TriangleBvhCache", "cached_triangle_bvh"][..],
            &[][..],
        ),
        (
            "src/geometry/spatial.rs",
            &[
                "struct TriangleBvh",
                "sort_by_key(|triangle| triangle.index)",
                "any_ray_candidate",
                "node_bounds_tests",
            ][..],
            &[][..],
        ),
        (
            "src/picking.rs",
            &[
                "inverse_transform_ray",
                "deformed_bvh_builds",
                "static_bvh_cache_hits",
                "ray_hits_bvh_bounds",
            ][..],
            &["for indices in geometry.indices().chunks_exact(3)"][..],
        ),
        (
            "src/render/prepare/shadows.rs",
            &[
                "struct ShadowOccluderSet",
                "TriangleBvh::from_triangles",
                "any_ray_candidate",
                "record_bvh_node_bounds_tests",
                "struct ShadowVisibilityKey",
                "world_position_bits",
                "light_state_signature",
                "occluder_state_signature",
            ][..],
            &["for occluder in occluders"][..],
        ),
        (
            "src/render/prepare.rs",
            &[
                "ShadowVisibilityCache::new(&lights, &shadow_occluders)",
                "shadow_visibility_cache: &shadow_visibility_cache",
            ][..],
            &[][..],
        ),
        (
            "tests/pf06_spatial_acceleration.rs",
            &[
                "pf06_static_geometry_reuses_bvh_and_reduces_triangle_tests_sublinearly",
                "pf06_deformed_geometry_rebuilds_from_current_pose_without_using_static_bvh",
                "pf06_prepare_scoped_shadow_cache_reuses_shared_deformed_world_positions",
            ][..],
            &[][..],
        ),
    ] {
        check_rule_source_contract(root, findings, PF06_RULE, relative, required, forbidden);
    }
}

pub(crate) fn check_pf09_parallel_work_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required, forbidden) in [
        (
            "src/render/parallel.rs",
            &[
                "RAYON_NUM_THREADS",
                "MAX_RENDER_WORKERS",
                "rayon::current_thread_index().is_some()",
                "pub(super) const fn worker_count",
            ][..],
            &[][..],
        ),
        (
            "src/render/prepare/environment_baker.rs",
            &[
                "bake_environment_ibl_profiled_with_workers",
                ".par_iter_mut()",
                ".par_chunks_mut(",
                "parallel_workers",
                "parallel_tasks",
                "pf09_parallel_environment_faces_and_rows_match_serial_bit_for_bit",
            ][..],
            &[][..],
        ),
        (
            "src/render/cpu.rs",
            &["primitive_screen_row_bounds"][..],
            &[][..],
        ),
        (
            "src/render/cpu_render.rs",
            &[
                "struct CpuRowBandBins",
                "projected_bounds",
                "primitive_indices: Some(&row_bands.bands[chunk_index])",
                "cpu_raster_candidate_triangles",
                "cpu_raster_full_rescan_triangles",
                "pf09_row_band_bins_reduce_candidate_scans_and_preserve_order",
            ][..],
            &[][..],
        ),
        (
            "tests/m9_platform_release.rs",
            &[
                "cold_bake_ms",
                "sidecar_hit_ms",
                "cpu_raster_candidate_triangles",
                "parallel_workers",
            ][..],
            &[][..],
        ),
    ] {
        check_rule_source_contract(root, findings, PF09_RULE, relative, required, forbidden);
    }
}

pub(crate) fn check_pf10_hot_path_contracts(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, required, forbidden) in [
        (
            "src/animation/sampling.rs",
            &["keyframe_segment", "sample_weights_into_profiled"][..],
            &["for index in 0..times.len().saturating_sub(1)"][..],
        ),
        (
            "src/render/prepare/types/geometry_storage.rs",
            &["world_to_model: Option<[f32; 16]>", "model_from_normal"][..],
            &[][..],
        ),
        (
            "src/render/gpu/vertices.rs",
            &[
                "struct DrawUniformInterner",
                "struct DrawUniformKey",
                "world_to_model()",
            ][..],
            &["draw_uniforms.iter().enumerate()", "invert_matrix4"][..],
        ),
        (
            "src/render/gpu/instancing.rs",
            &["struct InstanceRangeIndex", "instance_records_bitwise_eq"][..],
            &["encoded_ranges: Vec<"][..],
        ),
        (
            "src/scene/import/animation_bindings.rs",
            &["struct SourceNodeIndex", "offsets: HashMap<usize, usize>"][..],
            &[".find(|record| record.source_index"][..],
        ),
        (
            "src/assets/gltf/textures.rs",
            &[
                "canonical_data_uri_image",
                "embedded_image_path(&bytes, extension)",
            ][..],
            &["AssetPath::from(uri)"][..],
        ),
        (
            "src/geometry.rs",
            &[
                "struct OptionalVertexAttribute",
                "try_new_with_optional_vertex_attributes",
                "vertex_color_or_default",
                "tex_coord0_or_default",
            ][..],
            &[][..],
        ),
        (
            "src/assets/gltf/meshes.rs",
            &["Option<Vec<Color>>", "Option<Vec<[f32; 2]>>"][..],
            &[
                "vec![Color::WHITE; positions.len()]",
                "vec![[0.0, 0.0]; positions.len()]",
            ][..],
        ),
        (
            "src/render/prepare/primitives.rs",
            &["vertex_color_or_default", "tex_coord0_or_default"][..],
            &[][..],
        ),
        (
            "src/scene_host/wasm.rs",
            &[
                "pub fn render(&mut self) -> Result<String, JsValue>",
                "js_name = renderTyped",
                "fn render_outcome_js",
            ][..],
            &[][..],
        ),
        (
            "src/render/culling.rs",
            &[
                "CPU_OCCLUSION_MIN_PRIMITIVES",
                "OCCLUSION_OVERLAP_TILE_DIMENSION",
                "has_projected_tile_overlap",
                "!gpu_active",
            ][..],
            &["_gpu_active"][..],
        ),
        (
            "src/render/settings.rs",
            &[
                "pub const fn cpu_occlusion_culling",
                "pub fn set_cpu_occlusion_culling",
            ][..],
            &[][..],
        ),
        (
            "tests/pf10_cpu_occlusion.rs",
            &[
                "SCENA_RUN_PF10_OCCLUSION_BENCHMARK",
                "sample_scene_pair",
                "cpu-occlusion-prepass-benefit",
            ][..],
            &[][..],
        ),
        (
            "tests/browser/scene_host_browser_proof.js",
            &[
                "[\"prototype\", \"renderTyped\"]",
                "render_typed_returns_native_object_matching_json_compatibility_result",
            ][..],
            &[][..],
        ),
        (
            "src/render.rs",
            &[
                "cpu_material_reflection_scratch",
                "cpu_effect_rgba8_scratch",
                "gpu_supersample_frame",
            ][..],
            &["let mut supersample_frame = Vec::new()"][..],
        ),
        (
            "src/render/cpu_render.rs",
            &["resize_reusable_scratch", "rgba8_scratch"][..],
            &["let mut scratch = vec![0;", "cpu_frame.frame.to_vec()"][..],
        ),
    ] {
        check_pf10_source_contract(root, findings, relative, required, forbidden);
    }
}
