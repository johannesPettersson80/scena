use std::fs;
use std::path::Path;

use image::{GrayImage, RgbaImage};
use sha2::{Digest, Sha256};

use super::{
    PhotographicMaterialPackError, PhotographicMaterialPackMapRoleV1, PhotographicMaterialPackMapV1,
};

pub(super) fn decode_rgba(
    bytes: &[u8],
    role: &'static str,
) -> Result<RgbaImage, PhotographicMaterialPackError> {
    image::load_from_memory(bytes)
        .map(|image| image.to_rgba8())
        .map_err(|error| PhotographicMaterialPackError::ImageDecode {
            role,
            reason: error.to_string(),
        })
}

pub(super) fn decode_luma(
    bytes: &[u8],
    role: &'static str,
) -> Result<GrayImage, PhotographicMaterialPackError> {
    image::load_from_memory(bytes)
        .map(|image| image.to_luma8())
        .map_err(|error| PhotographicMaterialPackError::ImageDecode {
            role,
            reason: error.to_string(),
        })
}

pub(super) fn require_dimensions(
    role: &'static str,
    expected: (u32, u32),
    actual: (u32, u32),
) -> Result<(), PhotographicMaterialPackError> {
    if expected == actual {
        Ok(())
    } else {
        Err(PhotographicMaterialPackError::DimensionMismatch {
            role,
            expected: [expected.0, expected.1],
            actual: [actual.0, actual.1],
        })
    }
}

pub(super) fn output_map(
    output_dir: &Path,
    role: PhotographicMaterialPackMapRoleV1,
    path: &str,
    color_space: &str,
    dimensions: (u32, u32),
) -> Result<PhotographicMaterialPackMapV1, PhotographicMaterialPackError> {
    let source_path = output_dir.join(path);
    let bytes = fs::read(&source_path).map_err(|error| io_error(&source_path, error))?;
    Ok(PhotographicMaterialPackMapV1 {
        role,
        path: path.to_string(),
        color_space: color_space.to_string(),
        sha256: sha256_hex(&bytes),
        width: dimensions.0,
        height: dimensions.1,
    })
}

pub(super) fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut encoded = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    encoded
}

pub(super) fn io_error(path: &Path, error: std::io::Error) -> PhotographicMaterialPackError {
    PhotographicMaterialPackError::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}

pub(super) fn image_write_error(
    path: &Path,
    error: image::ImageError,
) -> PhotographicMaterialPackError {
    PhotographicMaterialPackError::Io {
        path: path.to_path_buf(),
        reason: error.to_string(),
    }
}
