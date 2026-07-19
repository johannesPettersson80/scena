mod contracts;
mod control_flow_allowlist;
mod feature_gated_tests;
mod feature_ownership;
mod recipe_policy;
mod runner;

pub(crate) use contracts::{
    ALLOWED_CONTEXT_TYPES, CATCH_ALL_TYPE_NAMES, CATCH_ALL_TYPE_SUFFIXES,
    MAX_SIGNIFICANT_LINES_PER_SOURCE_MODULE, MAX_SIGNIFICANT_LINES_PER_XTASK_MODULE,
    REQUIRED_SOURCE_MODULES, SOURCE_SCOPE_TERMS, STALE_DOC_TERMS,
};
pub(crate) use contracts::{check_cli_output_contracts, require_files};
pub(crate) use feature_gated_tests::check_feature_gated_contract_tests_documented;
pub(crate) use feature_ownership::{
    check_feature_ownership_contracts, check_q07_claim_truth_contracts,
};
pub(crate) use recipe_policy::check_recipe_build_policy_boundary;
pub(crate) use runner::{
    REQUIRED_DOCS, check_no_ignored_release_tests, check_test_control_flow_policy,
    find_env_var_names,
};
pub(crate) use runner::{
    check_cpu_ibl_gap_documented, check_m8_real_asset_dual_lane, check_tests_env_flags_documented,
    check_waterbottle_third_party_reference, repo_root, run_architecture_doctor, run_docs_doctor,
    run_doctor,
};
