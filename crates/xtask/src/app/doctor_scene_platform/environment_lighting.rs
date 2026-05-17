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
        "src/assets/environment.rs",
        &[
            "EnvironmentSourceKind::EquirectangularHdr",
            "pub fn from_equirectangular_hdr_path",
            "from_equirectangular_hdr_bytes",
            "is_equirectangular_hdr_path",
            "parse_equirectangular_hdr_dimensions",
            "parse_radiance_hdr_preview",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-HDR",
        "src/assets.rs",
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
            "prefilter_passes: 1",
            "brdf_luts: 1",
            "environment.cubemap_faces().is_some()",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ENV-IBL-PREP",
        "src/render/prepare_lifecycle.rs",
        &[
            "prepare::collect_environment_prepare_stats(environment_desc.as_ref())",
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
            "prepare::collect_environment_lighting(environment_desc, self.target.backend)",
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
            "environment.preview_irradiance_rgb()",
            "build_face_pixels_rgba32f",
            "PreparedEnvironmentCubemap",
            "prefilter_specular_cubemap_mips",
            "build_brdf_lut",
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
        "src/render/prepare/environment_prefilter.rs",
        &[
            "pub(in crate::render) fn prefilter_specular_cubemap_mips",
            "pub(in crate::render) fn build_brdf_lut",
            "fn integrate_ggx_specular",
            "fn integrate_brdf_lut_cell",
            "fn importance_sample_ggx_local",
            "fn hammersley_2d",
            "fn radical_inverse_van_der_corput",
            "fn geometry_smith_ggx",
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
            "NodeKind::Light",
        ],
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
        "src/scene.rs",
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
            "use self::lighting::{MaterialShadingInput, PreparedLights, material_color};",
            "let lights = PreparedLights::from_scene(scene, origin_shift)",
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
