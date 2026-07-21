mod asset_alpha_output;
mod diagnostics_stats_world;
mod material_reflection;
mod movement;
mod primitive_winding;
mod quality;
mod quality_composition;
mod quality_reflection;
mod render_truth;
mod standard_math_prepare;
mod transfer_contract;

pub(crate) use asset_alpha_output::{
    check_asset_api_contracts, check_fxaa_output_contracts, check_output_stage_contracts,
    check_render_alpha_contracts,
};
pub(crate) use diagnostics_stats_world::{
    check_diagnostics_contracts, check_headless_gpu_test_guard_contracts,
    check_render_world_bake_contracts, check_renderer_stats_contracts,
};
pub(crate) use material_reflection::check_material_reflection_quality_contracts;
pub(crate) use movement::check_render_movement_contracts;
pub(crate) use primitive_winding::check_c05_primitive_winding_contract;
pub(crate) use quality::check_render_quality_contracts;
pub(crate) use quality_reflection::check_render_quality_reflection_contracts;
pub(crate) use render_truth::check_renderer_truth_contracts;
pub(crate) use standard_math_prepare::{
    check_area_light_acceptance_honesty, check_particle_prepare_allocation_contract,
    check_prepare_asset_contracts, check_renderer_standard_math_contracts,
};
pub(crate) use transfer_contract::check_c07_target_transfer_contract;
