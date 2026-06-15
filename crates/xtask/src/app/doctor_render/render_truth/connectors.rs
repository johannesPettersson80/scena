use crate::app::prelude::*;

pub(crate) fn check_renderer_truth_connector_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/picking.rs",
        &[
            "struct Ray",
            "fn camera_ray",
            "ray_hits_bounds",
            "ray_triangle_intersection",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors.rs",
        &[
            "with_allowed_mate",
            "with_snap_tolerance",
            "with_clearance_hint",
            "with_roll_policy",
            "with_polarity",
            "with_metadata",
            "pub const fn metadata",
            "pub struct ConnectionLineOverlay",
            "pub const fn connection_line",
            "pub const fn resolved_parent",
            "fn reparent_for_connection",
            "pub fn from_anchor_frame",
            "pub fn add_connector",
            "pub fn connector_named",
            "pub fn validate_connections",
            "pub fn connect_by_key",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/magnet.rs",
        &[
            "pub struct ConnectionMagnetPreview",
            "pub enum ConnectionMagnetVisualCue",
            "pub fn preview_connector_magnet",
            "connector_magnet_tolerance",
            "pub const fn ghost_transform",
            "pub const fn css_class",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/scale.rs",
        &["fn preserve_source_scale", "rotate_vec3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/metadata.rs",
        &[
            "pub struct ConnectorMetadata",
            "pub enum ConnectorRollPolicy",
            "pub enum ConnectorPolarity",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/error.rs",
        &[
            "pub enum ConnectionError",
            "StaleConnectorHandle",
            "AmbiguousConnector",
            "UnitMismatch",
            "CoordinateSystemMismatch",
            "FlippedConnection",
            "ConnectionWouldMoveLockedNode",
            "ConnectionWouldCreateCycle",
            "ConnectorHostNotPrepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/options.rs",
        &[
            "pub enum ConnectionAlignment",
            "pub enum ConnectionRoll",
            "pub enum ConnectionParenting",
            "ForwardToBack",
            "NormalToOpposite",
            "pub const fn with_alignment",
            "pub const fn preserve_roll",
            "pub fn choose_nearest_roll_degrees",
            "pub const fn with_explicit_roll_degrees",
            "pub const fn reparent_source_to_target_parent",
            "alignment_transform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/imports.rs",
        &[
            "pub fn connect_import_connectors",
            "ConnectorFrame::from_import_connector",
            "connector_lookup_error",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/locks.rs",
        &[
            "pub fn lock_node_for_connections",
            "pub fn unlock_node_for_connections",
            "pub fn node_connections_locked",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene_host/connectors.rs",
        &[
            "CONNECTOR_BROWSER_SCHEMA_V1",
            "ConnectorBrowserReportV1",
            "connector_browser_json",
            "connector_browser_subtree_json",
            "connector_browser_selection_json",
            "metadata_invalid_reasons",
            "polarity_mismatch",
            "tag_mismatch",
            "preview_connector_magnet",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/connector_browser_contracts.rs",
        &[
            "connector_browser_reports_import_connectors_and_metadata_candidates",
            "connector_browser_reports_subtree_and_selection_scopes",
            "connector_browser_golden_fixture_matches_live_schema_serialization",
            "connector_browser_targets.gltf",
            "CONNECTOR_BROWSER_SCHEMA_V1",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "examples/assembly_connector_browser.rs",
        &[
            "connector_browser_json",
            "connector_debug_scene.gltf",
            "connector_browser_targets.gltf",
            "CONNECTOR_BROWSER_SCHEMA_V1",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Connector browser and snap preview",
            "SceneHostCore::connector_browser_json",
            "examples/assembly_connector_browser.rs",
            "connector-magnet-preview",
            "scena.connector_browser.v1",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/import/variants.rs",
        &["variant_index_for", "AmbiguousVariantName", "matches"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene_host/reporting.rs",
        &["from_import", "material_variants", "active_variant"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/inspection/schema.rs",
        &["SceneImportInspectionV1", "imports", "active_variant"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/material_variant_helpers.rs",
        &[
            "scene_host_material_variant_reports_include_available_and_active_state",
            "scene_host_material_variant_patch_fails_for_stale_and_ambiguous_imports",
            "material_variants_ambiguous_scene.gltf",
            "VisualPatchMaterialVariantV1",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Material variant helpers",
            "scena.scene_host_asset_import.v1",
            "cargo test --features scene-host --test material_variant_helpers",
            "scena-viewer-material-variant-render",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene_host/product_options.rs",
        &[
            "PRODUCT_OPTIONS_SCHEMA_V1",
            "ProductOptionsV1",
            "store_product_options",
            "apply_product_option",
            "self.apply_patch(&patch)",
            "result.failed.is_empty()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/product_configurator_helpers.rs",
        &[
            "product_options_apply_visual_patches_and_report_active_choices",
            "product_options_fail_closed_for_unknown_groups_options_and_bad_patches",
            "product_options_golden_fixture_matches_live_schema_serialization",
            "tests/assets/stable-contracts/product_options.v1.json",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "examples/product_configurator.rs",
        &[
            "store_product_options",
            "apply_product_option_json",
            "product_options_json",
            "VisualPatchV1",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Product configurator helpers",
            "scena.product_options.v1",
            "cargo run --example product_configurator --features",
            "cargo test --test product_configurator_helpers --features scene-host",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene_host/presentation_timeline.rs",
        &[
            "PRESENTATION_TIMELINE_SCHEMA_V1",
            "PresentationTimelineV1",
            "timeline_patch",
            "seek_timeline",
            "advance_timeline",
            "self.apply_patch(&patch)",
            "TimelinePatchBuilder",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/presentation_timeline.rs",
        &[
            "presentation_timeline_seeks_flattened_visual_patch_deterministically",
            "presentation_timeline_advance_samples_animation_clip_from_host_tick",
            "presentation_timeline_golden_fixture_matches_live_schema_serialization",
            "tests/assets/stable-contracts/presentation_timeline.v1.json",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/scene_host_browser_proof.js",
        &[
            "timelinePatchJson",
            "seekTimelineJson",
            "guided_tour_timeline_emits_visual_patch_channels",
            "guided_tour_timeline_browser_render_nonblank",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Presentation timeline",
            "scena.presentation_timeline.v1",
            "cargo test --test presentation_timeline --features",
            "timelinePatchJson",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene_host/product.rs",
        &[
            "SCENE_HOST_GROUNDING_SCHEMA_V1",
            "SceneHostGroundingReportV1",
            "apply_product_grounding_preset",
            "SceneHostGroundingPathV1::FloorReceiver",
            "directional_shadow_receiver_degraded",
            "physical_shadow_claimed: false",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/contact_grounding.rs",
        &[
            "product_grounding_preset_renders_visible_receiver_and_reports_non_physical_shadow_scope",
            "tests/assets/gltf/mesh_material_vertex_color_scene.gltf",
            "target/gate-artifacts/contact-grounding/headless-product-grounding.png",
            "ambient_occlusion_passes > 0",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/browser/scene_host_browser_proof.js",
        &[
            "applyProductGroundingPresetJson",
            "contact_grounding_report_lists_floor_ssao_and_shadow_fallback",
            "contact_grounding_browser_render_nonblank",
            "contact_grounding_browser_runs_ssao_pass",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "docs/checklists/application-builder-roadmap.md",
        &[
            "Contact grounding preset",
            "scena.scene_host_grounding.v1",
            "cargo test --test contact_grounding --features scene-host,inspection",
            "physical_shadow_claimed: false",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/assets/gltf/transform.rs",
        &["basis_rotation", "forward", "up", "Quat::from_mat3"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/assets/gltf/anchors.rs",
        &[
            "tags",
            "label",
            "source_units",
            "parse_source_units",
            "pub struct SceneAssetAnchor",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/anchors.rs",
        &[
            "pub struct AnchorFrame",
            "pub fn from_import_anchor",
            "pub fn add_anchor",
            "pub fn anchor_named",
            "placement_node",
            "MissingAnchor",
            "StaleAnchorHandle",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "src/scene/connectors/validation.rs",
        &[
            "fn is_valid_rotation",
            "validate_connector_live",
            "validate_connector_host_prepared",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-TRUTH",
        "tests/m7_threejs_ergonomics.rs",
        &[
            "m7_camera_projection_renders_world_space_triangle_outside_ndc",
            "m7_moving_camera_changes_rendered_pixels_and_screen_position",
            "m7_rotating_camera_changes_rendered_pixels_and_recenters_target",
            "m7_perspective_and_orthographic_cameras_project_different_pixel_footprints",
            "m7_default_perspective_camera_uses_render_target_aspect_on_wide_viewports",
            "m7_device_pixel_ratio_resize_preserves_projection_aspect",
            "m7_cpu_and_headless_gpu_camera_projection_match_within_tolerance_when_available",
            "m7_picking_uses_camera_ray_for_world_space_triangle_outside_ndc",
            "m7_rotated_connector_connects_without_sideways_orientation_or_offset",
            "m7_forward_to_back_alignment_flips_source_without_manual_rotation",
            "m7_connector_mate_offset_is_applied_in_target_connector_space",
            "m7_connectors_reject_degenerate_connector_rotation",
            "m7_manual_source_unit_mismatch_returns_structured_error",
            "m7_manual_source_coordinate_mismatch_returns_structured_error",
            "m7_negative_determinant_connector_scale_returns_flipped_connection",
            "m7_negative_determinant_node_scale_returns_flipped_connection",
            "m7_locked_connection_source_fails_before_moving_node",
            "m7_gltf_anchor_and_connector_basis_fields_avoid_manual_quaternions",
            "m7_z_up_import_node_rotation_converts_before_connection_solving",
            "m7_explicit_roll_alignment_rotates_around_mated_connector_forward_axis",
            "m7_preserve_roll_alignment_keeps_source_roll_without_manual_matrix_math",
            "m7_choose_nearest_roll_alignment_snaps_source_roll_without_guessing",
            "m7_connection_reparenting_is_explicit_and_preserves_world_transform",
            "m7_connector_placement_preserves_fit_inside_scale_when_solving_position",
            "m7_connector_name_lookup_reports_ambiguity_with_typed_handles",
            "m7_connector_magnet_preview_reports_snap_range_and_visual_cue_without_mutating",
            "m7_validate_connections_returns_preview_without_mutating_scene",
            "connection_line",
            "ConnectionMagnetVisualCue::SnapReady",
            "m7_stale_import_connector_handle_after_hot_reload_is_detected",
            "m7_connector_placement_applies_source_units_before_solving",
            "m7_gltf_anchor_units_override_import_units_for_connection_solving",
            "m7_anchor_frame_registry_uses_typed_handles_and_metadata",
            "m7_import_anchor_tags_and_label_survive_anchor_frame_adapter",
            "m7_import_anchor_frame_preserves_source_metadata_for_connector_adapter",
            "m7_connector_frame_metadata_guides_compatibility_without_domain_logic",
            "m7_imported_gltf_connectors_have_kind_lookup_and_stale_errors",
            "m7_imported_gltf_connector_metadata_survives_frame_adapter",
            "m7_three_imported_objects_connect_into_assembly_without_raw_matrix_math",
            "m7_first_assembly_helper_connects_imported_connectors_by_name",
            "m7_imported_nested_connector_moves_import_root_without_breaking_child_local_transform",
            "m7_imported_animated_connector_keeps_import_local_animation_binding_after_connection",
            "ImportDiagnosticOverlayKind::Connector",
            "AnchorFrame::from_import_anchor",
            "ConnectorFrame::from_import_anchor",
            "ConnectorFrame::from_import_connector",
            "connect_import_connectors",
            "ConnectorFrame::from_anchor_frame",
            "NonUniformScaleConnectionRisk",
            "ConnectionAlignment::ForwardToBack",
            "with_explicit_roll_degrees",
            "choose_nearest_roll_degrees",
            "ConnectorRollPolicy::ChooseNearest",
            "ConnectorPolarity::Plug",
        ],
    );
}
