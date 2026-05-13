pub(crate) use std::collections::{BTreeMap, BTreeSet};
pub(crate) use std::env;
pub(crate) use std::ffi::OsStr;
pub(crate) use std::fs;
pub(crate) use std::path::{Path, PathBuf};
pub(crate) use std::process::{self, Command as ProcessCommand};
pub(crate) use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) use serde_json::{Value, json};
pub(crate) use sha2::{Digest, Sha256};

pub(crate) use crate::app::architecture_map::{
    ARCHITECTURE_OWNER_MODULES, PublicApiOwnershipEntry, architecture_dependencies_dot,
    architecture_dependency_owners, architecture_modules_json, architecture_owner_for_source_path,
    declared_public_type_names, public_api_ownership_json, read_public_api_ownership,
    run_architecture_map, write_pretty_json_artifact,
};
pub(crate) use crate::app::core::{
    Command, DoctorMode, Finding, VisualProofCommand, finding_reference, parse_command,
    print_usage, run,
};
pub(crate) use crate::app::doctor_architecture::{
    check_architecture_contract, check_architecture_dependency_direction, check_module_boundaries,
    check_public_api_ownership, check_render_asset_loading_contracts,
    check_render_singleton_contracts, check_viewer_facade_contracts, check_xtask_module_split,
    collect_xtask_source_files, contains_xtask_include_macro, find_public_api_definition_path,
    forbid_contains_required_path, public_api_definition_exists, public_definition_name,
    public_reexported_type_names, public_struct_names, public_use_exports_name,
    render_asset_loading_patterns, xtask_cross_module_glob_import, xtask_source_files,
};
pub(crate) use crate::app::doctor_core::{
    ALLOWED_CONTEXT_TYPES, CATCH_ALL_TYPE_NAMES, CATCH_ALL_TYPE_SUFFIXES,
    MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE, MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE, REQUIRED_DOCS,
    REQUIRED_SOURCE_MODULES, SOURCE_SCOPE_TERMS, STALE_DOC_TERMS, check_cpu_ibl_gap_documented,
    check_m8_real_asset_dual_lane, check_no_ignored_release_tests,
    check_tests_env_flags_documented, check_waterbottle_third_party_reference, find_env_var_names,
    repo_root, require_files, run_architecture_doctor, run_docs_doctor, run_doctor,
};
pub(crate) use crate::app::doctor_docs::{
    check_for_stale_doc_terms, check_markdown_links, check_required_doc_contracts,
    check_source_scope, collect_markdown, is_external_link, markdown_files, markdown_link_targets,
    require_contains,
};
pub(crate) use crate::app::doctor_m7_m8_assets::{
    backtick_values, binary_render_asset_extension, check_binary_render_asset_contracts,
    check_gltf_asset_matrix_contract, check_m7_ergonomics_contracts,
    check_m8_assets_materials_contracts, check_manifest_file_hash,
    check_state_of_art_checklist_links, check_tangent_generation_dependency_contracts,
    collect_text_binary_asset_findings, contains_placeholder, derivative_manifest_entries,
    expected_result_is_explicit, first_backtick_value, is_local_evidence_path, is_lower_hex_sha256,
    khronos_manifest_file_hashes, looks_like_text_fixture, quoted_array_item, quoted_assignment,
    quoted_manifest_assignment, require_manifest_u32, require_manifest_value, sha256_hex,
    u32_manifest_assignment,
};
pub(crate) use crate::app::doctor_render::{
    check_asset_api_contracts, check_diagnostics_contracts, check_fxaa_output_contracts,
    check_output_stage_contracts, check_prepare_asset_contracts, check_render_alpha_contracts,
    check_render_world_bake_contracts, check_renderer_standard_math_contracts,
    check_renderer_stats_contracts, check_renderer_truth_contracts,
};
pub(crate) use crate::app::doctor_scene_platform::{
    MILESTONE_CHECKLISTS, REQUIRED_EXAMPLES, REQUIRED_M5_GATE_ARTIFACTS, check_agent_validation,
    check_backend_vocabulary, check_camera_depth_contracts, check_clipping_contracts,
    check_depth_prepass_contracts, check_direct_light_shading_contracts,
    check_directional_shadow_contracts, check_environment_ibl_prepare_contracts,
    check_environment_lifecycle_contracts, check_equirectangular_hdr_environment_contracts,
    check_m2_leak_stats_contracts, check_m3a_scene_import_contracts, check_m3b_animation_contracts,
    check_m4_platform_contracts, check_m5_release_contracts, check_material_desc_fields_private,
    check_origin_shift_contracts, check_reversed_z_contracts, check_scene_light_contracts,
    check_shadow_map_contracts, check_unit_test_first_governance, check_webgl2_depth_contracts,
    contains_scope_term,
};
pub(crate) use crate::app::doctor_visual_release::{
    check_default_environment_derivative_payload, check_default_environment_manifest,
    check_m1_browser_rendered_output, check_m2_browser_rendered_output,
    check_m2_visual_fixture_metadata, check_m6_browser_renderer_probe, check_m9_ci_release_lanes,
    check_m10_claim_audit_contract, check_ndc_smoke_fixture_classification,
    check_release_publish_dry_run_helper, check_release_readiness_ci_fail_closed,
    check_visual_fixture_metadata, fixture_block, jobs_with_continue_on_error_release_readiness,
    require_contains_in_xtask_app_tree,
};
pub(crate) use crate::app::release::{
    MIN_BENCHMARK_SAMPLE_COUNT, RELEASE_ARTIFACT_MAX_AGE_SECONDS,
    RELEASE_ARTIFACT_MAX_FUTURE_SKEW_SECONDS, RELEASE_LANE_ARTIFACT_SUFFIXES,
    REQUIRED_BENCHMARK_ARTIFACT_SUFFIXES, REQUIRED_JSON_COMMIT_ARTIFACT_SUFFIXES,
    REQUIRED_JSON_TIMESTAMP_ARTIFACT_SUFFIXES, REQUIRED_MEASURED_CAPABILITY_ARTIFACT_SUFFIXES,
    REQUIRED_NATIVE_GPU_RENDER_ARTIFACT_SUFFIXES, REQUIRED_NON_CONSTANT_PPM_ARTIFACT_SUFFIXES,
    REQUIRED_PASSED_STATUS_ARTIFACT_SUFFIXES, REQUIRED_RELEASE_ARTIFACT_SUFFIXES,
    REQUIRED_RENDERED_OUTPUT_METADATA_ARTIFACT_SUFFIXES, REQUIRED_REVIEW_ROLES,
    REQUIRED_VISUAL_PROOF_ARTIFACT_SUFFIXES, ReleaseFindingBlock, check_release_artifact_bundle,
    check_release_readiness, check_release_readiness_adr, check_release_readiness_artifact_env,
    check_release_readiness_checklists, check_release_review_artifacts, copy_optional_json_field,
    iterate_finding_blocks, parse_release_review_frontmatter, release_artifact_commit_label,
    release_lane_artifact, release_lane_command_records, release_lane_command_records_pass,
    release_lane_content_ok, release_lane_evidence, release_lane_expected_commands,
    release_lane_measured_command_records, release_lane_required_artifacts,
    require_json_status_passed, run_claim_audit, run_release_lane_artifact, run_release_readiness,
    scrape_toml_bool_value, scrape_toml_string_value, validate_findings_register_schema,
    validate_maintainer_signoff_schema, validate_release_review_report,
};
pub(crate) use crate::app::util::{
    brace_delta, braced_body_after, check_solid_kiss, collect_source_files, declared_type_name,
    declared_type_names, forbid_contains, forbid_contains_path, is_catch_all_type_name,
    public_fields_in_struct, significant_line_count, source_files, strip_rust_visibility,
};
pub(crate) use crate::app::visual_artifacts::{
    build_claim_audit, check_required_visual_proof_artifacts, claim_audit_paths, claim_categories,
    collect_files_with_extensions, current_unix_seconds, evidence_links_for_category,
    headless_cpu_render_proof_passes, json_contains_string_value, native_gpu_render_proof_passes,
    path_ends_with, pbr_light_render_proof_passes, ppm_payload_is_constant_rgb, ppm_pixel_payload,
    reject_constant_ppm_artifact, reject_stale_json_commit, reject_stale_json_timestamp,
    reject_unmeasured_capability_matrix_rows, require_benchmark_baseline_comparison,
    require_benchmark_baseline_comparison_file, require_native_gpu_render_proof,
    require_release_lane_artifact_evidence, require_release_lane_artifact_file,
    require_rendered_output_screenshot_metadata, require_rendered_output_screenshot_metadata_file,
    require_screenshot_metadata_entry, require_visual_proof_artifact_file,
};
pub(crate) use crate::app::visual_proof::{
    path_to_forward_slash, process_signal, run_visual_proof, run_visual_proof_command,
    sanitize_visual_proof_lane, visual_proof_rust_test_nonzero_pass_summary_observed,
};
