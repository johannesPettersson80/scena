use crate::app::prelude::*;

pub(crate) fn check_environment_lifecycle_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render.rs",
        &[
            "environment: Option<EnvironmentHandle>",
            "environment_revision: u64",
            "NotPreparedReason::EnvironmentChanged",
            "ChangeKind::Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render/prepare_lifecycle.rs",
        &[
            "PrepareError::EnvironmentAssetsRequired",
            "PrepareError::EnvironmentNotFound",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/render/settings.rs",
        &[
            "pub fn environment(&self) -> Option<EnvironmentHandle>",
            "pub fn set_environment(&mut self, environment: EnvironmentHandle)",
            "pub fn clear_environment(&mut self)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "src/diagnostics.rs",
        &[
            "EnvironmentAssetsRequired",
            "EnvironmentNotFound",
            "EnvironmentChanged",
            "Environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_geometry_materials.rs",
        &[
            "renderer_environment_is_structural_and_validated_during_prepare",
            "m1_logical_asset_resource_counters_return_to_baseline_after_empty_prepare",
            "renderer.clear_environment()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENVIRONMENT-LIFECYCLE",
        "tests/m1_visual_proof.rs",
        &[
            "render_default_cube_with_default_environment",
            "validate_default_cube_luminance_and_silhouette",
        ],
    );
}

pub(crate) fn check_equirectangular_hdr_environment_contracts(
    root: &Path,
    findings: &mut Vec<Finding>,
) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets/environment_hdr.rs",
        &[
            "parse_equirectangular_hdr_dimensions",
            "parse_radiance_hdr_preview",
            "decode_radiance_hdr",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets/environment_loading.rs",
        &[
            "AssetError::UnsupportedEnvironmentFormat",
            "embedded_environment_bytes",
            "only base64 Radiance HDR data URIs",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets/environment.rs",
        &[
            "EnvironmentSourceKind::EquirectangularHdr",
            "pub fn from_equirectangular_hdr_path",
            "from_equirectangular_hdr_bytes",
            "is_equirectangular_hdr_path",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/lib.rs",
        &["EnvironmentSourceKind"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_hdr_environment_loading_records_source_contract",
            "EnvironmentSourceKind::EquirectangularHdr",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "tests/m8_assets_materials_ecosystem.rs",
        &[
            "m8_environment_hdr_lights_pbr_preview_pixels",
            "m8_environment_hdr_data_uri_lights_pbr_preview_pixels",
            "tiny_radiance_hdr_rgbe",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "Equirectangular HDR environment loading",
            "EnvironmentSourceKind",
        ],
    );
}

pub(crate) fn check_environment_ibl_prepare_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/stats.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentStats",
            "cubemaps: 1",
            "!environment.has_prefilter_sidecar_profile(sidecar_profile)",
            "brdf_luts: 1",
            "environment.has_cubemap_face_source()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare_lifecycle.rs",
        &[
            "prepare::collect_environment_prepare_stats(",
            "self.target.backend",
            "self.stats.environment_cubemaps = environment_prepare_stats.cubemaps",
            "self.stats.environment_prefilter_passes = environment_prepare_stats.prefilter_passes",
            "self.stats.environment_brdf_luts = environment_prepare_stats.brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/environment_cache.rs",
        &[
            "EnvironmentLightingCache",
            "PreparedEnvironmentLighting::from_environment_with_profile(",
            "EnvironmentLightingProfile::for_backend(self.target.backend)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/environment.rs",
        &[
            "pub(in crate::render) struct PreparedEnvironmentLighting",
            // Phase 1C steps 1-2: prepare-side decoder reads the bundled
            // cubemap through `EnvironmentDesc::cubemap_faces()`, builds
            // RGBA32F face pixels, runs the GGX prefilter mip chain, and
            // builds the split-sum BRDF LUT for the GPU upload. The CPU
            // shading path keeps consuming the preview-irradiance scalar
            // so existing CPU rasterizer fixtures hold; the GPU shader
            // switches to a real `texture_cube<f32>` mip-roughness sample
            // composed with `prefiltered * (F0 * lut.x + lut.y)`.
            "environment.cubemap_faces()",
            "warn_environment_sidecar_profile_mismatch",
            "profile_mismatched_sidecar_preserves_specular_reflection_contrast",
            "environment.preview_irradiance_rgb()",
            "build_face_pixels_rgba32f",
            "PreparedEnvironmentCubemap",
            "bake_environment_ibl",
            "EnvironmentIblBakeRequest",
            "prefilter_lod_for_roughness",
            "PREFILTER_MIP_COUNT",
            "BRDF_LUT_SIZE",
            "gpu_diffuse_intensity",
            "gpu_specular_intensity",
            "pbr_contribution",
            "collect_environment_lighting",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "tests/m8_hdr_rle.rs",
        &[
            "profile_mismatched_sidecar_renders_structured_chrome_not_flat",
            "environment_prefilter_passes",
            "range >= 80.0 && gradient >= 0.8",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/assets/environment_sidecar.rs",
        &[
            "SCENA_ENV_PF_V2",
            "const SIDECAR_VERSION: u32 = 2",
            "legacy_v1_sidecar_is_rejected_after_khronos_baker_change",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare.rs",
        &["mod environment_baker;"],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare.rs",
        &["mod environment_prefilter;"],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/environment_baker.rs",
        &[
            "Rust-owned image-based-lighting baker",
            "Khronos glTF IBL Sampler filtered",
            "pub(in crate::render) struct EnvironmentIblBakeRequest",
            "pub(in crate::render) struct BakedEnvironmentIbl",
            "pub(in crate::render) fn bake_environment_ibl",
            "fn prefilter_specular_cubemap_mips",
            "prefilter_roughness_for_mip",
            "prefilter_lod_for_roughness",
            "fn build_brdf_lut",
            "fn integrate_ggx_specular",
            "fn source_mip_level_for_sample",
            "ggx_prefilter_suppresses_tiny_hdr_firefly_outliers",
            "bake_environment_ibl_owns_specular_mips_and_brdf_lut_product",
            "prefilter_roughness_lod_mapping_is_shared_and_low_roughness_concentrated",
            "source_mip_lod_matches_khronos_filtered_importance_sampling_formula",
            "fn integrate_brdf_lut_cell",
            "ggx_visibility_correlated",
            "fn importance_sample_ggx_local",
            "fn hammersley_2d",
            "fn radical_inverse_van_der_corput",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/pbr_brdf.rs",
        &[
            "KhronosGroup/glTF-Sample-Renderer commit",
            "pub(in crate::render) fn ggx_visibility_correlated",
            "pub(in crate::render) fn brdf_specular_ggx",
            "pub(in crate::render) fn split_sum_brdf_approx",
            "khronos_correlated_ggx_reference_probes_match_bec106e",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "tests/pbr_brdf_parity.rs",
        &[
            "scena.core_pbr_brdf_parity_sweep.v1",
            "core_pbr_brdf_matches_cpu_and_gpu_across_metallic_roughness_sweep",
            "render_scene_cpu_gpu_pair_with_renderer",
            "dielectric-glossy",
            "metal-glossy",
            "metallic must visibly change direct-PBR output",
            "pbr-brdf-parity.json",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/environment_baker.rs",
        &[
            "pub(in crate::render) fn prefilter_specular_cubemap_mips",
            "pub(in crate::render) fn prefilter_specular_cubemap_mips_with_quality",
            "pub(in crate::render) fn build_brdf_lut",
            "pub(in crate::render) fn build_brdf_lut_with_sample_count",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare/environment_baker/source_mips.rs",
        &[
            "sample_source_cubemap_lod",
            "build_source_cubemap_mip_chain",
            "source_mip_resolution",
            "direction_to_face_uv_round_trips_face_centers",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "equirectangular_environment_prepare_generates_ibl_resources",
            "environment_cubemaps",
            "environment_prefilter_passes",
            "environment_brdf_luts",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["Cubemap conversion", "ARCH-ENV-IBL-PREP"],
    );
}

pub(crate) fn check_scene_light_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene.rs",
        &[
            "pub struct LightKey",
            "mod lights;",
            "pub use lights::{",
            "DirectionalLight,",
            "LightBuilder,",
            "StudioLightingHandles",
            "Light(LightKey)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene/lights.rs",
        &["NodeKind::Light"],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/scene/lights.rs",
        &[
            "pub enum Light",
            "pub struct DirectionalLight",
            "pub struct PointLight",
            "pub struct SpotLight",
            "casts_shadows: bool",
            "pub fn directional_light(&mut self, light: DirectionalLight) -> LightBuilder<'_>",
            "pub fn point_light(&mut self, light: PointLight) -> LightBuilder<'_>",
            "pub fn spot_light(&mut self, light: SpotLight) -> LightBuilder<'_>",
            "pub fn light(&self, light: LightKey) -> Option<&Light>",
            "pub const fn casts_shadows",
            "pub const fn with_shadows",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "src/lib.rs",
        &[
            "DirectionalLight",
            "LightBuilder",
            "LightKey",
            "PointLight",
            "SpotLight",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-SCENE-LIGHTS",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "scene_light_components_are_typed_and_node_owned",
            ".directional_light",
            ".point_light",
            ".spot_light",
            "NodeKind::Light",
        ],
    );
}

pub(crate) fn check_direct_light_shading_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/scene/render_nodes.rs",
        &[
            "impl Iterator<Item = (NodeKey, LightKey, Light, Transform)>",
            "self.world_transform(node_key)",
            "map(|transform| (node_key, light_key, light, transform))",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare.rs",
        &[
            "mod lighting;",
            "let lights = PreparedLights::from_scene(scene, origin_shift)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare/primitives.rs",
        &[
            "use super::lighting::{MaterialShadingInput, material_color};",
            "material_color(",
            "MaterialShadingInput {",
            ".map(CameraProjection::camera_position)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare/lighting.rs",
        &[
            "pub(super) struct MaterialShadingInput",
            "pub(super) struct PreparedLights",
            "pub(super) fn from_scene(scene: &Scene, origin_shift: Vec3) -> Self",
            "lights.has_direct_lights() || input.environment.is_active()",
            "shade_pbr_base_color",
            "PbrMaterial::new",
            "punctual_light_contribution",
            "LayeredMaterialLobes",
            "material.clearcoat_factor()",
            "material.clearcoat_roughness_factor()",
            "inverse_square_range_attenuation",
            "input.environment",
            ".pbr_contribution(",
            "input.metallic_roughness_texture",
            "input.occlusion_texture",
            "input.emissive_texture",
            "material.metallic_factor()",
            "material.roughness_factor()",
            "light_direction(transform)",
            "light.illuminance_lux()",
            "light.intensity_candela()",
            "spot_cone_attenuation",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "src/render/prepare/lighting/lobes.rs",
        &[
            "clearcoat_light_contribution",
            "sheen_light_contribution",
            "anisotropy_light_contribution",
            "iridescence_light_contribution",
            "dispersion_light_contribution",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "MaterialDesc::pbr_metallic_roughness",
            "with_color(Color::from_linear_rgb(1.0, 0.0, 0.0))",
            "red-dominant PBR preview output",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-DIRECT-LIGHT-SHADING",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &[
            "direct_lights_tint_pbr_mesh_output",
            "ARCH-DIRECT-LIGHT-SHADING",
        ],
    );
}
