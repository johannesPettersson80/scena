//! Checked environment presets for image-based lighting.
//!
//! The presets bundle license, source URL, SHA-256, and file-list metadata so
//! applications can render with realistic studio lighting without writing
//! HDR path plumbing or asset-provenance bookkeeping themselves.
//!
//! # Examples
//!
//! ```no_run
//! # async fn example() -> scena::Result<()> {
//! use scena::{Assets, EnvironmentPreset};
//!
//! let assets = Assets::new();
//! let env = assets.load_environment_preset(EnvironmentPreset::Studio).await?;
//! let _ = env;
//! # Ok(())
//! # }
//! ```

use crate::diagnostics::AssetError;

use super::{AssetFetcher, AssetLoadOptions, Assets, EnvironmentHandle};

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

/// One bundled, checked environment preset.
///
/// Use [`EnvironmentPreset::metadata`] to inspect source URL, license, SHA-256,
/// and the full file list. Variants are `#[non_exhaustive]` so the catalog can
/// grow without breaking matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum EnvironmentPreset {
    /// Neutral CPU-preview studio fixture with checked cubemap and BRDF LUT
    /// derivatives. Suitable for headless previews and CI gates.
    NeutralStudio,
    /// Real Poly Haven studio HDR with smooth radiance for product-material
    /// environment reflections.
    Studio,
}

/// Static metadata for an [`EnvironmentPreset`] entry.
///
/// The metadata pins the upstream source URL, license, SHA-256 of the primary
/// fixture, and the full file list so package-budget tests and provenance
/// audits can verify which bytes ship with the crate.
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
    /// Every preset in the checked catalog.
    pub const ALL: &'static [Self] = &[Self::NeutralStudio, Self::Studio];

    pub const fn recipe_name(self) -> &'static str {
        match self {
            Self::NeutralStudio => "neutral_studio",
            Self::Studio => "studio",
        }
    }

    pub fn from_recipe_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|preset| preset.recipe_name() == name)
    }

    /// The currently checked environment preset source files must stay small
    /// enough for crate packaging. This budget covers the neutral preview
    /// fixture derivatives plus the bundled Poly Haven studio HDR.
    pub const PACKAGE_SIZE_BUDGET_BYTES: u64 = 2_000_000;

    /// Returns the static metadata for this preset.
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
    /// Catalog identifier (`"NeutralStudio"`, `"Studio"`, …).
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// Primary file path passed to the asset fetcher.
    pub const fn source_path(self) -> &'static str {
        self.source_path
    }

    /// SHA-256 of the primary fixture; used by package-budget tests.
    pub const fn source_sha256(self) -> &'static str {
        self.source_sha256
    }

    /// Upstream URL for the source asset.
    pub const fn source_url(self) -> &'static str {
        self.source_url
    }

    /// License identifier (e.g. `"CC0-1.0"`).
    pub const fn license(self) -> &'static str {
        self.license
    }

    /// One-line description of what the preset provides.
    pub const fn contract(self) -> &'static str {
        self.contract
    }

    /// Every file that the preset depends on.
    pub const fn files(self) -> &'static [&'static str] {
        self.files
    }
}

impl<F: AssetFetcher> Assets<F> {
    /// Loads a bundled [`EnvironmentPreset`] through the host's asset fetcher.
    ///
    /// The preset's primary source path is resolved through the standard
    /// [`Assets::load_environment`](super::Assets::load_environment) pathway,
    /// so the host's asset fetcher is responsible for actually reading bytes
    /// off disk or over the network.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # async fn example() -> scena::Result<()> {
    /// use scena::{Assets, EnvironmentPreset};
    ///
    /// let assets = Assets::new();
    /// let env = assets.load_environment_preset(EnvironmentPreset::Studio).await?;
    /// let _ = env;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn load_environment_preset(
        &self,
        preset: EnvironmentPreset,
    ) -> Result<EnvironmentHandle, AssetError> {
        self.load_environment_preset_with_options(preset, AssetLoadOptions::default())
            .await
    }

    /// Loads a bundled [`EnvironmentPreset`] with explicit asset-load policy.
    ///
    /// Recipe execution uses this path so the preset source and optional
    /// sidecar files obey the same fetch-byte budget as imported assets.
    pub async fn load_environment_preset_with_options(
        &self,
        preset: EnvironmentPreset,
        options: AssetLoadOptions,
    ) -> Result<EnvironmentHandle, AssetError> {
        self.load_environment_with_options(preset.metadata().source_path(), options)
            .await
    }
}
