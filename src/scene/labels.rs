use crate::diagnostics::LookupError;
use crate::material::Color;
use std::sync::Arc;

use super::{LabelKey, NodeKey, NodeKind, Scene, Transform};

mod font;

pub use font::{LabelFontError, LabelFontFace};
use font::{
    default_label_font_face, truetype_glyph_rasters, truetype_metrics, validate_basic_latin_text,
};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelDesc {
    text: String,
    font: LabelFontFace,
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
        Ok(self.add_label_node(parent, label, transform)?.0)
    }

    pub(crate) fn add_label_node(
        &mut self,
        parent: NodeKey,
        label: LabelDesc,
        transform: Transform,
    ) -> Result<(LabelKey, NodeKey), LookupError> {
        label.validate_style()?;
        let label_key = self.labels.insert(label);
        match self.insert_node(parent, NodeKind::Label(label_key), transform) {
            Ok(node) => Ok((label_key, node)),
            Err(error) => {
                self.labels.remove(label_key);
                Err(error)
            }
        }
    }

    pub fn label(&self, label: LabelKey) -> Option<&LabelDesc> {
        self.labels.get(label)
    }

    pub fn set_label_text(
        &mut self,
        label: LabelKey,
        text: impl Into<String>,
    ) -> Result<(), LookupError> {
        let label_key = label;
        let label = self
            .labels
            .get_mut(label_key)
            .ok_or(LookupError::LabelNotFound(label_key))?;
        let text = text.into();
        if let Err(error) = label.validate_text(&text) {
            return Err(LookupError::UnsupportedLabelText {
                label: label_key,
                reason: error.to_string(),
            });
        }
        if label.text != text {
            label.text = text;
            self.structure_revision = self.structure_revision.saturating_add(1);
        }
        Ok(())
    }
}

impl LabelDesc {
    /// Creates a label rendered through scena's embedded TrueType font.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font: default_label_font_face(),
            billboard: LabelBillboard::ScreenAligned,
            color: Color::WHITE,
            background: None,
            halo: None,
            size: 14.0,
        }
    }

    /// Creates a label rendered through a caller-supplied TrueType/OpenType font.
    ///
    /// This first font path intentionally supports basic Latin text only. Complex
    /// shaping scripts are rejected instead of being approximated by broken glyph
    /// order or fallback boxes.
    pub fn truetype(
        text: impl Into<String>,
        font_bytes: impl AsRef<[u8]>,
    ) -> Result<Self, LabelFontError> {
        let text = text.into();
        validate_basic_latin_text(&text)?;
        let font = LabelFontFace::from_truetype_bytes(font_bytes)?;
        Self::truetype_face(text, font)
    }

    pub fn truetype_face(
        text: impl Into<String>,
        font: LabelFontFace,
    ) -> Result<Self, LabelFontError> {
        let text = text.into();
        validate_basic_latin_text(&text)?;
        Ok(Self {
            text,
            font,
            billboard: LabelBillboard::ScreenAligned,
            color: Color::WHITE,
            background: None,
            halo: None,
            size: 14.0,
        })
    }

    pub fn text(&self) -> &str {
        &self.text
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
        truetype_metrics(&self.font, &self.text, self.size)
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

    pub(crate) fn glyph_rasters(&self) -> Vec<LabelGlyphRaster> {
        truetype_glyph_rasters(&self.font, &self.text, self.size)
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = readable_size_or(size, self.size);
        self
    }

    pub const fn with_billboard(mut self, billboard: LabelBillboard) -> Self {
        self.billboard = billboard;
        self
    }

    fn validate_text(&self, text: &str) -> Result<(), LabelFontError> {
        validate_basic_latin_text(text)
    }

    fn validate_style(&self) -> Result<(), LookupError> {
        if self.validate_text(&self.text).is_err() {
            return Err(LookupError::InvalidLabelStyle {
                field: "label text",
                reason: "embedded label font supports basic Latin text only",
            });
        }
        validate_opaque_color("label text color", self.color)?;
        if let Some(background) = self.background {
            validate_opaque_color("label background color", background)?;
        }
        if let Some(halo) = self.halo {
            validate_opaque_color("label halo color", halo)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LabelGlyphRaster {
    pub(crate) x_px: f32,
    pub(crate) y_px: f32,
    pub(crate) width_px: f32,
    pub(crate) height_px: f32,
    pub(crate) alpha_width: u32,
    pub(crate) alpha_height: u32,
    pub(crate) alpha: Arc<[u8]>,
}

fn readable_size_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value >= 4.0 {
        value
    } else {
        fallback
    }
}

fn validate_opaque_color(field: &'static str, color: Color) -> Result<(), LookupError> {
    if color.r.is_finite()
        && color.g.is_finite()
        && color.b.is_finite()
        && color.a.is_finite()
        && color.a == 1.0
    {
        Ok(())
    } else {
        Err(LookupError::InvalidLabelStyle {
            field,
            reason: "label billboard colors must be finite and opaque until a transparent label path exists",
        })
    }
}
