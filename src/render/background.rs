use crate::material::Color;

/// Named background schemes for first-path viewer and product-render scenes.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum Background {
    Studio,
    DarkStudio,
    NeutralGray,
    White,
    Black,
    Sky,
    Transparent,
    Custom(Color),
}

impl Background {
    /// Returns the linear color used to clear the render target.
    ///
    /// # Examples
    ///
    /// ```
    /// use scena::{Background, Color};
    ///
    /// assert_eq!(Background::DarkStudio.color(), Color::CHARCOAL);
    /// ```
    pub const fn color(self) -> Color {
        match self {
            Self::Studio => Color::STUDIO_BACKDROP,
            Self::DarkStudio => Color::CHARCOAL,
            Self::NeutralGray => Color::GRAY,
            Self::White => Color::WHITE,
            Self::Black => Color::BLACK,
            Self::Sky => Color::from_linear_rgba(0.242_281_11, 0.617_206_63, 0.830_77, 1.0),
            Self::Transparent => Color::from_linear_rgba(0.0, 0.0, 0.0, 0.0),
            Self::Custom(color) => color,
        }
    }
}
