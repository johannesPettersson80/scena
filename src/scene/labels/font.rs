use std::collections::BTreeMap;
use std::fmt;
use std::sync::{Arc, Mutex};

use super::{LabelGlyphCell, LabelMetrics};

#[derive(Clone)]
pub struct LabelFontFace {
    fingerprint: u64,
    font: Arc<fontdue::Font>,
    glyph_cache: Arc<Mutex<BTreeMap<CachedGlyphKey, CachedGlyph>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LabelFontError {
    InvalidFont { reason: String },
    UnsupportedText { character: char },
}

#[derive(Debug, Clone, Copy)]
struct TrueTypeLayout {
    metrics: LabelMetrics,
    min_x: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CachedGlyphKey {
    character: char,
    size_bits: u32,
}

#[derive(Debug, Clone)]
struct CachedGlyph {
    xmin: i32,
    ymin: i32,
    width: usize,
    height: usize,
    advance_width: f32,
    alpha: Vec<u8>,
}

impl LabelFontFace {
    pub fn from_truetype_bytes(font_bytes: impl AsRef<[u8]>) -> Result<Self, LabelFontError> {
        let font = fontdue::Font::from_bytes(font_bytes.as_ref(), fontdue::FontSettings::default())
            .map_err(|reason| LabelFontError::InvalidFont {
                reason: reason.to_string(),
            })?;
        Ok(Self {
            fingerprint: stable_font_fingerprint(font_bytes.as_ref()),
            font: Arc::new(font),
            glyph_cache: Arc::new(Mutex::new(BTreeMap::new())),
        })
    }

    fn cached_glyph(&self, character: char, size: f32) -> CachedGlyph {
        let key = CachedGlyphKey {
            character,
            size_bits: size.to_bits(),
        };
        if let Ok(cache) = self.glyph_cache.lock()
            && let Some(glyph) = cache.get(&key)
        {
            return glyph.clone();
        }

        let (metrics, alpha) = self.font.rasterize(character, size);
        let glyph = CachedGlyph {
            xmin: metrics.xmin,
            ymin: metrics.ymin,
            width: metrics.width,
            height: metrics.height,
            advance_width: metrics.advance_width,
            alpha,
        };
        if let Ok(mut cache) = self.glyph_cache.lock() {
            cache.insert(key, glyph.clone());
        }
        glyph
    }
}

impl fmt::Debug for LabelFontFace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LabelFontFace")
            .field("fingerprint", &self.fingerprint)
            .finish()
    }
}

impl PartialEq for LabelFontFace {
    fn eq(&self, other: &Self) -> bool {
        self.fingerprint == other.fingerprint
    }
}

impl fmt::Display for LabelFontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFont { reason } => write!(formatter, "font could not be parsed: {reason}"),
            Self::UnsupportedText { character } => write!(
                formatter,
                "TrueType labels support basic Latin text only; unsupported character U+{:04X}",
                *character as u32
            ),
        }
    }
}

impl std::error::Error for LabelFontError {}

pub(super) fn truetype_metrics(font: &LabelFontFace, text: &str, size: f32) -> LabelMetrics {
    truetype_layout(font, text, size).metrics
}

pub(super) fn truetype_glyph_cells(
    font: &LabelFontFace,
    text: &str,
    size: f32,
) -> Vec<LabelGlyphCell> {
    let layout = truetype_layout(font, text, size);
    let baseline = layout.metrics.baseline_px;
    let mut cells = Vec::new();
    let mut pen_x = 0.0;
    let mut previous = None;
    for ch in text.chars() {
        if let Some(previous) = previous {
            pen_x += font.font.horizontal_kern(previous, ch, size).unwrap_or(0.0);
        }
        let glyph = font.cached_glyph(ch, size);
        let glyph_left = pen_x + glyph.xmin as f32 - layout.min_x;
        let glyph_top = baseline - (glyph.ymin as f32 + glyph.height as f32);
        for row in 0..glyph.height {
            for col in 0..glyph.width {
                let index = row * glyph.width + col;
                let Some(alpha) = glyph.alpha.get(index).copied() else {
                    continue;
                };
                if alpha <= 16 {
                    continue;
                }
                let x0 = glyph_left + col as f32;
                let y0 = glyph_top + row as f32;
                cells.push(LabelGlyphCell {
                    x0_px: x0,
                    y0_px: y0,
                    x1_px: x0 + 1.0,
                    y1_px: y0 + 1.0,
                });
            }
        }
        pen_x += glyph.advance_width;
        previous = Some(ch);
    }
    cells
}

pub(super) fn validate_basic_latin_text(text: &str) -> Result<(), LabelFontError> {
    for character in text.chars() {
        if matches!(character, '\u{20}'..='\u{7e}') {
            continue;
        }
        return Err(LabelFontError::UnsupportedText { character });
    }
    Ok(())
}

fn truetype_layout(font: &LabelFontFace, text: &str, size: f32) -> TrueTypeLayout {
    let glyph_count = text.chars().count();
    let Some(line_metrics) = font.font.horizontal_line_metrics(size) else {
        return TrueTypeLayout {
            metrics: LabelMetrics {
                glyph_count,
                width_px: 0.0,
                height_px: size,
                baseline_px: size * 0.8,
            },
            min_x: 0.0,
        };
    };
    if glyph_count == 0 {
        return TrueTypeLayout {
            metrics: LabelMetrics {
                glyph_count,
                width_px: 0.0,
                height_px: (line_metrics.ascent - line_metrics.descent).max(size),
                baseline_px: line_metrics.ascent,
            },
            min_x: 0.0,
        };
    }

    let mut pen_x = 0.0;
    let mut min_x = 0.0f32;
    let mut min_y = 0.0f32;
    let mut max_x = 0.0f32;
    let mut max_y = (line_metrics.ascent - line_metrics.descent).max(size);
    let mut previous = None;
    for ch in text.chars() {
        if let Some(previous) = previous {
            pen_x += font.font.horizontal_kern(previous, ch, size).unwrap_or(0.0);
        }
        let glyph = font.cached_glyph(ch, size);
        let glyph_left = pen_x + glyph.xmin as f32;
        let glyph_right = glyph_left + glyph.width as f32;
        let glyph_top = line_metrics.ascent - (glyph.ymin as f32 + glyph.height as f32);
        let glyph_bottom = glyph_top + glyph.height as f32;
        min_x = min_x.min(glyph_left);
        min_y = min_y.min(glyph_top);
        max_x = max_x.max(glyph_right);
        max_y = max_y.max(glyph_bottom);
        pen_x += glyph.advance_width;
        max_x = max_x.max(pen_x);
        previous = Some(ch);
    }

    TrueTypeLayout {
        metrics: LabelMetrics {
            glyph_count,
            width_px: (max_x - min_x).max(0.0),
            height_px: (max_y - min_y).max(size),
            baseline_px: line_metrics.ascent - min_y,
        },
        min_x,
    }
}

fn stable_font_fingerprint(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}
