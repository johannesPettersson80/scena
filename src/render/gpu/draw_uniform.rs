/// One DrawUniform entry packs world_from_model + normal_from_model = 32
/// floats = 128 bytes. WebGPU requires dynamic-offset uniform binding offsets
/// to be aligned to `min_uniform_buffer_offset_alignment`, which is 256 on
/// every wgpu adapter we target. We pad each entry up to 256 bytes so the
/// runtime stride matches the alignment requirement; the trailing 128 bytes
/// per entry are zero-padding.
pub(super) const DRAW_UNIFORM_ENTRY_SIZE: u64 = 128;
pub(super) const DRAW_UNIFORM_ENTRY_STRIDE: u64 = 256;

pub(super) fn create_draw_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("scena.draw.bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: std::num::NonZeroU64::new(DRAW_UNIFORM_ENTRY_SIZE),
            },
            count: None,
        }],
    })
}

pub(super) fn create_draw_uniform_buffer(device: &wgpu::Device, entry_count: u64) -> wgpu::Buffer {
    let size = DRAW_UNIFORM_ENTRY_STRIDE.saturating_mul(entry_count.max(1));
    device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.draw.uniform"),
        size,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    })
}

pub(super) fn create_draw_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("scena.draw.bind_group"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: uniform,
                offset: 0,
                size: std::num::NonZeroU64::new(DRAW_UNIFORM_ENTRY_SIZE),
            }),
        }],
    })
}

/// Encodes a `Vec<DrawUniformValue>` into a packed byte buffer where each
/// entry occupies `DRAW_UNIFORM_ENTRY_STRIDE` bytes. The first
/// `DRAW_UNIFORM_ENTRY_SIZE` bytes of each entry hold the world_from_model +
/// normal_from_model matrices; the trailing bytes are zero padding required
/// by `min_uniform_buffer_offset_alignment` for dynamic-offset binding.
pub(super) fn encode_draw_uniform_bytes(
    values: &[(/*world*/ [f32; 16], /*normal*/ [f32; 16])],
) -> Vec<u8> {
    let mut bytes = vec![0u8; values.len().max(1) * DRAW_UNIFORM_ENTRY_STRIDE as usize];
    for (entry_index, (world_from_model, normal_from_model)) in values.iter().enumerate() {
        let entry_offset = entry_index * DRAW_UNIFORM_ENTRY_STRIDE as usize;
        for (i, value) in world_from_model.iter().enumerate() {
            let byte_offset = entry_offset + i * 4;
            bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
        for (i, value) in normal_from_model.iter().enumerate() {
            let byte_offset = entry_offset + 64 + i * 4;
            bytes[byte_offset..byte_offset + 4].copy_from_slice(&value.to_ne_bytes());
        }
    }
    bytes
}
