use super::*;
use std::path::Path;

mod frame_reference;
mod label;
mod line_geometry;
mod reflection_quality;
fn eroded_label_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for x in (2..width.saturating_sub(2)).step_by(3) {
        for y in (3..height.saturating_sub(3)).step_by(4) {
            set_gray(&mut rgba, width, x, y, 255);
        }
    }
    rgba
}

fn antialiased_label_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for y in 4..12 {
        for x in 3..29 {
            let edge = x == 3 || x == 28 || y == 4 || y == 11;
            set_gray(&mut rgba, width, x, y, if edge { 96 } else { 230 });
        }
    }
    rgba
}

fn blocky_text_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for cell_y in 0..7 {
        for cell_x in 0..18 {
            let on = cell_x % 3 != 1 || cell_y == 0 || cell_y == 3 || cell_y == 6;
            if !on {
                continue;
            }
            for y in 0..3 {
                for x in 0..3 {
                    set_gray(
                        &mut rgba,
                        width,
                        8 + cell_x * 3 + x,
                        4 + cell_y * 3 + y,
                        255,
                    );
                }
            }
        }
    }
    rgba
}

fn solid_label_background_fixture(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for _ in 0..width.saturating_mul(height) {
        rgba.extend_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
    }
    rgba
}

fn nonuniform_label_background_fixture(width: u32, height: u32, rgb: [u8; 3]) -> Vec<u8> {
    let mut rgba = solid_label_background_fixture(width, height, rgb);
    for y in 0..height {
        for x in 0..width {
            if x > width / 2 || y > height * 2 / 3 {
                let offset = ((y * width + x) * 4) as usize;
                rgba[offset] = rgba[offset].saturating_add(58);
                rgba[offset + 1] = rgba[offset + 1].saturating_add(58);
                rgba[offset + 2] = rgba[offset + 2].saturating_add(58);
            }
        }
    }
    rgba
}

fn aliased_line_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for x in 6..width.saturating_sub(6) {
        let t = (x - 6) as f32 / width.saturating_sub(12).max(1) as f32;
        let y = (height as f32 * 0.75 + (height as f32 * -0.5) * t).round() as u32;
        set_gray(&mut rgba, width, x, y.min(height.saturating_sub(1)), 255);
    }
    rgba
}

fn antialiased_line_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    let start = (6.0f32, height as f32 * 0.75);
    let end = (width as f32 - 6.0, height as f32 * 0.25);
    let dx = end.0 - start.0;
    let dy = end.1 - start.1;
    let length_squared = dx * dx + dy * dy;
    for y in 0..height {
        for x in 0..width {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let t = (((px - start.0) * dx + (py - start.1) * dy) / length_squared).clamp(0.0, 1.0);
            let closest_x = start.0 + dx * t;
            let closest_y = start.1 + dy * t;
            let distance = ((px - closest_x).powi(2) + (py - closest_y).powi(2)).sqrt();
            let coverage = if distance <= 1.0 {
                1.0
            } else {
                (2.0 - distance).clamp(0.0, 1.0)
            };
            if coverage > 0.0 {
                set_gray(&mut rgba, width, x, y, (coverage * 255.0).round() as u8);
            }
        }
    }
    rgba
}

fn hard_geometry_edge_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for y in 3..height.saturating_sub(3) {
        for x in width / 2..width.saturating_sub(3) {
            set_gray(&mut rgba, width, x, y, 255);
        }
    }
    rgba
}

fn antialiased_geometry_edge_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    let edge = width / 2;
    for y in 3..height.saturating_sub(3) {
        for x in edge.saturating_sub(1)..width.saturating_sub(3) {
            let value = if x == edge.saturating_sub(1) {
                72
            } else if x == edge {
                192
            } else {
                255
            };
            set_gray(&mut rgba, width, x, y, value);
        }
    }
    rgba
}

fn structured_reflection_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for y in 0..height {
        for x in 0..width {
            let band = (x / 6 + y / 5) % 4;
            let color = match band {
                0 => [24, 44, 86],
                1 => [92, 128, 210],
                2 => [150, 86, 36],
                _ => [222, 210, 164],
            };
            set_rgb(&mut rgba, width, x, y, color);
        }
    }
    rgba
}

fn firefly_reflection_fixture(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = structured_reflection_fixture(width, height);
    for y in (3..height.saturating_sub(3)).step_by(7) {
        for x in (5..width.saturating_sub(5)).step_by(11) {
            set_rgb(&mut rgba, width, x, y, [255, 255, 255]);
        }
    }
    rgba
}

fn black_frame(width: u32, height: u32) -> Vec<u8> {
    solid_frame(width, height, [0, 0, 0, 255])
}

fn solid_frame(width: u32, height: u32, color: [u8; 4]) -> Vec<u8> {
    let mut rgba = vec![0; (width * height * 4) as usize];
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.copy_from_slice(&color);
    }
    rgba
}

fn checker_frame(width: u32, height: u32) -> Vec<u8> {
    let mut rgba = black_frame(width, height);
    for y in 0..height {
        for x in 0..width {
            let value = if (x / 4 + y / 4) % 2 == 0 { 32 } else { 224 };
            set_gray(&mut rgba, width, x, y, value);
        }
    }
    rgba
}

fn set_gray(rgba: &mut [u8], width: u32, x: u32, y: u32, value: u8) {
    set_rgb(rgba, width, x, y, [value, value, value]);
}

fn set_rgb(rgba: &mut [u8], width: u32, x: u32, y: u32, value: [u8; 3]) {
    let offset = ((y * width + x) * 4) as usize;
    rgba[offset] = value[0];
    rgba[offset + 1] = value[1];
    rgba[offset + 2] = value[2];
    rgba[offset + 3] = 255;
}

fn minimal_capabilities() -> RenderIntrospectionCapabilitiesV1 {
    RenderIntrospectionCapabilitiesV1 {
        backend: crate::diagnostics::Backend::Headless,
        gpu_device: false,
        surface_attached: false,
        hardware_tier: crate::diagnostics::HardwareTier::Low,
        forward_pbr: crate::diagnostics::CapabilityStatus::ErrorIfRequired,
        readback_headless_screenshots: crate::diagnostics::CapabilityStatus::Supported,
    }
}

fn quality_input<'a>(
    rgba8: &'a [u8],
    width: u32,
    height: u32,
    visible_pixel_fraction: f32,
    tiny_in_frame: bool,
    fit_fraction: f32,
) -> RenderQualityRgba8Input<'a> {
    RenderQualityRgba8Input {
        rgba8,
        width,
        height,
        capabilities: minimal_capabilities(),
        visible_pixel_fraction,
        tiny_in_frame,
        fit_fraction,
    }
}

fn assert_has_code(report: &RenderQualityReportV1, code: &str) {
    assert!(
        report.checks.iter().any(|check| check.code == code),
        "expected quality code {code} in report: {report:#?}"
    );
}

fn assert_lacks_code(report: &RenderQualityReportV1, code: &str) {
    assert!(
        report.checks.iter().all(|check| check.code != code),
        "expected no quality code {code} in report: {report:#?}"
    );
}

fn write_ppm_crop_artifact(path: &str, rgba: &[u8], frame_width: u32, region: RenderQualityRegion) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("render-quality artifact dir exists");
    }
    let mut ppm = format!("P6\n{} {}\n255\n", region.width, region.height).into_bytes();
    for y in region.y..region.y.saturating_add(region.height) {
        for x in region.x..region.x.saturating_add(region.width) {
            let offset = ((y * frame_width + x) * 4) as usize;
            ppm.extend_from_slice(&rgba[offset..offset + 3]);
        }
    }
    std::fs::write(path, ppm).expect("render-quality crop artifact writes");
}

fn write_ppm_artifact(path: &str, rgba8: &[u8], width: u32, height: u32) {
    let path = Path::new(path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("artifact directory is created");
    }
    let mut ppm = format!("P6\n{width} {height}\n255\n").into_bytes();
    for pixel in rgba8.chunks_exact(4) {
        ppm.extend_from_slice(&pixel[..3]);
    }
    std::fs::write(path, ppm).expect("quality fixture artifact writes");
}

fn read_ppm_fixture(path: impl AsRef<Path>) -> (Vec<u8>, u32, u32) {
    let bytes = std::fs::read(path.as_ref()).expect("PPM fixture reads");
    let mut cursor = 0;
    let magic = next_ppm_token(&bytes, &mut cursor).expect("PPM magic");
    assert_eq!(magic, "P6");
    let width = next_ppm_token(&bytes, &mut cursor)
        .expect("PPM width")
        .parse::<u32>()
        .expect("PPM width parses");
    let height = next_ppm_token(&bytes, &mut cursor)
        .expect("PPM height")
        .parse::<u32>()
        .expect("PPM height parses");
    let max = next_ppm_token(&bytes, &mut cursor).expect("PPM max value");
    assert_eq!(max, "255");
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    let rgb = &bytes[cursor..];
    assert_eq!(rgb.len(), (width * height * 3) as usize);
    let mut rgba = Vec::with_capacity((width * height * 4) as usize);
    for pixel in rgb.chunks_exact(3) {
        rgba.extend_from_slice(pixel);
        rgba.push(255);
    }
    (rgba, width, height)
}

fn next_ppm_token(bytes: &[u8], cursor: &mut usize) -> Option<String> {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
    let start = *cursor;
    while bytes
        .get(*cursor)
        .is_some_and(|byte| !byte.is_ascii_whitespace())
    {
        *cursor += 1;
    }
    (start < *cursor).then(|| String::from_utf8_lossy(&bytes[start..*cursor]).into_owned())
}
