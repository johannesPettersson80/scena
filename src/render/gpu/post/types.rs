use super::super::RasterTarget;
use super::super::pipeline::MeshPipelineSet;
use crate::render::{AntiAliasing, PostBloomConfig, ScreenSpaceAmbientOcclusionConfig};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(in crate::render) struct GpuPostPassCounts {
    pub(in crate::render) ambient_occlusion: u64,
    pub(in crate::render) bloom: u64,
    pub(in crate::render) fxaa: u64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::render) struct GpuPostSettings {
    pub(super) anti_aliasing: AntiAliasing,
    pub(super) bloom: Option<PostBloomConfig>,
    pub(super) ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
}

impl GpuPostSettings {
    pub(in crate::render) const fn new(
        anti_aliasing: AntiAliasing,
        bloom: Option<PostBloomConfig>,
        ambient_occlusion: Option<ScreenSpaceAmbientOcclusionConfig>,
    ) -> Self {
        Self {
            anti_aliasing,
            bloom,
            ambient_occlusion,
        }
    }

    pub(in crate::render::gpu) const fn enabled(self) -> bool {
        matches!(self.anti_aliasing, AntiAliasing::Fxaa)
            || self.bloom.is_some()
            || self.ambient_occlusion.is_some()
    }

    pub(in crate::render::gpu) const fn needs_depth_color(self) -> bool {
        self.ambient_occlusion.is_some()
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn uses_fxaa(self) -> bool {
        matches!(self.anti_aliasing, AntiAliasing::Fxaa)
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn bloom(self) -> Option<PostBloomConfig> {
        self.bloom
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn without_fxaa(self) -> Self {
        Self {
            anti_aliasing: AntiAliasing::None,
            bloom: self.bloom,
            ambient_occlusion: self.ambient_occlusion,
        }
    }

    #[allow(dead_code)]
    pub(in crate::render::gpu) const fn without_bloom_and_fxaa(self) -> Self {
        Self {
            anti_aliasing: AntiAliasing::None,
            bloom: None,
            ambient_occlusion: self.ambient_occlusion,
        }
    }
}

#[derive(Debug)]
pub(in crate::render::gpu) struct PostResources {
    pub(in crate::render::gpu) target: RasterTarget,
    #[allow(dead_code)]
    pub(in crate::render::gpu) scene_texture: wgpu::Texture,
    pub(in crate::render::gpu) scene_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(in crate::render::gpu) ping_texture: wgpu::Texture,
    pub(in crate::render::gpu) ping_view: wgpu::TextureView,
    #[allow(dead_code)]
    pub(in crate::render::gpu) pong_texture: wgpu::Texture,
    pub(in crate::render::gpu) pong_view: wgpu::TextureView,
    pub(in crate::render::gpu) uniform: wgpu::Buffer,
    pub(in crate::render::gpu) ssao_bind_group_layout: wgpu::BindGroupLayout,
    pub(in crate::render::gpu) texture_bind_groups: [wgpu::BindGroup; 3],
    pub(in crate::render::gpu) scene_pipelines: MeshPipelineSet,
    pub(in crate::render::gpu) surface_blit_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    pub(in crate::render::gpu) surface_fxaa_pipeline: Option<wgpu::RenderPipeline>,
    #[allow(dead_code)]
    pub(in crate::render::gpu) surface_bloom_fxaa_pipeline: Option<wgpu::RenderPipeline>,
    pub(in crate::render::gpu) fxaa_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) bloom_pipeline: wgpu::RenderPipeline,
    pub(in crate::render::gpu) ssao_pipeline: wgpu::RenderPipeline,
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
