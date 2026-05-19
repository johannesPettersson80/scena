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

#[derive(Debug, Clone, Copy)]
pub struct KhronosSamples<'a, F> {
    assets: &'a Assets<F>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KhronosSample {
    RiggedSimple,
    SimpleSkin,
    SimpleMorph,
    MorphCube,
    RiggedFigure,
    BrainStem,
    AlphaBlendModeTest,
    TextureSettingsTest,
    TextureTransformTest,
    UnlitTest,
    WaterBottle,
    TransmissionTest,
}

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
    pub fn khronos(&self) -> KhronosSamples<'_, F> {
        KhronosSamples { assets: self }
    }
}

impl<'a, F: AssetFetcher> KhronosSamples<'a, F> {
    pub async fn load(&self, sample: KhronosSample) -> Result<SceneAsset, AssetError> {
        self.assets
            .load_scene(sample.metadata().primary_path())
            .await
    }

    pub async fn water_bottle(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::WaterBottle).await
    }

    pub async fn transmission_test(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::TransmissionTest).await
    }

    pub async fn rigged_simple(&self) -> Result<SceneAsset, AssetError> {
        self.load(KhronosSample::RiggedSimple).await
    }
}

impl KhronosSample {
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
    pub const fn name(self) -> &'static str {
        self.name
    }

    pub const fn upstream_name(self) -> Option<&'static str> {
        self.upstream_name
    }

    pub const fn primary_path(self) -> &'static str {
        self.primary_path
    }

    pub const fn files(self) -> &'static [&'static str] {
        self.files
    }

    pub const fn primary_sha256(self) -> &'static str {
        self.primary_sha256
    }

    pub const fn contract(self) -> &'static str {
        self.contract
    }

    pub const fn source_repository(self) -> &'static str {
        SOURCE_REPOSITORY
    }

    pub const fn source_commit(self) -> &'static str {
        SOURCE_COMMIT
    }

    pub const fn license_reference(self) -> &'static str {
        LICENSE_REFERENCE
    }
}
