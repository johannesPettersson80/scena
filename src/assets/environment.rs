use super::AssetPath;
use crate::diagnostics::AssetError;

const DEFAULT_ENVIRONMENT_NAME: &str = "neutral-studio";
pub(super) const DEFAULT_ENVIRONMENT_SOURCE_PATH: &str =
    "tests/assets/environment/neutral-studio.fixture.txt";
const DEFAULT_ENVIRONMENT_SOURCE_SHA256: &str =
    "955af3ed33b2ad3d525ac8c0c1f83ed9c531a4317994eaa501531e5e35b90d13";
const DEFAULT_ENVIRONMENT_LICENSE: &str = "CC0-1.0";
const DEFAULT_ENVIRONMENT_GENERATOR: &str = "xtask generate-default-env-fixture --input tests/assets/environment/neutral-studio.fixture.txt";
const DEFAULT_ENVIRONMENT_CUBEMAP_PATH: &str =
    "tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml";
const DEFAULT_ENVIRONMENT_CUBEMAP_SHA256: &str =
    "41189e81657848c028b0335a86901890f9a48744d9f51a3b5ff19d5b54ef86f8";
const DEFAULT_ENVIRONMENT_BRDF_LUT_PATH: &str =
    "tests/assets/environment/generated/brdf-lut-256.fixture.toml";
const DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256: &str =
    "5d50ac6c5639f1d2344831dc648be932989f81af7a1bd8f2a0f9c94313be2563";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmEnvironmentDelivery {
    Bundled,
    SeparateFetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentSourceKind {
    BundledPreviewFixture,
    EquirectangularHdr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDerivative {
    path: AssetPath,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentDesc {
    name: String,
    source_path: AssetPath,
    source_kind: EnvironmentSourceKind,
    source_dimensions: Option<(u32, u32)>,
    source_sha256: Option<String>,
    preview_irradiance_rgb: Option<[f32; 3]>,
    license: Option<String>,
    generator: Option<String>,
    cubemap_resolution: u32,
    brdf_lut_size: u32,
    wasm_delivery: WasmEnvironmentDelivery,
    derivatives: Vec<EnvironmentDerivative>,
}

impl EnvironmentDesc {
    pub fn neutral_studio() -> Self {
        Self {
            name: DEFAULT_ENVIRONMENT_NAME.to_string(),
            source_path: AssetPath::from(DEFAULT_ENVIRONMENT_SOURCE_PATH),
            source_kind: EnvironmentSourceKind::BundledPreviewFixture,
            source_dimensions: None,
            source_sha256: Some(DEFAULT_ENVIRONMENT_SOURCE_SHA256.to_string()),
            preview_irradiance_rgb: None,
            license: Some(DEFAULT_ENVIRONMENT_LICENSE.to_string()),
            generator: Some(DEFAULT_ENVIRONMENT_GENERATOR.to_string()),
            cubemap_resolution: 256,
            brdf_lut_size: 256,
            wasm_delivery: WasmEnvironmentDelivery::Bundled,
            derivatives: vec![
                EnvironmentDerivative {
                    path: AssetPath::from(DEFAULT_ENVIRONMENT_CUBEMAP_PATH),
                    sha256: DEFAULT_ENVIRONMENT_CUBEMAP_SHA256.to_string(),
                },
                EnvironmentDerivative {
                    path: AssetPath::from(DEFAULT_ENVIRONMENT_BRDF_LUT_PATH),
                    sha256: DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256.to_string(),
                },
            ],
        }
    }

    pub fn from_equirectangular_hdr_path(path: impl Into<AssetPath>) -> Self {
        let path = path.into();
        let source_dimensions = parse_equirectangular_hdr_dimensions(&path);
        Self {
            name: environment_name_from_path(&path).to_string(),
            source_path: path,
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions,
            source_sha256: None,
            preview_irradiance_rgb: None,
            license: None,
            generator: None,
            cubemap_resolution: 0,
            brdf_lut_size: 0,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
        }
    }

    pub(crate) fn from_equirectangular_hdr_bytes(
        path: impl Into<AssetPath>,
        source_bytes: &[u8],
    ) -> Result<Self, AssetError> {
        let path = path.into();
        let (source_dimensions, preview_irradiance_rgb) =
            parse_radiance_hdr_preview(&path, source_bytes)?;
        Ok(Self {
            name: environment_name_from_path(&path).to_string(),
            source_path: path,
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: Some(source_dimensions),
            source_sha256: None,
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            license: None,
            generator: None,
            cubemap_resolution: 0,
            brdf_lut_size: 0,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_path(&self) -> &AssetPath {
        &self.source_path
    }

    pub const fn source_kind(&self) -> EnvironmentSourceKind {
        self.source_kind
    }

    pub const fn source_dimensions(&self) -> Option<(u32, u32)> {
        self.source_dimensions
    }

    pub const fn is_equirectangular_hdr(&self) -> bool {
        matches!(self.source_kind, EnvironmentSourceKind::EquirectangularHdr)
    }

    pub fn source_sha256(&self) -> Option<&str> {
        self.source_sha256.as_deref()
    }

    pub const fn preview_irradiance_rgb(&self) -> Option<[f32; 3]> {
        self.preview_irradiance_rgb
    }

    pub fn license(&self) -> Option<&str> {
        self.license.as_deref()
    }

    pub fn generator(&self) -> Option<&str> {
        self.generator.as_deref()
    }

    pub const fn cubemap_resolution(&self) -> u32 {
        self.cubemap_resolution
    }

    pub const fn brdf_lut_size(&self) -> u32 {
        self.brdf_lut_size
    }

    pub const fn wasm_delivery(&self) -> WasmEnvironmentDelivery {
        self.wasm_delivery
    }

    pub fn derivatives(&self) -> &[EnvironmentDerivative] {
        &self.derivatives
    }
}

impl EnvironmentDerivative {
    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

fn environment_name_from_path(path: &AssetPath) -> &str {
    path.as_str()
        .rsplit('/')
        .next()
        .filter(|name| !name.is_empty())
        .unwrap_or(path.as_str())
}

pub(super) fn is_equirectangular_hdr_path(path: &AssetPath) -> bool {
    path.as_str().to_ascii_lowercase().ends_with(".hdr")
}

fn parse_equirectangular_hdr_dimensions(path: &AssetPath) -> Option<(u32, u32)> {
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

fn parse_radiance_hdr_preview(
    path: &AssetPath,
    source_bytes: &[u8],
) -> Result<((u32, u32), [f32; 3]), AssetError> {
    let Some(header_end) = find_bytes(source_bytes, b"\n\n") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "Radiance HDR header is missing a blank-line terminator".to_string(),
        });
    };
    let resolution_start = header_end + 2;
    let Some(resolution_end_relative) = find_bytes(&source_bytes[resolution_start..], b"\n") else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "Radiance HDR header is missing the resolution line".to_string(),
        });
    };
    let resolution_end = resolution_start + resolution_end_relative;
    let resolution =
        std::str::from_utf8(&source_bytes[resolution_start..resolution_end]).map_err(|error| {
            AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!("Radiance HDR resolution line is not UTF-8: {error}"),
            }
        })?;
    let (width, height) = parse_radiance_resolution(path, resolution)?;
    let pixel_start = resolution_end + 1;
    let pixel_count = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "Radiance HDR dimensions overflow pixel count".to_string(),
        })?;
    let expected_bytes = pixel_count
        .checked_mul(4)
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "Radiance HDR dimensions overflow byte count".to_string(),
        })?;
    let pixel_bytes = source_bytes
        .get(pixel_start..pixel_start + expected_bytes)
        .ok_or_else(|| AssetError::Parse {
            path: path.as_str().to_string(),
            reason: "Radiance HDR fixture is shorter than the declared raw RGBE data".to_string(),
        })?;
    let mut average = [0.0_f32; 3];
    for rgbae in pixel_bytes.chunks_exact(4) {
        let rgb = decode_rgbe(rgbae[0], rgbae[1], rgbae[2], rgbae[3]);
        average[0] += rgb[0];
        average[1] += rgb[1];
        average[2] += rgb[2];
    }
    let inverse_count = (pixel_count as f32).recip();
    average[0] *= inverse_count;
    average[1] *= inverse_count;
    average[2] *= inverse_count;
    Ok(((width, height), average))
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn parse_radiance_resolution(path: &AssetPath, resolution: &str) -> Result<(u32, u32), AssetError> {
    let parts = resolution.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 4 {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("unsupported Radiance HDR resolution line '{resolution}'"),
        });
    }
    let mut width = None;
    let mut height = None;
    for pair in parts.chunks_exact(2) {
        match pair[0] {
            "+X" | "-X" => width = pair[1].parse::<u32>().ok(),
            "+Y" | "-Y" => height = pair[1].parse::<u32>().ok(),
            _ => {}
        }
    }
    let Some(width) = width.filter(|value| *value > 0) else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("Radiance HDR resolution line has invalid width '{resolution}'"),
        });
    };
    let Some(height) = height.filter(|value| *value > 0) else {
        return Err(AssetError::Parse {
            path: path.as_str().to_string(),
            reason: format!("Radiance HDR resolution line has invalid height '{resolution}'"),
        });
    };
    Ok((width, height))
}

fn decode_rgbe(red: u8, green: u8, blue: u8, exponent: u8) -> [f32; 3] {
    if exponent == 0 {
        return [0.0, 0.0, 0.0];
    }
    let scale = 2.0_f32.powi(exponent as i32 - 136);
    [
        (f32::from(red) + 0.5) * scale,
        (f32::from(green) + 0.5) * scale,
        (f32::from(blue) + 0.5) * scale,
    ]
}
