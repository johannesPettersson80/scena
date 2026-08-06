//! Renderer-owned shading lookup tables uploaded as uniform blocks.
//!
//! These tables are read with a runtime index. Baked into WGSL as module-scope
//! `const` arrays they cannot be expressed by hardware without an indexed
//! constant-register file, so the driver expands every read into a select chain
//! over each element; a uniform block is a memory load instead. Uniform buffers
//! also cost no texture unit or sampler, which matters because the fragment
//! stage already sits at exactly 16 of each — the ceiling `downlevel_defaults()`
//! imposes on every backend scena builds, not only WebGL2.

use crate::render::area_ltc;

/// Byte length of the packed LTC uniform block: two row-major 16x16 tables of
/// `vec4<f32>`. Well inside WebGL2's 16 KiB `max_uniform_buffer_binding_size`.
pub(super) const LTC_TABLE_BYTE_LEN: u64 = 2 * 256 * 4 * 4;

/// WebGL2's `max_uniform_buffer_binding_size` floor. Enforced at compile time so
/// a table that outgrows the smallest backend cannot reach a device.
const _: () = assert!(LTC_TABLE_BYTE_LEN <= 16_384);

/// Packs both LTC tables into the byte layout `LtcTables` declares in
/// `area_ltc_tables.wgsl`: `table_1` then `table_2`, each row-major `y * 16 + x`.
/// `array<vec4<f32>, N>` has a std140 stride of 16 bytes, so the Rust
/// `[[[f32; 4]; 16]; 16]` layout transfers with no padding.
pub(super) fn ltc_table_bytes() -> Vec<u8> {
    let mut bytes = Vec::with_capacity(LTC_TABLE_BYTE_LEN as usize);
    for table in [area_ltc::LTC_1, area_ltc::LTC_2] {
        for row in table.iter() {
            for texel in row.iter() {
                for channel in texel.iter() {
                    bytes.extend_from_slice(&channel.to_le_bytes());
                }
            }
        }
    }
    debug_assert_eq!(bytes.len() as u64, LTC_TABLE_BYTE_LEN);
    bytes
}

pub(super) fn create_ltc_table_buffer(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.shading.ltc_tables"),
        size: LTC_TABLE_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&buffer, 0, &ltc_table_bytes());
    buffer
}

/// Edge length of the BRDF table the shader reads.
///
/// The baked table is `BRDF_LUT_SIZE`, but a `vec2<f32>` array pads to a 16-byte
/// std140 stride, so 64x64 would need 32 KiB - twice WebGL2's uniform-block
/// floor. Packing two texels per `vec4` and halving the edge gives 8 KiB, and
/// `brdf_table_resample_error_stays_far_below_the_analytic_fit` measures what
/// that costs against the same reference the analytic fit was measured against.
pub(crate) const BRDF_TABLE_SIZE: u32 = 32;

/// 512 `vec4`s of two packed `(scale, bias)` texels.
pub(crate) const BRDF_TABLE_BYTE_LEN: u64 = (BRDF_TABLE_SIZE * BRDF_TABLE_SIZE / 2) as u64 * 16;

const _: () = assert!(BRDF_TABLE_BYTE_LEN <= 16_384);

/// Box-filters the baked table down to `BRDF_TABLE_SIZE` and packs it in the
/// order `BrdfLutTable` declares. Averaging rather than point-sampling keeps the
/// resampled cell equal to the mean of the cells it replaces, which is what the
/// integral over that cell means.
pub(crate) fn brdf_table_bytes(baked: &[f32], baked_size: u32) -> Vec<u8> {
    let mut packed = Vec::with_capacity((BRDF_TABLE_SIZE * BRDF_TABLE_SIZE) as usize * 2);
    let ratio = (baked_size / BRDF_TABLE_SIZE).max(1) as usize;
    for y in 0..BRDF_TABLE_SIZE as usize {
        for x in 0..BRDF_TABLE_SIZE as usize {
            let mut scale = 0.0_f64;
            let mut bias = 0.0_f64;
            let mut samples = 0.0_f64;
            for dy in 0..ratio {
                for dx in 0..ratio {
                    let sy = (y * ratio + dy).min(baked_size as usize - 1);
                    let sx = (x * ratio + dx).min(baked_size as usize - 1);
                    let index = (sy * baked_size as usize + sx) * 2;
                    let (Some(s), Some(b)) = (baked.get(index), baked.get(index + 1)) else {
                        continue;
                    };
                    scale += f64::from(*s);
                    bias += f64::from(*b);
                    samples += 1.0;
                }
            }
            let samples = samples.max(1.0);
            packed.push((scale / samples) as f32);
            packed.push((bias / samples) as f32);
        }
    }
    let mut bytes = Vec::with_capacity(BRDF_TABLE_BYTE_LEN as usize);
    for value in &packed {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.resize(BRDF_TABLE_BYTE_LEN as usize, 0);
    bytes
}

pub(super) fn create_brdf_table_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    prepared: Option<&crate::render::prepare::PreparedEnvironmentCubemap>,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("scena.shading.brdf_table"),
        size: BRDF_TABLE_BYTE_LEN,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    // With no environment prepared the block stays zeroed; the shader still
    // reads it, and a zero DFG term is the correct "no image-based specular".
    if let Some(prepared) = prepared {
        queue.write_buffer(
            &buffer,
            0,
            &brdf_table_bytes(&prepared.brdf_lut, prepared.brdf_lut_size),
        );
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ltc_uniform_bytes_reproduce_the_cpu_reference_tables() {
        let bytes = ltc_table_bytes();
        assert_eq!(bytes.len() as u64, LTC_TABLE_BYTE_LEN);

        // Reinterpret the uploaded bytes the way the shader indexes them and
        // compare against the CPU renderer's own tables. This is the only thing
        // keeping the GPU upload and the Rust evaluator on the same data.
        let mut offset = 0usize;
        for table in [area_ltc::LTC_1, area_ltc::LTC_2] {
            for (y, row) in table.iter().enumerate() {
                for (x, texel) in row.iter().enumerate() {
                    for (channel, expected) in texel.iter().enumerate() {
                        let raw: [u8; 4] = bytes[offset..offset + 4]
                            .try_into()
                            .expect("uniform block is a whole number of f32 values");
                        let actual = f32::from_le_bytes(raw);
                        assert_eq!(
                            actual, *expected,
                            "LTC texel [{y}][{x}].{channel} must survive the upload"
                        );
                        offset += 4;
                    }
                }
            }
        }
        assert_eq!(offset, bytes.len());
    }
}
