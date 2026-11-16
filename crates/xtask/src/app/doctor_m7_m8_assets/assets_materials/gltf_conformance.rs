use crate::app::prelude::*;

const RULE: &str = "GLTF-IMPORT-CONFORMANCE";

/// Source text that implements a glTF construct the importer once dropped,
/// altered, or rejected. Losing one of these brings the defect back.
const REQUIRED_SOURCE: &[(&str, &[&str])] = &[
    (
        // Percent-encoded buffer and image URIs name the decoded file.
        "src/assets/gltf/external.rs",
        &[
            "fn percent_decoded",
            "urlencoding::decode(uri)",
            "cfg!(target_arch = \"wasm32\") || base.as_str().contains(\"://\")",
        ],
    ),
    (
        // A node may carry a mesh and a KHR_lights_punctual light at once.
        "src/scene/import/nodes.rs",
        &[
            "fn add_imported_light",
            "&& let Some(light) = source_node.light()",
            "self.add_imported_light(node, light.light(), Transform::IDENTITY)",
        ],
    ),
    (
        // Morphed normals are normalized once, after every target.
        "src/geometry/morph.rs",
        &[
            "if normals_morphed {",
            "normalize_or(vertex.normal, authored.normal)",
        ],
    ),
    (
        // Strips, fans, and loops are rewritten as lists.
        "src/assets/gltf/meshes/primitive_modes.rs",
        &[
            "pub(super) fn list_indices",
            "Mode::TriangleStrip => (\"TRIANGLE_STRIP\", 3, 3)",
            "Mode::TriangleFan => (\"TRIANGLE_FAN\", 3, 3)",
            "Mode::LineStrip => (\"LINE_STRIP\", 2, 2)",
            "Mode::LineLoop => (\"LINE_LOOP\", 2, 2)",
            "if triangle % 2 == 0 { [1, 2] } else { [2, 1] }",
            "POINTS is not supported",
        ],
    ),
    (
        "src/assets/gltf/meshes.rs",
        &["primitive_modes::list_indices(source, primitive.mode(), indices)"],
    ),
    (
        // Asset GC roots variant materials and the textures of every
        // retained material.
        "src/assets/gc.rs",
        &[
            "mesh.material_variant_bindings()",
            ".chain(&storage.user_created_materials)",
        ],
    ),
    (
        // A handle returned by Assets::load_texture is caller-owned.
        "src/assets/texture_fetch.rs",
        &["storage.user_created_textures.insert(handle);"],
    ),
    (
        "docs/assets.md",
        &[
            "Buffer and image URIs are RFC 3986 references",
            "### Primitive modes",
            "### Lights",
            "normalized once after the sum",
            "every material a `KHR_materials_variants` binding of that mesh can select",
        ],
    ),
];

/// Tests that fail when one of the constructs above stops importing as the
/// glTF specification describes it.
const REQUIRED_TESTS: &[(&str, &[&str])] = &[
    (
        "tests/gltf_import_hardening.rs",
        &[
            "percent_encoded_external_uris_resolve_to_the_decoded_file_names",
            "external_uris_stay_encoded_when_the_scene_path_is_a_url",
        ],
    ),
    (
        "tests/gltf_import_conformance.rs",
        &[
            "a_node_with_a_mesh_and_a_light_instantiates_both",
            "triangle_strips_and_fans_import_as_triangle_lists",
            "strip_winding_alternation_gives_every_computed_normal_the_same_side",
            "strips_drop_the_degenerate_triangles_that_stitch_them_together",
            "line_strips_and_loops_import_as_line_lists",
            "strips_fans_and_loops_that_are_too_short_fail_with_a_structured_error",
            "point_primitives_are_rejected_with_a_structured_error",
            "expanded_strip_indices_count_against_the_accessor_byte_budget",
        ],
    ),
    (
        "tests/c04_gltf_deformation_contracts.rs",
        &["a_light_on_a_multi_primitive_morph_node_is_not_a_morph_target"],
    ),
    (
        "src/geometry/morph.rs",
        &["morph_normals_sum_every_weighted_delta_before_normalizing_once"],
    ),
    (
        "tests/m8_stale_handle_proof.rs",
        &[
            "m8_release_unreferenced_retains_materials_reachable_only_through_material_variants",
            "m8_release_unreferenced_with_scene_roots_retains_variant_materials_of_the_roots",
            "m8_release_unreferenced_retains_textures_of_user_created_materials",
            "m8_release_unreferenced_retains_loaded_textures_that_no_material_uses_yet",
            "m8_release_unreferenced_retains_scene_textures_that_a_user_material_still_samples",
        ],
    ),
];

/// The glTF constructs the importer handles as the specification describes
/// them stay implemented, documented, and tested.
pub(crate) fn check_gltf_import_conformance(root: &Path, findings: &mut Vec<Finding>) {
    for (relative, needles) in REQUIRED_SOURCE {
        require_contains(root, findings, RULE, relative, needles);
    }
    for (relative, names) in REQUIRED_TESTS {
        require_rust_test_functions(root, findings, RULE, relative, names);
    }
    // The mode match used to reject everything but TRIANGLES and LINES.
    forbid_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/meshes.rs",
        &["unsupported glTF primitive mode"],
    );
}
