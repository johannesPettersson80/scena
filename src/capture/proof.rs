use std::error::Error as StdError;
use std::fmt;

use serde::{Deserialize, Serialize};

use super::{CaptureDescriptor, CapturePngError, CaptureRgba8, png};
use crate::reference_image::{
    ReferenceImage, ReferenceImageError, ReferenceImageReport, ReferenceImageTolerance,
    regress_with_tolerance,
};

pub const CAPTURE_BASELINE_SCHEMA_V1: &str = "scena.capture_baseline.v1";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureContactSheet {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
    tiles: Vec<CaptureContactSheetTile>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureContactSheetTile {
    pub index: usize,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub descriptor: CaptureDescriptor,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureContactSheetError {
    Empty,
    InvalidColumns,
    Encode(CapturePngError),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureBaselineReport {
    pub schema: String,
    pub status: String,
    pub actual: CaptureDescriptor,
    pub expected: CaptureDescriptor,
    pub tolerance: CaptureBaselineTolerance,
    pub diff: CaptureBaselineDiff,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureBaselineTolerance {
    pub max_abs_diff: u8,
    pub max_mismatched_pixels: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CaptureBaselineDiff {
    pub width: u32,
    pub height: u32,
    pub total_pixels: usize,
    pub mismatched_pixels: usize,
    pub max_abs_diff: u8,
    pub mean_abs_diff: f64,
    pub total_abs_diff: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaptureBaselineError {
    ReferenceImage(ReferenceImageError),
    DiffExceeded(Box<CaptureBaselineReport>),
}

pub fn capture_contact_sheet_rgba8(
    captures: &[CaptureRgba8],
    columns: u32,
) -> Result<CaptureContactSheet, CaptureContactSheetError> {
    if captures.is_empty() {
        return Err(CaptureContactSheetError::Empty);
    }
    if columns == 0 {
        return Err(CaptureContactSheetError::InvalidColumns);
    }

    let tile_width = captures
        .iter()
        .map(|capture| capture.descriptor.width)
        .max()
        .unwrap_or(0);
    let tile_height = captures
        .iter()
        .map(|capture| capture.descriptor.height)
        .max()
        .unwrap_or(0);
    let rows = (captures.len() as u32).div_ceil(columns);
    let width = tile_width.saturating_mul(columns);
    let height = tile_height.saturating_mul(rows);
    let mut rgba8 = vec![0; width as usize * height as usize * 4];
    let mut tiles = Vec::with_capacity(captures.len());

    for (index, capture) in captures.iter().enumerate() {
        let index_u32 = index as u32;
        let tile_x = (index_u32 % columns).saturating_mul(tile_width);
        let tile_y = (index_u32 / columns).saturating_mul(tile_height);
        blit_capture(capture, width, tile_x, tile_y, rgba8.as_mut_slice());
        tiles.push(CaptureContactSheetTile {
            index,
            x: tile_x,
            y: tile_y,
            width: capture.descriptor.width,
            height: capture.descriptor.height,
            descriptor: capture.descriptor.clone(),
        });
    }

    Ok(CaptureContactSheet {
        width,
        height,
        rgba8,
        tiles,
    })
}

pub fn compare_captures_with_tolerance(
    actual: &CaptureRgba8,
    expected: &CaptureRgba8,
    tolerance: ReferenceImageTolerance,
) -> Result<CaptureBaselineReport, CaptureBaselineError> {
    let actual_image = ReferenceImage::from_rgba8(
        actual.descriptor.width,
        actual.descriptor.height,
        actual.rgba8.clone(),
    )
    .map_err(CaptureBaselineError::ReferenceImage)?;
    let expected_image = ReferenceImage::from_rgba8(
        expected.descriptor.width,
        expected.descriptor.height,
        expected.rgba8.clone(),
    )
    .map_err(CaptureBaselineError::ReferenceImage)?;

    match regress_with_tolerance(&actual_image, &expected_image, tolerance) {
        Ok(diff) => Ok(baseline_report(actual, expected, tolerance, diff, "passed")),
        Err(ReferenceImageError::DiffExceeded(diff)) => Err(CaptureBaselineError::DiffExceeded(
            Box::new(baseline_report(actual, expected, tolerance, diff, "failed")),
        )),
        Err(error) => Err(CaptureBaselineError::ReferenceImage(error)),
    }
}

impl CaptureContactSheet {
    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba8(&self) -> &[u8] {
        self.rgba8.as_slice()
    }

    pub fn tiles(&self) -> &[CaptureContactSheetTile] {
        self.tiles.as_slice()
    }

    pub fn to_png_bytes(&self) -> Result<Vec<u8>, CaptureContactSheetError> {
        png::encode_png_rgba8(self.width, self.height, self.rgba8.as_slice())
            .map_err(CaptureContactSheetError::Encode)
    }
}

impl From<ReferenceImageTolerance> for CaptureBaselineTolerance {
    fn from(tolerance: ReferenceImageTolerance) -> Self {
        Self {
            max_abs_diff: tolerance.max_abs_diff(),
            max_mismatched_pixels: tolerance.max_mismatched_pixels(),
        }
    }
}

impl fmt::Display for CaptureContactSheetError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(formatter, "contact sheet requires at least one capture"),
            Self::InvalidColumns => {
                write!(formatter, "contact sheet columns must be greater than zero")
            }
            Self::Encode(error) => error.fmt(formatter),
        }
    }
}

impl StdError for CaptureContactSheetError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Encode(error) => Some(error),
            Self::Empty | Self::InvalidColumns => None,
        }
    }
}

impl fmt::Display for CaptureBaselineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReferenceImage(error) => error.fmt(formatter),
            Self::DiffExceeded(report) => write!(
                formatter,
                "capture baseline diff exceeded tolerance: {} mismatched pixels, max channel diff {}",
                report.diff.mismatched_pixels, report.diff.max_abs_diff
            ),
        }
    }
}

impl StdError for CaptureBaselineError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::ReferenceImage(error) => Some(error),
            Self::DiffExceeded(_) => None,
        }
    }
}

fn baseline_report(
    actual: &CaptureRgba8,
    expected: &CaptureRgba8,
    tolerance: ReferenceImageTolerance,
    diff: ReferenceImageReport,
    status: &str,
) -> CaptureBaselineReport {
    CaptureBaselineReport {
        schema: CAPTURE_BASELINE_SCHEMA_V1.to_owned(),
        status: status.to_owned(),
        actual: actual.descriptor.clone(),
        expected: expected.descriptor.clone(),
        tolerance: tolerance.into(),
        diff: CaptureBaselineDiff {
            width: diff.width(),
            height: diff.height(),
            total_pixels: diff.total_pixels(),
            mismatched_pixels: diff.mismatched_pixels(),
            max_abs_diff: diff.max_abs_diff(),
            mean_abs_diff: diff.mean_abs_diff(),
            total_abs_diff: diff.total_abs_diff(),
        },
    }
}

fn blit_capture(capture: &CaptureRgba8, sheet_width: u32, x: u32, y: u32, sheet: &mut [u8]) {
    let capture_width = capture.descriptor.width;
    let capture_height = capture.descriptor.height;
    for source_y in 0..capture_height {
        let source_offset = source_y as usize * capture_width as usize * 4;
        let dest_offset =
            ((y + source_y) as usize * sheet_width as usize + x as usize).saturating_mul(4);
        let row_len = capture_width as usize * 4;
        let source = &capture.rgba8[source_offset..source_offset + row_len];
        let dest = &mut sheet[dest_offset..dest_offset + row_len];
        dest.copy_from_slice(source);
    }
}
