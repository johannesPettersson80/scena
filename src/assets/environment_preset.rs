use crate::diagnostics::AssetError;

use super::{AssetFetcher, Assets, EnvironmentHandle};

const NEUTRAL_STUDIO_SOURCE_PATH: &str = "tests/assets/environment/neutral-studio.fixture.txt";
const NEUTRAL_STUDIO_SOURCE_SHA256: &str =
    "955af3ed33b2ad3d525ac8c0c1f83ed9c531a4317994eaa501531e5e35b90d13";
const NEUTRAL_STUDIO_FILES: &[&str] = &[
    "tests/assets/environment/neutral-studio.fixture.txt",
    "tests/assets/environment/generated/neutral-studio-cubemap.fixture.toml",
    "tests/assets/environment/generated/brdf-lut-256.fixture.toml",
];

const STUDIO_SOURCE_PATH: &str = "tests/assets/environment/polyhaven/studio_small_03_1k.hdr";
const STUDIO_SOURCE_SHA256: &str =
    "30933d55e45f0795daf49f3cbefbe0e5ebcb821ee04fb0a2818c02ffc3938817";
const STUDIO_FILES: &[&str] = &[STUDIO_SOURCE_PATH];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvironmentPreset {
    NeutralStudio,
    Studio,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnvironmentPresetMetadata {
    name: &'static str,
    source_path: &'static str,
    source_sha256: &'static str,
    source_url: &'static str,
    license: &'static str,
    contract: &'static str,
    files: &'static [&'static str],
}

impl EnvironmentPreset {
    pub const ALL: &'static [Self] = &[Self::NeutralStudio, Self::Studio];

    /// The currently checked environment preset source files must stay small
    /// enough for crate packaging. This budget covers the neutral preview
    /// fixture derivatives plus the bundled Poly Haven studio HDR.
    pub const PACKAGE_SIZE_BUDGET_BYTES: u64 = 2_000_000;

    pub const fn metadata(self) -> EnvironmentPresetMetadata {
        match self {
            Self::NeutralStudio => EnvironmentPresetMetadata {
                name: "NeutralStudio",
                source_path: NEUTRAL_STUDIO_SOURCE_PATH,
                source_sha256: NEUTRAL_STUDIO_SOURCE_SHA256,
                source_url: "scena://bundled/neutral-studio",
                license: "CC0-1.0",
                contract: "neutral CPU-preview studio fixture with checked cubemap and BRDF LUT derivatives",
                files: NEUTRAL_STUDIO_FILES,
            },
            Self::Studio => EnvironmentPresetMetadata {
                name: "Studio",
                source_path: STUDIO_SOURCE_PATH,
                source_sha256: STUDIO_SOURCE_SHA256,
                source_url: "https://polyhaven.com/a/studio_small_03",
                license: "CC0-1.0",
                contract: "real Poly Haven studio HDR with smooth radiance for product-material environment reflections",
                files: STUDIO_FILES,
            },
        }
    }
}

impl EnvironmentPresetMetadata {
    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn source_path(self) -> &'static str {
        self.source_path
    }

    pub const fn source_sha256(self) -> &'static str {
        self.source_sha256
    }

    pub const fn source_url(self) -> &'static str {
        self.source_url
    }

    pub const fn license(self) -> &'static str {
        self.license
    }

    pub const fn contract(self) -> &'static str {
        self.contract
    }

    pub const fn files(self) -> &'static [&'static str] {
        self.files
    }
}

impl<F: AssetFetcher> Assets<F> {
    pub async fn load_environment_preset(
        &self,
        preset: EnvironmentPreset,
    ) -> Result<EnvironmentHandle, AssetError> {
        self.load_environment(preset.metadata().source_path()).await
    }
}
