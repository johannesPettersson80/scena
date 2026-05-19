use crate::app::prelude::*;

pub(super) fn check_m8_instancing_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/assets/gltf/instancing.rs",
        &[
            "EXT_mesh_gpu_instancing",
            "parse_node_instance_transforms",
            "TRANSLATION",
            "ROTATION",
            "SCALE",
            "quat_from_xyzw",
        ],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/scene/import.rs",
        &["source_node.instance_transforms()", "instanced_bounds"],
    );
    require_contains(
        root,
        findings,
        "ASSETS-M8",
        "src/scene/import/instancing.rs",
        &[
            "add_import_instance_set",
            "NodeKind::InstanceSet",
            "push_import_instances",
            "transform_aabb",
        ],
    );
}
