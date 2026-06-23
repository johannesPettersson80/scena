use std::collections::BTreeMap;

use crate::scene::recipe::{SceneRecipeColorV1, SceneRecipeDiagnosticV1, SceneRecipePrimitiveV1};
use crate::{Color, Vec3};

use super::super::error_diagnostic;

pub(super) fn vec3(value: [f64; 3]) -> Vec3 {
    Vec3::new(value[0] as f32, value[1] as f32, value[2] as f32)
}

pub(super) fn primitive_size3(primitive: &SceneRecipePrimitiveV1) -> Option<[f64; 3]> {
    let size = primitive.size.as_ref()?;
    let [x, y, z] = size.as_slice() else {
        return None;
    };
    Some([*x, *y, *z])
}

pub(super) fn primitive_size2(primitive: &SceneRecipePrimitiveV1) -> Option<[f64; 2]> {
    let size = primitive.size.as_ref()?;
    let [x, y] = size.as_slice() else {
        return None;
    };
    Some([*x, *y])
}

pub(super) fn positive_f64(value: Option<f64>) -> Option<f64> {
    value.filter(|value| value.is_finite() && *value > 0.0)
}

pub(super) fn u32_at_least(value: Option<u32>, default: u32, min: u32) -> Option<u32> {
    let value = value.unwrap_or(default);
    (value >= min).then_some(value)
}

pub(in crate::scene_host::recipe) fn authored_color(
    colors: &BTreeMap<String, SceneRecipeColorV1>,
    value: &str,
) -> Result<Color, Box<SceneRecipeDiagnosticV1>> {
    if let Some(color) = colors.get(value) {
        return match color {
            SceneRecipeColorV1::Hex(hex) => parse_color_string(hex).map_err(|error| {
                Box::new(error_diagnostic(
                    "$",
                    "invalid_color",
                    format!("color '{value}' is not valid hex: {error}"),
                    "use a six-digit #RRGGBB color",
                ))
            }),
            SceneRecipeColorV1::Srgb8 { srgb8 } => {
                Ok(Color::from_srgb_u8(srgb8[0], srgb8[1], srgb8[2]))
            }
            SceneRecipeColorV1::Linear { linear } => Ok(Color::from_linear_rgb(
                linear[0] as f32,
                linear[1] as f32,
                linear[2] as f32,
            )),
            SceneRecipeColorV1::Kelvin { kelvin } => Ok(Color::from_kelvin(*kelvin as f32)),
        };
    }
    parse_color_string(value).map_err(|error| {
        Box::new(error_diagnostic(
            "$",
            "unknown_color_ref",
            format!("base_color '{value}' is not a declared color or direct hex value: {error}"),
            "reference a key from colors or use a direct #RRGGBB value",
        ))
    })
}

fn parse_color_string(value: &str) -> Result<Color, crate::ColorParseError> {
    if let Some(color) = Color::from_named_constant(value) {
        Ok(color)
    } else {
        Color::from_hex(value)
    }
}

pub(in crate::scene_host::recipe) trait DiagnosticPathExt {
    fn with_path(self, path: String) -> Self;
}

impl DiagnosticPathExt for SceneRecipeDiagnosticV1 {
    fn with_path(mut self, path: String) -> Self {
        self.path = path;
        self
    }
}
