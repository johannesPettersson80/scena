use super::*;
use crate::scene::Vec3;

use crate::assets::environment_projection::sample_equirectangular;

impl EnvironmentCubemapFaces {
    /// Parses the bundled `SCENA_CUBEMAP_V1` text fixture into per-face
    /// radiance triplets. Returns `None` if the magic header is missing or any
    /// face block fails to provide three finite, non-negative channel values.
    pub fn try_parse_fixture(text: &str) -> Option<Self> {
        let mut lines = text.lines();
        if lines.next()?.trim() != "SCENA_CUBEMAP_V1" {
            return None;
        }
        let mut radiance = [[0.0_f32; 3]; 6];
        let mut seen = [false; 6];
        let mut current_face: Option<usize> = None;
        let mut resolution = DEFAULT_ENVIRONMENT_CUBEMAP_FACE_RESOLUTION;
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Some(face_label) = line
                .strip_prefix("[face.")
                .and_then(|rest| rest.strip_suffix(']'))
            {
                current_face = match face_label {
                    "px" => Some(0),
                    "nx" => Some(1),
                    "py" => Some(2),
                    "ny" => Some(3),
                    "pz" => Some(4),
                    "nz" => Some(5),
                    _ => None,
                };
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if key == "resolution" {
                    if let Ok(parsed) = value.parse::<u32>()
                        && parsed > 0
                    {
                        resolution = parsed;
                    }
                    continue;
                }
                if key == "radiance"
                    && let Some(face) = current_face
                {
                    let channels = parse_radiance_triplet(value)?;
                    radiance[face] = channels;
                    seen[face] = true;
                }
            }
        }
        seen.iter().all(|present| *present).then_some(Self {
            face_radiance: radiance,
            resolution,
            face_pixels: None,
        })
    }

    /// Project an equirectangular HDR into 6 cubemap face grids at
    /// `face_resolution` per side. For each face pixel, derives the world
    /// direction, converts to spherical (longitude, latitude), and samples
    /// the equirectangular HDR via bilinear filtering. The resulting cubemap
    /// preserves real per-pixel radiance — the prefiltered specular pass
    /// downstream then produces the bright sheen/reflections that pure
    /// face-centre interpolation cannot.
    pub fn from_equirectangular(
        equirect: &DecodedEquirectangular,
        face_resolution: u32,
    ) -> Option<Self> {
        if equirect.width == 0 || equirect.height == 0 || face_resolution == 0 {
            return None;
        }
        let resolution = face_resolution;
        let face_pixel_count = (resolution as usize).pow(2);
        let mut faces: [Vec<[f32; 3]>; 6] =
            std::array::from_fn(|_| vec![[0.0, 0.0, 0.0]; face_pixel_count]);
        let mut face_radiance = [[0.0_f32; 3]; 6];
        for (face_index, face) in faces.iter_mut().enumerate() {
            let mut sum = [0.0_f64; 3];
            for y in 0..resolution {
                for x in 0..resolution {
                    let u = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let direction = cube_face_direction(face_index, u, v);
                    let sample = sample_equirectangular(equirect, direction);
                    let pixel_index = (y * resolution + x) as usize;
                    face[pixel_index] = sample;
                    sum[0] += f64::from(sample[0]);
                    sum[1] += f64::from(sample[1]);
                    sum[2] += f64::from(sample[2]);
                }
            }
            let inv = (face_pixel_count as f64).recip();
            face_radiance[face_index] = [
                (sum[0] * inv) as f32,
                (sum[1] * inv) as f32,
                (sum[2] * inv) as f32,
            ];
        }
        Some(Self {
            face_radiance,
            resolution,
            face_pixels: Some(faces),
        })
    }

    pub fn face_radiance(&self) -> &[[f32; 3]; 6] {
        &self.face_radiance
    }

    pub fn resolution(&self) -> u32 {
        self.resolution
    }

    /// Builds six RGBA32F face buffers (resolution × resolution × 4 channels).
    /// When the cubemap carries real per-pixel radiance (set by
    /// `from_equirectangular`), expand those into the RGBA32F shape directly.
    /// Otherwise, fall back to spherically-interpolating the six face-centre
    /// radiances: each pixel direction's radiance is a
    /// `max(0, dot(d, face_normal))`-weighted average of the face values.
    /// Alpha is always 1.0.
    pub fn build_face_pixels_rgba32f(&self) -> [Vec<f32>; 6] {
        let resolution = self.resolution.max(1);
        let pixel_count = (resolution as usize).pow(2);
        let mut faces: [Vec<f32>; 6] =
            std::array::from_fn(|_| vec![0.0_f32; pixel_count.saturating_mul(4)]);
        for (face_index, face_pixels) in faces.iter_mut().enumerate() {
            if let Some(stored) = self.face_pixels.as_ref() {
                let source = &stored[face_index];
                for (pixel_index, radiance) in source.iter().enumerate().take(pixel_count) {
                    let offset = pixel_index * 4;
                    face_pixels[offset] = radiance[0];
                    face_pixels[offset + 1] = radiance[1];
                    face_pixels[offset + 2] = radiance[2];
                    face_pixels[offset + 3] = 1.0;
                }
                continue;
            }
            for y in 0..resolution {
                for x in 0..resolution {
                    let u = (x as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let v = (y as f32 + 0.5) / resolution as f32 * 2.0 - 1.0;
                    let direction = cube_face_direction(face_index, u, v);
                    let radiance = blend_face_radiance(&self.face_radiance, direction);
                    let pixel_index = ((y * resolution + x) * 4) as usize;
                    face_pixels[pixel_index] = radiance[0];
                    face_pixels[pixel_index + 1] = radiance[1];
                    face_pixels[pixel_index + 2] = radiance[2];
                    face_pixels[pixel_index + 3] = 1.0;
                }
            }
        }
        faces
    }

    /// Lambertian diffuse irradiance computed by averaging the six per-face
    /// radiances with cosine-weighted hemispherical visibility. Used as a
    /// fallback for backends that do not yet sample the cubemap (WebGL2,
    /// CPU rasterizer).
    pub fn lambertian_irradiance(&self) -> [f32; 3] {
        let mut sum = [0.0_f32; 3];
        for radiance in &self.face_radiance {
            sum[0] += radiance[0];
            sum[1] += radiance[1];
            sum[2] += radiance[2];
        }
        let inv = (self.face_radiance.len() as f32).recip();
        [sum[0] * inv, sum[1] * inv, sum[2] * inv]
    }
}

fn parse_radiance_triplet(value: &str) -> Option<[f32; 3]> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    if parts.len() != 3 {
        return None;
    }
    let mut channels = [0.0_f32; 3];
    for (slot, raw) in channels.iter_mut().zip(parts) {
        let parsed: f32 = raw.parse().ok()?;
        if !parsed.is_finite() || parsed < 0.0 {
            return None;
        }
        *slot = parsed;
    }
    Some(channels)
}

/// Maps the (face, u, v) coordinate to a unit direction vector pointing from
/// the cube center through the face pixel. Mirrors WebGPU's cubemap face
/// orientation (px, nx, py, ny, pz, nz) so the cube uploaded with this
/// mapping samples correctly with `textureSampleLevel(cube, sampler, dir)`.
pub(super) fn cube_face_direction(face_index: usize, u: f32, v: f32) -> Vec3 {
    let raw = match face_index {
        0 => Vec3::new(1.0, -v, -u),
        1 => Vec3::new(-1.0, -v, u),
        2 => Vec3::new(u, 1.0, v),
        3 => Vec3::new(u, -1.0, -v),
        4 => Vec3::new(u, -v, 1.0),
        _ => Vec3::new(-u, -v, -1.0),
    };
    let length = (raw.x * raw.x + raw.y * raw.y + raw.z * raw.z).sqrt();
    if length <= f32::EPSILON || !length.is_finite() {
        Vec3::new(0.0, 0.0, 1.0)
    } else {
        let inv = length.recip();
        Vec3::new(raw.x * inv, raw.y * inv, raw.z * inv)
    }
}

/// Direction-weighted blend of six per-face radiance triplets. At every
/// direction the contribution from face `i` is `max(0, dot(direction, n_i))`,
/// so radiance is C0 continuous across face boundaries (where the dominant
/// face pair contribute equally) and reduces to face-center radiance at the
/// face center (where dot is 1 for that face and ≤0 for the others).
fn blend_face_radiance(face_radiance: &[[f32; 3]; 6], direction: Vec3) -> [f32; 3] {
    let mut accumulated = [0.0_f32; 3];
    let mut weight_sum = 0.0_f32;
    for (face, normal) in ENVIRONMENT_CUBEMAP_FACE_NORMALS.iter().enumerate() {
        let dot = direction.x * normal[0] + direction.y * normal[1] + direction.z * normal[2];
        if dot <= 0.0 {
            continue;
        }
        accumulated[0] += face_radiance[face][0] * dot;
        accumulated[1] += face_radiance[face][1] * dot;
        accumulated[2] += face_radiance[face][2] * dot;
        weight_sum += dot;
    }
    if weight_sum <= f32::EPSILON {
        return [0.0; 3];
    }
    let inv = weight_sum.recip();
    [
        accumulated[0] * inv,
        accumulated[1] * inv,
        accumulated[2] * inv,
    ]
}
