use super::AssetPath;
use crate::diagnostics::AssetError;

pub(super) fn parse_equirectangular_hdr_dimensions(path: &AssetPath) -> Option<(u32, u32)> {
    let stem = path
        .as_str()
        .rsplit('/')
        .next()
        .unwrap_or(path.as_str())
        .strip_suffix(".hdr")?;
    let dimensions = stem.rsplit('_').next()?;
    let (width, height) = dimensions.split_once('x')?;
    let width = width.parse().ok()?;
    let height = height.parse().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

/// Decoded equirectangular HDR pixel grid. Stored as row-major linear RGB
/// floats so the cubemap projection pass can sample by longitude and latitude.
/// Built by the `radiant` crate, which handles both uncompressed RGBE and the
/// RLE-compressed scanline format used by real-world HDRI sources.
#[derive(Debug, Clone)]
pub struct DecodedEquirectangular {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<[f32; 3]>,
}

// `parse_radiance_hdr_preview` was the legacy average-radiance-only entry
// point. `decode_radiance_hdr` keeps the full pixel grid for cubemap
// projection while callers still compute preview irradiance from the pixels.

pub(super) fn decode_radiance_hdr(
    path: &AssetPath,
    source_bytes: &[u8],
) -> Result<DecodedEquirectangular, AssetError> {
    let image =
        radiant::load(std::io::Cursor::new(source_bytes)).map_err(|error| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("Radiance HDR decode failed: {error}"),
        })?;
    let width: u32 = image.width.try_into().map_err(|_| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "Radiance HDR width does not fit in u32".to_string(),
    })?;
    let height: u32 = image.height.try_into().map_err(|_| AssetError::Parse {
        path: path.as_str().to_string(),
        reason: "Radiance HDR height does not fit in u32".to_string(),
    })?;
    let pixels = image
        .data
        .into_iter()
        .map(|rgb| [rgb.r, rgb.g, rgb.b])
        .collect::<Vec<_>>();
    Ok(DecodedEquirectangular {
        width,
        height,
        pixels,
    })
}

// `find_bytes`, `parse_radiance_resolution`, and `decode_rgbe` previously lived
// in the environment module as a hand-rolled Radiance HDR decoder. They were
// removed when scena adopted `radiant`, which properly handles RLE-compressed
// scanlines and the header variants real exporters emit.
