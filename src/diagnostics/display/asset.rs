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
                help,
            } => write!(
                formatter,
                "asset {path} references missing texture index {texture_index} in material slot {material_slot}: {help}"
            ),
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
