use crate::app::prelude::*;

pub(crate) fn check_q10_rendered_waterbottle_mutations(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "Q10-RENDERED-WATERBOTTLE-MUTATIONS";
    require_contains(
        root,
        findings,
        RULE,
        "tests/q01_waterbottle_cpu_reference.rs",
        &[
            "render_wrong_material_scene",
            "let wrong_material_frame = render_wrong_material_scene();",
            "set_mesh_material",
            "render_wrong_camera_scene",
            "let wrong_camera_frame = render_wrong_camera_scene();",
            "camera_node",
            "set_transform",
            "render_current",
            "rendered-scene",
            "pipeline_coverage",
            "pbr-neutral-tonemap",
            "srgb8-output",
        ],
    );
    if let Ok(text) = fs::read_to_string(root.join("tests/q01_waterbottle_cpu_reference.rs"))
        && (text.contains("fn wrong_material_mutation(source:")
            || text.contains("fn wrong_camera_mutation(source:"))
    {
        findings.push(Finding::new(
            RULE,
            "wrong-material and wrong-camera WaterBottle mutations must rerender scene state, not edit passing pixels",
        ));
    }
    require_contains(
        root,
        findings,
        RULE,
        "crates/xtask/src/app/release/waterbottle_results.rs",
        &[
            "validate_waterbottle_mutation_provenance",
            "scene-mesh-material-before-prepare",
            "active-camera-transform-before-prepare",
            "does not prove the required {kind} pipeline execution",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "tests/assets/gltf/khronos/WaterBottle/reference_metadata.toml",
        &[
            "rendered-scene mutations",
            "post-hoc-pixel mutation",
            "pipeline_coverage",
        ],
    );
}
