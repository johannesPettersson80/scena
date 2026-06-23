//! Linearly transformed cosine area-light evaluation.
//!
//! The fitted table layout and polygon integration follow the public
//! `selfshadow/ltc_code` WebGL reference for "Real-Time Polygonal-Light
//! Shading with Linearly Transformed Cosines".  The same table convention is
//! mirrored in `area_ltc.wgsl` for the GPU path.

use crate::scene::Vec3;

#[path = "area_ltc_tables.rs"]
mod area_ltc_tables;

const LUT_LAST: f32 = (area_ltc_tables::LTC_LUT_SIZE - 1) as f32;
const MIN_DENOMINATOR: f32 = 0.0001;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct LtcSpecularProbe {
    pub(crate) irradiance: f32,
    pub(crate) fresnel_scale: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LtcTableSample {
    inverse_matrix: [f32; 4],
    fresnel_terms: [f32; 4],
}

pub(crate) fn evaluate_specular_polygon(
    polygon: [Vec3; 4],
    position: Vec3,
    normal: Vec3,
    view: Vec3,
    roughness: f32,
    f0: Vec3,
) -> LtcSpecularProbe {
    let normal = normalize_or(normal, Vec3::Y);
    let view = normalize_or(view, normal);
    let n_dot_v = normal.dot(view).clamp(0.0, 1.0);
    if n_dot_v <= f32::EPSILON {
        return LtcSpecularProbe {
            irradiance: 0.0,
            fresnel_scale: Vec3::ZERO,
        };
    }

    let sample = sample_ltc_tables(roughness, n_dot_v);
    let tangent = normalize_or(view - normal * view.dot(normal), fallback_tangent(normal));
    let bitangent = normalize_or(normal.cross(tangent), Vec3::Z);
    let mut transformed = [Vec3::ZERO; 4];
    for (index, point) in polygon.iter().copied().enumerate() {
        let local = point - position;
        let basis = Vec3::new(local.dot(tangent), local.dot(bitangent), local.dot(normal));
        transformed[index] = apply_inverse_ltc_matrix(sample.inverse_matrix, basis);
    }
    let integral = integrate_transformed_quad(transformed, true);
    let irradiance = integral;
    let fresnel_scale = f0 * sample.fresnel_terms[0]
        + (Vec3::new(1.0, 1.0, 1.0) - f0) * sample.fresnel_terms[1];
    LtcSpecularProbe {
        irradiance,
        fresnel_scale,
    }
}

fn sample_ltc_tables(roughness: f32, n_dot_v: f32) -> LtcTableSample {
    let roughness = roughness.clamp(0.0, 1.0);
    let view = (1.0 - n_dot_v.clamp(0.0, 1.0)).max(0.0).sqrt();
    let x = roughness * LUT_LAST;
    let y = view * LUT_LAST;
    LtcTableSample {
        inverse_matrix: bilinear_sample(area_ltc_tables::LTC_1, x, y),
        fresnel_terms: bilinear_sample(area_ltc_tables::LTC_2, x, y),
    }
}

fn bilinear_sample(table: &[[[f32; 4]; area_ltc_tables::LTC_LUT_SIZE]; area_ltc_tables::LTC_LUT_SIZE], x: f32, y: f32) -> [f32; 4] {
    let x0 = x.floor() as usize;
    let y0 = y.floor() as usize;
    let x1 = (x0 + 1).min(area_ltc_tables::LTC_LUT_SIZE - 1);
    let y1 = (y0 + 1).min(area_ltc_tables::LTC_LUT_SIZE - 1);
    let tx = x - x0 as f32;
    let ty = y - y0 as f32;
    let top = lerp_vec4(table[y0][x0], table[y0][x1], tx);
    let bottom = lerp_vec4(table[y1][x0], table[y1][x1], tx);
    lerp_vec4(top, bottom, ty)
}

fn lerp_vec4(left: [f32; 4], right: [f32; 4], t: f32) -> [f32; 4] {
    [
        left[0].mul_add(1.0 - t, right[0] * t),
        left[1].mul_add(1.0 - t, right[1] * t),
        left[2].mul_add(1.0 - t, right[2] * t),
        left[3].mul_add(1.0 - t, right[3] * t),
    ]
}

fn apply_inverse_ltc_matrix(matrix: [f32; 4], value: Vec3) -> Vec3 {
    Vec3::new(
        matrix[0].mul_add(value.x, matrix[2] * value.z),
        value.y,
        matrix[1].mul_add(value.x, matrix[3] * value.z),
    )
}

fn integrate_transformed_quad(vertices: [Vec3; 4], two_sided: bool) -> f32 {
    let (mut clipped, count) = clip_quad_to_horizon(vertices);
    if count == 0 {
        return 0.0;
    }
    for vertex in clipped.iter_mut().take(count + 1) {
        *vertex = normalize_or(*vertex, Vec3::Z);
    }
    let mut sum = integrate_edge(clipped[0], clipped[1])
        + integrate_edge(clipped[1], clipped[2])
        + integrate_edge(clipped[2], clipped[3]);
    if count >= 4 {
        sum += integrate_edge(clipped[3], clipped[4]);
    }
    if count == 5 {
        sum += integrate_edge(clipped[4], clipped[0]);
    }
    if two_sided { sum.abs() } else { sum.max(0.0) }
}

fn clip_quad_to_horizon(vertices: [Vec3; 4]) -> ([Vec3; 5], usize) {
    let mut clipped = [
        vertices[0],
        vertices[1],
        vertices[2],
        vertices[3],
        Vec3::ZERO,
    ];
    let mut config = 0u8;
    for (index, vertex) in vertices.iter().enumerate() {
        if vertex.z > 0.0 {
            config |= 1 << index;
        }
    }

    let count = match config {
        0 => 0,
        1 => {
            clipped[1] = horizon_intersection(clipped[0], clipped[1]);
            clipped[2] = horizon_intersection(clipped[0], clipped[3]);
            3
        }
        2 => {
            clipped[0] = horizon_intersection(clipped[1], clipped[0]);
            clipped[2] = horizon_intersection(clipped[1], clipped[2]);
            3
        }
        3 => {
            clipped[2] = horizon_intersection(clipped[1], clipped[2]);
            clipped[3] = horizon_intersection(clipped[0], clipped[3]);
            4
        }
        4 => {
            clipped[0] = horizon_intersection(clipped[2], clipped[3]);
            clipped[1] = horizon_intersection(clipped[2], clipped[1]);
            3
        }
        5 => 0,
        6 => {
            clipped[0] = horizon_intersection(clipped[1], clipped[0]);
            clipped[3] = horizon_intersection(clipped[2], clipped[3]);
            4
        }
        7 => {
            clipped[4] = horizon_intersection(clipped[0], clipped[3]);
            clipped[3] = horizon_intersection(clipped[2], clipped[3]);
            5
        }
        8 => {
            clipped[0] = horizon_intersection(clipped[3], clipped[0]);
            clipped[1] = horizon_intersection(clipped[3], clipped[2]);
            clipped[2] = clipped[3];
            3
        }
        9 => {
            clipped[1] = horizon_intersection(clipped[0], clipped[1]);
            clipped[2] = horizon_intersection(clipped[3], clipped[2]);
            4
        }
        10 => 0,
        11 => {
            clipped[4] = clipped[3];
            clipped[3] = horizon_intersection(clipped[3], clipped[2]);
            clipped[2] = horizon_intersection(clipped[1], clipped[2]);
            5
        }
        12 => {
            clipped[1] = horizon_intersection(clipped[2], clipped[1]);
            clipped[0] = horizon_intersection(clipped[3], clipped[0]);
            4
        }
        13 => {
            clipped[4] = clipped[3];
            clipped[3] = clipped[2];
            clipped[2] = horizon_intersection(clipped[2], clipped[1]);
            clipped[1] = horizon_intersection(clipped[0], clipped[1]);
            5
        }
        14 => {
            clipped[4] = horizon_intersection(clipped[3], clipped[0]);
            clipped[0] = horizon_intersection(clipped[1], clipped[0]);
            5
        }
        15 => 4,
        _ => 0,
    };

    if count == 3 {
        clipped[3] = clipped[0];
    }
    if count == 4 {
        clipped[4] = clipped[0];
    }
    (clipped, count)
}

fn horizon_intersection(inside: Vec3, outside: Vec3) -> Vec3 {
    -outside.z * inside + inside.z * outside
}

fn integrate_edge(left: Vec3, right: Vec3) -> f32 {
    let cosine = left.dot(right).clamp(-0.9999, 0.9999);
    let y = cosine.abs();
    let numerator = 0.854_398_5 + (0.496_515_5 + 0.014_520_6 * y) * y;
    let denominator = (3.417_594 + (4.161_672_4 + y) * y).max(MIN_DENOMINATOR);
    let approximation = numerator / denominator;
    let theta_sin_theta = if cosine > 0.0 {
        approximation
    } else {
        0.5 * (1.0 - cosine * cosine).max(0.000_000_1).sqrt().recip() - approximation
    };
    left.cross(right).z * theta_sin_theta
}

fn fallback_tangent(normal: Vec3) -> Vec3 {
    let fallback_axis = if normal.z.abs() < 0.9 {
        Vec3::Z
    } else {
        Vec3::Y
    };
    normalize_or(fallback_axis.cross(normal), Vec3::X)
}

fn normalize_or(value: Vec3, fallback: Vec3) -> Vec3 {
    let length_squared = value.length_squared();
    if length_squared > 0.000_000_01 {
        value * length_squared.sqrt().recip()
    } else {
        fallback
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ltc_lookup_matches_selfshadow_table_texel_probes() {
        let probes = [
            (
                0usize,
                0usize,
                [1.0, 0.0, 0.0, 0.000_02],
                [1.0, 0.0, 0.0, 0.0],
            ),
            (
                17,
                23,
                [0.863_371, -0.088_664_3, 0.347_744, 0.229_351],
                [0.980_491, 0.000_105_514, 0.0, 0.023_963_5],
            ),
            (
                42,
                11,
                [0.309_18, -0.028_105_4, 0.462_417, 0.018_949_8],
                [0.998_878, 0.018_258, 0.0, 0.011_055_2],
            ),
            (
                63,
                63,
                [0.996_389, -0.080_812_4, 0.048_900_7, 1.657_7],
                [0.932_164, 0.047_189_9, 0.0, 1.0],
            ),
        ];

        for (row, column, expected_ltc_1, expected_ltc_2) in probes {
            let roughness = column as f32 / LUT_LAST;
            let n_dot_v = 1.0 - (row as f32 / LUT_LAST).powi(2);
            let actual = sample_ltc_tables(roughness, n_dot_v);
            assert_vec4_close(actual.inverse_matrix, expected_ltc_1, 0.000_002);
            assert_vec4_close(actual.fresnel_terms, expected_ltc_2, 0.000_002);
        }
    }

    #[test]
    fn ltc_rect_probe_matches_selfshadow_reference_irradiance() {
        let position = Vec3::ZERO;
        let normal = Vec3::Y;
        let view = normalize_or(Vec3::new(0.0, 0.8, 1.6), Vec3::Y);
        let center = Vec3::new(0.0, 1.35, 0.32);
        let axis_x = Vec3::X * 0.9;
        let axis_y = Vec3::Z * 0.45;
        let polygon = [
            center - axis_x - axis_y,
            center + axis_x - axis_y,
            center + axis_x + axis_y,
            center - axis_x + axis_y,
        ];

        let actual = evaluate_specular_polygon(
            polygon,
            position,
            normal,
            view,
            0.34,
            Vec3::new(0.82, 0.78, 0.72),
        );

        assert!(
            (actual.irradiance - 0.005_732_002).abs() <= 0.000_000_5,
            "LTC reference irradiance drifted: {actual:?}"
        );
        let expected_fresnel = Vec3::new(0.788_894_3, 0.752_738_24, 0.698_504_1);
        let delta = (actual.fresnel_scale - expected_fresnel).abs();
        assert!(
            delta.max_element() <= 0.000_01,
            "LTC reference Fresnel scale drifted: actual={actual:?}, expected={expected_fresnel:?}, delta={delta:?}"
        );
    }

    fn assert_vec4_close(actual: [f32; 4], expected: [f32; 4], tolerance: f32) {
        for index in 0..4 {
            assert!(
                (actual[index] - expected[index]).abs() <= tolerance,
                "vec4 component {index} drifted: actual={actual:?}, expected={expected:?}"
            );
        }
    }
}
