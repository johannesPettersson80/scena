use crate::app::prelude::*;

pub(crate) fn check_c04_deformation_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSETS-C04";
    let required: &[(&str, &[&str])] = &[
        (
            "src/assets/gltf/meshes.rs",
            &[
                "Vec3Encoding::Position, DataType::I8, false",
                "Vec3Encoding::Position, DataType::U8, false",
                "Vec3Encoding::Position, DataType::I16, false",
                "Vec3Encoding::Position, DataType::U16, false",
                "Vec3Encoding::SignedUnit, DataType::I8, true",
                "Vec3Encoding::SignedUnit, DataType::I16, true",
                "must use FLOAT or normalized signed BYTE/SHORT",
                "GeometryMorphTarget::new_with_semantics",
                "vec![Vec3::ZERO; vertex_count]",
            ],
        ),
        (
            "src/assets/gltf/meshes/skin_influences.rs",
            &[
                "validate_and_normalize",
                "must have a finite non-zero sum",
                "WEIGHTS_{set} vertex {vertex_index}",
            ],
        ),
        (
            "src/assets/gltf/animation.rs",
            &[
                "AnimationSourceClip::imported",
                "chunk_size = targets_per_keyframe",
                "glTF {property} animation output must use",
                "Dimensions::Scalar",
            ],
        ),
        ("src/animation.rs", &["rebind_imported_many"]),
        (
            "src/animation/validation.rs",
            &[
                "validate_imported_source_clip",
                "validate_imported_clip",
                "imported clip must contain at least one channel",
                "imported duration_seconds must be finite and non-negative",
                "translation channel output must use VEC3 values",
            ],
        ),
        ("src/scene/import/types.rs", &["morph_nodes: Vec<NodeKey>"]),
        (
            "src/scene/import/animation_bindings.rs",
            &[
                "rebind_imported_many",
                "AnimationTarget::Weights",
                "record.morph_nodes.clone()",
                "InvalidAnimationClip",
            ],
        ),
        (
            "src/geometry/morph.rs",
            &[
                "tangent_deltas: Option<Vec<Vec3>>",
                "new_with_semantics",
                "morphed_tangents",
                "tangent[0] += delta.x * weight",
            ],
        ),
        (
            "src/render/prepare/primitives.rs",
            &[
                "source.geometry.morphed_tangents(weights)",
                "morphed_tangents",
                ".as_deref()",
                "corner.tangent_handedness",
            ],
        ),
        (
            "src/render/prepare/materials.rs",
            &[
                "tangent_space_normal_from_sample",
                "tangent_space_to_world",
                "tangent_handedness",
                "bitangent",
            ],
        ),
        (
            "docs/assets.md",
            &[
                "Quantized geometry, morphs, animation, and skins",
                "including non-normalized integer",
                "`POSITION` values",
                "one-key clip at",
                "time zero while rejecting empty",
                "finite, non-negative, non-zero-sum skin weights",
            ],
        ),
    ];
    for (relative, needles) in required {
        require_contains(root, findings, RULE, relative, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c04_gltf_deformation_contracts.rs",
        &[
            "quantized_signed_and_unsigned_positions_preserve_vertices_bounds_and_render",
            "quantized_tangent_and_morph_component_matrix_decodes_exact_values",
            "invalid_integer_normal_is_an_error_not_a_default_normal",
            "cubic_spline_weights_preserve_target_width_and_tangent_influence",
            "multi_primitive_weight_channel_fans_out_to_each_renderable_child",
            "morph_targets_preserve_cardinality_and_optional_normal_tangent_semantics",
            "imported_animation_static_policy_accepts_one_key_and_rejects_malformed_clips",
            "skin_weights_are_validated_and_renormalized_for_all_legal_encodings",
        ],
    );

    for (relative, forbidden) in [
        (
            "src/assets/gltf/animation.rs",
            "chunk_size = targets_per_keyframe * stride_factor",
        ),
        (
            "src/assets/gltf/meshes.rs",
            "let weights: Vec<[f32; 4]> = weights.into_f32().collect()",
        ),
        (
            "src/assets/gltf/meshes.rs",
            ".filter_map(|(positions, normals, _tangents)|",
        ),
    ] {
        let Ok(source) = fs::read_to_string(root.join(relative)) else {
            continue;
        };
        if source.contains(forbidden) {
            findings.push(Finding::new(
                RULE,
                format!("{relative} contains forbidden C04 fallback `{forbidden}`"),
            ));
        }
    }
}
