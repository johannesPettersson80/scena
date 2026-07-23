use crate::app::prelude::*;

use super::check_hot_path_source_contract;

pub(crate) fn check_pf03_pf05_hot_path_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/cpu_render.rs",
        &[".take()", "draw_cpu_from_prepared"],
        &[
            "prepared.primitives.clone()",
            "prepared.strokes.clone()",
            "prepared.labels.clone()",
        ],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/state.rs",
        &[
            "retained_primitives: Arc<[prepare::PreparedPrimitive]>",
            "primitives: Arc<[prepare::PreparedPrimitive]>",
        ],
        &["retained_primitives: Vec<prepare::PreparedPrimitive>"],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/prepare/types/geometry_storage.rs",
        &[
            "struct PreparedDrawTransform",
            "fn share_model_space_vertex_buffer",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/prepare/types.rs",
        &[
            "draw_transform: Arc<PreparedDrawTransform>",
            "model_vertices: Option<Arc<[PreparedModelVertex]>>",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/prepare_lifecycle.rs",
        &[
            "Cow::Borrowed(primitives)",
            "Arc<[prepare::PreparedPrimitive]>",
            "share_model_space_vertex_buffer",
            "collect_depth_prepass_stats_iter",
            "retained_primitives",
            "culling::cull_prepared_primitives",
        ],
        &["let mut depth_primitives = primitives.clone()"],
    );
    require_contains(
        root,
        findings,
        "PF03-PF05-HOT-PATH-CONTRACTS",
        "src/render/phase5_tests.rs",
        &["off_frustum_source_stays_in_retained_template_across_camera_motion"],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/gpu/resource_encoding.rs",
        &[
            "let retained_instance_primitives = retained_instances",
            ".chain(retained_instance_primitives)",
        ],
        &["all_retained_primitives"],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/geometry.rs",
        &[],
        &[
            "world_from_model: [f32; 16]",
            "normal_from_model: [f32; 16]",
        ],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/assets.rs",
        &[
            "SlotMap<GeometryHandle, Arc<GeometryDesc>>",
            "SlotMap<MaterialHandle, Arc<MaterialDesc>>",
            "SlotMap<TextureHandle, Arc<TextureDesc>>",
            "pub fn geometry_snapshot",
            "pub fn material_snapshot",
            "pub fn texture_snapshot",
            "pub(crate) fn texture_snapshots",
            "pf04_snapshot_cache_replacement_preserves_old_view_and_exposes_fresh_content",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/prepare/materials.rs",
        &[
            "struct PreparedMaterialTextures",
            "assets.texture_snapshots(handles)",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/render/prepare/primitives.rs",
        &["source.textures"],
        &["source.assets"],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/material/color.rs",
        &[
            "OnceLock<[f32; 256]>",
            "srgb_u8_to_linear",
            "for channel in u8::MIN..=u8::MAX",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/scene/resolved_cache.rs",
        &[
            "struct ResolvedSceneCache",
            "traversal_stack",
            "structure_revision",
            "transform_revision",
            "visibility_revision",
            "active_camera",
            "camera_layer_mask",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "src/scene/transforms.rs",
        &["self.resolved_node_state(node)"],
        &["let mut chain = Vec::new()"],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "tests/pf03_pf05_hot_path_contracts.rs",
        &[
            "prepared_primitive_list_clones",
            "prepared_geometry_shares_model_vertices_and_draw_transforms",
            "prepared_model_vertex_buffer_count",
            "prepared_list_copy_bytes",
            "actual_delta < 64",
            "resolved_scene_cache_stats",
        ],
        &[],
    );
    check_hot_path_source_contract(
        root,
        findings,
        "tests/m9_platform_release.rs",
        &[
            "fn m9_pf03_release_scale_prepared_storage_artifact",
            "SCENA_RUN_PF03_STORAGE_BENCHMARK",
            "scena.pf03.prepared_storage.v1",
            "prepared-storage-100k-triangles.json",
            "prepared_list_copy_bytes",
        ],
        &[],
    );
}
