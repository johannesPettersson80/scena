use crate::diagnostics::LookupError;
use crate::material::Color;

use super::{LabelKey, NodeKey, NodeKind, Scene, Transform};

#[derive(Debug, Clone, PartialEq)]
pub struct LabelDesc {
    text: String,
    rasterization: LabelRasterization,
    billboard: LabelBillboard,
    color: Color,
    size: f32,
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

    pub const fn size(&self) -> f32 {
        self.size
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn with_size(mut self, size: f32) -> Self {
        self.size = positive_or(size, 1.0);
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
            size: 1.0,
        }
    }
}

fn positive_or(value: f32, fallback: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        fallback
    } else {
        value
    }
}
