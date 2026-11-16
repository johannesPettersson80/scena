//! Screen-space ambient occlusion evaluated ahead of the colour pass.
//!
//! The post-process filter in `post/ssao.wgsl` darkens the finished frame,
//! including direct light and highlights, and can only see a few pixels. This
//! pass instead reads the depth prepass, rebuilds view-space geometry, and
//! writes per-pixel visibility that the colour pass multiplies into
//! environment lighting alone. Objects gain a soft contact shadow where they
//! meet the floor and creases lose their ambient fill, while lit surfaces keep
//! their colour.
//!
//! Visibility travels in a storage buffer rather than a texture: the colour
//! pass already binds the sixteen sampled textures WebGPU guarantees per
//! stage. The passes are therefore compute passes, and the feature is only
//! active where the texture-array colour shader runs natively. Browser targets
//! bind a one-element placeholder and keep the post-process filter.

pub(super) const AMBIENT_OCCLUSION_SHADER: &str = include_str!("ambient_occlusion.wgsl");

#[cfg(not(target_arch = "wasm32"))]
mod pass;

#[cfg(not(target_arch = "wasm32"))]
pub(super) use pass::AmbientOcclusionResources;

const VISIBILITY_BYTES_PER_PIXEL: u64 = std::mem::size_of::<f32>() as u64;

/// The one-element buffer bound where no occlusion pass runs.
pub(super) fn placeholder_visibility(device: &wgpu::Device) -> wgpu::Buffer {
    create_visibility_buffer(device, "scena.ambient_occlusion.placeholder", 1)
}

/// A buffer of `pixels` visibility values, initialised to fully visible so a
/// frame that never runs the pass still lights correctly.
fn create_visibility_buffer(
    device: &wgpu::Device,
    label: &'static str,
    pixels: u64,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some(label),
        size: pixels.max(1) * VISIBILITY_BYTES_PER_PIXEL,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: true,
    });
    let visible = 1.0_f32.to_ne_bytes().repeat(pixels.max(1) as usize);
    buffer
        .slice(..)
        .get_mapped_range_mut()
        .copy_from_slice(&visible);
    buffer.unmap();
    buffer
}
