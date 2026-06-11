//! Material descriptors, texture slots, color space, alpha modes, and technical materials.

use crate::assets::TextureHandle;

mod color;
mod extensions;
mod presets;
mod scalars;
mod types;

pub use color::{Color, ColorParseError};
use scalars::{clamp_degrees_or, clamp_unit_or, non_negative_or, positive_or, sanitize_alpha_mode};
pub use types::{AlphaMode, MaterialKind, TextureColorSpace, TextureTransform};

pub const DEFAULT_STROKE_WIDTH_PX: f32 = 1.0;
pub const DEFAULT_EDGE_ANGLE_THRESHOLD_DEGREES: f32 = 30.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MaterialDesc {
    kind: MaterialKind,
    base_color: Color,
    base_color_texture: Option<TextureHandle>,
    normal_texture: Option<TextureHandle>,
    metallic_roughness_texture: Option<TextureHandle>,
    occlusion_texture: Option<TextureHandle>,
    emissive_texture: Option<TextureHandle>,
    clearcoat_texture: Option<TextureHandle>,
    clearcoat_roughness_texture: Option<TextureHandle>,
    clearcoat_normal_texture: Option<TextureHandle>,
    sheen_color_texture: Option<TextureHandle>,
    sheen_roughness_texture: Option<TextureHandle>,
    anisotropy_texture: Option<TextureHandle>,
    iridescence_texture: Option<TextureHandle>,
    iridescence_thickness_texture: Option<TextureHandle>,
    transmission_texture: Option<TextureHandle>,
    thickness_texture: Option<TextureHandle>,
    alpha_mode: AlphaMode,
    emissive: Color,
    emissive_strength: f32,
    metallic_factor: f32,
    roughness_factor: f32,
    clearcoat_factor: f32,
    clearcoat_roughness_factor: f32,
    sheen_color_factor: Color,
    sheen_roughness_factor: f32,
    anisotropy_strength_factor: f32,
    anisotropy_rotation_radians: f32,
    iridescence_factor: f32,
    iridescence_ior: f32,
    iridescence_thickness_minimum_nm: f32,
    iridescence_thickness_maximum_nm: f32,
    dispersion_factor: f32,
    transmission_factor: f32,
    ior: f32,
    thickness_factor: f32,
    attenuation_distance: f32,
    attenuation_color: Color,
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
    clearcoat_texture_transform: Option<TextureTransform>,
    clearcoat_roughness_texture_transform: Option<TextureTransform>,
    clearcoat_normal_texture_transform: Option<TextureTransform>,
    sheen_color_texture_transform: Option<TextureTransform>,
    sheen_roughness_texture_transform: Option<TextureTransform>,
    anisotropy_texture_transform: Option<TextureTransform>,
    iridescence_texture_transform: Option<TextureTransform>,
    iridescence_thickness_texture_transform: Option<TextureTransform>,
    transmission_texture_transform: Option<TextureTransform>,
    thickness_texture_transform: Option<TextureTransform>,
    clearcoat_normal_scale: f32,
    stroke_width_px: Option<f32>,
    edge_angle_threshold_degrees: Option<f32>,
}

impl MaterialDesc {
    pub const fn unlit(base_color: Color) -> Self {
        Self::base(MaterialKind::Unlit, base_color, 0.0, 1.0, None, None)
    }

    pub const fn pbr_metallic_roughness(
        base_color: Color,
        metallic_factor: f32,
        roughness_factor: f32,
    ) -> Self {
        Self::base(
            MaterialKind::PbrMetallicRoughness,
            base_color,
            clamp_unit_or(metallic_factor, 0.0),
            clamp_unit_or(roughness_factor, 1.0),
            None,
            None,
        )
    }

    /// Creates a world-space stroke material for line-topology geometry and polylines.
    ///
    /// The segment endpoints live in scene/model space; GPU backends expand them in the
    /// vertex shader to a screen-constant physical-pixel width. Strokes are depth-tested
    /// against scene geometry, clipped at the near plane, and are not pickable in v1.7.
    pub const fn line(base_color: Color, width_px: f32) -> Self {
        Self::technical(MaterialKind::Line, base_color, width_px, None)
    }

    /// Creates a world-space stroke material that renders triangle mesh edges.
    ///
    /// GPU backends retain model-space edge endpoints and expand them to
    /// screen-constant width during rendering; strokes are depth-tested, near-plane
    /// clipped, and ignored by picking in v1.7.
    pub const fn wireframe(base_color: Color, width_px: f32) -> Self {
        Self::technical(MaterialKind::Wireframe, base_color, width_px, None)
    }

    /// Creates a world-space stroke material for extracted mesh edges.
    ///
    /// The default edge threshold is 30 degrees. Use
    /// [`with_edge_angle_threshold_degrees`](Self::with_edge_angle_threshold_degrees) to
    /// override it. GPU backends retain model-space edge endpoints and expand them to
    /// screen-constant width during rendering; strokes are depth-tested, near-plane
    /// clipped, and ignored by picking in v1.7.
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
        let stroke_width_px = Some(positive_or(width_px, DEFAULT_STROKE_WIDTH_PX));
        Self::base(
            kind,
            color,
            0.0,
            1.0,
            stroke_width_px,
            edge_angle_threshold_degrees,
        )
    }

    const fn base(
        kind: MaterialKind,
        base_color: Color,
        metallic_factor: f32,
        roughness_factor: f32,
        stroke_width_px: Option<f32>,
        edge_angle_threshold_degrees: Option<f32>,
    ) -> Self {
        Self {
            kind,
            base_color,
            base_color_texture: None,
            normal_texture: None,
            metallic_roughness_texture: None,
            occlusion_texture: None,
            emissive_texture: None,
            clearcoat_texture: None,
            clearcoat_roughness_texture: None,
            clearcoat_normal_texture: None,
            sheen_color_texture: None,
            sheen_roughness_texture: None,
            anisotropy_texture: None,
            iridescence_texture: None,
            iridescence_thickness_texture: None,
            transmission_texture: None,
            thickness_texture: None,
            alpha_mode: AlphaMode::Opaque,
            emissive: Color::BLACK,
            emissive_strength: 1.0,
            metallic_factor,
            roughness_factor,
            clearcoat_factor: 0.0,
            clearcoat_roughness_factor: 0.0,
            sheen_color_factor: Color::BLACK,
            sheen_roughness_factor: 0.0,
            anisotropy_strength_factor: 0.0,
            anisotropy_rotation_radians: 0.0,
            iridescence_factor: 0.0,
            iridescence_ior: 1.3,
            iridescence_thickness_minimum_nm: 100.0,
            iridescence_thickness_maximum_nm: 400.0,
            dispersion_factor: 0.0,
            transmission_factor: 0.0,
            ior: 1.5,
            thickness_factor: 0.0,
            attenuation_distance: f32::INFINITY,
            attenuation_color: Color::WHITE,
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            double_sided: false,
            base_color_texture_transform: None,
            normal_texture_transform: None,
            metallic_roughness_texture_transform: None,
            occlusion_texture_transform: None,
            emissive_texture_transform: None,
            clearcoat_texture_transform: None,
            clearcoat_roughness_texture_transform: None,
            clearcoat_normal_texture_transform: None,
            sheen_color_texture_transform: None,
            sheen_roughness_texture_transform: None,
            anisotropy_texture_transform: None,
            iridescence_texture_transform: None,
            iridescence_thickness_texture_transform: None,
            transmission_texture_transform: None,
            thickness_texture_transform: None,
            clearcoat_normal_scale: 1.0,
            stroke_width_px,
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

    pub const fn double_sided(&self) -> bool {
        self.double_sided
    }

    /// Returns the screen-constant stroke width in physical pixels for line, wireframe, and
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

    /// Updates the screen-constant stroke width for line, wireframe, and edge materials.
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
