//! Stage C2: External buffer/image URI walks now use the `gltf` crate's
//! typed `Buffer::source()` and `Image::source()` accessors. The crate
//! handles GLB vs JSON detection, base64 data URIs, and bufferView-backed
//! images uniformly.

use std::borrow::Cow;

use ::gltf::Gltf;
use ::gltf::buffer::Source as BufferSource;
use ::gltf::image::Source as ImageSource;

use crate::diagnostics::AssetError;

use super::super::AssetPath;
use super::super::load::GltfImportLimits;

pub(super) fn external_buffer_paths(
    path: &AssetPath,
    bytes: &[u8],
    limits: &GltfImportLimits,
) -> Result<Vec<(usize, AssetPath, usize)>, AssetError> {
    let gltf = open_gltf(path, bytes, limits)?;
    Ok(gltf
        .document
        .buffers()
        .filter_map(|buffer| match buffer.source() {
            BufferSource::Uri(uri) if !uri.starts_with("data:") => Some((
                buffer.index(),
                resolve_relative_path(path, uri),
                buffer.length(),
            )),
            _ => None,
        })
        .collect())
}

pub(super) fn external_image_paths(
    path: &AssetPath,
    bytes: &[u8],
    limits: &GltfImportLimits,
) -> Result<Vec<AssetPath>, AssetError> {
    let gltf = open_gltf(path, bytes, limits)?;
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

pub(super) fn open_gltf(
    path: &AssetPath,
    bytes: &[u8],
    limits: &GltfImportLimits,
) -> Result<Gltf, AssetError> {
    super::open_gltf_with_massage(path, bytes, limits)
}

/// Resolves a glTF buffer or image URI against the path of the asset that
/// references it.
///
/// glTF URIs are RFC 3986 references, so a file called `Box With Spaces.bin`
/// is written as `Box%20With%20Spaces.bin`. When the asset is addressed by a
/// file-system style path the reference is percent-decoded, so the fetcher is
/// asked for the name the file really has. When the asset is addressed by a
/// URL — its path carries a scheme, or the build targets the browser, where
/// every path is fetched as a URL — the reference keeps its encoding: decoding
/// `%23` or `%3F` there would turn part of the file name into a fragment or a
/// query. Data URIs and absolute URLs are returned untouched.
pub(super) fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let base_is_url = cfg!(target_arch = "wasm32") || base.as_str().contains("://");
    let reference = if base_is_url {
        Cow::Borrowed(uri)
    } else {
        percent_decoded(uri)
    };
    if reference.starts_with('/') {
        return AssetPath::from(reference.into_owned());
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(reference.into_owned());
    };
    AssetPath::from(format!("{directory}/{reference}"))
}

/// Percent-decodes a URI reference. Escapes that do not form valid UTF-8
/// cannot name a file through an [`AssetPath`], so such a reference is kept as
/// written and reported by the fetcher as the path it could not find.
fn percent_decoded(uri: &str) -> Cow<'_, str> {
    urlencoding::decode(uri).unwrap_or(Cow::Borrowed(uri))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolved(base: &str, uri: &str) -> String {
        resolve_relative_path(&AssetPath::from(base), uri)
            .as_str()
            .to_string()
    }

    #[test]
    fn file_system_references_are_percent_decoded_before_they_are_joined() {
        assert_eq!(
            resolved("models/Box With Spaces.gltf", "Box%20With%20Spaces.bin"),
            "models/Box With Spaces.bin",
        );
        assert_eq!(
            resolved("models/scene.gltf", "textures/caf%C3%A9%231.png"),
            "models/textures/café#1.png",
        );
        assert_eq!(resolved("scene.gltf", "a%2Bb.bin"), "a+b.bin");
        assert_eq!(
            resolved("models/scene.gltf", "plain.bin"),
            "models/plain.bin"
        );
        assert_eq!(
            resolved("models/scene.gltf", "100%25.bin"),
            "models/100%.bin"
        );
    }

    #[test]
    fn a_plus_sign_is_a_literal_character_and_not_an_encoded_space() {
        assert_eq!(resolved("models/scene.gltf", "a+b.bin"), "models/a+b.bin");
    }

    #[test]
    fn references_that_do_not_decode_to_utf8_are_kept_as_written() {
        assert_eq!(resolved("models/scene.gltf", "%FF.bin"), "models/%FF.bin");
        assert_eq!(
            resolved("models/scene.gltf", "50%.bin"),
            "models/50%.bin",
            "a stray percent sign is not an escape",
        );
    }

    #[test]
    fn url_addressed_assets_keep_the_reference_encoding() {
        assert_eq!(
            resolved(
                "https://example.invalid/models/scene.gltf",
                "Box%20With%20Spaces%23.bin"
            ),
            "https://example.invalid/models/Box%20With%20Spaces%23.bin",
        );
    }

    #[test]
    fn data_uris_and_absolute_urls_are_returned_untouched() {
        let data = "data:application/octet-stream;base64,AAAA%20";
        assert_eq!(resolved("models/scene.gltf", data), data);
        let absolute = "https://example.invalid/a%20b.bin";
        assert_eq!(resolved("models/scene.gltf", absolute), absolute);
        assert_eq!(
            resolved("models/scene.gltf", "/abs/a%20b.bin"),
            "/abs/a b.bin"
        );
    }
}
