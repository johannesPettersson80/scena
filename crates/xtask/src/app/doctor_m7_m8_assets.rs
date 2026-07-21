mod anisotropy_materials;
mod asset_fetch_evidence;
mod asset_instancing;
mod asset_load_reports;
mod asset_matrix;
mod assets_materials;
mod clearcoat_materials;
mod compressed_asset_proof;
mod dispersion_materials;
mod ergonomics;
mod iridescence_materials;
mod manifest_helpers;
mod material_texture_diagnostics;
mod sheen_materials;
mod state_of_art;
mod texture_baseline;
mod visual_materials;

pub(crate) use asset_fetch_evidence::check_asset_load_test_evidence;
pub(crate) use asset_matrix::{
    binary_render_asset_extension, check_binary_render_asset_contracts,
    check_gltf_asset_matrix_contract, check_tangent_generation_dependency_contracts,
    collect_text_binary_asset_findings, looks_like_text_fixture,
};
pub(crate) use assets_materials::{
    check_c03_texture_contracts, check_c04_deformation_contracts, check_c05_unit_contracts,
    check_c08_animation_basis_contract, check_c09_transactional_reload_contract,
    check_c10_cache_policy_contract, check_c14_gltf_semantic_contract,
    check_c15_marker_transform_contract, check_m8_assets_materials_contracts,
};
pub(crate) use ergonomics::check_m7_ergonomics_contracts;
pub(crate) use manifest_helpers::{
    backtick_values, contains_placeholder, expected_result_is_explicit, first_backtick_value,
    is_local_evidence_path, khronos_manifest_file_hashes, quoted_array_item,
    require_manifest_value,
};
pub(crate) use manifest_helpers::{
    check_manifest_file_hash, derivative_manifest_entries, is_lower_hex_sha256, quoted_assignment,
    quoted_manifest_assignment, require_manifest_u32, sha256_hex, u32_manifest_assignment,
};
pub(crate) use state_of_art::{
    check_document_governance_truth, check_state_of_art_checklist_links,
};
