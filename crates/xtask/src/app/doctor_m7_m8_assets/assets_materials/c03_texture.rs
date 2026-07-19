use crate::app::prelude::*;

pub(crate) fn check_c03_texture_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSETS-C03";
    let required: &[(&str, &[&str])] = &[
        (
            "Cargo.toml",
            &[
                "image = { version = \"0.25\"",
                "features = [\"png\", \"jpeg\", \"webp\"]",
            ],
        ),
        (
            "src/assets/gltf/textures.rs",
            &[
                "embedded_image_path",
                "sha256_hex(bytes)",
                "memory:image-sha256-",
                "validate_ktx2_material_color_space",
                "material_slot",
            ],
        ),
        (
            "src/assets/texture.rs",
            &[
                "decode_webp_rgba8",
                "TextureSourceFormat::Webp => decode_webp_rgba8",
                "texture cache identity collision",
                "incoming_provenance",
                "has_source_payload",
            ],
        ),
        (
            "src/assets/texture_image_decode.rs",
            &[
                "decode_webp_rgba8",
                "image::ImageFormat::WebP",
                "IMAGE_DECODE_MAX_DIMENSION",
                "IMAGE_DECODE_MAX_ALLOC_BYTES",
            ],
        ),
        (
            "src/assets.rs",
            &[
                "texture_format_has_cpu_decoder",
                "TextureSourceFormat::Webp",
            ],
        ),
        (
            "src/assets/texture_ktx2.rs",
            &[
                "validate_ktx2_material_color_space",
                "ColorPrimaries::BT709",
                "TransferFunction::SRGB",
                "TransferFunction::Linear",
                "expected_primaries",
                "actual_primaries",
                "material_slot",
                "Repair:",
            ],
        ),
        (
            "src/diagnostics.rs",
            &[
                "Ktx2ColorSpaceMismatch",
                "material_slot",
                "expected_primaries",
                "expected_transfer",
                "actual_primaries",
                "actual_transfer",
            ],
        ),
        (
            "src/diagnostics/display/asset.rs",
            &["Ktx2ColorSpaceMismatch", "material slot"],
        ),
        (
            "src/diagnostics/help.rs",
            &["Ktx2ColorSpaceMismatch", "help"],
        ),
        (
            "src/assets/doctor.rs",
            &[
                "Ktx2ColorSpaceMismatch",
                "ktx2_color_space_mismatch",
                "KHR_texture_basisu",
            ],
        ),
        (
            "docs/assets.md",
            &[
                "PNG, JPEG, and WebP image paths decode natively",
                "content-addressed in-memory path",
                "`EXT_texture_webp` texture-source rebinding remains deferred",
            ],
        ),
        (
            "docs/feature-flags.md",
            &[
                "PNG, JPEG, and WebP decoding is available",
                "baseline native image paths",
                "KTX2/Basis remains optional",
            ],
        ),
        (
            "docs/checklists/next-release-easy-use-and-state-of-the-art.md",
            &[
                "PNG/JPEG/WebP paths",
                "EXT_texture_webp` source rebinding remains a separate",
                "deferred capability even though plain WebP bytes decode natively",
            ],
        ),
    ];
    for (rel, needles) in required {
        require_contains(root, findings, RULE, rel, needles);
    }

    require_rust_test_functions(
        root,
        findings,
        RULE,
        "tests/c03_texture_identity.rs",
        &[
            "c03_structured_texture_diagnostic_keeps_asset_error_compact",
            "c03_distinct_glb_embedded_images_have_distinct_identity_pixels_provenance_and_output",
            "c03_embedded_basis_fallbacks_are_namespaced_and_same_asset_still_deduplicates",
            "c03_native_webp_decodes_real_pixels_and_renders_the_texture",
            "c03_embedded_basis_images_are_content_namespaced_and_same_asset_deduplicates",
            "c03_ktx2_accepts_compliant_color_and_non_color_dfd_contracts",
            "c03_ktx2_mismatch_diagnostic_names_slot_dfd_expected_values_and_repair",
        ],
    );

    for (rel, forbidden) in [
        (
            "src/assets/texture.rs",
            "TextureSourceFormat::Webp => Ok(None)",
        ),
        ("src/assets/gltf/textures.rs", "memory:image-{}"),
    ] {
        let Ok(source) = fs::read_to_string(root.join(rel)) else {
            continue;
        };
        if source.contains(forbidden) {
            findings.push(Finding::new(
                RULE,
                format!("{rel} contains forbidden C03 fallback `{forbidden}`"),
            ));
        }
    }
}
