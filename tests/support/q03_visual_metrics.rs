#![allow(dead_code)]

use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub min_x: u32,
    pub min_y: u32,
    pub max_x: u32,
    pub max_y: u32,
}

impl PixelRect {
    pub const fn width(self) -> u32 {
        self.max_x - self.min_x + 1
    }

    pub const fn height(self) -> u32 {
        self.max_y - self.min_y + 1
    }

    pub const fn intersects(self, other: Self) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameMetrics {
    pub pixel_count: usize,
    pub component_count: usize,
    pub rect: Option<PixelRect>,
    pub centroid_x: f32,
    pub centroid_y: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DifferenceMetrics {
    pub changed_pixels: usize,
    pub rect: Option<PixelRect>,
    pub centroid_x: f32,
    pub centroid_y: f32,
}

pub fn foreground_metrics(rgba: &[u8], width: u32, height: u32) -> FrameMetrics {
    masked_metrics(rgba, width, height, |pixel| {
        pixel[0] != 0 || pixel[1] != 0 || pixel[2] != 0
    })
}

pub fn masked_metrics(
    rgba: &[u8],
    width: u32,
    height: u32,
    mut include: impl FnMut(&[u8]) -> bool,
) -> FrameMetrics {
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    let mut mask = vec![false; width as usize * height as usize];
    for (index, pixel) in rgba.chunks_exact(4).enumerate() {
        mask[index] = include(pixel);
    }
    metrics_from_mask(&mask, width, height)
}

pub fn difference_metrics(
    before: &[u8],
    after: &[u8],
    width: u32,
    height: u32,
    channel_threshold: u8,
) -> DifferenceMetrics {
    assert_eq!(before.len(), after.len());
    assert_eq!(before.len(), width as usize * height as usize * 4);
    let mut count = 0usize;
    let mut sum_x = 0u64;
    let mut sum_y = 0u64;
    let mut rect = None;
    for (index, (left, right)) in before
        .chunks_exact(4)
        .zip(after.chunks_exact(4))
        .enumerate()
    {
        let changed =
            (0..4).any(|channel| left[channel].abs_diff(right[channel]) > channel_threshold);
        if !changed {
            continue;
        }
        let x = index as u32 % width;
        let y = index as u32 / width;
        count += 1;
        sum_x += u64::from(x);
        sum_y += u64::from(y);
        extend_rect(&mut rect, x, y);
    }
    DifferenceMetrics {
        changed_pixels: count,
        rect,
        centroid_x: sum_x as f32 / count.max(1) as f32,
        centroid_y: sum_y as f32 / count.max(1) as f32,
    }
}

pub fn clear_rect(rgba: &mut [u8], width: u32, height: u32, rect: PixelRect) {
    assert_eq!(rgba.len(), width as usize * height as usize * 4);
    for y in rect.min_y..=rect.max_y.min(height.saturating_sub(1)) {
        for x in rect.min_x..=rect.max_x.min(width.saturating_sub(1)) {
            let offset = (y as usize * width as usize + x as usize) * 4;
            rgba[offset..offset + 4].copy_from_slice(&[0, 0, 0, 255]);
        }
    }
}

fn metrics_from_mask(mask: &[bool], width: u32, height: u32) -> FrameMetrics {
    let mut count = 0usize;
    let mut sum_x = 0u64;
    let mut sum_y = 0u64;
    let mut rect = None;
    for (index, included) in mask.iter().copied().enumerate() {
        if !included {
            continue;
        }
        let x = index as u32 % width;
        let y = index as u32 / width;
        count += 1;
        sum_x += u64::from(x);
        sum_y += u64::from(y);
        extend_rect(&mut rect, x, y);
    }

    let mut visited = vec![false; mask.len()];
    let mut component_count = 0usize;
    for start in 0..mask.len() {
        if !mask[start] || visited[start] {
            continue;
        }
        let mut queue = VecDeque::from([start]);
        visited[start] = true;
        let mut component_pixels = 0usize;
        while let Some(index) = queue.pop_front() {
            component_pixels += 1;
            let x = index % width as usize;
            let y = index / width as usize;
            for (nx, ny) in [
                (x.wrapping_sub(1), y),
                (x + 1, y),
                (x, y.wrapping_sub(1)),
                (x, y + 1),
            ] {
                if nx >= width as usize || ny >= height as usize {
                    continue;
                }
                let neighbor = ny * width as usize + nx;
                if mask[neighbor] && !visited[neighbor] {
                    visited[neighbor] = true;
                    queue.push_back(neighbor);
                }
            }
        }
        if component_pixels >= 2 {
            component_count += 1;
        }
    }

    FrameMetrics {
        pixel_count: count,
        component_count,
        rect,
        centroid_x: sum_x as f32 / count.max(1) as f32,
        centroid_y: sum_y as f32 / count.max(1) as f32,
    }
}

fn extend_rect(rect: &mut Option<PixelRect>, x: u32, y: u32) {
    *rect = Some(match *rect {
        Some(current) => PixelRect {
            min_x: current.min_x.min(x),
            min_y: current.min_y.min(y),
            max_x: current.max_x.max(x),
            max_y: current.max_y.max(y),
        },
        None => PixelRect {
            min_x: x,
            min_y: y,
            max_x: x,
            max_y: y,
        },
    });
}
