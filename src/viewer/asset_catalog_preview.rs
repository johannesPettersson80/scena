use std::error::Error as StdError;
use std::fmt;

use crate::assets::AssetCatalogAssetV1;
use crate::capture::fnv1a64_hex;
use crate::material::Color;

use super::{ViewerPngError, headless_gltf_viewer};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetCatalogPreviewPng {
    pub asset_id: String,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub png_fnv1a64: String,
    pub png_bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AssetCatalogPreviewError {
    MissingPreview,
    UnsupportedPreviewKind {
        kind: String,
    },
    InvalidGeneratedPreviewSize {
        width: Option<u32>,
        height: Option<u32>,
    },
    Render {
        reason: String,
    },
}

pub async fn render_asset_catalog_preview_png(
    asset: &AssetCatalogAssetV1,
) -> Result<AssetCatalogPreviewPng, AssetCatalogPreviewError> {
    let preview = asset
        .preview
        .as_ref()
        .ok_or(AssetCatalogPreviewError::MissingPreview)?;
    if preview.kind != "generated" {
        return Err(AssetCatalogPreviewError::UnsupportedPreviewKind {
            kind: preview.kind.clone(),
        });
    }
    let (Some(width), Some(height)) = (
        positive_dimension(preview.width),
        positive_dimension(preview.height),
    ) else {
        return Err(AssetCatalogPreviewError::InvalidGeneratedPreviewSize {
            width: preview.width,
            height: preview.height,
        });
    };

    let mut builder = headless_gltf_viewer(asset.source.as_str())
        .size(width, height)
        .with_default_light();
    if let Some(background) = preview.background {
        builder = builder.with_background_color(Color::from_linear_rgba(
            background[0],
            background[1],
            background[2],
            background[3],
        ));
    }

    let png_bytes = builder
        .render_png_bytes()
        .await
        .map_err(AssetCatalogPreviewError::from)?;
    let png_fnv1a64 = fnv1a64_hex(&png_bytes);
    Ok(AssetCatalogPreviewPng {
        asset_id: asset.id.clone(),
        source: asset.source.clone(),
        width,
        height,
        png_fnv1a64,
        png_bytes,
    })
}

fn positive_dimension(value: Option<u32>) -> Option<u32> {
    value.filter(|dimension| *dimension > 0)
}

impl fmt::Display for AssetCatalogPreviewError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPreview => {
                write!(formatter, "catalog asset does not declare preview metadata")
            }
            Self::UnsupportedPreviewKind { kind } => write!(
                formatter,
                "catalog preview kind '{kind}' cannot be generated; use kind 'generated'"
            ),
            Self::InvalidGeneratedPreviewSize { width, height } => write!(
                formatter,
                "generated catalog preview requires positive width and height; got width={width:?}, height={height:?}"
            ),
            Self::Render { reason } => write!(formatter, "catalog preview render failed: {reason}"),
        }
    }
}

impl StdError for AssetCatalogPreviewError {}

impl From<ViewerPngError> for AssetCatalogPreviewError {
    fn from(error: ViewerPngError) -> Self {
        Self::Render {
            reason: error.to_string(),
        }
    }
}
