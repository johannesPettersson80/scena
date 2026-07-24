use std::fmt;

use crate::diagnostics::AssetError;

impl fmt::Display for AssetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { path } => write!(formatter, "asset was not found: {path}"),
            Self::Io { path, reason } => {
                write!(formatter, "failed to read asset {path}: {reason}")
            }
            Self::PolicyViolation { path, reason, .. } => {
                write!(formatter, "asset {path} violates policy: {reason}")
            }
            Self::Parse { path, reason } => {
                write!(formatter, "failed to parse asset {path}: {reason}")
            }
            Self::InvalidTextureIdentity { identity, reason } => write!(
                formatter,
                "invalid in-memory texture identity {identity:?}: {reason}"
            ),
            Self::InvalidTextureData {
                identity,
                width,
                height,
                expected_elements,
                actual_elements,
                reason,
            } => write!(
                formatter,
                "invalid in-memory texture {identity:?} ({width}x{height}): expected {expected_elements} elements, got {actual_elements}: {reason}"
            ),
            Self::TextureSizeLimit {
                path,
                width,
                height,
                maximum_dimension,
                required_bytes,
                maximum_bytes,
            } => write!(
                formatter,
                "texture {path} dimensions {width}x{height} require {required_bytes} decoded bytes; limits are {maximum_dimension}px per axis and {maximum_bytes} bytes"
            ),
            Self::TextureIdentityCollision { identity } => write!(
                formatter,
                "in-memory texture identity {identity:?} was reused with different content or options"
            ),
            Self::TextureColorSpaceMismatch {
                identity,
                slot,
                expected,
                actual,
            } => write!(
                formatter,
                "in-memory texture {identity:?} uses {actual} color data, but material slot {slot} requires {expected}"
            ),
            Self::MorphWeightWidthMismatch {
                path,
                clip_index,
                channel_index,
                node_index,
                primitive_index,
                expected,
                actual,
            } => write!(
                formatter,
                "asset {path} animation clip {clip_index} channel {channel_index} targets node {node_index} primitive {primitive_index} with {actual} morph weights per sample, but the geometry has {expected} morph targets"
            ),
            Self::UnsupportedRequiredExtension { path, extension } => write!(
                formatter,
                "asset {path} requires unsupported extension {extension}"
            ),
            Self::UnsupportedOptionalExtensionUsed {
                path,
                extension,
                help,
            } => write!(
                formatter,
                "asset {path} uses unsupported optional extension {extension}: {help}"
            ),
            Self::MissingTexture {
                path,
                material_slot,
                texture_index,
                context,
                help,
            } => {
                let material = match (context.material_index, context.material_name.as_deref()) {
                    (Some(index), Some(name)) => format!("material {index} ({name:?})"),
                    (Some(index), None) => format!("material {index}"),
                    (None, Some(name)) => format!("material {name:?}"),
                    (None, None) => "an unknown material".to_owned(),
                };
                let image_source = context.image_source.as_deref().map_or_else(
                    || "unknown image source".to_owned(),
                    |source| source.to_owned(),
                );
                write!(
                    formatter,
                    "asset {path} {material} references unresolved texture index {texture_index} in slot {material_slot} from {image_source}: {reason}. {help}",
                    reason = context.reason,
                )
            }
            Self::UnsupportedTextureFormat { path, help } => write!(
                formatter,
                "texture {path} uses an unsupported format: {help}"
            ),
            Self::Ktx2ColorSpaceMismatch {
                path,
                material_slot,
                dfd,
                help,
            } => write!(
                formatter,
                "KTX2 texture {path} has a DFD color-space mismatch for material slot \
                 {material_slot}: got colorPrimaries={actual_primaries}, \
                 transferFunction={actual_transfer}; expected \
                 colorPrimaries={expected_primaries}, transferFunction={expected_transfer}. \
                 {help}",
                actual_primaries = dfd.actual_primaries,
                actual_transfer = dfd.actual_transfer,
                expected_primaries = dfd.expected_primaries,
                expected_transfer = dfd.expected_transfer,
            ),
            Self::Cancelled { path, help } => {
                write!(formatter, "asset load for {path} was cancelled: {help}")
            }
            Self::UnsupportedEnvironmentFormat { path, help } => write!(
                formatter,
                "environment {path} uses an unsupported format: {help}"
            ),
            Self::ReloadRequiresRetain { path, help } => {
                write!(formatter, "asset {path} cannot be reloaded: {help}")
            }
            Self::GeometryHandleNotFound { geometry } => {
                write!(
                    formatter,
                    "geometry handle {geometry:?} was not found in Assets"
                )
            }
            Self::MaterialHandleNotFound { material } => {
                write!(
                    formatter,
                    "material handle {material:?} was not found in Assets"
                )
            }
            Self::TextureHandleNotFound { texture } => {
                write!(
                    formatter,
                    "texture handle {texture:?} was not found in Assets"
                )
            }
            Self::EnvironmentHandleNotFound { environment } => write!(
                formatter,
                "environment handle {environment:?} was not found in Assets"
            ),
        }
    }
}
