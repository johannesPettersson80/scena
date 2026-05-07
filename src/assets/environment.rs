use super::AssetPath;

const DEFAULT_ENVIRONMENT_NAME: &str = "neutral-studio";
pub(super) const DEFAULT_ENVIRONMENT_SOURCE_PATH: &str =
    "tests/assets/environment/neutral-studio.placeholder.hdr";
const DEFAULT_ENVIRONMENT_SOURCE_SHA256: &str =
    "b95916ffe38d8825bbf701fd2a6efe56983e1f7d241856426440869138e3973e";
const DEFAULT_ENVIRONMENT_LICENSE: &str = "CC0-1.0";
const DEFAULT_ENVIRONMENT_GENERATOR: &str =
    "xtask generate-default-env --input tests/assets/environment/neutral-studio.placeholder.hdr";
const DEFAULT_ENVIRONMENT_CUBEMAP_PATH: &str =
    "tests/assets/environment/generated/neutral-studio-cubemap.ktx2";
const DEFAULT_ENVIRONMENT_CUBEMAP_SHA256: &str =
    "e6c9093c4dc8efd2fa9f46be2a41d5bc97e977240dd81eccbc8cbc50e5181f24";
const DEFAULT_ENVIRONMENT_BRDF_LUT_PATH: &str =
    "tests/assets/environment/generated/brdf-lut-256.rgba16f";
const DEFAULT_ENVIRONMENT_BRDF_LUT_SHA256: &str =
    "08a2a2c32fe45ccf0d799db947a729269aaf58ec0c933c3e6e8dd99784789ef7";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmEnvironmentDelivery {
    Bundled,
    SeparateFetch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnvironmentSourceKind {
    EquirectangularHdr,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDerivative {
    path: AssetPath,
    sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvironmentDesc {
    name: String,
    source_path: AssetPath,
    source_kind: EnvironmentSourceKind,
    source_dimensions: Option<(u32, u32)>,
    source_sha256: Option<String>,
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
            source_kind: EnvironmentSourceKind::EquirectangularHdr,
            source_dimensions: None,
            source_sha256: Some(DEFAULT_ENVIRONMENT_SOURCE_SHA256.to_string()),
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
            license: None,
            generator: None,
            cubemap_resolution: 0,
            brdf_lut_size: 0,
            wasm_delivery: WasmEnvironmentDelivery::SeparateFetch,
            derivatives: Vec::new(),
        }
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
