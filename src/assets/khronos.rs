//! Bundled Khronos glTF sample loaders.
//!
//! Behind the `khronos-samples` feature, this module exposes a checked catalog
//! of Khronos reference assets (`WaterBottle`, `RiggedSimple`, the transmission
//! and texture tests, etc.) so applications and examples can demonstrate scena
//! without shipping their own asset pipeline. The samples are loaded as
//! ordinary [`SceneAsset`]s through the host's asset fetcher; no bytes are
//! embedded into the library binary.
//!
//! # Examples
//!
//! ```no_run
//! # #[cfg(feature = "khronos-samples")]
//! # async fn example() -> scena::Result<()> {
//! use scena::Assets;
//!
//! let assets = Assets::new();
//! let bottle = assets.khronos().water_bottle().await?;
//! let _ = bottle;
//! # Ok(())
//! # }
//! ```

use crate::diagnostics::AssetError;

use super::{AssetFetcher, Assets, SceneAsset};

const SOURCE_REPOSITORY: &str = "https://github.com/KhronosGroup/glTF-Sample-Assets";
const SOURCE_COMMIT: &str = "2bac6f8c57bf471df0d2a1e8a8ec023c7801dddf";
const LICENSE_REFERENCE: &str = "Upstream LICENSES directory in glTF-Sample-Assets";

macro_rules! sample_path {
    ($path:literal) => {
        concat!("tests/assets/gltf/khronos", $path)
    };
}

/// Borrowed loader handle for the Khronos sample catalog.
///
/// Obtained from [`Assets::khronos`]. Each method loads one catalog entry
/// through the host's asset fetcher and returns a [`SceneAsset`].
#[derive(Debug, Clone, Copy)]
pub struct KhronosSamples<'a, F> {
    assets: &'a Assets<F>,
}

/// One entry in the checked Khronos sample catalog.
///
/// Use [`KhronosSample::metadata`] for the asset's upstream name, file list,
/// SHA-256 of the primary fixture, and contract describing what the asset
/// exercises. Variants are marked `#[non_exhaustive]` so the catalog can grow
/// without breaking matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KhronosSample {
    /// Minimal skinned/animated cylinder used to exercise the skin and
    /// animation channels in a small, self-contained fixture.
    RiggedSimple,
    /// Minimal skinning fixture for skeleton + inverse bind matrices.
    SimpleSkin,
    /// Minimal morph-target fixture.
    SimpleMorph,
    /// Animated morph-target cube; the headline morph fixture.
    MorphCube,
    /// Skinning with both matrix and TRS node hierarchies.
    RiggedFigure,
    /// Larger skinning and animation hierarchy.
    BrainStem,
    /// Alpha-blend and alpha-mask cutoff coverage with PBR textures.
    AlphaBlendModeTest,
    /// Sampler filter, wrap-mode, and double-sided metadata coverage.
    TextureSettingsTest,
    /// `KHR_texture_transform` offset / rotation / scale coverage.
    TextureTransformTest,
    /// `KHR_materials_unlit` mapping coverage.
    UnlitTest,
    /// Production-grade PBR product with millimeter-scale framing.
    WaterBottle,
    /// `KHR_materials_transmission` control asset.
    TransmissionTest,
}

/// Static metadata for one [`KhronosSample`] catalog entry.
///
/// All fields are static strings or slices so the catalog can be inspected at
/// `const` time. The metadata is what lets package-budget tests and license /
/// provenance audits prove which upstream commit and license the bundled
/// samples come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KhronosSampleMetadata {
    name: &'static str,
    upstream_name: Option<&'static str>,
    primary_path: &'static str,
    files: &'static [&'static str],
    primary_sha256: &'static str,
    contract: &'static str,
}

impl<F: AssetFetcher> Assets<F> {
    /// Returns a borrowed handle for loading bundled Khronos sample assets.
    ///
    /// Feature-gated behind `khronos-samples`. The handle borrows `self`, so
    /// the usual sequence is `assets.khronos().water_bottle().await?`.
    pub fn khronos(&self) -> KhronosSamples<'_, F> {
        KhronosSamples { assets: self }
    }
}

impl<'a, F: AssetFetcher> KhronosSamples<'a, F> {
    /// Loads a catalog entry by enum identity.
    ///
    /// Equivalent to looking the sample's primary path up through
    /// [`KhronosSample::metadata`] and feeding it to
    /// [`Assets::load_scene`](super::Assets::load_scene).
    pub async fn load(&self, sample: KhronosSample) -> Result<SceneAsset, AssetError> {
        self.assets
            .load_scene(sample.metadata().primary_path())
            .await
    }

    /// Loads [`KhronosSample::WaterBottle`] — the headline PBR product asset.
    pub async fn water_bottle(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::WaterBottle).await
    }

    /// Loads [`KhronosSample::TransmissionTest`] — the transmission control asset.
    pub async fn transmission_test(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::TransmissionTest).await
    }

    /// Loads [`KhronosSample::RiggedSimple`] — the headline skinning fixture.
    pub async fn rigged_simple(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::RiggedSimple).await
    }
}

impl KhronosSample {
    /// Every variant in the checked catalog, in display order.
    pub const ALL: &'static [Self] = &[
        Self::RiggedSimple,
        Self::SimpleSkin,
        Self::SimpleMorph,
        Self::MorphCube,
        Self::RiggedFigure,
        Self::BrainStem,
        Self::AlphaBlendModeTest,
        Self::TextureSettingsTest,
        Self::TextureTransformTest,
        Self::UnlitTest,
        Self::WaterBottle,
        Self::TransmissionTest,
    ];

    /// The checked fixture catalog must stay below 9 MB. These are dev/test
    /// fixture paths included by the crate package; the loader does not embed
    /// bytes into the library binary.
    pub const PACKAGE_SIZE_BUDGET_BYTES: u64 = 9_000_000;

    /// Returns the static metadata for this sample — primary path, full file
    /// list, SHA-256 of the primary fixture, upstream name, and contract.
    pub const fn metadata(self) -> KhronosSampleMetadata {
        match self {
            Self::RiggedSimple => KhronosSampleMetadata {
                name: "RiggedSimple",
                upstream_name: None,
                primary_path: sample_path!("/RiggedSimple/RiggedSimple.gltf"),
                files: &[
                    sample_path!("/RiggedSimple/RiggedSimple.gltf"),
                    sample_path!("/RiggedSimple/RiggedSimple0.bin"),
                ],
                primary_sha256: "18cd3e4d50ecadc93f6b6a720a4ee0d5154443c2fa331bc0dd3bef7499c02eb0",
                contract: "skin, animation channels, inverse bind matrices",
            },
            Self::SimpleSkin => KhronosSampleMetadata {
                name: "SimpleSkin",
                upstream_name: None,
                primary_path: sample_path!("/SimpleSkin/SimpleSkin.gltf"),
                files: &[
                    sample_path!("/SimpleSkin/SimpleSkin.gltf"),
                    sample_path!("/SimpleSkin/SimpleSkin_animation.bin"),
                    sample_path!("/SimpleSkin/SimpleSkin_geometry.bin"),
                    sample_path!("/SimpleSkin/SimpleSkin_inverseBindMatrices.bin"),
                    sample_path!("/SimpleSkin/SimpleSkin_skinningData.bin"),
                ],
                primary_sha256: "8d92e9888340eb98e82a65a1f8b037d9a1d9f09e5fd0b6ba6d40fd93700b3239",
                contract: "minimal skinning fixture",
            },
            Self::SimpleMorph => KhronosSampleMetadata {
                name: "SimpleMorph",
                upstream_name: None,
                primary_path: sample_path!("/SimpleMorph/SimpleMorph.gltf"),
                files: &[
                    sample_path!("/SimpleMorph/SimpleMorph.gltf"),
                    sample_path!("/SimpleMorph/SimpleMorph_animation.bin"),
                    sample_path!("/SimpleMorph/SimpleMorph_geometry.bin"),
                ],
                primary_sha256: "99ec94e30b326077edb3958f5d2b68b4d8ad181529da86f5adf883065e68a2d2",
                contract: "minimal morph-target fixture",
            },
            Self::MorphCube => KhronosSampleMetadata {
                name: "MorphCube",
                upstream_name: Some("AnimatedMorphCube"),
                primary_path: sample_path!("/MorphCube/AnimatedMorphCube.gltf"),
                files: &[
                    sample_path!("/MorphCube/AnimatedMorphCube.gltf"),
                    sample_path!("/MorphCube/AnimatedMorphCube.bin"),
                ],
                primary_sha256: "0b910ed4b52fd9fbb565911fc7f9f285edb0f30fa4913a93143466d439d1092e",
                contract: "animated morph-target fixture",
            },
            Self::RiggedFigure => KhronosSampleMetadata {
                name: "RiggedFigure",
                upstream_name: None,
                primary_path: sample_path!("/RiggedFigure/RiggedFigure.gltf"),
                files: &[
                    sample_path!("/RiggedFigure/RiggedFigure.gltf"),
                    sample_path!("/RiggedFigure/RiggedFigure0.bin"),
                ],
                primary_sha256: "ca84ebbf6084d755ff7ac81b3b7c8bc388da68c8aa4b2cf236503b07d3858cc6",
                contract: "skinning with matrix/TRS node hierarchy",
            },
            Self::BrainStem => KhronosSampleMetadata {
                name: "BrainStem",
                upstream_name: None,
                primary_path: sample_path!("/BrainStem/BrainStem.gltf"),
                files: &[
                    sample_path!("/BrainStem/BrainStem.gltf"),
                    sample_path!("/BrainStem/BrainStem0.bin"),
                ],
                primary_sha256: "8fc99ed74161aa79573236f9a457e238b0649993dfb41234fd7f6eafad696b75",
                contract: "larger skinning and animation hierarchy",
            },
            Self::AlphaBlendModeTest => KhronosSampleMetadata {
                name: "AlphaBlendModeTest",
                upstream_name: None,
                primary_path: sample_path!("/AlphaBlendModeTest/AlphaBlendModeTest.gltf"),
                files: &[
                    sample_path!("/AlphaBlendModeTest/AlphaBlendModeTest.gltf"),
                    sample_path!("/AlphaBlendModeTest/AlphaBlendModeTest.bin"),
                    sample_path!("/AlphaBlendModeTest/AlphaBlendLabels.png"),
                    sample_path!("/AlphaBlendModeTest/MatBed_baseColor.jpg"),
                    sample_path!("/AlphaBlendModeTest/MatBed_normal.jpg"),
                    sample_path!("/AlphaBlendModeTest/MatBed_occlusionRoughnessMetallic.jpg"),
                ],
                primary_sha256: "49e06672900df95593040d35bbc7a2ee5921ae8d46edc44b64ccf2df65e64849",
                contract: "alpha blend, alpha mask cutoffs, normal/occlusion/metallic-roughness/base-color texture slots",
            },
            Self::TextureSettingsTest => KhronosSampleMetadata {
                name: "TextureSettingsTest",
                upstream_name: None,
                primary_path: sample_path!("/TextureSettingsTest/TextureSettingsTest.gltf"),
                files: &[
                    sample_path!("/TextureSettingsTest/TextureSettingsTest.gltf"),
                    sample_path!("/TextureSettingsTest/TextureSettingsTest0.bin"),
                    sample_path!("/TextureSettingsTest/CheckAndX.png"),
                    sample_path!("/TextureSettingsTest/CheckAndX_V.png"),
                    sample_path!("/TextureSettingsTest/TextureTestLabels.png"),
                ],
                primary_sha256: "dd1f85b4638ffc72f1e09e19b101b7a05d94a0353facced8f59c16a2d9c09447",
                contract: "texture sampler filters, clamp/repeat/mirrored wrap modes, and double-sided metadata",
            },
            Self::TextureTransformTest => KhronosSampleMetadata {
                name: "TextureTransformTest",
                upstream_name: None,
                primary_path: sample_path!("/TextureTransformTest/TextureTransformTest.gltf"),
                files: &[
                    sample_path!("/TextureTransformTest/TextureTransformTest.gltf"),
                    sample_path!("/TextureTransformTest/TextureTransformTest.bin"),
                    sample_path!("/TextureTransformTest/Arrow.png"),
                    sample_path!("/TextureTransformTest/Correct.png"),
                    sample_path!("/TextureTransformTest/Error.png"),
                    sample_path!("/TextureTransformTest/NotSupported.png"),
                    sample_path!("/TextureTransformTest/UV.png"),
                ],
                primary_sha256: "c22c8c6c96c0ea4bcbb9b47ea245a093c5ef59acc5fd425effa4c00da4cdf164",
                contract: "KHR_texture_transform offset, rotation, scale, and alternate texcoord metadata",
            },
            Self::UnlitTest => KhronosSampleMetadata {
                name: "UnlitTest",
                upstream_name: None,
                primary_path: sample_path!("/UnlitTest/UnlitTest.gltf"),
                files: &[
                    sample_path!("/UnlitTest/UnlitTest.gltf"),
                    sample_path!("/UnlitTest/UnlitTest.bin"),
                ],
                primary_sha256: "b611ca0892cb1a0118e92b72ff52b3afed3e4903c15378daa72c4c252a0334e1",
                contract: "KHR_materials_unlit required extension material mapping",
            },
            Self::WaterBottle => KhronosSampleMetadata {
                name: "WaterBottle",
                upstream_name: None,
                primary_path: sample_path!("/WaterBottle/WaterBottle.gltf"),
                files: &[
                    sample_path!("/WaterBottle/WaterBottle.gltf"),
                    sample_path!("/WaterBottle/WaterBottle.bin"),
                    sample_path!("/WaterBottle/WaterBottle_baseColor.png"),
                    sample_path!("/WaterBottle/WaterBottle_normal.png"),
                    sample_path!("/WaterBottle/WaterBottle_occlusionRoughnessMetallic.png"),
                    sample_path!("/WaterBottle/WaterBottle_emissive.png"),
                ],
                primary_sha256: "0596f4e61dc781439d254fdfb5e3462daf1762c18715e3e3ac13001aa8f3f547",
                contract: "real product PBR with base-color + normal + occlusion-roughness-metallic + emissive textures and real-world millimeter scale",
            },
            Self::TransmissionTest => KhronosSampleMetadata {
                name: "TransmissionTest",
                upstream_name: None,
                primary_path: sample_path!("/TransmissionTest/TransmissionTest.glb"),
                files: &[sample_path!("/TransmissionTest/TransmissionTest.glb")],
                primary_sha256: "dd9732dae5517f8605ad4324d78b077b969c3e8357c056280d0a4e4b67797d15",
                contract: "Khronos optional KHR_materials_transmission control asset for framing and extension-degradation coverage",
            },
        }
    }
}

impl KhronosSampleMetadata {
    /// The catalog identifier (`"WaterBottle"`, `"RiggedSimple"`, …).
    pub const fn name(self) -> &'static str {
        self.name
    }

    /// The upstream Khronos sample name, when it differs from the catalog name.
    pub const fn upstream_name(self) -> Option<&'static str> {
        self.upstream_name
    }

    /// The primary file path passed to the asset fetcher when loading the sample.
    pub const fn primary_path(self) -> &'static str {
        self.primary_path
    }

    /// Every file that the sample depends on (primary glTF/GLB plus referenced bin/png/jpg).
    pub const fn files(self) -> &'static [&'static str] {
        self.files
    }

    /// SHA-256 of the primary fixture; used by package-budget tests to detect drift.
    pub const fn primary_sha256(self) -> &'static str {
        self.primary_sha256
    }

    /// One-line description of what behavior the sample exercises.
    pub const fn contract(self) -> &'static str {
        self.contract
    }

    /// Upstream repository URL where the bundled samples were sourced from.
    pub const fn source_repository(self) -> &'static str {
        SOURCE_REPOSITORY
    }

    /// Pinned upstream commit hash for provenance.
    pub const fn source_commit(self) -> &'static str {
        SOURCE_COMMIT
    }

    /// Pointer to the upstream license directory.
    pub const fn license_reference(self) -> &'static str {
        LICENSE_REFERENCE
    }
}
