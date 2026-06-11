#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextureColorSpace {
    Linear,
    Srgb,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureTransform {
    offset: [f32; 2],
    rotation_radians: f32,
    scale: [f32; 2],
    tex_coord: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Discriminant for [`crate::material::MaterialDesc`]; selects the shading model and which metadata fields apply.
pub enum MaterialKind {
    /// Unlit color material for flat UI, labels, helper meshes, and simple preview surfaces.
    Unlit,
    /// Physically based metallic-roughness material for lit mesh surfaces.
    PbrMetallicRoughness,
    /// World-space stroke material for line-topology geometry and polylines.
    Line,
    /// World-space stroke material that renders triangle mesh edges as a wire overlay.
    Wireframe,
    /// World-space stroke material for extracted triangle-pair boundaries above an angle threshold.
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}

impl TextureTransform {
    pub const fn new(
        offset: [f32; 2],
        rotation_radians: f32,
        scale: [f32; 2],
        tex_coord: Option<u32>,
    ) -> Self {
        Self {
            offset,
            rotation_radians,
            scale,
            tex_coord,
        }
    }

    pub const fn offset(self) -> [f32; 2] {
        self.offset
    }

    pub const fn rotation_radians(self) -> f32 {
        self.rotation_radians
    }

    pub const fn scale(self) -> [f32; 2] {
        self.scale
    }

    pub const fn tex_coord(self) -> Option<u32> {
        self.tex_coord
    }
}
