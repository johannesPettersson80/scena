use serde::de;

const COLOR_TARGET_FORMATS: &[&str] = &[
    "Rgba8Unorm",
    "Rgba8UnormSrgb",
    "Bgra8Unorm",
    "Bgra8UnormSrgb",
    "Rgba8UnormSrgb+DisplayP3Canvas",
];

pub(super) fn static_color_target_format<E>(value: &str) -> Result<&'static str, E>
where
    E: de::Error,
{
    match value {
        "Rgba8Unorm" => Ok("Rgba8Unorm"),
        "Rgba8UnormSrgb" => Ok("Rgba8UnormSrgb"),
        "Bgra8Unorm" => Ok("Bgra8Unorm"),
        "Bgra8UnormSrgb" => Ok("Bgra8UnormSrgb"),
        "Rgba8UnormSrgb+DisplayP3Canvas" => Ok("Rgba8UnormSrgb+DisplayP3Canvas"),
        unknown => Err(de::Error::unknown_variant(unknown, COLOR_TARGET_FORMATS)),
    }
}
