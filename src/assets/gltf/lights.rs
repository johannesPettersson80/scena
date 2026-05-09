use serde_json::Value as JsonValue;

use crate::material::Color;
use crate::scene::{Angle, DirectionalLight, Light, PointLight, SpotLight};

use super::SceneAssetLight;

pub(super) fn parse_punctual_lights(json: &JsonValue) -> Vec<SceneAssetLight> {
    json.get("extensions")
        .and_then(|extensions| extensions.get("KHR_lights_punctual"))
        .and_then(|extension| extension.get("lights"))
        .and_then(JsonValue::as_array)
        .map(|lights| lights.iter().filter_map(parse_punctual_light).collect())
        .unwrap_or_default()
}

fn parse_punctual_light(light: &JsonValue) -> Option<SceneAssetLight> {
    let color = color3_field(light, "color", Color::WHITE);
    let intensity = number_field(light, "intensity").unwrap_or(1.0);
    let range = number_field(light, "range");
    let light = match light.get("type").and_then(JsonValue::as_str)? {
        "directional" => Light::Directional(
            DirectionalLight::default()
                .with_color(color)
                .with_illuminance_lux(intensity),
        ),
        "point" => {
            let mut point = PointLight::default()
                .with_color(color)
                .with_intensity_candela(intensity);
            if let Some(range) = range {
                point = point.with_range(range);
            }
            Light::Point(point)
        }
        "spot" => {
            let spot_json = light.get("spot").unwrap_or(&JsonValue::Null);
            let mut spot = SpotLight::default()
                .with_color(color)
                .with_intensity_candela(intensity)
                .with_inner_cone_angle(Angle::from_radians(
                    number_field(spot_json, "innerConeAngle").unwrap_or(0.0),
                ))
                .with_outer_cone_angle(Angle::from_radians(
                    number_field(spot_json, "outerConeAngle")
                        .unwrap_or(std::f32::consts::FRAC_PI_4),
                ));
            if let Some(range) = range {
                spot = spot.with_range(range);
            }
            Light::Spot(spot)
        }
        _ => return None,
    };
    Some(SceneAssetLight { light })
}

fn number_field(value: &JsonValue, field: &str) -> Option<f32> {
    value
        .get(field)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}

fn color3_field(value: &JsonValue, field: &str, fallback: Color) -> Color {
    let Some(values) = value.get(field).and_then(JsonValue::as_array) else {
        return fallback;
    };
    Color::from_linear_rgb(
        array_f32(values, 0).unwrap_or(fallback.r),
        array_f32(values, 1).unwrap_or(fallback.g),
        array_f32(values, 2).unwrap_or(fallback.b),
    )
}

fn array_f32(values: &[JsonValue], index: usize) -> Option<f32> {
    values
        .get(index)
        .and_then(JsonValue::as_f64)
        .map(|value| value as f32)
}
