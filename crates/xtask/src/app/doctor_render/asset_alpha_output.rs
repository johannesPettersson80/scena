use crate::app::prelude::*;

pub(crate) fn check_asset_api_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets.rs",
        &[
            "pub async fn load_texture",
            "&self,",
            "color_space: TextureColorSpace",
            "Result<TextureHandle, AssetError>",
            "pub fn create_material(&self, material: impl Into<MaterialDesc>) -> MaterialHandle",
            "pub fn default_environment(&self) -> EnvironmentHandle",
            "pub async fn load_environment",
            "pub fn environment(&self, handle: EnvironmentHandle) -> Option<EnvironmentDesc>",
            "pub fn try_geometry",
            "pub fn try_material",
            "pub fn try_texture",
            "pub fn try_environment",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/assets/environment.rs",
        &[
            "pub struct EnvironmentDesc",
            "pub struct EnvironmentDerivative",
            "pub enum EnvironmentSourceKind",
            "pub enum WasmEnvironmentDelivery",
            "pub const fn source_kind(&self) -> EnvironmentSourceKind",
            "pub const fn source_dimensions(&self) -> Option<(u32, u32)>",
            "pub const fn is_equirectangular_hdr(&self) -> bool",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/diagnostics.rs",
        &[
            "pub enum AssetError",
            "UnsupportedRequiredExtension",
            "UnsupportedEnvironmentFormat",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub struct MaterialDesc",
            "pub const DEFAULT_STROKE_WIDTH_PX",
            "pub const DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "pub enum MaterialKind",
            "Unlit",
            "PbrMetallicRoughness",
            "Line",
            "Wireframe",
            "Edge",
            "pub const fn unlit",
            "pub const fn pbr_metallic_roughness",
            "pub const fn line",
            "pub const fn wireframe",
            "pub const fn edge",
            "pub const fn with_stroke_width_px",
            "pub const fn with_edge_angle_threshold_degrees",
            "pub const fn with_base_color_texture",
            "pub const fn with_normal_texture",
            "pub const fn with_metallic_roughness_texture",
            "pub const fn with_occlusion_texture",
            "pub const fn with_emissive_texture",
            "pub const fn with_alpha_mode",
            "pub const fn with_emissive(",
            "pub const fn with_emissive_strength",
            "pub const fn with_double_sided",
            "pub const fn kind(&self) -> MaterialKind",
            "pub const fn base_color(&self) -> Color",
            "pub const fn base_color_texture(&self) -> Option<TextureHandle>",
            "pub const fn normal_texture(&self) -> Option<TextureHandle>",
            "pub const fn metallic_roughness_texture(&self) -> Option<TextureHandle>",
            "pub const fn occlusion_texture(&self) -> Option<TextureHandle>",
            "pub const fn emissive_texture(&self) -> Option<TextureHandle>",
            "pub const fn alpha_mode(&self) -> AlphaMode",
            "pub const fn emissive(&self) -> Color",
            "pub const fn emissive_strength(&self) -> f32",
            "pub const fn metallic_factor(&self) -> f32",
            "pub const fn roughness_factor(&self) -> f32",
            "pub const fn double_sided(&self) -> bool",
            "pub const fn stroke_width_px(&self) -> Option<f32>",
            "pub const fn edge_angle_threshold_degrees(&self) -> Option<f32>",
            "metallic_factor: clamp_unit_or",
            "roughness_factor: clamp_unit_or",
            "cutoff: clamp_unit_or",
            "self.emissive_strength = non_negative_or",
            "DEFAULT_STROKE_WIDTH_PX",
            "DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES",
            "stroke_width_px: Some(positive_or",
            "Some(clamp_degrees_or",
        ],
    );
    check_material_desc_fields_private(root, findings);
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/material.rs",
        &[
            "pub kind:",
            "pub base_color:",
            "pub base_color_texture:",
            "pub normal_texture:",
            "pub metallic_roughness_texture:",
            "pub occlusion_texture:",
            "pub emissive_texture:",
            "pub alpha_mode:",
            "pub emissive:",
            "pub emissive_strength:",
            "pub metallic_factor:",
            "pub roughness_factor:",
            "pub double_sided:",
            "pub struct MaterialTexture",
            "pub enum MaterialTexture",
            "pub type MaterialTexture",
            "pub trait MaterialTexture",
            "pub fn basic(",
            "pub const fn basic(",
            "Basic",
            "basic(",
            "Basic,",
        ],
    );
    forbid_contains(
        root,
        findings,
        "ARCH-ASSET-API",
        "src/lib.rs",
        &["MaterialTexture", "Basic", "basic"],
    );
}

pub(crate) fn check_render_alpha_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum AlphaPipelineStatus",
            "LinearSourceOver",
            "BackendPassthrough",
            "pub alpha_pipeline: AlphaPipelineStatus",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/cpu.rs",
        &[
            "fn blend_source_over(source: Color, destination: Color) -> Color",
            "let blended = blend_source_over(color, cpu_frame.linear_frame[pixel_index])",
            "cpu_frame.linear_frame[pixel_index] = blended",
            "cpu_frame.output.encode_rgba8(blended)",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render.rs",
        &[
            "linear_frame: Option<Vec<Color>>",
            "cpu::clear_cpu",
            "cpu::draw_primitive_cpu",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/build.rs",
        &["linear_frame: (!has_gpu).then(|| vec![Color::BLACK; target.pixel_len()])"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/gpu/pipeline.rs",
        &["blend: Some(wgpu::BlendState::ALPHA_BLENDING)"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/prepare/cpu_bake.rs",
        &["average_sort_depth", "camera_projection.camera_depth"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "src/render/prepare/types.rs",
        &["camera_projection: Option<&'lights CameraProjection>"],
    );
    require_contains(
        root,
        findings,
        "ARCH-RENDER-ALPHA",
        "tests/m1_geometry_materials.rs",
        &[
            "headless_alpha_blends_in_linear_before_output_encoding",
            "prepare_with_assets_sorts_blend_meshes_by_camera_space_depth",
            "headless_gpu_alpha_blends_sorted_asset_meshes_when_available",
            "AlphaPipelineStatus::LinearSourceOver",
        ],
    );
}

pub(crate) fn check_output_stage_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/output.rs",
        &[
            "aces_tonemap",
            "pbr_neutral_tonemap",
            "Tonemapper::PbrNeutral",
            "linear_rgba_to_srgb8",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/color_contract.rs",
        &[
            "pub(super) fn aces_tonemap",
            "fn rrt_and_odt_fit",
            "ACES_INPUT_MATRIX",
            "ACES_OUTPUT_MATRIX",
            "pub(super) fn pbr_neutral_tonemap",
            "Srgb::from_linear",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/output.rs",
        &[
            "fn aces_tonemap(color: vec3<f32>) -> vec3<f32>",
            "fn rrt_and_odt_fit(value: f32) -> f32",
            "camera_position_exposure: vec4<f32>",
            "viewport_near_far: vec4<f32>",
            "color_management: vec4<f32>",
            "fn encode_output_uniform",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/render/gpu/pipeline.rs",
        &[
            "GPU_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb",
            "pass.set_bind_group(0, inputs.output_bind_group, &[])",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "src/diagnostics/capabilities.rs",
        &[
            "pub enum OutputStageStatus",
            "output_stage: OutputStageStatus::PbrNeutralSrgb",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-OUTPUT-STAGE",
        "tests/m1_geometry_materials.rs",
        &["headless_gpu_output_stage_applies_pbr_neutral_srgb_for_pinned_white_fixture"],
    );
}

pub(crate) fn check_fxaa_output_contracts(root: &Path, findings: &mut Vec<Finding>) {
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/diagnostics.rs",
        &["pub fxaa_passes: u64"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render.rs",
        &[
            "fxaa_scratch: Vec<u8>",
            "output::apply_fxaa_rgba8(self.target, &mut self.frame, &mut self.fxaa_scratch)",
            "self.stats.fxaa_passes",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "src/render/output.rs",
        &[
            "pub(super) fn apply_fxaa_rgba8",
            "luma_from_srgb8",
            "FXAA_LUMA_THRESHOLD",
            "pbr_neutral_tonemap",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "tests/m2_lighting_depth_clipping.rs",
        &[
            "fxaa_pass_runs_after_pbr_neutral_without_second_tonemap",
            "stats.fxaa_passes",
            "[160, 160, 160, 255]",
        ],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/specs/public-api.md",
        &["pub fxaa_passes: u64", "tonemapper again"],
    );
    require_contains(
        root,
        findings,
        "ARCH-FXAA-OUTPUT",
        "docs/checklists/m2-lighting-depth-clipping.md",
        &["FXAA pass attached", "ARCH-FXAA-OUTPUT"],
    );
}
