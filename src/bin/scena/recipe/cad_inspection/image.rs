use std::fs;
use std::io::Cursor;
use std::path::Path;

use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub(super) struct RgbaImage {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct PostprocessMetrics {
    foreground_pixels: u64,
    edge_pixels: u64,
    bbox: PixelBbox,
}

#[derive(Debug, Clone, Copy)]
struct PixelBbox {
    min_x: u32,
    min_y: u32,
    max_x: u32,
    max_y: u32,
}

pub(super) fn process_cad_png(
    raw_png: &Path,
    processed_png: &Path,
) -> Result<(RgbaImage, PostprocessMetrics), String> {
    let input = read_png_rgba8(raw_png)?;
    let mut output = input.rgba8.clone();
    let foreground = foreground_mask(&input);
    let mut bbox = None;
    let mut foreground_pixels = 0_u64;
    for y in 0..input.height {
        for x in 0..input.width {
            let index = (y * input.width + x) as usize;
            let offset = index * 4;
            if foreground[index] {
                foreground_pixels += 1;
                let lum = luminance(
                    input.rgba8[offset],
                    input.rgba8[offset + 1],
                    input.rgba8[offset + 2],
                );
                let shade = (lum as f32 / 255.0).clamp(0.0, 1.0);
                output[offset] = (92.0 + shade * 128.0) as u8;
                output[offset + 1] = (123.0 + shade * 104.0) as u8;
                output[offset + 2] = (162.0 + shade * 82.0) as u8;
                output[offset + 3] = 255;
                bbox = Some(match bbox {
                    Some(existing) => expand_bbox(existing, x, y),
                    None => PixelBbox {
                        min_x: x,
                        min_y: y,
                        max_x: x,
                        max_y: y,
                    },
                });
            } else {
                output[offset] = 12;
                output[offset + 1] = 15;
                output[offset + 2] = 20;
                output[offset + 3] = 255;
            }
        }
    }

    let mut edge_pixels = 0_u64;
    for y in 0..input.height {
        for x in 0..input.width {
            let index = (y * input.width + x) as usize;
            if !foreground[index] || !is_edge(&input, &foreground, x, y) {
                continue;
            }
            edge_pixels += 1;
            let offset = index * 4;
            output[offset] = 255;
            output[offset + 1] = 176;
            output[offset + 2] = 48;
            output[offset + 3] = 255;
        }
    }

    let bbox = bbox.ok_or_else(|| {
        format!(
            "raw CAD inspection render '{}' contains no foreground pixels",
            raw_png.display()
        )
    })?;
    write_png_rgba8(processed_png, input.width, input.height, &output)?;
    Ok((
        RgbaImage {
            width: input.width,
            height: input.height,
            rgba8: output,
        },
        PostprocessMetrics {
            foreground_pixels,
            edge_pixels,
            bbox,
        },
    ))
}

pub(super) fn postprocess_json(metrics: PostprocessMetrics, width: u32, height: u32) -> Value {
    let bbox_width = metrics.bbox.max_x.saturating_sub(metrics.bbox.min_x) + 1;
    let bbox_height = metrics.bbox.max_y.saturating_sub(metrics.bbox.min_y) + 1;
    json!({
        "tone_override": true,
        "edge_emphasis": true,
        "presentation_only": true,
        "foreground_pixels": metrics.foreground_pixels,
        "edge_pixels": metrics.edge_pixels,
        "content_bbox_css_px": {
            "min_x": metrics.bbox.min_x,
            "min_y": metrics.bbox.min_y,
            "max_x": metrics.bbox.max_x,
            "max_y": metrics.bbox.max_y,
            "width": bbox_width,
            "height": bbox_height
        },
        "content_bbox_fraction": {
            "min_x": f64::from(metrics.bbox.min_x) / f64::from(width),
            "min_y": f64::from(metrics.bbox.min_y) / f64::from(height),
            "max_x": f64::from(metrics.bbox.max_x) / f64::from(width),
            "max_y": f64::from(metrics.bbox.max_y) / f64::from(height),
            "width": f64::from(bbox_width) / f64::from(width),
            "height": f64::from(bbox_height) / f64::from(height)
        }
    })
}

pub(super) fn write_contact_sheet(images: &[RgbaImage], path: &Path) -> Result<(), String> {
    let first = images
        .first()
        .ok_or_else(|| "contact sheet requires at least one image".to_owned())?;
    let tile_width = first.width;
    let tile_height = first.height;
    let width = tile_width * images.len() as u32;
    let height = tile_height;
    let mut sheet = vec![10_u8; (width * height * 4) as usize];
    for alpha in sheet.chunks_exact_mut(4).map(|pixel| &mut pixel[3]) {
        *alpha = 255;
    }
    for (tile, image) in images.iter().enumerate() {
        if image.width != tile_width || image.height != tile_height {
            return Err("contact sheet images must share dimensions".to_owned());
        }
        let x_offset = tile as u32 * tile_width;
        for y in 0..tile_height {
            for x in 0..tile_width {
                let src = ((y * tile_width + x) * 4) as usize;
                let dst = ((y * width + x + x_offset) * 4) as usize;
                sheet[dst..dst + 4].copy_from_slice(&image.rgba8[src..src + 4]);
            }
        }
    }
    write_png_rgba8(path, width, height, &sheet)
}

fn foreground_mask(image: &RgbaImage) -> Vec<bool> {
    let mut mask = Vec::with_capacity((image.width * image.height) as usize);
    for pixel in image.rgba8.chunks_exact(4) {
        let distance_from_background = pixel[0].abs_diff(16) as u16
            + pixel[1].abs_diff(19) as u16
            + pixel[2].abs_diff(24) as u16;
        mask.push(
            pixel[3] > 0
                && (distance_from_background > 35 || luminance(pixel[0], pixel[1], pixel[2]) > 45),
        );
    }
    mask
}

fn is_edge(image: &RgbaImage, foreground: &[bool], x: u32, y: u32) -> bool {
    let index = (y * image.width + x) as usize;
    let offset = index * 4;
    let lum = luminance(
        image.rgba8[offset],
        image.rgba8[offset + 1],
        image.rgba8[offset + 2],
    );
    for (nx, ny) in neighbor_pixels(image.width, image.height, x, y) {
        let neighbor_index = (ny * image.width + nx) as usize;
        if !foreground[neighbor_index] {
            return true;
        }
        let neighbor_offset = neighbor_index * 4;
        let neighbor_lum = luminance(
            image.rgba8[neighbor_offset],
            image.rgba8[neighbor_offset + 1],
            image.rgba8[neighbor_offset + 2],
        );
        if lum.abs_diff(neighbor_lum) > 10 {
            return true;
        }
    }
    false
}

fn neighbor_pixels(width: u32, height: u32, x: u32, y: u32) -> impl Iterator<Item = (u32, u32)> {
    let mut neighbors = Vec::with_capacity(4);
    if x > 0 {
        neighbors.push((x - 1, y));
    }
    if y > 0 {
        neighbors.push((x, y - 1));
    }
    if x + 1 < width {
        neighbors.push((x + 1, y));
    }
    if y + 1 < height {
        neighbors.push((x, y + 1));
    }
    neighbors.into_iter()
}

fn luminance(red: u8, green: u8, blue: u8) -> u8 {
    (0.2126 * f32::from(red) + 0.7152 * f32::from(green) + 0.0722 * f32::from(blue))
        .round()
        .clamp(0.0, 255.0) as u8
}

fn expand_bbox(bbox: PixelBbox, x: u32, y: u32) -> PixelBbox {
    PixelBbox {
        min_x: bbox.min_x.min(x),
        min_y: bbox.min_y.min(y),
        max_x: bbox.max_x.max(x),
        max_y: bbox.max_y.max(y),
    }
}

fn read_png_rgba8(path: &Path) -> Result<RgbaImage, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("failed to read PNG '{}': {error}", path.display()))?;
    let decoder = png::Decoder::new(Cursor::new(bytes));
    let mut reader = decoder
        .read_info()
        .map_err(|error| format!("failed to read PNG header '{}': {error}", path.display()))?;
    let mut buffer = vec![
        0;
        reader.output_buffer_size().ok_or_else(|| format!(
            "PNG '{}' has unknown output buffer size",
            path.display()
        ))?
    ];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|error| format!("failed to decode PNG '{}': {error}", path.display()))?;
    let raw = &buffer[..info.buffer_size()];
    let rgba8 = match info.color_type {
        png::ColorType::Rgba => raw.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((info.width * info.height * 4) as usize);
            for pixel in raw.chunks_exact(3) {
                out.extend_from_slice(&[pixel[0], pixel[1], pixel[2], 255]);
            }
            out
        }
        other => {
            return Err(format!(
                "unsupported PNG color type {other:?} in '{}'; expected RGB or RGBA",
                path.display()
            ));
        }
    };
    Ok(RgbaImage {
        width: info.width,
        height: info.height,
        rgba8,
    })
}

fn write_png_rgba8(path: &Path, width: u32, height: u32, rgba8: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let file = fs::File::create(path)
        .map_err(|error| format!("failed to create PNG '{}': {error}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder
        .write_header()
        .map_err(|error| format!("failed to write PNG header '{}': {error}", path.display()))?;
    writer
        .write_image_data(rgba8)
        .map_err(|error| format!("failed to write PNG '{}': {error}", path.display()))
}
