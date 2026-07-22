use crate::app::prelude::*;

pub(super) fn check_texture_baseline_contracts(root: &Path, findings: &mut Vec<Finding>) {
    const RULE: &str = "ASSETS-M8";
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/texture.rs",
        &[
            "validate_texture_source_format",
            "TextureSourceFormat",
            "source_format",
            "decode_png_rgba8",
            "decode_jpeg_rgba8",
            "has_decoded_pixels",
            "decode_missing_pixels_from_bytes",
            "wrap_texture_coordinate",
            "decoded_mip_metadata",
        ],
    );
    require_contains(
        root,
        findings,
        RULE,
        "src/assets/texture_format.rs",
        &[
            "UnsupportedTextureFormat",
            "TextureSourceFormat::Jpeg",
            "validate_texture_source_format",
            "wrap_texture_coordinate",
            "TextureWrap::ClampToEdge",
            "TextureWrap::MirroredRepeat",
        ],
    );
}
