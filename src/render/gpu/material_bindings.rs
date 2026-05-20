#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
pub(super) enum MaterialTextureBindingMode {
    Texture2d,
    Texture2dArray,
}

impl MaterialTextureBindingMode {
    pub(super) fn view_dimension(self) -> wgpu::TextureViewDimension {
        match self {
            Self::Texture2d => wgpu::TextureViewDimension::D2,
            Self::Texture2dArray => wgpu::TextureViewDimension::D2Array,
        }
    }

    pub(super) fn supports_batching(self) -> bool {
        matches!(self, Self::Texture2dArray)
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct TextureBindingIndices {
    pub(super) sampler: u32,
    pub(super) texture: u32,
}

pub(super) const BASE_COLOR_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 0,
    texture: 1,
};
pub(super) const NORMAL_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 3,
    texture: 4,
};
pub(super) const METALLIC_ROUGHNESS_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 5,
    texture: 6,
};
pub(super) const OCCLUSION_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 7,
    texture: 8,
};
pub(super) const EMISSIVE_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 9,
    texture: 10,
};
pub(super) const CLEARCOAT_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 11,
    texture: 12,
};
pub(super) const CLEARCOAT_ROUGHNESS_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 13,
    texture: 14,
};
pub(super) const CLEARCOAT_NORMAL_BINDINGS: TextureBindingIndices = TextureBindingIndices {
    sampler: 15,
    texture: 16,
};

pub(super) const MATERIAL_TEXTURE_BINDING_INDICES: [TextureBindingIndices; 8] = [
    BASE_COLOR_BINDINGS,
    NORMAL_BINDINGS,
    METALLIC_ROUGHNESS_BINDINGS,
    OCCLUSION_BINDINGS,
    EMISSIVE_BINDINGS,
    CLEARCOAT_BINDINGS,
    CLEARCOAT_ROUGHNESS_BINDINGS,
    CLEARCOAT_NORMAL_BINDINGS,
];

pub(super) fn create_material_texture_layout_entries(
    texture_binding_mode: MaterialTextureBindingMode,
) -> Vec<wgpu::BindGroupLayoutEntry> {
    let mut entries = Vec::with_capacity(MATERIAL_TEXTURE_BINDING_INDICES.len() * 2);
    for bindings in MATERIAL_TEXTURE_BINDING_INDICES {
        entries.push(texture_sampler_layout_entry(bindings.sampler));
        entries.push(texture_layout_entry(bindings.texture, texture_binding_mode));
    }
    entries
}

fn texture_sampler_layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

fn texture_layout_entry(
    binding: u32,
    texture_binding_mode: MaterialTextureBindingMode,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: texture_binding_mode.view_dimension(),
            multisampled: false,
        },
        count: None,
    }
}
