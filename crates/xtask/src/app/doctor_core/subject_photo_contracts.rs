use crate::app::prelude::*;

pub(crate) fn check_x01_subject_photo_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "X01-SUBJECT-PHOTO-CONTRACTS";
    const REPORT_FIXTURES: &[(&str, &str)] = &[
        (
            "scena.focus_report.v1",
            "tests/assets/stable-contracts/focus_report.v1.json",
        ),
        (
            "scena.exposure_report.v1",
            "tests/assets/stable-contracts/exposure_report.v1.json",
        ),
        (
            "scena.subject_observation.v1",
            "tests/assets/stable-contracts/subject_observation.v1.json",
        ),
        (
            "scena.photo_render_result.v1",
            "tests/assets/stable-contracts/photo_render_result.v1.json",
        ),
        (
            "scena.photo_plan.v1",
            "tests/assets/stable-contracts/photo_plan.v1.json",
        ),
        (
            "scena.photo_candidate_plan.v1",
            "tests/assets/stable-contracts/photo_candidate_plan.v1.json",
        ),
        (
            "scena.photo_shaded_candidate_selection.v1",
            "tests/assets/stable-contracts/photo_shaded_candidate_selection.v1.json",
        ),
        (
            "scena.photo_report.v1",
            "tests/assets/stable-contracts/photo_report.v1.json",
        ),
    ];

    for (schema, fixture) in REPORT_FIXTURES {
        require_contains(root, findings, RULE, "docs/schema-contracts.md", &[*schema]);
        require_contains(
            root,
            findings,
            RULE,
            "src/schema_catalog.rs",
            &[*schema, *fixture],
        );
        require_contains(
            root,
            findings,
            RULE,
            "src/schema_catalog/fixtures.rs",
            &[*schema, *fixture],
        );
        require_contains(root, findings, RULE, fixture, &[*schema]);
    }

    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/help.rs",
        &[
            "photo plan <asset-or-recipe>",
            "photo render <asset-or-recipe>",
            "--report <json>",
            "scena.photo_plan.v1",
            "scena.photo_render_result.v1",
            "scena.photo_report.v1",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/bin/scena/photo.rs",
        &[
            "geometry_derived_semantic_mask",
            "highlight_continuity",
            "shadow_presence",
            "silhouette_separation",
            "reflection_washout",
            "ensure_photographic_asset_usable",
            "texture_resolution_below_output_demand",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/scene_host/photographic_surface.rs",
        &[
            "PhotographicAssetIssueClassV1",
            "SafeRepair",
            "AppearanceChangeRequired",
            "Unrecoverable",
            "folded_geometry",
            "self_intersecting_geometry",
            "hidden_subject_component",
            "duplicate_subject_component",
            "automatic photorealistic rendering for coherent geometry",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/scene_host/photographic_surface.rs",
        &[
            "photographic_asset_health_reports_safe_repairs_and_supported_promise",
            "photographic_asset_health_reports_hidden_and_duplicate_components",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/geometry/photographic.rs",
        &[
            "triangle_self_intersection_count",
            "deduplicate_safe_vertices",
            "folded_edges",
            "duplicate_vertices_removed",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "src/geometry/photographic.rs",
        &["photographic_geometry_health_detects_crossing_faces_and_safe_duplicates"],
    );

    require_contains(
        root,
        findings,
        RULE,
        ".github/workflows/ci.yml",
        &[
            "TESTS-FEATURE-GATED-WORKFLOW-BIJECTION",
            "cargo test --workspace --all-features --tests",
        ],
    );

    require_contains(
        root,
        findings,
        RULE,
        "tests/assets/photo/camera_behavior_cad_terminal_block.fixture.json",
        &[
            "scena.camera_behavior_fixture.v1",
            "average_metered_silhouette",
            "stale_subject_mask",
            "wrong_subject_target",
            "old_ev_cap_underexposed",
            "post_tonemap_metering_strict_lane",
            "pulled_back_empty_slab",
            "off_center_subject",
            "flat_gray_metal",
            "wrong_focus",
            "missing_steel_reflection_structure",
            "blown_highlights",
        ],
    );

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/photo_render_cli.rs",
        &[
            "camera_behavior_fixture_manifest_pins_source_bands_and_mutations",
            "photo_render_camera_behavior_is_easy_path_for_imported_asset",
            "photo_plan_camera_behavior_emits_render_free_public_plan_for_imported_asset",
            "photo_render_failed_loop_reports_measured_candidate_history",
            "recipe_render_camera_behavior_photo_intent_is_easy_path_for_imported_asset",
            "checked_in_demo_hero_recipe_uses_photo_intent_without_manual_overrides",
            "demo_next_hero_uses_checked_camera_behavior_proof_asset",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scena_cli_recipe.rs",
        &[
            "recipe_render_subject_focus_resolves_depth_and_runs_dof_pass",
            "recipe_render_subject_focus_accepts_authored_node_targets",
            "recipe_render_subject_metering_and_focus_work_without_photo_intent",
            "recipe_render_product_quality_uses_exact_subject_observation_pixels",
        ],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/scena_cli_schema.rs",
        &["scena_schema_cli_lists_and_gets_stable_contracts"],
    );

    require_contains(
        root,
        findings,
        RULE,
        "docs/troubleshooting.md",
        &[
            "Camera behavior or subject-aware render failed",
            "stale_subject_observation",
            "unsupported subject mask",
            "focus fallback",
            "failed camera-behavior acceptance",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "docs/errors.md",
        &[
            "Camera-behavior and subject-observation failures",
            "scena.photo_render_result.v1",
            "scena.photo_report.v1",
            "subject_visible_mask_backend_unsupported",
            "stale_subject_observation",
        ],
    );

    for path in [
        "README.md",
        "docs/getting-started.md",
        "docs/guides/easy-scene-setup.md",
        "docs/guides/llm-app-builder.md",
    ] {
        require_contains(
            root,
            findings,
            RULE,
            path,
            &[
                "scena photo render model.glb --out hero.png --report hero.report.json",
                "photo.intent",
            ],
        );
    }
    require_contains(
        root,
        findings,
        RULE,
        "README.md",
        &["no manual camera, exposure, or focus"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "docs/getting-started.md",
        &["no manual camera, exposure, or focus"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "docs/guides/easy-scene-setup.md",
        &["no manual camera, exposure, or focus"],
    );
    require_contains(
        root,
        findings,
        RULE,
        "docs/guides/llm-app-builder.md",
        &["no hand-tuned camera, exposure, focus"],
    );

    require_contains(
        root,
        findings,
        RULE,
        "demo-next/index.html",
        &["assets/hero.recipe.json", "assets/hero-915e9e36c3.png"],
    );

    check_camera_behavior_recipe_has_no_manual_overrides(
        root,
        findings,
        RULE,
        "evidence/demo-hero/hero.recipe.json",
    );
    check_camera_behavior_recipe_has_no_manual_overrides(
        root,
        findings,
        RULE,
        "demo-next/assets/hero.recipe.json",
    );
}

fn check_camera_behavior_recipe_has_no_manual_overrides(
    root: &Path,
    findings: &mut Vec<Finding>,
    rule: &'static str,
    rel: &str,
) {
    let Ok(text) = fs::read_to_string(root.join(rel)) else {
        findings.push(Finding::new(rule, format!("could not read {rel}")));
        return;
    };
    let Ok(recipe) = serde_json::from_str::<Value>(&text) else {
        findings.push(Finding::new(
            rule,
            format!("{rel} must remain valid JSON for camera-behavior doctor coverage"),
        ));
        return;
    };
    if recipe.pointer("/photo/intent") != Some(&Value::String("camera_behavior".to_owned())) {
        findings.push(Finding::new(
            rule,
            format!("{rel} must use photo.intent camera_behavior"),
        ));
        return;
    }
    if recipe.pointer("/photo/subject").is_none() {
        findings.push(Finding::new(
            rule,
            format!("{rel} must declare photo.subject for subject-driven metering/focus"),
        ));
    }

    for pointer in [
        "/render/exposure_ev",
        "/render/exposure_compensation",
        "/render/exposure_compensation_ev",
        "/render/depth_of_field/focus_distance",
        "/scene/grid",
        "/scene/background",
    ] {
        if recipe.pointer(pointer).is_some() {
            findings.push(Finding::new(
                rule,
                format!("{rel} contains manual camera-behavior override {pointer}"),
            ));
        }
    }

    for field in ["geometries", "nodes", "lights", "cameras"] {
        if recipe.get(field).is_some() {
            findings.push(Finding::new(
                rule,
                format!("{rel} contains manual camera-behavior override {field}"),
            ));
        }
    }
}
