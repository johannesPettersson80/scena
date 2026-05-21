use super::{AlphaMode, Color, MaterialDesc};

impl MaterialDesc {
    /// Matte dielectric material preset: high roughness, non-metallic.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::matte(Color::BLUE);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// assert!(material.roughness_factor() > 0.9);
    /// ```
    pub const fn matte(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 0.0, 0.92)
    }

    /// Smooth dielectric plastic preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::plastic(Color::ORANGE);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// assert!(material.roughness_factor() < MaterialDesc::matte(Color::ORANGE).roughness_factor());
    /// ```
    pub const fn plastic(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 0.0, 0.42)
    }

    /// Metallic-roughness metal preset for honest bare-metal behavior.
    ///
    /// This is intentionally not named chrome or brushed steel; those require
    /// renderer features that this preset does not claim.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::metal(Color::LIGHT_GRAY);
    /// assert_eq!(material.metallic_factor(), 1.0);
    /// ```
    pub const fn metal(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 1.0, 0.28)
    }

    /// Rough bare-metal preset for less mirror-like product surfaces.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::rough_metal(Color::LIGHT_GRAY);
    /// assert_eq!(material.metallic_factor(), 1.0);
    /// assert!(material.roughness_factor() > MaterialDesc::metal(Color::LIGHT_GRAY).roughness_factor());
    /// ```
    pub const fn rough_metal(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 1.0, 0.62)
    }

    /// Smooth chrome preset backed by metallic roughness and environment reflection quality.
    ///
    /// This preset does not claim screen-space reflected floors or caustics.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::MaterialDesc;
    ///
    /// let material = MaterialDesc::chrome();
    /// assert_eq!(material.metallic_factor(), 1.0);
    /// assert!(material.roughness_factor() < 0.1);
    /// ```
    pub const fn chrome() -> Self {
        Self::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.04)
    }

    /// Brushed steel preset backed by metallic roughness plus scalar anisotropy.
    ///
    /// This is a texture-free brushed-metal shortcut. Use anisotropy textures
    /// for authored brush direction maps.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::MaterialDesc;
    ///
    /// let material = MaterialDesc::brushed_steel();
    /// assert_eq!(material.metallic_factor(), 1.0);
    /// assert!(material.anisotropy_strength_factor() > 0.5);
    /// ```
    pub const fn brushed_steel() -> Self {
        Self::pbr_metallic_roughness(Color::LIGHT_GRAY, 1.0, 0.36)
            .with_anisotropy_strength_factor(0.72)
    }

    /// Glossy coated plastic preset backed by `KHR_materials_clearcoat` factors.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::clearcoat_plastic(Color::BLUE);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// assert!(material.clearcoat_factor() > 0.0);
    /// ```
    pub const fn clearcoat_plastic(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 0.0, 0.32)
            .with_clearcoat_factor(0.9)
            .with_clearcoat_roughness_factor(0.08)
    }

    /// Satin fabric-like preset backed by the sheen material lane.
    ///
    /// This is a smooth sheen shortcut, not a procedural weave texture.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::satin(Color::MAGENTA);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// assert!(material.sheen_roughness_factor() > 0.0);
    /// ```
    pub const fn satin(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 0.0, 0.68)
            .with_sheen_color_factor(Color::WHITE)
            .with_sheen_roughness_factor(0.48)
    }

    /// Smooth leather-like preset backed by rough dielectric shading plus sheen.
    ///
    /// This preset does not synthesize leather grain or normal-map detail.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::leather(Color::ORANGE);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// assert!(material.sheen_roughness_factor() > 0.0);
    /// ```
    pub const fn leather(base_color: Color) -> Self {
        Self::pbr_metallic_roughness(base_color, 0.0, 0.78)
            .with_sheen_color_factor(base_color)
            .with_sheen_roughness_factor(0.72)
    }

    /// Transparent glass preset backed by blend mode plus transmission, IOR, and volume metadata.
    ///
    /// This is a browser-preview glass material, not a full refraction or
    /// caustics model.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{AlphaMode, Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::clear_glass(Color::CYAN);
    /// assert_eq!(material.alpha_mode(), AlphaMode::Blend);
    /// assert_eq!(material.transmission_factor(), 1.0);
    /// ```
    pub const fn clear_glass(tint: Color) -> Self {
        let base_color = Color::from_linear_rgba(tint.r, tint.g, tint.b, 0.28);
        Self::pbr_metallic_roughness(base_color, 0.0, 0.02)
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true)
            .with_transmission_factor(1.0)
            .with_ior(1.45)
            .with_thickness_factor(0.02)
            .with_attenuation_distance(2.0)
            .with_attenuation_color(tint)
    }

    /// Frosted glass preset backed by blend mode plus rough transmission, IOR, and volume metadata.
    ///
    /// This is a translucent material shortcut, not a physically complete
    /// rough refraction model.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{AlphaMode, Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::frosted_glass(Color::COOL_WHITE);
    /// assert_eq!(material.alpha_mode(), AlphaMode::Blend);
    /// assert!(material.roughness_factor() > MaterialDesc::clear_glass(Color::COOL_WHITE).roughness_factor());
    /// ```
    pub const fn frosted_glass(tint: Color) -> Self {
        let base_color = Color::from_linear_rgba(tint.r, tint.g, tint.b, 0.42);
        Self::pbr_metallic_roughness(base_color, 0.0, 0.62)
            .with_alpha_mode(AlphaMode::Blend)
            .with_double_sided(true)
            .with_transmission_factor(0.72)
            .with_ior(1.45)
            .with_thickness_factor(0.08)
            .with_attenuation_distance(1.25)
            .with_attenuation_color(tint)
    }

    /// Dark rough rubber preset.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Color, MaterialDesc};
    ///
    /// let material = MaterialDesc::rubber();
    /// assert_eq!(material.base_color(), Color::CHARCOAL);
    /// assert_eq!(material.metallic_factor(), 0.0);
    /// ```
    pub const fn rubber() -> Self {
        Self::pbr_metallic_roughness(Color::CHARCOAL, 0.0, 0.86)
    }
}
