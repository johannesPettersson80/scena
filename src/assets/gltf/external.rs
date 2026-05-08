use serde_json::Value as JsonValue;

use crate::diagnostics::AssetError;

use super::super::AssetPath;
use super::glb::{is_glb, parse_glb};

pub(super) fn external_buffer_paths(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<Vec<(usize, AssetPath)>, AssetError> {
    let json = parse_gltf_json(path, bytes)?;
    Ok(json
        .get("buffers")
        .and_then(JsonValue::as_array)
        .map(|buffers| {
            buffers
                .iter()
                .enumerate()
                .filter_map(|(index, buffer)| {
                    let uri = buffer.get("uri").and_then(JsonValue::as_str)?;
                    (!uri.starts_with("data:")).then(|| (index, resolve_relative_path(path, uri)))
                })
                .collect()
        })
        .unwrap_or_default())
}

pub(super) fn external_image_paths(
    path: &AssetPath,
    bytes: &[u8],
) -> Result<Vec<AssetPath>, AssetError> {
    let json = parse_gltf_json(path, bytes)?;
    Ok(json
        .get("images")
        .and_then(JsonValue::as_array)
        .map(|images| {
            images
                .iter()
                .filter_map(|image| {
                    let uri = image.get("uri").and_then(JsonValue::as_str)?;
                    (!uri.starts_with("data:")).then(|| resolve_relative_path(path, uri))
                })
                .collect()
        })
        .unwrap_or_default())
}

fn parse_gltf_json(path: &AssetPath, bytes: &[u8]) -> Result<JsonValue, AssetError> {
    let json = if is_glb(bytes) {
        parse_glb(path, bytes)?.0
    } else {
        std::str::from_utf8(bytes)
            .map_err(|error| AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("expected UTF-8 glTF JSON source: {error}"),
            })?
            .to_string()
    };
    serde_json::from_str(&json).map_err(|error| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: error.to_string(),
    })
}

fn resolve_relative_path(base: &AssetPath, uri: &str) -> AssetPath {
    if uri.starts_with("data:") || uri.starts_with('/') || uri.contains("://") {
        return AssetPath::from(uri);
    }
    let Some((directory, _file)) = base.as_str().rsplit_once('/') else {
        return AssetPath::from(uri);
    };
    AssetPath::from(format!("{directory}/{uri}"))
}
