use std::fmt;

/// Owned RGBA8 image used by public visual-regression helpers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceImage {
    width: u32,
    height: u32,
    rgba8: Vec<u8>,
}

/// Pixel-diff tolerance for [`regress_with_tolerance`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReferenceImageTolerance {
    max_abs_diff: u8,
    max_mismatched_pixels: usize,
}

/// Diff summary returned by reference-image regression checks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReferenceImageReport {
    width: u32,
    height: u32,
    total_pixels: usize,
    mismatched_pixels: usize,
    max_abs_diff: u8,
    total_abs_diff: u64,
    channel_count: usize,
    tolerance: ReferenceImageTolerance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceImageError {
    InvalidRgbaLength {
        width: u32,
        height: u32,
        expected_len: usize,
        actual_len: usize,
    },
    DimensionMismatch {
        actual_width: u32,
        actual_height: u32,
        expected_width: u32,
        expected_height: u32,
    },
    DiffExceeded(ReferenceImageReport),
}

impl ReferenceImage {
    /// Creates an owned RGBA8 reference image from row-major bytes.
    pub fn from_rgba8(
        width: u32,
        height: u32,
        rgba8: Vec<u8>,
    ) -> Result<Self, ReferenceImageError> {
        let expected_len = expected_rgba_len(width, height);
        if rgba8.len() != expected_len {
            return Err(ReferenceImageError::InvalidRgbaLength {
                width,
                height,
                expected_len,
                actual_len: rgba8.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba8,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width
    }

    pub const fn height(&self) -> u32 {
        self.height
    }

    pub fn rgba8(&self) -> &[u8] {
        &self.rgba8
    }

    pub fn into_rgba8(self) -> Vec<u8> {
        self.rgba8
    }
}

impl ReferenceImageTolerance {
    /// Exact reference-image comparison.
    pub const fn new() -> Self {
        Self {
            max_abs_diff: 0,
            max_mismatched_pixels: 0,
        }
    }

    pub const fn exact() -> Self {
        Self::new()
    }

    pub const fn with_max_abs_diff(mut self, max_abs_diff: u8) -> Self {
        self.max_abs_diff = max_abs_diff;
        self
    }

    pub const fn with_max_mismatched_pixels(mut self, max_mismatched_pixels: usize) -> Self {
        self.max_mismatched_pixels = max_mismatched_pixels;
        self
    }

    pub const fn max_abs_diff(self) -> u8 {
        self.max_abs_diff
    }

    pub const fn max_mismatched_pixels(self) -> usize {
        self.max_mismatched_pixels
    }
}

impl ReferenceImageReport {
    pub const fn passed(self) -> bool {
        self.mismatched_pixels <= self.tolerance.max_mismatched_pixels
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }

    pub const fn total_pixels(self) -> usize {
        self.total_pixels
    }

    pub const fn mismatched_pixels(self) -> usize {
        self.mismatched_pixels
    }

    pub const fn max_abs_diff(self) -> u8 {
        self.max_abs_diff
    }

    pub const fn total_abs_diff(self) -> u64 {
        self.total_abs_diff
    }

    pub const fn channel_count(self) -> usize {
        self.channel_count
    }

    pub const fn tolerance(self) -> ReferenceImageTolerance {
        self.tolerance
    }

    pub fn mean_abs_diff(self) -> f64 {
        if self.channel_count == 0 {
            0.0
        } else {
            self.total_abs_diff as f64 / self.channel_count as f64
        }
    }
}

/// Compares two RGBA8 reference images with exact tolerance.
pub fn regress(
    actual: &ReferenceImage,
    expected: &ReferenceImage,
) -> Result<ReferenceImageReport, ReferenceImageError> {
    regress_with_tolerance(actual, expected, ReferenceImageTolerance::exact())
}

/// Compares two RGBA8 reference images with explicit channel and pixel tolerance.
pub fn regress_with_tolerance(
    actual: &ReferenceImage,
    expected: &ReferenceImage,
    tolerance: ReferenceImageTolerance,
) -> Result<ReferenceImageReport, ReferenceImageError> {
    if actual.width != expected.width || actual.height != expected.height {
        return Err(ReferenceImageError::DimensionMismatch {
            actual_width: actual.width,
            actual_height: actual.height,
            expected_width: expected.width,
            expected_height: expected.height,
        });
    }

    let total_pixels = (actual.width as usize).saturating_mul(actual.height as usize);
    let mut mismatched_pixels = 0usize;
    let mut max_abs_diff = 0u8;
    let mut total_abs_diff = 0u64;

    for (actual_pixel, expected_pixel) in actual
        .rgba8
        .chunks_exact(4)
        .zip(expected.rgba8.chunks_exact(4))
    {
        let mut pixel_mismatched = false;
        for (actual_channel, expected_channel) in actual_pixel.iter().zip(expected_pixel) {
            let diff = actual_channel.abs_diff(*expected_channel);
            max_abs_diff = max_abs_diff.max(diff);
            total_abs_diff = total_abs_diff.saturating_add(u64::from(diff));
            if diff > tolerance.max_abs_diff {
                pixel_mismatched = true;
            }
        }
        if pixel_mismatched {
            mismatched_pixels = mismatched_pixels.saturating_add(1);
        }
    }

    let report = ReferenceImageReport {
        width: actual.width,
        height: actual.height,
        total_pixels,
        mismatched_pixels,
        max_abs_diff,
        total_abs_diff,
        channel_count: actual.rgba8.len(),
        tolerance,
    };

    if report.passed() {
        Ok(report)
    } else {
        Err(ReferenceImageError::DiffExceeded(report))
    }
}

fn expected_rgba_len(width: u32, height: u32) -> usize {
    (width as usize)
        .saturating_mul(height as usize)
        .saturating_mul(4)
}

impl fmt::Display for ReferenceImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRgbaLength {
                width,
                height,
                expected_len,
                actual_len,
            } => write!(
                formatter,
                "reference image {width}x{height} has {actual_len} RGBA8 bytes; expected {expected_len}"
            ),
            Self::DimensionMismatch {
                actual_width,
                actual_height,
                expected_width,
                expected_height,
            } => write!(
                formatter,
                "reference image dimensions differ: actual {actual_width}x{actual_height}, expected {expected_width}x{expected_height}"
            ),
            Self::DiffExceeded(report) => write!(
                formatter,
                "reference image diff exceeded tolerance: {} mismatched pixels, max channel diff {}",
                report.mismatched_pixels(),
                report.max_abs_diff()
            ),
        }
    }
}

impl std::error::Error for ReferenceImageError {}
