use crate::scena_cli_error::{CliErrorKind, CliFailure};
use std::path::Path;

pub(in crate::scena_recipe) struct RgbaFrameRef<'a> {
    pub(in crate::scena_recipe) width: u32,
    pub(in crate::scena_recipe) height: u32,
    pub(in crate::scena_recipe) rgba8: &'a [u8],
}

pub(in crate::scena_recipe) struct ContactSheetTile {
    pub(in crate::scena_recipe) index: usize,
    pub(in crate::scena_recipe) x: u32,
    pub(in crate::scena_recipe) y: u32,
    pub(in crate::scena_recipe) width: u32,
    pub(in crate::scena_recipe) height: u32,
}

pub(in crate::scena_recipe) struct ContactSheetRgba8 {
    pub(in crate::scena_recipe) width: u32,
    pub(in crate::scena_recipe) height: u32,
    pub(in crate::scena_recipe) rgba8: Vec<u8>,
    pub(in crate::scena_recipe) tiles: Vec<ContactSheetTile>,
}

pub(in crate::scena_recipe) fn compose_contact_sheet_rgba8(
    frames: &[RgbaFrameRef<'_>],
    columns: u32,
    background: [u8; 4],
) -> Result<ContactSheetRgba8, String> {
    if frames.is_empty() {
        return Err("contact sheet requires at least one frame".to_owned());
    }
    if columns == 0 {
        return Err("contact sheet columns must be greater than zero".to_owned());
    }
    for (index, frame) in frames.iter().enumerate() {
        let expected = rgba8_len(frame.width, frame.height)?;
        if frame.rgba8.len() != expected {
            return Err(format!(
                "contact sheet frame {index} has {} RGBA bytes; expected {expected}",
                frame.rgba8.len()
            ));
        }
    }

    let tile_width = frames.iter().map(|frame| frame.width).max().unwrap_or(0);
    let tile_height = frames.iter().map(|frame| frame.height).max().unwrap_or(0);
    let frame_count = u32::try_from(frames.len())
        .map_err(|_| "contact sheet frame count exceeds u32".to_owned())?;
    let rows = frame_count.div_ceil(columns);
    let width = tile_width
        .checked_mul(columns)
        .ok_or_else(|| "contact sheet width overflowed u32".to_owned())?;
    let height = tile_height
        .checked_mul(rows)
        .ok_or_else(|| "contact sheet height overflowed u32".to_owned())?;
    let mut rgba8 = vec![0_u8; rgba8_len(width, height)?];
    for pixel in rgba8.chunks_exact_mut(4) {
        pixel.copy_from_slice(&background);
    }

    let mut tiles = Vec::with_capacity(frames.len());
    for (index, frame) in frames.iter().enumerate() {
        let index_u32 = index as u32;
        let x = (index_u32 % columns) * tile_width;
        let y = (index_u32 / columns) * tile_height;
        for source_y in 0..frame.height {
            let source = source_y as usize * frame.width as usize * 4;
            let destination = ((y + source_y) as usize * width as usize + x as usize) * 4;
            let row_bytes = frame.width as usize * 4;
            rgba8[destination..destination + row_bytes]
                .copy_from_slice(&frame.rgba8[source..source + row_bytes]);
        }
        tiles.push(ContactSheetTile {
            index,
            x,
            y,
            width: frame.width,
            height: frame.height,
        });
    }
    Ok(ContactSheetRgba8 {
        width,
        height,
        rgba8,
        tiles,
    })
}

pub(in crate::scena_recipe) fn write_png_rgba8(
    path: &Path,
    width: u32,
    height: u32,
    rgba8: &[u8],
) -> Result<(), CliFailure> {
    let expected = rgba8_len(width, height)?;
    if rgba8.len() != expected {
        return Err(CliFailure::new(
            CliErrorKind::Internal,
            format!(
                "PNG '{}' has {} RGBA bytes; expected {expected}",
                path.display(),
                rgba8.len()
            ),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create PNG '{}': {error}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG header '{}': {error}", path.display()),
        )
    })?;
    writer.write_image_data(rgba8).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG '{}': {error}", path.display()),
        )
    })
}

pub(in crate::scena_recipe) fn write_png_gray16(
    path: &Path,
    width: u32,
    height: u32,
    samples: &[u16],
) -> Result<(), CliFailure> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| format!("grayscale dimensions {width}x{height} overflow usize"))?;
    if samples.len() != expected {
        return Err(CliFailure::new(
            CliErrorKind::Internal,
            format!(
                "PNG '{}' has {} grayscale samples; expected {expected}",
                path.display(),
                samples.len()
            ),
        ));
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| format!("failed to create '{}': {error}", parent.display()))?;
    }
    let bytes = samples
        .iter()
        .flat_map(|sample| sample.to_be_bytes())
        .collect::<Vec<_>>();
    let file = std::fs::File::create(path)
        .map_err(|error| format!("failed to create PNG '{}': {error}", path.display()))?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Grayscale);
    encoder.set_depth(png::BitDepth::Sixteen);
    let mut writer = encoder.write_header().map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG header '{}': {error}", path.display()),
        )
    })?;
    writer.write_image_data(&bytes).map_err(|error| {
        CliFailure::new(
            CliErrorKind::Io,
            format!("failed to write PNG '{}': {error}", path.display()),
        )
    })
}

fn rgba8_len(width: u32, height: u32) -> Result<usize, String> {
    (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| format!("RGBA dimensions {width}x{height} overflow usize"))
}
