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
        "src/render/prepare/lights.rs",
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
        "src/scene.rs",
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
        "src/geometry/primitive_meshes.rs",
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
