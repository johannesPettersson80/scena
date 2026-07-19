use crate::app::prelude::*;

pub(crate) fn check_renderer_standard_math_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STANDARD-MATH",
        "src/render/prepare/pbr_contract.rs",
        &[
            "pbr_material_uses_gltf_dielectric_and_metallic_f0",
            "light_units_do_not_apply_scene_tuned_divisors_or_clamps",
            "clearcoat_light_contribution",
            "clearcoat_light_contribution_adds_dielectric_lobe",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-STANDARD-MATH",
        "docs/specs/color-contract.md",
        &["glTF", "sRGB", "linear"],
    );

    for rel in [
        "src/render/gpu/output_shader.wgsl",
        "src/render/prepare.rs",
        "src/render/prepare/lighting.rs",
        "src/render/prepare/lighting/area.rs",
        "src/render/prepare/lighting/counts.rs",
        "src/render/prepare/lighting/gpu_uniform.rs",
        "src/render/prepare/lighting/lobes.rs",
        "src/render/prepare/lighting/ltc.rs",
        "src/render/prepare/lighting/math.rs",
        "src/render/prepare/lighting/tiled.rs",
        "src/render/prepare/materials.rs",
    ] {
        forbid_contains(
            root,
            findings,
            "ARCH-RENDER-STANDARD-MATH",
            rel,
            &[
                "mix(0.92, 1.0, roughness)",
                "metallic damp",
                "metallic_damp",
                "lux / 10000",
                "candela / 100",
                "scene_tuned",
            ],
        );
    }
}

pub(crate) fn check_prepare_asset_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare_lifecycle.rs",
        &[
            "pub fn prepare_with_assets",
            "prepare::collect_prepared_primitives",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare.rs",
        &[
            "fn collect_prepared_primitives",
            "PrepareError::AssetsRequired",
            "TransparentPrimitive",
            "total_cmp",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/primitives.rs",
        &["fn append_geometry_primitives"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/cpu_bake.rs",
        &[
            "fn average_sort_depth",
            "push_material_pass_primitive",
            "subdivided_cpu_corners",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/materials.rs",
        &["fn material_pass", "validate_material_texture_handles"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/render/prepare/strokes.rs",
        &[
            "fn append_line_primitives",
            "fn append_wireframe_primitives",
            "fn append_edge_primitives",
            "struct EdgeCandidate",
            "fn append_line_segment",
            "fn screen_x_to_ndc",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/diagnostics.rs",
        &[
            "AssetsRequired",
            "GeometryNotFound",
            "MaterialNotFound",
            "TextureNotFound",
            "UnsupportedGeometryTopology",
            "UnsupportedMaterialKind",
            "UnsupportedAlphaMode",
            "UnsupportedModelNode",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/scene/render_nodes.rs",
        &["pub(crate) fn mesh_nodes", "pub(crate) fn model_nodes"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "tests/m1_geometry_materials.rs",
        &[
            "prepare_with_assets_renders_scene_mesh_unlit_geometry",
            "prepare_without_assets_rejects_asset_backed_mesh_nodes",
            "prepare_with_assets_sorts_blend_meshes_back_to_front_before_render",
            "prepare_with_assets_renders_line_material_as_screen_space_stroke",
            "prepare_with_assets_renders_wireframe_material_triangle_edges",
            "prepare_with_assets_renders_edge_material_without_coplanar_internal_edges",
            "headless_gpu_renders_technical_material_primitives_when_available",
            "prepare_with_assets_rejects_unsupported_mesh_inputs_structurally",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/geometry/primitive_meshes/tests.rs",
        &["built_in_triangle_primitives_are_wound_against_vertex_normals"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/scene_host/recipe/authoring/geometry/projection.rs",
        &["projected_geometry_counts_match_authored_primitive_builders"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "tests/scene_recipe_contracts.rs",
        &["scene_recipe_build_policy_rejects_arrow_projection_underestimate"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "src/scene_host/recipe.rs",
        &["recipe_primitives_render_lit_single_sided_pixels_on_headless_gpu"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-ASSETS",
        "docs/specs/public-api.md",
        &["pub fn prepare_with_assets<F>"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/prepare/stats.rs",
        &[
            "validate_gpu_light_capacity",
            "gpu_light_uniform_capacity",
            "PreparedLights::from_scene",
            "MAX_GPU_AREA_LIGHTS",
            "tiled light assignment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/prepare/lighting.rs",
        &[
            "collect_gpu_tiled_light_assignment",
            "AREA_LIGHT_SAMPLE_COUNT",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/prepare/lighting/tests.rs",
        &[
            "gpu_lighting_stats_accept_many_point_lights_for_tiled_assignment",
            "area_lights_use_separate_gpu_capacity_from_point_lights",
            "area_light_shape_encodes_gpu_visible_area_lane",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/gpu/output_shader.wgsl",
        &[
            "light_tile_indices",
            "tiled_light_records",
            "tiled_lighting_active",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/prepare/shadows.rs",
        &["area_shadow_visibility_uses_dense_emitter_samples"],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/gpu/output.rs",
        &[
            "triangle_shader_multiplies_area_lights_by_prepared_area_shadow_visibility",
            "include_str!(\"../area_ltc_tables.wgsl\")",
            "include_str!(\"../area_ltc.wgsl\")",
            "triangle_shader_contains_ltc_area_light_specular_path_for_both_texture_layouts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/area_ltc.rs",
        &[
            "selfshadow/ltc_code",
            "sample_ltc_tables",
            "clip_quad_to_horizon",
            "ltc_lookup_matches_reference_derived_compact_table_probes",
            "ltc_rect_probe_matches_selfshadow_reference_irradiance",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/area_ltc_tables.rs",
        &[
            "90c2ae903e5e460c03f28bc14d0391dba9578e71",
            "pub(super) const LTC_1",
            "pub(super) const LTC_2",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "tests/scena_cli_recipe.rs",
        &[
            "scena_recipe_render_gpu_many_lights_use_tiled_assignment_before_truncation",
            "scena_recipe_render_gpu_tiled_many_point_lights_use_late_light",
            "tiled-many-point-light-blue-delta.json",
        ],
    );
    check_area_light_acceptance_honesty(root, findings);
    forbid_contains(
        root,
        findings,
        "ARCH-PREPARE-LIGHT-CAPACITY",
        "src/render/prepare/lighting.rs",
        &["fifth directional light is intentionally capped"],
    );
}

pub(crate) fn check_area_light_acceptance_honesty(root: &Path, findings: &mut Vec<Finding>) {
    let checklist_rel = "docs/checklists/stunning-renders-and-performance.md";
    let checklist_path = root.join(checklist_rel);
    let Ok(checklist) = fs::read_to_string(&checklist_path) else {
        findings.push(Finding::new(
            "ARCH-PREPARE-AREA-LIGHT-HONESTY",
            format!("could not read {checklist_rel}"),
        ));
        return;
    };

    let render_source = render_source_text(root);
    let render_source_lower = render_source.to_ascii_lowercase();
    if checklist_claims_ltc_shipped(&checklist) && !render_source_contains_ltc(&render_source_lower)
    {
        findings.push(Finding::new(
            "ARCH-PREPARE-AREA-LIGHT-HONESTY",
            "A3 cannot be marked shipped or its LTC checkbox checked until src/render contains the shared fitted-table linearly-transformed-cosine implementation and CPU/GPU shader route",
        ));
    }
    if checklist_claims_clustered_light_culling_shipped(&checklist)
        && !render_source_contains_clustered_light_assignment(&render_source)
    {
        findings.push(Finding::new(
            "ARCH-PREPARE-AREA-LIGHT-HONESTY",
            "B2 cannot be marked shipped until src/render contains a clustered/tiled light-assignment implementation marker such as clustered_light_grid, light_tile_indices, or assign_lights_to_tiles",
        ));
    }
}

fn checklist_claims_ltc_shipped(checklist: &str) -> bool {
    checklist.lines().any(|line| {
        (line.contains("A3") && line.contains("Soft area lights") && line.contains("[shipped]"))
            || (line.contains("[x]")
                && line.contains("LTC")
                && line.contains("linearly-transformed cosines"))
    })
}

fn checklist_claims_clustered_light_culling_shipped(checklist: &str) -> bool {
    checklist.lines().any(|line| {
        (line.contains("B2")
            && line.contains("Clustered / tiled light culling")
            && line.contains("[shipped]"))
            || (line.contains("[x]") && line.contains("Cluster/tile light assignment"))
    })
}

fn render_source_text(root: &Path) -> String {
    let mut text = String::new();
    for rel in source_files(root) {
        if !rel.starts_with("src/render") {
            continue;
        }
        let Ok(source) = fs::read_to_string(root.join(&rel)) else {
            continue;
        };
        text.push_str(&source);
        text.push('\n');
    }
    text
}

fn render_source_contains_ltc(render_source_lower: &str) -> bool {
    [
        "selfshadow/ltc_code",
        "sample_ltc_tables",
        "ltc_1",
        "ltc_2",
        "evaluate_specular_polygon",
        "clip_quad_to_horizon",
    ]
    .iter()
    .all(|marker| render_source_lower.contains(marker))
}

fn render_source_contains_clustered_light_assignment(render_source: &str) -> bool {
    [
        "clustered_light_grid",
        "light_tile_indices",
        "light_cluster_indices",
        "assign_lights_to_tiles",
        "assign_lights_to_clusters",
        "lights_per_tile",
        "lights_per_cluster",
        "LightCluster",
        "TiledLightAssignment",
    ]
    .iter()
    .any(|marker| render_source.contains(marker))
}

pub(crate) fn check_particle_prepare_allocation_contract(root: &Path, findings: &mut Vec<Finding>) {
    let rel = "src/render/prepare/particles.rs";
    let path = root.join(rel);
    let Ok(source) = fs::read_to_string(&path) else {
        findings.push(Finding::new(
            "ARCH-PREPARE-PARTICLES",
            format!("could not read {rel}"),
        ));
        return;
    };
    for forbidden in [".collect::<Vec", ".collect()"] {
        if source.contains(forbidden) {
            findings.push(Finding::new(
                "ARCH-PREPARE-PARTICLES",
                format!(
                    "{rel} must not collect particle iterators into an intermediate Vec before CPU-baked billboard emission; found `{forbidden}`"
                ),
            ));
        }
    }
    for required in ["particle_primitive_count", "primitives.reserve"] {
        if !source.contains(required) {
            findings.push(Finding::new(
                "ARCH-PREPARE-PARTICLES",
                format!(
                    "{rel} must pre-count particle primitives and reserve output capacity before appending; missing `{required}`"
                ),
            ));
        }
    }
}
