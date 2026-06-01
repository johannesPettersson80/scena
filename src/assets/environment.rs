use super::environment_hdr::{
    DecodedEquirectangular, decode_radiance_hdr, parse_equirectangular_hdr_dimensions,
};
use super::environment_sidecar::{
    EnvironmentPrefilterSidecar, EnvironmentSidecarProfile, sha256_hex,
};
use super::{AssetDerivative, AssetPath, AssetProvenance};
use crate::diagnostics::AssetError;
use crate::scene::Vec3;

/// Default cubemap face resolution for HDR IBL. 256×256 faces preserve enough
/// room-scale contrast for smooth-metal specular reflections while staying
/// small enough for the demo/browser prefilter budget.
pub const DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION: u32 = 256;
/// BRDF LUT default resolution for environment lighting. Matches the
/// 64×64 grid built by `build_brdf_lut`.
pub const DEFAULT_ENVIRONMENT_BRDF_LUT_SIZE: u32 = 64;

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
    /// Optional per-pixel radiance for each of the 6 cubemap faces. When
    /// `Some`, `build_face_pixels_rgba32f` returns these grids directly
    /// (preserving the equirectangular HDR's per-pixel detail); when
    /// `None`, the path falls back to interpolating the 6 face-centre
    /// summaries via `blend_face_radiance`.
    pub(crate) face_pixels: Option<[Vec<[f32; 3]>; 6]>,
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
            face_pixels: None,
        })
    }

    /// Project an equirectangular HDR into 6 cubemap face grids at
    /// `face_resolution` per side. For each face pixel, derives the world
    /// direction, converts to spherical (longitude, latitude), and samples
    /// the equirectangular HDR via bilinear filtering. The resulting cubemap
    /// preserves real per-pixel radiance — the prefiltered specular pass
    /// downstream then produces the bright sheen/reflections that pure
    /// face-centre interpolation cannot.
    pub fn from_equirectangular(
        equirect: &DecodedEquirectangular,
        face_resolution: u32,
    ) -> Option<Self> {
        if equirect.width == 0 || equirect.height == 0 || face_resolution == 0 {
            return None;
        }
        let resolution = face_resolution;
        let face_pixel_count = (resolution as usize).pow(2);
        let mut faces: [Vec<[f32; 3]>; 6] =
            std::array::from_fn(|_| vec![[0.0, 0.0, 0.0]; face_pixel_count]);
        let mut face_radiance = [[0.0_f32; 3]; 6];
        for (face_index, face) in faces.iter_mut().enumerate() {
            let mut sum = [0.0_f64; 3];
            for y in 0..resolution {
                for x in 0..resolution {
                    let u = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let direction = cube_face_direction(face_index, u, v);
                    let sample = sample_equirectangular(equirect, direction);
                    let pixel_index = (y * resolution + x) as usize;
                    face[pixel_index] = sample;
                    sum[0] += f64::from(sample[0]);
                    sum[1] += f64::from(sample[1]);
                    sum[2] += f64::from(sample[2]);
                }
            }
            let inv = (face_pixel_count as f64).recip();
            face_radiance[face_index] = [
                (sum[0] * inv) as f32,
                (sum[1] * inv) as f32,
                (sum[2] * inv) as f32,
            ];
        }
        Some(Self {
            face_radiance,
            resolution,
            face_pixels: Some(faces),
        })
    }

    pub fn face_radiance(&self) -> &[[f32; 3]; 6] {
        &self.face_radiance
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    /// Builds six RGBA32F face buffers (resolution × resolution × 4 channels).
    /// When the cubemap carries real per-pixel radiance (set by
    /// `from_equirectangular`), expand those into the RGBA32F shape directly.
    /// Otherwise, fall back to spherically-interpolating the six face-centre
    /// radiances: each pixel direction's radiance is a
    /// `max(0, dot(d, face_normal))`-weighted average of the face values.
    /// Alpha is always 1.0.
    pub fn build_face_pixels_rgba32f(&self) -> [Vec<f32>; 6] {
        let resolution = self.resolution.max(1);
        let pixel_count = (resolution as usize).pow(2);
        let mut faces: [Vec<f32>; 6] =
            std::array::from_fn(|_| vec![0.0_f32; pixel_count.saturating_mul(4)]);
        for (face_index, face_pixels) in faces.iter_mut().enumerate() {
            if let Some(stored) = self.face_pixels.as_ref() {
                let source = &stored[face_index];
                for (pixel_index, radiance) in source.iter().enumerate().take(pixel_count) {
                    let offset = pixel_index * 4;
                    face_pixels[offset] = radiance[0];
                    face_pixels[offset + 1] = radiance[1];
                    face_pixels[offset + 2] = radiance[2];
                    face_pixels[offset + 3] = 1.0;
                }
                continue;
            }
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

use super::environment_projection::sample_equirectangular;

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

#[derive(Debug, Clone)]
pub struct EnvironmentDesc {
    name: String,
    provenance: AssetProvenance,
    source_kind: EnvironmentSourceKind,
    source_dimensions: Option<(u32, u32)>,
    preview_irradiance_rgb: Option<[f32; 3]>,
    cubemap_resolution: u32,
    brdf_lut_size: u32,
    wasm_delivery: WasmEnvironmentDelivery,
    derivatives: Vec<EnvironmentDerivative>,
    prefilter_sidecar: Option<std::sync::Arc<EnvironmentPrefilterSidecar>>,
    /// When the environment originates from a Radiance HDR (equirectangular)
    /// the decoded pixel grid is kept here. `cubemap_faces()` then projects
    /// it into per-face radiance grids so the prefiltered specular pass
    /// downstream has real HDR contrast to reflect. `Arc` keeps clones
    /// cheap. Skipped from `PartialEq` because two equal `EnvironmentDesc`s
    /// should compare by name + source identity, not by raw pixel pointers.
    equirectangular_pixels: Option<std::sync::Arc<DecodedEquirectangular>>,
}

impl PartialEq for EnvironmentDesc {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
            && self.provenance == other.provenance
            && self.source_kind == other.source_kind
            && self.source_dimensions == other.source_dimensions
            && self.preview_irradiance_rgb == other.preview_irradiance_rgb
            && self.cubemap_resolution == other.cubemap_resolution
            && self.brdf_lut_size == other.brdf_lut_size
            && self.wasm_delivery == other.wasm_delivery
            && self.derivatives == other.derivatives
    }
}

impl EnvironmentDesc {
    pub fn neutral_studio() -> Self {
        let derivatives = vec![
            EnvironmentDerivative::new(
                DEFAULT_ENVIRONMENT_CUBEMAP_PATH,
                DEFAULT_ENVIRONMENT_CUBEMAP_SHA256,
            ),
            EnvironmentDerivative::new(
                DEFAULT_ENVIRONMENT_BRDF_LUT_PATH,
                DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256,
            ),
        ];
        Self {
            name: DEFAULT_ENVIRONMENT_NAME.to_string(),
            provenance: AssetProvenance::new(DEFAULT_ENVIRONMENT_SOURCE_PATH)
                .with_source_sha256(DEFAULT_ENVIRONMENT_SOURCE_SHA256)
                .with_license(DEFAULT_ENVIRONMENT_LICENSE)
                .with_generator(DEFAULT_ENVIRONMENT_GENERATOR)
                .with_derivatives(derivatives.iter().map(AssetDerivative::from)),
            source_kind: EnvironmentSourceKind::BundledPreviewFixture,
            source_dimensions: None,
            preview_irradiance_rgb: None,
            cubemap_resolution: 256,
            brdf_lut_size: 256,
            wasm_delivery: WasmEnvironmentDelivery::Bundled,
            derivatives,
            prefilter_sidecar: None,
            equirectangular_pixels: None,
        }
    }

    pub fn from_equirectangular_hdr_path(path: impl Into<AssetPath>) -> Self {
        let path = path.into();
        let source_dimensions = parse_equirectangular_hdr_dimensions(&path);
        Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::new(path),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions,
            preview_irradiance_rgb: None,
            cubemap_resolution: 0,
            brdf_lut_size: 0,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: None,
            equirectangular_pixels: None,
        }
    }

    pub(crate) fn from_equirectangular_hdr_bytes(
        path: impl Into<AssetPath>,
        source_bytes: &[u8],
    ) -> Result<Self, AssetError> {
        let path = path.into();
        let decoded = decode_radiance_hdr(&path, source_bytes)?;
        let source_dimensions = (decoded.width, decoded.height);
        let inverse_count = (decoded.pixels.len() as f32).recip();
        let mut preview_irradiance_rgb = [0.0_f32; 3];
        for pixel in &decoded.pixels {
            preview_irradiance_rgb[0] += pixel[0];
            preview_irradiance_rgb[1] += pixel[1];
            preview_irradiance_rgb[2] += pixel[2];
        }
        preview_irradiance_rgb[0] *= inverse_count;
        preview_irradiance_rgb[1] *= inverse_count;
        preview_irradiance_rgb[2] *= inverse_count;
        // Project the equirectangular HDR into a cubemap so the
        // prefiltered specular path has real per-pixel radiance to reflect.
        // Cap the projected face resolution at 256 (the default) — beyond
        // this point the prefilter cost dominates without visible gain.
        let cubemap_resolution = DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION;
        Ok(Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::from_source_bytes(path, source_bytes),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: Some(source_dimensions),
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            cubemap_resolution,
            brdf_lut_size: DEFAULT_ENVIRONMENT_BRDF_LUT_SIZE,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: None,
            equirectangular_pixels: Some(std::sync::Arc::new(decoded)),
        })
    }

    pub(crate) fn from_equirectangular_hdr_sidecar_bytes(
        path: impl Into<AssetPath>,
        source_bytes: &[u8],
        sidecar: EnvironmentPrefilterSidecar,
    ) -> Result<Option<Self>, AssetError> {
        let path = path.into();
        let source_sha256 = sha256_hex(source_bytes);
        if sidecar.source_sha256_hex() != source_sha256 {
            return Ok(None);
        }
        let source_dimensions = parse_equirectangular_hdr_dimensions(&path);
        let preview_irradiance_rgb = sidecar.diffuse_rgb();
        let cubemap_resolution = sidecar.cubemap_resolution();
        let brdf_lut_size = sidecar.brdf_lut_size();
        Ok(Some(Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::new(path).with_source_sha256(source_sha256),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions,
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            cubemap_resolution,
            brdf_lut_size,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: Some(std::sync::Arc::new(sidecar)),
            equirectangular_pixels: None,
        }))
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn source_path(&self) -> &AssetPath {
        self.provenance.source_path()
    }

    pub fn provenance(&self) -> &AssetProvenance {
        &self.provenance
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
        self.provenance.source_sha256()
    }

    pub const fn preview_irradiance_rgb(&self) -> Option<[f32; 3]> {
        self.preview_irradiance_rgb
    }

    pub fn license(&self) -> Option<&str> {
        self.provenance.license()
    }

    pub fn generator(&self) -> Option<&str> {
        self.provenance.generator()
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

    pub(crate) fn prefilter_sidecar(
        &self,
        profile: EnvironmentSidecarProfile,
    ) -> Option<&EnvironmentPrefilterSidecar> {
        self.prefilter_sidecar
            .as_deref()
            .filter(|sidecar| sidecar.profile() == profile)
    }

    pub(crate) fn prefilter_sidecar_identity(&self) -> Option<String> {
        self.prefilter_sidecar.as_ref().map(|sidecar| {
            format!(
                "{}|{}|{}|{}",
                sidecar.profile().name(),
                sidecar.source_sha256_hex(),
                sidecar.cubemap_resolution(),
                sidecar.brdf_lut_size()
            )
        })
    }

    /// Returns the bundled cubemap radiance for this environment when one is
    /// available. Phase 1C step 1: only the bundled `neutral-studio` preview
    /// fixture decodes today. Equirectangular HDR sources will gain a real
    /// face decode in step 2 alongside the GGX prefilter and BRDF LUT.
    pub fn cubemap_faces(&self) -> Option<EnvironmentCubemapFaces> {
        if let Some(equirect) = self.equirectangular_pixels.as_ref() {
            return EnvironmentCubemapFaces::from_equirectangular(
                equirect,
                self.cubemap_resolution.max(1),
            );
        }
        if self.name == DEFAULT_ENVIRONMENT_NAME {
            return EnvironmentCubemapFaces::try_parse_fixture(BUNDLED_NEUTRAL_STUDIO_CUBEMAP);
        }
        None
    }
}

const BUNDLED_NEUTRAL_STUDIO_CUBEMAP: &str =
    include_str!("../../tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml");

impl EnvironmentDerivative {
    pub fn new(path: impl Into<AssetPath>, sha256: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sha256: sha256.into(),
        }
    }

    pub fn path(&self) -> &AssetPath {
        &self.path
    }

    pub fn sha256(&self) -> &str {
        &self.sha256
    }
}

impl From<&EnvironmentDerivative> for AssetDerivative {
    fn from(value: &EnvironmentDerivative) -> Self {
        AssetDerivative::new(value.path.clone(), value.sha256.clone())
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

#[cfg(test)]
mod environment_cubemap_tests;
