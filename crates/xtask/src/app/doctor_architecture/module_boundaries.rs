use crate::app::prelude::*;

pub(crate) fn check_module_boundaries(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-MODULES",
        "docs/specs/module-boundaries.md",
        &[
            "`scene`",
            "`scene_host`",
            "`assets`",
            "`geometry`",
            "`material`",
            "`render`",
            "`animation`",
            "`controls`",
            "`picking`",
            "`diagnostics`",
            "`platform`",
            "`vocabulary`",
            "No hidden asset fetch, shader compile, or first-time GPU upload inside `render()`",
            "Photo intent planning and candidate selection stay outside `render`",
            "Shared recipe target resolution stays centralized in `src/scene/recipe/target_resolution.rs`",
            "Host-owned convenience facade exceptions",
            "`HeadlessGltfViewer` and `InteractiveGltfViewer` are the v1.0 host-owned convenience",
            "Large module allowlist",
            "`src/assets.rs`",
            "`src/viewer.rs`",
            "`src/bin/scena/photo.rs`",
            "`src/scene_host/photo.rs`",
        ],
    );

    forbid_contains(
        root,
        findings,
        "ARCH-PLATFORM",
        "src/platform.rs",
        &["wgpu::", "ForwardPass", "ShadowPass", "PostProcessPass"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSETS",
        "src/assets.rs",
        &["wgpu::", "RenderPass", "Surface"],
    );
    check_render_asset_loading_contracts(root, findings);
    check_render_photo_planning_boundary(root, findings);
    check_shared_target_resolver_consumers(root, findings);
    check_subject_visibility_reason_contract(root, findings);
    forbid_contains_required_path(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        Path::new("src/render/gpu/draw.rs"),
        &[
            "create_shader_module",
            "create_render_pipeline",
            "create_buffer",
            "create_texture",
            "create_bind_group",
            "request_adapter",
            "request_device",
            "mapped_at_creation: true",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/draw_surface.rs",
        &[
            "pub(in crate::render) fn render_to_surface",
            "GpuResourcesNotPrepared",
            "surface_frame::acquire_surface_frame",
            "encode_scene_color_passes",
            "surface_output.present();",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/scene_color.rs",
        &["encode_unlit_pass", "ColorLoad::Load", "TransparentOnly"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/prepare_resources.rs",
        &[
            "self.configure_surface(target);",
            "self.release_prepared_resources();",
            "encode_retained_vertices(retained_primitives, retained_instances)",
            "encode_draw_resources(",
            "create_material_resources",
            "material_slots",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/resource_encoding.rs",
        &[
            "let retained_instance_primitives = retained_instances",
            ".chain(retained_instance_primitives)",
            "encode_vertices_iter(",
            "vertices::encode_draw_batches_indexed_with_semantics(",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/gpu/lifecycle.rs",
        &["pub(in crate::render) fn clear_prepared_resources_for_context_recovery"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-LIFECYCLE",
        "src/render/surface.rs",
        &["gpu.clear_prepared_resources_for_context_recovery();"],
    );
}

pub(crate) fn check_shared_target_resolver_consumers(root: &Path, findings: &mut Vec<Finding>) {
    for (rel, owner) in [
        (
            "src/bin/scena/photo.rs",
            "photo plan/render subject selection",
        ),
        (
            "src/bin/scena/recipe/subject_metering.rs",
            "subject auto-exposure metering",
        ),
        (
            "src/bin/scena/recipe/subject_focus.rs",
            "subject depth-of-field focus",
        ),
        (
            "src/bin/scena/recipe/verification.rs",
            "recipe expectation target resolution",
        ),
        (
            "src/scene_host/composition/helpers.rs",
            "scene composition target resolution",
        ),
        (
            "src/scene_host/composition/subject.rs",
            "subject observation target resolution",
        ),
    ] {
        require_contains(
            root,
            findings,
            "ARCH-SHARED-TARGET-RESOLVER",
            rel,
            &["resolve_scene_recipe_target_handles("],
        );
        let path = root.join(rel);
        if path.exists() {
            let Ok(text) = read_source_to_string(root, Path::new(rel)) else {
                continue;
            };
            if text.contains("SceneRecipeTargetV1::Import")
                && text.contains("SceneRecipeTargetV1::Node")
                && !text.contains("resolve_scene_recipe_target_handles(")
            {
                findings.push(Finding::new(
                    "ARCH-SHARED-TARGET-RESOLVER",
                    format!(
                        "{rel} matches import and node targets for {owner} without calling resolve_scene_recipe_target_handles; keep target handle resolution centralized in src/scene/recipe/target_resolution.rs",
                    ),
                ));
            }
        }
    }
}

pub(crate) fn check_subject_visibility_reason_contract(root: &Path, findings: &mut Vec<Finding>) {
    let required_reason_codes = [
        "subject_hidden",
        "subject_outside_viewport",
        "subject_behind_camera",
        "subject_degenerate_geometry",
        "subject_clipped_by_section_box",
        "subject_clipped_by_clipping_plane",
        "subject_transparent_unsupported",
        "subject_occluded",
        "subject_visible_mask_empty",
        "stale_subject_observation",
    ];
    require_contains(
        root,
        findings,
        "ARCH-SUBJECT-VISIBILITY-REASONS",
        "src/scene_host/composition/subject.rs",
        &required_reason_codes[..required_reason_codes.len() - 1],
    );
    require_contains(
        root,
        findings,
        "ARCH-SUBJECT-VISIBILITY-REASONS",
        "tests/scena_cli_recipe.rs",
        &[
            "scena_recipe_render_verify_reports_zero_visible_subject_reason_codes",
            "scena_recipe_render_verify_reports_zero_visible_photo_and_focus_subject_reason_codes",
            "subject.photo_subject.visible_mask",
            "subject.render_depth_of_field_focus.visible_mask",
            "subject_hidden",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SUBJECT-VISIBILITY-REASONS",
        "docs/schema-contracts.md",
        &required_reason_codes,
    );
}

pub(crate) fn check_render_photo_planning_boundary(root: &Path, findings: &mut Vec<Finding>) {
    for rel in source_files(root)
        .into_iter()
        .filter(|rel| is_render_source_path(rel))
    {
        let Ok(text) = read_source_to_string(root, &rel) else {
            findings.push(Finding::new(
                "ARCH-RENDER-PHOTO-BOUNDARY",
                format!(
                    "could not read {} for photo-planning boundary scan",
                    rel.display()
                ),
            ));
            continue;
        };
        for needle in [
            "PhotoCandidateRequest",
            "PhotoCandidatePlanV1",
            "PhotoCompositionCandidateV1",
            "PhotoCandidateScoringReport",
            "PhotoPlanV1",
            "PhotoReportV1",
            "PHOTO_PLAN_SCHEMA_V1",
            "PHOTO_REPORT_SCHEMA_V1",
            "PHOTO_SHADED_CANDIDATE_SELECTION_SCHEMA_V1",
            "product_hero_candidate_plan",
            "apply_product_hero_setup",
        ] {
            if text.contains(needle) {
                findings.push(Finding::new(
                    "ARCH-RENDER-PHOTO-BOUNDARY",
                    format!(
                        "{} contains photo-planning boundary text '{}'; keep photo intent planning and candidate selection in scene_host/CLI setup before explicit prepare/render",
                        rel.display(),
                        needle
                    ),
                ));
            }
        }
    }
}

fn is_render_source_path(rel: &Path) -> bool {
    rel == Path::new("src/render.rs") || rel.starts_with(Path::new("src/render/"))
}
