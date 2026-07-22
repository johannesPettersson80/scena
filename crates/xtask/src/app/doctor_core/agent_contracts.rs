use std::path::Path;

use crate::app::core::Finding;

use super::{
    check_a03_live_capability_discovery, check_a04_cli_ergonomics,
    check_a05_scena_convert_contract, check_a06_repair_and_doctor_inputs,
    check_a07_name_candidates_and_remedies, check_a08_transform_grammar,
    check_a09_feature_discoverability,
};

pub(crate) fn check_agent_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_a03_live_capability_discovery(root, findings);
    check_a04_cli_ergonomics(root, findings);
    check_a05_scena_convert_contract(root, findings);
    check_a06_repair_and_doctor_inputs(root, findings);
    check_a07_name_candidates_and_remedies(root, findings);
    check_a08_transform_grammar(root, findings);
    check_a09_feature_discoverability(root, findings);
}
