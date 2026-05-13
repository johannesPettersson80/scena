mod asset_alpha_output;
mod diagnostics_stats_world;
mod render_truth;
mod standard_math_prepare;

pub(crate) use asset_alpha_output::{
    check_asset_api_contracts, check_fxaa_output_contracts, check_output_stage_contracts,
    check_render_alpha_contracts,
};
pub(crate) use diagnostics_stats_world::{
    check_diagnostics_contracts, check_render_world_bake_contracts, check_renderer_stats_contracts,
};
pub(crate) use render_truth::check_renderer_truth_contracts;
pub(crate) use standard_math_prepare::{
    check_prepare_asset_contracts, check_renderer_standard_math_contracts,
};
