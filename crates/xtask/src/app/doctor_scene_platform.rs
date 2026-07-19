mod animation_material;
mod calibration_oracles;
mod camera_depth;
mod capture_sequence;
mod deformed_picking;
mod environment_lighting;
mod finite_atomic;
mod governance_backend;
mod gpu_resource_lifecycle;
mod handle_namespaces;
mod overlay_ownership;
mod platform_contracts;
mod presentation_timeline;
mod recipe_diff;
mod recipe_spatial_state;
mod release_contracts;
mod remote_builder;
mod scene_import;
mod semantic_aov;
mod shadow_depth;
mod strict_gpu_construction;

pub(crate) use animation_material::{
    check_m3b_animation_contracts, check_material_desc_fields_private,
};
pub(crate) use calibration_oracles::check_calibration_oracles_pair_parity_sweeps;
pub(crate) use camera_depth::{
    check_camera_depth_contracts, check_clipping_contracts, check_m2_leak_stats_contracts,
    check_origin_shift_contracts, check_reversed_z_contracts, check_webgl2_depth_contracts,
};
pub(crate) use capture_sequence::check_fr05_capture_sequence_contracts;
pub(crate) use deformed_picking::check_c12_deformed_picking_contracts;
pub(crate) use environment_lighting::{
    check_direct_light_shading_contracts, check_environment_ibl_prepare_contracts,
    check_environment_lifecycle_contracts, check_equirectangular_hdr_environment_contracts,
    check_scene_light_contracts,
};
pub(crate) use finite_atomic::check_c06_finite_atomic_contracts;
pub(crate) use governance_backend::{
    check_agent_validation, check_backend_vocabulary, check_unit_test_first_governance,
    contains_scope_term,
};
pub(crate) use gpu_resource_lifecycle::check_c09_gpu_resource_lifecycle_contracts;
pub(crate) use handle_namespaces::check_c07_handle_namespace_contracts;
pub(crate) use overlay_ownership::check_c10_overlay_ownership_contracts;
pub(crate) use platform_contracts::check_m4_platform_contracts;
pub(crate) use presentation_timeline::check_c08_presentation_timeline_contracts;
pub(crate) use recipe_diff::check_fr07_recipe_diff_contracts;
pub(crate) use recipe_spatial_state::check_fr08_recipe_spatial_state_contracts;
pub(crate) use release_contracts::{REQUIRED_EXAMPLES, check_m5_release_contracts};
pub(crate) use remote_builder::check_remote_builder_bootstrap_contracts;
pub(crate) use scene_import::check_m3a_scene_import_contracts;
pub(crate) use semantic_aov::check_fr06_semantic_aov_contracts;
pub(crate) use shadow_depth::{
    check_depth_prepass_contracts, check_directional_shadow_contracts, check_shadow_map_contracts,
};
pub(crate) use strict_gpu_construction::check_c13_strict_gpu_construction_contracts;
