use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{LabelKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelDesc {
    text: String,
    rasterization: LabelRasterization,
    billboard: LabelBillboard,
    color: Color,
    background: Option<Color>,
    halo: Option<Color>,
    size: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LabelMetrics {
    pub glyph_count: usize,
    pub width_px: f32,
    pub height_px: f32,
    pub baseline_px: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelRasterization {
    Sdf,
    Msdf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelBillboard {
    ScreenAligned,
}

impl Scene {
    pub fn add_label(
        &mut self,
        parent: NodeKey,
        label: LabelDesc,
        transform: Transform,
    ) -> Result<LabelKey, LookupError> {
        let label_key = self.labels.insert(label);
        if let Err(error) = self.insert_node(parent, NodeKind::Label(label_key), transform) {
            self.labels.remove(label_key);
            return Err(error);
        }
        Ok(label_key)
    }

    pub fn label(&self, label: LabelKey) -> Option<&LabelDesc> {
        self.labels.get(label)
    }

    pub fn set_label_text(
        &mut self,
        label: LabelKey,
        text: impl Into<String>,
    ) -> Result<(), LookupError> {
        let label = self
            .labels
            .get_mut(label)
            .ok_or(LookupError::LabelNotFound(label))?;
        let text = text.into();
        if label.text != text {
            label.text = text;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }
}

impl LabelDesc {
    pub fn sdf(text: impl Into<String>) -> Self {
        Self::new(text, LabelRasterization::Sdf)
    }

    pub fn msdf(text: impl Into<String>) -> Self {
        Self::new(text, LabelRasterization::Msdf)
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub const fn rasterization(&self) -> LabelRasterization {
        self.rasterization
    }

    pub const fn billboard(&self) -> LabelBillboard {
        self.billboard
    }

    pub const fn color(&self) -> Color {
        self.color
    }

    pub const fn background(&self) -> Option<Color> {
        self.background
    }

    pub const fn halo(&self) -> Option<Color> {
        self.halo
    }

    pub const fn size(&self) -> f32 {
        self.size
    }

    pub fn metrics(&self) -> LabelMetrics {
        label_metrics(&self.text, self.size)
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_background(mut self, color: Color) -> Self {
        self.background = Some(color);
        self
    }

    pub const fn without_background(mut self) -> Self {
        self.background = None;
        self
    }

    pub fn with_halo(mut self, color: Color) -> Self {
        self.halo = Some(color);
        self
    }

    pub const fn without_halo(mut self) -> Self {
        self.halo = None;
        self
    }

    pub(crate) fn glyph_cells(&self) -> Vec<LabelGlyphCell> {
        glyph_cells(&self.text, self.size)
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = readable_size_or(size, self.size);
        self
    }

    pub const fn with_billboard(mut self, billboard: LabelBillboard) -> Self {
        self.billboard = billboard;
        self
    }

    fn new(text: impl Into<String>, rasterization: LabelRasterization) -> Self {
        Self {
            text: text.into(),
            rasterization,
            billboard: LabelBillboard::ScreenAligned,
            color: Color::WHITE,
            background: None,
            halo: None,
            size: 14.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LabelGlyphCell {
    pub(crate) x0_px: f32,
    pub(crate) y0_px: f32,
    pub(crate) x1_px: f32,
    pub(crate) y1_px: f32,
}

const FONT_WIDTH: f32 = 5.0;
const FONT_HEIGHT: f32 = 7.0;
const FONT_ADVANCE: f32 = 6.0;
const FONT_BASELINE: f32 = 6.0;

fn label_metrics(text: &str, size: f32) -> LabelMetrics {
    let glyph_count = text.chars().count();
    let scale = size / FONT_HEIGHT;
    let width_units = if glyph_count == 0 {
        0.0
    } else {
        (glyph_count.saturating_sub(1) as f32 * FONT_ADVANCE) + FONT_WIDTH
    };
    LabelMetrics {
        glyph_count,
        width_px: width_units * scale,
        height_px: size,
        baseline_px: FONT_BASELINE * scale,
    }
}

fn glyph_cells(text: &str, size: f32) -> Vec<LabelGlyphCell> {
    let scale = size / FONT_HEIGHT;
    let mut cells = Vec::new();
    for (index, ch) in text.chars().enumerate() {
        let x_offset = index as f32 * FONT_ADVANCE * scale;
        let rows = glyph_rows(ch);
        for (row, bits) in rows.iter().copied().enumerate() {
            for col in 0..FONT_WIDTH as u8 {
                let mask = 1u8 << ((FONT_WIDTH as u8 - 1 - col) as u32);
                if bits & mask == 0 {
                    continue;
                }
                let x0 = x_offset + col as f32 * scale;
                let y0 = row as f32 * scale;
                cells.push(LabelGlyphCell {
                    x0_px: x0,
                    y0_px: y0,
                    x1_px: x0 + scale,
                    y1_px: y0 + scale,
                });
            }
        }
    }
    cells
}

fn glyph_rows(ch: char) -> [u8; 7] {
    match ch.to_ascii_uppercase() {
        'A' => rows([
            "01110", "10001", "10001", "11111", "10001", "10001", "10001",
        ]),
        'B' => rows([
            "11110", "10001", "10001", "11110", "10001", "10001", "11110",
        ]),
        'C' => rows([
            "01111", "10000", "10000", "10000", "10000", "10000", "01111",
        ]),
        'D' => rows([
            "11110", "10001", "10001", "10001", "10001", "10001", "11110",
        ]),
        'E' => rows([
            "11111", "10000", "10000", "11110", "10000", "10000", "11111",
        ]),
        'F' => rows([
            "11111", "10000", "10000", "11110", "10000", "10000", "10000",
        ]),
        'G' => rows([
            "01111", "10000", "10000", "10011", "10001", "10001", "01111",
        ]),
        'H' => rows([
            "10001", "10001", "10001", "11111", "10001", "10001", "10001",
        ]),
        'I' => rows([
            "11111", "00100", "00100", "00100", "00100", "00100", "11111",
        ]),
        'J' => rows([
            "00111", "00010", "00010", "00010", "10010", "10010", "01100",
        ]),
        'K' => rows([
            "10001", "10010", "10100", "11000", "10100", "10010", "10001",
        ]),
        'L' => rows([
            "10000", "10000", "10000", "10000", "10000", "10000", "11111",
        ]),
        'M' => rows([
            "10001", "11011", "10101", "10101", "10001", "10001", "10001",
        ]),
        'N' => rows([
            "10001", "11001", "10101", "10011", "10001", "10001", "10001",
        ]),
        'O' => rows([
            "01110", "10001", "10001", "10001", "10001", "10001", "01110",
        ]),
        'P' => rows([
            "11110", "10001", "10001", "11110", "10000", "10000", "10000",
        ]),
        'Q' => rows([
            "01110", "10001", "10001", "10001", "10101", "10010", "01101",
        ]),
        'R' => rows([
            "11110", "10001", "10001", "11110", "10100", "10010", "10001",
        ]),
        'S' => rows([
            "01111", "10000", "10000", "01110", "00001", "00001", "11110",
        ]),
        'T' => rows([
            "11111", "00100", "00100", "00100", "00100", "00100", "00100",
        ]),
        'U' => rows([
            "10001", "10001", "10001", "10001", "10001", "10001", "01110",
        ]),
        'V' => rows([
            "10001", "10001", "10001", "10001", "10001", "01010", "00100",
        ]),
        'W' => rows([
            "10001", "10001", "10001", "10101", "10101", "10101", "01010",
        ]),
        'X' => rows([
            "10001", "10001", "01010", "00100", "01010", "10001", "10001",
        ]),
        'Y' => rows([
            "10001", "10001", "01010", "00100", "00100", "00100", "00100",
        ]),
        'Z' => rows([
            "11111", "00001", "00010", "00100", "01000", "10000", "11111",
        ]),
        '0' => rows([
            "01110", "10001", "10011", "10101", "11001", "10001", "01110",
        ]),
        '1' => rows([
            "00100", "01100", "00100", "00100", "00100", "00100", "01110",
        ]),
        '2' => rows([
            "01110", "10001", "00001", "00010", "00100", "01000", "11111",
        ]),
        '3' => rows([
            "11110", "00001", "00001", "01110", "00001", "00001", "11110",
        ]),
        '4' => rows([
            "00010", "00110", "01010", "10010", "11111", "00010", "00010",
        ]),
        '5' => rows([
            "11111", "10000", "10000", "11110", "00001", "00001", "11110",
        ]),
        '6' => rows([
            "01110", "10000", "10000", "11110", "10001", "10001", "01110",
        ]),
        '7' => rows([
            "11111", "00001", "00010", "00100", "01000", "01000", "01000",
        ]),
        '8' => rows([
            "01110", "10001", "10001", "01110", "10001", "10001", "01110",
        ]),
        '9' => rows([
            "01110", "10001", "10001", "01111", "00001", "00001", "01110",
        ]),
        '-' => rows([
            "00000", "00000", "00000", "11111", "00000", "00000", "00000",
        ]),
        '_' => rows([
            "00000", "00000", "00000", "00000", "00000", "00000", "11111",
        ]),
        '.' => rows([
            "00000", "00000", "00000", "00000", "00000", "01100", "01100",
        ]),
        ':' => rows([
            "00000", "01100", "01100", "00000", "01100", "01100", "00000",
        ]),
        '/' => rows([
            "00001", "00010", "00010", "00100", "01000", "01000", "10000",
        ]),
        ' ' => [0; 7],
        _ => rows([
            "01110", "10001", "00001", "00010", "00100", "00000", "00100",
        ]),
    }
}

fn rows(pattern: [&str; 7]) -> [u8; 7] {
    let mut output = [0; 7];
    for (row_index, row) in pattern.iter().enumerate() {
        let mut bits = 0;
        for (col, byte) in row
            .as_bytes()
            .iter()
            .copied()
            .enumerate()
            .take(FONT_WIDTH as usize)
        {
            if byte == b'1' {
                bits |= 1u8 << ((FONT_WIDTH as usize - 1 - col) as u32);
            }
        }
        output[row_index] = bits;
    }
    output
}

fn readable_size_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value >= 4.0 {
        value
    } else {
        fallback
    }
}
