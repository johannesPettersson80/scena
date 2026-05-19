use super::{Color, MaterialDesc};

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
