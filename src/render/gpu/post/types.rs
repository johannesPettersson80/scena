use super::super::RasterTarget;
use super::super::pipeline::MeshPipelineSet;
use crate::render::output::DepthOfFieldPostConfig;
use crate::render::{
    AntiAliasing, PostBloomConfig, ScreenSpaceAmbientOcclusionConfig, ScreenSpaceReflectionConfig,
};

pub(super) const POST_UNIFORM_BYTE_LEN: u64 = std::mem::size_of::<[f32; 12]>() as u64;
pub(super) const POST_UNIFORM_SLOT_COUNT: u64 = 6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render) struct GpuOutputPlan {
    sample_count: u32,
    post_enabled: bool,
    depth_color_enabled: bool,
    scene_linear_capture: bool,
}

impl GpuOutputPlan {
    pub(in crate::render) const fn new(
        anti_aliasing: AntiAliasing,
        bloom: bool,
        ambient_occlusion: bool,
        reflections: bool,
        depth_of_field: bool,
        automatic_exposure: bool,
        scene_linear_capture: bool,
    ) -> Self {
        Self {
            sample_count: anti_aliasing.gpu_sample_count(),
            post_enabled: anti_aliasing.uses_post_fxaa()
                || bloom
                || ambient_occlusion
                || reflections
                || depth_of_field
                || automatic_exposure
                || scene_linear_capture,
            depth_color_enabled: ambient_occlusion || depth_of_field,
            scene_linear_capture,
        }
    }

    pub(in crate::render::gpu) const fn sample_count(self) -> u32 {
        self.sample_count
    }

    pub(in crate::render::gpu) const fn post_enabled(self) -> bool {
        self.post_enabled
    }

    pub(in crate::render::gpu) const fn depth_color_enabled(self) -> bool {
        self.depth_color_enabled
    }

    pub(in crate::render::gpu) const fn scene_linear_capture_enabled(self) -> bool {
        self.scene_linear_capture
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct GpuPostPassCounts {
    pub(in crate::render) screen_space_reflections: u64,
    pub(in crate::render) ambient_occlusion: u64,
    pub(in crate::render) depth_of_field: u64,
    pub(in crate::render) bloom: u64,
    pub(in crate::render) fxaa: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct GpuPostSettings {
    pub(super) anti_aliasing: AntiAliasing,
    pub(super) bloom: Option<PostBloomConfig>,
    pub(super) ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
    pub(super) reflections: Option<ScreenSpaceReflectionConfig>,
    pub(super) depth_of_field: Option<DepthOfFieldPostConfig>,
    pub(super) automatic_exposure: bool,
    pub(super) scene_linear_capture: bool,
}

impl GpuPostSettings {
    pub(in crate::render) const fn new(
        anti_aliasing: AntiAliasing,
        bloom: Option<PostBloomConfig>,
        ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
        reflections: Option<ScreenSpaceReflectionConfig>,
        depth_of_field: Option<DepthOfFieldPostConfig>,
        automatic_exposure: bool,
        scene_linear_capture: bool,
    ) -> Self {
        Self {
            anti_aliasing,
            bloom,
            ambient_occlusion,
            reflections,
            depth_of_field,
            automatic_exposure,
            scene_linear_capture,
        }
    }

    pub(in crate::render::gpu) const fn enabled(self) -> bool {
        self.anti_aliasing.uses_post_fxaa()
            || self.bloom.is_some()
            || self.ambient_occlusion.is_some()
            || self.reflections.is_some()
            || self.depth_of_field.is_some()
            || self.automatic_exposure
            || self.scene_linear_capture
    }

    pub(in crate::render::gpu) const fn needs_depth_color(self) -> bool {
        self.ambient_occlusion.is_some() || self.depth_of_field.is_some()
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn uses_fxaa(self) -> bool {
        self.anti_aliasing.uses_post_fxaa()
    }

    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(in crate::render::gpu) const fn sample_count(self) -> u32 {
        self.anti_aliasing.gpu_sample_count()
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn bloom(self) -> Option<PostBloomConfig> {
        self.bloom
    }

    pub(in crate::render::gpu) const fn reflections(self) -> Option<ScreenSpaceReflectionConfig> {
        self.reflections
    }

    pub(in crate::render::gpu) const fn depth_of_field(self) -> Option<DepthOfFieldPostConfig> {
        self.depth_of_field
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn without_fxaa(self) -> Self {
        Self {
            anti_aliasing: AntiAliasing::None,
            bloom: self.bloom,
            ambient_occlusion: self.ambient_occlusion,
            reflections: self.reflections,
            depth_of_field: self.depth_of_field,
            automatic_exposure: self.automatic_exposure,
            scene_linear_capture: self.scene_linear_capture,
        }
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn without_bloom_and_fxaa(self) -> Self {
        Self {
            anti_aliasing: AntiAliasing::None,
            bloom: None,
            ambient_occlusion: self.ambient_occlusion,
            reflections: self.reflections,
            depth_of_field: self.depth_of_field,
            automatic_exposure: self.automatic_exposure,
            scene_linear_capture: self.scene_linear_capture,
        }
    }
}

#[derive(Debug)]
pub(in crate::render::gpu) struct LinearSceneReadbackResources {
    // On wasm the buffer is allocated but never read: a synchronous readback
    // cannot complete inside one browser task, so only native code maps it.
    #[cfg_attr(
        any(target_arch = "wasm32", not(any(feature = "scene-host", test))),
        allow(dead_code)
    )]
    pub(in crate::render::gpu) buffer: wgpu::Buffer,
    pub(in crate::render::gpu) padded_bytes_per_row: u32,
}

#[derive(Debug)]
pub(in crate::render::gpu) struct PostResources {
    pub(in crate::render::gpu) target: RasterTarget,
    #[allow(dead_code)]
    pub(in crate::render::gpu) scene_texture: wgpu::Texture,
    pub(in crate::render::gpu) scene_view: wgpu::TextureView,
    pub(in crate::render::gpu) linear_scene_readback: Option<LinearSceneReadbackResources>,
    #[allow(dead_code)]
    pub(in crate::render::gpu) ping_texture: wgpu::Texture,
    pub(in crate::render::gpu) ping_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(in crate::render::gpu) pong_texture: wgpu::Texture,
    pub(in crate::render::gpu) pong_view: wgpu::TextureView,
    pub(in crate::render::gpu) uniform: wgpu::Buffer,
    pub(in crate::render::gpu) uniform_staging: wgpu::Buffer,
    pub(in crate::render::gpu) texture_bind_groups: [wgpu::BindGroup; 3],
    pub(in crate::render::gpu) depth_texture_bind_groups: Option<[wgpu::BindGroup; 3]>,
    pub(in crate::render::gpu) scene_pipelines: MeshPipelineSet,
    pub(in crate::render::gpu) scene_msaa4_pipelines: MeshPipelineSet,
    pub(in crate::render::gpu) scene_msaa8_pipelines: Option<MeshPipelineSet>,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    pub(in crate::render::gpu) output_blit_pipeline: wgpu::RenderPipeline,
    #[allow(dead_code)]
    pub(in crate::render::gpu) readback_blit_pipeline: Option<wgpu::RenderPipeline>,
    pub(in crate::render::gpu) surface_blit_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    pub(in crate::render::gpu) surface_fxaa_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    pub(in crate::render::gpu) surface_bloom_fxaa_pipeline: Option<wgpu::RenderPipeline>,
    pub(in crate::render::gpu) fxaa_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) ssr_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) bloom_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) ssao_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) depth_of_field_pipeline: wgpu::RenderPipeline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::render::gpu) struct PostChainOutput {
    pub(super) slot: PostTextureSlot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PostTextureSlot {
    Scene,
    Ping,
    Pong,
}

impl PostTextureSlot {
    pub(super) const fn alternate(self) -> Self {
        match self {
            Self::Scene | Self::Pong => Self::Ping,
            Self::Ping => Self::Pong,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u64)]
pub(super) enum PostUniformSlot {
    Reflections = 0,
    AmbientOcclusion = 1,
    DepthOfField = 2,
    Bloom = 3,
    Fxaa = 4,
    Surface = 5,
}

impl PostUniformSlot {
    pub(super) const fn byte_offset(self) -> u64 {
        self as u64 * POST_UNIFORM_BYTE_LEN
    }
}
