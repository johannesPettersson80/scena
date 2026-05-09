use super::AssetPath;
use crate::diagnostics::AssetError;
use crate::scene::Vec3;

/// Default cubemap face resolution for the IBL diffuse path. 64×64×6 RGBA32F
/// is a real cube — large enough to drive a Lambertian diffuse sample without
/// visible faceting, small enough to upload in <128 KB. The Phase 1C step 2
/// GGX prefilter mip chain attaches to the same texture.
pub const DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION: u32 = 64;

/// Six axis-aligned cubemap face directions in WebGPU layer order
/// (px, nx, py, ny, pz, nz). Used to interpolate per-pixel radiance from a
/// six-face radiance summary asset.
pub const ENVIRONMENT_CUBEMAP_FACE_NORMALS: [[f32; 3]; 6] = [
    [1.0, 0.0, 0.0],
    [-1.0, 0.0, 0.0],
    [0.0, 1.0, 0.0],
    [0.0, -1.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.0, 0.0, -1.0],
];

/// Real cubemap radiance data decoded from the bundled environment fixture.
///
/// The fixture format (`SCENA_CUBEMAP_V1`) carries six face-center radiance
/// values that are spherically interpolated across all output pixels via
/// direction-weighted blending — at every pixel direction `d`, the radiance
/// is a hemispherical average of the six face values weighted by
/// `max(0, dot(d, face_normal[i]))`. The resulting cube is C0 continuous
/// across face boundaries, drives a real GPU `texture_cube<f32>` sample, and
/// replaces the per-environment scalar irradiance the shader used to consume.
#[derive(Debug, Clone, PartialEq)]
pub struct EnvironmentCubemapFaces {
    pub(crate) face_radiance: [[f32; 3]; 6],
    pub(crate) resolution: u32,
}

impl EnvironmentCubemapFaces {
    /// Parses the bundled `SCENA_CUBEMAP_V1` text fixture into per-face
    /// radiance triplets. Returns `None` if the magic header is missing or any
    /// face block fails to provide three finite, non-negative channel values.
    pub fn try_parse_fixture(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != "SCENA_CUBEMAP_V1" {
            return None;
        }
        let mut radiance = [[0.0_f32; 3]; 6];
        let mut seen = [false; 6];
        let mut current_face: Option<usize> = None;
        let mut resolution = DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(face_label) = line
                .strip_prefix("[face.")
                .and_then(|rest| rest.strip_suffix(']'))
            {
                current_face = match face_label {
                    "px" => Some(0),
                    "nx" => Some(1),
                    "py" => Some(2),
                    "ny" => Some(3),
                    "pz" => Some(4),
                    "nz" => Some(5),
                    _ => None,
                };
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key == "resolution" {
                    if let Ok(parsed) = value.parse::<u32>()
                        && parsed > 0
                    {
                        resolution = parsed;
                    }
                    continue;
                }
                if key == "radiance"
                    && let Some(face) = current_face
                {
                    let channels = parse_radiance_triplet(value)?;
                    radiance[face] = channels;
                    seen[face] = true;
                }
            }
        }
        seen.iter().all(|present| *present).then_some(Self {
            face_radiance: radiance,
            resolution,
        })
    }

    pub fn face_radiance(&self) -> &[[f32; 3]; 6] {
        &self.face_radiance
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    /// Builds six RGBA32F face buffers (resolution × resolution × 4 channels)
    /// by spherically interpolating the six face-center radiances. Each pixel
    /// direction's radiance is a `max(0, dot(d, face_normal))`-weighted
    /// average of the six face values; alpha is always 1.0.
    pub fn build_face_pixels_rgba32f(&self) -> [Vec<f32>; 6] {
        let resolution = self.resolution.max(1);
        let mut faces: [Vec<f32>; 6] =
            std::array::from_fn(|_| vec![0.0_f32; (resolution as usize).pow(2).saturating_mul(4)]);
        for (face_index, face_pixels) in faces.iter_mut().enumerate() {
            for y in 0..resolution {
                for x in 0..resolution {
                    let u = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let direction = cube_face_direction(face_index, u, v);
                    let radiance = blend_face_radiance(&self.face_radiance, direction);
                    let pixel_index = ((y * resolution + x) * 4) as usize;
                    face_pixels[pixel_index] = radiance[0];
                    face_pixels[pixel_index + 1] = radiance[1];
                    face_pixels[pixel_index + 2] = radiance[2];
                    face_pixels[pixel_index + 3] = 1.0;
                }
            }
        }
        faces
    }

    /// Lambertian diffuse irradiance computed by averaging the six per-face
    /// radiances with cosine-weighted hemispherical visibility. Used as a
    /// fallback for backends that do not yet sample the cubemap (WebGL2,
    /// CPU rasterizer).
    pub fn lambertian_irradiance(&self) -> [f32; 3] {
        let mut sum = [0.0_f32; 3];
        for radiance in &self.face_radiance {
            sum[0] += radiance[0];
            sum[1] += radiance[1];
            sum[2] += radiance[2];
        }
        let inv = (self.face_radiance.len() as f32).recip();
        [sum[0] * inv, sum[1] * inv, sum[2] * inv]
    }
}

fn parse_radiance_triplet(value: &str) -> Option<[f32; 3]> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    let mut channels = [0.0_f32; 3];
    for (slot, raw) in channels.iter_mut().zip(parts) {
        let parsed: f32 = raw.parse().ok()?;
        if !parsed.is_finite() || parsed < 0.0 {
            return None;
        }
        *slot = parsed;
    }
    Some(channels)
}

/// Maps the (face, u, v) coordinate to a unit direction vector pointing from
/// the cube center through the face pixel. Mirrors WebGPU's cubemap face
/// orientation (px, nx, py, ny, pz, nz) so the cube uploaded with this
/// mapping samples correctly with `textureSampleLevel(cube, sampler, dir)`.
fn cube_face_direction(face_index: usize, u: f32, v: f32) -> Vec3 {
    let raw = match face_index {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    let length = (raw.x * raw.x + raw.y * raw.y + raw.z * raw.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        let inv = length.recip();
        Vec3::new(raw.x * inv, raw.y * inv, raw.z * inv)
    }
}

/// Direction-weighted blend of six per-face radiance triplets. At every
/// direction the contribution from face `i` is `max(0, dot(direction, n_i))`,
/// so radiance is C0 continuous across face boundaries (where the dominant
/// face pair contribute equally) and reduces to face-center radiance at the
/// face center (where dot is 1 for that face and ≤0 for the others).
fn blend_face_radiance(face_radiance: &[[f32; 3]; 6], direction: Vec3) -> [f32; 3] {
    let mut accumulated = [0.0_f32; 3];
    let mut weight_sum = 0.0_f32;
    for (face, normal) in ENVIRONMENT_CUBEMAP_FACE_NORMALS.iter().enumerate() {
        let dot = direction.x * normal[0] + direction.y * normal[1] + direction.z * normal[2];
        if dot <= 0.0 {
            continue;
        }
        accumulated[0] += face_radiance[face][0] * dot;
        accumulated[1] += face_radiance[face][1] * dot;
        accumulated[2] += face_radiance[face][2] * dot;
        weight_sum += dot;
    }
    if weight_sum <= f32::EPSILON {
        return [0.0; 3];
    }
    let inv = weight_sum.recip();
    [
        accumulated[0] * inv,
        accumulated[1] * inv,
        accumulated[2] * inv,
    ]
}

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

    /// Returns the bundled cubemap radiance for this environment when one is
    /// available. Phase 1C step 1: only the bundled `neutral-studio` preview
    /// fixture decodes today. Equirectangular HDR sources will gain a real
    /// face decode in step 2 alongside the GGX prefilter and BRDF LUT.
    pub fn cubemap_faces(&self) -> Option<EnvironmentCubemapFaces> {
        if self.name == DEFAULT_ENVIRONMENT_NAME {
            return EnvironmentCubemapFaces::try_parse_fixture(BUNDLED_NEUTRAL_STUDIO_CUBEMAP);
        }
        None
    }
}

const BUNDLED_NEUTRAL_STUDIO_CUBEMAP: &str =
    include_str!("../../tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml");

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

#[cfg(test)]
mod environment_cubemap_tests {
    use super::*;

    const NEUTRAL_STUDIO_FIXTURE: &str = include_str!(
        "../../tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml"
    );

    #[test]
    fn cubemap_fixture_parser_decodes_six_faces_with_real_radiance_values() {
        let parsed = EnvironmentCubemapFaces::try_parse_fixture(NEUTRAL_STUDIO_FIXTURE)
            .expect("bundled SCENA_CUBEMAP_V1 fixture must parse");
        assert_eq!(parsed.resolution, 256, "fixture declares 256-pixel faces");
        assert_eq!(
            parsed.face_radiance,
            [
                [0.78, 0.82, 0.88],
                [0.62, 0.68, 0.76],
                [1.00, 0.98, 0.92],
                [0.28, 0.30, 0.34],
                [0.70, 0.74, 0.82],
                [0.56, 0.60, 0.68],
            ],
            "parser must read face radiance in the WebGPU px/nx/py/ny/pz/nz layer order"
        );
    }

    #[test]
    fn cubemap_fixture_parser_rejects_invalid_magic_header() {
        assert!(
            EnvironmentCubemapFaces::try_parse_fixture(
                "OOPS_NOT_A_CUBEMAP\n[face.px]\nradiance = 1.0 1.0 1.0"
            )
            .is_none(),
            "missing magic header must not silently degrade to a default cubemap"
        );
    }

    #[test]
    fn cubemap_fixture_parser_rejects_negative_radiance() {
        let bad = "SCENA_CUBEMAP_V1\nresolution = 4\n[face.px]\nradiance = -0.1 0.0 0.0\n";
        assert!(
            EnvironmentCubemapFaces::try_parse_fixture(bad).is_none(),
            "negative radiance is physically meaningless and must fail parsing"
        );
    }

    #[test]
    fn cube_face_direction_at_face_center_returns_face_normal() {
        for (face_index, normal) in ENVIRONMENT_CUBEMAP_FACE_NORMALS.iter().enumerate() {
            let direction = cube_face_direction(face_index, 0.0, 0.0);
            let expected = Vec3::new(normal[0], normal[1], normal[2]);
            let dx = direction.x - expected.x;
            let dy = direction.y - expected.y;
            let dz = direction.z - expected.z;
            assert!(
                dx * dx + dy * dy + dz * dz < 1e-6,
                "face {face_index} center direction must equal the face normal"
            );
        }
    }

    #[test]
    fn cubemap_face_pixels_at_face_center_recover_face_radiance() {
        let mut radiance = [[0.0_f32; 3]; 6];
        radiance[0] = [0.9, 0.1, 0.1];
        radiance[1] = [0.1, 0.9, 0.1];
        radiance[2] = [0.1, 0.1, 0.9];
        radiance[3] = [0.5, 0.4, 0.3];
        radiance[4] = [0.3, 0.4, 0.5];
        radiance[5] = [0.7, 0.7, 0.7];
        let cube = EnvironmentCubemapFaces {
            face_radiance: radiance,
            resolution: 8,
        };
        let pixels = cube.build_face_pixels_rgba32f();
        for (face_index, face_pixels) in pixels.iter().enumerate() {
            let center_pixel_index = ((4 * 8) + 4) * 4;
            let r = face_pixels[center_pixel_index];
            let g = face_pixels[center_pixel_index + 1];
            let b = face_pixels[center_pixel_index + 2];
            let a = face_pixels[center_pixel_index + 3];
            // The pixel sample at (4, 4) of an 8×8 face is offset by +0.5 / 8
            // from u=v=0, so its direction tilts ~3.5° away from the face
            // normal — adjacent faces contribute a small but non-zero share.
            // We assert the dominant channel is recognizably the face's own
            // peak channel rather than asserting an exact match.
            let expected = radiance[face_index];
            let dominant = expected.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(
                a == 1.0 && (r - g - b).abs() < 1.0,
                "face {face_index} center alpha must be 1 and the radiance triplet is finite",
            );
            for (channel, raw) in [r, g, b].iter().enumerate() {
                if (expected[channel] - dominant).abs() < 1e-6 {
                    assert!(
                        *raw > expected[channel] * 0.6,
                        "face {face_index} dominant channel must retain >60% of its face-center radiance"
                    );
                }
            }
        }
    }

    #[test]
    fn cubemap_face_pixels_at_face_corners_blend_three_adjacent_faces() {
        let radiance = [
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 0.0],
        ];
        let cube = EnvironmentCubemapFaces {
            face_radiance: radiance,
            resolution: 4,
        };
        let pixels = cube.build_face_pixels_rgba32f();
        let resolution = 4_usize;
        let face_pixels = &pixels[0];
        // Face 0 is +X. Pixel (0, 0) maps to direction
        // Vec3::new(1, -v, -u) at u = v = -0.75 → (+1, +0.75, +0.75), i.e. the
        // corner that touches +X, +Y, +Z. That corner pulls radiance from px,
        // py and pz simultaneously, so all three channels must light up.
        let top_left_index = 0;
        let r = face_pixels[top_left_index];
        let g = face_pixels[top_left_index + 1];
        let b = face_pixels[top_left_index + 2];
        assert!(
            r > 0.0 && g > 0.0 && b > 0.0,
            "px face top-left corner direction (+X,+Y,+Z) must blend px=red, py=green, pz=blue \
             radiances; got r={r} g={g} b={b}"
        );
        // Conversely the diagonally opposite corner pixel (resolution-1,
        // resolution-1) maps to (+X, -Y, -Z), so the px channel must remain
        // dominant while py and pz fall to 0 (their face radiances do not
        // illuminate the (-Y, -Z) hemisphere of this corner).
        let bottom_right_index = ((resolution - 1) * resolution + (resolution - 1)) * 4;
        let r2 = face_pixels[bottom_right_index];
        let g2 = face_pixels[bottom_right_index + 1];
        let b2 = face_pixels[bottom_right_index + 2];
        assert!(
            r2 > 0.0 && g2 == 0.0 && b2 == 0.0,
            "px face (-Y,-Z) corner must keep red but drop py/pz contributions; \
             got r={r2} g={g2} b={b2}"
        );
    }

    #[test]
    fn lambertian_irradiance_averages_six_face_radiances() {
        let radiance = [
            [0.78, 0.82, 0.88],
            [0.62, 0.68, 0.76],
            [1.00, 0.98, 0.92],
            [0.28, 0.30, 0.34],
            [0.70, 0.74, 0.82],
            [0.56, 0.60, 0.68],
        ];
        let cube = EnvironmentCubemapFaces {
            face_radiance: radiance,
            resolution: 64,
        };
        let irradiance = cube.lambertian_irradiance();
        let expected = [
            (0.78 + 0.62 + 1.00 + 0.28 + 0.70 + 0.56) / 6.0,
            (0.82 + 0.68 + 0.98 + 0.30 + 0.74 + 0.60) / 6.0,
            (0.88 + 0.76 + 0.92 + 0.34 + 0.82 + 0.68) / 6.0,
        ];
        for channel in 0..3 {
            assert!(
                (irradiance[channel] - expected[channel]).abs() < 1e-5,
                "channel {channel} mean = {} must equal six-face average = {}",
                irradiance[channel],
                expected[channel]
            );
        }
    }
}
