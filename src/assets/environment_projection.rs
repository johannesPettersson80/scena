//! Equirectangular HDR ↔ cubemap face direction math. Extracted from
//! `environment.rs` so the asset module stays under the KISS-SIZE cap; the
//! projection logic is purely numerical and has no dependency on the rest
//! of the asset surface.

use crate::scene::Vec3;

use super::environment_hdr::DecodedEquirectangular;

/// Bilinear-sample a Radiance equirectangular HDR in the given world-space
/// direction. Direction → spherical (longitude, latitude) → equirect UV.
/// `longitude` runs around Y; `latitude` runs from -π/2 (south pole) to
/// +π/2 (north pole). UV (0,0) is top-left of the equirect image.
///
/// Equirect convention: the image centre column (u=0.5) maps to +X, +Z lies
/// at the right quarter (u=0.75), -Z at the left quarter (u=0.25), and -X at
/// the seam (u=0/u=1). This is three.js's `equirectUv` mapping, which
/// model-viewer and the Khronos glTF Sample Viewer share, so an HDR lights a
/// glTF scene from the same side in scena as in those viewers.
pub(super) fn sample_equirectangular(
    equirect: &DecodedEquirectangular,
    direction: Vec3,
) -> [f32; 3] {
    if equirect.width == 0 || equirect.height == 0 {
        return [0.0, 0.0, 0.0];
    }
    let length =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        return [0.0, 0.0, 0.0];
    }
    let inv = length.recip();
    let dx = direction.x * inv;
    let dy = direction.y * inv;
    let dz = direction.z * inv;
    let longitude = dz.atan2(dx);
    let latitude = dy.clamp(-1.0, 1.0).asin();
    let u = (longitude / (2.0 * std::f32::consts::PI)) + 0.5;
    let u = ((u % 1.0) + 1.0) % 1.0;
    let v = 0.5 - latitude / std::f32::consts::PI;
    let fx = u * equirect.width as f32 - 0.5;
    let fy = v * equirect.height as f32 - 0.5;
    let x0 = fx.floor() as i32;
    let y0 = fy.floor() as i32;
    let x1 = x0 + 1;
    let y1 = y0 + 1;
    let tx = fx - x0 as f32;
    let ty = fy - y0 as f32;
    let s00 = fetch_equirect_pixel(equirect, x0, y0);
    let s10 = fetch_equirect_pixel(equirect, x1, y0);
    let s01 = fetch_equirect_pixel(equirect, x0, y1);
    let s11 = fetch_equirect_pixel(equirect, x1, y1);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    [
        lerp(lerp(s00[0], s10[0], tx), lerp(s01[0], s11[0], tx), ty),
        lerp(lerp(s00[1], s10[1], tx), lerp(s01[1], s11[1], tx), ty),
        lerp(lerp(s00[2], s10[2], tx), lerp(s01[2], s11[2], tx), ty),
    ]
}

fn fetch_equirect_pixel(equirect: &DecodedEquirectangular, x: i32, y: i32) -> [f32; 3] {
    let width = equirect.width as i32;
    let height = equirect.height as i32;
    let wrapped_x = ((x % width) + width) % width;
    let clamped_y = y.clamp(0, height - 1);
    let index = (clamped_y * width + wrapped_x) as usize;
    equirect.pixels[index]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An 8x4 image whose red channel is the square of the column number, so
    /// a sample tells which pair of columns it was filtered from.
    fn numbered_columns() -> DecodedEquirectangular {
        let (width, height) = (8, 4);
        DecodedEquirectangular {
            width,
            height,
            pixels: (0..width * height)
                .map(|index| [((index % width) * (index % width)) as f32, 0.0, 0.0])
                .collect(),
        }
    }

    fn assert_columns(direction: Vec3, left: u32, right: u32) {
        let sample = sample_equirectangular(&numbered_columns(), direction)[0];
        let expected = ((left * left + right * right) as f32) / 2.0;
        assert!(
            (sample - expected).abs() < 1.0e-3,
            "{direction:?} sampled {sample}, expected halfway between columns {left} and {right}",
        );
    }

    #[test]
    fn directions_map_to_columns_as_in_three_js() {
        // u = atan2(z, x) / 2pi + 0.5, as three.js's `equirectUv`.
        // +X is the centre of the image.
        assert_columns(Vec3::new(1.0, 0.0, 0.0), 3, 4);
        // +Z is the right quarter, -Z the left quarter.
        assert_columns(Vec3::new(0.0, 0.0, 1.0), 5, 6);
        assert_columns(Vec3::new(0.0, 0.0, -1.0), 1, 2);
        // -X is the seam, between the last column and the first.
        assert_columns(Vec3::new(-1.0, 0.0, 0.0), 7, 0);
    }
}
