mod agent_contracts;
mod capability_discovery;
mod cli_ergonomics;
mod contracts;
mod control_flow_allowlist;
mod conversion_contract;
mod execution;
mod feature_discoverability;
mod feature_gated_tests;
mod feature_ownership;
mod name_candidates;
mod recipe_policy;
mod repair_inputs;
mod runner;
mod silent_failure_contracts;
mod transform_grammar;

pub(crate) use agent_contracts::check_agent_contracts;
pub(crate) use capability_discovery::check_a03_live_capability_discovery;
pub(crate) use cli_ergonomics::check_a04_cli_ergonomics;
pub(crate) use contracts::{
    ALLOWED_CONTEXT_TYPES, CATCH_ALL_TYPE_NAMES, CATCH_ALL_TYPE_SUFFIXES,
    MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE, MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE,
    REQUIRED_SOURCE_MODULES, SOURCE_SCOPE_TERMS, STALE_DOC_TERMS,
};
pub(crate) use contracts::{check_cli_output_contracts, require_files};
pub(crate) use conversion_contract::check_a05_scena_convert_contract;
pub(crate) use execution::run_doctor;
pub(crate) use feature_discoverability::check_a09_feature_discoverability;
pub(crate) use feature_gated_tests::{
    check_feature_gated_contract_tests_documented, check_feature_gated_tests_run_in_a_workflow,
};
pub(crate) use feature_ownership::{
    check_feature_ownership_contracts, check_q07_claim_truth_contracts,
};
pub(crate) use name_candidates::check_a07_name_candidates_and_remedies;
pub(crate) use recipe_policy::{
    check_a01_recipe_resource_resolution, check_a02_operator_recipe_roots,
    check_c03_canonical_recipe_command_routing, check_recipe_build_policy_boundary,
};
pub(crate) use repair_inputs::check_a06_repair_and_doctor_inputs;
pub(crate) use runner::{
    REQUIRED_DOCS, check_current_release_document_version, check_no_ignored_release_tests,
    check_test_control_flow_policy, find_env_var_names,
};
pub(crate) use runner::{
    check_cpu_ibl_gap_documented, check_m8_real_asset_dual_lane, check_tests_env_flags_documented,
    check_waterbottle_third_party_reference, repo_root, run_architecture_doctor, run_docs_doctor,
};
pub(crate) use silent_failure_contracts::check_full_review_q06_silent_failure_contracts;
pub(crate) use transform_grammar::check_a08_transform_grammar;
