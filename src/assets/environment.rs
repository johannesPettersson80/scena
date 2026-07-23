use super::environment_hdr::{
    DecodedEquirectangular, decode_radiance_hdr, parse_equirectangular_hdr_dimensions,
    radiance_hdr_dimensions,
};
use super::environment_sidecar::{
    EnvironmentPrefilterSidecar, EnvironmentSidecarProfile, sha256_hex,
};
use super::{AssetDerivative, AssetPath, AssetProvenance};
use crate::diagnostics::AssetError;
use std::sync::{Arc, OnceLock};

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

#[derive(Debug)]
struct LazyEquirectangularSource {
    bytes: Arc<[u8]>,
    decoded: OnceLock<Arc<DecodedEquirectangular>>,
}

impl LazyEquirectangularSource {
    fn new(source_bytes: &[u8]) -> Self {
        Self {
            bytes: Arc::from(source_bytes),
            decoded: OnceLock::new(),
        }
    }

    fn decoded(&self, path: &AssetPath) -> Option<Arc<DecodedEquirectangular>> {
        if let Some(decoded) = self.decoded.get() {
            return Some(Arc::clone(decoded));
        }
        let decoded = Arc::new(decode_radiance_hdr(path, &self.bytes).ok()?);
        if self.decoded.set(Arc::clone(&decoded)).is_ok() {
            Some(decoded)
        } else {
            self.decoded.get().map(Arc::clone)
        }
    }
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
    /// Compressed HDR bytes retained only for sidecar-backed environments. The
    /// matching sidecar path stays cheap: it never decodes these bytes. They are
    /// decoded lazily only when a later render requests a different sidecar
    /// profile and must bake a fresh cubemap rather than silently flatten IBL.
    lazy_equirectangular_source: Option<Arc<LazyEquirectangularSource>>,
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
            lazy_equirectangular_source: None,
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
            lazy_equirectangular_source: None,
        }
    }

    #[doc(hidden)]
    pub fn from_equirectangular_hdr_bytes(
        path: impl Into<AssetPath>,
        source_bytes: &[u8],
    ) -> Result<Self, AssetError> {
        let path = path.into();
        let decoded = decode_radiance_hdr(&path, source_bytes)?;
        let inverse_count = (decoded.pixels.len() as f32).recip();
        let mut preview_irradiance_rgb = [0.0_f32; 3];
        for pixel in &decoded.pixels {
            preview_irradiance_rgb[0] += pixel[0];
            preview_irradiance_rgb[1] += pixel[1];
            preview_irradiance_rgb[2] += pixel[2];
        }
        preview_irradiance_rgb = preview_irradiance_rgb.map(|channel| channel * inverse_count);
        // Project the equirectangular HDR into a cubemap so the
        // prefiltered specular path has real per-pixel radiance to reflect.
        Ok(Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::from_source_bytes(path, source_bytes),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: Some((decoded.width, decoded.height)),
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            cubemap_resolution: DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION,
            brdf_lut_size: DEFAULT_ENVIRONMENT_BRDF_LUT_SIZE,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: None,
            equirectangular_pixels: Some(std::sync::Arc::new(decoded)),
            lazy_equirectangular_source: None,
        })
    }

    #[doc(hidden)]
    pub fn from_equirectangular_radiance(
        path: impl Into<AssetPath>,
        width: u32,
        height: u32,
        pixels: Vec<[f32; 3]>,
    ) -> Result<Self, AssetError> {
        let path = path.into();
        let expected_len =
            (width as u64)
                .checked_mul(height as u64)
                .ok_or_else(|| AssetError::Parse {
                    path: path.as_str().to_string(),
                    reason: "equirectangular radiance dimensions overflow".to_string(),
                })?;
        if width == 0 || height == 0 || pixels.len() as u64 != expected_len {
            return Err(AssetError::Parse {
                path: path.as_str().to_string(),
                reason: format!(
                    "equirectangular radiance must contain width*height pixels, got {} for {width}x{height}",
                    pixels.len()
                ),
            });
        }
        let mut preview_irradiance_rgb = [0.0_f32; 3];
        for pixel in &pixels {
            if !pixel
                .iter()
                .all(|channel| channel.is_finite() && *channel >= 0.0)
            {
                return Err(AssetError::Parse {
                    path: path.as_str().to_string(),
                    reason: "equirectangular radiance pixels must be finite and non-negative"
                        .to_string(),
                });
            }
            preview_irradiance_rgb[0] += pixel[0];
            preview_irradiance_rgb[1] += pixel[1];
            preview_irradiance_rgb[2] += pixel[2];
        }
        let inverse_count = (pixels.len() as f32).recip();
        preview_irradiance_rgb = preview_irradiance_rgb.map(|channel| channel * inverse_count);
        Ok(Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::new(path.clone()),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: Some((width, height)),
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            cubemap_resolution: DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION,
            brdf_lut_size: DEFAULT_ENVIRONMENT_BRDF_LUT_SIZE,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: None,
            equirectangular_pixels: Some(std::sync::Arc::new(DecodedEquirectangular {
                width,
                height,
                pixels,
            })),
            lazy_equirectangular_source: None,
        })
    }

    #[doc(hidden)]
    pub fn with_cubemap_resolution(mut self, cubemap_resolution: u32) -> Self {
        self.cubemap_resolution = cubemap_resolution.max(1);
        self
    }

    /// Attaches a precomputed sidecar after verifying that it belongs to this
    /// environment's exact source payload.
    #[doc(hidden)]
    pub fn with_prefilter_sidecar(
        mut self,
        sidecar: EnvironmentPrefilterSidecar,
    ) -> Result<Self, AssetError> {
        let source_sha256 = self.source_sha256().ok_or_else(|| AssetError::Parse {
            path: self.source_path().as_str().to_string(),
            reason: "environment sidecar attachment requires source SHA-256".to_string(),
        })?;
        if sidecar.source_sha256_hex() != source_sha256 {
            return Err(AssetError::Parse {
                path: self.source_path().as_str().to_string(),
                reason: "environment sidecar source SHA-256 does not match the environment"
                    .to_string(),
            });
        }
        self.prefilter_sidecar = Some(std::sync::Arc::new(sidecar));
        Ok(self)
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
        // Record true source dimensions from the HDR header (cheap; no RGBE
        // decode) so a sidecar-backed environment keeps the same source contract
        // and cache identity as the decode path. Fall back to the filename tag.
        let source_dimensions = radiance_hdr_dimensions(source_bytes)
            .or_else(|| parse_equirectangular_hdr_dimensions(&path));
        let preview_irradiance_rgb = sidecar.diffuse_rgb();
        let brdf_lut_size = sidecar.brdf_lut_size();
        Ok(Some(Self {
            name: environment_name_from_path(&path).to_string(),
            provenance: AssetProvenance::new(path).with_source_sha256(source_sha256),
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions,
            preview_irradiance_rgb: Some(preview_irradiance_rgb),
            cubemap_resolution: DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION,
            brdf_lut_size,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
            prefilter_sidecar: Some(std::sync::Arc::new(sidecar)),
            equirectangular_pixels: None,
            lazy_equirectangular_source: Some(Arc::new(LazyEquirectangularSource::new(
                source_bytes,
            ))),
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

    pub(crate) fn prefilter_sidecar_profile(&self) -> Option<EnvironmentSidecarProfile> {
        self.prefilter_sidecar
            .as_deref()
            .map(EnvironmentPrefilterSidecar::profile)
    }

    pub(crate) fn has_prefilter_sidecar_profile(&self, profile: EnvironmentSidecarProfile) -> bool {
        self.prefilter_sidecar(profile).is_some()
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
        if let Some(source) = self.lazy_equirectangular_source.as_ref()
            && let Some(equirect) = source.decoded(self.source_path())
        {
            return EnvironmentCubemapFaces::from_equirectangular(
                &equirect,
                self.cubemap_resolution.max(1),
            );
        }
        if self.name == DEFAULT_ENVIRONMENT_NAME {
            return EnvironmentCubemapFaces::try_parse_fixture(BUNDLED_NEUTRAL_STUDIO_CUBEMAP);
        }
        None
    }

    pub(crate) fn has_cubemap_face_source(&self) -> bool {
        self.equirectangular_pixels.is_some()
            || self.lazy_equirectangular_source.is_some()
            || self.name == DEFAULT_ENVIRONMENT_NAME
    }
}

const BUNDLED_NEUTRAL_STUDIO_CUBEMAP: &str =
    include_str!("../../tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml");

mod cubemap_faces;

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

#[cfg(test)]
mod sidecar_tests;
