use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::scena_recipe::capture_shared::{
    RgbaFrameRef, compose_contact_sheet_rgba8, write_png_rgba8,
};

pub(super) struct CapturedFrame {
    pub(super) contact_sheet_capture: scena::CaptureRgba8,
    pub(super) report: Value,
}

pub(super) fn capture_frame(
    host: &mut scena::SceneHostCore,
    out_dir: &Path,
    index: usize,
    kind: &str,
    label: &str,
    camera: scena::SceneHostCameraState,
    sequence: Value,
) -> Result<CapturedFrame, String> {
    host.set_camera(camera)
        .map_err(|error| format!("failed to apply {label} camera: {error}"))?;
    host.prepare()
        .map_err(|error| format!("failed to prepare {label} capture: {error}"))?;
    host.render()
        .map_err(|error| format!("failed to render {label} capture: {error}"))?;
    let capture = host
        .capture()
        .map_err(|error| format!("failed to capture {label}: {error}"))?;
    let file_stem = format!("{index:03}-{}", safe_file_label(label));
    let png = out_dir.join(format!("{file_stem}.png"));
    let descriptor = out_dir.join(format!("{file_stem}.capture.json"));
    capture
        .write_png(&png)
        .map_err(|error| format!("failed to write '{}': {error}", png.display()))?;
    std::fs::write(
        &descriptor,
        serde_json::to_vec_pretty(&capture.descriptor)
            .map_err(|error| format!("failed to encode capture descriptor: {error}"))?,
    )
    .map_err(|error| format!("failed to write '{}': {error}", descriptor.display()))?;
    let mut report = json!({
        "index": index,
        "kind": kind,
        "label": label,
        "png": path_json(&png),
        "descriptor_json": path_json(&descriptor),
        "camera": camera,
        "capture": capture.descriptor.clone(),
    });
    let report_object = report
        .as_object_mut()
        .expect("capture frame report is constructed as an object");
    let sequence_object = sequence
        .as_object()
        .ok_or_else(|| "capture sequence metadata must be a JSON object".to_owned())?;
    report_object.extend(sequence_object.clone());
    let contact_sheet_capture = contact_sheet_thumbnail(&capture)?;
    Ok(CapturedFrame {
        contact_sheet_capture,
        report,
    })
}

pub(super) fn write_contact_sheet(
    out_dir: &Path,
    captures: &[scena::CaptureRgba8],
    frames: &[Value],
) -> Result<Value, String> {
    let columns = u32::try_from(captures.len().min(4)).unwrap_or(1).max(1);
    let sheet_frames = captures
        .iter()
        .map(|capture| RgbaFrameRef {
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            rgba8: &capture.rgba8,
        })
        .collect::<Vec<_>>();
    let sheet = compose_contact_sheet_rgba8(&sheet_frames, columns, [0, 0, 0, 0])?;
    let png = out_dir.join("capture-contact-sheet.png");
    write_png_rgba8(&png, sheet.width, sheet.height, &sheet.rgba8)?;
    let tiles = sheet
        .tiles
        .iter()
        .map(|tile| {
            json!({
                "index": tile.index,
                "label": frames[tile.index]["label"],
                "kind": frames[tile.index]["kind"],
                "x": tile.x,
                "y": tile.y,
                "width": tile.width,
                "height": tile.height,
                "capture_payload_fnv1a64": frames[tile.index]["capture"]["payload"]["fnv1a64"],
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "png": path_json(&png),
        "width": sheet.width,
        "height": sheet.height,
        "columns": columns,
        "tile_max_dimension": CONTACT_SHEET_TILE_MAX_DIMENSION,
        "resampling": "nearest",
        "tiles": tiles,
    }))
}

pub(super) fn path_json(path: &Path) -> String {
    path.display().to_string()
}

pub(super) fn ensure_output_dir(path: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(path).map_err(|error| {
        format!(
            "failed to create output directory '{}': {error}",
            path.display()
        )
    })?;
    Ok(path.to_path_buf())
}

fn safe_file_label(label: &str) -> String {
    const MAX_FILE_LABEL_CHARS: usize = 96;
    let sanitized = label
        .chars()
        .take(MAX_FILE_LABEL_CHARS)
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    if sanitized.is_empty() {
        "frame".to_owned()
    } else {
        sanitized
    }
}

const CONTACT_SHEET_TILE_MAX_DIMENSION: u32 = 192;

fn thumbnail_dimensions(width: u32, height: u32) -> (u32, u32) {
    let longest = width.max(height);
    if longest <= CONTACT_SHEET_TILE_MAX_DIMENSION || longest == 0 {
        return (width, height);
    }
    let scale = u64::from(CONTACT_SHEET_TILE_MAX_DIMENSION);
    let longest = u64::from(longest);
    let scaled_width = (u64::from(width) * scale / longest).max(1) as u32;
    let scaled_height = (u64::from(height) * scale / longest).max(1) as u32;
    (scaled_width, scaled_height)
}

fn contact_sheet_thumbnail(capture: &scena::CaptureRgba8) -> Result<scena::CaptureRgba8, String> {
    let source_width = capture.descriptor.width;
    let source_height = capture.descriptor.height;
    let (width, height) = thumbnail_dimensions(source_width, source_height);
    if (width, height) == (source_width, source_height) {
        return Ok(capture.clone());
    }

    let mut rgba8 = vec![0_u8; width as usize * height as usize * 4];
    for y in 0..height {
        let source_y = (u64::from(y) * u64::from(source_height) / u64::from(height)) as u32;
        for x in 0..width {
            let source_x = (u64::from(x) * u64::from(source_width) / u64::from(width)) as u32;
            let source_offset =
                ((source_y as usize * source_width as usize) + source_x as usize) * 4;
            let destination_offset = ((y as usize * width as usize) + x as usize) * 4;
            rgba8[destination_offset..destination_offset + 4]
                .copy_from_slice(&capture.rgba8[source_offset..source_offset + 4]);
        }
    }

    let mut descriptor = capture.descriptor.clone();
    descriptor.width = width;
    descriptor.height = height;
    descriptor.payload.byte_length = rgba8.len();
    descriptor.payload.fnv1a64 = scena::fnv1a64_hex(&rgba8);
    descriptor.pixels = scena::summarize_rgba8(width, height, &rgba8)
        .map_err(|error| format!("failed to summarize contact-sheet thumbnail: {error}"))?;
    Ok(scena::CaptureRgba8 { descriptor, rgba8 })
}

#[cfg(test)]
mod tests {
    use super::{safe_file_label, thumbnail_dimensions};

    #[test]
    fn capture_sequence_file_labels_cannot_escape_the_output_directory() {
        assert_eq!(
            safe_file_label("clip-../Door/Open 50%"),
            "clip-___Door_Open_50_"
        );
        assert_eq!(safe_file_label("front"), "front");
    }

    #[test]
    fn capture_sequence_contact_sheet_tiles_have_a_bounded_memory_footprint() {
        assert_eq!(thumbnail_dimensions(4096, 2160), (192, 101));
        assert_eq!(thumbnail_dimensions(120, 160), (120, 160));
    }
}
