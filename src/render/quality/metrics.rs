use super::types::{
    RenderQualityFrameMetrics, RenderQualityLabelMetrics, RenderQualityLineMetrics,
    RenderQualityRegion, round3,
};

pub fn frame_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityFrameMetrics {
    let luminance = collect_luminance(rgba8, width, height, region);
    if luminance.is_empty() {
        return RenderQualityFrameMetrics {
            low_clip_fraction: 1.0,
            high_clip_fraction: 0.0,
            p01: 0.0,
            p05: 0.0,
            p50: 0.0,
            p95: 0.0,
            p99: 0.0,
            luminance_range: 0.0,
            luminance_stddev: 0.0,
            sobel_energy: 0.0,
            noise_outlier_fraction: 0.0,
        };
    }
    let mut sorted = luminance.clone();
    sorted.sort_by(f32::total_cmp);
    let mean = sorted.iter().sum::<f32>() / sorted.len() as f32;
    let variance = sorted
        .iter()
        .map(|value| {
            let delta = *value - mean;
            delta * delta
        })
        .sum::<f32>()
        / sorted.len() as f32;
    let p05 = percentile_sorted(&sorted, 0.05);
    let p95 = percentile_sorted(&sorted, 0.95);
    RenderQualityFrameMetrics {
        low_clip_fraction: round3(
            sorted.iter().filter(|value| **value <= 0.02).count() as f32 / sorted.len() as f32,
        ),
        high_clip_fraction: round3(
            sorted.iter().filter(|value| **value >= 0.98).count() as f32 / sorted.len() as f32,
        ),
        p01: round3(percentile_sorted(&sorted, 0.01)),
        p05: round3(p05),
        p50: round3(percentile_sorted(&sorted, 0.50)),
        p95: round3(p95),
        p99: round3(percentile_sorted(&sorted, 0.99)),
        luminance_range: round3((p95 - p05).max(0.0)),
        luminance_stddev: round3(variance.sqrt()),
        sobel_energy: round3(sobel_energy(rgba8, width, height, region)),
        noise_outlier_fraction: round3(noise_outlier_fraction(rgba8, width, height, region)),
    }
}

pub fn label_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityLabelMetrics {
    let mut ink_pixels = 0usize;
    let mut isolated = 0usize;
    let mut intermediate = 0usize;
    let area = region_area(region);
    for y in region.y..region.y.saturating_add(region.height).min(height) {
        for x in region.x..region.x.saturating_add(region.width).min(width) {
            let lum = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            if lum > 0.05 {
                ink_pixels = ink_pixels.saturating_add(1);
                let neighbors = bright_neighbor_count(rgba8, width, height, x, y);
                if neighbors <= 1 {
                    isolated = isolated.saturating_add(1);
                }
            }
            if (0.05..0.95).contains(&lum) {
                intermediate = intermediate.saturating_add(1);
            }
        }
    }
    let area = area.max(1) as f32;
    RenderQualityLabelMetrics {
        ink_coverage: round3(ink_pixels as f32 / area),
        ink_isolation: round3(if ink_pixels == 0 {
            1.0
        } else {
            isolated as f32 / ink_pixels as f32
        }),
        intermediate_edge_fraction: round3(intermediate as f32 / area),
    }
}

pub fn line_metrics(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> RenderQualityLineMetrics {
    let mut samples = Vec::new();
    let mut luminance = Vec::new();
    for y in region.y..region.y.saturating_add(region.height).min(height) {
        for x in region.x..region.x.saturating_add(region.width).min(width) {
            let lum = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            samples.push((x as f32 + 0.5, y as f32 + 0.5, lum));
            luminance.push(lum);
        }
    }
    if luminance.is_empty() {
        return RenderQualityLineMetrics {
            ink_coverage: 0.0,
            intermediate_edge_fraction: 0.0,
            straightness_error: 1.0,
        };
    }
    luminance.sort_by(f32::total_cmp);
    let low = percentile_sorted(&luminance, 0.05);
    let high = percentile_sorted(&luminance, 0.95);
    let low_count = luminance
        .iter()
        .filter(|value| (**value - low).abs() <= 0.02)
        .count();
    let high_count = luminance
        .iter()
        .filter(|value| (**value - high).abs() <= 0.02)
        .count();
    let (background, foreground) = if low_count >= high_count {
        (low, high)
    } else {
        (high, low)
    };
    let contrast = (foreground - background).abs();
    if contrast <= 0.02 {
        return RenderQualityLineMetrics {
            ink_coverage: 0.0,
            intermediate_edge_fraction: 0.0,
            straightness_error: 1.0,
        };
    }

    let mut ink_pixels = 0usize;
    let mut intermediate = 0usize;
    let mut sum_x = 0.0f32;
    let mut sum_y = 0.0f32;
    let mut points = Vec::new();
    for (px, py, lum) in samples {
        let coverage = ((lum - background).abs() / contrast).clamp(0.0, 1.0);
        if coverage > 0.05 {
            ink_pixels = ink_pixels.saturating_add(1);
            sum_x += px;
            sum_y += py;
            points.push((px, py));
        }
        if (0.05..0.95).contains(&coverage) {
            intermediate = intermediate.saturating_add(1);
        }
    }
    let area = region_area(region).max(1) as f32;
    let intermediate_edge_fraction = if ink_pixels == 0 {
        0.0
    } else {
        intermediate as f32 / ink_pixels as f32
    };
    RenderQualityLineMetrics {
        ink_coverage: round3(ink_pixels as f32 / area),
        intermediate_edge_fraction: round3(intermediate_edge_fraction),
        straightness_error: round3(line_straightness_error(&points, sum_x, sum_y, region)),
    }
}

fn collect_luminance(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> Vec<f32> {
    let mut values = Vec::with_capacity(region_area(region));
    for y in region.y..region.y.saturating_add(region.height).min(height) {
        for x in region.x..region.x.saturating_add(region.width).min(width) {
            if let Some(value) = pixel_luminance(rgba8, width, x, y) {
                values.push(value);
            }
        }
    }
    values
}

fn sobel_energy(rgba8: &[u8], width: u32, height: u32, region: RenderQualityRegion) -> f32 {
    if width < 3 || height < 3 {
        return 0.0;
    }
    let x0 = region.x.max(1);
    let y0 = region.y.max(1);
    let x1 = region
        .x
        .saturating_add(region.width)
        .min(width.saturating_sub(1));
    let y1 = region
        .y
        .saturating_add(region.height)
        .min(height.saturating_sub(1));
    let mut total = 0.0;
    let mut count = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let sample = |sx, sy| pixel_luminance(rgba8, width, sx, sy).unwrap_or(0.0);
            let gx = -sample(x - 1, y - 1) + sample(x + 1, y - 1) - 2.0 * sample(x - 1, y)
                + 2.0 * sample(x + 1, y)
                - sample(x - 1, y + 1)
                + sample(x + 1, y + 1);
            let gy = -sample(x - 1, y - 1) - 2.0 * sample(x, y - 1) - sample(x + 1, y - 1)
                + sample(x - 1, y + 1)
                + 2.0 * sample(x, y + 1)
                + sample(x + 1, y + 1);
            total += (gx * gx + gy * gy).sqrt();
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn noise_outlier_fraction(
    rgba8: &[u8],
    width: u32,
    height: u32,
    region: RenderQualityRegion,
) -> f32 {
    if width < 3 || height < 3 {
        return 0.0;
    }
    let x0 = region.x.max(1);
    let y0 = region.y.max(1);
    let x1 = region
        .x
        .saturating_add(region.width)
        .min(width.saturating_sub(1));
    let y1 = region
        .y
        .saturating_add(region.height)
        .min(height.saturating_sub(1));
    let mut outliers = 0usize;
    let mut count = 0usize;
    for y in y0..y1 {
        for x in x0..x1 {
            let center = pixel_luminance(rgba8, width, x, y).unwrap_or(0.0);
            let mut neighbors = [
                pixel_luminance(rgba8, width, x - 1, y - 1).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x, y - 1).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x + 1, y - 1).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x - 1, y).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x + 1, y).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x - 1, y + 1).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x, y + 1).unwrap_or(0.0),
                pixel_luminance(rgba8, width, x + 1, y + 1).unwrap_or(0.0),
            ];
            neighbors.sort_by(f32::total_cmp);
            let median = (neighbors[3] + neighbors[4]) * 0.5;
            if (center - median).abs() > 0.35 {
                outliers = outliers.saturating_add(1);
            }
            count = count.saturating_add(1);
        }
    }
    if count == 0 {
        0.0
    } else {
        outliers as f32 / count as f32
    }
}

fn bright_neighbor_count(rgba8: &[u8], width: u32, height: u32, x: u32, y: u32) -> usize {
    let mut count = 0usize;
    let y_min = y.saturating_sub(1);
    let y_max = y.saturating_add(1).min(height.saturating_sub(1));
    let x_min = x.saturating_sub(1);
    let x_max = x.saturating_add(1).min(width.saturating_sub(1));
    for ny in y_min..=y_max {
        for nx in x_min..=x_max {
            if nx == x && ny == y {
                continue;
            }
            if pixel_luminance(rgba8, width, nx, ny).unwrap_or(0.0) > 0.05 {
                count = count.saturating_add(1);
            }
        }
    }
    count
}

fn line_straightness_error(
    points: &[(f32, f32)],
    sum_x: f32,
    sum_y: f32,
    region: RenderQualityRegion,
) -> f32 {
    if points.len() < 2 {
        return 1.0;
    }
    let count = points.len() as f32;
    let mean_x = sum_x / count;
    let mean_y = sum_y / count;
    let mut var_x = 0.0f32;
    let mut var_y = 0.0f32;
    let mut cov_xy = 0.0f32;
    for (x, y) in points {
        let dx = *x - mean_x;
        let dy = *y - mean_y;
        var_x += dx * dx;
        var_y += dy * dy;
        cov_xy += dx * dy;
    }
    var_x /= count;
    var_y /= count;
    cov_xy /= count;
    let trace = var_x + var_y;
    let determinant = var_x * var_y - cov_xy * cov_xy;
    let discriminant = (trace * trace - 4.0 * determinant).max(0.0).sqrt();
    let minor = ((trace - discriminant) * 0.5).max(0.0);
    let diagonal = ((region.width as f32).hypot(region.height as f32)).max(1.0);
    minor.sqrt() / diagonal
}

fn pixel_luminance(rgba8: &[u8], width: u32, x: u32, y: u32) -> Option<f32> {
    let offset = ((y.checked_mul(width)?).checked_add(x)?).checked_mul(4)? as usize;
    let pixel = rgba8.get(offset..offset + 4)?;
    Some(luminance_from_srgb8(pixel[0], pixel[1], pixel[2]))
}

pub(super) fn luminance_from_srgb8(r: u8, g: u8, b: u8) -> f32 {
    let srgb_to_linear = |value: u8| {
        let value = f32::from(value) / 255.0;
        if value <= 0.04045 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * srgb_to_linear(r) + 0.7152 * srgb_to_linear(g) + 0.0722 * srgb_to_linear(b)
}

fn percentile_sorted(values: &[f32], percentile: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let index = ((values.len() - 1) as f32 * percentile.clamp(0.0, 1.0)).round() as usize;
    values[index.min(values.len() - 1)]
}

fn region_area(region: RenderQualityRegion) -> usize {
    (region.width as usize).saturating_mul(region.height as usize)
}
