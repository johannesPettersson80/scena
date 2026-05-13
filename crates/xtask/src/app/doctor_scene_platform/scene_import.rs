use crate::app::prelude::*;
mod evidence;
mod gltf_scene;
mod import_runtime;
mod loading;
mod render_view;
use evidence::check_m3a_evidence_contracts;
use gltf_scene::check_m3a_gltf_scene_contracts;
use import_runtime::check_m3a_import_runtime_contracts;
use loading::check_m3a_loading_contracts;
use render_view::check_m3a_render_view_contracts;

pub(crate) fn check_m3a_scene_import_contracts(root: &Path, findings: &mut Vec<Finding>) {
    check_m3a_loading_contracts(root, findings);
    check_m3a_gltf_scene_contracts(root, findings);
    check_m3a_render_view_contracts(root, findings);
    check_m3a_import_runtime_contracts(root, findings);
    check_m3a_evidence_contracts(root, findings);
}
