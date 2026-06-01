use serde::{Deserialize, Serialize};

use crate::render::PixelReadback;

use super::CaptureError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePixelSummary {
    pub nonblack: u64,
    pub bbox: Option<CapturePixelBounds>,
    pub center: [u8; 4],
    pub fnv1a64: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturePixelBounds {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
    pub width: u32,
    pub height: u32,
}

pub fn summarize_pixel_readback(
    readback: &PixelReadback,
) -> Result<CapturePixelSummary, CaptureError> {
    summarize_rgba8(readback.width(), readback.height(), readback.rgba8())
}

pub fn summarize_rgba8(
    width: u32,
    height: u32,
    rgba8: &[u8],
) -> Result<CapturePixelSummary, CaptureError> {
    validate_rgba8_len(width, height, rgba8.len())?;
    let mut nonblack = 0_u64;
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = 0_u32;
    let mut max_y = 0_u32;

    for y in 0..height {
        for x in 0..width {
            let offset = ((y as usize) * (width as usize) + (x as usize)) * 4;
            let pixel = &rgba8[offset..offset + 4];
            if pixel[0] > 0 || pixel[1] > 0 || pixel[2] > 0 {
                nonblack = nonblack.saturating_add(1);
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    let bbox = (nonblack > 0).then_some(CapturePixelBounds {
        min_x,
        min_y,
        max_x,
        max_y,
        width: max_x.saturating_sub(min_x).saturating_add(1),
        height: max_y.saturating_sub(min_y).saturating_add(1),
    });

    Ok(CapturePixelSummary {
        nonblack,
        bbox,
        center: sample_rgba8(
            rgba8,
            width,
            height,
            width as f32 * 0.5,
            height as f32 * 0.5,
        ),
        fnv1a64: fnv1a64_hex(rgba8),
    })
}

pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = FNV_OFFSET;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

pub fn sample_rgba8(rgba8: &[u8], width: u32, height: u32, x: f32, y: f32) -> [u8; 4] {
    if width == 0 || height == 0 {
        return [0; 4];
    }
    let x = x.floor().clamp(0.0, width.saturating_sub(1) as f32) as u32;
    let y = y.floor().clamp(0.0, height.saturating_sub(1) as f32) as u32;
    let offset = ((y as usize) * (width as usize) + (x as usize)) * 4;
    if let Some(pixel) = rgba8.get(offset..offset + 4) {
        [pixel[0], pixel[1], pixel[2], pixel[3]]
    } else {
        [0; 4]
    }
}

pub(super) fn validate_rgba8_len(
    width: u32,
    height: u32,
    actual_len: usize,
) -> Result<(), CaptureError> {
    let expected_len = width as usize * height as usize * 4;
    if actual_len == expected_len {
        Ok(())
    } else {
        Err(CaptureError::InvalidPixelBuffer {
            width,
            height,
            expected_len,
            actual_len,
        })
    }
}
