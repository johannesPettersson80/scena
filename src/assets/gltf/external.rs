//! Stage C2: External buffer/image URI walks now use the `gltf` crate's
//! typed `Buffer::source()` and `Image::source()` accessors. The crate
//! handles GLB vs JSON detection, base64 data URIs, and bufferView-backed
//! images uniformly.

use ::gltf::Gltf;
use ::gltf::buffer::Source as BufferSource;
use ::gltf::image::Source as ImageSource;

use crate::diagnostics::AssetError;

use super::super::AssetPath;

pub(super) fn external_buffer_paths(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<Vec<(usize, AssetPath)>, AssetError> {
    let gltf = open_gltf(path, bytes)?;
    Ok(gltf
        .document
        .buffers()
        .filter_map(|buffer| match buffer.source() {
            BufferSource::Uri(uri) if !uri.starts_with("data:") => {
                Some((buffer.index(), resolve_relative_path(path, uri)))
            }
            _ => None,
        })
        .collect())
}

pub(super) fn external_image_paths(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<Vec<AssetPath>, AssetError> {
    let gltf = open_gltf(path, bytes)?;
    Ok(gltf
        .document
        .images()
        .filter_map(|image| match image.source() {
            ImageSource::Uri { uri, .. } if !uri.starts_with("data:") => {
                Some(resolve_relative_path(path, uri))
            }
            _ => None,
        })
        .collect())
}

pub(super) fn open_gltf(path: &AssetPath, bytes: &[u8]) -> Result<Gltf, AssetError> {
    super::open_gltf_with_massage(path, bytes)
}

pub(super) fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.starts_with('/') || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(uri);
    };
    AssetPath::from(format!("{directory}/{uri}"))
}
