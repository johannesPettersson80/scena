use crate::material::Color;

use super::RasterTarget;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct MaterialReflectionPixel {
    sample_x: f32,
    sample_y: f32,
    weight: f32,
    roughness: f32,
}

impl MaterialReflectionPixel {
    pub(super) fn new(sample_x: f32, sample_y: f32, weight: f32, roughness: f32) -> Option<Self> {
        if !sample_x.is_finite()
            || !sample_y.is_finite()
            || !weight.is_finite()
            || !roughness.is_finite()
        {
            return None;
        }
        let weight = weight.clamp(0.0, 0.88);
        (weight > 0.0).then_some(Self {
            sample_x,
            sample_y,
            weight,
            roughness: roughness.clamp(0.0, 1.0),
        })
    }

    const fn is_active(self) -> bool {
        self.weight > 0.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScreenSpaceReflectionConfig {
    strength: f32,
    roughness: f32,
    horizon_fraction: f32,
    fade: f32,
}

impl ScreenSpaceReflectionConfig {
    pub const fn studio_floor() -> Self {
        Self {
            strength: 0.72,
            roughness: 0.18,
            horizon_fraction: 0.58,
            fade: 0.35,
        }
    }

    pub fn new(strength: f32, roughness: f32, horizon_fraction: f32, fade: f32) -> Self {
        Self {
            strength: clamp_unit_or(strength, 0.72),
            roughness: clamp_unit_or(roughness, 0.18),
            horizon_fraction: clamp_unit_or(horizon_fraction, 0.58),
            fade: clamp_unit_or(fade, 0.35),
        }
    }

    pub const fn strength(self) -> f32 {
        self.strength
    }

    pub const fn roughness(self) -> f32 {
        self.roughness
    }

    pub const fn horizon_fraction(self) -> f32 {
        self.horizon_fraction
    }

    pub const fn fade(self) -> f32 {
        self.fade
    }

    pub fn roughness_radius_px(self) -> u32 {
        (self.roughness * 8.0).round().clamp(0.0, 8.0) as u32
    }
}

impl Default for ScreenSpaceReflectionConfig {
    fn default() -> Self {
        Self::studio_floor()
    }
}

fn clamp_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        fallback
    }
}

#[allow(dead_code)]
pub(super) fn apply_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
    config: ScreenSpaceReflectionConfig,
) -> u64 {
    if target.width < 3 || target.height < 3 || config.strength() <= 0.0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(scratch.len(), target.byte_len());
    scratch.copy_from_slice(frame);

    let horizon_y = ((target.height as f32) * config.horizon_fraction())
        .round()
        .clamp(1.0, (target.height.saturating_sub(2)) as f32) as u32;
    let floor_height = target.height.saturating_sub(horizon_y).max(1);
    let radius = config.roughness_radius_px();
    for y in horizon_y..target.height {
        let distance = (y.saturating_sub(horizon_y) as f32 / floor_height as f32).clamp(0.0, 1.0);
        let fade = (1.0 - distance * config.fade()).clamp(0.0, 1.0);
        let mirrored_y = horizon_y
            .saturating_sub(y.saturating_sub(horizon_y))
            .min(target.height - 1);
        for x in 0..target.width {
            let offset = pixel_offset(target, x, y);
            let source_luma = luma_from_srgb8(&scratch[offset..offset + 4]) / 255.0;
            let floor_mask = (1.0 - source_luma).clamp(0.0, 1.0);
            let alpha = (config.strength() * fade * floor_mask).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }
            let reflected = blurred_reflection_sample(target, scratch, x, mirrored_y, radius);
            for channel in 0..3 {
                let base = f32::from(scratch[offset + channel]);
                let value = base * (1.0 - alpha) + reflected[channel] * alpha;
                frame[offset + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
            frame[offset + 3] = scratch[offset + 3];
        }
    }

    1
}

pub(super) fn apply_linear(
    target: RasterTarget,
    frame: &mut [Color],
    scratch: &mut [Color],
    config: ScreenSpaceReflectionConfig,
) -> u64 {
    if target.width < 3 || target.height < 3 || config.strength() <= 0.0 {
        return 0;
    }
    scratch.copy_from_slice(frame);
    let horizon_y = ((target.height as f32) * config.horizon_fraction())
        .round()
        .clamp(1.0, target.height.saturating_sub(2) as f32) as u32;
    let floor_height = target.height.saturating_sub(horizon_y).max(1);
    let radius = config.roughness_radius_px();
    for y in horizon_y..target.height {
        let distance = y.saturating_sub(horizon_y) as f32 / floor_height as f32;
        let fade = (1.0 - distance * config.fade()).clamp(0.0, 1.0);
        let mirrored_y = horizon_y
            .saturating_sub(y.saturating_sub(horizon_y))
            .min(target.height - 1);
        for x in 0..target.width {
            let index = target.pixel_index(x, y);
            let base = scratch[index];
            let luma = base.r * 0.2126 + base.g * 0.7152 + base.b * 0.0722;
            let alpha = (config.strength() * fade * (1.0 - luma).clamp(0.0, 1.0)).clamp(0.0, 1.0);
            if alpha <= 0.0 {
                continue;
            }
            let reflected = blurred_reflection_sample_linear(
                target,
                scratch,
                x as f32,
                mirrored_y as f32,
                radius,
            );
            frame[index] = mix_linear(base, reflected, alpha);
        }
    }
    1
}

#[allow(dead_code)]
pub(super) fn apply_material_rgba8(
    target: RasterTarget,
    frame: &mut [u8],
    scratch: &mut [u8],
    material_reflections: &[MaterialReflectionPixel],
    config: ScreenSpaceReflectionConfig,
) -> u64 {
    if target.width < 3 || target.height < 3 || config.strength() <= 0.0 {
        return 0;
    }
    debug_assert_eq!(frame.len(), target.byte_len());
    debug_assert_eq!(scratch.len(), target.byte_len());
    debug_assert_eq!(material_reflections.len(), target.pixel_len());
    scratch.copy_from_slice(frame);

    let mut touched = false;
    for y in 0..target.height {
        for x in 0..target.width {
            let pixel_index = target.pixel_index(x, y);
            let reflection = material_reflections[pixel_index];
            if !reflection.is_active() {
                continue;
            }
            touched = true;
            let offset = pixel_index * 4;
            let radius = ((reflection.roughness * reflection.roughness * 14.0
                + config.roughness() * 5.0)
                .round()
                .clamp(0.0, 8.0)) as u32;
            let reflected = blurred_reflection_sample_f32(
                target,
                scratch,
                reflection.sample_x,
                reflection.sample_y,
                radius,
            );
            for channel in 0..3 {
                let base = f32::from(scratch[offset + channel]);
                let value =
                    base * (1.0 - reflection.weight) + reflected[channel] * reflection.weight;
                frame[offset + channel] = value.round().clamp(0.0, 255.0) as u8;
            }
            frame[offset + 3] = scratch[offset + 3];
        }
    }

    u64::from(touched)
}

pub(super) fn apply_material_linear(
    target: RasterTarget,
    frame: &mut [Color],
    scratch: &mut [Color],
    material_reflections: &[MaterialReflectionPixel],
    config: ScreenSpaceReflectionConfig,
) -> u64 {
    if target.width < 3 || target.height < 3 || config.strength() <= 0.0 {
        return 0;
    }
    scratch.copy_from_slice(frame);
    let mut touched = false;
    for (index, reflection) in material_reflections.iter().copied().enumerate() {
        if !reflection.is_active() {
            continue;
        }
        touched = true;
        let radius = ((reflection.roughness * reflection.roughness * 14.0
            + config.roughness() * 5.0)
            .round()
            .clamp(0.0, 8.0)) as u32;
        let reflected = blurred_reflection_sample_linear(
            target,
            scratch,
            reflection.sample_x,
            reflection.sample_y,
            radius,
        );
        frame[index] = mix_linear(scratch[index], reflected, reflection.weight);
    }
    u64::from(touched)
}

fn mix_linear(base: Color, reflected: Color, weight: f32) -> Color {
    Color::from_linear_rgba(
        base.r * (1.0 - weight) + reflected.r * weight,
        base.g * (1.0 - weight) + reflected.g * weight,
        base.b * (1.0 - weight) + reflected.b * weight,
        base.a,
    )
}

fn blurred_reflection_sample_linear(
    target: RasterTarget,
    frame: &[Color],
    x: f32,
    y: f32,
    radius: u32,
) -> Color {
    let cx = x.clamp(0.0, target.width.saturating_sub(1) as f32);
    let cy = y.clamp(0.0, target.height.saturating_sub(1) as f32);
    if radius == 0 {
        return bilinear_sample_linear(target, frame, cx, cy);
    }
    let mut sum = [0.0_f32; 4];
    let mut weight_sum = 0.0;
    for sy in (cy.floor() as u32).saturating_sub(radius)
        ..=(cy.ceil() as u32)
            .saturating_add(radius)
            .min(target.height - 1)
    {
        for sx in (cx.floor() as u32).saturating_sub(radius)
            ..=(cx.ceil() as u32)
                .saturating_add(radius)
                .min(target.width - 1)
        {
            let dx = sx as f32 - cx;
            let dy = sy as f32 - cy;
            let weight = (1.0 - (dx * dx + dy * dy).sqrt() / (radius as f32 + 1.0)).max(0.0);
            let sample = frame[target.pixel_index(sx, sy)];
            sum[0] += sample.r * weight;
            sum[1] += sample.g * weight;
            sum[2] += sample.b * weight;
            sum[3] += sample.a * weight;
            weight_sum += weight;
        }
    }
    if weight_sum <= 0.0 {
        return bilinear_sample_linear(target, frame, cx, cy);
    }
    Color::from_linear_rgba(
        sum[0] / weight_sum,
        sum[1] / weight_sum,
        sum[2] / weight_sum,
        sum[3] / weight_sum,
    )
}

fn bilinear_sample_linear(target: RasterTarget, frame: &[Color], x: f32, y: f32) -> Color {
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = x0.saturating_add(1).min(target.width - 1);
    let y1 = y0.saturating_add(1).min(target.height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let c00 = frame[target.pixel_index(x0, y0)];
    let c10 = frame[target.pixel_index(x1, y0)];
    let c01 = frame[target.pixel_index(x0, y1)];
    let c11 = frame[target.pixel_index(x1, y1)];
    let channel = |a: f32, b: f32, c: f32, d: f32| {
        (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty
    };
    Color::from_linear_rgba(
        channel(c00.r, c10.r, c01.r, c11.r),
        channel(c00.g, c10.g, c01.g, c11.g),
        channel(c00.b, c10.b, c01.b, c11.b),
        channel(c00.a, c10.a, c01.a, c11.a),
    )
}

#[allow(dead_code)]
fn blurred_reflection_sample(
    target: RasterTarget,
    frame: &[u8],
    x: u32,
    y: u32,
    radius: u32,
) -> [f32; 3] {
    if radius == 0 {
        let offset = pixel_offset(target, x, y);
        return [
            f32::from(frame[offset]),
            f32::from(frame[offset + 1]),
            f32::from(frame[offset + 2]),
        ];
    }
    let min_x = x.saturating_sub(radius);
    let max_x = x.saturating_add(radius).min(target.width - 1);
    let min_y = y.saturating_sub(radius);
    let max_y = y.saturating_add(radius).min(target.height - 1);
    let mut sum = [0.0_f32; 3];
    let mut weight_sum = 0.0_f32;
    for sample_y in min_y..=max_y {
        for sample_x in min_x..=max_x {
            let dx = sample_x.abs_diff(x) as f32;
            let dy = sample_y.abs_diff(y) as f32;
            let distance = (dx * dx + dy * dy).sqrt();
            let weight = (1.0 - distance / (radius as f32 + 1.0)).max(0.0);
            if weight <= 0.0 {
                continue;
            }
            let offset = pixel_offset(target, sample_x, sample_y);
            sum[0] += f32::from(frame[offset]) * weight;
            sum[1] += f32::from(frame[offset + 1]) * weight;
            sum[2] += f32::from(frame[offset + 2]) * weight;
            weight_sum += weight;
        }
    }
    if weight_sum <= 0.0 {
        return blurred_reflection_sample(target, frame, x, y, 0);
    }
    [
        sum[0] / weight_sum,
        sum[1] / weight_sum,
        sum[2] / weight_sum,
    ]
}

#[allow(dead_code)]
fn blurred_reflection_sample_f32(
    target: RasterTarget,
    frame: &[u8],
    x: f32,
    y: f32,
    radius: u32,
) -> [f32; 3] {
    if radius == 0 {
        return bilinear_sample_srgb8(target, frame, x, y);
    }
    let center_x = x.clamp(0.0, target.width.saturating_sub(1) as f32);
    let center_y = y.clamp(0.0, target.height.saturating_sub(1) as f32);
    let min_x = (center_x.floor() as u32).saturating_sub(radius);
    let max_x = (center_x.ceil() as u32)
        .saturating_add(radius)
        .min(target.width - 1);
    let min_y = (center_y.floor() as u32).saturating_sub(radius);
    let max_y = (center_y.ceil() as u32)
        .saturating_add(radius)
        .min(target.height - 1);
    let mut sum = [0.0_f32; 3];
    let mut weight_sum = 0.0_f32;
    for sample_y in min_y..=max_y {
        for sample_x in min_x..=max_x {
            let dx = sample_x as f32 - center_x;
            let dy = sample_y as f32 - center_y;
            let distance = (dx * dx + dy * dy).sqrt();
            let weight = (1.0 - distance / (radius as f32 + 1.0)).max(0.0);
            if weight <= 0.0 {
                continue;
            }
            let offset = pixel_offset(target, sample_x, sample_y);
            sum[0] += f32::from(frame[offset]) * weight;
            sum[1] += f32::from(frame[offset + 1]) * weight;
            sum[2] += f32::from(frame[offset + 2]) * weight;
            weight_sum += weight;
        }
    }
    if weight_sum <= 0.0 {
        return bilinear_sample_srgb8(target, frame, x, y);
    }
    [
        sum[0] / weight_sum,
        sum[1] / weight_sum,
        sum[2] / weight_sum,
    ]
}

#[allow(dead_code)]
fn bilinear_sample_srgb8(target: RasterTarget, frame: &[u8], x: f32, y: f32) -> [f32; 3] {
    let x = x.clamp(0.0, target.width.saturating_sub(1) as f32);
    let y = y.clamp(0.0, target.height.saturating_sub(1) as f32);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = x0.saturating_add(1).min(target.width - 1);
    let y1 = y0.saturating_add(1).min(target.height - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let c00 = sample_pixel_rgb(target, frame, x0, y0);
    let c10 = sample_pixel_rgb(target, frame, x1, y0);
    let c01 = sample_pixel_rgb(target, frame, x0, y1);
    let c11 = sample_pixel_rgb(target, frame, x1, y1);
    [
        bilinear_channel(c00[0], c10[0], c01[0], c11[0], tx, ty),
        bilinear_channel(c00[1], c10[1], c01[1], c11[1], tx, ty),
        bilinear_channel(c00[2], c10[2], c01[2], c11[2], tx, ty),
    ]
}

#[allow(dead_code)]
fn sample_pixel_rgb(target: RasterTarget, frame: &[u8], x: u32, y: u32) -> [f32; 3] {
    let offset = pixel_offset(target, x, y);
    [
        f32::from(frame[offset]),
        f32::from(frame[offset + 1]),
        f32::from(frame[offset + 2]),
    ]
}

#[allow(dead_code)]
fn bilinear_channel(c00: f32, c10: f32, c01: f32, c11: f32, tx: f32, ty: f32) -> f32 {
    let top = c00 * (1.0 - tx) + c10 * tx;
    let bottom = c01 * (1.0 - tx) + c11 * tx;
    top * (1.0 - ty) + bottom * ty
}

#[allow(dead_code)]
fn pixel_offset(target: RasterTarget, x: u32, y: u32) -> usize {
    target.pixel_index(x, y) * 4
}

#[allow(dead_code)]
fn luma_from_srgb8(pixel: &[u8]) -> f32 {
    f32::from(pixel[0]) * 0.299 + f32::from(pixel[1]) * 0.587 + f32::from(pixel[2]) * 0.114
}
