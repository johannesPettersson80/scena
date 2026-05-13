use crate::app::prelude::*;
mod camera_shader;
mod capability_claims;
mod connectors;
mod materials_depth;
mod webgl2;
use camera_shader::check_renderer_truth_camera_shader_contracts;
use capability_claims::check_renderer_truth_capability_claim_contracts;
use connectors::check_renderer_truth_connector_contracts;
use materials_depth::check_renderer_truth_material_depth_contracts;
use webgl2::check_renderer_truth_webgl2_contracts;

pub(crate) fn check_renderer_truth_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_renderer_truth_camera_shader_contracts(root, findings);
    check_renderer_truth_material_depth_contracts(root, findings);
    check_renderer_truth_webgl2_contracts(root, findings);
    check_renderer_truth_connector_contracts(root, findings);
    check_renderer_truth_capability_claim_contracts(root, findings);
}
