mod antialiasing_effect;
mod browser_probe;
mod ci_release_lanes;
mod ci_release_policy;
mod cpu_webgl2_parity;
mod feature_specific_oracles;
mod fixture_metadata;
mod performance_truth;
mod publish_fail_closed;
mod q03_local_structure;
mod q05_effect_footprints;
mod q06_required_gpu_lanes;
mod q08_physical_parity;
mod q09_adapter_expectations;
mod q10_rendered_mutations;
mod q11_reference_stability;
mod q12_semantic_doctor;
mod release_readiness_contract;
mod required_webgpu_parity;
mod round_e_materials;
mod waterbottle_cpu;
mod workflow_dependencies;

pub(crate) use antialiasing_effect::check_q07_antialiasing_effect_contract;
pub(crate) use browser_probe::check_m6_browser_renderer_probe;
pub(crate) use ci_release_lanes::{
    check_ci_attestation_contracts, check_m9_ci_release_lanes, check_m10_claim_audit_contract,
    require_contains_in_xtask_app_tree,
};
pub(crate) use cpu_webgl2_parity::check_q04_cpu_webgl2_parity_contracts;
pub(crate) use feature_specific_oracles::check_feature_specific_visual_oracles;
pub(crate) use fixture_metadata::{
    check_default_environment_derivative_payload, check_default_environment_manifest,
    check_m1_browser_rendered_output, check_m2_browser_rendered_output,
    check_m2_visual_fixture_metadata, check_ndc_smoke_fixture_classification,
    check_visual_fixture_metadata, fixture_block,
};
pub(crate) use performance_truth::{
    check_pf00_performance_truth_contracts, check_pf03_pf05_hot_path_contracts,
    check_pf06_spatial_acceleration_contracts, check_pf07_pf08_cpu_prepare_contracts,
    check_pf09_parallel_work_contracts, check_pf10_hot_path_contracts,
};
pub(crate) use publish_fail_closed::{
    check_release_publish_dry_run_helper, check_release_readiness_ci_fail_closed,
    jobs_with_continue_on_error_release_readiness,
};
pub(crate) use q03_local_structure::check_q03_m2_local_structure;
pub(crate) use q05_effect_footprints::check_q05_effect_footprint_contracts;
pub(crate) use q06_required_gpu_lanes::check_q06_required_gpu_lane_contracts;
pub(crate) use q08_physical_parity::check_q08_required_physical_parity;
pub(crate) use q09_adapter_expectations::check_q09_structured_adapter_expectations;
pub(crate) use q10_rendered_mutations::check_q10_rendered_waterbottle_mutations;
pub(crate) use q11_reference_stability::check_q11_reference_stability;
pub(crate) use q12_semantic_doctor::check_q12_semantic_doctor_contracts;
pub(crate) use release_readiness_contract::check_c04_release_readiness_contract;
pub(crate) use required_webgpu_parity::check_q01_required_webgpu_pixel_parity;
pub(crate) use required_webgpu_parity::check_q04_browser_evidence_classification;
pub(crate) use round_e_materials::check_q02_round_e_material_proof;
pub(crate) use waterbottle_cpu::check_q01_waterbottle_cpu_proof;
pub(crate) use workflow_dependencies::check_workflow_action_pins;
