use super::AlphaMode;

pub(super) const fn sanitize_alpha_mode(alpha_mode: AlphaMode) -> AlphaMode {
    match alpha_mode {
        AlphaMode::Opaque => AlphaMode::Opaque,
        AlphaMode::Mask { cutoff } => AlphaMode::Mask {
            cutoff: clamp_unit_or(cutoff, 0.5),
        },
        AlphaMode::Blend => AlphaMode::Blend,
    }
}

pub(super) const fn clamp_unit_or(value: f32, fallback: f32) -> f32 {
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

pub(super) const fn non_negative_or(value: f32, fallback: f32) -> f32 {
    if value.is_nan() {
        fallback
    } else if value < 0.0 {
        0.0
    } else {
        value
    }
}

pub(super) const fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

pub(super) const fn positive_or(value: f32, fallback: f32) -> f32 {
    if !value.is_finite() || value <= 0.0 {
        fallback
    } else {
        value
    }
}

pub(super) const fn clamp_degrees_or(value: f32, fallback: f32) -> f32 {
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
