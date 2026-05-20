use crate::assets::TextureHandle;

use super::scalars::{clamp_unit_or, finite_or, non_negative_or, positive_or};
use super::{Color, MaterialDesc, TextureTransform};

impl MaterialDesc {
    pub const fn clearcoat_texture(&self) -> Option<TextureHandle> {
        self.clearcoat_texture
    }

    pub const fn clearcoat_texture_transform(&self) -> Option<TextureTransform> {
        self.clearcoat_texture_transform
    }

    pub const fn clearcoat_roughness_texture(&self) -> Option<TextureHandle> {
        self.clearcoat_roughness_texture
    }

    pub const fn clearcoat_roughness_texture_transform(&self) -> Option<TextureTransform> {
        self.clearcoat_roughness_texture_transform
    }

    pub const fn clearcoat_normal_texture(&self) -> Option<TextureHandle> {
        self.clearcoat_normal_texture
    }

    pub const fn clearcoat_normal_texture_transform(&self) -> Option<TextureTransform> {
        self.clearcoat_normal_texture_transform
    }

    pub const fn sheen_color_texture(&self) -> Option<TextureHandle> {
        self.sheen_color_texture
    }

    pub const fn sheen_color_texture_transform(&self) -> Option<TextureTransform> {
        self.sheen_color_texture_transform
    }

    pub const fn sheen_roughness_texture(&self) -> Option<TextureHandle> {
        self.sheen_roughness_texture
    }

    pub const fn sheen_roughness_texture_transform(&self) -> Option<TextureTransform> {
        self.sheen_roughness_texture_transform
    }

    pub const fn anisotropy_texture(&self) -> Option<TextureHandle> {
        self.anisotropy_texture
    }

    pub const fn anisotropy_texture_transform(&self) -> Option<TextureTransform> {
        self.anisotropy_texture_transform
    }

    pub const fn iridescence_texture(&self) -> Option<TextureHandle> {
        self.iridescence_texture
    }

    pub const fn iridescence_texture_transform(&self) -> Option<TextureTransform> {
        self.iridescence_texture_transform
    }

    pub const fn iridescence_thickness_texture(&self) -> Option<TextureHandle> {
        self.iridescence_thickness_texture
    }

    pub const fn iridescence_thickness_texture_transform(&self) -> Option<TextureTransform> {
        self.iridescence_thickness_texture_transform
    }

    /// Returns the scalar `KHR_materials_clearcoat.clearcoatFactor`.
    ///
    /// The renderer multiplies this value by the sampled clearcoat texture
    /// when one is present.
    pub const fn clearcoat_factor(&self) -> f32 {
        self.clearcoat_factor
    }

    /// Returns the untextured `KHR_materials_clearcoat.clearcoatRoughnessFactor`.
    pub const fn clearcoat_roughness_factor(&self) -> f32 {
        self.clearcoat_roughness_factor
    }

    pub const fn clearcoat_normal_scale(&self) -> f32 {
        self.clearcoat_normal_scale
    }

    /// Returns the scalar `KHR_materials_sheen.sheenColorFactor`.
    pub const fn sheen_color_factor(&self) -> Color {
        self.sheen_color_factor
    }

    /// Returns the scalar `KHR_materials_sheen.sheenRoughnessFactor`.
    pub const fn sheen_roughness_factor(&self) -> f32 {
        self.sheen_roughness_factor
    }

    /// Returns `KHR_materials_anisotropy.anisotropyStrength`.
    pub const fn anisotropy_strength_factor(&self) -> f32 {
        self.anisotropy_strength_factor
    }

    /// Returns `KHR_materials_anisotropy.anisotropyRotation` in radians.
    pub const fn anisotropy_rotation_radians(&self) -> f32 {
        self.anisotropy_rotation_radians
    }

    /// Returns `KHR_materials_iridescence.iridescenceFactor`.
    pub const fn iridescence_factor(&self) -> f32 {
        self.iridescence_factor
    }

    /// Returns `KHR_materials_iridescence.iridescenceIor`.
    pub const fn iridescence_ior(&self) -> f32 {
        self.iridescence_ior
    }

    /// Returns `KHR_materials_iridescence.iridescenceThicknessMinimum`, in nanometers.
    pub const fn iridescence_thickness_minimum_nm(&self) -> f32 {
        self.iridescence_thickness_minimum_nm
    }

    /// Returns `KHR_materials_iridescence.iridescenceThicknessMaximum`, in nanometers.
    pub const fn iridescence_thickness_maximum_nm(&self) -> f32 {
        self.iridescence_thickness_maximum_nm
    }

    /// Returns `KHR_materials_dispersion.dispersion` as `20 / Abbe number`.
    pub const fn dispersion_factor(&self) -> f32 {
        self.dispersion_factor
    }

    pub const fn with_clearcoat_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_texture = Some(texture);
        self
    }

    pub const fn with_clearcoat_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.clearcoat_texture_transform = Some(transform);
        self
    }

    pub const fn with_clearcoat_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_roughness_texture = Some(texture);
        self
    }

    pub const fn with_clearcoat_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.clearcoat_roughness_texture_transform = Some(transform);
        self
    }

    pub const fn with_clearcoat_normal_texture(mut self, texture: TextureHandle) -> Self {
        self.clearcoat_normal_texture = Some(texture);
        self
    }

    pub const fn with_clearcoat_normal_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.clearcoat_normal_texture_transform = Some(transform);
        self
    }

    pub const fn with_clearcoat_normal_scale(mut self, scale: f32) -> Self {
        self.clearcoat_normal_scale = non_negative_or(scale, 1.0);
        self
    }

    pub const fn with_sheen_color_texture(mut self, texture: TextureHandle) -> Self {
        self.sheen_color_texture = Some(texture);
        self
    }

    pub const fn with_sheen_color_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.sheen_color_texture_transform = Some(transform);
        self
    }

    pub const fn with_sheen_roughness_texture(mut self, texture: TextureHandle) -> Self {
        self.sheen_roughness_texture = Some(texture);
        self
    }

    pub const fn with_sheen_roughness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.sheen_roughness_texture_transform = Some(transform);
        self
    }

    pub const fn with_anisotropy_texture(mut self, texture: TextureHandle) -> Self {
        self.anisotropy_texture = Some(texture);
        self
    }

    pub const fn with_anisotropy_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.anisotropy_texture_transform = Some(transform);
        self
    }

    pub const fn with_iridescence_texture(mut self, texture: TextureHandle) -> Self {
        self.iridescence_texture = Some(texture);
        self
    }

    pub const fn with_iridescence_texture_transform(mut self, transform: TextureTransform) -> Self {
        self.iridescence_texture_transform = Some(transform);
        self
    }

    pub const fn with_iridescence_thickness_texture(mut self, texture: TextureHandle) -> Self {
        self.iridescence_thickness_texture = Some(texture);
        self
    }

    pub const fn with_iridescence_thickness_texture_transform(
        mut self,
        transform: TextureTransform,
    ) -> Self {
        self.iridescence_thickness_texture_transform = Some(transform);
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

    /// Sets the scalar sheen color from `KHR_materials_sheen`.
    pub const fn with_sheen_color_factor(mut self, sheen_color_factor: Color) -> Self {
        self.sheen_color_factor = Color::from_linear_rgb(
            clamp_unit_or(sheen_color_factor.r, 0.0),
            clamp_unit_or(sheen_color_factor.g, 0.0),
            clamp_unit_or(sheen_color_factor.b, 0.0),
        );
        self
    }

    /// Sets the scalar sheen roughness from `KHR_materials_sheen`.
    pub const fn with_sheen_roughness_factor(mut self, sheen_roughness_factor: f32) -> Self {
        self.sheen_roughness_factor = clamp_unit_or(sheen_roughness_factor, 0.0);
        self
    }

    /// Sets the scalar anisotropy strength from `KHR_materials_anisotropy`.
    ///
    /// Values are clamped to `[0, 1]`; `NaN` falls back to `0`.
    pub const fn with_anisotropy_strength_factor(
        mut self,
        anisotropy_strength_factor: f32,
    ) -> Self {
        self.anisotropy_strength_factor = clamp_unit_or(anisotropy_strength_factor, 0.0);
        self
    }

    /// Sets the anisotropy rotation in tangent space, in radians.
    pub const fn with_anisotropy_rotation_radians(
        mut self,
        anisotropy_rotation_radians: f32,
    ) -> Self {
        self.anisotropy_rotation_radians = finite_or(anisotropy_rotation_radians, 0.0);
        self
    }

    /// Sets the scalar iridescence strength from `KHR_materials_iridescence`.
    ///
    /// Values are clamped to `[0, 1]`; `NaN` falls back to `0`.
    pub const fn with_iridescence_factor(mut self, iridescence_factor: f32) -> Self {
        self.iridescence_factor = clamp_unit_or(iridescence_factor, 0.0);
        self
    }

    /// Sets the thin-film index of refraction from `KHR_materials_iridescence`.
    pub const fn with_iridescence_ior(mut self, iridescence_ior: f32) -> Self {
        self.iridescence_ior = positive_or(iridescence_ior, 1.3);
        self
    }

    /// Sets the thin-film thickness range in nanometers.
    ///
    /// The glTF extension permits `minimum > maximum`, so values are sanitized
    /// independently rather than reordered.
    pub const fn with_iridescence_thickness_range_nm(
        mut self,
        minimum_nm: f32,
        maximum_nm: f32,
    ) -> Self {
        self.iridescence_thickness_minimum_nm = non_negative_or(minimum_nm, 100.0);
        self.iridescence_thickness_maximum_nm = non_negative_or(maximum_nm, 400.0);
        self
    }

    /// Sets the scalar dispersion strength from `KHR_materials_dispersion`.
    ///
    /// Values are sanitized to `>= 0`; `NaN` falls back to `0`.
    pub const fn with_dispersion_factor(mut self, dispersion_factor: f32) -> Self {
        self.dispersion_factor = non_negative_or(dispersion_factor, 0.0);
        self
    }
}
