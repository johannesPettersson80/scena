use crate::app::prelude::*;

const RULE: &str = "GLTF-CHECKED-ACCESSOR-READS";
const GLTF_IMPORTER: &str = "src/assets/gltf";
const CHECKED_READER: &str = "src/assets/gltf/typed_accessors.rs";

/// Typed reader calls of the upstream `gltf` crate. They reach
/// `unreachable!()` on an unexpected component type, underflow on `count` 0,
/// and allocate whatever a sparse accessor declares, so importer code must not
/// call them.
const UPSTREAM_READER_CALLS: &[&str] = &[
    ".reader(",
    ".read_positions(",
    ".read_normals(",
    ".read_tangents(",
    ".read_colors(",
    ".read_tex_coords(",
    ".read_joints(",
    ".read_weights(",
    ".read_indices(",
    ".read_morph_targets(",
    ".read_inverse_bind_matrices(",
    ".read_inputs(",
    ".read_outputs(",
];

/// The raw accessor iterator may only be named by the checked reader.
const RAW_ACCESSOR_ITERATORS: &[&str] = &["AccessorIter", "accessor::Iter", "accessor::util"];

/// Every accessor read in the glTF importer goes through the checked reader,
/// which validates the layout and reserves the decoded size from the accessor
/// byte budget. A bypass would bring back the panic and the unbounded
/// allocation that reader exists to prevent.
pub(crate) fn check_gltf_checked_accessor_reads(root: &Path, findings: &mut Vec<Finding>) {
    let importer_files = std::iter::once(PathBuf::from(format!("{GLTF_IMPORTER}.rs")))
        .chain(cached_rust_files_below(root, Path::new(GLTF_IMPORTER)));
    for rel in importer_files {
        let Ok(text) = read_source_to_string(root, &rel) else {
            continue;
        };
        let file = path_to_forward_slash(&rel);
        for (line_index, line) in text.lines().enumerate() {
            let code = line.split("//").next().unwrap_or_default();
            for call in UPSTREAM_READER_CALLS {
                if code.contains(call) {
                    findings.push(Finding::new(
                        RULE,
                        format!(
                            "{file}:{} calls the upstream glTF reader `{call}`; decode accessors with decode_accessor from {CHECKED_READER}",
                            line_index + 1
                        ),
                    ));
                }
            }
            if file == CHECKED_READER {
                continue;
            }
            for iterator in RAW_ACCESSOR_ITERATORS {
                if code.contains(iterator) {
                    findings.push(Finding::new(
                        RULE,
                        format!(
                            "{file}:{} names the raw accessor iterator `{iterator}`; only {CHECKED_READER} may open one",
                            line_index + 1
                        ),
                    ));
                }
            }
        }
    }

    require_contains(
        root,
        findings,
        RULE,
        CHECKED_READER,
        &[
            "pub(super) fn decode_accessor",
            "buffers.charge_accessor_bytes(",
            "declares count 0",
            "declares sparse.count 0",
            "is smaller than its",
            "has neither a bufferView nor sparse storage to read",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/buffers.rs",
        &[
            "pub(super) fn charge_accessor_bytes",
            "exceeds max_accessor_bytes",
            "bytes but declares byteLength",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/gltf/validation.rs",
        &["$.bufferViews[{index}] byte range"],
    );
    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/gltf_import_hardening.rs",
        &[
            "normalized_byte_texcoords_load_and_the_asset_store_stays_usable",
            "unexpected_accessor_layouts_fail_with_errors_that_name_mesh_primitive_and_attribute",
            "animation_and_skin_accessors_with_unexpected_layouts_fail_with_structured_errors",
            "accessors_that_declare_no_elements_fail_with_structured_errors",
            "a_sparse_accessor_with_a_huge_count_fails_before_anything_is_allocated",
            "every_read_of_a_shared_accessor_counts_against_the_budget",
            "derived_vertex_data_counts_against_the_budget",
            "buffer_views_must_lie_inside_their_buffer",
            "embedded_buffers_shorter_than_their_byte_length_are_rejected",
        ],
    );
}
