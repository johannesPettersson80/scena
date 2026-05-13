mod animation_material;
mod camera_depth;
mod environment_lighting;
mod governance_backend;
mod platform_contracts;
mod release_contracts;
mod scene_import;
mod shadow_depth;

pub(crate) use animation_material::{
    check_m3b_animation_contracts, check_material_desc_fields_private,
};
pub(crate) use camera_depth::{
    check_camera_depth_contracts, check_clipping_contracts, check_m2_leak_stats_contracts,
    check_origin_shift_contracts, check_reversed_z_contracts, check_webgl2_depth_contracts,
};
pub(crate) use environment_lighting::{
    check_direct_light_shading_contracts, check_environment_ibl_prepare_contracts,
    check_environment_lifecycle_contracts, check_equirectangular_hdr_environment_contracts,
    check_scene_light_contracts,
};
pub(crate) use governance_backend::{
    MILESTONE_CHECKLISTS, check_agent_validation, check_backend_vocabulary,
    check_unit_test_first_governance, contains_scope_term,
};
pub(crate) use platform_contracts::check_m4_platform_contracts;
pub(crate) use release_contracts::{
    REQUIRED_EXAMPLES, REQUIRED_M5_GATE_ARTIFACTS, check_m5_release_contracts,
};
pub(crate) use scene_import::check_m3a_scene_import_contracts;
pub(crate) use shadow_depth::{
    check_depth_prepass_contracts, check_directional_shadow_contracts, check_shadow_map_contracts,
};
