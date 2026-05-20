//! Material descriptors, texture slots, color space, alpha modes, and technical materials.

use crate::assets::TextureHandle;

mod color;
mod presets;

pub use color::{Color, ColorParseError};

pub const DEFAULT_STROKE_WIDTH_PX: f32 = 1.0;
pub const DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES: f32 = 30.0;

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
/// Discriminant for [`MaterialDesc`]; selects the shading model and which metadata fields apply.
pub enum MaterialKind {
    /// Unlit color material for flat UI, labels, helper meshes, and simple preview surfaces.
    Unlit,
    /// Physically based metallic-roughness material for lit mesh surfaces.
    PbrMetallicRoughness,
    /// Screen-space stroke material for line-topology geometry and polylines.
    Line,
    /// Screen-space stroke material that renders triangle mesh edges as a wire overlay.
    Wireframe,
    /// Screen-space stroke material for extracted triangle-pair boundaries above an angle threshold.
    Edge,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlphaMode {
    Opaque,
    Mask { cutoff: f32 },
    Blend,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialDesc {
    kind: MaterialKind,
    base_color: Color,
    base_color_texture: Option<TextureHandle>,
    normal_texture: Option<TextureHandle>,
    metallic_roughness_texture: Option<TextureHandle>,
    occlusion_texture: Option<TextureHandle>,
    emissive_texture: Option<TextureHandle>,
    alpha_mode: AlphaMode,
    emissive: Color,
    emissive_strength: f32,
    metallic_factor: f32,
    roughness_factor: f32,
    clearcoat_factor: f32,
    clearcoat_roughness_factor: f32,
    /// glTF spec `normalTexture.scale` — scales tangent-space normal
    /// X/Y components before TBN reconstruction. Default 1.0.
    normal_scale: f32,
    /// glTF spec `occlusionTexture.strength` — `lerp(1, occlusion, strength)`.
    /// Default 1.0.
    occlusion_strength: f32,
    double_sided: bool,
    base_color_texture_transform: Option<TextureTransform>,
    normal_texture_transform: Option<TextureTransform>,
    metallic_roughness_texture_transform: Option<TextureTransform>,
    occlusion_texture_transform: Option<TextureTransform>,
    emissive_texture_transform: Option<TextureTransform>,
    stroke_width_px: Option<f32>,
    edge_angle_threshold_degrees: Option<f32>,
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

impl MaterialDesc {
    pub const fn unlit(base_color: Color) -> Self {
        Self {
            kind: MaterialKind::Unlit,
            base_color,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            clearcoat_factor: 0.0,
            clearcoat_roughness_factor: 0.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            double_sided: false,
            base_color_texture_transform: None,
            normal_texture_transform: None,
            metallic_roughness_texture_transform: None,
            occlusion_texture_transform: None,
            emissive_texture_transform: None,
            stroke_width_px: None,
            edge_angle_threshold_degrees: None,
        }
    }

    pub const fn pbr_metallic_roughness(
        base_color: Color,
        metallic_factor: f32,
        roughness_factor: f32,
    ) -> Self {
        Self {
            kind: MaterialKind::PbrMetallicRoughness,
            base_color,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor: clamp_unit_or(metallic_factor, 0.0),
            roughness_factor: clamp_unit_or(roughness_factor, 1.0),
            clearcoat_factor: 0.0,
            clearcoat_roughness_factor: 0.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            double_sided: false,
            base_color_texture_transform: None,
            normal_texture_transform: None,
            metallic_roughness_texture_transform: None,
            occlusion_texture_transform: None,
            emissive_texture_transform: None,
            stroke_width_px: None,
            edge_angle_threshold_degrees: None,
        }
    }

    /// Creates a screen-space stroke material for line-topology geometry and polylines.
    pub const fn line(base_color: Color, width_px: f32) -> Self {
        Self::technical(MaterialKind::Line, base_color, width_px, None)
    }

    /// Creates a screen-space stroke material that renders triangle mesh edges.
    pub const fn wireframe(base_color: Color, width_px: f32) -> Self {
        Self::technical(MaterialKind::Wireframe, base_color, width_px, None)
    }

    /// Creates a screen-space stroke material for extracted mesh edges.
    ///
    /// The default edge threshold is 30 degrees. Use
    /// [`with_edge_angle_threshold_degrees`](Self::with_edge_angle_threshold_degrees) to
    /// override it.
    pub const fn edge(base_color: Color, width_px: f32) -> Self {
        Self::technical(
            MaterialKind::Edge,
            base_color,
            width_px,
            Some(DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES),
        )
    }

    const fn technical(
        kind: MaterialKind,
        color: Color,
        width_px: f32,
        edge_angle_threshold_degrees: Option<f32>,
    ) -> Self {
        // Keep the three technical constructors aligned until render-path-specific fields split.
        Self {
            kind,
            base_color: color,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor: 0.0,
            roughness_factor: 1.0,
            clearcoat_factor: 0.0,
            clearcoat_roughness_factor: 0.0,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            double_sided: false,
            base_color_texture_transform: None,
            normal_texture_transform: None,
            metallic_roughness_texture_transform: None,
            occlusion_texture_transform: None,
            emissive_texture_transform: None,
            stroke_width_px: Some(positive_or(width_px, DEFAULT_STROKE_WIDTH_PX)),
            edge_angle_threshold_degrees,
        }
    }

    pub const fn kind(&self) -> MaterialKind {
        self.kind
    }

    pub const fn base_color(&self) -> Color {
        self.base_color
    }

    pub const fn base_color_texture(&self) -> Option<TextureHandle> {
        self.base_color_texture
    }

    pub const fn base_color_texture_transform(&self) -> Option<TextureTransform> {
        self.base_color_texture_transform
    }

    pub const fn normal_texture(&self) -> Option<TextureHandle> {
        self.normal_texture
    }

    pub const fn normal_texture_transform(&self) -> Option<TextureTransform> {
        self.normal_texture_transform
    }

    pub const fn metallic_roughness_texture(&self) -> Option<TextureHandle> {
        self.metallic_roughness_texture
    }

    pub const fn metallic_roughness_texture_transform(&self) -> Option<TextureTransform> {
        self.metallic_roughness_texture_transform
    }

    pub const fn occlusion_texture(&self) -> Option<TextureHandle> {
        self.occlusion_texture
    }

    pub const fn occlusion_texture_transform(&self) -> Option<TextureTransform> {
        self.occlusion_texture_transform
    }

    pub const fn emissive_texture(&self) -> Option<TextureHandle> {
        self.emissive_texture
    }

    pub const fn emissive_texture_transform(&self) -> Option<TextureTransform> {
        self.emissive_texture_transform
    }

    pub const fn alpha_mode(&self) -> AlphaMode {
        self.alpha_mode
    }

    pub const fn emissive(&self) -> Color {
        self.emissive
    }

    pub const fn emissive_strength(&self) -> f32 {
        self.emissive_strength
    }

    pub const fn metallic_factor(&self) -> f32 {
        self.metallic_factor
    }

    pub const fn roughness_factor(&self) -> f32 {
        self.roughness_factor
    }

    /// Returns the untextured `KHR_materials_clearcoat.clearcoatFactor`.
    ///
    /// A value of `0.0` disables the clearcoat layer. This first clearcoat
    /// baseline supports scalar factors on the CPU/reference path; clearcoat
    /// texture slots remain a separate glTF support lane.
    pub const fn clearcoat_factor(&self) -> f32 {
        self.clearcoat_factor
    }

    /// Returns the untextured `KHR_materials_clearcoat.clearcoatRoughnessFactor`.
    pub const fn clearcoat_roughness_factor(&self) -> f32 {
        self.clearcoat_roughness_factor
    }

    pub const fn double_sided(&self) -> bool {
        self.double_sided
    }

    /// Returns the screen-space stroke width in physical pixels for line, wireframe, and
    /// edge materials. Returns `None` for non-stroke materials.
    pub const fn stroke_width_px(&self) -> Option<f32> {
        self.stroke_width_px
    }

    /// Returns the edge dihedral-angle threshold in degrees for edge materials.
    ///
    /// `0.0` means nearly every triangle pair can become an edge; `180.0` means only
    /// explicit boundaries remain. Returns `None` for non-edge materials.
    pub const fn edge_angle_threshold_degrees(&self) -> Option<f32> {
        self.edge_angle_threshold_degrees
    }

    /// Updates the screen-space stroke width for line, wireframe, and edge materials.
    ///
    /// Invalid values fall back to [`DEFAULT_STROKE_WIDTH_PX`]. This has no effect on
    /// non-stroke materials.
    pub const fn with_stroke_width_px(mut self, width_px: f32) -> Self {
        if matches!(
            self.kind,
            MaterialKind::Line | MaterialKind::Wireframe | MaterialKind::Edge
        ) {
            self.stroke_width_px = Some(positive_or(width_px, DEFAULT_STROKE_WIDTH_PX));
        }
        self
    }

    /// Updates the edge dihedral-angle threshold in degrees.
    ///
    /// Values clamp to `[0.0, 180.0]`; non-finite values fall back to
    /// [`DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES`], replacing any previous value. This has no
    /// effect on non-edge materials.
    pub const fn with_edge_angle_threshold_degrees(mut self, angle_threshold_degrees: f32) -> Self {
        if matches!(self.kind, MaterialKind::Edge) {
            self.edge_angle_threshold_degrees = Some(clamp_degrees_or(
                angle_threshold_degrees,
                DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES,
            ));
        }
        self
    }

    pub const fn with_base_color_texture(mut self, texture: TextureHandle) -> Self {
        self.base_color_texture = Some(texture);
        self
    }

    pub const fn with_base_color_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.base_color_texture_transform = Some(transform);
        self
    }

    pub const fn with_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.normal_texture = Some(texture);
        self
    }

    pub const fn with_normal_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.normal_texture_transform = Some(transform);
        self
    }

    /// Sets `normalTexture.scale` (glTF spec): tangent-space normal X/Y
    /// components are multiplied by this factor before TBN
    /// reconstruction. Default 1.0.
    pub const fn with_normal_scale(mut self, scale: f32) -> Self {
        self.normal_scale = scale;
        self
    }

    /// Sets `occlusionTexture.strength` (glTF spec): the AO factor
    /// applied is `lerp(1.0, occlusion_sample, strength)`. Default 1.0
    /// applies the full AO; 0.0 disables it.
    pub const fn with_occlusion_strength(mut self, strength: f32) -> Self {
        self.occlusion_strength = strength;
        self
    }

    pub const fn normal_scale(&self) -> f32 {
        self.normal_scale
    }

    pub const fn occlusion_strength(&self) -> f32 {
        self.occlusion_strength
    }

    pub const fn with_metallic_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.metallic_roughness_texture = Some(texture);
        self
    }

    pub const fn with_metallic_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.metallic_roughness_texture_transform = Some(transform);
        self
    }

    pub const fn with_occlusion_texture(mut self, texture: TextureHandle) -> Self {
        self.occlusion_texture = Some(texture);
        self
    }

    pub const fn with_occlusion_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.occlusion_texture_transform = Some(transform);
        self
    }

    pub const fn with_emissive_texture(mut self, texture: TextureHandle) -> Self {
        self.emissive_texture = Some(texture);
        self
    }

    pub const fn with_emissive_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.emissive_texture_transform = Some(transform);
        self
    }

    pub const fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.alpha_mode = sanitize_alpha_mode(alpha_mode);
        self
    }

    pub const fn with_emissive(mut self, emissive: Color) -> Self {
        self.emissive = emissive;
        self
    }

    pub const fn with_emissive_strength(mut self, emissive_strength: f32) -> Self {
        self.emissive_strength = non_negative_or(emissive_strength, 1.0);
        self
    }

    /// Sets the scalar clearcoat layer strength from `KHR_materials_clearcoat`.
    ///
    /// Values are clamped to `[0, 1]`; `NaN` falls back to `0`.
    pub const fn with_clearcoat_factor(mut self, clearcoat_factor: f32) -> Self {
        self.clearcoat_factor = clamp_unit_or(clearcoat_factor, 0.0);
        self
    }

    /// Sets the scalar clearcoat roughness from `KHR_materials_clearcoat`.
    ///
    /// Values are clamped to `[0, 1]`; `NaN` falls back to `0`.
    pub const fn with_clearcoat_roughness_factor(
        mut self,
        clearcoat_roughness_factor: f32,
    ) -> Self {
        self.clearcoat_roughness_factor = clamp_unit_or(clearcoat_roughness_factor, 0.0);
        self
    }

    pub const fn with_double_sided(mut self, double_sided: bool) -> Self {
        self.double_sided = double_sided;
        self
    }
}

impl Default for MaterialDesc {
    fn default() -> Self {
        Self::pbr_metallic_roughness(Color::WHITE, 0.0, 1.0)
    }
}

const fn sanitize_alpha_mode(alpha_mode: AlphaMode) -> AlphaMode {
    match alpha_mode {
        AlphaMode::Opaque => AlphaMode::Opaque,
        AlphaMode::Mask { cutoff } => AlphaMode::Mask {
            cutoff: clamp_unit_or(cutoff, 0.5),
        },
        AlphaMode::Blend => AlphaMode::Blend,
    }
}

const fn clamp_unit_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else if value > 1.0 {
        1.0
    } else {
        value
    }
}

const fn non_negative_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

const fn positive_or(value: f32, fallback: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        fallback
    } else {
        value
    }
}

const fn clamp_degrees_or(value: f32, fallback: f32) -> f32 {
    if !value.is_finite() {
        fallback
    } else if value < 0.0 {
        0.0
    } else if value > 180.0 {
        180.0
    } else {
        value
    }
}
