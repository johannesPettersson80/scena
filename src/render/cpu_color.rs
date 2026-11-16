//! Per-pixel surface colour shared by the CPU raster loops.

use crate::material::Color;

use super::cpu_geometry::CpuScreenVertex;

/// Interpolates the triangle's vertex colours with barycentric `weights`.
pub(super) fn mix_color(vertices: [CpuScreenVertex; 3], weights: [f32; 3]) -> Color {
    let [a, b, c] = vertices.map(|vertex| vertex.color);
    let [w0, w1, w2] = weights;
    Color::from_linear_rgba(
        a.r * w0 + b.r * w1 + c.r * w2,
        a.g * w0 + b.g * w1 + c.g * w2,
        a.b * w0 + b.b * w1 + c.b * w2,
        a.a * w0 + b.a * w1 + c.a * w2,
    )
}

pub(super) fn multiply_color(color: Color, tint: Color) -> Color {
    Color::from_linear_rgba(
        color.r * tint.r,
        color.g * tint.g,
        color.b * tint.b,
        color.a * tint.a,
    )
}

/// Resolves the glTF alpha mode of a surface drawn in the opaque pass.
///
/// A masked surface (`alpha_cutoff` set) is cut out where its interpolated
/// alpha falls below the cutoff and is opaque everywhere else, so only a
/// ghosting tint (`tint_alpha`) can keep it see-through. Returns `None` for a
/// cut-out pixel. Surfaces without a cutoff keep their colour unchanged.
pub(super) fn opaque_pass_color(
    color: Color,
    alpha_cutoff: Option<f32>,
    tint_alpha: f32,
) -> Option<Color> {
    match alpha_cutoff {
        Some(cutoff) if color.a < cutoff => None,
        Some(_) => Some(Color::from_linear_rgba(
            color.r, color.g, color.b, tint_alpha,
        )),
        None => Some(color),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masked_surface_is_cut_out_below_the_cutoff_and_opaque_above_it() {
        let texel = |alpha| Color::from_linear_rgba(0.2, 0.4, 0.6, alpha);

        assert_eq!(opaque_pass_color(texel(0.49), Some(0.5), 1.0), None);
        // Equal to the cutoff is kept: glTF discards only strictly below it.
        assert_eq!(
            opaque_pass_color(texel(0.5), Some(0.5), 1.0),
            Some(texel(1.0))
        );
        assert_eq!(
            opaque_pass_color(texel(0.9), Some(0.5), 1.0),
            Some(texel(1.0)),
            "a masked texel above the cutoff must not blend with what is behind it"
        );
    }

    #[test]
    fn ghosting_tint_keeps_a_masked_surface_see_through() {
        let color = Color::from_linear_rgba(0.2, 0.4, 0.6, 0.3);
        assert_eq!(
            opaque_pass_color(color, Some(0.25), 0.3),
            Some(Color::from_linear_rgba(0.2, 0.4, 0.6, 0.3))
        );
    }

    #[test]
    fn surface_without_a_cutoff_keeps_its_alpha() {
        let color = Color::from_linear_rgba(0.2, 0.4, 0.6, 0.35);
        assert_eq!(opaque_pass_color(color, None, 1.0), Some(color));
    }
}
