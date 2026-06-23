use crate::scene::Vec3;

use super::normalize_or_z;

/// Bilinearly samples the source cubemap mip chain at the given direction
/// and LOD. This is used by GGX prefiltered importance sampling so tiny HDR
/// emitters are integrated from source mips instead of point-sampled into
/// firefly texels.
pub(super) fn sample_source_cubemap_lod(
    source_mips: &[[Vec<f32>; 6]],
    direction: Vec3,
    mip_level: f32,
) -> Vec3 {
    if source_mips.is_empty() {
        return Vec3::ZERO;
    }
    let max_mip = source_mips.len().saturating_sub(1) as f32;
    let lod = mip_level.clamp(0.0, max_mip);
    let low_mip = lod.floor() as usize;
    let high_mip = (low_mip + 1).min(source_mips.len().saturating_sub(1));
    let t = lod - low_mip as f32;
    let low = sample_source_cubemap_mip(
        &source_mips[low_mip],
        source_mip_resolution(source_mips, low_mip),
        direction,
    );
    if high_mip == low_mip {
        return low;
    }
    let high = sample_source_cubemap_mip(
        &source_mips[high_mip],
        source_mip_resolution(source_mips, high_mip),
        direction,
    );
    lerp_vec3(low, high, t)
}

pub(super) fn build_source_cubemap_mip_chain(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
) -> Vec<[Vec<f32>; 6]> {
    let mut mips = Vec::new();
    let mut current_resolution = resolution.max(1);
    let mut current = source_face_pixels.clone();
    loop {
        mips.push(current.clone());
        if current_resolution == 1 {
            break;
        }
        let next_resolution = (current_resolution / 2).max(1);
        current = std::array::from_fn(|face| {
            downsample_cubemap_face(&current[face], current_resolution, next_resolution)
        });
        current_resolution = next_resolution;
    }
    mips
}

pub(super) fn source_mip_resolution(source_mips: &[[Vec<f32>; 6]], mip: usize) -> u32 {
    let len = source_mips
        .get(mip)
        .and_then(|faces| faces.first())
        .map(Vec::len)
        .unwrap_or(4);
    ((len / 4) as f32).sqrt().round().max(1.0) as u32
}

fn sample_source_cubemap_mip(
    source_face_pixels: &[Vec<f32>; 6],
    resolution: u32,
    direction: Vec3,
) -> Vec3 {
    let resolution = resolution.max(1);
    let normalized = normalize_or_z(direction);
    let (face, u, v) = direction_to_face_uv(normalized);
    let pixel_x = ((u + 1.0) * 0.5 * resolution as f32 - 0.5).clamp(0.0, (resolution - 1) as f32);
    let pixel_y = ((v + 1.0) * 0.5 * resolution as f32 - 0.5).clamp(0.0, (resolution - 1) as f32);
    let x_low = pixel_x.floor() as u32;
    let y_low = pixel_y.floor() as u32;
    let x_high = (x_low + 1).min(resolution - 1);
    let y_high = (y_low + 1).min(resolution - 1);
    let fx = pixel_x - x_low as f32;
    let fy = pixel_y - y_low as f32;
    let face_pixels = &source_face_pixels[face];
    let texel = |x: u32, y: u32| -> Vec3 {
        let index = ((y * resolution + x) * 4) as usize;
        Vec3::new(
            face_pixels[index],
            face_pixels[index + 1],
            face_pixels[index + 2],
        )
    };
    let lt = texel(x_low, y_low);
    let rt = texel(x_high, y_low);
    let lb = texel(x_low, y_high);
    let rb = texel(x_high, y_high);
    let top = lerp_vec3(lt, rt, fx);
    let bottom = lerp_vec3(lb, rb, fx);
    lerp_vec3(top, bottom, fy)
}

fn downsample_cubemap_face(
    source: &[f32],
    source_resolution: u32,
    target_resolution: u32,
) -> Vec<f32> {
    let mut target = vec![0.0_f32; (target_resolution as usize).pow(2) * 4];
    for y in 0..target_resolution {
        for x in 0..target_resolution {
            let source_x = (x * 2).min(source_resolution.saturating_sub(1));
            let source_y = (y * 2).min(source_resolution.saturating_sub(1));
            let mut sum = [0.0_f32; 4];
            let mut count = 0.0_f32;
            for oy in 0..2 {
                for ox in 0..2 {
                    let sx = (source_x + ox).min(source_resolution.saturating_sub(1));
                    let sy = (source_y + oy).min(source_resolution.saturating_sub(1));
                    let index = ((sy * source_resolution + sx) * 4) as usize;
                    if index + 3 >= source.len() {
                        continue;
                    }
                    sum[0] += source[index];
                    sum[1] += source[index + 1];
                    sum[2] += source[index + 2];
                    sum[3] += source[index + 3];
                    count += 1.0;
                }
            }
            let target_index = ((y * target_resolution + x) * 4) as usize;
            let inv = count.max(1.0).recip();
            target[target_index] = sum[0] * inv;
            target[target_index + 1] = sum[1] * inv;
            target[target_index + 2] = sum[2] * inv;
            target[target_index + 3] = sum[3] * inv;
        }
    }
    target
}

fn direction_to_face_uv(direction: Vec3) -> (usize, f32, f32) {
    let abs_x = direction.x.abs();
    let abs_y = direction.y.abs();
    let abs_z = direction.z.abs();
    if abs_x >= abs_y && abs_x >= abs_z {
        if direction.x > 0.0 {
            (0, -direction.z / abs_x, -direction.y / abs_x)
        } else {
            (1, direction.z / abs_x, -direction.y / abs_x)
        }
    } else if abs_y >= abs_z {
        if direction.y > 0.0 {
            (2, direction.x / abs_y, direction.z / abs_y)
        } else {
            (3, direction.x / abs_y, -direction.z / abs_y)
        }
    } else if direction.z > 0.0 {
        (4, direction.x / abs_z, -direction.y / abs_z)
    } else {
        (5, -direction.x / abs_z, -direction.y / abs_z)
    }
}

fn lerp_vec3(start: Vec3, end: Vec3, t: f32) -> Vec3 {
    let clamped = t.clamp(0.0, 1.0);
    Vec3::new(
        start.x + (end.x - start.x) * clamped,
        start.y + (end.y - start.y) * clamped,
        start.z + (end.z - start.z) * clamped,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direction_to_face_uv_round_trips_face_centers() {
        for face in 0..6 {
            let direction = super::super::cubemap_face_direction(face, 0.0, 0.0);
            let (decoded_face, u, v) = direction_to_face_uv(direction);
            assert_eq!(
                decoded_face, face,
                "direction at face {face} center must decode back to that face, got {decoded_face}"
            );
            assert!(
                u.abs() < 1e-4 && v.abs() < 1e-4,
                "face center should round-trip to (0, 0) UV; got ({u}, {v}) for face {face}"
            );
        }
    }
}
