use crate::assets::TextureFilter;

pub(super) fn mip_level_extents(
    width: u32,
    height: u32,
    filter: Option<TextureFilter>,
) -> Vec<(u32, u32)> {
    let mut extents = vec![(width.max(1), height.max(1))];
    if !texture_filter_uses_mipmaps(filter) {
        return extents;
    }
    while extents
        .last()
        .is_some_and(|(width, height)| *width > 1 || *height > 1)
    {
        let (width, height) = *extents.last().expect("at least one mip extent exists");
        extents.push(((width / 2).max(1), (height / 2).max(1)));
    }
    extents
}

pub(super) fn downsample_rgba8_mip(
    previous: &[u8],
    previous_width: u32,
    previous_height: u32,
    next_width: u32,
    next_height: u32,
) -> Vec<u8> {
    let mut next = vec![0; next_width as usize * next_height as usize * 4];
    for y in 0..next_height {
        for x in 0..next_width {
            let mut sum = [0_u32; 4];
            let mut count = 0_u32;
            for sample_y in [y.saturating_mul(2), y.saturating_mul(2).saturating_add(1)] {
                for sample_x in [x.saturating_mul(2), x.saturating_mul(2).saturating_add(1)] {
                    let source_x = sample_x.min(previous_width.saturating_sub(1));
                    let source_y = sample_y.min(previous_height.saturating_sub(1));
                    let source = ((source_y * previous_width + source_x) * 4) as usize;
                    if let Some(pixel) = previous.get(source..source + 4) {
                        for channel in 0..4 {
                            sum[channel] += u32::from(pixel[channel]);
                        }
                        count += 1;
                    }
                }
            }
            let target = ((y * next_width + x) * 4) as usize;
            for channel in 0..4 {
                next[target + channel] = (sum[channel] / count.max(1)) as u8;
            }
        }
    }
    next
}

fn texture_filter_uses_mipmaps(filter: Option<TextureFilter>) -> bool {
    matches!(
        filter,
        Some(
            TextureFilter::NearestMipmapNearest
                | TextureFilter::LinearMipmapNearest
                | TextureFilter::NearestMipmapLinear
                | TextureFilter::LinearMipmapLinear
        )
    )
}

#[cfg(test)]
mod tests {
    use crate::assets::{TextureFilter, TextureSamplerDesc, TextureWrap};
    use crate::render::gpu::materials::MaterialTextureUpload;

    #[test]
    fn material_texture_upload_counts_requested_mip_levels() {
        let upload = MaterialTextureUpload {
            width: 4,
            height: 2,
            rgba8: &[255; 32],
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: TextureSamplerDesc::new(
                None,
                Some(TextureFilter::LinearMipmapLinear),
                TextureWrap::Repeat,
                TextureWrap::Repeat,
            ),
            uses_decoded_texture: true,
        };

        assert_eq!(
            super::mip_level_extents(upload.width, upload.height, upload.sampler.min_filter()),
            vec![(4, 2), (2, 1), (1, 1)]
        );
        assert_eq!(upload.byte_len(), 44);
    }

    #[test]
    fn material_texture_mip_downsample_averages_rgba8_pixels() {
        let previous = [
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ];

        let mip = super::downsample_rgba8_mip(&previous, 2, 2, 1, 1);

        assert_eq!(mip, vec![127, 127, 127, 255]);
    }
}
