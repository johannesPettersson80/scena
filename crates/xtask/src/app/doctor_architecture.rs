use crate::app::prelude::*;
mod contract;
mod dependency_direction;
mod module_boundaries;
mod public_api;
mod viewer_singletons;
mod xtask_split;

pub(crate) use contract::check_architecture_contract;
pub(crate) use dependency_direction::{
    check_architecture_dependency_direction, check_render_asset_loading_contracts,
    render_asset_loading_patterns,
};
pub(crate) use module_boundaries::check_module_boundaries;
pub(crate) use public_api::{
    check_public_api_ownership, find_public_api_definition_path, forbid_contains_required_path,
    public_api_definition_exists, public_definition_name, public_reexported_type_names,
    public_use_exports_name,
};
pub(crate) use viewer_singletons::{
    check_render_singleton_contracts, check_viewer_facade_contracts, public_struct_names,
};
pub(crate) use xtask_split::{
    check_xtask_module_split, collect_xtask_source_files, contains_xtask_include_macro,
    xtask_cross_module_glob_import, xtask_source_files,
};
