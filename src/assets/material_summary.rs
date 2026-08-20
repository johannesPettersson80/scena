use super::TextureDesc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct EffectiveMaterialPbr {
    pub metallic_mean: f32,
    pub roughness_mean: f32,
}

pub(super) fn metallic_roughness_texture_means(texture: &TextureDesc) -> Option<(f32, f32)> {
    const MAX_SAMPLES_PER_AXIS: u32 = 64;

    let (width, height, rgba8) = texture.decoded_rgba8()?;
    if width == 0 || height == 0 {
        return None;
    }
    let samples_x = width.min(MAX_SAMPLES_PER_AXIS);
    let samples_y = height.min(MAX_SAMPLES_PER_AXIS);
    let mut roughness = 0_u64;
    let mut metallic = 0_u64;
    let mut sample_count = 0_u64;
    for sample_y in 0..samples_y {
        let y = ((u64::from(sample_y) * 2 + 1) * u64::from(height) / (u64::from(samples_y) * 2))
            .min(u64::from(height - 1)) as usize;
        for sample_x in 0..samples_x {
            let x = ((u64::from(sample_x) * 2 + 1) * u64::from(width) / (u64::from(samples_x) * 2))
                .min(u64::from(width - 1)) as usize;
            let offset = (y * width as usize + x).checked_mul(4)?;
            let pixel = rgba8.get(offset..offset + 4)?;
            roughness = roughness.saturating_add(u64::from(pixel[1]));
            metallic = metallic.saturating_add(u64::from(pixel[2]));
            sample_count = sample_count.saturating_add(1);
        }
    }
    let denominator = (sample_count.max(1) * 255) as f32;
    Some((
        roughness as f32 / denominator,
        metallic as f32 / denominator,
    ))
}
