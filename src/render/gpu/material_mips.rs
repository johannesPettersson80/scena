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
    // Stage B2: delegate to the `image` crate's Triangle (bilinear) filter.
    // For the 2:1 → 1 mip-chain case Triangle produces the same average as
    // the prior hand-rolled box filter (the existing pinning tests
    // continue to pass byte-for-byte). For larger source mips (e.g.
    // 256×256 → 128×128 with a sharp edge), Triangle filters more
    // gracefully than box-averaging, improving texture sampling quality.
    let buffer: image::ImageBuffer<image::Rgba<u8>, Vec<u8>> =
        image::ImageBuffer::from_raw(previous_width, previous_height, previous.to_vec())
            .expect("downsample input must be width × height × 4 RGBA bytes");
    let resized = image::imageops::resize(
        &buffer,
        next_width.max(1),
        next_height.max(1),
        image::imageops::FilterType::Triangle,
    );
    resized.into_raw()
}

pub(super) fn downsample_rgba16f_mip(
    previous: &[u16],
    previous_width: u32,
    previous_height: u32,
    next_width: u32,
    next_height: u32,
) -> Vec<u16> {
    let mut output = Vec::with_capacity(next_width as usize * next_height as usize * 4);
    for y in 0..next_height {
        let y0 = (u64::from(y) * u64::from(previous_height) / u64::from(next_height)) as u32;
        let y1 = ((u64::from(y + 1) * u64::from(previous_height)).div_ceil(u64::from(next_height)))
            .max(u64::from(y0 + 1))
            .min(u64::from(previous_height)) as u32;
        for x in 0..next_width {
            let x0 = (u64::from(x) * u64::from(previous_width) / u64::from(next_width)) as u32;
            let x1 = ((u64::from(x + 1) * u64::from(previous_width))
                .div_ceil(u64::from(next_width)))
            .max(u64::from(x0 + 1))
            .min(u64::from(previous_width)) as u32;
            let sample_count = ((x1 - x0) * (y1 - y0)) as f32;
            for channel in 0..4 {
                let mut sum = 0.0f32;
                for source_y in y0..y1 {
                    for source_x in x0..x1 {
                        let index = ((source_y * previous_width + source_x) * 4 + channel) as usize;
                        sum += half::f16::from_bits(previous[index]).to_f32();
                    }
                }
                output.push(half::f16::from_f32(sum / sample_count).to_bits());
            }
        }
    }
    output
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
            rgba16f_bits: None,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            sampler: TextureSamplerDesc::new(
                None,
                Some(TextureFilter::LinearMipmapLinear),
                TextureWrap::Repeat,
                TextureWrap::Repeat,
            ),
            mip_policy: crate::assets::TextureMipPolicy::Generate,
            #[cfg(target_arch = "wasm32")]
            browser_image: None,
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

        // Stage B2: switched from a hand-rolled truncating box filter to
        // the `image` crate's Triangle (bilinear) filter. Triangle uses
        // round-half-up rather than truncate, so the average of 4 channels
        // = 510/4 = 127.5 rounds to 128 (not 127). Strictly more correct;
        // matches GIMP/Photoshop default mip resampling.
        assert_eq!(mip, vec![128, 128, 128, 255]);
    }

    /// Stage B2 pin: 4×4 checker → 2×2 with Triangle filter. Triangle
    /// uses a 4-tap kernel that includes the diagonal neighbours, so each
    /// output pixel is a weighted average of 16 inputs (with edge weights
    /// reduced). For our checker pattern this yields 130 not the box
    /// filter's 127.
    #[test]
    fn material_texture_mip_downsample_4x4_checker_pins_midgrey() {
        let mut previous = Vec::with_capacity(4 * 4 * 4);
        for y in 0..4 {
            for x in 0..4 {
                if (x + y) % 2 == 0 {
                    previous.extend_from_slice(&[255, 0, 0, 255]);
                } else {
                    previous.extend_from_slice(&[0, 0, 0, 255]);
                }
            }
        }
        let mip = super::downsample_rgba8_mip(&previous, 4, 4, 2, 2);
        for px in 0..4 {
            let i = px * 4;
            assert!(
                (120..=135).contains(&mip[i]),
                "pixel {px} R {} should be Triangle-resampled mid-grey",
                mip[i]
            );
            assert_eq!(mip[i + 1], 0, "pixel {px} G");
            assert_eq!(mip[i + 2], 0, "pixel {px} B");
            assert_eq!(mip[i + 3], 255, "pixel {px} A");
        }
    }

    #[test]
    fn a14_linear_float_mip_downsample_preserves_hdr_range() {
        let previous = [
            half::f16::from_f32(2.0).to_bits(),
            half::f16::from_f32(0.0).to_bits(),
            half::f16::from_f32(0.0).to_bits(),
            half::f16::from_f32(1.0).to_bits(),
            half::f16::from_f32(4.0).to_bits(),
            half::f16::from_f32(0.0).to_bits(),
            half::f16::from_f32(0.0).to_bits(),
            half::f16::from_f32(1.0).to_bits(),
        ];
        let mip = super::downsample_rgba16f_mip(&previous, 2, 1, 1, 1);
        assert_eq!(half::f16::from_bits(mip[0]).to_f32(), 3.0);
        assert_eq!(half::f16::from_bits(mip[3]).to_f32(), 1.0);
    }
}
